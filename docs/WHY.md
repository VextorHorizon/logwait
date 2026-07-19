# logwait — Why This Project Matters

## For the author

logwait is a deliberately laddered learning vehicle for Rust systems programming. v0.1 teaches file I/O, byte offsets, partial-read buffering, error handling without helper crates, and industry-standard CLI testing. Each later version is scoped to unlock exactly one new systems topic in turn: OS file-event APIs, process APIs, text encodings, concurrency. The working contract is that the author reads every line of the codebase and can answer *why* it is written that way — including the lines where AI tooling assisted with the writing.

The project also did not start as logwait. The first idea was a multi-source log correlator, merging logs from many services by trace id into a single timeline. Testing that idea against the author's actual experience showed the real pain wasn't correlation across sources at all — it was waiting for a completion signal in a single file. The idea was deliberately narrowed down to that core: build the smallest tool that kills the real pain, and let everything larger live in the roadmap. That narrowing is itself part of the portfolio evidence — it demonstrates the discipline of solving the problem you actually have, not the more impressive-sounding one.

Taken together, the repository is portfolio evidence of the full engineering arc: real problem → analysis → minimal solution → verification. The personal standard behind it is **"make it work → make it verifiable"** — every behavior documented in the README is covered by a test.

## For everyone

The pain generalizes far beyond SCCM. Any long-running process that writes a log with a known terminal line — installers, data migrations, batch jobs, backups, CI steps, model training runs — currently turns a human into a polling loop, watching for a signal a small program could watch for instead.

That's compounded by where this kind of tool actually needs to run: production machines, often locked down, where installing a language runtime is unacceptable. That's why logwait ships as a single static binary with an essentially zero runtime-dependency budget — copy one file, run it, and its exit code becomes a universal building block for shell chains and automation, on machines where heavier tooling could never be installed in the first place.

## Honest non-goals

logwait is not an observability platform, not a log aggregator, and not a replacement for structured logging or tracing. It does one thing.
