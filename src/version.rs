//! frontmatter の version と、版ごとの移行の手順。docs/rule/docs/guideline.md の 3 章、docv/service/manual.yaml の docv migrate-guide。
//!
//! frontmatter の version は、そのファイルが従う docv の版（メジャーとマイナー）。
//! 移行の手順は、版ごとに「1 つ前の版から」だけを持つ。古いファイルには、手順を順につないで示す。

use std::fmt;

use serde_yaml_ng::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl Version {
    /// `0.1` のような文字列か数を読む。
    pub fn parse(value: &Value) -> Option<Version> {
        let text = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return None,
        };
        let (major, minor) = text.trim().split_once('.')?;
        Some(Version {
            major: major.parse().ok()?,
            minor: minor.parse().ok()?,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// この CLI の版。frontmatter の version と比べる。
pub fn current() -> Version {
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    let mut next = || parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Version {
        major: next(),
        minor: next(),
    }
}

/// 1 つ前の版から、その版へ移るときの手順。版の順に並べる。版を上げるときは、末尾に 1 つ足すだけでよい。
pub struct Migration {
    pub from: Version,
    pub to: Version,
    pub steps: &'static [&'static str],
}

pub const MIGRATIONS: &[Migration] = &[];

/// from の版から、この CLI の版までの手順を、古い順に返す。
pub fn path_from(from: Version) -> Vec<&'static Migration> {
    path_between(MIGRATIONS, from, current())
}

fn path_between(
    migrations: &'static [Migration],
    from: Version,
    to: Version,
) -> Vec<&'static Migration> {
    migrations
        .iter()
        .filter(|m| m.from >= from && m.to <= to)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u32, minor: u32) -> Version {
        Version { major, minor }
    }

    #[test]
    fn parses_strings_and_numbers() {
        assert_eq!(Version::parse(&Value::from("1.10")), Some(v(1, 10)));
        assert_eq!(
            Version::parse(&serde_yaml_ng::from_str("0.1").unwrap()),
            Some(v(0, 1))
        );
        assert_eq!(Version::parse(&Value::from("1")), None);
        assert!(v(1, 2) < v(1, 10));
    }

    #[test]
    fn chains_one_step_migrations() {
        static ALL: &[Migration] = &[
            Migration {
                from: Version { major: 1, minor: 0 },
                to: Version { major: 1, minor: 1 },
                steps: &["a"],
            },
            Migration {
                from: Version { major: 1, minor: 1 },
                to: Version { major: 1, minor: 2 },
                steps: &["b"],
            },
            Migration {
                from: Version { major: 1, minor: 2 },
                to: Version { major: 1, minor: 3 },
                steps: &["c"],
            },
        ];
        let steps: Vec<&str> = path_between(ALL, v(1, 0), v(1, 3))
            .iter()
            .flat_map(|m| m.steps.iter().copied())
            .collect();
        assert_eq!(steps, ["a", "b", "c"]);
        assert_eq!(path_between(ALL, v(1, 2), v(1, 3)).len(), 1);
    }
}
