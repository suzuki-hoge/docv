//! docv.yaml の読み込みと検証。項目の意味は docs/spec/config.md。
//!
//! docv.yaml にはツールの設定と、プロジェクト全体で共有する語彙だけを書く。ファイルごとの設定は YAML front matter に書く。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;

pub const FILE_NAME: &str = "docv.yaml";
pub const SCHEMA: &str = include_str!("../assets/schema/docv.schema.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: Option<String>,
    #[serde(default)]
    pub dir: Dirs,
    #[serde(default)]
    pub ignore: Vec<String>,
    /// タグの一覧。YAML front matter の tags には、ここにあるタグだけを書ける。
    #[serde(default)]
    pub tags: Vec<TagDef>,
    #[serde(default)]
    pub lint: Lint,
    #[serde(default)]
    pub format: Format,
}

/// tags の 1 つ。color は HTML でのタグの色。
#[derive(Debug, Clone, Deserialize)]
pub struct TagDef {
    pub name: String,
    pub color: TagColor,
}

/// タグの色。並びは CSS の `.tag.c0`〜`.tag.c7` の順。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagColor {
    Blue,
    Green,
    Yellow,
    Red,
    Purple,
    Orange,
    Teal,
    Pink,
}

impl TagColor {
    pub fn index(self) -> usize {
        self as usize
    }
}

/// dir。docv が読み書きするディレクトリ。どれもルートからの相対パス。docs/spec/config.md の 3 章。
#[derive(Debug, Clone, Deserialize)]
pub struct Dirs {
    /// 管理対象の Markdown（Free ファイル）を置くディレクトリ。
    #[serde(default = "default_free")]
    pub free: String,
    /// Slot ファイルを置くディレクトリ。
    #[serde(default = "default_slot")]
    pub slot: String,
    /// docv build が HTML を出すディレクトリ。
    #[serde(default = "default_html")]
    pub html: String,
    /// docv build が Slot のページの JPG を出すディレクトリ。書かなければ JPG を作らない。
    pub image: Option<String>,
}

impl Default for Dirs {
    fn default() -> Self {
        Dirs {
            free: default_free(),
            slot: default_slot(),
            html: default_html(),
            image: None,
        }
    }
}

/// lint。docs/spec/config.md の 4 章と 5 章。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Lint {
    #[serde(default)]
    pub bold: Permission,
    #[serde(default)]
    pub italic: Permission,
    #[serde(default)]
    pub dollar: Permission,
    /// textlint で文章を校正するか。使うルールは docv が決める。
    #[serde(default)]
    pub textlint: Switch,
}

