// BNF-driven parser tests — one test per grammar rule.
// by the section wiki/14-bnf.md.

use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::{Parser, Precedence};

fn expr(input: &str) -> Expr {
    let mut p = Parser::new(Lexer::new(input));
    let e = p.parse_expr(Precedence::Lowest);
    assert!(p.errors.is_empty(), "unexpected errors for {:?}: {:?}", input, p.errors);
    e.node
}

fn pat(input: &str) -> Pattern {
    Parser::new(Lexer::new(input))
        .parse_pattern()
        .expect("pattern parse failed")
        .node
}

fn decl(input: &str) -> Decl {
    let mut p = Parser::new(Lexer::new(input));
    let d = p.parse_decl().expect("decl parse failed");
    assert!(p.errors.is_empty(), "unexpected errors for {:?}: {:?}", input, p.errors);
    d
}

fn prog(input: &str) -> Vec<Decl> {
    let mut p = Parser::new(Lexer::new(input));
    let ds = p.parse_program();
    assert!(p.errors.is_empty(), "unexpected parse errors: {:?}", p.errors);
    ds
}

fn errs(input: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(input));
    let _ = p.parse_program();
    p.errors.into_iter().map(|e| e.message).collect()
}

// §1  Top-level 

// <import> without an import-list  (items are optional in the BNF)
#[test]
fn import_no_items() {
    let ds = prog("use std.io; fn main() -> Int { 0 }");
    assert_eq!(ds.len(), 2);
    if let Decl::Use { path, items } = &ds[0] {
        assert_eq!(path, &vec!["std".to_string(), "io".to_string()]);
        assert!(items.is_empty());
    } else { panic!("expected Import"); }
}

// <import-item> with 'as' rename
#[test]
fn import_item_as_rename() {
    let ds = prog("use io.{File as F, Read}; fn main() -> Int { 0 }");
    assert_eq!(ds.len(), 2);
    if let Decl::Use { items, .. } = &ds[0] {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "File");
        assert_eq!(items[0].alias, Some("F".to_string()));
        assert_eq!(items[1].alias, None);
    } else { panic!("expected Import"); }
}

// <mod-decl> must come first; second fn follows cleanly
#[test]
fn mod_decl_dotted_path_then_effect() {
    let ds = prog("mod a.b effect E { } fn main() -> Int { 0 }");
    assert_eq!(ds.len(), 3);
    assert!(matches!(ds[0], Decl::Mod(ref s) if s == "a.b"));
}

// §2  Type declarations 

// <variant-body> — unit cases
#[test]
fn type_decl_variant_unit_cases() {
    let d = decl("type Color = | Red | Green | Blue");
    if let Decl::Type { name, body: TypeBody::Variant(cases), .. } = d {
        assert_eq!(name, "Color");
        assert_eq!(cases.len(), 3);
        assert!(matches!(&cases[0], VariantCase::Unit(n) if n == "Red"));
        assert!(matches!(&cases[1], VariantCase::Unit(n) if n == "Green"));
        assert!(matches!(&cases[2], VariantCase::Unit(n) if n == "Blue"));
    } else { panic!("expected Variant type"); }
}

// <variant-body> — no leading pipe
#[test]
fn type_decl_variant_no_leading_pipe() {
    let d = decl("type Bool2 = True | False");
    if let Decl::Type { body: TypeBody::Variant(cases), .. } = d {
        assert_eq!(cases.len(), 2);
    } else { panic!("expected Variant type"); }
}

// <variant-case> — tuple payload
#[test]
fn type_decl_variant_tuple_payload() {
    let d = decl("type Shape = | Circle(Float) | Rect(Int, Int)");
    if let Decl::Type { body: TypeBody::Variant(cases), .. } = d {
        assert_eq!(cases.len(), 2);
        assert!(matches!(&cases[0], VariantCase::Tuple(n, ts) if n == "Circle" && ts.len() == 1));
        assert!(matches!(&cases[1], VariantCase::Tuple(n, ts) if n == "Rect" && ts.len() == 2));
    } else { panic!("expected Variant type"); }
}

// <variant-case> — record payload
#[test]
fn type_decl_variant_record_payload() {
    let d = decl("type Node = | Leaf { val: Int } | Branch { left: Int, right: Int }");
    if let Decl::Type { body: TypeBody::Variant(cases), .. } = d {
        assert_eq!(cases.len(), 2);
        assert!(matches!(&cases[0], VariantCase::Record(n, _) if n == "Leaf"));
        assert!(matches!(&cases[1], VariantCase::Record(n, fs) if n == "Branch" && fs.len() == 2));
    } else { panic!("expected Variant type"); }
}

