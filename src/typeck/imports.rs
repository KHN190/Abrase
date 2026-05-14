use std::collections::HashMap;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::ty::{Ownership, Type};
use super::*;

impl Checker {

    // Namespace Mapping

    pub fn register_import_items(
        &mut self,
        module_path: Vec<String>,
        items: Vec<ast::ImportItem>,
    ) {
        for item in items {
            let accessible_name = item.alias.clone().unwrap_or_else(|| item.name.clone());
            self.imported_names.insert(
                accessible_name,
                (module_path.clone(), item.name),
            );
        }
    }

    pub fn get_imported_name(&self, name: &str) -> Option<(Vec<String>, String)> {
        self.imported_names.get(name).cloned()
    }

    pub fn check_import_collision(&mut self, name: &str, module_path: Vec<String>) -> bool {
        let conflicts_with_var = self.scopes.last()
            .map_or(false, |s| s.vars.contains_key(name));

        if conflicts_with_var {
            self.import_collisions.insert(name.to_string());
            self.report_error(
                format!("'{}' imported from {:?} conflicts with existing binding", name, module_path),
                crate::ast::Span { line: 0, col: 0 },
            );
            return true;
        }

        let existing_module = self.imported_names.get(name)
            .filter(|(m, _)| m != &module_path)
            .map(|(m, _)| m.clone());

        if let Some(existing) = existing_module {
            self.import_collisions.insert(name.to_string());
            self.report_error(
                format!("'{}' imported from {:?} conflicts with import from {:?}", name, module_path, existing),
                crate::ast::Span { line: 0, col: 0 },
            );
            return true;
        }

        false
    }

    pub fn has_import_collision(&self, name: &str) -> bool {
        self.import_collisions.contains(name)
    }

    pub fn get_import_collisions(&self) -> usize {
        self.import_collisions.len()
    }

    pub fn register_qualified_name(&mut self, simple_name: String, qualified_path: Vec<String>) {
        self.qualified_names
            .entry(simple_name)
            .or_insert_with(Vec::new)
            .push(qualified_path);
    }

    pub fn resolve_qualified_name(&self, name_parts: &[String]) -> Option<Vec<String>> {
        if name_parts.is_empty() {
            return None;
        }
        // Try exact match
        if name_parts[0] == "root" {
            if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
                for path in paths_list {
                    if path == name_parts {
                        return Some(path.clone());
                    }
                }
            }
        }
        // Try relative to current module
        let mut candidate = self.current_module.clone();
        for part in name_parts {
            candidate.push(part.clone());
        }
        // Look in qualified_names
        if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
            for path in paths_list {
                if path == &candidate {
                    return Some(path.clone());
                }
            }
        }
        // Try as-is
        if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
            for path in paths_list {
                if path == name_parts {
                    return Some(path.clone());
                }
            }
        }
        None
    }

    pub fn resolve_name(&self, name: &str) -> Option<Vec<String>> {
        if let Some(paths) = self.qualified_names.get(name) {
            if !paths.is_empty() {
                return Some(paths[0].clone());
            }
        }
        None
    }

    pub fn is_name_resolvable(&self, name_parts: &[String]) -> bool {
        self.resolve_qualified_name(name_parts).is_some()
    }

    pub fn get_all_resolutions(&self, name: &str) -> Vec<Vec<String>> {
        self.qualified_names
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_name_resolution(&mut self) {
        self.qualified_names.clear();
    }
}
