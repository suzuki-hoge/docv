//! check の結果の 1 件。出力の形は docv/service/manual.md の docv check。

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub rule: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {} {}",
            self.path, self.line, self.rule, self.message
        )
    }
}
