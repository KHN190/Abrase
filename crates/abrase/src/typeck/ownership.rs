use crate::ast::Span;
use crate::ty::Ownership;
use super::*;

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
    
    pub fn register_ownership(&mut self, type_name: String, ownership: Ownership) {
        self.ownership_registry.insert(type_name, ownership);
    }

    pub fn get_type_ownership(&self, type_name: &str) -> Option<Ownership> {
        self.ownership_registry.get(type_name).cloned()
    }

    pub fn push_region(&mut self, region_name: String) {
        self.region_stack.push(region_name);
    }

    pub fn pop_region(&mut self) -> Option<String> {
        self.region_stack.pop()
    }
}
