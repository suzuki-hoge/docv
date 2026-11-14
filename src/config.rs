//! docv.yaml の読み込みと検証。項目の意味は docv/slot/service/service/config.md。
//!
//! docv.yaml にはツールの設定と、プロジェクト全体で共有する語彙だけを書く。ファイルごとの設定は YAML front matter に書く。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;

pub const FILE_NAME: &str = "docv.yaml";
pub const SCHEMA: &str = include_str!("../assets/schema/docv.schema.json");

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// ドキュメントを合わせた docv の版。docv の版と違えば、check が error にする。docv/slot/service/service/config.md の「docv の版」。
    pub docv_version: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub dir: Dirs,
    /// docv が読まないファイルの glob。docv/slot/service/service/config.md の「docv が読むファイル」。
    #[serde(default)]
    pub docv_exclude: Vec<String>,
    #[serde(default)]
    pub generate: Generate,
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

/// dir。docv が読み書きするディレクトリ。どちらもルートからの相対パス。docv/slot/service/service/config.md の「配置」。
/// その下の配置は docv が決める。
#[derive(Debug, Clone, Deserialize)]
pub struct Dirs {
    /// ドキュメントを置くディレクトリ。この下に free/、slot/、adr/、pdr/ と、build が作るインデックスの README.md を置く。
    #[serde(default = "default_document_root")]
    pub document_root: String,
    /// docv build の出力先。この下に html/ と image/ を作る。
    #[serde(default = "default_output")]
    pub output: String,
}

impl Default for Dirs {
    fn default() -> Self {
        Dirs {
            document_root: default_document_root(),
            output: default_output(),
        }
    }
}

/// generate。docv build が作るもの。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Generate {
    /// YAML の Slot のページを JPG にするか。
    #[serde(default)]
    pub image: bool,
}

/// document_root の下の、Free ファイルのディレクトリ。
pub const FREE_DIR: &str = "free";
/// document_root の下の、Slot ファイルのディレクトリ。
pub const SLOT_DIR: &str = "slot";
/// document_root の直下に build が書くインデックス。
pub const INDEX_FILE: &str = "README.md";
/// dir.output の下の、HTML のディレクトリ。
pub const HTML_DIR: &str = "html";
/// dir.output の下の、JPG のディレクトリ。
pub const IMAGE_DIR: &str = "image";

/// lint。docv/slot/service/service/config.md の 4 章と 5 章。
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

fn default_document_root() -> String {
    "docv".to_string()
}

fn default_output() -> String {
    "output/docv".to_string()
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
    parse(&text).map_err(|e| {
        let e = e.context(format!("{} が正しくない", path.display()));
        // 古い版の docv.yaml がスキーマに合わず読めないときも、docv update-guide にたどり着けるように、版の直し方を添える。
        match crate::version::advice_for_unreadable(&text) {
            Some(advice) => anyhow!("{e:#}\n{advice}"),
            None => e,
        }
    })
}

impl Config {
    /// ドキュメントを置くディレクトリ（ルートからの相対パス）。
    pub fn document_root(&self) -> String {
        normalize_dir(&self.dir.document_root)
    }

    /// docv build の出力先（ルートからの相対パス）。
    pub fn output_dir(&self) -> String {
        normalize_dir(&self.dir.output)
    }

    /// 管理対象の Markdown（Free ファイル）を置くディレクトリ（ルートからの相対パス）。
    pub fn free_dir(&self) -> String {
        format!("{}/{FREE_DIR}", self.document_root())
    }

    /// Slot ファイルを置くディレクトリ（ルートからの相対パス）。
    pub fn slot_dir(&self) -> String {
        format!("{}/{SLOT_DIR}", self.document_root())
    }

    /// HTML を出すディレクトリ（ルートからの相対パス）。
    pub fn html_dir(&self) -> String {
        format!("{}/{HTML_DIR}", self.output_dir())
    }

    /// JPG を出すディレクトリ（ルートからの相対パス）。generate.image が false なら None で、JPG を作らない。
    pub fn image_dir(&self) -> Option<String> {
        self.generate
            .image
            .then(|| format!("{}/{IMAGE_DIR}", self.output_dir()))
    }

