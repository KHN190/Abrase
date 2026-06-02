use crate::ast::Span;

#[derive(Debug, Clone)]
pub struct Lint {
    pub code: &'static str,
    pub span: Span,
    pub message: String,
    pub module: Vec<String>,
}

impl Lint {
    pub fn new(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Self { code, span, message: message.into(), module: Vec::new() }
    }

    pub fn with_module(mut self, module: Vec<String>) -> Self {
        self.module = module;
        self
    }

    pub fn pretty_print(&self, source: &str) -> String {
        let mut out = if self.span.line > 0 {
            format!("Warning[{}] at line {}, col {}: {}\n",
                self.code, self.span.line, self.span.col, self.message)
        } else {
            format!("Warning[{}]: {}\n", self.code, self.message)
        };
        let lines: Vec<&str> = source.lines().collect();
        if self.span.line > 0 && self.span.line <= lines.len() {
            let line = lines[self.span.line - 1];
            out.push_str(&format!("  {} | {}\n    | ", self.span.line, line));
            for i in 0..self.span.col.saturating_sub(1) {
                out.push(if i < line.len() && line.chars().nth(i) == Some('\t') { '\t' } else { ' ' });
            }
            out.push_str("^\n");
        }
        out
    }
}
