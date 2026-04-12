pub mod ast;
pub mod parser;

pub use ast::AstNode;
pub use parser::{parse_one, parse_program, ParseError};

pub fn frontend_name() -> &'static str {
    "eshkol-frontend"
}
