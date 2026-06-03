use abrase::ty::{Type, Ownership};

#[test]
fn verify_ownership_derivation() {
    assert_eq!(Type::Int.ownership(), Ownership::Copy);
    assert_eq!(Type::String.ownership(), Ownership::Move);
    assert_eq!(
        Type::Tuple(vec![Type::Int, Type::Bool]).ownership(),
        Ownership::Copy
    );
    assert_eq!(
        Type::Tuple(vec![Type::Int, Type::String]).ownership(),
        Ownership::Move
    );
    assert_eq!(
        Type::Reference { is_mut: false, inner: Box::new(Type::String) }.ownership(),
        Ownership::Copy
    );
}