// <record-field> with 'pub'
#[test]
fn type_decl_record_pub_field() {
    let d = decl("type Pt = { pub x: Int, y: Int }");
    if let Decl::Type { body: TypeBody::Record(fields), .. } = d {
        assert_eq!(fields.len(), 2);
        assert!(fields[0].is_pub);
        assert!(!fields[1].is_pub);
    } else { panic!("expected Record type"); }
}

// <ownership-attr> — @copy
#[test]
fn type_decl_ownership_copy() {
    let d = decl("@copy type Pt = { x: Int }");
    if let Decl::Type { ownership, .. } = d {
        assert!(matches!(ownership, Some(OwnershipAttr::Copy)));
    } else { panic!("expected Type"); }
}

// <ownership-attr> — @move
#[test]
fn type_decl_ownership_move() {
    let d = decl("@move type Resource = { id: Int }");
    if let Decl::Type { ownership, .. } = d {
        assert!(matches!(ownership, Some(OwnershipAttr::Move)));
    } else { panic!("expected Type"); }
}

// <ownership-attr> — @share
#[test]
fn type_decl_ownership_share() {
    let d = decl("@share type Shared = { v: Int }");
    if let Decl::Type { ownership, .. } = d {
        assert!(matches!(ownership, Some(OwnershipAttr::Share)));
    } else { panic!("expected Type"); }
}

// 'pub' type decl
#[test]
fn type_decl_pub() {
    let d = decl("pub type X = { n: Int }");
    if let Decl::Type { is_pub, .. } = d {
        assert!(is_pub);
    } else { panic!("expected Type"); }
}

// §4  Effect declarations 

// 'pub' effect
#[test]
fn effect_decl_pub() {
    let d = decl("pub effect Io { }");
    if let Decl::Effect { is_pub, name, .. } = d {
        assert!(is_pub);
        assert_eq!(name, "Io");
    } else { panic!("expected Effect"); }
}

// 'pub' effect alias
#[test]
fn effect_alias_pub() {
    let d = decl("pub effect alias Std = <io, exn>;");
    if let Decl::EffectAlias { is_pub, name, effects } = d {
        assert!(is_pub);
        assert_eq!(name, "Std");
        assert_eq!(effects.len(), 2);
    } else { panic!("expected EffectAlias"); }
}

// §5  Function declarations 

// <param> — bare 'self'
#[test]
fn fn_param_self_val() {
    let d = decl("fn consume(self) -> Int { 0 }");
    if let Decl::Fn(f) = d {
        assert_eq!(f.params.len(), 1);
        assert!(matches!(f.params[0], Param::SelfVal));
    } else { panic!("expected Fn"); }
}

// <param> — '&self'
#[test]
fn fn_param_self_ref() {
    let d = decl("fn get(&self) -> Int { 0 }");
    if let Decl::Fn(f) = d {
        assert_eq!(f.params.len(), 1);
        assert!(matches!(f.params[0], Param::SelfRef { is_mut: false }));
    } else { panic!("expected Fn"); }
}

// <param> — '&mut self'
#[test]
fn fn_param_self_ref_mut() {
    let d = decl("fn set(&mut self, x: Int) -> Int { x }");
    if let Decl::Fn(f) = d {
        assert!(matches!(f.params[0], Param::SelfRef { is_mut: true }));
        assert_eq!(f.params.len(), 2);
    } else { panic!("expected Fn"); }
}

// 'pub' fn
#[test]
fn fn_decl_pub() {
    let d = decl("pub fn run() -> Int { 0 }");
    if let Decl::Fn(f) = d {
        assert!(f.is_pub);
    } else { panic!("expected Fn"); }
}

// 'pub' const
#[test]
fn const_decl_pub() {
    let d = decl("pub const N: Int = 42;");
    if let Decl::Const { is_pub, name, .. } = d {
        assert!(is_pub);
        assert_eq!(name, "N");
    } else { panic!("expected Const"); }
}

// §6  Trait & Impl 

// <trait-item> — required fn signature (no body)
#[test]
fn trait_with_required_fn_signature() {
    let d = decl("trait Show { fn show(self) -> Int }");
    if let Decl::Trait { name, items, .. } = d {
        assert_eq!(name, "Show");
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], TraitItem::Required(_)));
        if let TraitItem::Required(sig) = &items[0] {
            assert_eq!(sig.name, "show");
        }
    } else { panic!("expected Trait"); }
}

