// Leaf-fn inlining (AST→AST, pre-codegen). Replaces a call to a small, pure,
// non-recursive, binding-free leaf fn with `{ let p_i = arg_i; <body> }`. 

use crate::ast::*;
use std::collections::HashMap;

const MAX_BODY_NODES: usize = 24;

// Identifiers resolve relative to their module (fns, statics, consts, types)
pub fn inline_leaf_fns(decls: Vec<Decl>) -> Vec<Decl> {
    let mut maps: HashMap<Vec<String>, HashMap<String, FnDecl>> = HashMap::new();
    let mut stack: Vec<Vec<String>> = Vec::new();
    for d in &decls {
        match d {
            Decl::ModEnter(p) => stack.push(p.clone()),
            Decl::ModExit => { stack.pop(); }
            Decl::Fn(f) if is_inlinable(f) => {
                let module = stack.last().cloned().unwrap_or_default();
                maps.entry(module).or_default().insert(f.name.clone(), f.clone());
            }
            _ => {}
        }
    }
    if maps.is_empty() { return decls; }

    let mut counter = 0usize;
    let mut out = decls;
    let mut stack: Vec<Vec<String>> = Vec::new();
    for d in &mut out {
        match d {
            Decl::ModEnter(p) => stack.push(p.clone()),
            Decl::ModExit => { stack.pop(); }
            Decl::Fn(f) => {
                let module = stack.last().cloned().unwrap_or_default();
                if let Some(inl) = maps.get(&module) {
                    inline_block(&mut f.body, inl, &mut counter);
                }
            }
            _ => {}
        }
    }
    out
}

fn is_inlinable(f: &FnDecl) -> bool {
    if !f.effects.is_empty() { return false; }
    if !f.generics.is_empty() { return false; }
    if !f.params.iter().all(|p| matches!(p, Param::Named { pattern, .. } if matches!(pattern.node, Pattern::Bind(_)))) {
        return false;
    }
    // body must be binding-free (no shadowing) and return-free, and not recurse.
    let mut ok = true;
    let mut nodes = 0usize;
    check_body(&f.body, &f.name, &mut ok, &mut nodes);
    ok && nodes <= MAX_BODY_NODES
}

fn check_body(b: &Block, self_name: &str, ok: &mut bool, nodes: &mut usize) {
    for s in &b.stmts {
        match &s.node {
            Stmt::Expr(e) => check_expr(e, self_name, ok, nodes),
            _ => *ok = false, // let / empty-in-stmt position
        }
    }
    if let Some(r) = &b.ret { check_expr(r, self_name, ok, nodes); }
}

fn check_expr(e: &Spanned<Expr>, self_name: &str, ok: &mut bool, nodes: &mut usize) {
    *nodes += 1;
    if !*ok { return; }
    match &e.node {
        Expr::Literal(Literal::StringInterp(parts)) =>
            *nodes += parts.iter().filter(|p| matches!(p, StringPart::Interp(_))).count(),
        Expr::Literal(_) | Expr::Continue | Expr::Error => {}
        Expr::Identifier(n) => { if n == self_name { *ok = false; } }
        Expr::Binary { left, right, .. } => { check_expr(left, self_name, ok, nodes); check_expr(right, self_name, ok, nodes); }
        Expr::Unary { right, .. } | Expr::Paren(right) | Expr::Throw(right) | Expr::Question(right) =>
            check_expr(right, self_name, ok, nodes),
        Expr::Call { callee, args } => {
            if let Expr::Identifier(n) = &callee.node { if n == self_name { *ok = false; } }
            check_expr(callee, self_name, ok, nodes);
            for a in args { check_expr(a, self_name, ok, nodes); }
        }
        Expr::Index { base, index } => { check_expr(base, self_name, ok, nodes); check_expr(index, self_name, ok, nodes); }
        Expr::If { condition, consequence, alternative } => {
            check_expr(condition, self_name, ok, nodes);
            check_expr(consequence, self_name, ok, nodes);
            if let Some(a) = alternative { check_expr(a, self_name, ok, nodes); }
        }
        Expr::FieldAccess { base, .. } => check_expr(base, self_name, ok, nodes),
        Expr::Tuple(xs) | Expr::Array(xs) | Expr::Variant { args: xs, .. } =>
            for x in xs { check_expr(x, self_name, ok, nodes); },
        Expr::ArrayRepeat { elem, count } => { check_expr(elem, self_name, ok, nodes); check_expr(count, self_name, ok, nodes); }
        Expr::Record { fields, .. } => for f in fields { if let Some(v) = &f.value { check_expr(v, self_name, ok, nodes); } },
        Expr::Block(b) => check_body(b, self_name, ok, nodes),
        Expr::Break(opt) => if let Some(x) = opt { check_expr(x, self_name, ok, nodes); },
        // binding-introducing / control we don't handle in MVP:
        _ => *ok = false, // Match/For/While/Loop/Closure/Region/Handle/Resume/Return/Range
    }
}

