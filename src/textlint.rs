//! textlint を npx で実行する。docs/spec/config.md の 6 章。

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::config::{self, TextlintLevel};
use crate::diagnostic::{Diagnostic, Level};
use crate::workspace::Project;

/// 組み込みの使い方で使うパッケージ。docv の版ごとに固定する。
const TEXTLINT: &str = "textlint@15.8.0";
const PRESETS: &[(&str, &str)] = &[
    (
        "ja-technical-writing",
        "textlint-rule-preset-ja-technical-writing@12.0.2",
    ),
    ("ja-spacing", "textlint-rule-preset-ja-spacing@3.0.3"),
    ("jtf-style", "textlint-rule-preset-jtf-style@3.0.3"),
];
const PRH: &str = "textlint-rule-prh@6.1.0";

pub struct Textlint<'a> {
    project: &'a Project,
    config: &'a config::Textlint,
}

impl<'a> Textlint<'a> {
    /// lint.textlint.enabled が true のときだけ返す。
    pub fn new(project: &'a Project) -> Option<Self> {
        let config = &project.config.lint.textlint;
        config.enabled.then_some(Textlint { project, config })
    }

    /// files はルートからの相対パス。
    pub fn lint(&self, files: &[String]) -> Result<Vec<Diagnostic>> {
        if files.is_empty() {
            return Ok(Vec::new());
        }
        let mut command = self.command()?;
        command.args(["--format", "json"]).args(files);
        let output = run(command)?;
        let results: Vec<FileResult> =
            serde_json::from_slice(&output.stdout).with_context(|| {
                format!(
                    "textlint の結果を読めない\n{}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )
            })?;
        let level = match self.config.level {
            TextlintLevel::Warn => Level::Warn,
            TextlintLevel::Error => Level::Error,
        };
        let mut diagnostics = Vec::new();
        for result in results {
            let path = self.relative(&result.file_path, files);
            for message in result.messages {
                diagnostics.push(Diagnostic {
                    path: path.clone(),
                    line: message.line,
                    rule: format!(
                        "textlint/{}",
                        message.rule_id.as_deref().unwrap_or("unknown")
                    ),
                    level,
                    message: message
                        .message
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                });
            }
        }
        Ok(diagnostics)
    }

    /// textlint の自動修正でファイルを書き換える。
    pub fn fix(&self, files: &[String]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let mut command = self.command()?;
        command.arg("--fix").args(files);
        run(command)?;
        Ok(())
    }

    fn command(&self) -> Result<Command> {
        let mut command = Command::new(if cfg!(windows) { "npx.cmd" } else { "npx" });
        command.current_dir(&self.project.root);
        match &self.config.config {
            Some(config) => {
                command.args(["--no", "textlint", "--config", config]);
            }
            None => {
                command.arg("--yes");
                for package in packages(self.config) {
                    command.args(["-p", package]);
                }
                let rc = write_builtin_config(&builtin_config(self.config, &self.project.root))?;
                command.arg("textlint").arg("--config").arg(rc);
            }
        }
        Ok(command)
    }

    /// textlint が出す絶対パスを、ルートからの相対パスにする。
    fn relative(&self, file_path: &str, files: &[String]) -> String {
        let path = Path::new(file_path);
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Ok(rel) = path.strip_prefix(&self.project.root) {
            return crate::markdown::slash_path(rel);
        }
        files
            .iter()
            .find(|f| file_path.replace('\\', "/").ends_with(f.as_str()))
            .cloned()
            .unwrap_or_else(|| file_path.to_string())
    }
}

#[derive(Deserialize)]
struct FileResult {
    #[serde(rename = "filePath")]
    file_path: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(rename = "ruleId")]
    rule_id: Option<String>,
    message: String,
    line: usize,
}

fn packages(config: &config::Textlint) -> Vec<&'static str> {
    let mut packages = vec![TEXTLINT];
    for (name, package) in PRESETS {
        if config.presets.iter().any(|p| p == name) {
            packages.push(package);
        }
    }
    if !config.prh.is_empty() {
        packages.push(PRH);
    }
    packages
}

