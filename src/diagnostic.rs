//! check の結果の 1 件。出力の形は docv/slot/service/manual.md の docv check。
//!
//! 直さなくても困らないもの（打ち消された決定を指しているなど）は warning にし、ほかは error にする。
//! 終了コードを 1 にするのは error だけ。

use std::fmt;
use std::fmt::Write as _;

/// 指摘の重さ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    /// 直さなくても困らないもの。終了コードを変えない。
    Warning,
}

/// 並びは、パス、行、重さ（error が先）、ルール、メッセージの順。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub severity: Severity,
    pub rule: String,
    pub message: String,
}

impl Diagnostic {
    pub fn error(
        path: impl Into<String>,
        line: usize,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, path, line, rule, message)
    }

    pub fn warning(
        path: impl Into<String>,
        line: usize,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warning, path, line, rule, message)
    }

    fn new(
        severity: Severity,
        path: impl Into<String>,
        line: usize,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            line,
            severity,
            rule: rule.into(),
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// error は `<パス>:<行>: <ルール> <メッセージ>`、warning はルールの前に `warning` を付ける。
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "",
            Severity::Warning => "warning ",
        };
        write!(
            f,
            "{}:{}: {severity}{} {}",
            self.path, self.line, self.rule, self.message
        )
    }
}

/// docv check の出力。指摘を 1 行ずつ並べ、最後に `error N 件` を、warning があれば続けて `warning N 件` を出す。
pub fn report(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        let _ = writeln!(out, "{d}");
    }
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    let warnings = diagnostics.len() - errors;
    let _ = writeln!(out, "error {errors} 件");
    if warnings > 0 {
        let _ = writeln!(out, "warning {warnings} 件");
    }
    out
}

/// error が 1 件でもあるか。warning だけなら false で、終了コードは 0 にする。
pub fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(Diagnostic::is_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_warning_before_the_rule() {
        assert_eq!(
            Diagnostic::error("a.md", 2, "link-broken", "リンク先 b.md がない").to_string(),
            "a.md:2: link-broken リンク先 b.md がない"
        );
        assert_eq!(
            Diagnostic::warning("a.md", 3, "decision-superseded", "x").to_string(),
            "a.md:3: warning decision-superseded x"
        );
    }

    #[test]
    fn counts_errors_and_warnings_apart() {
        let error = Diagnostic::error("a.md", 2, "link-broken", "x");
        let warning = Diagnostic::warning("a.md", 3, "decision-superseded", "y");
        // warning がなければ、error の件数だけを出す
        assert_eq!(
            report(std::slice::from_ref(&error)),
            "a.md:2: link-broken x\nerror 1 件\n"
        );
        assert!(has_error(std::slice::from_ref(&error)));
        // warning だけなら、error 0 件に続けて warning の件数を出し、失敗にしない
        assert_eq!(
            report(std::slice::from_ref(&warning)),
            "a.md:3: warning decision-superseded y\nerror 0 件\nwarning 1 件\n"
        );
        assert!(!has_error(std::slice::from_ref(&warning)));
        let both = [error, warning];
        assert_eq!(
            report(&both),
            "a.md:2: link-broken x\na.md:3: warning decision-superseded y\nerror 1 件\nwarning 1 件\n"
        );
        assert!(has_error(&both));
        assert_eq!(report(&[]), "error 0 件\n");
        assert!(!has_error(&[]));
    }

    #[test]
    fn sorts_errors_before_warnings_on_the_same_line() {
        let mut found = [
            Diagnostic::warning("a.md", 2, "decision-superseded", "y"),
            Diagnostic::error("a.md", 2, "z-rule", "x"),
        ];
        found.sort();
        assert!(found[0].is_error());
    }
}