// <trait-item> — default fn (with body)
#[test]
fn trait_with_default_fn() {
    let d = decl("trait Greet { fn hello() -> Int { 0 } }");
    if let Decl::Trait { items, .. } = d {
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], TraitItem::Default(_)));
    } else { panic!("expected Trait"); }
}

// Trait with both required and default items
#[test]
fn trait_mixed_items() {
    let d = decl("trait Foo { fn req(self) -> Int fn def() -> Int { 0 } }");
    if let Decl::Trait { items, .. } = d {
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], TraitItem::Required(_)));
        assert!(matches!(items[1], TraitItem::Default(_)));
    } else { panic!("expected Trait"); }
}

// <impl-decl> with 'for' — `impl Trait for Type`
#[test]
fn impl_decl_for_type() {
    let d = decl("impl Show for Int { }");
    if let Decl::Impl { trait_name, for_type, .. } = d {
        assert_eq!(trait_name, Some(vec!["Show".to_string()]));
        assert_eq!(for_type, Type::Named("Int".into()));
    } else { panic!("expected Impl"); }
}

// <impl-decl> with generic params
#[test]
fn impl_decl_with_generics() {
    let d = decl("impl<T> Container<T> { }");
    if let Decl::Impl { generics, .. } = d {
        assert_eq!(generics.len(), 1);
        assert_eq!(generics[0].name, "T");
    } else { panic!("expected Impl"); }
}

// <impl-decl> with method
#[test]
fn impl_decl_with_method() {
    let d = decl("impl Int { fn inc(self) -> Int { 0 } }");
    if let Decl::Impl { methods, .. } = d {
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, "inc");
    } else { panic!("expected Impl"); }
}

// impl with where clause
#[test]
fn impl_decl_with_where_clause() {
    let d = decl("impl<T> Show for T where T: Debug { }");
    if let Decl::Impl { where_clause, .. } = d {
        assert_eq!(where_clause.len(), 1);
    } else { panic!("expected Impl"); }
}

// §8  Expressions 

// <or-expr>  a || b
#[test]
fn expr_logical_or() {
    let e = expr("a || b");
    assert!(matches!(e, Expr::Binary { op: BinaryOp::Or, .. }));
}

// <and-expr>  a && b
#[test]
fn expr_logical_and() {
    let e = expr("a && b");
    assert!(matches!(e, Expr::Binary { op: BinaryOp::And, .. }));
}

// <unary-expr>  !x
#[test]
fn expr_unary_not() {
    let e = expr("!true");
    assert!(matches!(e, Expr::Unary { op: UnaryOp::Not, .. }));
}

// <unary-expr>  -x
#[test]
fn expr_unary_neg() {
    let e = expr("-5");
    assert!(matches!(e, Expr::Unary { op: UnaryOp::Neg, .. }));
}

// <unary-expr>  *ptr  (deref)
#[test]
fn expr_unary_deref() {
    let e = expr("*ptr");
    assert!(matches!(e, Expr::Unary { op: UnaryOp::Deref, .. }));
}

// <unary-expr>  &x  (borrow)
#[test]
fn expr_unary_borrow() {
    let e = expr("&x");
    assert!(matches!(e, Expr::Unary { op: UnaryOp::Ref, .. }));
}

// <unary-expr>  &mut x  (mutable borrow)
#[test]
fn expr_unary_borrow_mut() {
    let e = expr("&mut x");
    assert!(matches!(e, Expr::Unary { op: UnaryOp::RefMut, .. }));
}

// <postfix-op>  field access  obj.field
#[test]
fn expr_field_access() {
    let e = expr("obj.field");
    assert!(matches!(e, Expr::FieldAccess { field, .. } if field == "field"));
}

// <postfix-op>  chained field access  a.b.c
#[test]
fn expr_field_access_chained() {
    let e = expr("a.b.c");
    if let Expr::FieldAccess { base, field } = e {
        assert_eq!(field, "c");
        assert!(matches!(base.node, Expr::FieldAccess { field: ref f, .. } if f == "b"));
    } else { panic!("expected FieldAccess, got {:?}", e); }
}

// <postfix-op>  index  a[0]
#[test]
fn expr_index() {
    let e = expr("a[0]");
    if let Expr::Index { base, index } = e {
        assert!(matches!(base.node, Expr::Identifier(ref n) if n == "a"));
        assert!(matches!(index.node, Expr::Literal(Literal::Int(0))));
    } else { panic!("expected Index, got {:?}", e); }
}

