use crate::ast::Span;
use super::*;

impl Checker {

    // Visibility & Module Scoping

    pub fn push_module(&mut self, module_name: String) {
        self.current_module.push(module_name);
    }

    pub fn pop_module(&mut self) {
        if self.current_module.len() > 1 {
            self.current_module.pop();
        }
    }

    pub fn get_current_module(&self) -> Vec<String> {
        self.current_module.clone()
    }

    pub fn set_current_module(&mut self, module_path: Vec<String>) {
        if !module_path.is_empty() {
            self.current_module = module_path;
        }
    }

    pub fn mark_public(&mut self, item_name: String) {
        let qualified_name = format!("{}::{}", self.current_module.join("::"), item_name);
        self.public_items.insert(qualified_name.clone());
        self.private_items.remove(&qualified_name);
    }

    pub fn mark_private(&mut self, item_name: String) {
        let qualified_name = format!("{}::{}", self.current_module.join("::"), item_name);
        self.private_items.insert(qualified_name.clone());
        self.public_items.remove(&qualified_name);
    }

    pub fn mark_module_private(&mut self, qualified_path: String) {
        self.private_items.insert(qualified_path);
    }

    pub fn is_public(&self, item_name: &str) -> bool {
        match item_name {
            "Int" | "String" | "Float" | "Bool" | "Unit" | "Char" => return true,
            _ => {}
        }
        for public_item in &self.public_items {
            if public_item.ends_with(&format!("::{}", item_name)) {
                return true;
            }
        }
        false
    }

    pub fn is_item_accessible(&self, item_name: &str) -> bool {
        match item_name {
            "Int" | "String" | "Float" | "Bool" | "Unit" | "Char" => return true,
            _ => {}
        }
        for public_item in &self.public_items {
            if public_item.ends_with(&format!("::{}", item_name)) {
                return true;
            }
        }
        let current_qualified = format!("{}::{}", self.current_module.join("::"), item_name);
        if self.public_items.contains(&current_qualified) || self.private_items.contains(&current_qualified) {
            return true;
        }
        false
    }

    pub fn is_accessible(&self, item_name: &str, item_module: &[String]) -> bool {
        if item_module == self.current_module {
            return true;
        }
        for i in 1..item_module.len() {
            let module_segment_path = &item_module[..=i];
            let segment_qualified = module_segment_path.join("::");

            if self.private_items.contains(&segment_qualified) {
                return false;
            }
        }
        let qualified_name = format!("{}::{}", item_module.join("::"), item_name);
        self.public_items.contains(&qualified_name)
    }

    pub fn is_qualified_accessible(&self, path: &[String]) -> bool {
        if path.is_empty() {
            return false;
        }

        let item_name = path[path.len() - 1].clone();
        let mut item_module: Vec<String> = if path.len() > 1 {
            path[..path.len() - 1].to_vec()
        } else {
            self.current_module.clone()
        };

        if !item_module.is_empty() && item_module[0] != "root" {
            let mut full_path = vec!["root".to_string()];
            full_path.extend(item_module);
            item_module = full_path;
        }

        self.is_accessible(&item_name, &item_module)
    }

    pub fn validate_visibility(&mut self, item_name: &str, item_module: &[String], access_span: Span) -> bool {
        if self.is_accessible(item_name, item_module) {
            return true;
        }
        let qualified = format!("{}::{}", item_module.join("::"), item_name);
        self.report_error(
            format!("Cannot access private item '{}'", qualified),
            access_span
        );
        false
    }

    pub fn get_public_items(&self) -> Vec<String> {
        self.public_items.iter().cloned().collect()
    }

    pub fn get_private_items(&self) -> Vec<String> {
        self.private_items.iter().cloned().collect()
    }

    pub fn clear_visibility_context(&mut self) {
        self.current_module = vec!["root".into()];
        self.public_items.clear();
        self.private_items.clear();
    }
}
