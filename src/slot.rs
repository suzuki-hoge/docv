//! Slot。どのプロジェクトにもある問いに、決まった形で答えるファイル。docs/spec/slot.md。
//!
//! Slot ファイルは、slots.dir の下の、まとまりのディレクトリに `<まとまり>/<ID>.<拡張子>` の名前で置く。拡張子は Slot ごとに決まる（yaml か md）。
//! ファイルの中のパスは、すべてルートからの相対パス。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_yaml_ng::Value;

use crate::diagnostic::{Diagnostic, Level};
use crate::markdown;
use crate::workspace::Project;

/// Slot のまとまり。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Service,
    System,
    Developer,
    Operate,
}

impl Group {
    pub const ALL: [Group; 4] = [
        Group::Service,
        Group::System,
        Group::Developer,
        Group::Operate,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Service => "サービス仕様",
            Group::System => "システム仕様",
            Group::Developer => "開発者向けガイド",
            Group::Operate => "運用",
        }
    }

    /// Slot ファイルを置くディレクトリの名前（slots.dir からの相対パス）。
    pub fn dir(self) -> &'static str {
        match self {
            Group::Service => "service",
            Group::System => "system",
            Group::Developer => "guide",
            Group::Operate => "operation",
        }
    }
}

/// Slot ファイルの形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 決まった項目を持つ YAML。
    Yaml,
    /// 形を決めない Markdown。HTML にはそのまま出す。
    Markdown,
    /// YAML（外の定義へのリンクだけ）か Markdown のどちらか。
    Either,
}

#[derive(Debug)]
pub struct Def {
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub question: &'static str,
    pub format: Format,
}

/// Slot の一覧。並び順は HTML に出す順。
pub const DEFS: &[Def] = &[
    Def {
        id: "install",
        label: "インストール",
        group: Group::Service,
        question: "利用者はどう入れるか",
        format: Format::Yaml,
    },
    Def {
        id: "service",
        label: "サービス仕様",
        group: Group::Service,
        question: "これは何で、誰のために、何を解決し、どんな部品からなるか",
        format: Format::Markdown,
    },
    Def {
        id: "manual",
        label: "利用マニュアル",
        group: Group::Service,
        question: "利用者はどう使うか（CLI のコマンド、GUI の操作など）",
        format: Format::Markdown,
    },
    Def {
        id: "words",
        label: "用語集",
        group: Group::Service,
        question: "このプロジェクトの言葉は何を意味するか",
        format: Format::Yaml,
    },
    Def {
        id: "stack",
        label: "技術スタック",
        group: Group::System,
        question: "何で作られているか",
        format: Format::Yaml,
    },
    Def {
        id: "system",
        label: "システム仕様",
        group: Group::System,
        question: "システムは、内部でどう動き、どんな決まりに従うか",
        format: Format::Markdown,
    },
    Def {
        id: "api",
        label: "API 仕様書",
        group: Group::System,
        question: "どんな API があり、何を受け取り、何を返すか",
        format: Format::Either,
    },
    Def {
        id: "db",
        label: "DB 仕様書",
        group: Group::System,
        question: "どんなデータを、どこにどう保存しているか",
        format: Format::Either,
    },
    Def {
        id: "directory-tree",
        label: "ディレクトリ",
        group: Group::Developer,
        question: "どこに何があるか",
        format: Format::Yaml,
    },
    Def {
        id: "setup",
        label: "環境構築",
        group: Group::Developer,
        question: "開発を始めるには、何を用意して何を実行するか",
        format: Format::Yaml,
    },
    Def {
        id: "env",
        label: "環境変数",
        group: Group::Developer,
        question: "システムを動かすのにどんな環境変数が要り、それぞれどこで設定されているか",
        format: Format::Yaml,
    },
    Def {
        id: "develop",
        label: "開発コマンド",
        group: Group::Developer,
        question: "開発者はテストやビルドなどをどう実行するか",
        format: Format::Yaml,
    },
    Def {
        id: "doc-guide",
        label: "Doc ガイド",
        group: Group::Developer,
        question: "このプロジェクトのドキュメントは、どこに何をどう書くか",
        format: Format::Markdown,
    },
    Def {
        id: "infrastructure",
        label: "インフラと環境",
        group: Group::Operate,
        question: "どんな環境（本番、ステージング、開発など）があり、それぞれ何に使うか",
        format: Format::Markdown,
    },
    Def {
        id: "deploy",
        label: "デプロイ",
        group: Group::Operate,
        question: "作ったものを、どの環境に、どう反映するか",
        format: Format::Markdown,
    },
    Def {
        id: "release",
        label: "リリース",
        group: Group::Operate,
        question: "版をどう決めて、利用者にどう届けるか",
        format: Format::Markdown,
    },
    Def {
        id: "cost",
        label: "費用",
        group: Group::Operate,
        question: "何にいくらかかり、費用を理由に何をしていないか",
        format: Format::Yaml,
    },
];

