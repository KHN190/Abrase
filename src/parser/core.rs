use crate::lexer::{Lexer, Token};
use crate::error::{Error, ErrorCode};
use crate::ast::Span;

pub const MAX_PARSE_DEPTH: usize = 256;

// RAII guard decrements `Parser::depth` on drop (raw ptr avoids lifetime parameter).
// SAFETY: `counter` points to `Parser::depth` and outlives every issued guard.
pub(crate) struct DepthGuard {
    counter: *mut usize,
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        // SAFETY: see DepthGuard struct comment — `counter` is valid for the
        // entire lifetime of the guard by construction in `enter_depth`.
        unsafe { *self.counter = (*self.counter).saturating_sub(1); }
    }
}

pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) current_token: Token,
    pub(crate) current_span: Span,
    pub(crate) peek_token: Token,
    pub(crate) peek_span: Span,
    pub errors: Vec<Error>,
    pub source: String,
    pub(crate) no_record_literal: bool,
    pub(crate) depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let (cur_tok, cur_span) = lexer.next_token();
        let (peek_tok, peek_span) = lexer.next_token();
        Self {
            lexer,
            current_token: cur_tok,
            current_span: cur_span,
            peek_token: peek_tok,
            peek_span,
            errors: Vec::new(),
            source: String::new(),
            no_record_literal: false,
            depth: 0,
        }
    }

    pub(crate) fn enter_depth(&mut self) -> Option<DepthGuard> {
        if self.depth >= MAX_PARSE_DEPTH {
            let span = self.current_span;
            self.report_error("Expression nested too deeply".to_string(), span);
            return None;
        }
        self.depth += 1;
        Some(DepthGuard { counter: &mut self.depth as *mut usize })
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = source;
        self
    }

    pub fn pretty_print_errors(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.pretty_print(&self.source))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.current_span = self.peek_span;
        let (next_tok, next_span) = self.lexer.next_token();
        self.peek_token = next_tok;
        self.peek_span = next_span;
    }

    pub(crate) fn report_error(&mut self, message: String, span: Span) {
        self.errors.push(Error::new(ErrorCode::ParseError, span, message));
    }

    pub(crate) fn synchronize(&mut self) {
        self.next_token();
        while self.current_token != Token::Eof {
            if self.current_token == Token::Semicolon {
                self.next_token();
                return;
            }
            match self.peek_token {
                Token::Fn | Token::Let | Token::If | Token::Return
                | Token::Match | Token::While | Token::For | Token::Loop
                | Token::Type | Token::Trait | Token::Impl | Token::Const
                | Token::Import | Token::Effect | Token::Pub => return,
                _ => self.next_token(),
            }
        }
    }

    pub(crate) fn expect_peek(&mut self, token: Token) -> bool {
        if self.peek_token == token {
            self.next_token();
            true
        } else {
            let msg = format!("Expected {:?}, got {:?}", token, self.peek_token);
            self.report_error(msg, self.peek_span);
            false
        }
    }

    pub(crate) fn prefix_to_expr_or_err(&mut self, r: Result<crate::ast::Expr, String>, span: Span) -> Option<crate::ast::Spanned<crate::ast::Expr>> {
        match r {
            Ok(e) => Some(crate::ast::Spanned { node: e, span }),
            Err(msg) => {
                self.report_error(msg, span);
                Some(crate::ast::Spanned { node: crate::ast::Expr::Error, span })
            }
        }
    }

    pub(crate) fn is_stmt_start(tok: &Token) -> bool {
        matches!(tok,
            Token::Let | Token::Return | Token::If | Token::Match
            | Token::While | Token::For | Token::Loop | Token::Break
            | Token::Continue | Token::Throw | Token::LBrace
            | Token::Region | Token::Handle | Token::Resume
            | Token::Ident(_) | Token::Int(_) | Token::Float(_)
            | Token::String(_) | Token::StringInterp(_) | Token::True | Token::False
            | Token::Bang | Token::Minus | Token::Ampersand | Token::Asterisk | Token::LParen
        )
    }
}
