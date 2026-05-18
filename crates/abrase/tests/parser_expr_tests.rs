use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::{Parser, Precedence};

fn parse_errs(input: &str) -> Vec<String> {
    let mut p = Parser::new(Lexer::new(input));
    let _ = p.parse_program();
    p.errors.into_iter().map(|e| e.message).collect()
}

fn fn_body_expr(input: &str) -> Expr {
    let mut p = Parser::new(Lexer::new(input));
    let decls = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors.iter().map(|e| &e.message).collect::<Vec<_>>());
    let fn_decl = decls.into_iter().find_map(|d| match d {
        Decl::Fn(f) => Some(f),
        _ => None,
    }).expect("expected a function declaration");
    fn_decl.body.ret.map(|b| b.node).or_else(|| {
        fn_decl.body.stmts.last().and_then(|s| match &s.node {
            Stmt::Expr(e) => Some(e.node.clone()),
            _ => None,
        })
    }).expect("expected an expression in fn body")
}

#[test]
fn test_expr_if() {
    let input = "if true { 1 } else { 2 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::If { condition, consequence: _, alternative } = expr.node {
        assert_eq!(condition.node, Expr::Literal(Literal::Bool(true)));
        assert!(alternative.is_some());
    } else {
        panic!("Expected If expression");
    }
}

#[test]
fn test_expr_if_without_else() {
    let input = "if x > 5 { 10 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::If { condition, alternative, .. } = expr.node {
        assert!(matches!(condition.node, Expr::Binary { .. }));
        assert!(alternative.is_none());
    } else {
        panic!("Expected If expression");
    }
}

#[test]
fn test_expr_if_else_if_chain() {
    let input = "if x { 1 } else if y { 2 } else { 3 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::If { alternative: Some(alt), .. } = expr.node {
        assert!(matches!(alt.node, Expr::If { .. }));
    } else {
        panic!("Expected If expression with else if");
    }
}

#[test]
fn test_expr_match() {
    let input = "match x { A => 1, B => 2 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Match { scrutinee, arms } = expr.node {
        assert_eq!(scrutinee.node, Expr::Identifier("x".into()));
        assert_eq!(arms.len(), 2);
    } else {
        panic!("Expected Match expression");
    }
}

#[test]
fn test_expr_match_with_guard() {
    let input = "match x { 1 if x > 0 => true, _ => false }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Match { scrutinee: _, arms } = expr.node {
        assert_eq!(arms.len(), 2);
        assert!(arms[0].guard.is_some());
        assert!(arms[1].guard.is_none());
    } else {
        panic!("Expected Match expression");
    }
}

#[test]
fn test_expr_match_block_body() {
    let input = "match x { A => { print(1); 1 }, B => 2 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Match { arms, .. } = expr.node {
        assert_eq!(arms.len(), 2);
    } else {
        panic!("Expected Match expression");
    }
}

#[test]
fn test_expr_for() {
    let input = "for x in items { x }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::For { pattern, iter, body } = expr.node {
        assert_eq!(pattern.node, Pattern::Bind("x".into()));
        assert_eq!(iter.node, Expr::Identifier("items".into()));
        assert_eq!(body.stmts.len(), 0);
    } else {
        panic!("Expected For expression");
    }
}

#[test]
fn test_expr_for_tuple_destructure() {
    let input = "for (x, y) in pairs { x + y }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::For { pattern, iter, .. } = expr.node {
        assert!(matches!(pattern.node, Pattern::Tuple(_)));
        assert_eq!(iter.node, Expr::Identifier("pairs".into()));
    } else {
        panic!("Expected For expression");
    }
}

#[test]
fn test_expr_while() {
    let input = "while true { 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::While { condition, body } = expr.node {
        assert_eq!(condition.node, Expr::Literal(Literal::Bool(true)));
        assert_eq!(body.stmts.len(), 0);
    } else {
        panic!("Expected While expression");
    }
}

#[test]
fn test_expr_while_complex_condition() {
    let input = "while x < 10 { x = x + 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::While { condition, body } = expr.node {
        assert!(matches!(condition.node, Expr::Binary { .. }));
        assert!(body.stmts.len() > 0 || body.ret.is_some());
    } else {
        panic!("Expected While expression");
    }
}

