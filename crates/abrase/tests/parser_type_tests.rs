use abrase::ast::*;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn ty(input: &str) -> Type {
    let mut p = Parser::new(Lexer::new(input));
    p.parse_type().expect("type parse failed")
}

#[test]
fn test_type_named() {
    assert_eq!(ty("Int"), Type::Named("Int".into()));
    assert_eq!(ty("Self"), Type::Named("Self".into()));
}

#[test]
fn test_type_generic() {
    assert_eq!(ty("List<Int>"), Type::Generic { name: "List".into(), args: vec![Type::Named("Int".into())] });
    assert_eq!(ty("Result<T, E>"), Type::Generic {
        name: "Result".into(),
        args: vec![Type::Named("T".into()), Type::Named("E".into())],
    });
}

#[test]
fn test_type_qualified() {
    assert_eq!(ty("io.Error"), Type::Qualified(vec!["io".into(), "Error".into()]));
    assert_eq!(ty("a.b.c"), Type::Qualified(vec!["a".into(), "b".into(), "c".into()]));
}

#[test]
fn test_type_array() {
    assert_eq!(ty("[Int; 16]"), Type::Array { elem: Box::new(Type::Named("Int".into())), size: 16 });
    assert_eq!(ty("[Bool; 4]"), Type::Array { elem: Box::new(Type::Named("Bool".into())), size: 4 });
}

#[test]
fn test_type_tuple() {
    assert_eq!(ty("()"), Type::Tuple(vec![]));
    assert_eq!(ty("(Int,)"), Type::Tuple(vec![Type::Named("Int".into())]));
    assert_eq!(ty("(Int, Bool)"), Type::Tuple(vec![Type::Named("Int".into()), Type::Named("Bool".into())]));
    assert_eq!(ty("(Int, Bool, String)"), Type::Tuple(vec![
        Type::Named("Int".into()), Type::Named("Bool".into()), Type::Named("String".into()),
    ]));
}

#[test]
fn test_type_reference() {
    assert_eq!(ty("&Int"), Type::Reference { is_mut: false, inner: Box::new(Type::Named("Int".into())), region: None });
    assert_eq!(ty("&mut String"), Type::Reference { is_mut: true, inner: Box::new(Type::Named("String".into())), region: None });
    assert_eq!(ty("&Int in r"), Type::Reference { is_mut: false, inner: Box::new(Type::Named("Int".into())), region: Some("r".into()) });
    assert_eq!(ty("&mut T in heap"), Type::Reference { is_mut: true, inner: Box::new(Type::Named("T".into())), region: Some("heap".into()) });
}

#[test]
fn test_type_function() {
    assert_eq!(ty("() -> String"), Type::Function {
        params: vec![], effects: vec![], ret: Box::new(Type::Named("String".into())),
    });
    assert_eq!(ty("(Int) -> Bool"), Type::Function {
        params: vec![Type::Named("Int".into())], effects: vec![], ret: Box::new(Type::Named("Bool".into())),
    });
    assert_eq!(ty("(Int, String) -> Bool"), Type::Function {
        params: vec![Type::Named("Int".into()), Type::Named("String".into())],
        effects: vec![],
        ret: Box::new(Type::Named("Bool".into())),
    });
    assert_eq!(ty("(Int) -> Option<String>"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![],
        ret: Box::new(Type::Generic { name: "Option".into(), args: vec![Type::Named("String".into())] }),
    });
}

#[test]
fn test_type_function_effects() {
    assert_eq!(ty("(Int) -> <exn> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![EffectItem { name: vec!["exn".into()], arg: None }],
        ret: Box::new(Type::Named("String".into())),
    });
    assert_eq!(ty("(Int) -> <exn, io> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![
            EffectItem { name: vec!["exn".into()], arg: None },
            EffectItem { name: vec!["io".into()],  arg: None },
        ],
        ret: Box::new(Type::Named("String".into())),
    });
    assert_eq!(ty("(Int) -> <exn<E>> String"), Type::Function {
        params: vec![Type::Named("Int".into())],
        effects: vec![EffectItem { name: vec!["exn".into()], arg: Some(Box::new(Type::Named("E".into()))) }],
        ret: Box::new(Type::Named("String".into())),
    });
}

#[test]
fn test_fn_type_with_effects() {
    assert_eq!(
        ty("() -> <io> String"),
        Type::Function {
            params: vec![],
            effects: vec![EffectItem { name: vec!["io".into()], arg: None }],
            ret: Box::new(Type::Named("String".into())),
        }
    );
    assert_eq!(
        ty("(Int) -> <io, exn> Bool"),
        Type::Function {
            params: vec![Type::Named("Int".into())],
            effects: vec![
                EffectItem { name: vec!["io".into()], arg: None },
                EffectItem { name: vec!["exn".into()], arg: None },
            ],
            ret: Box::new(Type::Named("Bool".into())),
        }
    );
    assert_eq!(
        ty("() -> <exn<String>> Int"),
        Type::Function {
            params: vec![],
            effects: vec![EffectItem { name: vec!["exn".into()], arg: Some(Box::new(Type::Named("String".into()))) }],
            ret: Box::new(Type::Named("Int".into())),
        }
    );
}