fn inline_block(b: &mut Block, inl: &HashMap<String, FnDecl>, ctr: &mut usize) {
    for s in &mut b.stmts {
        match &mut s.node {
            Stmt::Expr(e) => inline_expr(e, inl, ctr),
            Stmt::Let { value, .. } => inline_expr(value, inl, ctr),
            Stmt::Empty => {}
        }
    }
    if let Some(r) = &mut b.ret { inline_expr(r, inl, ctr); }
}

fn inline_expr(e: &mut Spanned<Expr>, inl: &HashMap<String, FnDecl>, ctr: &mut usize) {
    // Recurse into children first (so nested calls inline too).
    match &mut e.node {
        Expr::Binary { left, right, .. } => { inline_expr(left, inl, ctr); inline_expr(right, inl, ctr); }
        Expr::Unary { right, .. } | Expr::Paren(right) | Expr::Throw(right) | Expr::Question(right) => inline_expr(right, inl, ctr),
        Expr::Call { callee, args } => { inline_expr(callee, inl, ctr); for a in args.iter_mut() { inline_expr(a, inl, ctr); } }
        Expr::Index { base, index } => { inline_expr(base, inl, ctr); inline_expr(index, inl, ctr); }
        Expr::If { condition, consequence, alternative } => {
            inline_expr(condition, inl, ctr); inline_expr(consequence, inl, ctr);
            if let Some(a) = alternative { inline_expr(a, inl, ctr); }
        }
        Expr::Match { scrutinee, arms } => { inline_expr(scrutinee, inl, ctr); for a in arms { inline_expr(&mut a.body, inl, ctr); } }
        Expr::For { iter, body, .. } => { inline_expr(iter, inl, ctr); inline_block(body, inl, ctr); }
        Expr::While { condition, body } => { inline_expr(condition, inl, ctr); inline_block(body, inl, ctr); }
        Expr::Loop { body } | Expr::Region { body, .. } => inline_block(body, inl, ctr),
        Expr::Block(b) => inline_block(b, inl, ctr),
        Expr::Tuple(xs) | Expr::Array(xs) | Expr::Variant { args: xs, .. } => for x in xs { inline_expr(x, inl, ctr); },
        Expr::ArrayRepeat { elem, count } => { inline_expr(elem, inl, ctr); inline_expr(count, inl, ctr); }
        Expr::Record { fields, .. } => for f in fields { if let Some(v) = &mut f.value { inline_expr(v, inl, ctr); } },
        Expr::FieldAccess { base, .. } => inline_expr(base, inl, ctr),
        Expr::Break(Some(x)) | Expr::Return(Some(x)) | Expr::Resume(Some(x)) => inline_expr(x, inl, ctr),
        Expr::Closure { body, .. } => inline_expr(body, inl, ctr),
        Expr::Handle { expr, arms } => { inline_expr(expr, inl, ctr); for a in arms { inline_expr(&mut a.body, inl, ctr); } }
        Expr::Range { start, end, .. } => { if let Some(s)=start { inline_expr(s, inl, ctr); } if let Some(en)=end { inline_expr(en, inl, ctr); } }
        _ => {}
    }
    // Then try to inline this call.
    let replace = if let Expr::Call { callee, args } = &e.node {
        if let Expr::Identifier(name) = &callee.node {
            inl.get(name).filter(|f| f.params.len() == args.len()).cloned()
        } else { None }
    } else { None };
    if let Some(f) = replace {
        if let Expr::Call { args, .. } = &e.node {
            let id = *ctr; *ctr += 1;
            e.node = build_inlined(&f, args, id, e.span);
        }
    }
}

