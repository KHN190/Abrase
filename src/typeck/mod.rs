// src/typeck.rs

use std::collections::HashMap;
use crate::ast::{self, Span, Spanned};
use crate::ty::{Ownership, Type};

fn elem_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(n) => n.clone(),
        _ => "?".into(),
    }
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub context: Vec<String>,
}

impl TypeError {
    pub fn display(&self) -> String {
        let mut output = format!("TypeError at line {}, col {}: {}", self.span.line, self.span.col, self.message);
        if !self.context.is_empty() {
            output.push_str("\n  Context stack:");
            for (i, ctx) in self.context.iter().enumerate() {
                output.push_str(&format!("\n    {}: {}", i + 1, ctx));
            }
        }
        output
    }
}

#[derive(Clone)]
struct VarMeta {
    ty: Type,
    is_mut: bool,
    is_moved: bool,
    defined_at: Span,
    moved_at: Option<Span>,
    immut_borrow_count: usize,
    mut_borrow_active: bool,
}

#[derive(Clone)]
pub struct Scope {
    vars: HashMap<String, VarMeta>,
}

pub struct Checker {
    scopes: Vec<Scope>,
    pub errors: Vec<TypeError>,
    context_stack: Vec<String>,
    loop_depth: usize,
    active_effects: Vec<String>,
    effect_stack: Vec<Vec<String>>,

    // Type Environment
    fn_registry: HashMap<String, (Vec<Type>, Type)>,
    type_registry: HashMap<String, ast::TypeBody>,
    variant_registry: HashMap<String, Vec<String>>, // type_name -> [case_names]
    const_registry: HashMap<String, Type>,

    // Import Namespace Mapping
    imported_names: HashMap<String, (Vec<String>, String)>, // alias/name -> (module_path, original_name)
    import_collisions: std::collections::HashSet<String>,

    // Ownership & Borrowing
    borrow_stack: Vec<(String, bool)>,

    // Effects System
    effect_registry: HashMap<String, Vec<String>>,
    effect_alias_registry: HashMap<String, Vec<crate::ty::Effect>>,
    current_effects: Vec<crate::ty::Effect>,

    // Type Ownership Attributes
    ownership_registry: HashMap<String, Ownership>,

    // Effect Unification & Inference
    fn_declared_effects: Vec<crate::ty::Effect>,
    fn_required_effects: Vec<crate::ty::Effect>,

    // Effect Shadowing, Propagation & Scope Semantics
    handled_effects: Vec<String>,
    unhandled_effects: Vec<crate::ty::Effect>,

    // Generics & Trait Constraints
    trait_registry: HashMap<String, Vec<String>>,
    impl_registry: HashMap<(String, String), bool>,
    generic_params: HashMap<String, Vec<String>>,
    trait_bounds: HashMap<String, Vec<String>>,

    // Region Escape Analysis & Advanced Borrow Checking
    region_stack: Vec<String>,
    reference_lifetimes: HashMap<String, String>,
    pattern_borrows: HashMap<String, Vec<String>>,

    // Pattern Matching Analysis (Exhaustiveness & Unreachability)
    covered_patterns: Vec<String>,
    unreachable_patterns: Vec<usize>,

    // Visibility & Module Scoping
    current_module: Vec<String>,
    public_items: std::collections::HashSet<String>,
    private_items: std::collections::HashSet<String>,

    // Qualified Name Resolution
    qualified_names: HashMap<String, Vec<Vec<String>>>,

    // Generic Variance
    variance_registry: HashMap<String, Vec<crate::ty::Variance>>,
    named_subtype_registry: HashMap<String, Vec<String>>,

    // Const Effect Checking
    function_effects: HashMap<String, Vec<ast::EffectItem>>,
    const_vars: std::collections::HashSet<String>,
    op_effects: HashMap<String, Vec<ast::EffectItem>>,

    // Effect Operations (effect_name::op_name -> Type)
    effect_ops_registry: HashMap<String, Type>,