// <postfix-op>  error-propagation  f()?
#[test]
fn expr_question_mark() {
    let e = expr("f()?");
    assert!(matches!(e, Expr::Question(_)));
}

// <range-expr>  1..10
#[test]
fn expr_range_exclusive() {
    let e = expr("1..10");
    if let Expr::Range { start, end, inclusive } = e {
        assert!(!inclusive);
        assert!(start.is_some());
        assert!(end.is_some());
    } else { panic!("expected Range, got {:?}", e); }
}

// <range-expr>  1..=10
#[test]
fn expr_range_inclusive() {
    let e = expr("1..=10");
    if let Expr::Range { inclusive, start, end } = e {
        assert!(inclusive);
        assert!(start.is_some());
        assert!(end.is_some());
    } else { panic!("expected Range, got {:?}", e); }
}

// <range-expr>  ..10  (no start)
#[test]
fn expr_range_no_start() {
    let e = expr("..10");
    if let Expr::Range { start, end, inclusive } = e {
        assert!(!inclusive);
        assert!(start.is_none());
        assert!(end.is_some());
    } else { panic!("expected Range, got {:?}", e); }
}

// <range-expr>  ..=10
#[test]
fn expr_range_no_start_inclusive() {
    let e = expr("..=10");
    if let Expr::Range { start, end, inclusive } = e {
        assert!(inclusive);
        assert!(start.is_none());
        assert!(end.is_some());
    } else { panic!("expected Range, got {:?}", e); }
}

// <range-expr>  1..  (no end)
#[test]
fn expr_range_no_end() {
    let e = expr("1..");
    if let Expr::Range { start, end, .. } = e {
        assert!(start.is_some());
        assert!(end.is_none());
    } else { panic!("expected Range, got {:?}", e); }
}

// <assign-op>  *=
#[test]
fn expr_mul_assign() {
    let e = expr("x *= 2");
    if let Expr::Binary { op: BinaryOp::Assign, right, .. } = e {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Mul, .. }));
    } else { panic!("expected Assign(Mul), got {:?}", e); }
}

// <assign-op>  /=
#[test]
fn expr_div_assign() {
    let e = expr("x /= 2");
    if let Expr::Binary { op: BinaryOp::Assign, right, .. } = e {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Div, .. }));
    } else { panic!("expected Assign(Div), got {:?}", e); }
}

// <assign-op>  %=
#[test]
fn expr_mod_assign() {
    let e = expr("x %= 2");
    if let Expr::Binary { op: BinaryOp::Assign, right, .. } = e {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Mod, .. }));
    } else { panic!("expected Assign(Mod), got {:?}", e); }
}

// <record-expr>  shorthand field init  Pt { x, y }
#[test]
fn expr_record_field_shorthand() {
    let e = expr("Pt { x, y }");
    if let Expr::Record { ty, fields } = e {
        assert_eq!(ty, vec!["Pt".to_string()]);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "x");
        assert!(fields[0].value.is_none(), "shorthand field must have no explicit value");
        assert_eq!(fields[1].name, "y");
        assert!(fields[1].value.is_none());
    } else { panic!("expected Record expr, got {:?}", e); }
}

// <record-expr>  mixed shorthand and explicit
#[test]
fn expr_record_mixed_fields() {
    let e = expr("Pt { x, y: 2 }");
    if let Expr::Record { fields, .. } = e {
        assert_eq!(fields.len(), 2);
        assert!(fields[0].value.is_none());
        assert!(fields[1].value.is_some());
    } else { panic!("expected Record expr, got {:?}", e); }
}

// <variant-expr>  qualified variant with args
// The parser cannot distinguish variant constructors from method calls at parse
// time; Option.Some(42) is represented as Call { callee: FieldAccess(...) }.
// Typeck resolves which case it is.
#[test]
fn expr_variant_qualified() {
    let e = expr("Option.Some(42)");
    if let Expr::Call { callee, args } = e {
        assert!(
            matches!(callee.node, Expr::FieldAccess { ref field, .. } if field == "Some"),
            "expected FieldAccess callee with field 'Some', got {:?}", callee.node
        );
        assert_eq!(args.len(), 1);
    } else { panic!("expected Call(FieldAccess) for qualified variant, got {:?}", e); }
}

// §9  Control flow 

// <return-expr>  with value
#[test]
fn expr_return_with_value() {
    let e = expr("return 5");
    if let Expr::Return(Some(v)) = e {
        assert!(matches!(v.node, Expr::Literal(Literal::Int(5))));
    } else { panic!("expected Return(Some), got {:?}", e); }
}

