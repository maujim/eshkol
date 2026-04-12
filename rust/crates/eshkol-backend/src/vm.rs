use eshkol_core::tagged_value::{
    apply_compare_op, apply_numeric_op, CompareOp, NumericError, NumericOp, TaggedValue,
};
use eshkol_frontend::ast::AstNode;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum VmError {
    UnknownSymbol(String),
    EmptyApplication,
    OperatorMustBeSymbol,
    UnsupportedForm(&'static str),
    InvalidDefine,
    InvalidIf,
    InvalidLet,
    InvalidLambda,
    NotCallable,
    TypeError(String),
    Arity {
        op: String,
        expected: &'static str,
        got: usize,
    },
    Numeric(NumericError),
}

impl Display for VmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::UnknownSymbol(sym) => write!(f, "unknown symbol: {sym}"),
            VmError::EmptyApplication => write!(f, "cannot evaluate empty application"),
            VmError::OperatorMustBeSymbol => write!(f, "operator must be a symbol"),
            VmError::UnsupportedForm(form) => write!(f, "unsupported form: {form}"),
            VmError::InvalidDefine => write!(f, "invalid define form"),
            VmError::InvalidIf => write!(f, "invalid if form"),
            VmError::InvalidLet => write!(f, "invalid let form"),
            VmError::InvalidLambda => write!(f, "invalid lambda form"),
            VmError::NotCallable => write!(f, "attempted to call a non-callable value"),
            VmError::TypeError(msg) => write!(f, "type error: {msg}"),
            VmError::Arity { op, expected, got } => {
                write!(f, "arity mismatch for `{op}`: expected {expected}, got {got}")
            }
            VmError::Numeric(err) => write!(f, "numeric error: {err}"),
        }
    }
}

impl Error for VmError {}

impl From<NumericError> for VmError {
    fn from(value: NumericError) -> Self {
        VmError::Numeric(value)
    }
}

type EnvRef = Rc<RefCell<Env>>;

#[derive(Debug)]
struct Env {
    bindings: HashMap<String, RuntimeValue>,
    parent: Option<EnvRef>,
}

impl Env {
    fn new_root() -> EnvRef {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::new(),
            parent: None,
        }))
    }

    fn new_child(parent: EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Self {
            bindings: HashMap::new(),
            parent: Some(parent),
        }))
    }
}

#[derive(Debug, Clone)]
struct Closure {
    params: Vec<String>,
    body: Vec<AstNode>,
    env: EnvRef,
}

#[derive(Debug, Clone)]
enum RuntimeValue {
    Scalar(TaggedValue),
    Closure(Closure),
}

impl RuntimeValue {
    fn scalar(value: TaggedValue) -> Self {
        RuntimeValue::Scalar(value)
    }
}

fn env_define(env: &EnvRef, name: String, value: RuntimeValue) {
    env.borrow_mut().bindings.insert(name, value);
}

fn env_lookup(env: &EnvRef, name: &str) -> Option<RuntimeValue> {
    if let Some(value) = env.borrow().bindings.get(name) {
        return Some(value.clone());
    }

    let parent = env.borrow().parent.clone();
    parent.and_then(|p| env_lookup(&p, name))
}

pub fn execute_program(forms: &[AstNode]) -> Result<TaggedValue, VmError> {
    let env = Env::new_root();
    let mut last = RuntimeValue::scalar(TaggedValue::null());
    for form in forms {
        last = eval(form, &env)?;
    }
    runtime_to_output(last)
}

fn runtime_to_output(value: RuntimeValue) -> Result<TaggedValue, VmError> {
    match value {
        RuntimeValue::Scalar(v) => Ok(v),
        RuntimeValue::Closure(_) => Err(VmError::UnsupportedForm("closure value output")),
    }
}

fn into_scalar(value: RuntimeValue) -> Result<TaggedValue, VmError> {
    match value {
        RuntimeValue::Scalar(v) => Ok(v),
        RuntimeValue::Closure(_) => Err(VmError::TypeError(
            "expected scalar value, found closure".to_string(),
        )),
    }
}

