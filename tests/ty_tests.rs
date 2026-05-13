use ect::ast::{self, Span, Spanned};
use ect::ty::{Type, Ownership};

#[cfg(test)]
mod tests {
    use super::*;

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
}