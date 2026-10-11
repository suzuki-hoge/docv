//! docv.yaml の読み込みと検証。項目の意味は docs/spec/config.md。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;

use crate::kind::{self, Time};

pub const FILE_NAME: &str = "docv.yaml";
pub const SCHEMA: &str = include_str!("../schema/docv.schema.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub version: u32,
    pub title: Option<String>,
    pub repository: Option<Repository>,
    #[serde(default)]
    pub topics: Vec<String>,
    pub docs: Vec<DocEntry>,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub kinds: Vec<CustomKind>,
    #[serde(default)]
    pub templates: BTreeMap<String, String>,
    #[serde(default)]
    pub archive: Archive,
    #[serde(default)]
    pub output: Output,
    #[serde(default)]
    pub lint: Lint,
    #[serde(default)]
    pub format: Format,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    pub url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocEntry {
    pub path: String,
    pub kind: String,
    #[serde(default)]
    pub check: CheckLevel,
    pub question: Option<String>,
    pub topics: Option<Vec<String>>,
    pub style: Option<Value>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    #[default]
    Error,
    Warn,
    Off,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomKind {
    pub id: String,
    pub label: Option<String>,
    pub time: Time,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Archive {
    #[serde(default = "default_archive_dir")]
    pub dir: String,
}

impl Default for Archive {
    fn default() -> Self {
        Archive {
            dir: default_archive_dir(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    #[serde(default = "default_output_dir")]
    pub dir: String,
    #[serde(default = "default_true")]
    pub commit: bool,
    #[serde(default)]
    pub jpg: bool,
}

impl Default for Output {
    fn default() -> Self {
        Output {
            dir: default_output_dir(),
            commit: true,
            jpg: false,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Lint {
    #[serde(default)]
    pub words: Words,
    pub style: Option<Value>,
    pub textlint: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Words {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Format {
    #[serde(default)]
    pub enabled: bool,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_archive_dir() -> String {
    "docs/archive".to_string()
}

fn default_output_dir() -> String {
    "docv".to_string()
}

fn default_true() -> bool {
    true
}

pub fn load(path: &Path) -> Result<Config> {
    let text =
        fs::read_to_string(path).with_context(|| format!("{} を読めない", path.display()))?;
    parse(&text).with_context(|| format!("{} が正しくない", path.display()))
}

pub fn parse(text: &str) -> Result<Config> {
    let value: Value = serde_yaml_ng::from_str(text).context("YAML として読めない")?;
    validate_schema(&value)?;
    let config: Config = serde_json::from_value(value).context("設定として読めない")?;
    validate_semantics(&config)?;
    Ok(config)
}

/// ルートからの相対パスに当てる glob。`*` は `/` をまたがず、`**` はまたぐ。
pub fn glob(pattern: &str) -> Result<GlobMatcher> {
    Ok(GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()?
        .compile_matcher())
}

fn is_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?', '[', '{'])
}

fn validate_schema(value: &Value) -> Result<()> {
    let schema: Value =
        serde_json::from_str(SCHEMA).expect("埋め込みのスキーマは JSON として正しい");
    let validator = jsonschema::validator_for(&schema).expect("埋め込みのスキーマは正しい");
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| {
            let path = e.instance_path().to_string();
            let path = if path.is_empty() {
                "/".to_string()
            } else {
                path
            };
            format!("{path}: {e}")
        })
        .collect();
    report("スキーマに合わない", errors)
}

/// スキーマだけでは確かめられないこと。
fn validate_semantics(config: &Config) -> Result<()> {
    let mut errors = Vec::new();

    let mut kind_ids: Vec<&str> = kind::BUILTIN.iter().map(|k| k.id).collect();
    for (i, k) in config.kinds.iter().enumerate() {
        if kind_ids.contains(&k.id.as_str()) {
            errors.push(format!("/kinds/{i}/id: 種類 {} はすでにある", k.id));
        } else {
            kind_ids.push(&k.id);
        }
    }

    for (i, entry) in config.docs.iter().enumerate() {
        if !kind_ids.contains(&entry.kind.as_str()) {
            errors.push(format!(
                "/docs/{i}/kind: 種類 {} は定義されていない",
                entry.kind
            ));
        }
        if let Err(e) = glob(&entry.path) {
            errors.push(format!("/docs/{i}/path: glob として読めない: {e}"));
        }
        if is_glob(&entry.path) {
            for key in [
                ("question", entry.question.is_some()),
                ("topics", entry.topics.is_some()),
            ]
            .into_iter()
            .filter(|(_, present)| *present)
            .map(|(key, _)| key)
            {
                errors.push(format!(
                    "/docs/{i}/{key}: path が 1 ファイルだけを指すときにだけ書ける"
                ));
            }
        }
        for topic in entry.topics.iter().flatten() {
            if !config.topics.contains(topic) {
                errors.push(format!("/docs/{i}/topics: 話題 {topic} は topics にない"));
            }
        }
    }

    for (i, pattern) in config.ignore.iter().enumerate() {
        if let Err(e) = glob(pattern) {
            errors.push(format!("/ignore/{i}: glob として読めない: {e}"));
        }
    }

    for id in config.templates.keys() {
        if !kind_ids.contains(&id.as_str()) {
            errors.push(format!("/templates/{id}: 種類 {id} は定義されていない"));
        }
    }

    report("設定が正しくない", errors)
}

fn report(title: &str, mut errors: Vec<String>) -> Result<()> {
    if errors.is_empty() {
        return Ok(());
    }
    errors.sort();
    errors.dedup();
    let lines: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
    bail!("{title}\n{}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_of(text: &str) -> String {
        format!("{:#}", parse(text).unwrap_err())
    }

    #[test]
    fn fills_defaults() {
        let config = parse("version: 1\ndocs: []\n").unwrap();
        assert_eq!(config.archive.dir, "docs/archive");
        assert_eq!(config.output.dir, "docv");
        assert!(config.output.commit);
        assert!(!config.output.jpg);
        assert!(!config.format.enabled);
    }

    #[test]
    fn reads_this_repository_config() {
        let config = parse(include_str!("../docv.yaml")).unwrap();
        assert_eq!(config.title.as_deref(), Some("docv"));
        assert_eq!(config.docs[0].check, CheckLevel::Error);
    }

    #[test]
    fn reads_off_as_check_level() {
        let config =
            parse("version: 1\ndocs:\n  - path: a.md\n    kind: spec\n    check: off\n").unwrap();
        assert_eq!(config.docs[0].check, CheckLevel::Off);
    }

    #[test]
    fn rejects_schema_violation() {
        let error = error_of("version: 2\ndocs: []\nunknown: 1\n");
        assert!(error.contains("スキーマに合わない"), "{error}");
        assert!(error.contains("/version"), "{error}");
    }

    #[test]
    fn rejects_undefined_kind_and_question_on_glob() {
        let error = error_of(
            "version: 1\ntopics: [a]\ndocs:\n  - path: docs/*.md\n    kind: research\n    question: q\n    topics: [b]\n",
        );
        assert!(
            error.contains("/docs/0/kind: 種類 research は定義されていない"),
            "{error}"
        );
        assert!(
            error.contains("/docs/0/question: path が 1 ファイルだけを指すときにだけ書ける"),
            "{error}"
        );
        assert!(
            error.contains("/docs/0/topics: 話題 b は topics にない"),
            "{error}"
        );
    }

    #[test]
    fn accepts_custom_kind() {
        let config = parse(
            "version: 1\nkinds:\n  - id: research\n    time: future\ndocs:\n  - path: docs/research/*.md\n    kind: research\ntemplates:\n  research: t.md\n",
        )
        .unwrap();
        assert_eq!(config.kinds[0].time, Time::Future);
    }

    #[test]
    fn glob_star_does_not_cross_directories() {
        let matcher = glob("docs/*.md").unwrap();
        assert!(matcher.is_match("docs/a.md"));
        assert!(!matcher.is_match("docs/sub/a.md"));
        assert!(glob("docs/**/*.md").unwrap().is_match("docs/sub/a.md"));
    }
}