/// 書き方を許すか。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    #[default]
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Switch {
    Enable,
    #[default]
    Disable,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Format {
    #[serde(default)]
    pub enabled: bool,
}

fn default_free() -> String {
    "docs".to_string()
}

fn default_slot() -> String {
    "docv".to_string()
}

fn default_html() -> String {
    "output/docv/html".to_string()
}

/// `./` と末尾の `/` を除く。
pub fn normalize_dir(dir: &str) -> String {
    dir.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

pub fn load(path: &Path) -> Result<Config> {
    let text =
        fs::read_to_string(path).with_context(|| format!("{} を読めない", path.display()))?;
    parse(&text).with_context(|| format!("{} が正しくない", path.display()))
}

impl Config {
    /// 管理対象の Markdown を置くディレクトリ（ルートからの相対パス）。
    pub fn free_dir(&self) -> String {
        normalize_dir(&self.dir.free)
    }

    pub fn slot_dir(&self) -> String {
        normalize_dir(&self.dir.slot)
    }

    pub fn html_dir(&self) -> String {
        normalize_dir(&self.dir.html)
    }

    /// JPG を出すディレクトリ。書いていなければ None で、JPG を作らない。
    pub fn image_dir(&self) -> Option<String> {
        self.dir.image.as_deref().map(normalize_dir)
    }

    /// tags に書いたタグの色。tags にないタグなら None。
    pub fn tag_color(&self, name: &str) -> Option<TagColor> {
        self.tags.iter().find(|t| t.name == name).map(|t| t.color)
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

    let is_root = |dir: &str| dir.is_empty() || dir == ".";
    let under = |path: &str, dir: &str| path == dir || path.starts_with(&format!("{dir}/"));
    // dir の 4 つは、どれも別々のディレクトリにする。同じか、どれかがどれかを含めば error。
    let mut dirs = vec![
        ("free", config.free_dir()),
        ("slot", config.slot_dir()),
        ("html", config.html_dir()),
    ];
    dirs.extend(config.image_dir().map(|dir| ("image", dir)));
    for (key, dir) in &dirs {
        if is_root(dir) {
            errors.push(format!("/dir/{key}: ルートは書けない"));
        }
    }
    for (i, (a, dir_a)) in dirs.iter().enumerate() {
        for (b, dir_b) in &dirs[i + 1..] {
            if is_root(dir_a) || is_root(dir_b) {
                continue;
            }
            if under(dir_a, dir_b) || under(dir_b, dir_a) {
                errors.push(format!(
                    "/dir/{b}: dir.{a} と同じか、含むか、含まれるディレクトリは書けない"
                ));
            }
        }
    }
    let mut names = std::collections::BTreeSet::new();
    for (i, tag) in config.tags.iter().enumerate() {
        if !names.insert(tag.name.as_str()) {
            errors.push(format!("/tags/{i}: タグ {} が重なっている", tag.name));
        }
    }
    for (i, pattern) in config.ignore.iter().enumerate() {
        if let Err(e) = glob(pattern) {
            errors.push(format!("/ignore/{i}: glob として読めない: {e}"));
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
        let config = parse("{}\n").unwrap();
        assert_eq!(config.free_dir(), "docs");
        assert_eq!(config.slot_dir(), "docv");
        assert_eq!(config.html_dir(), "output/docv/html");
        assert_eq!(config.image_dir(), None);
        assert!(!config.format.enabled);
        assert!(config.tags.is_empty());
    }

    #[test]
    fn reads_this_repository_config() {
        let config = parse(include_str!("../docv.yaml")).unwrap();
        assert_eq!(config.title.as_deref(), Some("docv"));
    }

    #[test]
    fn rejects_schema_violation() {
        let error = error_of("unknown: 1\n");
        assert!(error.contains("スキーマに合わない"), "{error}");
        assert!(error.contains("unknown"), "{error}");
    }

    #[test]
    fn reads_lint() {
        let config = parse("lint:\n  bold: deny\n  textlint: enable\n").unwrap();
        assert_eq!(config.lint.bold, Permission::Deny);
        assert_eq!(config.lint.italic, Permission::Allow);
        assert_eq!(config.lint.textlint, Switch::Enable);
        let error = error_of("lint:\n  bold: true\n");
        assert!(error.contains("/lint/bold"), "{error}");
    }

    #[test]
    fn reads_dirs() {
        let config = parse("dir:\n  free: ./notes/\n  image: out/jpg\n").unwrap();
        assert_eq!(config.free_dir(), "notes");
        assert_eq!(config.image_dir().as_deref(), Some("out/jpg"));
        let error = error_of("dir:\n  free: .\n");
        assert!(error.contains("/dir/free: ルートは書けない"), "{error}");
        assert!(parse("dir:\n  html: out/html\n  image: out/jpg\n").is_ok());
    }

    #[test]
    fn rejects_nested_dirs() {
        // 同じディレクトリ
        let error = error_of("dir:\n  html: out\n  image: out\n");
        assert!(
            error.contains("/dir/image: dir.html と同じか、含むか、含まれるディレクトリは書けない"),
            "{error}"
        );
        // 後ろの項目が前の項目を含む
        let error = error_of("dir:\n  html: out/html\n  image: out\n");
        assert!(error.contains("/dir/image: dir.html と同じか"), "{error}");
        // 後ろの項目が前の項目の中にある
        let error = error_of("dir:\n  slot: docv\n  html: docv/build\n");
        assert!(error.contains("/dir/html: dir.slot と同じか"), "{error}");
        let error = error_of("dir:\n  free: docs\n  slot: docs/docv\n");
        assert!(error.contains("/dir/slot: dir.free と同じか"), "{error}");
        // 名前の頭が同じだけなら、含まない
        assert!(parse("dir:\n  free: docs\n  slot: docs-slot\n").is_ok());
    }

    #[test]
    fn reads_tags_with_colors() {
        let config = parse("tags:\n  - name: 仕様\n    color: red\n").unwrap();
        assert_eq!(config.tag_color("仕様"), Some(TagColor::Red));
        assert_eq!(config.tag_color("手順"), None);
        let error = error_of("tags:\n  - name: a\n    color: red\n  - name: a\n    color: blue\n");
        assert!(error.contains("/tags/1: タグ a が重なっている"), "{error}");
        let error = error_of("tags:\n  - name: a\n    color: black\n");
        assert!(error.contains("/tags/0/color"), "{error}");
        let error = error_of("tags:\n  - name: a\n");
        assert!(error.contains("color"), "{error}");
    }

    #[test]
    fn glob_star_does_not_cross_directories() {
        let matcher = glob("docs/*.md").unwrap();
        assert!(matcher.is_match("docs/a.md"));
        assert!(!matcher.is_match("docs/sub/a.md"));
        assert!(glob("docs/**/*.md").unwrap().is_match("docs/sub/a.md"));
    }
}