// <return-expr>  bare return (no value)
#[test]
fn expr_return_bare() {
    // Inside fn body so the block parser sees it as a statement expression.
    let ds = prog("fn f() -> Int { return; 0 }");
    if let Decl::Fn(f) = &ds[0] {
        let stmt = &f.body.stmts[0];
        if let Stmt::Expr(ref e) = stmt.node {
            assert!(matches!(e.node, Expr::Return(None)));
        } else { panic!("expected Expr stmt"); }
    } else { panic!("expected Fn"); }
}

// <break-expr>  with value
#[test]
fn expr_break_with_value() {
    let e = expr("break 42");
    if let Expr::Break(Some(v)) = e {
        assert!(matches!(v.node, Expr::Literal(Literal::Int(42))));
    } else { panic!("expected Break(Some), got {:?}", e); }
}

// <break-expr>  bare
#[test]
fn expr_break_bare() {
    let e = expr("break");
    assert!(matches!(e, Expr::Break(None)));
}

// <continue-expr>
#[test]
fn expr_continue() {
    let e = expr("continue");
    assert!(matches!(e, Expr::Continue));
}

// <throw-expr>
#[test]
fn expr_throw() {
    let e = expr("throw err");
    assert!(matches!(e, Expr::Throw(_)));
}

// §10  Region & Handle 

// <handle-arm>  effect qualified-name arm  (e.g. `Exn e => body`)
#[test]
fn handle_arm_effect_name() {
    let e = expr("handle f() { return v => v, Exn e => 0 }");
    if let Expr::Handle { arms, .. } = e {
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].kind, HandleArmKind::Return));
        assert!(matches!(&arms[1].kind, HandleArmKind::Effect(name) if name == &vec!["Exn".to_string()]));
    } else { panic!("expected Handle expr, got {:?}", e); }
}

// <handle-arm>  exn arm
#[test]
fn handle_arm_exn() {
    let e = expr("handle f() { exn e => 0 }");
    if let Expr::Handle { arms, .. } = e {
        assert_eq!(arms.len(), 1);
        assert!(matches!(arms[0].kind, HandleArmKind::Exn));
    } else { panic!("expected Handle expr"); }
}

// <handle-arm>  qualified effect name  io.read k => body
#[test]
fn handle_arm_qualified_effect_name() {
    let e = expr("handle f() { io.read k => resume() }");
    if let Expr::Handle { arms, .. } = e {
        assert_eq!(arms.len(), 1);
        assert!(matches!(&arms[0].kind,
            HandleArmKind::Effect(name) if name == &vec!["io".to_string(), "read".to_string()]));
    } else { panic!("expected Handle"); }
}

// §11  Patterns 

// <array-pattern>  basic
#[test]
fn pattern_array_basic() {
    let p = pat("[a, b]");
    if let Pattern::Array(elems) = p {
        assert_eq!(elems.len(), 2);
        assert!(matches!(elems[0].node, Pattern::Bind(ref n) if n == "a"));
        assert!(matches!(elems[1].node, Pattern::Bind(ref n) if n == "b"));
    } else { panic!("expected Array pattern, got {:?}", p); }
}

// <array-pattern>  empty
#[test]
fn pattern_array_empty() {
    let p = pat("[]");
    assert!(matches!(p, Pattern::Array(ref v) if v.is_empty()));
}

// <array-pattern>  with rest ..
#[test]
fn pattern_array_with_rest() {
    // [a, .., b] — rest in middle
    let p = pat("[a, .., b]");
    if let Pattern::Array(elems) = p {
        assert_eq!(elems.len(), 3);
        // middle element is the rest pattern — represented as Wildcard or a dedicated node
        // The AST doesn't have a separate Rest variant; check that three elements parsed.
    } else { panic!("expected Array pattern, got {:?}", p); }
}

// <record-pattern>  shorthand fields
#[test]
fn pattern_record_shorthand() {
    let p = pat("Pt { x, y }");
    if let Pattern::Record { ty, fields, rest } = p {
        assert_eq!(ty, vec!["Pt".to_string()]);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "x");
        assert!(fields[0].pattern.is_none(), "shorthand: no inner pattern");
        assert!(!rest);
    } else { panic!("expected Record pattern, got {:?}", p); }
}

