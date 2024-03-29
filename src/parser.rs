use crate::ast::{BlockStatement, Expression, Identifier, Program, Statement};
use crate::lexer::Lexer;
use std::fmt::{self, Display};

use crate::token::Token;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    Lowest,
    Equals,
    LessGreater,
    Sum,
    Product,
    Prefix,
    Call,
}

impl Precedence {
    pub fn for_token(token: &Token) -> Self {
        match token {
            Token::Eq | Token::NotEq => Precedence::Equals,
            Token::Gt | Token::Lt => Precedence::LessGreater,
            Token::Plus | Token::Minus => Precedence::Sum,
            Token::Slash | Token::Asterisk => Precedence::Product,
            _ => Precedence::Lowest,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ParserError {
    ExpectedToken { expected: Token, saw: Token },
    ExpectedIdent(Token),
    IntegerParseFailure(String),
    UnhandledPrefix(Token),
    UnhandledExpression(Token),
}

type ParserResult<T> = Result<T, ParserError>;

impl Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParserError::ExpectedToken { expected, saw } => write!(
                f,
                "Expected next token to be {:?}, got {:?} instead",
                expected, saw
            ),
            ParserError::ExpectedIdent(token) => write!(
                f,
                "Expected next token to be Ident, got {:?} instead",
                token
            ),
            ParserError::IntegerParseFailure(expr) => {
                write!(f, "Could not parse {} as integer", expr)
            }
            ParserError::UnhandledPrefix(tok) => {
                write!(f, "No prefix parse function for {:?}", tok)
            }
            ParserError::UnhandledExpression(tok) => {
                write!(f, "No handler for expression: {:?}", tok)
            }
        }
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    errors: Vec<ParserError>,
    cur_token: Token,
    peek_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let cur_token = lexer.next_token();
        let peek_token = lexer.next_token();
        Parser {
            lexer,
            errors: vec![],
            cur_token,
            peek_token,
        }
    }

    pub fn from_input(input: &'a str) -> Self {
        Parser::new(Lexer::new(input))
    }

    pub fn errors(&self) -> &[ParserError] {
        &self.errors
    }

    pub fn peek_error(&mut self, expected: &Token) -> ParserError {
        ParserError::ExpectedToken {
            expected: expected.clone(),
            saw: self.peek_token.clone(),
        }
    }

    pub fn next_token(&mut self) {
        std::mem::swap(&mut self.cur_token, &mut self.peek_token);
        self.peek_token = self.lexer.next_token();
    }

    pub fn current_token_is(&self, tok: &Token) -> bool {
        match (&tok, &self.cur_token) {
            (Token::Ident(_), Token::Ident(_)) => true,
            (Token::Int(_), Token::Int(_)) => true,
            _ => tok == &self.cur_token,
        }
    }

    pub fn peek_token_is(&self, tok: &Token) -> bool {
        match (&tok, &self.peek_token) {
            (Token::Ident(_), Token::Ident(_)) => true,
            (Token::Int(_), Token::Int(_)) => true,
            _ => tok == &self.peek_token,
        }
    }

    pub fn expect_peek(&mut self, expected: &Token) -> ParserResult<()> {
        if self.peek_token_is(expected) {
            self.next_token();
            return Ok(());
        }
        Err(ParserError::ExpectedToken {
            expected: expected.clone(),
            saw: self.peek_token.clone(),
        })
    }

    pub fn expect_ident(&mut self) -> ParserResult<Identifier> {
        match &self.peek_token {
            Token::Ident(name) => {
                let ident = Identifier::new(name);
                self.next_token();
                Ok(ident)
            }
            _ => Err(ParserError::ExpectedIdent(self.peek_token.clone())),
        }
    }

    pub fn parse_program(&mut self) -> Option<Program> {
        let mut program = Program::default();

        while self.cur_token != Token::EOF {
            match self.parse_statement() {
                Ok(stmt) => program.statements.push(stmt),
                Err(err) => self.errors.push(err),
            }
            self.next_token();
        }
        Some(program)
    }

