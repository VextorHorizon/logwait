# logwait — Origin

## 1. The manual procedure this tool replaces

During an enterprise endpoint migration, machines were migrated one at a time onto a new management stack, which included installing the SCCM (Microsoft Configuration Manager) client via `ccmsetup.exe`. `ccmsetup` is a long-running installer with no useful foreground feedback, so the outcome of every install had to be confirmed by hand, following this standard operating procedure:

1. Start `ccmsetup` on the target machine.
2. Wait an unknown amount of time — minutes to much longer; there is no way to predict it.
3. Open `ccmsetup.log` and scroll to the end, looking for an exit-code line.
4. If it isn't there yet, close the log and go do something else.
5. Remember to come back and check again. Repeat from step 3 until the line finally appears.
6. Read which exit code it was, and act accordingly: `exit code 0` — done; `exit code 7` — done, but a reboot is required; anything else — troubleshoot.

## 2. Cost analysis

Step 5 is the real cost. The human becomes a manual polling loop: opening, scrolling, closing, and — worse — holding the mental state "I must go check that log again" alive for the entire wait, on top of whatever else is being worked on in parallel. That retained state taxes focus continuously, not just at the moment of checking.

The procedure also creates a specific misreading risk: `exit code 7` looks like a failure to an inattentive reader, but it means success with a reboot required. Missing that distinction, or treating it as a failure, wastes time it was never supposed to cost. Multiply the whole procedure — the waiting, the interruption, and the risk of misreading — across a fleet of machines migrated one at a time, and the overhead comes to dominate the actual work.

## 3. The trigger insight

The insight that triggered this project: the waiting, not the reading, was the real cost — and a program can hold that waiting state at zero cognitive cost. Time itself is not a variable the tool needs to reason about; it never knows or guesses *when* the job will finish, only *what* finishing looks like. It can wait ten seconds or a full day and the logic is identical, which means the human never has to hold "check again later" in their head at all.

## 4. The generalization

The pain is not specific to SCCM or to endpoint migrations. It exists for any long-running process that writes a log with a known terminal line: installers, data migrations, batch jobs, backups, CI steps, model training runs. In every case, a human is currently reduced to a polling loop for a signal that a small program could watch for instead.