// <record-pattern>  explicit field binding
#[test]
fn pattern_record_explicit_binding() {
    let p = pat("Pt { x: px, y: py }");
    if let Pattern::Record { fields, rest, .. } = p {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "x");
        assert!(fields[0].pattern.is_some());
        assert!(!rest);
    } else { panic!("expected Record pattern, got {:?}", p); }
}

// <record-pattern>  with rest ..
#[test]
fn pattern_record_with_rest() {
    let p = pat("Pt { x, .. }");
    if let Pattern::Record { fields, rest, .. } = p {
        assert_eq!(fields.len(), 1);
        assert!(rest);
    } else { panic!("expected Record pattern, got {:?}", p); }
}

// <variant-pattern>  with payload
#[test]
fn pattern_variant_with_payload() {
    let p = pat("Some(x)");
    if let Pattern::Variant { ty, args } = p {
        assert_eq!(ty, vec!["Some".to_string()]);
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0].node, Pattern::Bind(ref n) if n == "x"));
    } else { panic!("expected Variant pattern, got {:?}", p); }
}

// <variant-pattern>  unit (no args)
#[test]
fn pattern_variant_unit() {
    let p = pat("None");
    // A bare uppercase name — parser may produce Bind or Variant with no args.
    assert!(
        matches!(&p, Pattern::Bind(n) if n == "None") ||
        matches!(&p, Pattern::Variant { ty, args } if ty == &vec!["None".to_string()] && args.is_empty()),
        "expected Bind(None) or Variant{{None,[]}}, got {:?}", p
    );
}

// <variant-pattern>  qualified  Option.Some(x)
#[test]
fn pattern_variant_qualified() {
    let p = pat("Option.Some(x)");
    if let Pattern::Variant { ty, args } = p {
        assert_eq!(ty, vec!["Option".to_string(), "Some".to_string()]);
        assert_eq!(args.len(), 1);
    } else { panic!("expected Variant pattern, got {:?}", p); }
}

// <range-pattern>  exclusive  1..5
#[test]
fn pattern_range_exclusive() {
    let p = pat("1..5");
    if let Pattern::Range { start, end, inclusive } = p {
        assert!(!inclusive);
        assert!(matches!(start, Some(Literal::Int(1))));
        assert!(matches!(end, Some(Literal::Int(5))));
    } else { panic!("expected Range pattern, got {:?}", p); }
}

// <range-pattern>  inclusive  1..=5
#[test]
fn pattern_range_inclusive() {
    let p = pat("1..=5");
    if let Pattern::Range { inclusive, start, end } = p {
        assert!(inclusive);
        assert!(matches!(start, Some(Literal::Int(1))));
        assert!(matches!(end, Some(Literal::Int(5))));
    } else { panic!("expected Range pattern, got {:?}", p); }
}

// &<pattern>  reference pattern
#[test]
fn pattern_ref() {
    let p = pat("&x");
    if let Pattern::Ref(inner) = p {
        assert!(matches!(inner.node, Pattern::Bind(ref n) if n == "x"));
    } else { panic!("expected Ref pattern, got {:?}", p); }
}

// <tuple-pattern>  with rest element  (a, ..)
#[test]
fn pattern_tuple_with_rest() {
    let p = pat("(a, ..)");
    if let Pattern::Tuple(elems) = p {
        assert_eq!(elems.len(), 2);
    } else { panic!("expected Tuple pattern, got {:?}", p); }
}

// §12  Literals 

// <float-literal>
#[test]
fn literal_float_basic() {
    let e = expr("3.14");
    assert!(matches!(e, Expr::Literal(Literal::Float(_))));
    if let Expr::Literal(Literal::Float(v)) = e {
        assert!((v - 3.14f64).abs() < 1e-9);
    }
}

// <float-literal>  with exponent
#[test]
fn literal_float_exponent() {
    let e = expr("1.0e10");
    assert!(matches!(e, Expr::Literal(Literal::Float(v)) if (v - 1.0e10_f64).abs() < 1e3));
}

// <char-literal>
#[test]
fn literal_char_basic() {
    let e = expr("'a'");
    assert!(matches!(e, Expr::Literal(Literal::Char('a'))));
}

// <char-literal>  escape  '\n'
#[test]
fn literal_char_escape_newline() {
    let e = expr(r"'\n'");
    assert!(matches!(e, Expr::Literal(Literal::Char('\n'))));
}

