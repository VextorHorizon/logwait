#!/usr/bin/env bash
# Simulates a long-running installer appending to a log file, for manual
# smoke-testing logwait. Run this in one terminal:
#   ./examples/simulate_log.sh /tmp/demo.log
# and, in another, point logwait at the same file:
#   logwait /tmp/demo.log --success "exit code 0" --warn "exit code 7" --fail "Failed"

set -euo pipefail

LOG_FILE="${1:-demo.log}"

echo "Writing demo log lines to: $LOG_FILE"
: > "$LOG_FILE"

append() {
    echo "$1" >> "$LOG_FILE"
    sleep 1
}

append "installer started"
append "copying files"
append "configuring services"
append "finishing up"
append "exit code 0"

echo "Done."