    /// インデックスのパス（ルートからの相対パス）。
    pub fn index_path(&self) -> String {
        format!("{}/{INDEX_FILE}", self.document_root())
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
    let under = |path: &str, dir: &str| path.starts_with(&format!("{dir}/"));
    let document_root = config.document_root();
    let output = config.output_dir();
    for (key, dir) in [("document_root", &document_root), ("output", &output)] {
        if is_root(dir) {
            errors.push(format!("/dir/{key}: ルートは書けない"));
        }
    }
    if !is_root(&document_root) && !is_root(&output) {
        // AI Agent が document_root を読むと、HTML に写した同じ内容がほぼ倍入る。
        if output == document_root {
            errors.push(
                "/dir/output: dir.document_root と同じディレクトリは書けない。AI Agent が document_root を読むと、HTML に写した同じ内容がほぼ倍入るため"
                    .to_string(),
            );
        } else if under(&output, &document_root) {
            errors.push(
                "/dir/output: dir.document_root の中は書けない。AI Agent が document_root を読むと、HTML に写した同じ内容がほぼ倍入るため"
                    .to_string(),
            );
        } else if under(&document_root, &output) {
            errors.push(
                "/dir/document_root: dir.output の中は書けない。build が出力先を作り直すときに、ドキュメントを消してしまうため"
                    .to_string(),
            );
        }
    }
    let mut names = std::collections::BTreeSet::new();
    for (i, tag) in config.tags.iter().enumerate() {
        if !names.insert(tag.name.as_str()) {
            errors.push(format!("/tags/{i}: タグ {} が重なっている", tag.name));
        }
    }
    for (i, pattern) in config.docv_exclude.iter().enumerate() {
        if let Err(e) = glob(pattern) {
            errors.push(format!("/docv_exclude/{i}: glob として読めない: {e}"));
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
        assert_eq!(config.document_root(), "docv");
        assert_eq!(config.output_dir(), "output/docv");
        assert_eq!(config.free_dir(), "docv/free");
        assert_eq!(config.slot_dir(), "docv/slot");
        assert_eq!(config.html_dir(), "output/docv/html");
        assert_eq!(config.image_dir(), None);
        assert_eq!(config.index_path(), "docv/README.md");
        assert!(config.docv_exclude.is_empty());
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
    fn reads_docv_version() {
        let config = parse("docv_version: 1.8.0\n").unwrap();
        assert_eq!(config.docv_version.as_deref(), Some("1.8.0"));
        assert_eq!(parse("{}\n").unwrap().docv_version, None);
        for text in [
            "docv_version: 1.8\n",
            "docv_version: v1.8.0\n",
            "docv_version: 1.8.0.1\n",
        ] {
            let error = error_of(text);
            assert!(error.contains("/docv_version"), "{text}: {error}");
        }
    }

    #[test]
    fn advises_the_version_when_the_config_cannot_be_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, "docv_version: 1.0.0\ndir:\n  slots: docv\n").unwrap();
        let error = format!("{:#}", load(&path).unwrap_err());
        assert!(error.contains("スキーマに合わない"), "{error}");
        assert!(
            error.ends_with(&format!(
                "docv update-guide --from 1.0.0 で変わったことと対応を確かめ、対応してから docv_version を {} にする",
                env!("CARGO_PKG_VERSION")
            )),
            "{error}"
        );
        // docv の版と同じなら、添えない
        fs::write(
            &path,
            format!("docv_version: {}\nunknown: 1\n", env!("CARGO_PKG_VERSION")),
        )
        .unwrap();
        let error = format!("{:#}", load(&path).unwrap_err());
        assert!(!error.contains("update-guide"), "{error}");
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
        let config =
            parse("dir:\n  document_root: ./notes/\n  output: out\ngenerate:\n  image: true\n")
                .unwrap();
        assert_eq!(config.document_root(), "notes");
        assert_eq!(config.free_dir(), "notes/free");
        assert_eq!(config.slot_dir(), "notes/slot");
        assert_eq!(config.index_path(), "notes/README.md");
        assert_eq!(config.html_dir(), "out/html");
        assert_eq!(config.image_dir().as_deref(), Some("out/image"));
        for key in ["document_root", "output"] {
            let error = error_of(&format!("dir:\n  {key}: .\n"));
            assert!(
                error.contains(&format!("/dir/{key}: ルートは書けない")),
                "{error}"
            );
        }
        // 昔の項目は書けない
        for key in ["free", "slot", "html", "image", "index"] {
            let error = error_of(&format!("dir:\n  {key}: x\n"));
            assert!(error.contains("スキーマに合わない"), "{key}: {error}");
        }
        let error = error_of("ignore:\n  - x\n");
        assert!(error.contains("スキーマに合わない"), "{error}");
    }

    #[test]
    fn rejects_output_inside_document_root() {
        // 同じディレクトリと、document_root の中の output は、AI Agent が読む範囲に同じ内容が入るので書けない
        let error = error_of("dir:\n  document_root: docv\n  output: docv\n");
        assert!(
            error.contains(
                "/dir/output: dir.document_root と同じディレクトリは書けない。AI Agent が"
            ),
            "{error}"
        );
        let error = error_of("dir:\n  document_root: docv\n  output: docv/output\n");
        assert!(
            error.contains(
                "/dir/output: dir.document_root の中は書けない。AI Agent が document_root を読むと"
            ),
            "{error}"
        );
        // output の中の document_root は、build が消してしまうので書けない
        let error = error_of("dir:\n  document_root: out/docv\n  output: out\n");
        assert!(
            error.contains("/dir/document_root: dir.output の中は書けない。build が"),
            "{error}"
        );
        // 名前の頭が同じだけなら、含まない
        assert!(parse("dir:\n  document_root: docv\n  output: docv-output\n").is_ok());
    }

    #[test]
    fn reads_docv_exclude() {
        let config = parse("docv_exclude:\n  - docv/free/tmp/**\n").unwrap();
        assert_eq!(config.docv_exclude, ["docv/free/tmp/**"]);
        let error = error_of("docv_exclude:\n  - \"a[\"\n");
        assert!(
            error.contains("/docv_exclude/0: glob として読めない"),
            "{error}"
        );
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