/// 組み込みの使い方の、textlint の設定ファイルの中身。
fn builtin_config(config: &config::Textlint, root: &Path) -> Value {
    let mut rules = Map::new();
    for preset in &config.presets {
        let overrides: Map<String, Value> = config
            .rules
            .iter()
            .filter_map(|(key, value)| {
                let (p, rule) = key.split_once('/')?;
                (p == preset).then(|| (rule.to_string(), value.clone()))
            })
            .collect();
        let value = if overrides.is_empty() {
            Value::Bool(true)
        } else {
            Value::Object(overrides)
        };
        rules.insert(format!("preset-{preset}"), value);
    }
    if !config.prh.is_empty() {
        let paths: Vec<String> = config
            .prh
            .iter()
            .map(|p| root.join(p).to_string_lossy().into_owned())
            .collect();
        rules.insert("prh".to_string(), json!({ "rulePaths": paths }));
    }
    json!({ "rules": rules })
}

/// 設定ファイルを OS のキャッシュのディレクトリに書く。リポジトリにはファイルを作らない。
fn write_builtin_config(value: &Value) -> Result<PathBuf> {
    let text = serde_json::to_string_pretty(value)?;
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let dir = dirs::cache_dir()
        .context("OS のキャッシュのディレクトリが分からない")?
        .join("docv")
        .join("textlint");
    let path = dir.join(format!("{:016x}.json", hasher.finish()));
    if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
        fs::create_dir_all(&dir).with_context(|| format!("{} を作れない", dir.display()))?;
        fs::write(&path, text).with_context(|| format!("{} を書けない", path.display()))?;
    }
    Ok(path)
}

/// textlint は、問題があると終了コード 1 で終わる。それより大きい終了コードは失敗として扱う。
fn run(mut command: Command) -> Result<Output> {
    let output = command.output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            anyhow!(
                "lint.textlint.enabled が true だが、npx が見つからない。Node.js を入れるか、--no-textlint で textlint を飛ばす"
            )
        } else {
            anyhow!(e).context("npx を実行できない")
        }
    })?;
    match output.status.code() {
        Some(0 | 1) => Ok(output),
        _ => bail!(
            "textlint が失敗した\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_builtin_config() {
        let config = config::parse(
            "docs: []\nlint:\n  textlint:\n    enabled: true\n    presets: [ja-technical-writing, ja-spacing]\n    rules:\n      ja-technical-writing/sentence-length: { max: 120 }\n      ja-technical-writing/no-exclamation-question-mark: false\n    prh: [docs/rule/prh.yml]\n",
        )
        .unwrap();
        let textlint = &config.lint.textlint;
        assert_eq!(
            builtin_config(textlint, Path::new("/repo")),
            json!({
                "rules": {
                    "preset-ja-technical-writing": {
                        "sentence-length": { "max": 120 },
                        "no-exclamation-question-mark": false,
                    },
                    "preset-ja-spacing": true,
                    "prh": { "rulePaths": ["/repo/docs/rule/prh.yml"] },
                }
            })
        );
        assert_eq!(
            packages(textlint),
            [
                TEXTLINT,
                "textlint-rule-preset-ja-technical-writing@12.0.2",
                "textlint-rule-preset-ja-spacing@3.0.3",
                PRH,
            ]
        );
    }

    #[test]
    fn reads_textlint_json() {
        let results: Vec<FileResult> = serde_json::from_str(
            r#"[{"filePath":"/repo/a.md","messages":[{"type":"lint","ruleId":"ja-spacing/x","message":"m\n n","line":3,"severity":2}]}]"#,
        )
        .unwrap();
        assert_eq!(
            results[0].messages[0].rule_id.as_deref(),
            Some("ja-spacing/x")
        );
        assert_eq!(results[0].messages[0].line, 3);
    }
}
