//! Integration tests for the `logwait` binary. Each test spawns the real
//! binary against a temp file, drives it with a writer thread, and asserts
//! on its exit code and stdout/stderr — end to end coverage of the
//! file -> tailer -> matcher -> verdict -> exit code flow described in
//! the module docs of `main.rs`, `tailer.rs`, and `matcher.rs`.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;

/// Kills the spawned `logwait` process on drop, so a failed assertion (which
/// unwinds the test) can't leave a hung child process holding the temp file
/// open -- on Windows that would prevent `TempDir` from deleting it.
struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }

    fn wait_with_output(mut self) -> Output {
        self.0
            .take()
            .expect("child guard already consumed")
            .wait_with_output()
            .expect("wait for logwait")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Spawns `logwait` against `log_path` with a short poll interval, capturing
/// stdout/stderr so the test can assert on them once it exits.
fn spawn_logwait(log_path: &Path, extra_args: &[&str]) -> Child {
    let mut cmd = Command::cargo_bin("logwait").expect("logwait binary should build");
    cmd.arg(log_path);
    cmd.args(extra_args);
    // WHY: a short, fast interval keeps these tests quick without making
    // them flaky -- 50ms is generous relative to the delays the writer
    // threads use, but far shorter than the default 500ms.
    cmd.args(["--interval-ms", "50"]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.spawn().expect("failed to spawn logwait")
}

/// Appends one line (with a trailing newline) to `path`, flushing so the
/// bytes are visible to a concurrent reader immediately.
fn append_line(path: &Path, line: &str) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open log file for append");
    writeln!(file, "{line}").expect("write line");
    file.flush().expect("flush appended line");
}

#[test]
fn success_flow_exits_zero() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let log_path = dir.path().join("install.log");
    File::create(&log_path).expect("create empty log file");

    let guard = ChildGuard::new(spawn_logwait(&log_path, &["--success", "exit code 0"]));

    let writer_path = log_path.clone();
    let writer = thread::spawn(move || {
        append_line(&writer_path, "starting install");
        thread::sleep(Duration::from_millis(120));
        append_line(&writer_path, "exit code 0");
    });

    let output = guard.wait_with_output();
    writer.join().expect("writer thread panicked");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(predicate::str::contains("starting install").eval(&stdout));
    assert!(predicate::str::contains("SUCCESS").eval(&stderr));
    assert!(predicate::str::contains("exit code 0").eval(&stderr));
}

#[test]
fn warn_and_fail_flows_exit_with_mapped_codes() {
    assert_terminal_flow("--warn", "exit code 7", 3, "WARN");
    assert_terminal_flow("--fail", "Failed", 4, "FAIL");
}

fn assert_terminal_flow(flag: &str, pattern: &str, expected_code: i32, expected_label: &str) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let log_path = dir.path().join("install.log");
    File::create(&log_path).expect("create empty log file");

    let guard = ChildGuard::new(spawn_logwait(&log_path, &[flag, pattern]));

    let writer_path = log_path.clone();
    let pattern_owned = pattern.to_string();
    let writer = thread::spawn(move || {
        append_line(&writer_path, "unrelated progress line");
        thread::sleep(Duration::from_millis(120));
        append_line(&writer_path, &pattern_owned);
    });

    let output = guard.wait_with_output();
    writer.join().expect("writer thread panicked");

    assert_eq!(output.status.code(), Some(expected_code));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(predicate::str::contains(expected_label).eval(&stderr));
}

#[test]
fn late_created_file_is_detected() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let log_path = dir.path().join("install.log"); // does not exist yet

    let guard = ChildGuard::new(spawn_logwait(&log_path, &["--success", "exit code 0"]));

    let writer_path = log_path.clone();
    let writer = thread::spawn(move || {
        // WHY: give logwait a chance to observe the missing file at least
        // once (and print its notice) before the file appears.
        thread::sleep(Duration::from_millis(150));
        let mut file = File::create(&writer_path).expect("create log file");
        writeln!(file, "exit code 0").expect("write line");
        file.flush().expect("flush");
    });

    let output = guard.wait_with_output();
    writer.join().expect("writer thread panicked");

    assert_eq!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(predicate::str::contains("waiting for file to appear").eval(&stderr));
    assert!(predicate::str::contains("SUCCESS").eval(&stderr));
}

#[test]
fn pre_existing_content_is_detected_near_immediately() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let log_path = dir.path().join("install.log");
    {
        let mut file = File::create(&log_path).expect("create log file");
        writeln!(file, "already finished before logwait started").expect("write");
        writeln!(file, "exit code 0").expect("write");
    }

    let start = Instant::now();
    let guard = ChildGuard::new(spawn_logwait(&log_path, &["--success", "exit code 0"]));
    let output = guard.wait_with_output();
    let elapsed = start.elapsed();

    assert_eq!(output.status.code(), Some(0));
    // WHY: this locks in the read-from-start decision (§5.2) -- a line that
    // was already in the file before logwait started must be detected on
    // essentially the first poll, not after some long, unrelated delay.
    assert!(
        elapsed < Duration::from_secs(2),
        "took too long: {elapsed:?}"
    );
}

#[test]
fn partial_line_is_not_matched_until_completed() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let log_path = dir.path().join("install.log");
    File::create(&log_path).expect("create empty log file");

    let guard = ChildGuard::new(spawn_logwait(&log_path, &["--success", "exit code 0"]));

    let writer_path = log_path.clone();
    let writer = thread::spawn(move || {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&writer_path)
            .expect("open for append");
        // WHY: no trailing newline yet -- this must not be treated as a
        // complete, matchable line.
        write!(file, "exit code 0").expect("write partial line");
        file.flush().expect("flush partial line");
        thread::sleep(Duration::from_millis(150)); // longer than one 50ms poll interval
        writeln!(file).expect("complete the line with a trailing newline");
        file.flush().expect("flush completed line");
    });

    let start = Instant::now();
    let output = guard.wait_with_output();
    let elapsed = start.elapsed();
    writer.join().expect("writer thread panicked");

    assert_eq!(output.status.code(), Some(0));
    // WHY: if the partial fragment had been matched early, logwait would
    // have exited right after the first write, well under the 150ms delay
    // before the line was completed.
    assert!(
        elapsed >= Duration::from_millis(140),
        "matched too early: {elapsed:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("SUCCESS").count(), 1);
}

#[test]
fn usage_error_without_any_pattern_exits_two() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let log_path = dir.path().join("install.log");

    let mut cmd = assert_cmd::Command::cargo_bin("logwait").expect("logwait binary should build");
    cmd.arg(&log_path);
    cmd.assert().failure().code(2);
}
