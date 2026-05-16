use crate::lexer::Token;

#[derive(PartialOrd, PartialEq, Clone, Copy)]
pub enum Precedence {
    Lowest,
    Assign,
    Range,
    Or,
    And,
    Equals,
    LessGreater,
    Sum,
    Product,
    Prefix,
    Call,
    Index,
}

impl Token {
    pub fn precedence(&self) -> Precedence {
        match self {
            Token::Assign | Token::PlusAssign | Token::MinusAssign => Precedence::Assign,
            Token::Range | Token::RangeInclusive => Precedence::Range,
            Token::Or => Precedence::Or,
            Token::And => Precedence::And,
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Lt | Token::Gt | Token::Lte | Token::Gte => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Asterisk | Token::Slash | Token::Percent => Precedence::Product,
            Token::LParen => Precedence::Call,
            Token::LBracket => Precedence::Index,
            Token::Dot => Precedence::Index,
            Token::Question => Precedence::Index,
            _ => Precedence::Lowest,
        }
    }
}