fn is_truthy(value: &RuntimeValue) -> bool {
    !matches!(value, RuntimeValue::Scalar(TaggedValue::Bool(false)))
}

fn eval(node: &AstNode, env: &EnvRef) -> Result<RuntimeValue, VmError> {
    match node {
        AstNode::Int(i) => Ok(RuntimeValue::scalar(TaggedValue::exact_int(*i))),
        AstNode::Float(f) => Ok(RuntimeValue::scalar(TaggedValue::double(*f))),
        AstNode::Bool(b) => Ok(RuntimeValue::scalar(TaggedValue::bool(*b))),
        AstNode::String(_) => Err(VmError::UnsupportedForm("string literal evaluation")),
        AstNode::Quote(_) => Err(VmError::UnsupportedForm("quote")),
        AstNode::Symbol(sym) => env_lookup(env, sym).ok_or_else(|| VmError::UnknownSymbol(sym.clone())),
        AstNode::List(items) => eval_list(items, env),
    }
}

fn eval_list(items: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    if items.is_empty() {
        return Ok(RuntimeValue::scalar(TaggedValue::null()));
    }

    if let Some(op) = items[0].as_symbol() {
        let args = &items[1..];
        match op {
            "define" => return eval_define(args, env),
            "if" => return eval_if(args, env),
            "begin" => return eval_begin(args, env),
            "let" => return eval_let(args, env),
            "lambda" => return eval_lambda(args, env),
            "+" | "-" | "*" | "/" | "<" | ">" | "=" | "<=" | ">=" => {
                return eval_builtin_call(op, args, env)
            }
            _ => {}
        }
    }

    let callee = eval(&items[0], env)?;
    let mut args = Vec::with_capacity(items.len().saturating_sub(1));
    for arg in &items[1..] {
        args.push(eval(arg, env)?);
    }

    apply_callable(callee, args)
}

fn eval_define(args: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    if args.is_empty() {
        return Err(VmError::InvalidDefine);
    }

    match &args[0] {
        AstNode::Symbol(name) => {
            if args.len() != 2 {
                return Err(VmError::InvalidDefine);
            }
            let value = eval(&args[1], env)?;
            env_define(env, name.clone(), value);
            Ok(RuntimeValue::scalar(TaggedValue::null()))
        }
        AstNode::List(signature) => {
            if signature.is_empty() || args.len() < 2 {
                return Err(VmError::InvalidDefine);
            }

            let fn_name = signature[0]
                .as_symbol()
                .ok_or(VmError::InvalidDefine)?
                .to_string();

            let mut params = Vec::new();
            for param in &signature[1..] {
                let p = param.as_symbol().ok_or(VmError::InvalidDefine)?;
                params.push(p.to_string());
            }

            let body = args[1..].to_vec();
            let closure = RuntimeValue::Closure(Closure {
                params,
                body,
                env: env.clone(),
            });
            env_define(env, fn_name, closure);
            Ok(RuntimeValue::scalar(TaggedValue::null()))
        }
        _ => Err(VmError::InvalidDefine),
    }
}

fn eval_if(args: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(VmError::InvalidIf);
    }

    let cond = eval(&args[0], env)?;
    if is_truthy(&cond) {
        eval(&args[1], env)
    } else if args.len() == 3 {
        eval(&args[2], env)
    } else {
        Ok(RuntimeValue::scalar(TaggedValue::null()))
    }
}

fn eval_begin(args: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    let mut last = RuntimeValue::scalar(TaggedValue::null());
    for expr in args {
        last = eval(expr, env)?;
    }
    Ok(last)
}

fn eval_let(args: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    if args.len() < 2 {
        return Err(VmError::InvalidLet);
    }

    let bindings = match &args[0] {
        AstNode::List(items) => items,
        _ => return Err(VmError::InvalidLet),
    };

    let mut evaluated_bindings: Vec<(String, RuntimeValue)> = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let pair = match binding {
            AstNode::List(items) => items,
            _ => return Err(VmError::InvalidLet),
        };

        if pair.len() != 2 {
            return Err(VmError::InvalidLet);
        }

        let name = pair[0].as_symbol().ok_or(VmError::InvalidLet)?.to_string();
        let value = eval(&pair[1], env)?;
        evaluated_bindings.push((name, value));
    }

    let let_env = Env::new_child(env.clone());
    for (name, value) in evaluated_bindings {
        env_define(&let_env, name, value);
    }

    eval_begin(&args[1..], &let_env)
}

