use abrase::ast::StringPart;
use abrase::lexer::Lexer;
use abrase::lexer::Token;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_keywords_and_identifiers() {
        let input = "fn let const mut pub region handle resume true false _ident Self";
        let mut lexer = Lexer::new(input);

        let expected = vec![
            Token::Fn, Token::Let, Token::Const, Token::Mut, Token::Pub,
            Token::Region, Token::Handle, Token::Resume, Token::True, Token::False,
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
    fn test_lexer_string_interp_single_ident() {
        // "answer: {x}" tokenises into a StringInterp with [Literal, Interp].
        let mut lexer = Lexer::new("\"answer: {x}\"");
        let (tok, _) = lexer.next_token();
        assert_eq!(
            tok,
            Token::StringInterp(vec![
                StringPart::Literal("answer: ".into()),
                StringPart::Interp(vec!["x".into()]),
            ]),
        );
    }

    #[test]
    fn test_lexer_string_interp_dotted_path() {
        // `{user.name}` produces an Interp with a multi-segment path.
        let mut lexer = Lexer::new("\"hi {user.name}!\"");
        let (tok, _) = lexer.next_token();
        assert_eq!(
            tok,
            Token::StringInterp(vec![
                StringPart::Literal("hi ".into()),
                StringPart::Interp(vec!["user".into(), "name".into()]),
                StringPart::Literal("!".into()),
            ]),
        );
    }

    #[test]
    fn test_lexer_string_interp_rejects_function_call() {
        // `{build(n)}` is not a simple path — must surface a lex error.
        let mut lexer = Lexer::new("\"{build(n)}y\"");
        let (tok, _) = lexer.next_token();
        match tok {
            Token::Illegal(msg) => assert!(msg.contains("simple paths"), "msg: {}", msg),
            other => panic!("expected Illegal, got {:?}", other),
        }
    }

    #[test]
    fn test_lexer_string_interp_rejects_arithmetic() {
        // `{a + b}` likewise must error, not silently truncate.
        let mut lexer = Lexer::new("\"{a + b}\"");
        let (tok, _) = lexer.next_token();
        assert!(matches!(tok, Token::Illegal(_)),
                "expected Illegal for arithmetic in interp, got {:?}", tok);
    }

    #[test]
    fn test_lexer_string_interp_rejects_unterminated() {
        // No closing `}` before EOF — must error rather than silently produce a string.
        let mut lexer = Lexer::new("\"{x");
        let (tok, _) = lexer.next_token();
        match tok {
            Token::Illegal(msg) => assert!(msg.contains("unterminated"), "msg: {}", msg),
            other => panic!("expected Illegal, got {:?}", other),
        }
    }

    #[test]
    fn test_lexer_plain_string_stays_string() {
        // A literal with no `{...}` segments must NOT become a StringInterp.
        let mut lexer = Lexer::new("\"plain\"");
        let (tok, _) = lexer.next_token();
        assert_eq!(tok, Token::String("plain".into()));
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