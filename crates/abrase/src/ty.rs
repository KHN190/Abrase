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
    Exn(Box<Type>),
    Alloc,
    Io,
    Nondet,
    UserEffect(String),
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
    Shared { inner: Box<Type>, region: Option<String> },
    Unknown,
}

// Single source for the builtin scalar name ↔ variant bijection. `from_name`
// and `builtin_name` both derive from this list, so the two directions stay
// inverse by construction (was two hand-written maps that drifted on `Never`).
pub const BUILTIN_SCALARS: &[(&str, Type)] = &[
    ("Int", Type::Int),
    ("Float", Type::Float),
    ("Bool", Type::Bool),
    ("Char", Type::Char),
    ("String", Type::String),
    ("Unit", Type::Unit),
    ("Never", Type::Never),
];

impl Type {
    pub fn from_name(n: &str) -> Type {
        for (name, t) in BUILTIN_SCALARS {
            if *name == n { return t.clone(); }
        }
        Type::Named(n.to_string())
    }

    pub fn builtin_name(&self) -> Option<String> {
        for (name, t) in BUILTIN_SCALARS {
            if t == self { return Some((*name).to_string()); }
        }
        match self {
            Type::Named(n) => Some(n.clone()),
            _ => None,
        }
    }

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
            Type::Generic { name, args } => {
                if name == "Shared" {
                    Ownership::Share
                } else if args.iter().all(|t| t.ownership() == Ownership::Copy) {
                    Ownership::Copy
                } else {
                    Ownership::Move
                }
            }
            Type::Shared { .. } => Ownership::Share,
            Type::Function { .. } => Ownership::Copy,
            Type::Named(name) if name == "Share" || name == "Addr" => Ownership::Copy,
            Type::Named(_) => Ownership::Move,
            // Conservative: treat Unknown as Copy to avoid spurious moves before inference.
            Type::Unknown => Ownership::Copy,
        }
    }
}
