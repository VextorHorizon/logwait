//! CLI entry point: parses arguments, wires the tailer to the matcher, and
//! maps the resulting verdict to an exit code. This is the outer end of the
//! data flow: file -> tailer -> matcher -> verdict -> exit code.

mod matcher;
mod tailer;

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::Parser;

use matcher::Verdict;
use tailer::Tailer;

/// Watch a log file another process writes, wait for a pattern, and exit
/// with a code that says which pattern matched.
#[derive(Parser, Debug)]
#[command(name = "logwait", version, about)]
struct Cli {
    /// Path of the log file to watch. May not exist yet.
    file: PathBuf,

    /// Pattern that means success (exit code 0). Repeatable.
    #[arg(long)]
    success: Vec<String>,

    /// Pattern that means a warning-level completion (exit code 3). Repeatable.
    #[arg(long)]
    warn: Vec<String>,

    /// Pattern that means failure (exit code 4). Repeatable.
    #[arg(long)]
    fail: Vec<String>,

    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 500)]
    interval_ms: u64,
}

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

fn main() {
    let cli = Cli::parse();

    if cli.success.is_empty() && cli.warn.is_empty() && cli.fail.is_empty() {
        // WHY: clap's derive doesn't cleanly express "at least one of these
        // three independently repeatable flags" as a single required rule,
        // so it is checked by hand here and reported with clap's own usage
        // error exit code (2) for consistency.
        eprintln!("error: at least one of --success, --warn, --fail is required");
        std::process::exit(2);
    }

    match run(cli) {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("logwait: error: {err}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<i32, AppError> {
    let interval = Duration::from_millis(cli.interval_ms);
    let mut tailer = Tailer::new(cli.file);

    loop {
        for line in tailer.poll()? {
            println!("{line}");
            // WHY: stdout is block-buffered (not line-buffered) once it is
            // piped rather than attached to a terminal, so an explicit flush
            // after every line is what actually makes logwait behave like
            // `tail -f` for a downstream reader instead of only surfacing
            // output once the internal buffer fills or the process exits.
            io::stdout().flush()?;

            if let Some((verdict, pattern)) =
                matcher::evaluate(&line, &cli.success, &cli.warn, &cli.fail)
            {
                print_verdict(verdict, pattern, &line);
                return Ok(exit_code(verdict));
            }
        }

        thread::sleep(interval);
    }
}

// WHY: async buys nothing here -- a single file polled on a fixed interval
// has no concurrency to manage, and `std::thread::sleep` keeps the loop
// trivial to read until an event-driven backend (roadmap v0.4) is worth the
// added complexity.

fn exit_code(verdict: Verdict) -> i32 {
    match verdict {
        Verdict::Success => 0,
        Verdict::Warn => 3,
        Verdict::Fail => 4,
    }
}

fn print_verdict(verdict: Verdict, pattern: &str, line: &str) {
    let label = match verdict {
        Verdict::Success => "SUCCESS",
        Verdict::Warn => "WARN",
        Verdict::Fail => "FAIL",
    };
    // WHY: all diagnostics (including the final verdict) go to stderr so
    // that stdout stays a clean passthrough of log content, safe to pipe.
    eprintln!("logwait: {label} — pattern \"{pattern}\" matched: {line}");
}