// <string-literal>  escape sequences in string
#[test]
fn literal_string_escape_tab() {
    let e = expr(r#""\t""#);
    if let Expr::Literal(Literal::String(s)) = e {
        assert_eq!(s, "\t");
    } else { panic!("expected String literal, got {:?}", e); }
}

// <escape-sequence>  unicode  "\u{0041}" == 'A'
#[test]
fn literal_string_unicode_escape() {
    let e = expr(r#""\u{0041}""#);
    if let Expr::Literal(Literal::String(s)) = e {
        assert_eq!(s, "A");
    } else { panic!("expected String literal, got {:?}", e); }
}

// <interpolation>  multiple segments
#[test]
fn literal_string_interp_multiple_segments() {
    let e = expr(r#""{a} and {b}""#);
    if let Expr::Literal(Literal::StringInterp(parts)) = e {
        assert_eq!(parts.len(), 3); // [Interp(a), Literal(" and "), Interp(b)]
    } else { panic!("expected StringInterp, got {:?}", e); }
}

// <string-literal>  only interpolation, no surrounding text
#[test]
fn literal_string_interp_only() {
    let e = expr(r#""{x}""#);
    if let Expr::Literal(Literal::StringInterp(parts)) = e {
        assert_eq!(parts.len(), 1);
        assert!(matches!(&parts[0], StringPart::Interp(segs) if segs == &vec!["x".to_string()]));
    } else { panic!("expected StringInterp, got {:?}", e); }
}

// §13  Attributes 

// <attr-arg>  identifier-only arg
#[test]
fn attribute_ident_arg() {
    let d = decl("@derive(Eq) type X = { n: Int }");
    if let Decl::Type { attrs, .. } = d {
        assert_eq!(attrs[0].args.len(), 1);
        assert!(matches!(attrs[0].args[0], AttrArg::Ident(ref s) if s == "Eq"));
    } else { panic!("expected Type"); }
}

// Multiple identifier args
#[test]
fn attribute_multiple_ident_args() {
    let d = decl("@derive(Eq, Ord, Show) type X = { n: Int }");
    if let Decl::Type { attrs, .. } = d {
        assert_eq!(attrs[0].args.len(), 3);
    } else { panic!("expected Type"); }
}

// Cross-section: operator precedence 

// || has lower precedence than &&
#[test]
fn precedence_or_lower_than_and() {
    // a || b && c  =>  a || (b && c)
    let e = expr("a || b && c");
    if let Expr::Binary { op: BinaryOp::Or, right, .. } = e {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::And, .. }));
    } else { panic!("expected Or at top level, got {:?}", e); }
}

// && has lower precedence than ==
#[test]
fn precedence_and_lower_than_eq() {
    // a && b == c  =>  a && (b == c)
    let e = expr("a && b == c");
    if let Expr::Binary { op: BinaryOp::And, right, .. } = e {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Eq, .. }));
    } else { panic!("expected And at top level, got {:?}", e); }
}

// unary ! binds tighter than &&
#[test]
fn precedence_not_tighter_than_and() {
    // !a && b  =>  (!a) && b
    let e = expr("!a && b");
    if let Expr::Binary { op: BinaryOp::And, left, .. } = e {
        assert!(matches!(left.node, Expr::Unary { op: UnaryOp::Not, .. }));
    } else { panic!("expected And at top, got {:?}", e); }
}

// postfix ? binds tighter than unary -
#[test]
fn precedence_question_tighter_than_neg() {
    // -f()?  =>  -(f()?)
    let e = expr("-f()?");
    if let Expr::Unary { op: UnaryOp::Neg, right } = e {
        assert!(matches!(right.node, Expr::Question(_)));
    } else { panic!("expected Neg(Question), got {:?}", e); }
}

// Cross-section: stmt / block 

// expr-with-block as statement — no trailing semicolon required
#[test]
fn stmt_expr_with_block_no_semicolon() {
    // `if` at statement position needs no `;`
    let ds = prog("fn f() -> Int { if true { 1 } 0 }");
    if let Decl::Fn(f) = &ds[0] {
        assert_eq!(f.body.stmts.len(), 1);
        if let Stmt::Expr(e) = &f.body.stmts[0].node {
            assert!(matches!(e.node, Expr::If { .. }));
        } else { panic!("expected Expr stmt"); }
        assert!(f.body.ret.is_some());
    } else { panic!("expected Fn"); }
}

// semicolon-only statement
#[test]
fn stmt_empty_semicolon() {
    let ds = prog("fn f() -> Int { ; 0 }");
    if let Decl::Fn(f) = &ds[0] {
        // empty stmt may be elided; body returns 0
        assert!(f.body.ret.is_some());
    } else { panic!("expected Fn"); }
}

// Error cases 

