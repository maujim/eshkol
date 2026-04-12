use eshkol_backend::execute_program;
use eshkol_frontend::parse_program;
use std::env;
use std::fs;
use std::io::{self, Write};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "-e" | "--eval" => {
            if args.len() < 2 {
                return Err("missing expression after --eval/-e".to_string());
            }
            eval_and_print(&args[1])
        }
        "--repl" => run_repl(),
        "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        path => {
            let source = fs::read_to_string(path)
                .map_err(|e| format!("failed to read source file `{path}`: {e}"))?;
            eval_and_print(&source)
        }
    }
}

fn run_repl() -> Result<(), String> {
    println!("Eshkol Rust REPL (prototype). Enter :quit to exit.");

    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        print!("eshkol-rs> ");
        io::stdout()
            .flush()
            .map_err(|e| format!("failed to flush stdout: {e}"))?;

        line.clear();
        let n = stdin
            .read_line(&mut line)
            .map_err(|e| format!("failed to read input: {e}"))?;

        if n == 0 {
            println!();
            return Ok(());
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, ":quit" | ":q" | "(exit)") {
            return Ok(());
        }

        if let Err(err) = eval_and_print(input) {
            eprintln!("error: {err}");
        }
    }
}

fn eval_and_print(source: &str) -> Result<(), String> {
    let forms = parse_program(source).map_err(|e| e.to_string())?;
    let result = execute_program(&forms).map_err(|e| e.to_string())?;
    println!("{result}");
    Ok(())
}

fn print_usage() {
    println!("eshkol-rs (Rust port prototype)");
    println!("Usage:");
    println!("  eshkol-rs -e \"(+ 1 2)\"      Evaluate inline expression");
    println!("  eshkol-rs <file.esk>          Evaluate source file");
    println!("  eshkol-rs --repl              Start interactive REPL prototype");
}
