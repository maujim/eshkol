#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Symbol(String),
    List(Vec<AstNode>),
    Quote(Box<AstNode>),
}

impl AstNode {
    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            AstNode::Symbol(s) => Some(s.as_str()),
            _ => None,
        }
    }
}
