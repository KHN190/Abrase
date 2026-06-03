use crate::ast;
use crate::ast::Span;
use crate::ty::{Ownership, Type};
use super::*;

// pattern-borrow constraints were stringly-typed; classify exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Immut,
    Mut,
    Move,
}

fn at_span(span: Option<Span>) -> String {
    match span {
        Some(s) => format!(" (previous borrow at line {}:{})", s.line, s.col),
        None => String::new(),
    }
}

impl Checker {

    pub fn try_immut_borrow(&mut self, var_name: &str, borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.is_moved {
                    return Err(format!("Cannot borrow '{}': value already moved", var_name));
                }
                if meta.mut_borrow_active {
                    return Err(format!("Cannot immutably borrow '{}': mutable borrow already active{} (it ends at its last use)", var_name, at_span(meta.active_borrow_span)));
                }
                meta.immut_borrow_count += 1;
                if meta.active_borrow_span.is_none() { meta.active_borrow_span = Some(borrow_span); }
                let depth = self.scopes.len();
                self.borrow_stack.push((var_name.to_string(), false, depth));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }

    pub fn try_mut_borrow(&mut self, var_name: &str, borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.is_moved {
                    return Err(format!("Cannot borrow '{}': value already moved", var_name));
                }
                if meta.immut_borrow_count > 0 {
                    return Err(format!("Cannot mutably borrow '{}': immutable borrow already active{} (it ends at its last use)", var_name, at_span(meta.active_borrow_span)));
                }
                if meta.mut_borrow_active {
                    return Err(format!("Cannot mutably borrow '{}': mutable borrow already active{} (it ends at its last use)", var_name, at_span(meta.active_borrow_span)));
                }
                if !meta.is_mut {
                    return Err(format!("Cannot mutably borrow immutable variable '{}'", var_name));
                }
                meta.mut_borrow_active = true;
                meta.active_borrow_span = Some(borrow_span);
                let depth = self.scopes.len();
                self.borrow_stack.push((var_name.to_string(), true, depth));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }

    pub fn release_borrow(&mut self, var_name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.immut_borrow_count > 0 {
                    meta.immut_borrow_count = meta.immut_borrow_count.saturating_sub(1);
                }
                if meta.mut_borrow_active && self.borrow_stack.last().map_or(false, |(name, _, _)| name == var_name) {
                    meta.mut_borrow_active = false;
                }
                return;
            }
        }
    }
    
    pub fn check_ownership(&self, ty: &Type) -> Ownership {
        ty.ownership()
    }

    // Type Ownership Attributes
    
    pub fn register_ownership(&mut self, type_name: String, ownership: Ownership) {
        self.ownership_registry.insert(type_name, ownership);
    }
    
    pub fn get_type_ownership(&self, type_name: &str) -> Option<Ownership> {
        self.ownership_registry.get(type_name).cloned()
    }
    
    pub fn infer_type_ownership(&self, type_name: &str) -> Ownership {
        // Primitives are always Copy (cannot be overridden)
        match type_name {
            "Int" | "Float" | "Bool" | "Char" | "Unit" => return Ownership::Copy,
            _ => {}
        }
        // Check registry for explicit declarations
        if let Some(ownership) = self.get_type_ownership(type_name) {
            return ownership;
        }
        // String is Move; no Share fallback.
        Ownership::Move
    }
    
    pub fn register_type_with_ownership(&mut self, type_name: String, ownership: Ownership, body: ast::TypeBody) {
        self.register_ownership(type_name.clone(), ownership);
        self.register_type(type_name, body);
    }
    
    pub fn convert_ownership_attr(&self, attr: &Option<ast::OwnershipAttr>) -> Ownership {
        match attr {
            Some(ast::OwnershipAttr::Copy) => Ownership::Copy,
            Some(ast::OwnershipAttr::Move) => Ownership::Move,
            Some(ast::OwnershipAttr::Share) => Ownership::Share,
            None => Ownership::Move,
        }
    }

    // Region Escape & Advanced Borrow Checking
    
    pub fn push_region(&mut self, region_name: String) {
        self.region_stack.push(region_name);
    }
    
    pub fn pop_region(&mut self) -> Option<String> {
        self.region_stack.pop()
    }
    
    pub fn get_current_region(&self) -> Option<&str> {
        self.region_stack.last().map(|s| s.as_str())
    }
    
    pub fn bind_reference_lifetime(&mut self, ref_name: String, region: String) {
        self.reference_lifetimes.insert(ref_name, region);
    }
    
    pub fn get_reference_lifetime(&self, ref_name: &str) -> Option<String> {
        self.reference_lifetimes.get(ref_name).cloned()
    }
    
    // A reference in an inner region should not escape to outer region
    pub fn check_escape_analysis(&mut self, expr_region: Option<&str>, ref_region: Option<&str>, escape_span: Span) -> bool {
        match (expr_region, ref_region) {
            (Some(outer), Some(inner)) if outer != inner => {
                self.report_error(
                    format!("Reference from region '{}' escapes to region '{}'", inner, outer),
                    escape_span
                );
                false
            }
            _ => true,
        }
    }
    
    pub fn register_pattern_borrow(&mut self, pattern_var: String, kind: BorrowKind) {
        self.pattern_borrows.entry(pattern_var)
            .or_insert_with(Vec::new)
            .push(kind);
    }

    pub fn get_pattern_borrows(&self, pattern_var: &str) -> Option<Vec<BorrowKind>> {
        self.pattern_borrows.get(pattern_var).cloned()
    }

    // No more than one Mut borrow may be active across the given patterns.
    pub fn check_pattern_borrow_exclusivity(&self, patterns: &[&str]) -> bool {
        let mut has_mut = false;
        for pattern in patterns {
            if let Some(borrows) = self.get_pattern_borrows(pattern) {
                for kind in borrows {
                    if matches!(kind, BorrowKind::Mut) {
                        if has_mut { return false; }
                        has_mut = true;
                    }
                }
            }
        }
        true
    }
    
    // Reference can only escape to parent scope, not to different region
    pub fn validate_reference_escape(&self, ref_var: &str, current_scope_region: Option<&str>) -> bool {
        if let Some(ref_region) = self.get_reference_lifetime(ref_var) {
            match current_scope_region {
                Some(current) if current != ref_region => false,
                _ => true,
            }
        } else {
            true
        }
    }
    
    pub fn clear_region_context(&mut self) {
        self.region_stack.clear();
        self.reference_lifetimes.clear();
        self.pattern_borrows.clear();
    }
}
