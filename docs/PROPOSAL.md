# logwait — Project Proposal

## Problem statement

During an enterprise endpoint migration, machines were migrated one at a time. Each install ran through `ccmsetup.exe`, a long-running installer with no useful foreground feedback — the only reliable signal of its outcome is a line appended to `ccmsetup.log` once it finishes. Waiting for that line meant repeatedly opening the log, scrolling to the end, finding nothing, closing it, doing something else, and remembering to come back — over and over, across a whole fleet of machines.

The real cost was never the reading. It was the *waiting with retained state*: the brain has to keep "I must go check that log again" alive for the entire duration, which taxes focus on everything else being done in parallel. This is a manual polling loop performed by a human, and it does not scale.

## Existing tools and why they don't fit

- **`matcharr/logwatcher` (Rust)** — real-time log monitoring with pattern highlighting, desktop notifications, regex, multi-file support, and rotation handling. But it is a monitor-forever tool: it has no completion semantics and no way to exit with a meaningful code once a line appears.
- **`nemethi/loop`** — re-runs a *command* until an exit code or output pattern appears. It owns the process it runs, so it cannot watch a file that some *other* process is writing.
- **`waitforoutput` (PyPI)** — the closest semantics: wait for a pattern, return the result as an exit code. But it is oriented to stdout/docker logs and requires a Python runtime on the target machine.

## The gap

**A completion detector for files written by another process, shipped as one static binary, whose exit code is the API.** None of the surveyed tools combine all three properties.

## Design principles

1. **Single static binary, minimal dependencies.** Trivially copyable onto production Windows machines with no runtime installed. Every dependency must justify its existence.
2. **Exit-code-first design.** The primary API is the exit code; human-readable output is secondary.
3. **Verifiable behavior.** Every behavior described in the README is covered by a test. Motto: *make it work → make it verifiable*.
4. **Learning codebase.** Built as a laddered learning vehicle for Rust systems programming — comments explaining *why* are a deliverable, not decoration.

## Success criteria for v0.1

Green CI on both Windows and Linux, covering:

1. Success flow — a `--success` pattern appears and logwait exits `0`.
2. Warn / fail flows — `--warn` and `--fail` patterns exit `3` and `4` respectively.
3. Late-created file — logwait is started before the file exists, then detects it once created.
4. Pre-existing content — the terminal line is already in the file before launch, and logwait exits almost immediately (locking in the read-from-start decision).
5. Partial line — a line written without its trailing newline is not matched until it is completed.
6. Usage error — no patterns supplied exits `2`.
