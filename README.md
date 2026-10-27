# logwait

logwait watches a log file that another process is writing, and exits with a meaningful code the moment a line you care about appears.

[![CI](https://github.com/VextorHorizon/logwait/actions/workflows/ci.yml/badge.svg)](https://github.com/VextorHorizon/logwait/actions/workflows/ci.yml)

> **Status: no longer maintained.** Development stopped in October 2026. The code works as described below, but the issues under [Known limitations](#known-limitations) will not be fixed.

## Why this exists

You start a long-running install, migration, or batch job, and the only way to know it finished is to keep re-opening its log file and scrolling to the end. You end up acting as a manual polling loop: check, nothing yet, go do something else, remember to come back, repeat — and the waiting itself, not the reading, is what drains your attention from everything else you're doing. logwait takes that loop over: point it at the log and the patterns that mean success, warning, or failure, and it exits the instant one of them shows up.

## Quick start

```
cargo install --path .
```

Then point it at a real log:

```
logwait C:\Windows\ccmsetup\Logs\ccmsetup.log --success "exit code 0"
```

No installer handy to try it against? Run the bundled demo script (`examples/simulate_log.ps1` on Windows, `examples/simulate_log.sh` on Linux/macOS) in one terminal — it appends lines to a demo log, ending with an `exit code 0` line — and point logwait at that same file in another terminal to watch it detect completion live.

## Usage

```
logwait C:\Windows\ccmsetup\Logs\ccmsetup.log --success "exit code 0" --warn "exit code 7" --fail "Failed"
```

Each of `--success`, `--warn`, and `--fail` is repeatable, so you can watch for more than one phrasing of the same outcome:

```
logwait app.log --success "exit code 0" --success "already installed" --fail "Failed" --fail "ERROR"
```

Because the exit code is the result, logwait chains directly into your next step — no human needs to be watching:

```
logwait ccmsetup.log --success "exit code 0" && shutdown /r
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | a `--success` pattern matched |
| `3` | a `--warn` pattern matched |
| `4` | a `--fail` pattern matched |
| `1` | runtime error (I/O failure, etc.) |
| `2` | CLI usage error (clap default) |

## How it works

logwait polls the file on a fixed interval (default `500`ms, configurable with `--interval-ms`) rather than relying on OS file-change events. It tracks a byte offset and reads only the bytes appended since the last check, starting from the very beginning of the file so a completion line written before logwait started is still caught (this also has a downside, see [Known limitations](#known-limitations)). Only complete, newline-terminated lines are handed to the matcher — a trailing partial line is buffered and completed on the next read, since a half-written line can otherwise cause a false match or a missed one. The moment a line matches one of your patterns, logwait prints the matched line and a verdict, then exits with the corresponding code.

## Known limitations

- Stale matches: logwait reads from byte 0, so if the log still holds a matching line from an earlier run (an old `exit code 0`, say), it exits with that verdict right away. Clear the log before starting the job.
- No timeout: if no pattern ever appears, logwait waits forever.
- Matching is plain substring, case-sensitive. There is no regex, so a rule like "any exit code except 0 or 7 is a failure" can't be expressed.
- Non-UTF-8 log files (some Windows tools emit UTF-16) are decoded lossily; garbled bytes won't crash logwait, but they also won't match correctly.
- Polling backend: on an interval rather than OS file-change events, so detection latency is bounded by `--interval-ms`, not instant.
- One file at a time.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
