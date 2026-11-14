//! docv の版。docv.yaml の docv_version と、YAML の 1 行目のスキーマの URL の版を確かめる。docv/slot/service/service/config.md の「docv の版」。

use std::fmt;
use std::fs;
use std::str::FromStr;

use anyhow::{Context, Result};

use crate::diagnostic::Diagnostic;
use crate::schema;
use crate::workspace::Project;

/// `1.8.0` の形の版。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u32, u32, u32);

/// 最初の版。docv update-guide は、この次の版からの変更点を持つ。
pub const FIRST: Version = Version(1, 0, 0);

impl Version {
    /// 実行している CLI の版。
    pub fn current() -> Version {
        env!("CARGO_PKG_VERSION")
            .parse()
            .expect("Cargo.toml の版は 1.8.0 の形")
    }

    /// コマンドラインで受け取った版。スキーマの URL と同じ `v1.8.0` の形も受け付ける。
    pub fn from_arg(text: &str) -> Option<Version> {
        text.strip_prefix('v').unwrap_or(text).parse().ok()
    }
}

impl FromStr for Version {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = text.split('.').collect();
        let [major, minor, patch] = parts.as_slice() else {
            return Err(());
        };
        let number = |part: &str| {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(());
            }
            part.parse::<u32>().map_err(|_| ())
        };
        Ok(Version(number(major)?, number(minor)?, number(patch)?))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// YAML の 1 行目に docv のスキーマの URL があれば、その版。ローカルのパスなど、版のタグの URL でなければ None。
pub fn schema_version(text: &str) -> Option<Version> {
    let line = text.lines().next()?;
    let (_, rest) = line.split_once(schema::URL_BASE)?;
    let (version, _) = rest.split_once('/')?;
    version.parse().ok()
}

/// docv_version を docv の版と比べ、違えば直し方を返す。
/// docv_version がなければ、docv.yaml の 1 行目のスキーマの URL から、使っていた版を推す。
pub fn advice(declared: Option<Version>, config_text: &str) -> Option<String> {
    let current = Version::current();
    match declared {
        Some(version) if version == current => None,
        Some(version) if version > current => Some(format!(
            "docv_version は {version} で、docv は {current}。docv を {version} にする（cargo install --git https://github.com/suzuki-hoge/docv --tag v{version}）"
        )),
        Some(version) => Some(format!(
            "docv_version は {version} で、docv は {current}。docv update-guide --from {version} で変わったことと対応を確かめ、対応してから docv_version を {current} にする"
        )),
        None => {
            let (from, note) = match schema_version(config_text).filter(|v| *v <= current) {
                Some(version) => (
                    version,
                    format!("{version} は 1 行目のスキーマの URL から推した版"),
                ),
                None => (
                    FIRST,
                    "使っていた版が分からないので、最初の版からにした".to_string(),
                ),
            };
            Some(format!(
                "docv_version がない。docv update-guide --from {from} で変わったことと対応を確かめ、対応してから docv_version: {current} を書く（{note}）"
            ))
        }
    }
}

/// スキーマに合わず読めなかった docv.yaml から、docv_version の直し方を返す。YAML として読めなければ None。
pub fn advice_for_unreadable(config_text: &str) -> Option<String> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(config_text).ok()?;
    let declared = match value.get("docv_version") {
        None => None,
        Some(version) => Some(version.as_str()?.parse().ok()?),
    };
    advice(declared, config_text)
}

