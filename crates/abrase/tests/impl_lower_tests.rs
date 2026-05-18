use abrase::ast::*;
use abrase::compiler::impls::ImplLowering;
use abrase::lexer::Lexer;
use abrase::parser::Parser;

fn lower_src(src: &str) -> ImplLowering {
    let mut p = Parser::new(Lexer::new(src));
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let mut il = ImplLowering::new();
    il.lower(&ast);
    il
}

#[test]
fn inherent_impl_lifts_methods() {
    let il = lower_src("impl Counter { fn zero() -> Int { 0 } }");
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    assert_eq!(il.synthetic_fns.len(), 1);
    assert_eq!(il.synthetic_fns[0].name, "Counter__zero");
    assert_eq!(
        il.method_dispatch.get(&("Counter".into(), "zero".into())),
        Some(&"Counter__zero".to_string())
    );
}

#[test]
fn trait_impl_mangles_with_trait_prefix() {
    let il = lower_src(
        "trait Show { fn show(self) -> Int } \
         impl Show for Counter { fn show(self) -> Int { 0 } }",
    );
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    let lifted = il.synthetic_fns.iter().find(|f| f.name == "Show__Counter__show");
    assert!(lifted.is_some(), "expected mangled trait method, got {:?}",
        il.synthetic_fns.iter().map(|f| &f.name).collect::<Vec<_>>());
}

#[test]
fn generic_inherent_impl_carries_generics() {
    let il = lower_src("impl<T> Container<T> { fn empty() -> Int { 0 } }");
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    assert_eq!(il.synthetic_fns.len(), 1);
    let f = &il.synthetic_fns[0];
    assert_eq!(f.name, "Container__empty");
    assert_eq!(f.generics.len(), 1);
    assert_eq!(f.generics[0].name, "T");
    assert_eq!(
        il.method_dispatch.get(&("Container".into(), "empty".into())),
        Some(&"Container__empty".to_string())
    );
}

#[test]
fn generic_impl_substitutes_self_with_parameterized_type() {
    let il = lower_src("impl<T> Container<T> { fn id(self) -> Self { self } }");
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    let f = &il.synthetic_fns[0];
    let self_param_ty = match &f.params[0] {
        Param::Named { ty, .. } => ty.clone(),
        _ => panic!("expected named self param"),
    };
    assert!(
        matches!(&self_param_ty, Type::Generic { name, args }
            if name == "Container" && args.len() == 1
            && matches!(&args[0], Type::Named(n) if n == "T")),
        "self should be Container<T>, got {:?}", self_param_ty
    );
    let ret_ty = f.return_type.clone().expect("return type");
    assert!(
        matches!(&ret_ty, Type::Generic { name, args }
            if name == "Container" && args.len() == 1
            && matches!(&args[0], Type::Named(n) if n == "T")),
        "return Self should resolve to Container<T>, got {:?}", ret_ty
    );
}

#[test]
fn generic_trait_impl_combines_impl_and_method_generics() {
    let il = lower_src(
        "trait Map { fn map(self) -> Int } \
         impl<T> Map for Container<T> { fn map<U>(self) -> Int { 0 } }",
    );
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    let f = il.synthetic_fns.iter().find(|f| f.name == "Map__Container__map").unwrap();
    let names: Vec<&str> = f.generics.iter().map(|g| g.name.as_str()).collect();
    assert!(names.contains(&"T") && names.contains(&"U"),
        "expected T and U in generics, got {:?}", names);
}

#[test]
fn impl_where_clause_propagated_to_lifted_method() {
    let il = lower_src(
        "trait Show { fn show(self) -> Int } \
         impl<T> Show for Container<T> where T: Show { fn show(self) -> Int { 0 } }",
    );
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    let f = il.synthetic_fns.iter().find(|f| f.name == "Show__Container__show").unwrap();
    assert_eq!(f.where_clause.len(), 1, "expected one where bound, got {:?}", f.where_clause);
    let bound = &f.where_clause[0];
    assert!(matches!(&bound.ty, Type::Named(n) if n == "T"),
        "expected bound on T, got {:?}", bound.ty);
}

#[test]
fn impl_where_merges_with_method_where() {
    let il = lower_src(
        "trait Show { fn show(self) -> Int } \
         impl<T> Show for Container<T> where T: Show { \
             fn show<U>(self) -> Int where U: Show { 0 } \
         }",
    );
    assert!(il.errors.is_empty(), "unexpected errors: {:?}", il.errors);
    let f = il.synthetic_fns.iter().find(|f| f.name == "Show__Container__show").unwrap();
    assert_eq!(f.where_clause.len(), 2,
        "expected merged impl + method where bounds, got {:?}", f.where_clause);
    let bound_tys: Vec<&Type> = f.where_clause.iter().map(|b| &b.ty).collect();
    assert!(bound_tys.iter().any(|t| matches!(t, Type::Named(n) if n == "T")));
    assert!(bound_tys.iter().any(|t| matches!(t, Type::Named(n) if n == "U")));
}

#[test]
fn impl_without_where_leaves_method_where_empty() {
    let il = lower_src("impl<T> Container<T> { fn empty() -> Int { 0 } }");
    let f = &il.synthetic_fns[0];
    assert!(f.where_clause.is_empty(),
        "no impl where + no method where = empty, got {:?}", f.where_clause);
}
