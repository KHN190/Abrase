use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct RecordLayout {
    pub fields: Vec<String>,
}

impl RecordLayout {
    pub fn offset_of(&self, field: &str) -> Option<u16> {
        self.fields.iter().position(|f| f == field).map(|i| i as u16)
    }

    pub fn size(&self) -> u32 {
        self.fields.len() as u32
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
}

impl VariantLayout {
    pub fn payload_size(&self) -> u32 {
        match &self.shape {
            VariantShape::Unit => 0,
            VariantShape::Tuple(n) => *n as u32,
            VariantShape::Record(fs) => fs.len() as u32,
        }
    }

    pub fn alloc_size(&self) -> u32 {
        1 + self.payload_size()
    }
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

    pub fn register_record(&mut self, name: String, fields: Vec<String>) {
        self.records.insert(name, RecordLayout { fields });
    }

    pub fn register_variant(&mut self, ctor: String, layout: VariantLayout) {
        self.variants.insert(ctor, layout);
    }
}