#[test]
fn test_expr_loop() {
    let input = "loop { break }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Loop { body } = expr.node {
        assert_eq!(body.stmts.len(), 0);
        assert!(matches!(body.ret, Some(r) if matches!(r.node, Expr::Break(_))));
    } else {
        panic!("Expected Loop expression");
    }
}

#[test]
fn test_expr_loop_with_continue() {
    let input = "loop { if x { continue } }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Loop { body } = expr.node {
        assert!(matches!(body.ret, Some(r) if matches!(r.node, Expr::If { .. })));
    } else {
        panic!("Expected Loop expression");
    }
}

#[test]
fn test_expr_closure() {
    let input = "|x| x + 1";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Closure { params, .. } = expr.node {
        assert_eq!(params.len(), 1);
    } else {
        panic!("Expected Closure expression");
    }
}

#[test]
fn test_expr_region() {
    let input = "region r { 1 }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Region { label, .. } = expr.node {
        assert_eq!(label, Some("r".into()));
    } else {
        panic!("Expected Region expression");
    }
}

#[test]
fn test_expr_region_without_label() {
    let input = "region { let x = 5; x }";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Region { label, .. } = expr.node {
        assert_eq!(label, None);
    } else {
        panic!("Expected Region expression");
    }
}

#[test]
fn test_operator_precedence() {
    let input = "1 + 2 * 3";
    let mut parser = Parser::new(Lexer::new(input));
    let expr = parser.parse_expr(Precedence::Lowest);
    if let Expr::Binary { op, left, right } = expr.node {
        assert_eq!(op, BinaryOp::Add);
        assert_eq!(left.node, Expr::Literal(Literal::Int(1)));
        if let Expr::Binary { op: op_inner, left: l_inner, right: r_inner } = &right.node {
            assert_eq!(*op_inner, BinaryOp::Mul);
            assert_eq!(l_inner.node, Expr::Literal(Literal::Int(2)));
            assert_eq!(r_inner.node, Expr::Literal(Literal::Int(3)));
        } else {
            panic!("Right side of addition should be multiplication");
        }
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
fn test_if_else_inside_fn_body_keeps_alternative() {
    let input = "fn f(n: Int) -> Int { if n <= 1 { n } else { n - 1 } }";
    let body = fn_body_expr(input);
    let Expr::If { alternative, .. } = body else {
        panic!("expected If at fn body, got {:?}", body);
    };
    assert!(alternative.is_some(), "else branch was dropped");
}

#[test]
fn test_recursive_fn_with_else_preserves_base_case() {
    let input = "
        fn fib(n: Int) -> Int {
            if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::If { consequence, alternative, .. } = body else {
        panic!("expected If at fn body, got {:?}", body);
    };
    let Expr::Block(cons_block) = &consequence.node else {
        panic!("expected Block consequence");
    };
    let Some(cons_ret) = &cons_block.ret else {
        panic!("expected consequence to have a tail expression");
    };
    assert!(matches!(cons_ret.node, Expr::Identifier(_)));
    let alt = alternative.expect("else branch missing — base case would be lost");
    let Expr::Block(alt_block) = &alt.node else {
        panic!("expected Block alternative");
    };
    let alt_ret = alt_block.ret.as_ref().expect("expected alternative tail expr");
    assert!(matches!(alt_ret.node, Expr::Binary { op: BinaryOp::Add, .. }));
}

#[test]
fn test_nested_if_else_chain_inside_fn_body() {
    let input = "
        fn classify(n: Int) -> Int {
            if n < 0 {
                0
            } else {
                if n == 0 { 1 } else { if n < 10 { 2 } else { 3 } }
            }
        }
    ";
    let body = fn_body_expr(input);
    let mut current = body;
    for depth in 0..3 {
        let Expr::If { alternative, .. } = current else {
            panic!("expected If at depth {}", depth);
        };
        let alt = alternative.unwrap_or_else(|| panic!("else dropped at depth {}", depth));
        let mut inner = alt.node;
        while let Expr::Block(b) = inner {
            inner = b.ret.expect("expected tail expr").node;
        }
        current = inner;
    }
}

#[test]
fn test_match_newline_separated_arms() {
    let input = "
        fn pick(x: Int) -> Int {
            match x {
                0 => 10
                1 => 20
                _ => 30
            }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::Match { arms, .. } = body else {
        panic!("expected Match at fn body");
    };
    assert_eq!(arms.len(), 3);
}

#[test]
fn test_nested_match_block_body_arm() {
    let input = "
        fn quadrant(x: Int, y: Int) -> Int {
            match x {
                0 => 0
                1 => match y {
                    1 => 1
                    _ => 0
                }
                _ => 0
            }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::Match { arms, .. } = body else {
        panic!("expected outer Match");
    };
    assert_eq!(arms.len(), 3, "outer match should keep all three arms");
    assert!(matches!(arms[1].body.node, Expr::Match { .. }),
        "expected nested Match as arm body, got {:?}", arms[1].body.node);
}

#[test]
fn test_match_block_body_with_following_arm_no_comma() {
    let input = "
        fn pick(x: Int) -> Int {
            match x {
                0 => { let a = 1; a }
                _ => 0
            }
        }
    ";
    let body = fn_body_expr(input);
    let Expr::Match { arms, .. } = body else {
        panic!("expected Match");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].body.node, Expr::Block(_)));
}

#[test]
fn test_plus_assign_desugars_to_assign_of_add() {
    let input = "a += 1";
    let mut p = Parser::new(Lexer::new(input));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Binary { op: BinaryOp::Assign, left, right } = expr.node {
        assert!(matches!(left.node, Expr::Identifier(ref n) if n == "a"));
        if let Expr::Binary { op: BinaryOp::Add, .. } = right.node {
            // ok
        } else { panic!("expected Add on RHS, got {:?}", right.node); }
    } else { panic!("expected Assign at top, got {:?}", expr.node); }
}

#[test]
fn test_minus_assign_desugars_to_assign_of_sub() {
    let input = "a -= 1";
    let mut p = Parser::new(Lexer::new(input));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Binary { op: BinaryOp::Assign, right, .. } = expr.node {
        assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Sub, .. }));
    } else { panic!("expected Assign with Sub RHS"); }
}

