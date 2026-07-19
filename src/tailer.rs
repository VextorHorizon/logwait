//! File tailer: polls a log file for new bytes, tracks a byte offset across
//! polls, and buffers partial (not-yet-newline-terminated) lines. Sits
//! between the raw file on disk and the matcher: file -> tailer -> matcher
//! -> verdict -> exit code.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::AppError;

/// Tails a single file from the beginning, surviving the file not existing
/// yet and simple truncation. Full rename-and-recreate rotation handling is
/// roadmap material.
pub struct Tailer {
    path: PathBuf,
    // WHY: u64, not usize -- a byte offset into a file is inherently a
    // 64-bit quantity on both target platforms and must not truncate on
    // 32-bit builds.
    offset: u64,
    // Fragment left over from the previous read that had no trailing '\n'
    // yet; prepended to the next chunk before it is split into lines.
    carry: String,
    waiting_notice_printed: bool,
}

impl Tailer {
    /// Creates a tailer that will read `path` from the very beginning.
    ///
    /// WHY: this is a completion detector, not a live monitor. If the
    /// terminal line was already written before logwait started, starting
    /// at end-of-file would wait forever on a job that already finished.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            offset: 0,
            carry: String::new(),
            waiting_notice_printed: false,
        }
    }

    /// Polls the file once and returns any complete lines found since the
    /// last poll. Returns an empty vec while the file does not exist yet, or
    /// when no new bytes have arrived since the last poll.
    pub fn poll(&mut self) -> Result<Vec<String>, AppError> {
        let metadata = match fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // WHY: in the real workflow the watcher is often started
                // before the installer creates its log file, so a missing
                // file is expected transient state, not an error. The
                // notice is printed once so a slow file creation doesn't
                // spam stderr once per poll interval.
                if !self.waiting_notice_printed {
                    eprintln!("logwait: waiting for file to appear");
                    self.waiting_notice_printed = true;
                }
                return Ok(Vec::new());
            }
            Err(err) => return Err(err.into()),
        };

        let len = metadata.len();

        if len < self.offset {
            // WHY: a file shrinking since the last poll means truncation or
            // rotation. Full rename-and-recreate rotation handling is
            // roadmap material; v0.1 just warns and restarts from the top,
            // discarding the carry since it belonged to the old file content.
            eprintln!("logwait: file shrank, assuming truncation; re-reading from start");
            self.offset = 0;
            self.carry.clear();
        }

        if len == self.offset {
            return Ok(Vec::new());
        }

        let mut file = File::open(&self.path)?;
        // WHY: seek to the known offset and read only the new bytes instead
        // of re-reading the whole file every poll, so the cost of a poll is
        // proportional to what changed, not to the file's total size.
        file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        self.offset += buf.len() as u64;

        // WHY: never crash on stray or non-UTF-8 bytes. UTF-16-encoded
        // Windows logs are a documented known limitation until the encoding
        // roadmap item (v0.6).
        let chunk = String::from_utf8_lossy(&buf);
        self.carry.push_str(&chunk);

        Ok(self.drain_complete_lines())
    }

    /// Splits off every newline-terminated line currently in `carry`,
    /// leaving any trailing fragment (a line the writer may still be
    /// mid-write on) buffered for the next poll.
    ///
    /// WHY: a reader can observe a line the writer has only half written,
    /// because the read can land mid-write. Matching on such fragments would
    /// produce both false negatives and false positives, so only complete,
    /// newline-terminated lines are ever handed to the matcher.
    fn drain_complete_lines(&mut self) -> Vec<String> {
        let data = std::mem::take(&mut self.carry);

        let Some(last_newline) = data.rfind('\n') else {
            self.carry = data;
            return Vec::new();
        };

        let complete = &data[..last_newline];
        let remainder = &data[last_newline + 1..];

        let lines = complete
            .split('\n')
            .map(|part| {
                // WHY: strip a trailing '\r' to tolerate CRLF-terminated
                // Windows log files without treating the carriage return as
                // part of the matchable content.
                part.strip_suffix('\r').unwrap_or(part).to_string()
            })
            .collect();

        self.carry = remainder.to_string();
        lines
    }
}