/// docv.yaml の docv_version と、docv.yaml と YAML の Slot の 1 行目のスキーマの URL の版を確かめる。
/// スキーマの URL は docv_version と比べる。docv_version がなければ、先に docv_version を直すので比べない。
pub fn check(project: &Project) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let config_text = fs::read_to_string(&project.config_path)
        .with_context(|| format!("{} を読めない", project.config_path.display()))?;
    let config_path = project.config_path.strip_prefix(&project.root).map_or_else(
        |_| project.config_path.display().to_string(),
        crate::markdown::slash_path,
    );
    let declared = project
        .config
        .docv_version
        .as_deref()
        .and_then(|v| v.parse().ok());
    if let Some(message) = advice(declared, &config_text) {
        let line = config_text
            .lines()
            .position(|l| l.starts_with("docv_version:"))
            .map_or(1, |i| i + 1);
        diagnostics.push(Diagnostic {
            path: config_path,
            line,
            rule: "docv-version".to_string(),
            message,
        });
    }
    let Some(declared) = declared else {
        return Ok(diagnostics);
    };
    for path in project.yaml_targets(&[])? {
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        if let Some(version) = schema_version(&text)
            && version != declared
        {
            diagnostics.push(Diagnostic {
                path,
                line: 1,
                rule: "schema-version".to_string(),
                message: format!(
                    "スキーマの URL の版は v{version} で、docv_version は {declared}。URL の版を v{declared} にする"
                ),
            });
        }
    }
    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn url(version: &str, file: &str) -> String {
        format!(
            "# yaml-language-server: $schema={}{version}/assets/schema/{file}\n",
            schema::URL_BASE
        )
    }

    fn check_files(files: &[(&str, &str)]) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        check(&project)
            .unwrap()
            .iter()
            .map(|d| d.to_string())
            .collect()
    }

    #[test]
    fn parses_versions() {
        assert_eq!("1.8.0".parse(), Ok(Version(1, 8, 0)));
        assert_eq!(Version::from_arg("v1.10.2"), Some(Version(1, 10, 2)));
        for text in ["1.8", "1.8.0.1", "1.x.0", "", "1..0", "+1.0.0", "1.8.0-rc"] {
            assert_eq!(text.parse::<Version>(), Err(()), "{text}");
        }
        assert!(Version(1, 10, 0) > Version(1, 9, 9));
        assert_eq!(Version(1, 8, 0).to_string(), "1.8.0");
        assert!(Version::current() >= FIRST);
    }

    #[test]
    fn reads_the_version_from_the_schema_url() {
        assert_eq!(
            schema_version(&url("1.4.0", "docv.schema.json")),
            Some(Version(1, 4, 0))
        );
        assert_eq!(
            schema_version(&schema::header("words.schema.json")),
            Some(Version::current())
        );
        // 1 行目だけを見る。ローカルのパスと、版のタグでない URL は読まない
        assert_eq!(
            schema_version(&format!("a: 1\n{}", url("1.4.0", "x"))),
            None
        );
        assert_eq!(
            schema_version(
                "# yaml-language-server: $schema=../../assets/schema/words.schema.json\n"
            ),
            None
        );
        assert_eq!(
            schema_version(
                "# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/main/assets/schema/x\n"
            ),
            None
        );
    }

    #[test]
    fn advises_how_to_follow_the_version() {
        let current = Version::current();
        assert_eq!(advice(Some(current), ""), None);
        let older = advice(Some(Version(1, 0, 0)), "").unwrap();
        assert!(
            older.contains(&format!(
                "docv update-guide --from 1.0.0 で変わったことと対応を確かめ、対応してから docv_version を {current} にする"
            )),
            "{older}"
        );
        let newer = advice(Some(Version(99, 0, 0)), "").unwrap();
        assert!(newer.contains("docv を 99.0.0 にする"), "{newer}");
        // docv_version がなければ、1 行目のスキーマの URL から使っていた版を推す。推せなければ最初の版から
        let missing = advice(None, &url("1.4.0", "docv.schema.json")).unwrap();
        assert!(
            missing.contains(&format!(
                "docv update-guide --from 1.4.0 で変わったことと対応を確かめ、対応してから docv_version: {current} を書く（1.4.0 は 1 行目のスキーマの URL から推した版）"
            )),
            "{missing}"
        );
        let unknown = advice(None, "title: t\n").unwrap();
        assert!(unknown.contains("--from 1.0.0"), "{unknown}");
        assert!(unknown.contains("最初の版からにした"), "{unknown}");
    }

    #[test]
    fn advises_for_unreadable_config() {
        let text = format!("{}dir:\n  slots: docv\n", url("1.0.0", "docv.schema.json"));
        let advice = advice_for_unreadable(&text).unwrap();
        assert!(advice.contains("--from 1.0.0"), "{advice}");
        assert_eq!(
            advice_for_unreadable(&format!(
                "docv_version: {}\nunknown: 1\n",
                Version::current()
            )),
            None
        );
        assert_eq!(advice_for_unreadable("a: [\n"), None);
    }

    #[test]
    fn checks_docv_version() {
        let current = Version::current();
        assert!(check_files(&[("docv.yaml", &format!("docv_version: {current}\n"))]).is_empty());
        assert_eq!(
            check_files(&[("docv.yaml", "title: t\ndocv_version: 1.0.0\n")]),
            [format!(
                "docv.yaml:2: docv-version docv_version は 1.0.0 で、docv は {current}。docv update-guide --from 1.0.0 で変わったことと対応を確かめ、対応してから docv_version を {current} にする"
            )]
        );
        let missing = check_files(&[("docv.yaml", "title: t\n")]);
        assert_eq!(missing.len(), 1);
        assert!(
            missing[0].starts_with("docv.yaml:1: docv-version docv_version がない。"),
            "{missing:?}"
        );
    }

    #[test]
    fn checks_schema_urls_against_docv_version() {
        let current = Version::current();
        let config = format!(
            "{}docv_version: {current}\n",
            url("1.4.0", "docv.schema.json")
        );
        let words = format!("{}groups: []\n", url("1.4.0", "words.schema.json"));
        let result = check_files(&[
            ("docv.yaml", &config),
            ("docv/slot/service/words.yaml", &words),
            // 今の版の URL と、ローカルのパスは指摘しない
            (
                "docv/slot/system/stack.yaml",
                &format!("{}stack: []\n", schema::header("stack.schema.json")),
            ),
            (
                "docv/slot/guide/env.yaml",
                "# yaml-language-server: $schema=../../assets/schema/env.schema.json\nenvs: []\n",
            ),
        ]);
        let message = format!(
            "schema-version スキーマの URL の版は v1.4.0 で、docv_version は {current}。URL の版を v{current} にする"
        );
        assert_eq!(
            result,
            [
                format!("docv.yaml:1: {message}"),
                format!("docv/slot/service/words.yaml:1: {message}"),
            ]
        );
        // docv_version がなければ、先に docv_version を直すので、URL は比べない
        let result = check_files(&[
            (
                "docv.yaml",
                &format!("{}title: t\n", url("1.4.0", "docv.schema.json")),
            ),
            ("docv/slot/service/words.yaml", &words),
        ]);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("docv-version"), "{result:?}");
    }
}