fn build_inlined(f: &FnDecl, args: &[Spanned<Expr>], id: usize, span: Span) -> Expr {
    let mut rename: HashMap<String, String> = HashMap::new();
    let mut stmts: Vec<Spanned<Stmt>> = Vec::new();
    for (p, a) in f.params.iter().zip(args.iter()) {
        let Param::Named { pattern, ty } = p else { continue };
        let Pattern::Bind(pname) = &pattern.node else { continue };
        let fresh = format!("__inl{}_{}", id, pname);
        rename.insert(pname.clone(), fresh.clone());
        stmts.push(Spanned {
            node: Stmt::Let {
                pattern: Spanned { node: Pattern::Bind(fresh), span },
                ty: Some(ty.clone()),
                value: a.clone(),
                is_mut: false,
            },
            span,
        });
    }
    let mut body = f.body.clone();
    rename_block(&mut body, &rename);
    let mut all_stmts = stmts;
    all_stmts.extend(body.stmts);
    Expr::Block(Block { stmts: all_stmts, ret: body.ret })
}

fn rename_block(b: &mut Block, m: &HashMap<String, String>) {
    for s in &mut b.stmts {
        match &mut s.node {
            Stmt::Expr(e) => rename_expr(e, m),
            Stmt::Let { value, .. } => rename_expr(value, m),
            Stmt::Empty => {}
        }
    }
    if let Some(r) = &mut b.ret { rename_expr(r, m); }
}

fn rename_expr(e: &mut Spanned<Expr>, m: &HashMap<String, String>) {
    match &mut e.node {
        Expr::Identifier(n) => { if let Some(f) = m.get(n) { *n = f.clone(); } }
        Expr::Literal(Literal::StringInterp(parts)) => for p in parts {
            if let StringPart::Interp(path) = p {
                if let Some(f) = path.first_mut().and_then(|h| m.get(h).cloned()) { path[0] = f; }
            }
        },
        Expr::Binary { left, right, .. } => { rename_expr(left, m); rename_expr(right, m); }
        Expr::Unary { right, .. } | Expr::Paren(right) | Expr::Throw(right) | Expr::Question(right) => rename_expr(right, m),
        Expr::Call { callee, args } => { rename_expr(callee, m); for a in args { rename_expr(a, m); } }
        Expr::Index { base, index } => { rename_expr(base, m); rename_expr(index, m); }
        Expr::If { condition, consequence, alternative } => {
            rename_expr(condition, m); rename_expr(consequence, m);
            if let Some(a) = alternative { rename_expr(a, m); }
        }
        Expr::FieldAccess { base, .. } => rename_expr(base, m),
        Expr::Tuple(xs) | Expr::Array(xs) | Expr::Variant { args: xs, .. } => for x in xs { rename_expr(x, m); },
        Expr::ArrayRepeat { elem, count } => { rename_expr(elem, m); rename_expr(count, m); }
        Expr::Record { fields, .. } => for fld in fields { if let Some(v) = &mut fld.value { rename_expr(v, m); } },
        Expr::Block(b) => rename_block(b, m),
        Expr::Break(Some(x)) => rename_expr(x, m),
        _ => {}
    }
}