    pub fn parse_statement(&mut self) -> ParserResult<Statement> {
        match self.cur_token {
            Token::Let => self.parse_let_statement(),
            Token::Return => self.parse_return_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    pub fn parse_let_statement(&mut self) -> ParserResult<Statement> {
        let token = self.cur_token.clone();
        let name = self.expect_ident()?;
        self.expect_peek(&Token::Assign)?;
        self.next_token();
        let value = self.parse_expression(Precedence::Lowest)?;
        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }
        Ok(Statement::Let { token, name, value })
    }

    pub fn cur_precedence(&self) -> Precedence {
        Precedence::for_token(&self.cur_token)
    }

    pub fn peek_precedence(&self) -> Precedence {
        Precedence::for_token(&self.peek_token)
    }

    pub fn parse_return_statement(&mut self) -> ParserResult<Statement> {
        let token = self.cur_token.clone();
        let stmt = Statement::Return {
            token,
            expr: Expression::Nothing,
        };
        self.next_token();
        while !self.current_token_is(&Token::Semicolon) {
            self.next_token();
        }
        Ok(stmt)
    }

    pub fn parse_expression_statement(&mut self) -> ParserResult<Statement> {
        let expr = self.parse_expression(Precedence::Lowest)?;
        let stmt = Statement::Expression {
            token: self.cur_token.clone(),
            expr,
        };

        if self.peek_token_is(&Token::Semicolon) {
            self.next_token();
        }
        Ok(stmt)
    }

    pub fn parse_int_expression(&self, value_str: &str) -> ParserResult<Expression> {
        match value_str.parse::<i64>() {
            Ok(value) => Ok(Expression::IntegerLiteral(value)),
            Err(_) => Err(ParserError::IntegerParseFailure(value_str.to_owned())),
        }
    }

    pub fn parse_prefix_expression(&mut self) -> ParserResult<Expression> {
        let operator = self.cur_token.clone();
        self.next_token();
        let right = self.parse_expression(Precedence::Prefix)?;
        Ok(Expression::Prefix {
            operator,
            right: Box::new(right),
        })
    }

    pub fn parse_infix_expression(&mut self, left: Expression) -> ParserResult<Expression> {
        let operator = self.cur_token.clone();
        let precedence = self.cur_precedence();
        self.next_token();
        let right = self.parse_expression(precedence)?;
        Ok(Expression::Infix {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn parse_boolean_expression(&self, is_true: bool) -> ParserResult<Expression> {
        Ok(Expression::Boolean(is_true))
    }

    pub fn parse_grouped_expression(&mut self) -> ParserResult<Expression> {
        self.next_token();
        let expr = self.parse_expression(Precedence::Lowest);
        self.expect_peek(&Token::RParen).and(expr)
    }

    pub fn parse_block_statement(&mut self) -> ParserResult<BlockStatement> {
        let mut block = BlockStatement::new(self.cur_token.clone());
        self.next_token();
        while !self.current_token_is(&Token::RBrace) && !self.current_token_is(&Token::EOF) {
            let stmt = self.parse_statement()?;
            block.statements.push(stmt);
            self.next_token();
        }
        Ok(block)
    }

    pub fn parse_if_expression(&mut self) -> ParserResult<Expression> {
        self.expect_peek(&Token::LParen)?;

        let condition = self.parse_expression(Precedence::Lowest)?;
        self.expect_peek(&Token::LBrace)?;
        let consequence = self.parse_block_statement()?;
        let alternative = match self.peek_token {
            Token::Else => {
                self.next_token();
                self.expect_peek(&Token::LBrace)?;
                Some(self.parse_block_statement()?)
            }
            _ => None,
        };
        Ok(Expression::If {
            condition: Box::new(condition),
            consequence,
            alternative,
        })
    }

    pub fn parse_expression(&mut self, precedence: Precedence) -> ParserResult<Expression> {
        let mut left = match &self.cur_token {
            Token::Ident(name) => Expression::Identifier(Identifier::new(name)),
            Token::Int(value_str) => self.parse_int_expression(value_str)?,
            Token::Bang => self.parse_prefix_expression()?,
            Token::Minus => self.parse_prefix_expression()?,
            Token::Plus => self.parse_prefix_expression()?,
            Token::True => self.parse_boolean_expression(true)?,
            Token::False => self.parse_boolean_expression(false)?,
            Token::LParen => self.parse_grouped_expression()?,
            Token::If => self.parse_if_expression()?,
            _ => return Err(ParserError::UnhandledPrefix(self.cur_token.clone())),
        };

        while !self.peek_token_is(&Token::Semicolon) && precedence < self.peek_precedence() {
            left = match &self.peek_token {
                Token::Plus
                | Token::Minus
                | Token::Asterisk
                | Token::Slash
                | Token::Gt
                | Token::Lt
                | Token::Eq
                | Token::NotEq => {
                    self.next_token();
                    self.parse_infix_expression(left)?
                }
                _ => return Ok(left),
            };
        }
        Ok(left)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Construct a parser to parser the input, returning the parser and parsed
    /// Program object.
    fn parser_for_input(input: &str) -> (Parser, Program) {
        let mut parser = Parser::from_input(input);
        let program = parser.parse_program();
        assert!(program.is_some(), "parse_program() returned None");
        (parser, program.unwrap())
    }

    /// Assert that a Parser contains no errors.
    fn assert_no_parser_errors(parser: &Parser) {
        let errors = parser.errors();
        assert!(
            errors.is_empty(),
            "Parser has {} errors: {:?}",
            errors.len(),
            errors
        )
    }

    /// Assert that a Parser contains no errors.
    fn assert_parser_errors_len(parser: &Parser, count: usize) {
        let errors = parser.errors();

        assert_eq!(
            errors.len(),
            count,
            "Parser.errors does not contain {} error{}: got {} ({:?})",
            count,
            if count >= 1 { "s" } else { "" },
            errors.len(),
            errors
        );
    }

    /// Assert that a Program contains a certain number of statements.
    fn assert_program_statements_len(program: &Program, count: usize) {
        assert_eq!(
            program.statements.len(),
            count,
            "program.statements does not contain {} statement{}: got {}",
            count,
            if count >= 1 { "s" } else { "" },
            program.statements.len()
        );
    }

    /// Assert that a Statement is a Statement::Expression wrapping an Expression
    /// equivalent to the passed Expression.
    fn assert_statement_expression_eq(stmt: &Statement, expected_expr: &Expression) {
        match stmt {
            Statement::Expression { expr, .. } => {
                assert_eq!(expr, expected_expr);
            }
            _ => assert!(
                false,
                "Expected Statement::Expression {{ expr: {} }}, saw {}",
                expected_expr, stmt
            ),
        }
    }

    #[test]
    fn test_let_statements() {
        let input = r#"
        let x = 5;
        let y = 10;
        let foobar = 838383;
        "#;

        let (parser, program) = parser_for_input(input);
        assert_no_parser_errors(&parser);
        assert_program_statements_len(&program, 3);
        let expected_names = vec![
            ("x", Expression::IntegerLiteral(5)),
            ("y", Expression::IntegerLiteral(10)),
            ("foobar", Expression::IntegerLiteral(838383)),
        ];
        for ((expected_identifier, expected_value), stmt) in
            expected_names.iter().zip(program.statements.iter())
        {
            match stmt {
                Statement::Let { token, name, value } => {
                    assert_eq!(token, &Token::Let);
                    assert_eq!(name.0, *expected_identifier);
                    assert_eq!(value, expected_value);
                }
                _ => assert!(false, "Expected Statement::Let, got {:?}", stmt),
            }
        }
    }

    #[test]
    fn test_invalid_let_statements() {
        let input = r#"
        let x 5;
        let = 10;
        let 838383;
        "#;

        let (parser, _program) = parser_for_input(input);
        assert_parser_errors_len(&parser, 4);
        let errors = parser.errors();

        assert!(
            match &errors[0] {
                ParserError::ExpectedToken { expected, saw } => {
                    assert_eq!(expected, &Token::Assign);
                    assert_eq!(saw, &Token::make_int("5"));
                    true
                }
                _ => false,
            },
            "Expected ParserError::ExpectedToken, saw {:?}",
            errors[0]
        );

        assert!(
            match &errors[1] {
                ParserError::ExpectedIdent(saw) => {
                    assert_eq!(saw, &Token::Assign);
                    true
                }
                _ => false,
            },
            "Expected ParserError::ExpectedIdent, saw {:?}",
            errors[1]
        );

        assert!(
            match &errors[2] {
                ParserError::UnhandledPrefix(saw) => {
                    assert_eq!(saw, &Token::Assign);
                    true
                }
                _ => false,
            },
            "Expected ParserError::UnhandledPrefix, saw {:?}",
            errors[1]
        );

        assert!(
            match &errors[3] {
                ParserError::ExpectedIdent(saw) => {
                    assert_eq!(saw, &Token::make_int("838383"));
                    true
                }
                _ => false,
            },
            "Expected ParserError::ExpectedIdent, saw {:?}",
            errors[2]
        );
    }

    #[test]
    fn test_return_statements() {
        let input = r#"
        return 5;
        return 10;
        return 993322;
        "#;

        let (parser, program) = parser_for_input(input);
        assert_no_parser_errors(&parser);
        assert_program_statements_len(&program, 3);
        let values = vec!["5", "10", "993322"];
        for (_expected_identifier, stmt) in values.iter().zip(program.statements.iter()) {
            match stmt {
                Statement::Return { token, .. } => {
                    assert_eq!(token, &Token::Return);
                }
                _ => assert!(false, "Expected ReturnStatement, got {:?}", stmt),
            }
        }
    }

    #[test]
    fn test_identifier_expression() {
        let input = "foobar;";

        let (parser, program) = parser_for_input(input);
        assert_no_parser_errors(&parser);
        assert_program_statements_len(&program, 1);
        assert_statement_expression_eq(
            program.statements.first().unwrap(),
            &Expression::new_ident("foobar"),
        );
    }

    #[test]
    fn test_integer_literal_expression() {
        let input = "5;";

        let (parser, program) = parser_for_input(input);
        assert_no_parser_errors(&parser);
        assert_program_statements_len(&program, 1);

        assert_statement_expression_eq(program.statements.first().unwrap(), &5.into());
    }

    #[test]
    fn test_boolean() {
        let input = vec![
            (
                "true;",
                Statement::Expression {
                    token: Token::True,
                    expr: Expression::Boolean(true),
                },
            ),
            (
                "false;",
                Statement::Expression {
                    token: Token::False,
                    expr: Expression::Boolean(false),
                },
            ),
            (
                "let foobar = true;",
                Statement::Let {
                    token: Token::Let,
                    name: Identifier::new("foobar"),
                    value: Expression::Boolean(true),
                },
            ),
            (
                "let foobar = false;",
                Statement::Let {
                    token: Token::Let,
                    name: Identifier::new("foobar"),
                    value: Expression::Boolean(false),
                },
            ),
        ];
        for (input, expected) in input {
            let (parser, program) = parser_for_input(input);
            assert_no_parser_errors(&parser);
            assert_program_statements_len(&program, 1);
            assert_eq!(&program.statements[0], &expected);
        }
    }

    #[test]
    fn test_parsing_prefix_expressions() {
        let prefix_tests = vec![
            ("!5;", Token::Bang, Expression::IntegerLiteral(5)),
            ("-15;", Token::Minus, Expression::IntegerLiteral(15)),
            ("!true;", Token::Bang, Expression::Boolean(true)),
            ("!false;", Token::Bang, Expression::Boolean(false)),
        ];

        for (input, operator, right) in prefix_tests {
            let (parser, program) = parser_for_input(input);
            assert_no_parser_errors(&parser);
            assert_program_statements_len(&program, 1);
            assert_statement_expression_eq(
                program.statements.first().unwrap(),
                &Expression::new_prefix(operator, right),
            );
        }
    }

    #[test]
    fn test_operator_precedence_parsing() {
        let precedence_tests = vec![
            ("true", "true"),
            ("false", "false"),
            ("3 > 5 == false", "((3 > 5) == false)"),
            ("3 < 5 == true", "((3 < 5) == true)"),
            ("-a + b", "((-a) + b)"),
            ("!-a", "(!(-a))"),
            ("a + b + c", "((a + b) + c)"),
            ("a + b - c", "((a + b) - c)"),
            ("a * b * c", "((a * b) * c)"),
            ("a * b / c", "((a * b) / c)"),
            ("a + b / c", "(a + (b / c))"),
            ("a + b * c + d / e - f", "(((a + (b * c)) + (d / e)) - f)"),
            ("3 + 4; -5 * 5", "(3 + 4)\n((-5) * 5)"),
            ("5 > 4 == 3 < 4", "((5 > 4) == (3 < 4))"),
            ("5 < 4 != 3 > 4", "((5 < 4) != (3 > 4))"),
            (
                "3 + 4 * 5 == 3 * 1 + 4 * 5",
                "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
            ),
            ("1 + (2 + 3) + 4", "((1 + (2 + 3)) + 4)"),
            ("(5 + 5) * 2", "((5 + 5) * 2)"),
            ("2 / (5 + 5)", "(2 / (5 + 5))"),
            ("(5 + 5) * 2 * (5 + 5)", "(((5 + 5) * 2) * (5 + 5))"),
            ("-(5 + 5)", "(-(5 + 5))"),
            ("!(true == true)", "(!(true == true))"),
        ];

        for (input, expected_output) in precedence_tests {
            let (parser, program) = parser_for_input(input);
            assert_no_parser_errors(&parser);

            assert_eq!(format!("{}", program).trim(), expected_output);
        }
    }

    #[test]
    fn test_parsing_infix_expressions() {
        let prefix_tests = vec![
            ("5 + 5;", Expression::new_infix(5, Token::Plus, 5)),
            ("5 - 5;", Expression::new_infix(5, Token::Minus, 5)),
            ("5 * 5;", Expression::new_infix(5, Token::Asterisk, 5)),
            ("5 / 5;", Expression::new_infix(5, Token::Slash, 5)),
            ("5 > 5;", Expression::new_infix(5, Token::Gt, 5)),
            ("5 < 5;", Expression::new_infix(5, Token::Lt, 5)),
            ("5 == 5;", Expression::new_infix(5, Token::Eq, 5)),
            ("5 != 5;", Expression::new_infix(5, Token::NotEq, 5)),
            (
                "true == true;",
                Expression::new_infix(true, Token::Eq, true),
            ),
            (
                "false == false;",
                Expression::new_infix(false, Token::Eq, false),
            ),
            (
                "true != false;",
                Expression::new_infix(true, Token::NotEq, false),
            ),
        ];
        for (input, expected_expr) in prefix_tests {
            let (parser, program) = parser_for_input(input);
            assert_no_parser_errors(&parser);
            assert_program_statements_len(&program, 1);
            assert_statement_expression_eq(program.statements.first().unwrap(), &expected_expr);
        }
    }

    #[test]
    fn test_if_expression() {
        let input = "if (x < y) { x }";
        let (parser, program) = parser_for_input(input);
        assert_no_parser_errors(&parser);
        assert_program_statements_len(&program, 1);

        match program.statements.first().unwrap() {
            Statement::Expression {
                expr:
                    Expression::If {
                        condition,
                        consequence,
                        alternative,
                    },
                ..
            } => {
                assert_eq!(
                    **condition,
                    Expression::new_infix(Identifier::new("x"), Token::Lt, Identifier::new("y"))
                );
                assert_eq!(consequence.statements.len(), 1);
                assert_statement_expression_eq(
                    consequence.statements.first().unwrap(),
                    &Expression::new_ident("x"),
                );
                assert!(alternative.is_none());
            }
            _ => {
                assert!(
                    false,
                    "Expected Statement::Expression, got {:?}",
                    program.statements[0]
                );
                unreachable!();
            }
        }
    }

    #[test]
    fn test_if_else_xpression() {
        let input = "if (x < y) { x } else { y }";
        let (parser, program) = parser_for_input(input);
        assert_no_parser_errors(&parser);
        assert_program_statements_len(&program, 1);

        match program.statements.first().unwrap() {
            Statement::Expression {
                expr:
                    Expression::If {
                        condition,
                        consequence,
                        alternative,
                    },
                ..
            } => {
                assert_eq!(
                    **condition,
                    Expression::new_infix(Identifier::new("x"), Token::Lt, Identifier::new("y"))
                );
                assert_eq!(consequence.statements.len(), 1);
                assert_statement_expression_eq(
                    consequence.statements.first().unwrap(),
                    &Expression::new_ident("x"),
                );
                if let Some(alternative) = alternative {
                    assert_eq!(alternative.statements.len(), 1);
                    assert_statement_expression_eq(
                        alternative.statements.first().unwrap(),
                        &Expression::new_ident("y"),
                    );
                } else {
                    assert!(false, "Expected Some(alternative), got None");
                }
            }
            _ => {
                assert!(
                    false,
                    "Expected Statement::Expression, got {:?}",
                    program.statements[0]
                );
                unreachable!();
            }
        }
    }
}
