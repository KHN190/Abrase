// src/ty.rs

#[derive(Debug, PartialEq, Clone)]
pub enum Ownership {
    Copy,
    Move,
    Share,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Effect {
    Total,
    Async,
    Exn(Box<Type>),
    Alloc,
    Nondet,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    String,
    Unit,
    Never,
    Named(String),
    Generic { name: String, args: Vec<Type> },
    Tuple(Vec<Type>),
    Reference { is_mut: bool, inner: Box<Type> },
    Function { params: Vec<Type>, effects: Vec<Effect>, ret: Box<Type> },
    Unknown,
}

impl Type {
    pub fn ownership(&self) -> Ownership {
        match self {
            Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit | Type::Never => {
                Ownership::Copy
            }
            Type::String => Ownership::Move,
            Type::Reference { .. } => Ownership::Copy,
            Type::Tuple(tys) => {
                if tys.iter().all(|t| t.ownership() == Ownership::Copy) {
                    Ownership::Copy
                } else {
                    Ownership::Move
                }
            }
            Type::Generic { args, .. } => {
                if args.iter().all(|t| t.ownership() == Ownership::Copy) {
                    Ownership::Copy
                } else {
                    Ownership::Move
                }
            }
            Type::Function { .. } => Ownership::Copy,
            Type::Named(_) | Type::Unknown => Ownership::Move,
        }
    }
}

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