impl Def {
    /// 置ける拡張子。
    pub fn extensions(&self) -> &'static [&'static str] {
        match self.format {
            Format::Yaml => &["yaml"],
            Format::Markdown => &["md"],
            Format::Either => &["yaml", "md"],
        }
    }

    /// Slot ファイルの、slots.dir からの相対パス。拡張子を選べる Slot は、最初の拡張子（yaml）のもの。
    pub fn file_name(&self) -> String {
        format!("{}/{}.{}", self.group.dir(), self.id, self.extensions()[0])
    }

    /// 置けるファイルの、slots.dir からの相対パスをすべて。
    pub fn file_names(&self) -> Vec<String> {
        self.extensions()
            .iter()
            .map(|ext| format!("{}/{}.{ext}", self.group.dir(), self.id))
            .collect()
    }
}

/// slots.dir からの相対パス（`service/words.yaml`）から Slot と拡張子を引く。
pub fn def_of_file(rel: &str) -> Option<(&'static Def, &'static str)> {
    DEFS.iter().find_map(|d| {
        d.extensions()
            .iter()
            .find(|ext| format!("{}/{}.{ext}", d.group.dir(), d.id) == rel)
            .map(|ext| (d, *ext))
    })
}

/// Slot ファイルとして置ける名前の一覧。メッセージに使う。
pub fn file_names() -> String {
    DEFS.iter()
        .flat_map(|d| d.file_names())
        .collect::<Vec<_>>()
        .join("、")
}

/// Slot の名前に近いパス（ディレクトリが違う `words.yaml`、番号が付いた `service/03-words.yaml`、
/// 拡張子が違う `service/service.yaml`）なら、正しいパスを返す。
/// dir は slots.dir。返すパスはルートからの相対パス。
pub fn suggest(dir: &str, rel: &str) -> Option<String> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let (stem, _) = name.rsplit_once('.')?;
    let id = stem
        .split_once('-')
        .filter(|(n, _)| n.len() == 2 && n.chars().all(|c| c.is_ascii_digit()))
        .map_or(stem, |(_, id)| id);
    def(id).map(|d| {
        d.file_names()
            .iter()
            .map(|f| format!("{dir}/{f}"))
            .collect::<Vec<_>>()
            .join(" か ")
    })
}

pub fn def(id: &str) -> Option<&'static Def> {
    DEFS.iter().find(|d| d.id == id)
}

// ---- Slot の中身 ----

/// 手順の Slot（インストール、環境構築、開発コマンド）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Procedure {
    #[serde(default)]
    pub requirements: Vec<Tool>,
    #[serde(default)]
    pub steps: Vec<Step>,
    pub body: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

/// 手順の 1 つ。items が複数あるときは、HTML で os ごとのタブにする。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub name: String,
    /// 手順の中身。1 つなら配列にしなくてよい。
    #[serde(default, deserialize_with = "one_or_many")]
    pub items: Vec<StepItem>,
}

/// 手順の中身。os ごとに分けるときは、os を書く。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepItem {
    pub os: Option<String>,
    pub markdown: Option<String>,
}

/// 1 つの値か、その配列を読む。
fn one_or_many<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        Many(Vec<T>),
        One(T),
    }
    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::Many(items) => items,
        OneOrMany::One(item) => vec![item],
    })
}

