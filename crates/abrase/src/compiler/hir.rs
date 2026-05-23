use std::collections::HashMap;
use crate::ast;

#[derive(Clone, Debug)]
pub struct RecordLayout {
    pub fields: Vec<String>,
    pub field_types: Vec<ast::Type>,
}

impl RecordLayout {
    pub fn offset_of(&self, field: &str) -> Option<u16> {
        self.fields.iter().position(|f| f == field).map(|i| i as u16)
    }
}

#[derive(Clone, Debug)]
pub enum VariantShape {
    Unit,
    Tuple(usize),
    Record(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct VariantLayout {
    pub type_name: String,
    pub tag: u32,
    pub shape: VariantShape,
    pub field_types: Vec<ast::Type>,
}

#[derive(Default)]
pub struct LayoutCtx {
    pub records: HashMap<String, RecordLayout>,
    pub variants: HashMap<String, VariantLayout>,
}

impl LayoutCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_record(&mut self, name: String, fields: Vec<String>, field_types: Vec<ast::Type>) {
        self.records.insert(name, RecordLayout { fields, field_types });
    }

    pub fn register_variant(&mut self, ctor: String, layout: VariantLayout) {
        self.variants.insert(ctor, layout);
    }
}