    // Type Aliases
    type_alias_registry: HashMap<String, Type>,
}


pub mod types;
pub mod imports;
pub mod ownership;
pub mod effects;
pub mod traits;
pub mod pattern;
pub mod privacy;
pub mod records;
pub mod interpolation;
pub mod expr;
pub mod decl;

impl Checker {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope { vars: HashMap::new() }],
            errors: Vec::new(),
            context_stack: Vec::new(),
            loop_depth: 0,
            active_effects: Vec::new(),
            effect_stack: vec![Vec::new()],
            fn_registry: HashMap::new(),
            type_registry: HashMap::new(),
            variant_registry: HashMap::new(),
            const_registry: HashMap::new(),
            borrow_stack: Vec::new(),
            effect_registry: HashMap::new(),
            effect_alias_registry: HashMap::new(),
            current_effects: Vec::new(),
            ownership_registry: HashMap::new(),
            fn_declared_effects: Vec::new(),
            fn_required_effects: Vec::new(),
            handled_effects: Vec::new(),
            unhandled_effects: Vec::new(),
            trait_registry: HashMap::new(),
            impl_registry: HashMap::new(),
            generic_params: HashMap::new(),
            trait_bounds: HashMap::new(),
            region_stack: Vec::new(),
            reference_lifetimes: HashMap::new(),
            pattern_borrows: HashMap::new(),
            covered_patterns: Vec::new(),
            unreachable_patterns: Vec::new(),
            current_module: vec!["root".into()],
            public_items: std::collections::HashSet::new(),
            private_items: std::collections::HashSet::new(),
            qualified_names: HashMap::new(),
            variance_registry: {
                let mut m = HashMap::new();
                use crate::ty::Variance::*;
                m.insert("List".into(),   vec![Covariant]);
                m.insert("Option".into(), vec![Covariant]);
                m.insert("Array".into(),  vec![Covariant]);
                m.insert("Result".into(), vec![Covariant, Covariant]);
                m.insert("Cell".into(),   vec![Invariant]);
                m.insert("Fn".into(),     vec![Contravariant, Covariant]);
                m
            },
            named_subtype_registry: HashMap::new(),
            function_effects: HashMap::new(),
            const_vars: std::collections::HashSet::new(),
            op_effects: HashMap::new(),
            effect_ops_registry: HashMap::new(),
            type_alias_registry: HashMap::new(),
            imported_names: HashMap::new(),
            import_collisions: std::collections::HashSet::new(),
        }
    }
    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope { vars: HashMap::new() });
    }
    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }
    pub fn display_errors(&self) -> String {
        if self.errors.is_empty() {
            return "No type errors".to_string();
        }
        let mut output = format!("Found {} type error(s):\n", self.errors.len());
        for (i, error) in self.errors.iter().enumerate() {
            output.push_str(&format!("\n{}: {}", i + 1, error.display()));
        }
        output
    }
    pub fn insert_var(&mut self, name: String, ty: Type, is_mut: bool, defined_at: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name, VarMeta {
                ty,
                is_mut,
                is_moved: false,
                defined_at,
                moved_at: None,
                immut_borrow_count: 0,
                mut_borrow_active: false,
            });
        }
    }

    pub fn report_error(&mut self, message: String, span: Span) -> Type {
        self.errors.push(TypeError {
            message,
            span,
            context: self.context_stack.clone(),
        });
        Type::Unknown
    }

    pub fn resolve_var_in_scopes(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.vars.get(name) {
                return Some(var.ty.clone());
            }
        }
        None
    }

    pub fn get_field_type(&self, type_name: &str, field_name: &str) -> Option<Type> {
        if let Some(body) = self.type_registry.get(type_name) {
            if let ast::TypeBody::Record(fields) = body {
                for field in fields {
                    if field.name == field_name {
                        return Some(self.convert_type(&field.ty));
                    }
                }
            }
        }
        None
    }
}
