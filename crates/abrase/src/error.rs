use crate::ast::Span;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ParseError,
    TypeError,
    CodegenError,
    RuntimeError,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::ParseError => write!(f, "ParseError"),
            ErrorCode::TypeError => write!(f, "TypeError"),
            ErrorCode::CodegenError => write!(f, "CompileError"),
            ErrorCode::RuntimeError => write!(f, "RuntimeError"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub code: ErrorCode,
    pub span: Span,
    pub message: String,
    pub context: Vec<String>,
    pub module: Vec<String>,
}

impl Error {
    pub fn new(code: ErrorCode, span: Span, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
            context: Vec::new(),
            module: Vec::new(),
        }
    }

    pub fn with_module(mut self, module: Vec<String>) -> Self {
        self.module = module;
        self
    }

    pub fn with_context(mut self, context: Vec<String>) -> Self {
        self.context = context;
        self
    }

    pub fn add_context(&mut self, info: String) {
        self.context.push(info);
    }

    pub fn pretty_print(&self, source: &str) -> String {
        let mut result = if self.span.line > 0 {
            format!("{} at line {}, col {}: {}\n", self.code, self.span.line, self.span.col, self.message)
        } else {
            format!("{}: {}\n", self.code, self.message)
        };

        let lines: Vec<&str> = source.lines().collect();
        if self.span.line > 0 && self.span.line <= lines.len() {
            let line_idx = self.span.line - 1;
            let line = lines[line_idx];
            result.push_str(&format!("  {} | {}\n", self.span.line, line));

            // Add caret pointing to the exact column
            result.push_str("    | ");
            for i in 0..self.span.col.saturating_sub(1) {
                if i < line.len() && line.chars().nth(i).map_or(false, |c| c == '\t') {
                    result.push('\t');
                } else {
                    result.push(' ');
                }
            }
            result.push_str("^\n");
        }

        if !self.context.is_empty() {
            result.push_str("  Context:\n");
            for ctx in &self.context {
                result.push_str(&format!("    - {}\n", ctx));
            }
        }

        result
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.span.line > 0 {
            write!(
                f,
                "{} at line {}, col {}: {}",
                self.code, self.span.line, self.span.col, self.message
            )?;
        } else {
            write!(f, "{}: {}", self.code, self.message)?;
        }
        if !self.context.is_empty() {
            write!(f, "\n  Context:")?;
            for ctx in &self.context {
                write!(f, "\n    - {}", ctx)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {}
