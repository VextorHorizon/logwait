mod matcher;
mod tailer;

use std::fmt;
use std::io;

// WHY: a small hand-rolled error enum instead of `anyhow`/`thiserror` --
// keeps the runtime dependency budget at zero (only `clap` is a dependency),
// and this codebase exists partly to learn the pattern those crates
// abstract away.
#[derive(Debug)]
enum AppError {
    Io(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

fn main() {}
