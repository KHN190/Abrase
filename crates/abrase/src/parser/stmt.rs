use crate::ast::*;
use crate::lexer::Token;
use super::core::Parser;
use super::precedence::Precedence;

impl<'a> Parser<'a> {
    pub fn parse_stmt(&mut self) -> Result<Spanned<Stmt>, String> {
        let span = self.current_span;
        match self.current_token {
            Token::Let => {
                self.next_token();
                let is_mut = if self.current_token == Token::Mut { self.next_token(); true } else { false };
                let pattern = self.parse_pattern()?;
                if !matches!(pattern.node, Pattern::Bind(_) | Pattern::Wildcard) {
                    self.report_error(
                        "destructuring let not yet supported; bind to a single name".to_string(),
                        pattern.span,
                    );
                }

                let mut ty = None;
                if self.peek_token == Token::Colon {
                    self.next_token();
                    self.next_token();
                    ty = Some(self.parse_type()?);
                }

                if !self.expect_peek(Token::Assign) {
                    return Err("Expected '='".into());
                }

                self.next_token();
                let value = self.parse_expr(Precedence::Lowest);

                if self.current_token != Token::RBrace
                    && self.current_token != Token::Eof
                    && self.peek_token == Token::Semicolon
                {
                    self.next_token();
                }

                Ok(Spanned { node: Stmt::Let { pattern, is_mut, ty, value }, span })
            }
            _ => {
                let expr = self.parse_expr(Precedence::Lowest);
                if self.current_token != Token::RBrace
                    && self.current_token != Token::Eof
                    && self.peek_token == Token::Semicolon
                {
                    self.next_token();
                }

                Ok(Spanned { node: Stmt::Expr(expr), span })
            }
        }
    }
}