/// 技術スタックの項目や、前提として要るツール。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    pub version: Option<String>,
    pub purpose: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    #[serde(default)]
    pub groups: Vec<WordGroup>,
    pub body: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

/// 用語のまとまり。HTML では、name を h2 にし、items を表にする。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordGroup {
    pub name: String,
    #[serde(default)]
    pub items: Vec<Word>,
}

/// 用語。ja と en は、どちらかがあればよい。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Word {
    pub ja: Option<String>,
    pub en: Option<String>,
    pub desc: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    #[serde(default)]
    pub stack: Vec<StackGroup>,
    pub body: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StackGroup {
    pub group: String,
    /// 図のパス。あれば表示する。
    pub image: Option<String>,
    #[serde(default)]
    pub items: Vec<Tool>,
}

/// 外の道具で作った定義へのリンク（API 仕様書、DB 仕様書を YAML で置いたとき）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ref {
    #[serde(rename = "ref")]
    pub target: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    #[serde(default)]
    pub entries: Vec<LayoutEntry>,
    pub body: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutEntry {
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Env {
    #[serde(default)]
    pub envs: Vec<EnvVar>,
    pub body: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvVar {
    pub key: String,
    pub desc: String,
    #[serde(default)]
    pub secret: bool,
    /// ローカル環境で、どこで設定しているか（Docker Compose、.env など）。
    pub local_from: Option<String>,
    /// ステージングや本番で、どこで設定しているか（ECS のタスク定義、SSM など）。
    pub release_from: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    /// 費用をかけて使っているもの。
    #[serde(default)]
    pub accepted_items: Vec<CostItem>,
    /// 費用を理由に、やっていないこと。
    #[serde(default)]
    pub rejected_items: Vec<CostItem>,
    pub body: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostItem {
    pub name: String,
    pub desc: Option<String>,
    /// 1 か月あたりの金額。数でも文字列でもよい。
    pub monthly: Option<Value>,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Content {
    Procedure(Procedure),
    Words(Words),
    Stack(Stack),
    Ref(Ref),
    Layout(Layout),
    Env(Env),
    Cost(Cost),
    /// Markdown の Slot。YAML front matter を除いた本文。
    Markdown(String),
}

impl Content {
    fn parse(id: &str, ext: &str, text: &str) -> serde_yaml_ng::Result<Content> {
        if ext == "md" {
            return Ok(Content::Markdown(markdown::split(text).body));
        }
        // 空のファイルは、項目のない Slot として扱う。
        let text = if text.trim().is_empty() { "{}" } else { text };
        Ok(match id {
            "install" | "setup" | "develop" => Content::Procedure(serde_yaml_ng::from_str(text)?),
            "words" => Content::Words(serde_yaml_ng::from_str(text)?),
            "stack" => Content::Stack(serde_yaml_ng::from_str(text)?),
            "api" | "db" => Content::Ref(serde_yaml_ng::from_str(text)?),
            "directory-tree" => Content::Layout(serde_yaml_ng::from_str(text)?),
            "env" => Content::Env(serde_yaml_ng::from_str(text)?),
            "cost" => Content::Cost(serde_yaml_ng::from_str(text)?),
            _ => unreachable!("YAML の Slot の ID は DEFS で確かめてある"),
        })
    }

    /// 本文（Markdown）と、埋め込む Markdown。
    pub fn body_and_include(&self) -> (Option<&str>, &[String]) {
        let (body, include) = match self {
            Content::Procedure(c) => (&c.body, &c.include),
            Content::Words(c) => (&c.body, &c.include),
            Content::Stack(c) => (&c.body, &c.include),
            Content::Layout(c) => (&c.body, &c.include),
            Content::Env(c) => (&c.body, &c.include),
            Content::Cost(c) => (&c.body, &c.include),
            Content::Ref(_) | Content::Markdown(_) => return (None, &[]),
        };
        (body.as_deref(), include)
    }

    /// 中身が指すルートの中のパス。check で、あるかどうかを確かめる。(フィールド名, パス)
    fn paths(&self) -> Vec<(&'static str, &str)> {
        let mut paths: Vec<(&'static str, &str)> = Vec::new();
        match self {
            Content::Stack(c) => {
                for g in &c.stack {
                    if let Some(image) = &g.image {
                        paths.push(("image", image));
                    }
                }
            }
            Content::Ref(c) if !is_url(&c.target) => paths.push(("ref", &c.target)),
            Content::Layout(c) => {
                for e in &c.entries {
                    paths.push(("path", &e.path));
                }
            }
            Content::Cost(c) => {
                for item in c.accepted_items.iter().chain(&c.rejected_items) {
                    if let Some(r) = item.reference.as_deref().filter(|r| !is_url(r)) {
                        paths.push(("ref", r));
                    }
                }
            }
            _ => {}
        }
        paths
    }

    /// 形のほかに確かめること。(行を探す文字列, メッセージ)
    fn problems(&self) -> Vec<(String, String)> {
        if let Content::Words(c) = self {
            return c
                .groups
                .iter()
                .flat_map(|g| &g.items)
                .filter(|w| w.ja.is_none() && w.en.is_none())
                .map(|w| {
                    (
                        format!("desc: {}", w.desc.lines().next().unwrap_or("")),
                        "用語に ja か en のどちらかを書く".to_string(),
                    )
                })
                .collect();
        }
        let Content::Procedure(c) = self else {
            return Vec::new();
        };
        let mut problems = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for step in &c.steps {
            let needle = format!("id: {}", step.id);
            if !ids.insert(step.id.as_str()) {
                problems.push((
                    needle.clone(),
                    format!("手順の id {} が重なっている", step.id),
                ));
            }
            if step.items.len() > 1 {
                if step.items.iter().any(|i| i.os.is_none()) {
                    problems.push((
                        needle.clone(),
                        format!(
                            "手順 {} の items が複数ある。どの item にも os を書く",
                            step.id
                        ),
                    ));
                }
                let mut oses = std::collections::HashSet::new();
                for os in step.items.iter().filter_map(|i| i.os.as_deref()) {
                    if !oses.insert(os) {
                        problems.push((
                            needle.clone(),
                            format!("手順 {} の os {os} が重なっている", step.id),
                        ));
                    }
                }
            }
        }
        problems
    }
}

pub fn is_url(href: &str) -> bool {
    href.contains("://") || href.starts_with("mailto:")
}

/// 読み込んだ Slot。
#[derive(Debug, Clone)]
pub struct Slot {
    pub def: &'static Def,
    /// ルートからの相対パス。
    pub path: String,
    pub content: Content,
}

/// Slot のディレクトリにあるファイル。
struct SlotFile {
    path: String,
    /// Slot と拡張子。Slot の名前でなければ None。
    def: Option<(&'static Def, &'static str)>,
}

/// 除外するファイルと、名前がドットで始まるファイルは含まない（Project::slot_dir_files）。
fn slot_files(project: &Project) -> Result<Vec<SlotFile>> {
    let dir = project.slots_dir();
    let prefix = format!("{dir}/");
    // adr/ は ADR のディレクトリ。Slot としては読まない（docs/spec/adr.md）。
    let adr_dir = format!("{prefix}{}/", crate::adr::DIR);
    Ok(project
        .slot_dir_files()?
        .into_iter()
        .filter(|path| !path.starts_with(&adr_dir))
        .map(|path| SlotFile {
            def: def_of_file(path.strip_prefix(&prefix).unwrap_or(&path)),
            path,
        })
        .collect())
}

/// Slot を DEFS の順に読む。読めない Slot と、同じ Slot の 2 つ目のファイルは飛ばす（check が報告する）。
pub fn load(project: &Project) -> Result<Vec<Slot>> {
    let mut slots: Vec<Slot> = Vec::new();
    for file in slot_files(project)? {
        let Some((def, ext)) = file.def else { continue };
        if slots.iter().any(|s| s.def.id == def.id) {
            continue;
        }
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        if let Ok(content) = Content::parse(def.id, ext, &text) {
            slots.push(Slot {
                def,
                path: file.path,
                content,
            });
        }
    }
    slots.sort_by_key(|s| DEFS.iter().position(|d| d.id == s.def.id));
    Ok(slots)
}

/// Slot ファイルを確かめる。
pub fn check(project: &Project) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut push = |path: &str, line: usize, rule: &str, level: Level, message: String| {
        diagnostics.push(Diagnostic {
            path: path.to_string(),
            line,
            rule: rule.to_string(),
            level,
            message,
        });
    };
    let mut found: Vec<&str> = Vec::new();
    for file in slot_files(project)? {
        let Some((def, ext)) = file.def else {
            let message = match suggest(&project.slots_dir(), &file.path) {
                Some(right) => format!("Slot ファイルは {right} に置く"),
                None => format!(
                    "Slot の名前ではない。Slot のディレクトリに置けるのは {}",
                    file_names()
                ),
            };
            push(&file.path, 1, "slot-unknown", Level::Error, message);
            continue;
        };
        if found.contains(&def.id) {
            push(
                &file.path,
                1,
                "slot-duplicate",
                Level::Error,
                format!(
                    "{}の Slot ファイルが 2 つある。どちらか 1 つにする",
                    def.label
                ),
            );
            continue;
        }
        found.push(def.id);
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        let content = match Content::parse(def.id, ext, &text) {
            Ok(content) => content,
            Err(e) => {
                let line = e.location().map_or(1, |l| l.line());
                push(
                    &file.path,
                    line,
                    "slot-invalid",
                    Level::Error,
                    format!("{}の Slot の形に合わない: {e}", def.label),
                );
                continue;
            }
        };
        let line_of = |needle: &str| {
            text.lines()
                .position(|l| l.contains(needle))
                .map_or(1, |i| i + 1)
        };
        for (needle, message) in content.problems() {
            push(
                &file.path,
                line_of(&needle),
                "slot-invalid",
                Level::Error,
                message,
            );
        }
        for (field, path) in content.paths() {
            if !project.root.join(path).exists() {
                let level = if field == "path" {
                    Level::Warn
                } else {
                    Level::Error
                };
                push(
                    &file.path,
                    line_of(path),
                    "slot-path-missing",
                    level,
                    format!("{field} の {path} がない"),
                );
            }
        }
        for target in content.body_and_include().1 {
            if let Err(message) = resolve_include(project, target) {
                push(
                    &file.path,
                    line_of(target),
                    "slot-include-missing",
                    Level::Error,
                    message,
                );
            }
        }
    }
    Ok(diagnostics)
}

/// 埋め込む Markdown。
pub struct Included {
    /// 埋め込むファイルのルートからの相対パス。リンクの起点に使う。
    pub source: String,
    /// 指した見出しの文字列。ファイル全体なら None。
    pub heading: Option<String>,
    pub markdown: String,
}

/// include の 1 項目（`パス` か `パス#見出し`）を、埋め込む Markdown にする。
/// ファイル全体なら YAML front matter と h1 を除き、見出しを指したら、その見出しの下から次の同じ深さ以上の見出しの手前まで。
pub fn resolve_include(project: &Project, target: &str) -> Result<Included, String> {
    let (path, heading) = match target.split_once('#') {
        Some((path, heading)) => (path, Some(heading.trim())),
        None => (target, None),
    };
    let text = fs::read_to_string(project.root.join(path))
        .map_err(|_| format!("include の {path} がない"))?;
    let document = markdown::split(&text);
    let outline = markdown::outline(&document);
    let lines: Vec<&str> = document.body.split_inclusive('\n').collect();
    let index_of = |line: usize| line - document.body_start_line;

    let mut found_heading = None;
    let (start, end) = match heading {
        None => {
            let start = outline
                .headings
                .iter()
                .find(|h| h.level == 1)
                .map_or(0, |h| index_of(h.line) + 1);
            (start, lines.len())
        }
        Some(heading) => {
            let position = outline
                .headings
                .iter()
                .position(|h| h.text == heading || strip_number(&h.text) == heading)
                .ok_or_else(|| format!("include の {path} に見出し「{heading}」がない"))?;
            let found = &outline.headings[position];
            found_heading = Some(found.text.clone());
            let end = outline.headings[position + 1..]
                .iter()
                .find(|h| h.level <= found.level)
                .map_or(lines.len(), |h| index_of(h.line));
            (index_of(found.line) + 1, end)
        }
    };
    let markdown = lines[start.min(lines.len())..end.min(lines.len())].concat();
    Ok(Included {
        source: path.to_string(),
        heading: found_heading,
        markdown: trim_separators(&markdown),
    })
}

/// 見出しの先頭の章番号（`1. `）を除く。
pub fn strip_number(text: &str) -> &str {
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(rest) = text[digits..].strip_prefix(". ")
    {
        return rest;
    }
    text
}

/// 前後の空行と、章の区切りの `---` を除く。
fn trim_separators(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();
    let is_blank = |l: &&str| l.trim().is_empty() || l.trim() == "---";
    let start = lines
        .iter()
        .position(|l| !is_blank(l))
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !is_blank(l))
        .map_or(start, |i| i + 1);
    let mut text = lines[start..end].join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

/// yaml の Slot のテンプレ。docv add で書く、項目のサンプル。
pub fn template(id: &str) -> Option<&'static str> {
    Some(match id {
        "install" => include_str!("../assets/template/install.yaml"),
        "words" => include_str!("../assets/template/words.yaml"),
        "stack" => include_str!("../assets/template/stack.yaml"),
        "api" => include_str!("../assets/template/api.yaml"),
        "db" => include_str!("../assets/template/db.yaml"),
        "directory-tree" => include_str!("../assets/template/directory-tree.yaml"),
        "setup" => include_str!("../assets/template/setup.yaml"),
        "env" => include_str!("../assets/template/env.yaml"),
        "develop" => include_str!("../assets/template/develop.yaml"),
        "cost" => include_str!("../assets/template/cost.yaml"),
        _ => return None,
    })
}

/// md の Slot のテンプレ。docv add で書く雛形。Doc ガイドの Slot（doc-guide）だけがガイドラインの雛形を持ち、ほかの md の Slot は空のファイルを作る。
pub fn md_template(id: &str) -> Option<&'static str> {
    match id {
        "doc-guide" => Some(include_str!("../assets/template/doc-guide.md")),
        _ => None,
    }
}

/// yaml の Slot のスキーマ（YAML で書いた JSON Schema）。docv schema で出す。md の Slot はスキーマを持たない。
pub fn schema(id: &str) -> Option<&'static str> {
    Some(match id {
        "install" => include_str!("../assets/schema/install.yaml"),
        "words" => include_str!("../assets/schema/words.yaml"),
        "stack" => include_str!("../assets/schema/stack.yaml"),
        "api" => include_str!("../assets/schema/api.yaml"),
        "db" => include_str!("../assets/schema/db.yaml"),
        "directory-tree" => include_str!("../assets/schema/directory-tree.yaml"),
        "setup" => include_str!("../assets/schema/setup.yaml"),
        "env" => include_str!("../assets/schema/env.yaml"),
        "develop" => include_str!("../assets/schema/develop.yaml"),
        "cost" => include_str!("../assets/schema/cost.yaml"),
        _ => return None,
    })
}

/// Value を表に出す文字列にする。
pub fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// パスがルートの中の、ディレクトリを指すか。
pub fn is_dir(project: &Project, path: &str) -> bool {
    path.ends_with('/') || Path::new(&project.root.join(path)).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", "docs: [docs/**/*.md]\n");
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn every_yaml_slot_has_a_template_that_parses() {
        for def in DEFS.iter().filter(|d| d.extensions().contains(&"yaml")) {
            let text = template(def.id).unwrap();
            Content::parse(def.id, "yaml", text).unwrap_or_else(|e| panic!("{}: {e}", def.id));
        }
    }

    #[test]
    fn loads_slots_in_definition_order() {
        let (_dir, project) = project(&[
            ("docv/service/manual.md", "# マニュアル\n"),
            ("docv/service/service.md", "# サービス\n\n本文\n"),
            ("docv/system/api.md", "API\n"),
        ]);
        let slots = load(&project).unwrap();
        let ids: Vec<&str> = slots.iter().map(|s| s.def.id).collect();
        assert_eq!(ids, ["service", "manual", "api"]);
        assert!(matches!(&slots[0].content, Content::Markdown(text) if text.contains("本文")));
    }

    #[test]
    fn reports_problems_of_slot_files() {
        let (_dir, project) = project(&[
            ("docv/unknown.yaml", "a: 1\n"),
            ("docv/words.yaml", "groups: []\n"),
            ("docv/service/03-words.yaml", "groups: []\n"),
            ("docv/service/words.yaml", "groups: []\nextra: 1\n"),
            ("docv/system/api.md", "API\n"),
            ("docv/system/api.yaml", "ref: openapi.yaml\n"),
            (
                "docv/service/install.yaml",
                "steps:\n  - id: binary\n    name: a\n    items:\n      - os: macos\n      - markdown: m\n",
            ),
            (
                "docv/guide/directory-tree.yaml",
                "entries:\n  - path: src/\n    description: d\ninclude: [docs/a.md#ない]\n",
            ),
            ("docs/a.md", "# A\n\n## 1. ある\n"),
        ]);
        let diagnostics = check(&project).unwrap();
        let result: Vec<String> = diagnostics
            .iter()
            .map(|d| format!("{}:{} {} {}", d.path, d.line, d.level, d.rule))
            .collect();
        assert_eq!(
            result,
            [
                "docv/guide/directory-tree.yaml:2 warn slot-path-missing",
                "docv/guide/directory-tree.yaml:4 error slot-include-missing",
                "docv/service/03-words.yaml:1 error slot-unknown",
                "docv/service/install.yaml:2 error slot-invalid",
                "docv/service/words.yaml:2 error slot-invalid",
                "docv/system/api.yaml:1 error slot-duplicate",
                "docv/unknown.yaml:1 error slot-unknown",
                "docv/words.yaml:1 error slot-unknown",
            ]
        );
        // ディレクトリが違う名前、番号を付けた名前には、正しいパスを示す
        assert!(diagnostics[2].message.contains("docv/service/words.yaml"));
        assert!(diagnostics[7].message.contains("docv/service/words.yaml"));
    }

    #[test]
    fn needs_ja_or_en_for_each_word() {
        let (_dir, project) = project(&[(
            "docv/service/words.yaml",
            "groups:\n  - name: g\n    items:\n      - en: Slot\n        desc: a\n      - desc: b\n",
        )]);
        let result: Vec<String> = check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{}:{} {}", d.path, d.line, d.rule))
            .collect();
        assert_eq!(result, ["docv/service/words.yaml:6 slot-invalid"]);
    }

    #[test]
    fn reads_one_item_or_many() {
        let one = Content::parse(
            "install",
            "yaml",
            "steps:\n  - id: cargo\n    name: c\n    items:\n      markdown: m\n",
        )
        .unwrap();
        let many = Content::parse(
            "install",
            "yaml",
            "steps:\n  - id: binary\n    name: b\n    items:\n      - os: windows\n      - os: macos\n",
        )
        .unwrap();
        let count = |c: &Content| match c {
            Content::Procedure(p) => p.steps[0].items.len(),
            _ => 0,
        };
        assert_eq!((count(&one), count(&many)), (1, 2));
    }

    #[test]
    fn includes_a_section_or_a_whole_file() {
        let (_dir, project) = project(&[(
            "docs/a.md",
            "---\nquestion: q\n---\n\n# A\n\n前書き\n\n---\n\n## 1. 入れ方\n\n手順\n\n### 細かく\n\n細部\n\n---\n\n## 2. 次\n\n次の章\n",
        )]);
        let section = resolve_include(&project, "docs/a.md#入れ方").unwrap();
        assert_eq!(section.markdown, "手順\n\n### 細かく\n\n細部\n");
        assert_eq!(section.source, "docs/a.md");
        let whole = resolve_include(&project, "docs/a.md").unwrap();
        assert!(whole.markdown.starts_with("前書き\n"), "{}", whole.markdown);
        assert!(!whole.markdown.contains("# A"));
        assert!(resolve_include(&project, "docs/none.md").is_err());
    }
}
