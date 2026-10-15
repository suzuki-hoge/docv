//! docv.yaml の読み込みと検証。項目の意味は docs/spec/config.md。
//!
//! docv.yaml にはツールの設定と、プロジェクト全体で共有する語彙だけを書く。ファイルごとの設定は YAML front matter に書く。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;

pub const FILE_NAME: &str = "docv.yaml";
pub const SCHEMA: &str = include_str!("../assets/schema/docv.schema.json");

/// タグに付けられるルール。docs/spec/free.md の 2 章。
pub const TAG_RULES: &[&str] = &["no-past-wording", "no-done-task"];

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: Option<String>,
    /// 管理対象のファイルの glob。
    pub docs: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
    /// タグの一覧。書いたときだけ、YAML front matter の tags をこの中に限る。
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub slots: Slots,
    #[serde(default)]
    pub output: Output,
    #[serde(default)]
    pub lint: Lint,
    #[serde(default)]
    pub format: Format,
    /// パスごとに、道具の設定を上書きする。後に書いたものが優先する。
    #[serde(default)]
    pub overrides: Vec<Override>,
}

/// overrides の 1 つ。paths に当てはまるファイルだけ、書いた項目を上書きする。docs/spec/config.md。
#[derive(Debug, Clone, Deserialize)]
pub struct Override {
    pub paths: Vec<String>,
    /// textlint をかけるか。
    pub textlint: Option<bool>,
    /// 書式の規則（lint.style）をかけるか。
    pub style: Option<bool>,
    /// 整形するか。
    pub format: Option<bool>,
    /// YAML front matter の mission を必須にするか。
    pub mission: Option<Requirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Requirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Slots {
    #[serde(default = "default_slots_dir")]
    pub dir: String,
}

impl Default for Slots {
    fn default() -> Self {
        Slots {
            dir: default_slots_dir(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Output {
    #[serde(default = "default_output_dir")]
    pub dir: String,
    #[serde(default = "default_true")]
    pub commit: bool,
}

impl Default for Output {
    fn default() -> Self {
        Output {
            dir: default_output_dir(),
            commit: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Lint {
    #[serde(default)]
    pub words: Words,
    pub style: Option<Value>,
    /// タグごとに、そのタグを持つファイルにかけるルール。
    #[serde(default)]
    pub tags: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub textlint: Textlint,
}

/// lint.textlint。docs/spec/config.md の 6 章。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Textlint {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub level: TextlintLevel,
    #[serde(default)]
    pub presets: Vec<String>,
    #[serde(default)]
    pub rules: BTreeMap<String, Value>,
    #[serde(default)]
    pub prh: Vec<String>,
    pub config: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextlintLevel {
    #[default]
    Warn,
    Error,
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

fn default_slots_dir() -> String {
    "docv".to_string()
}

fn default_output_dir() -> String {
    "docv/build".to_string()
}

fn default_true() -> bool {
    true
}

pub fn load(path: &Path) -> Result<Config> {
    let text =
        fs::read_to_string(path).with_context(|| format!("{} を読めない", path.display()))?;
    parse(&text).with_context(|| format!("{} が正しくない", path.display()))
}

impl Config {
    /// タグにかけるルール。
    pub fn rules_of<'a>(&'a self, tags: &'a [String]) -> impl Iterator<Item = &'a str> + 'a {
        tags.iter()
            .filter_map(|tag| self.lint.tags.get(tag))
            .flatten()
            .map(String::as_str)
    }
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

    for (i, pattern) in config.docs.iter().enumerate() {
        if let Err(e) = glob(pattern) {
            errors.push(format!("/docs/{i}: glob として読めない: {e}"));
        }
    }
    for (i, pattern) in config.ignore.iter().enumerate() {
        if let Err(e) = glob(pattern) {
            errors.push(format!("/ignore/{i}: glob として読めない: {e}"));
        }
    }
    for (i, o) in config.overrides.iter().enumerate() {
        for (j, pattern) in o.paths.iter().enumerate() {
            if let Err(e) = glob(pattern) {
                errors.push(format!(
                    "/overrides/{i}/paths/{j}: glob として読めない: {e}"
                ));
            }
        }
    }

    for (tag, rules) in &config.lint.tags {
        if let Some(tags) = &config.tags
            && !tags.contains(tag)
        {
            errors.push(format!("/lint/tags/{tag}: タグ {tag} は tags にない"));
        }
        for rule in rules {
            if !TAG_RULES.contains(&rule.as_str()) {
                errors.push(format!(
                    "/lint/tags/{tag}: ルール {rule} はない。使えるのは {}",
                    TAG_RULES.join("、")
                ));
            }
        }
    }

    let textlint = &config.lint.textlint;
    for key in textlint.rules.keys() {
        let preset = key.split_once('/').map_or(key.as_str(), |(p, _)| p);
        if !textlint.presets.iter().any(|p| p == preset) {
            errors.push(format!(
                "/lint/textlint/rules/{key}: プリセット {preset} が presets にない"
            ));
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
        let config = parse("docs: []\n").unwrap();
        assert_eq!(config.slots.dir, "docv");
        assert_eq!(config.output.dir, "docv/build");
        assert!(config.output.commit);
        assert!(!config.format.enabled);
        assert!(config.tags.is_none());
    }

    #[test]
    fn reads_this_repository_config() {
        let config = parse(include_str!("../docv.yaml")).unwrap();
        assert_eq!(config.title.as_deref(), Some("docv"));
    }

    #[test]
    fn rejects_schema_violation() {
        let error = error_of("docs: []\nunknown: 1\n");
        assert!(error.contains("スキーマに合わない"), "{error}");
        assert!(error.contains("unknown"), "{error}");
    }

    #[test]
    fn rejects_unknown_tag_and_rule() {
        let error = error_of("docs: []\ntags: [仕様]\nlint:\n  tags:\n    計画: [no-done-task]\n");
        assert!(
            error.contains("/lint/tags/計画: タグ 計画 は tags にない"),
            "{error}"
        );
        let error = error_of("docs: []\nlint:\n  tags:\n    仕様: [no-such-rule]\n");
        assert!(error.contains("/lint/tags/仕様/0"), "{error}");
    }

    #[test]
    fn collects_rules_of_tags() {
        let config = parse(
            "docs: []\nlint:\n  tags:\n    仕様: [no-past-wording]\n    計画: [no-done-task]\n",
        )
        .unwrap();
        let tags = ["計画".to_string(), "その他".to_string()];
        assert_eq!(config.rules_of(&tags).collect::<Vec<_>>(), ["no-done-task"]);
    }

    #[test]
    fn rejects_textlint_rule_of_unused_preset() {
        let error = error_of(
            "docs: []\nlint:\n  textlint:\n    presets: [ja-spacing]\n    rules:\n      jtf-style/x: false\n",
        );
        assert!(
            error.contains(
                "/lint/textlint/rules/jtf-style/x: プリセット jtf-style が presets にない"
            ),
            "{error}"
        );
    }

    #[test]
    fn reads_overrides() {
        let config = parse(
            "docs: []\noverrides:\n  - paths: [docs/tmp/**]\n    textlint: false\n    mission: optional\n",
        )
        .unwrap();
        assert_eq!(config.overrides[0].textlint, Some(false));
        assert_eq!(config.overrides[0].mission, Some(Requirement::Optional));
        assert_eq!(config.overrides[0].style, None);
    }

    #[test]
    fn glob_star_does_not_cross_directories() {
        let matcher = glob("docs/*.md").unwrap();
        assert!(matcher.is_match("docs/a.md"));
        assert!(!matcher.is_match("docs/sub/a.md"));
        assert!(glob("docs/**/*.md").unwrap().is_match("docs/sub/a.md"));
    }
}
