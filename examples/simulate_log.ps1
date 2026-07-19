# Simulates a long-running installer appending to a log file, for manual
# smoke-testing logwait. Run this in one PowerShell window:
#   .\examples\simulate_log.ps1 -LogFile .\demo.log
# and, in another, point logwait at the same file:
#   logwait .\demo.log --success "exit code 0" --warn "exit code 7" --fail "Failed"

param(
    [string]$LogFile = "demo.log"
)

Write-Host "Writing demo log lines to: $LogFile"
New-Item -Path $LogFile -ItemType File -Force | Out-Null

function Append-Line {
    param([string]$Line)
    Add-Content -Path $LogFile -Value $Line
    Start-Sleep -Seconds 1
}

Append-Line "installer started"
Append-Line "copying files"
Append-Line "configuring services"
Append-Line "finishing up"
Append-Line "exit code 0"

Write-Host "Done."
