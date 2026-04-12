pub mod vm;

pub use vm::{execute_program, VmError};

pub fn backend_name() -> &'static str {
    "eshkol-backend"
}
