use std::collections::HashMap;
use crate::ast;
use crate::ty::Type;
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
            // Check if this name is already imported from a different module
            if let Some((existing_module, _)) = self.imported_names.get(&accessible_name) {
                if existing_module != &module_path {
                    // Collision detected - mark as collision but allow insertion
                    self.import_collisions.insert(accessible_name.clone());
                    self.report_error(
                        format!("'{}' already imported from {:?}, cannot import from {:?}",
                            accessible_name, existing_module, module_path),
                        crate::ast::Span { line: 0, col: 0 },
                    );
                    continue; // Skip adding this conflicting import
                }
            }
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

        // Record the import if no collision
        self.imported_names.insert(name.to_string(), (module_path, name.to_string()));
        false
    }

    pub fn has_import_collision(&self, name: &str) -> bool {
        self.import_collisions.contains(name)
    }

    pub fn get_import_collisions(&self) -> usize {
        self.import_collisions.len()
    }

    // Module Registry: segment-by-segment traversal

    pub fn register_module_item(&mut self, module_path: &[String], item_name: String, ty: Type) {
        let module_key = module_path.join("::");
        self.module_registry
            .entry(module_key)
            .or_insert_with(HashMap::new)
            .insert(item_name, ty);
    }

    pub fn lookup_module_item(&self, module_path: &[String], item_name: &str) -> Option<Type> {
        let module_key = module_path.join("::");
        self.module_registry.get(&module_key)?.get(item_name).cloned()
    }

    pub fn get_module_items(&self, module_path: &[String]) -> Option<&HashMap<String, Type>> {
        let module_key = module_path.join("::");
        self.module_registry.get(&module_key)
    }

    fn traverse_module_path(&self, starting_module: &[String], name_parts: &[String]) -> Option<Vec<String>> {
        let mut current_module = starting_module.to_vec();
        for (i, segment) in name_parts.iter().enumerate() {
            let module_key = current_module.join("::");
            if let Some(items) = self.module_registry.get(&module_key) {
                if items.contains_key(segment) {
                    // Visibility: each segment must be accessible from the caller's module
                    if !self.is_accessible(segment, &current_module) {
                        return None;
                    }
                    current_module.push(segment.clone());
                    if i == name_parts.len() - 1 {
                        return Some(current_module);
                    }
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        None
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

        // Segment-by-segment traversal through module_registry (primary path)
        if !self.module_registry.is_empty() {
            // Try from root
            let root = vec!["root".to_string()];
            if let Some(resolved) = self.traverse_module_path(&root, name_parts) {
                return Some(resolved);
            }
            // Try relative to current module
            if let Some(resolved) = self.traverse_module_path(&self.current_module.clone(), name_parts) {
                return Some(resolved);
            }
            // Try skipping explicit "root" prefix if caller included it
            if name_parts[0] == "root" && name_parts.len() > 1 {
                if let Some(resolved) = self.traverse_module_path(&root, &name_parts[1..]) {
                    return Some(resolved);
                }
            }
        }

        // Fall back to qualified_names for backward compatibility
        if name_parts[0] == "root" {
            if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
                for path in paths_list {
                    if path == name_parts {
                        return Some(path.clone());
                    }
                }
            }
        }
        let mut candidate = self.current_module.clone();
        for part in name_parts {
            candidate.push(part.clone());
        }
        if let Some(paths_list) = self.qualified_names.get(&name_parts[name_parts.len() - 1]) {
            for path in paths_list {
                if path == &candidate {
                    return Some(path.clone());
                }
            }
        }
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
