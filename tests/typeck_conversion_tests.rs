use ect::ast::{self};
use ect::ty::Type;
use ect::typeck::Checker;

// Type Conversion & Compatibility

#[test]
fn verify_convert_primitive_types() {
    let checker = Checker::new();
    assert_eq!(checker.convert_type(&ast::Type::Named("Int".into())), Type::Int);
    assert_eq!(checker.convert_type(&ast::Type::Named("Float".into())), Type::Float);
    assert_eq!(checker.convert_type(&ast::Type::Named("Bool".into())), Type::Bool);
    assert_eq!(checker.convert_type(&ast::Type::Named("Char".into())), Type::Char);
    assert_eq!(checker.convert_type(&ast::Type::Named("String".into())), Type::String);
    assert_eq!(checker.convert_type(&ast::Type::Named("Unit".into())), Type::Unit);
}

#[test]
fn verify_convert_qualified_types() {
    let checker = Checker::new();
    let qualified = ast::Type::Qualified(vec!["std".into(), "io".into(), "Error".into()]);
    let converted = checker.convert_type(&qualified);
    assert_eq!(converted, Type::Named("std.io.Error".into()));
}

#[test]
fn verify_convert_generic_types() {
    let checker = Checker::new();
    let generic = ast::Type::Generic {
        name: "List".into(),
        args: vec![ast::Type::Named("Int".into())],
    };
    let converted = checker.convert_type(&generic);
    assert!(format!("{:?}", converted).contains("List"));
}

#[test]
fn verify_convert_array_with_size() {
    let checker = Checker::new();
    let array = ast::Type::Array {
        elem: Box::new(ast::Type::Named("Int".into())),
        size: 16,
    };
    let converted = checker.convert_type(&array);
    assert!(format!("{:?}", converted).contains("16"));
}

#[test]
fn verify_convert_tuple_types() {
    let checker = Checker::new();
    let tuple = ast::Type::Tuple(vec![
        ast::Type::Named("Int".into()),
        ast::Type::Named("Bool".into()),
    ]);
    let converted = checker.convert_type(&tuple);
    assert_eq!(converted, Type::Tuple(vec![Type::Int, Type::Bool]));
}

#[test]
fn verify_convert_reference_types() {
    let checker = Checker::new();
    let reference = ast::Type::Reference {
        is_mut: false,
        inner: Box::new(ast::Type::Named("String".into())),
        region: None,
    };
    let converted = checker.convert_type(&reference);
    assert_eq!(
        converted,
        Type::Reference {
            is_mut: false,
            inner: Box::new(Type::String),
        }
    );
}

#[test]
fn verify_convert_mutable_reference() {
    let checker = Checker::new();
    let reference = ast::Type::Reference {
        is_mut: true,
        inner: Box::new(ast::Type::Named("Int".into())),
        region: None,
    };
    let converted = checker.convert_type(&reference);
    assert_eq!(
        converted,
        Type::Reference {
            is_mut: true,
            inner: Box::new(Type::Int),
        }
    );
}

#[test]
fn verify_convert_function_types() {
    let checker = Checker::new();
    let function = ast::Type::Function {
        params: vec![
            ast::Type::Named("Int".into()),
            ast::Type::Named("Bool".into()),
        ],
        effects: vec![],
        ret: Box::new(ast::Type::Named("String".into())),
    };
    let converted = checker.convert_type(&function);
    assert_eq!(
        converted,
        Type::Function {
            params: vec![Type::Int, Type::Bool],
            effects: vec![],
            ret: Box::new(Type::String),
        }
    );
}

#[test]
fn verify_types_compatible_same_types() {
    let checker = Checker::new();
    assert!(checker.types_compatible(&Type::Int, &Type::Int));
    assert!(checker.types_compatible(&Type::Bool, &Type::Bool));
    assert!(checker.types_compatible(&Type::String, &Type::String));
}

#[test]
fn verify_types_compatible_with_unknown() {
    let checker = Checker::new();
    assert!(checker.types_compatible(&Type::Int, &Type::Unknown));
    assert!(checker.types_compatible(&Type::Unknown, &Type::Int));
    assert!(checker.types_compatible(&Type::Unknown, &Type::Unknown));
}

#[test]
fn verify_types_compatible_different_types() {
    let checker = Checker::new();
    assert!(!checker.types_compatible(&Type::Int, &Type::Bool));
    assert!(!checker.types_compatible(&Type::String, &Type::Float));
}

#[test]
fn verify_types_compatible_tuples() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Bool]);
    assert!(checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_tuple_length_mismatch() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int]);
    assert!(!checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_tuple_element_mismatch() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::String]);
    assert!(!checker.types_compatible(&tuple1, &tuple2));
}

#[test]
fn verify_types_compatible_references() {
    let checker = Checker::new();
    let ref1 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref2 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    assert!(checker.types_compatible(&ref1, &ref2));
}

#[test]
fn verify_types_compatible_reference_mutability_mismatch() {
    let checker = Checker::new();
    let ref_immut = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref_mut = Type::Reference { is_mut: true, inner: Box::new(Type::Int) };
    assert!(!checker.types_compatible(&ref_immut, &ref_mut));
}

#[test]
fn verify_types_compatible_functions() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    assert!(checker.types_compatible(&fn1, &fn2));
}

#[test]
fn verify_types_compatible_function_param_mismatch() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int, Type::Bool],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int, Type::String],
        effects: vec![],
        ret: Box::new(Type::String),
    };
    assert!(!checker.types_compatible(&fn1, &fn2));
}

#[test]
fn verify_unify_same_types() {
    let checker = Checker::new();
    let result = checker.unify_types(&Type::Int, &Type::Int);
    assert_eq!(result, Some(Type::Int));
}

#[test]
fn verify_unify_with_unknown() {
    let checker = Checker::new();
    assert_eq!(checker.unify_types(&Type::Int, &Type::Unknown), Some(Type::Int));
    assert_eq!(checker.unify_types(&Type::Unknown, &Type::Bool), Some(Type::Bool));
}

#[test]
fn verify_unify_tuples() {
    let checker = Checker::new();
    let tuple1 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::Bool]);
    let unified = checker.unify_types(&tuple1, &tuple2);
    assert_eq!(unified, Some(tuple1));
}

#[test]
fn verify_unify_incompatible_types() {
    let checker = Checker::new();
    let result = checker.unify_types(&Type::Int, &Type::Bool);
    assert_eq!(result, None);
}

#[test]
fn verify_unify_references() {
    let checker = Checker::new();
    let ref1 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let ref2 = Type::Reference { is_mut: false, inner: Box::new(Type::Int) };
    let unified = checker.unify_types(&ref1, &ref2);
    assert!(unified.is_some());
}

#[test]
fn verify_unify_functions() {
    let checker = Checker::new();
    let fn1 = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Bool),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int],
        effects: vec![],
        ret: Box::new(Type::Bool),
    };
    let unified = checker.unify_types(&fn1, &fn2);
    assert!(unified.is_some());
}

#[test]
fn verify_is_assignable() {
    let checker = Checker::new();
    assert!(checker.is_assignable(&Type::Int, &Type::Int));
    assert!(checker.is_assignable(&Type::Int, &Type::Unknown));
    assert!(checker.is_assignable(&Type::Unknown, &Type::Bool));
    assert!(!checker.is_assignable(&Type::Int, &Type::Bool));
}