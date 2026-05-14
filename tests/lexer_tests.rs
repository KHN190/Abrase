use ect::lexer::Lexer;
use ect::lexer::Token;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_keywords_and_identifiers() {
        let input = "fn let const mut pub async await true false _ident Self";
        let mut lexer = Lexer::new(input);
        
        let expected = vec![
            Token::Fn, Token::Let, Token::Const, Token::Mut, Token::Pub, 
            Token::Async, Token::Await, Token::True, Token::False, 
            Token::Ident("_ident".into()), Token::SelfUpper, Token::Eof
        ];
        for t in expected {
            let (token, _span) = lexer.next_token();
            assert_eq!(token, t);
        }
    }

    #[test]
    fn test_lexer_operators() {
        let input = "= == => -> + += - -= * *= / /= % %= ! != < <= > >= && || .. ..= & | @";
        let mut lexer = Lexer::new(input);
        
        let expected = vec![
            Token::Assign, Token::Eq, Token::FatArrow, Token::Arrow,
            Token::Plus, Token::PlusAssign, Token::Minus, Token::MinusAssign,
            Token::Asterisk, Token::MulAssign, Token::Slash, Token::DivAssign,
            Token::Percent, Token::ModAssign, Token::Bang, Token::NotEq,
            Token::Lt, Token::Lte, Token::Gt, Token::Gte,
            Token::And, Token::Or, Token::Range, Token::RangeInclusive,
            Token::Ampersand, Token::Pipe, Token::At, Token::Eof
        ];
        for t in expected {
            let (token, _span) = lexer.next_token();
            assert_eq!(token, t);
        }
    }

    #[test]
    fn test_lexer_literals() {
        let input = "42 3.14 \"hello\" 'a' ()";
        let mut lexer = Lexer::new(input);
        
        let expected = vec![
            Token::Int(42), Token::Float(3.14), Token::String("hello".into()), 
            Token::Char('a'), Token::LParen, Token::RParen, Token::Eof
        ];
        for t in expected {
            let (token, _span) = lexer.next_token();
            assert_eq!(token, t);
        }
    }

    #[test]
    fn test_lexer_comments() {
        let input = "let x = 10; // This is a comment\n let y = 20;";
        let mut lexer = Lexer::new(input);
        
        let expected = vec![
            Token::Let, Token::Ident("x".into()), Token::Assign, Token::Int(10), Token::Semicolon,
            Token::Let, Token::Ident("y".into()), Token::Assign, Token::Int(20), Token::Semicolon, Token::Eof
        ];

        for t in expected {
            let (token, _span) = lexer.next_token();
            assert_eq!(token, t);
        }
    }
}