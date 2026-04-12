use crate::ast::AstNode;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at {}: {}", self.position, self.message)
    }
}

impl Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    LParen,
    RParen,
    Quote,
    Atom(String),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    position: usize,
}

pub fn parse_program(input: &str) -> Result<Vec<AstNode>, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let mut forms = Vec::new();

    while !parser.is_eof() {
        forms.push(parser.parse_expr()?);
    }

    Ok(forms)
}

pub fn parse_one(input: &str) -> Result<AstNode, ParseError> {
    let mut forms = parse_program(input)?;
    match forms.len() {
        0 => Err(ParseError::new("expected one expression, got empty input", 0)),
        1 => Ok(forms.remove(0)),
        _ => Err(ParseError::new(
            "expected one expression, got multiple forms",
            0,
        )),
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        let ch = chars[i];
        match ch {
            c if c.is_whitespace() => {
                i += 1;
            }
            ';' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    position: i,
                });
                i += 1;
            }
            ')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    position: i,
                });
                i += 1;
            }
            '\'' => {
                tokens.push(Token {
                    kind: TokenKind::Quote,
                    position: i,
                });
                i += 1;
            }
            '"' => {
                let start = i;
                i += 1;
                let mut out = String::new();
                let mut closed = false;

                while i < chars.len() {
                    let c = chars[i];
                    if c == '"' {
                        closed = true;
                        i += 1;
                        break;
                    }

                    if c == '\\' {
                        i += 1;
                        if i >= chars.len() {
                            return Err(ParseError::new("unterminated string escape", start));
                        }
                        let escaped = chars[i];
                        match escaped {
                            'n' => out.push('\n'),
                            't' => out.push('\t'),
                            'r' => out.push('\r'),
                            '"' => out.push('"'),
                            '\\' => out.push('\\'),
                            other => out.push(other),
                        }
                        i += 1;
                        continue;
                    }

                    out.push(c);
                    i += 1;
                }

                if !closed {
                    return Err(ParseError::new("unterminated string literal", start));
                }

                tokens.push(Token {
                    kind: TokenKind::String(out),
                    position: start,
                });
            }
            _ => {
                let start = i;
                let mut atom = String::new();
                while i < chars.len() {
                    let c = chars[i];
                    if c.is_whitespace() || matches!(c, '(' | ')' | '\'' | ';') {
                        break;
                    }
                    atom.push(c);
                    i += 1;
                }

                tokens.push(Token {
                    kind: TokenKind::Atom(atom),
                    position: start,
                });
            }
        }
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn is_eof(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor).cloned();
        if token.is_some() {
            self.cursor += 1;
        }
        token
    }

    fn parse_expr(&mut self) -> Result<AstNode, ParseError> {
        let token = self
            .next()
            .ok_or_else(|| ParseError::new("unexpected end of input", 0))?;

        match token.kind {
            TokenKind::LParen => self.parse_list(token.position),
            TokenKind::RParen => Err(ParseError::new(
                "unexpected ')'",
                token.position,
            )),
            TokenKind::Quote => {
                let quoted = self.parse_expr()?;
                Ok(AstNode::Quote(Box::new(quoted)))
            }
            TokenKind::String(s) => Ok(AstNode::String(s)),
            TokenKind::Atom(atom) => Ok(parse_atom(&atom)),
        }
    }

    fn parse_list(&mut self, start_position: usize) -> Result<AstNode, ParseError> {
        let mut items = Vec::new();

        loop {
            let Some(next) = self.peek() else {
                return Err(ParseError::new("unclosed list", start_position));
            };

            if matches!(next.kind, TokenKind::RParen) {
                self.next();
                return Ok(AstNode::List(items));
            }

            items.push(self.parse_expr()?);
        }
    }
}

fn parse_atom(atom: &str) -> AstNode {
    if atom == "#t" {
        return AstNode::Bool(true);
    }
    if atom == "#f" {
        return AstNode::Bool(false);
    }

    if let Ok(i) = atom.parse::<i64>() {
        return AstNode::Int(i);
    }

    if (atom.contains('.') || atom.contains('e') || atom.contains('E')) && atom.parse::<f64>().is_ok() {
        return AstNode::Float(atom.parse::<f64>().unwrap());
    }

    AstNode::Symbol(atom.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_atoms() {
        assert_eq!(parse_one("42").unwrap(), AstNode::Int(42));
        assert_eq!(parse_one("3.14").unwrap(), AstNode::Float(3.14));
        assert_eq!(parse_one("#t").unwrap(), AstNode::Bool(true));
        assert_eq!(parse_one("hello").unwrap(), AstNode::Symbol("hello".to_string()));
    }

    #[test]
    fn parses_nested_lists() {
        let parsed = parse_one("(+ 1 (* 2 3))").unwrap();
        assert_eq!(
            parsed,
            AstNode::List(vec![
                AstNode::Symbol("+".to_string()),
                AstNode::Int(1),
                AstNode::List(vec![
                    AstNode::Symbol("*".to_string()),
                    AstNode::Int(2),
                    AstNode::Int(3),
                ]),
            ])
        );
    }

    #[test]
    fn parses_quote_prefix() {
        let parsed = parse_one("'x").unwrap();
        assert_eq!(
            parsed,
            AstNode::Quote(Box::new(AstNode::Symbol("x".to_string())))
        );
    }

    #[test]
    fn ignores_comments() {
        let program = "; heading\n(+ 1 2) ; trailing";
        let forms = parse_program(program).unwrap();
        assert_eq!(forms.len(), 1);
    }

    #[test]
    fn parses_strings() {
        let parsed = parse_one("\"hello\\nworld\"").unwrap();
        assert_eq!(parsed, AstNode::String("hello\nworld".to_string()));
    }

    #[test]
    fn reports_unclosed_list() {
        let err = parse_one("(+ 1 2").unwrap_err();
        assert!(err.message.contains("unclosed"));
    }
}