#[test]
fn test_array_repeat_literal() {
    let mut p = Parser::new(Lexer::new("[0; 4]"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::ArrayRepeat { elem, count } = expr.node {
        assert!(matches!(elem.node, Expr::Literal(Literal::Int(0))));
        assert!(matches!(count.node, Expr::Literal(Literal::Int(4))));
    } else { panic!("expected ArrayRepeat, got {:?}", expr.node); }
}

#[test]
fn test_array_list_literal_still_works() {
    let mut p = Parser::new(Lexer::new("[1, 2, 3]"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Array(items) = expr.node {
        assert_eq!(items.len(), 3);
    } else { panic!("expected Array, got {:?}", expr.node); }
}

#[test]
fn test_array_list_trailing_comma() {
    let mut p = Parser::new(Lexer::new("[1, 2, 3,]"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Array(items) = expr.node {
        assert_eq!(items.len(), 3);
    } else { panic!("expected Array"); }
}

#[test]
fn test_paren_unit_literal() {
    let mut p = Parser::new(Lexer::new("()"));
    let expr = p.parse_expr(Precedence::Lowest);
    assert_eq!(expr.node, Expr::Literal(Literal::Unit));
}

#[test]
fn test_paren_single_expr_strips_parens() {
    let mut p = Parser::new(Lexer::new("(1 + 2)"));
    let expr = p.parse_expr(Precedence::Lowest);
    assert!(matches!(expr.node, Expr::Binary { op: BinaryOp::Add, .. }),
        "expected Binary Add, got {:?}", expr.node);
}

#[test]
fn test_paren_two_element_tuple() {
    let mut p = Parser::new(Lexer::new("(1, 2)"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Tuple(elems) = expr.node {
        assert_eq!(elems.len(), 2);
    } else { panic!("expected Tuple, got {:?}", expr.node); }
}

#[test]
fn test_paren_three_element_tuple() {
    let mut p = Parser::new(Lexer::new("(1, 2, 3)"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Tuple(elems) = expr.node {
        assert_eq!(elems.len(), 3);
    } else { panic!("expected Tuple, got {:?}", expr.node); }
}

#[test]
fn test_paren_tuple_trailing_comma() {
    let mut p = Parser::new(Lexer::new("(1, 2,)"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Tuple(elems) = expr.node {
        assert_eq!(elems.len(), 2);
    } else { panic!("expected Tuple, got {:?}", expr.node); }
}

#[test]
fn test_closure_implicit_borrow_default() {
    let mut p = Parser::new(Lexer::new("|x| x + 1"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Closure { is_move, params, .. } = expr.node {
        assert!(!is_move, "default closure must have is_move = false");
        assert_eq!(params.len(), 1);
    } else { panic!("expected Closure, got {:?}", expr.node); }
}

#[test]
fn test_closure_move_keyword_sets_is_move() {
    let mut p = Parser::new(Lexer::new("move |x| x + 1"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Closure { is_move, .. } = expr.node {
        assert!(is_move, "`move |...|` must set is_move = true");
    } else { panic!("expected Closure, got {:?}", expr.node); }
}

#[test]
fn test_closure_with_typed_params() {
    let mut p = Parser::new(Lexer::new("|x: Int, y: Int| x + y"));
    let expr = p.parse_expr(Precedence::Lowest);
    if let Expr::Closure { params, .. } = expr.node {
        assert_eq!(params.len(), 2);
        assert!(params[0].ty.is_some());
        assert!(params[1].ty.is_some());
    } else { panic!("expected Closure"); }
}

#[test]
fn test_expr_string_interp_single_ident() {
    let mut p = Parser::new(Lexer::new("\"answer: {x}\""));
    let expr = p.parse_expr(Precedence::Lowest);
    let parts = match expr.node {
        Expr::Literal(Literal::StringInterp(parts)) => parts,
        other => panic!("expected StringInterp literal, got {:?}", other),
    };
    assert_eq!(parts, vec![
        StringPart::Literal("answer: ".into()),
        StringPart::Interp(vec!["x".into()]),
    ]);
}

#[test]
fn test_expr_string_interp_dotted_path() {
    let mut p = Parser::new(Lexer::new("\"name={user.name}!\""));
    let expr = p.parse_expr(Precedence::Lowest);
    let parts = match expr.node {
        Expr::Literal(Literal::StringInterp(parts)) => parts,
        other => panic!("expected StringInterp literal, got {:?}", other),
    };
    assert_eq!(parts, vec![
        StringPart::Literal("name=".into()),
        StringPart::Interp(vec!["user".into(), "name".into()]),
        StringPart::Literal("!".into()),
    ]);
}

#[test]
fn test_expr_string_no_interp_stays_plain() {
    let mut p = Parser::new(Lexer::new("\"plain\""));
    let expr = p.parse_expr(Precedence::Lowest);
    assert_eq!(expr.node, Expr::Literal(Literal::String("plain".into())));
}

#[test]
fn test_record_literal_rejects_duplicate_field() {
    let errs = parse_errs("fn f() -> Int { Pt { x: 1, x: 2 } }");
    assert!(errs.iter().any(|e| e.contains("Duplicate field")),
            "expected duplicate-field error, got: {:?}", errs);
}

#[test]
fn test_record_literal_distinct_fields_ok() {
    let errs = parse_errs("fn f() -> Int { Pt { x: 1, y: 2 } }");
    assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
}

#[test]
fn test_parser_traps_deeply_nested_input() {
    let opens = "(".repeat(1000);
    let closes = ")".repeat(1000);
    let src = format!("fn f() -> Int {{ {}5{} }}", opens, closes);
    let errs = parse_errs(&src);
    assert!(errs.iter().any(|e| e.contains("nested too deeply")),
            "expected depth-limit error, got: {:?}", errs);
}