// Variant type with duplicate case name
#[test]
fn error_variant_type_duplicate_case() {
    let e = errs("type X = | A | A");
    assert!(!e.is_empty(), "expected error for duplicate variant case, got none");
}

// Record type with duplicate field
#[test]
fn error_record_type_duplicate_field() {
    let e = errs("type Pt = { x: Int, x: Int }");
    assert!(!e.is_empty(), "expected duplicate-field error, got none");
}

// Impl with 'for' but no trait (just a type)
#[test]
fn impl_for_type_only_parses() {
    // `impl Foo` should parse as impl-decl with for_type = Foo, no trait.
    let d = decl("impl Int { }");
    assert!(matches!(d, Decl::Impl { trait_name: None, .. }));
}

// ── Negative tests (one per BNF section) ─────────────────────────────────────

// §1  <import> — path is required after 'import'
#[test]
fn error_import_missing_path() {
    let e = errs("import; fn main() -> Int { 0 }");
    assert!(!e.is_empty(), "expected error: import with no module path, got none");
}

// §2  <record-body> — field must have ': <type>'
#[test]
fn error_type_record_field_missing_colon() {
    let e = errs("type Pt = { x Int }");
    assert!(!e.is_empty(), "expected error: record field without ':', got none");
}

// §3  <function-type> — '->' must be followed by a return type
#[test]
fn error_type_function_missing_return() {
    let e = errs("fn f(cb: (Int) ->) -> Int { 0 }");
    assert!(!e.is_empty(), "expected error: function type with nothing after '->', got none");
}

// §4  <effect-op> — '->' and return type are required
#[test]
fn error_effect_op_missing_arrow() {
    let e = errs("effect E { op foo() Unit }");
    assert!(!e.is_empty(), "expected error: effect op missing '->', got none");
}

// §5  <param> — pattern must be followed by ': <type>'
#[test]
fn error_fn_param_missing_type() {
    let e = errs("fn f(x) -> Int { 0 }");
    assert!(!e.is_empty(), "expected error: fn param without type annotation, got none");
}

// §6  <impl-decl> — a type name is required after 'impl'
#[test]
fn error_impl_no_type_name() {
    let e = errs("impl { }");
    assert!(!e.is_empty(), "expected error: impl with no type name, got none");
}

// §7  <let-stmt> — '=' and a value are required
#[test]
fn error_let_missing_value() {
    let e = errs("fn f() -> Int { let x: Int; 0 }");
    assert!(!e.is_empty(), "expected error: let with no '=' and no value, got none");
}

// §8  <postfix-op> index — the index expression cannot be empty
#[test]
fn error_expr_index_empty() {
    let e = errs("fn f(a: Int) -> Int { a[] }");
    assert!(!e.is_empty(), "expected error: empty index expression a[], got none");
}

// §9  <if-expr> — a block body '{...}' is required after the condition
#[test]
fn error_if_missing_block() {
    let e = errs("fn f() -> Int { if true }");
    assert!(!e.is_empty(), "expected error: if with no block body, got none");
}

// §10  <resume-expr> — at most one argument is accepted
#[test]
fn error_resume_too_many_args() {
    let e = errs("fn f() -> Int { resume(1, 2) }");
    assert!(!e.is_empty(), "expected error: resume(1, 2) with two args, got none");
}

// §11  <pattern-alt> — a pattern must follow the '|' operator
#[test]
fn error_pattern_trailing_pipe() {
    let e = errs("fn f(x: Int) -> Int { match x { A | => 0 } }");
    assert!(!e.is_empty(), "expected error: trailing '|' with no following pattern, got none");
}

// §12  <escape-sequence> — '\u{...}' requires at least one hex digit
#[test]
fn error_literal_unicode_escape_empty() {
    let e = errs(r#"fn f() -> Int { "\u{}" }"#);
    assert!(!e.is_empty(), "expected error: '\\u{{}}' with empty braces, got none");
}

// §13  <attribute> — the name after '@' must be an identifier, not a number
#[test]
fn error_attribute_numeric_name() {
    let e = errs("@123 fn foo() -> Int { 0 }");
    assert!(!e.is_empty(), "expected error: attribute name is a number, got none");
}

#[test]
fn literal_int_hex() {
    let e = expr("0xFF");
    assert!(matches!(e, Expr::Literal(Literal::Int(255))));
}

#[test]
fn literal_int_binary() {
    let e = expr("0b1010");
    assert!(matches!(e, Expr::Literal(Literal::Int(10))));
}