fn eval_lambda(args: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    if args.len() < 2 {
        return Err(VmError::InvalidLambda);
    }

    let params = match &args[0] {
        AstNode::List(items) => items,
        _ => return Err(VmError::InvalidLambda),
    };

    let mut param_names = Vec::with_capacity(params.len());
    for p in params {
        let name = p.as_symbol().ok_or(VmError::InvalidLambda)?;
        param_names.push(name.to_string());
    }

    let body = args[1..].to_vec();
    Ok(RuntimeValue::Closure(Closure {
        params: param_names,
        body,
        env: env.clone(),
    }))
}

fn eval_builtin_call(op: &str, args: &[AstNode], env: &EnvRef) -> Result<RuntimeValue, VmError> {
    let tagged = match op {
        "+" => fold_numeric(args, env, NumericOp::Add, TaggedValue::exact_int(0), false, "+")?,
        "*" => fold_numeric(args, env, NumericOp::Mul, TaggedValue::exact_int(1), false, "*")?,
        "-" => {
            if args.is_empty() {
                return Err(VmError::Arity {
                    op: "-".to_string(),
                    expected: "at least 1 argument",
                    got: 0,
                });
            }

            if args.len() == 1 {
                let value = into_scalar(eval(&args[0], env)?)?;
                apply_numeric_op(&TaggedValue::exact_int(0), &value, NumericOp::Sub)?
            } else {
                fold_numeric_from_first(args, env, NumericOp::Sub, "-")?
            }
        }
        "/" => {
            if args.is_empty() {
                return Err(VmError::Arity {
                    op: "/".to_string(),
                    expected: "at least 1 argument",
                    got: 0,
                });
            }

            if args.len() == 1 {
                let value = into_scalar(eval(&args[0], env)?)?;
                apply_numeric_op(&TaggedValue::exact_int(1), &value, NumericOp::Div)?
            } else {
                fold_numeric_from_first(args, env, NumericOp::Div, "/")?
            }
        }
        "<" => compare_chain(args, env, CompareOp::Lt, "<")?,
        ">" => compare_chain(args, env, CompareOp::Gt, ">")?,
        "=" => compare_chain(args, env, CompareOp::Eq, "=")?,
        "<=" => compare_chain(args, env, CompareOp::Le, "<=")?,
        ">=" => compare_chain(args, env, CompareOp::Ge, ">=")?,
        _ => return Err(VmError::UnknownSymbol(op.to_string())),
    };

    Ok(RuntimeValue::scalar(tagged))
}

fn fold_numeric(
    args: &[AstNode],
    env: &EnvRef,
    op: NumericOp,
    identity: TaggedValue,
    require_at_least_one: bool,
    op_name: &str,
) -> Result<TaggedValue, VmError> {
    if require_at_least_one && args.is_empty() {
        return Err(VmError::Arity {
            op: op_name.to_string(),
            expected: "at least 1 argument",
            got: 0,
        });
    }

    let mut acc = identity;
    for arg in args {
        let value = into_scalar(eval(arg, env)?)?;
        acc = apply_numeric_op(&acc, &value, op)?;
    }

    Ok(acc)
}

fn fold_numeric_from_first(
    args: &[AstNode],
    env: &EnvRef,
    op: NumericOp,
    op_name: &str,
) -> Result<TaggedValue, VmError> {
    let mut iter = args.iter();
    let first = iter
        .next()
        .ok_or(VmError::Arity {
            op: op_name.to_string(),
            expected: "at least 1 argument",
            got: 0,
        })
        .and_then(|expr| eval(expr, env))
        .and_then(into_scalar)?;

    let mut acc = first;
    for arg in iter {
        let value = into_scalar(eval(arg, env)?)?;
        acc = apply_numeric_op(&acc, &value, op)?;
    }

    Ok(acc)
}

