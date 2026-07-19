//! Pure pattern-matching core. Given a complete line and the three pattern
//! sets from the CLI, decides which `Verdict` (if any) applies. No I/O and
//! no state here — this is the innermost stage of the data flow:
//! file -> tailer -> matcher -> verdict -> exit code.

/// The three possible outcomes of matching a line against the pattern sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Success,
    Warn,
    Fail,
}

/// Checks a single line against the three pattern categories and returns the
/// verdict that applies together with the pattern that matched, or `None` if
/// none of the patterns matched.
///
/// WHY: fail is checked before warn before success, so a line matching more
/// than one category is reported as the worst of the matches. A false alarm
/// (reporting fail/warn too eagerly) is cheaper than silently missing a real
/// failure.
pub fn evaluate<'a>(
    line: &str,
    success: &'a [String],
    warn: &'a [String],
    fail: &'a [String],
) -> Option<(Verdict, &'a str)> {
    if let Some(pattern) = first_match(line, fail) {
        return Some((Verdict::Fail, pattern));
    }
    if let Some(pattern) = first_match(line, warn) {
        return Some((Verdict::Warn, pattern));
    }
    if let Some(pattern) = first_match(line, success) {
        return Some((Verdict::Success, pattern));
    }
    None
}

/// Returns the first pattern that occurs as a substring of `line`, using
/// plain case-sensitive substring matching.
///
/// WHY: the target log lines are stable literal strings (e.g. "exit code
/// 0"); regex would add a dependency and escaping complexity with no v0.1
/// value (regex arrives in roadmap v0.2).
fn first_match<'a>(line: &str, patterns: &'a [String]) -> Option<&'a str> {
    patterns
        .iter()
        .find(|p| line.contains(p.as_str()))
        .map(|p| p.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn success_pattern_matches() {
        let success = patterns(&["exit code 0"]);
        let warn = patterns(&[]);
        let fail = patterns(&[]);
        let result = evaluate("installer finished: exit code 0", &success, &warn, &fail);
        assert_eq!(result, Some((Verdict::Success, "exit code 0")));
    }

    #[test]
    fn warn_pattern_matches() {
        let success = patterns(&[]);
        let warn = patterns(&["exit code 7"]);
        let fail = patterns(&[]);
        let result = evaluate("installer finished: exit code 7", &success, &warn, &fail);
        assert_eq!(result, Some((Verdict::Warn, "exit code 7")));
    }

    #[test]
    fn fail_pattern_matches() {
        let success = patterns(&[]);
        let warn = patterns(&[]);
        let fail = patterns(&["Failed"]);
        let result = evaluate("installer status: Failed", &success, &warn, &fail);
        assert_eq!(result, Some((Verdict::Fail, "Failed")));
    }

    #[test]
    fn precedence_is_fail_over_warn_over_success() {
        let success = patterns(&["ok"]);
        let warn = patterns(&["reboot"]);
        let fail = patterns(&["error"]);
        let line = "ok reboot error all in one line";
        assert_eq!(
            evaluate(line, &success, &warn, &fail),
            Some((Verdict::Fail, "error"))
        );

        let success = patterns(&["ok"]);
        let warn = patterns(&["reboot"]);
        let fail = patterns(&[]);
        let line = "ok reboot but no failure here";
        assert_eq!(
            evaluate(line, &success, &warn, &fail),
            Some((Verdict::Warn, "reboot"))
        );
    }

    #[test]
    fn matching_is_case_sensitive() {
        let success = patterns(&["Exit Code 0"]);
        let warn = patterns(&[]);
        let fail = patterns(&[]);
        let result = evaluate("exit code 0", &success, &warn, &fail);
        assert_eq!(result, None);
    }

    #[test]
    fn no_match_returns_none() {
        let success = patterns(&["exit code 0"]);
        let warn = patterns(&["exit code 7"]);
        let fail = patterns(&["Failed"]);
        let result = evaluate("still running, nothing to see yet", &success, &warn, &fail);
        assert_eq!(result, None);
    }
}
