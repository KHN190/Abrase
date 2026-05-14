use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    // Ownership & Borrowing

    pub fn try_immut_borrow(&mut self, var_name: &str, _borrow_span: Span) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {
                if meta.mut_borrow_active {
                    return Err(format!("Cannot immutably borrow '{}': mutable borrow already active", var_name));
                }
                // All ownership kinds can be immutably borrowed; Move types just can't be used by value again
                meta.immut_borrow_count += 1;
                self.borrow_stack.push((var_name.to_string(), false));
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not found", var_name))
    }
    
    pub fn try_mut_borrow(&mut self, var_name: &str, borrow_span: Span) -> Result<(), String> {
        // First, check the type and ownership outside the loop
        let mut var_type = None;
        for scope in self.scopes.iter().rev() {
            if let Some(meta) = scope.vars.get(var_name) {
                var_type = Some(meta.ty.clone());
                break;
            }
        }

        let var_type = match var_type {
            Some(ty) => ty,
            None => return Err(format!("Variable '{}' not found", var_name)),
        };

        // Check ownership based on type
        let type_ownership = match &var_type {
            Type::String => Ownership::Share,  // String defaults to Share semantics for borrowing
            Type::Named(name) => self.infer_type_ownership(name),
            _ => var_type.ownership(),
        };

        // Now apply the borrow
        for scope in self.scopes.iter_mut().rev() {
            if let Some(meta) = scope.vars.get_mut(var_name) {

                // Strict writer/reader exclusivity enforcement
                // Mutable references always require exclusive access

                if meta.immut_borrow_count > 0 {
                    return Err(format!("Cannot mutably borrow '{}': immutable borrow already active", var_name));
                }
                if meta.mut_borrow_active {
                    return Err(format!("Cannot mutably borrow '{}': mutable borrow already active", var_name));
                }
                if !meta.is_mut {
                    return Err(format!("Cannot mutably borrow immutable variable '{}'", var_name));
                }

                // Enforce move semantics: Move-semantics types move on mutable borrow
                if type_ownership == Ownership::Move {
                    meta.is_moved = true;
                    meta.moved_at = Some(borrow_span);
                }

                meta.mut_borrow_active = true;
                self.borrow_stack.push((var_name.to_string(), true));
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
                if meta.mut_borrow_active && self.borrow_stack.last().map_or(false, |(name, _)| name == var_name) {
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
        // String defaults to Share
        if type_name == "String" {
            return Ownership::Share;
        }
        // Unknown types default to Move
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
    
    pub fn register_pattern_borrow(&mut self, pattern_var: String, borrow_constraint: String) {
        self.pattern_borrows.entry(pattern_var)
            .or_insert_with(Vec::new)
            .push(borrow_constraint);
    }
    
    pub fn get_pattern_borrows(&self, pattern_var: &str) -> Option<Vec<String>> {
        self.pattern_borrows.get(pattern_var).cloned()
    }
    
    // Check that borrow constraints from pattern matching don't conflict
    pub fn check_pattern_borrow_exclusivity(&self, patterns: &[&str]) -> bool {
        let mut has_mut_borrow = false;

        for pattern in patterns {
            if let Some(borrows) = self.get_pattern_borrows(pattern) {
                for borrow in borrows {
                    // Check if it's exactly "mut" or starts with "mut_" (not "immut")
                    if borrow == "mut" || (borrow.starts_with("mut") && !borrow.starts_with("immut")) {
                        // Can't have multiple mutable borrows
                        if has_mut_borrow {
                            return false;
                        }
                        has_mut_borrow = true;
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