fn compare_chain(
    args: &[AstNode],
    env: &EnvRef,
    op: CompareOp,
    op_name: &str,
) -> Result<TaggedValue, VmError> {
    if args.len() < 2 {
        return Err(VmError::Arity {
            op: op_name.to_string(),
            expected: "at least 2 arguments",
            got: args.len(),
        });
    }

    let mut prev = into_scalar(eval(&args[0], env)?)?;
    for arg in &args[1..] {
        let next = into_scalar(eval(arg, env)?)?;
        let cmp = apply_compare_op(&prev, &next, op)?;
        if cmp.as_bool() != Some(true) {
            return Ok(TaggedValue::bool(false));
        }
        prev = next;
    }

    Ok(TaggedValue::bool(true))
}

fn apply_callable(callee: RuntimeValue, args: Vec<RuntimeValue>) -> Result<RuntimeValue, VmError> {
    match callee {
        RuntimeValue::Closure(closure) => {
            if args.len() != closure.params.len() {
                return Err(VmError::Arity {
                    op: "lambda".to_string(),
                    expected: "exact parameter count",
                    got: args.len(),
                });
            }

            let call_env = Env::new_child(closure.env.clone());
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                env_define(&call_env, param.clone(), value);
            }

            eval_begin(&closure.body, &call_env)
        }
        RuntimeValue::Scalar(_) => Err(VmError::NotCallable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eshkol_frontend::parser::parse_program;

    fn eval_src(src: &str) -> TaggedValue {
        let forms = parse_program(src).expect("parse should succeed");
        execute_program(&forms).expect("eval should succeed")
    }

    #[test]
    fn evaluates_nested_arithmetic() {
        let out = eval_src("(* (+ 1 2) 4)");
        assert_eq!(out, TaggedValue::exact_int(12));
    }

    #[test]
    fn mixed_exact_inexact_promotes() {
        let out = eval_src("(+ 1 2.5)");
        assert_eq!(out, TaggedValue::double(3.5));
    }

    #[test]
    fn comparison_chain_works() {
        let out = eval_src("(< 1 2 3)");
        assert_eq!(out.as_bool(), Some(true));

        let out_false = eval_src("(<= 1 3 2)");
        assert_eq!(out_false.as_bool(), Some(false));
    }

    #[test]
    fn divide_by_zero_errors() {
        let forms = parse_program("(/ 1 0)").unwrap();
        let err = execute_program(&forms).unwrap_err();
        assert!(matches!(err, VmError::Numeric(NumericError::DivideByZero)));
    }

    #[test]
    fn define_and_begin_work() {
        let out = eval_src("(begin (define x 41) (+ x 1))");
        assert_eq!(out, TaggedValue::exact_int(42));
    }

    #[test]
    fn if_form_works() {
        let out = eval_src("(if (< 1 2) 7 9)");
        assert_eq!(out, TaggedValue::exact_int(7));
    }

    #[test]
    fn let_form_works() {
        let out = eval_src("(let ((x 5) (y 6)) (+ x y))");
        assert_eq!(out, TaggedValue::exact_int(11));
    }

    #[test]
    fn lambda_application_works() {
        let out = eval_src("((lambda (x) (+ x 2)) 5)");
        assert_eq!(out, TaggedValue::exact_int(7));
    }

    #[test]
    fn closure_capture_works() {
        let out = eval_src(
            "(begin
                (define (make-adder x) (lambda (y) (+ x y)))
                (define add5 (make-adder 5))
                (add5 3))",
        );
        assert_eq!(out, TaggedValue::exact_int(8));
    }

    #[test]
    fn recursive_function_definition_works() {
        let out = eval_src(
            "(begin
                (define (fact n)
                    (if (<= n 1)
                        1
                        (* n (fact (- n 1)))))
                (fact 5))",
        );
        assert_eq!(out, TaggedValue::exact_int(120));
    }
}
