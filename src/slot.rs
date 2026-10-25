//! Slot。どのプロジェクトにも要るドキュメントを、docv が決めたミッションと形で書くファイル。docs/spec/slot.md。
//!
//! Slot ファイルは、dir.slot の下の、まとまりのディレクトリに `<まとまり>/<ID>.<拡張子>` の名前で置く。拡張子は Slot ごとに決まる（yaml か md）。
//! サービス仕様だけは、`<まとまり>/<ID>/` の直下に 1 ページ 1 つの md で置いて、ページに分けられる。ページの名前は h1、並びは YAML front matter の order。
//! md の Slot（分けたページも）は、YAML front matter の mission に、docv が決めたミッションをそのまま書く。違えば check が error にする。
//! ファイルの中のパスは、すべてルートからの相対パス。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_yaml_ng::Value;

use crate::diagnostic::Diagnostic;
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
            Group::Service => "サービス",
            Group::System => "システム",
            Group::Developer => "開発者向けガイド",
            Group::Operate => "運用",
        }
    }

    /// Slot ファイルを置くディレクトリの名前（dir.slot からの相対パス）。
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
}

#[derive(Debug)]
pub struct Def {
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    /// ミッション。この Slot を読むと何が分かるか。
    pub mission: &'static str,
    pub format: Format,
}

/// Slot の一覧。並び順は HTML に出す順。
pub const DEFS: &[Def] = &[
    Def {
        id: "overview",
        label: "サービス概要",
        group: Group::Service,
        mission: "利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明",
        format: Format::Markdown,
    },
    Def {
        id: "service",
        label: "サービス仕様",
        group: Group::Service,
        mission: "外から見た機能と、それぞれの振る舞いと決まり",
        format: Format::Markdown,
    },
    Def {
        id: "manual",
        label: "利用マニュアル",
        group: Group::Service,
        mission: "利用者の使い方（CLI のコマンド、GUI の操作など）",
        format: Format::Markdown,
    },
    Def {
        id: "install",
        label: "インストール",
        group: Group::Service,
        mission: "利用者がこれを入れる手順",
        format: Format::Yaml,
    },
    Def {
        id: "words",
        label: "用語集",
        group: Group::Service,
        mission: "このプロジェクトの言葉の意味",
        format: Format::Yaml,
    },
    Def {
        id: "system-overview",
        label: "システム概要",
        group: Group::System,
        mission: "開発者に向けた、システムを作る部品と、そのつながりの全体像",
        format: Format::Markdown,
    },
    Def {
        id: "system",
        label: "システム仕様",
        group: Group::System,
        mission: "部品ごとの振る舞いと、部品の間の流れと、部品をまたいで守る決まり",
        format: Format::Markdown,
    },
    Def {
        id: "stack",
        label: "技術スタック",
        group: Group::System,
        mission: "作るのに使っている言語、ライブラリ、サービス",
        format: Format::Yaml,
    },
    Def {
        id: "api",
        label: "API 仕様書",
        group: Group::System,
        mission: "API の一覧と、それぞれが受け取るものと返すもの",
        format: Format::Markdown,
    },
    Def {
        id: "db",
        label: "DB 仕様書",
        group: Group::System,
        mission: "保存しているデータと、その保存先と形",
        format: Format::Markdown,
    },
    Def {
        id: "directory-tree",
        label: "ディレクトリ",
        group: Group::Developer,
        mission: "ディレクトリごとの役割",
        format: Format::Yaml,
    },
    Def {
        id: "setup",
        label: "環境構築",
        group: Group::Developer,
        mission: "開発を始めるのに用意するものと、実行するコマンド",
        format: Format::Yaml,
    },
    Def {
        id: "env",
        label: "環境変数",
        group: Group::Developer,
        mission: "システムを動かすのに要る環境変数と、それぞれを設定している場所",
        format: Format::Yaml,
    },
    Def {
        id: "develop",
        label: "開発コマンド",
        group: Group::Developer,
        mission: "テストやビルドなど、開発者が実行するコマンド",
        format: Format::Yaml,
    },
    Def {
        id: "doc-guide",
        label: "Doc ガイド",
        group: Group::Developer,
        mission: "このプロジェクトのドキュメントの置き場所と書き方",
        format: Format::Markdown,
    },
    Def {
        id: "infrastructure",
        label: "インフラと環境",
        group: Group::Operate,
        mission: "環境（本番、ステージング、開発など）の一覧と、それぞれの用途",
        format: Format::Markdown,
    },
    Def {
        id: "deploy",
        label: "デプロイ",
        group: Group::Operate,
        mission: "作ったものを環境に反映する手順",
        format: Format::Markdown,
    },
    Def {
        id: "release",
        label: "リリース",
        group: Group::Operate,
        mission: "版の決め方と、利用者への届け方",
        format: Format::Markdown,
    },
    Def {
        id: "cost",
        label: "費用",
        group: Group::Operate,
        mission: "かかっている費用と、費用を理由にやっていないこと",
        format: Format::Yaml,
    },
];

impl Def {
    /// 拡張子。
    pub fn extension(&self) -> &'static str {
        match self.format {
            Format::Yaml => "yaml",
            Format::Markdown => "md",
        }
    }

    /// Slot ファイルの、dir.slot からの相対パス。
    pub fn file_name(&self) -> String {
        format!("{}/{}.{}", self.group.dir(), self.id, self.extension())
    }

    /// ページに分けるときのディレクトリの、dir.slot からの相対パス。直下に 1 ページ 1 つの md を置く。
    /// 分けられるのはサービス仕様だけで、ほかの Slot は None。
    pub fn pages_dir(&self) -> Option<String> {
        (self.id == "service").then(|| format!("{}/{}", self.group.dir(), self.id))
    }
}

/// dir.slot からの相対パス（`service/words.yaml`）から Slot を引く。
pub fn def_of_file(rel: &str) -> Option<&'static Def> {
    DEFS.iter().find(|d| d.file_name() == rel)
}

/// dir.slot からの相対パスが、分けた Slot のページ（`service/service/admin.md`）なら、その Slot を引く。
pub fn def_of_page(rel: &str) -> Option<&'static Def> {
    let (dir, name) = rel.rsplit_once('/')?;
    if !name.ends_with(".md") {
        return None;
    }
    DEFS.iter()
        .find(|d| d.pages_dir().is_some_and(|pages| pages == dir))
}

/// Slot ファイルとして置ける名前の一覧。メッセージに使う。
pub fn file_names() -> String {
    DEFS.iter()
        .flat_map(|d| {
            std::iter::once(d.file_name()).chain(d.pages_dir().map(|dir| format!("{dir}/*.md")))
        })
        .collect::<Vec<_>>()
        .join("、")
}

/// Slot の名前に近いパス（ディレクトリが違う `words.yaml`、番号が付いた `service/03-words.yaml`、
/// 拡張子が違う `service/service.yaml`）なら、正しいパスを返す。
/// dir は dir.slot。返すパスはルートからの相対パス。
pub fn suggest(dir: &str, rel: &str) -> Option<String> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let (stem, _) = name.rsplit_once('.')?;
    let id = stem
        .split_once('-')
        .filter(|(n, _)| n.len() == 2 && n.chars().all(|c| c.is_ascii_digit()))
        .map_or(stem, |(_, id)| id);
    def(id).map(|d| format!("{dir}/{}", d.file_name()))
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
    /// 項目のあとに続けて出す Markdown。
    #[serde(rename = "appendix_markdown")]
    pub appendix: Option<String>,
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
    #[serde(rename = "body_markdown")]
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
    #[serde(rename = "purpose_markdown")]
    pub purpose: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    #[serde(default)]
    pub groups: Vec<WordGroup>,
    /// 項目のあとに続けて出す Markdown。
    #[serde(rename = "appendix_markdown")]
    pub appendix: Option<String>,
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
    #[serde(rename = "desc_markdown")]
    pub desc: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    #[serde(default)]
    pub stack: Vec<StackGroup>,
    /// 項目のあとに続けて出す Markdown。
    #[serde(rename = "appendix_markdown")]
    pub appendix: Option<String>,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    #[serde(default)]
    pub entries: Vec<LayoutEntry>,
    /// 項目のあとに続けて出す Markdown。
    #[serde(rename = "appendix_markdown")]
    pub appendix: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutEntry {
    pub path: String,
    #[serde(rename = "desc_markdown")]
    pub description: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Env {
    #[serde(default)]
    pub envs: Vec<EnvVar>,
    /// 項目のあとに続けて出す Markdown。
    #[serde(rename = "appendix_markdown")]
    pub appendix: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvVar {
    pub key: String,
    #[serde(rename = "desc_markdown")]
    pub desc: String,
    #[serde(default)]
    pub secret: bool,
    /// ローカル環境で、どこで設定しているか（Docker Compose、.env など）。
    #[serde(rename = "local_from_markdown")]
    pub local_from: Option<String>,
    /// ステージングや本番で、どこで設定しているか（ECS のタスク定義、SSM など）。
    #[serde(rename = "release_from_markdown")]
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
    /// 項目のあとに続けて出す Markdown。
    #[serde(rename = "appendix_markdown")]
    pub appendix: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostItem {
    pub name: String,
    #[serde(rename = "desc_markdown")]
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
    Layout(Layout),
    Env(Env),
    Cost(Cost),
    /// Markdown の Slot。YAML front matter を除いた本文。
    Markdown(String),
}

impl Content {
    fn parse(def: &Def, text: &str) -> serde_yaml_ng::Result<Content> {
        let id = def.id;
        if def.format == Format::Markdown {
            return Ok(Content::Markdown(markdown::split(text).body));
        }
        // 空のファイルは、項目のない Slot として扱う。
        let text = if text.trim().is_empty() { "{}" } else { text };
        Ok(match id {
            "install" | "setup" | "develop" => Content::Procedure(serde_yaml_ng::from_str(text)?),
            "words" => Content::Words(serde_yaml_ng::from_str(text)?),
            "stack" => Content::Stack(serde_yaml_ng::from_str(text)?),
            "directory-tree" => Content::Layout(serde_yaml_ng::from_str(text)?),
            "env" => Content::Env(serde_yaml_ng::from_str(text)?),
            "cost" => Content::Cost(serde_yaml_ng::from_str(text)?),
            _ => unreachable!("YAML の Slot の ID は DEFS で確かめてある"),
        })
    }

    /// 項目のあとに続けて出す Markdown（appendix_markdown）。md の Slot は None。
    pub fn appendix(&self) -> Option<&str> {
        match self {
            Content::Procedure(c) => c.appendix.as_deref(),
            Content::Words(c) => c.appendix.as_deref(),
            Content::Stack(c) => c.appendix.as_deref(),
            Content::Layout(c) => c.appendix.as_deref(),
            Content::Env(c) => c.appendix.as_deref(),
            Content::Cost(c) => c.appendix.as_deref(),
            Content::Markdown(_) => None,
        }
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
                        w.desc
                            .lines()
                            .find(|l| !l.trim().is_empty())
                            .unwrap_or("")
                            .to_string(),
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

/// 読み込んだ Slot。ページに分けた Slot は、ページごとに 1 つ。
#[derive(Debug, Clone)]
pub struct Slot {
    pub def: &'static Def,
    /// ルートからの相対パス。
    pub path: String,
    pub content: Content,
    /// 分けた Slot のページなら、その名前と並び。分けていなければ None。
    pub page: Option<Page>,
}

/// 分けた Slot の 1 ページ。
#[derive(Debug, Clone)]
pub struct Page {
    /// ページの名前。本文の h1。
    pub title: String,
    /// 並び。YAML front matter の order。
    pub order: i64,
}

/// md の Slot の YAML front matter。mission は必ず書き、docv が決めたミッションと同じ文にする（check が確かめる）。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MdFront {
    mission: Option<String>,
}

/// 分けた Slot のページの YAML front matter。mission は、分けていない md の Slot と同じ。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageFront {
    order: i64,
    mission: Option<String>,
}

/// 読んだページ。
struct ParsedPage {
    page: Page,
    /// YAML front matter を除いた本文。
    body: String,
    /// h1 の行。check で、h1 の重なりを指す。
    title_line: usize,
    /// YAML front matter の mission と、その行。
    mission: Option<String>,
    mission_line: usize,
}

/// md の Slot の mission を確かめる。ないか、docv が決めたミッションと違えば、その行とメッセージ。
fn mission_problem(def: &Def, mission: Option<&str>, line: usize) -> Option<(usize, String)> {
    match mission {
        Some(mission) if mission == def.mission => None,
        Some(_) => Some((
            line,
            format!(
                "mission が、{}のミッションと違う。mission: {} にする",
                def.label, def.mission
            ),
        )),
        None => Some((
            line,
            format!(
                "YAML front matter に mission がない。mission: {} を書く",
                def.mission
            ),
        )),
    }
}

/// 分けていない md の Slot の YAML front matter を確かめる。問題があれば、その行、ルール、メッセージ。
fn md_front_problem(def: &Def, text: &str) -> Option<(usize, &'static str, String)> {
    let document = markdown::split(text);
    let Some(fm) = &document.frontmatter else {
        return Some((
            1,
            "slot-mission",
            format!(
                "YAML front matter がない。先頭に --- で囲んで mission: {} を書く",
                def.mission
            ),
        ));
    };
    if !fm.closed {
        return Some((
            1,
            "slot-invalid",
            "YAML front matter を閉じる --- がない".to_string(),
        ));
    }
    let yaml = if fm.yaml.trim().is_empty() {
        "{}"
    } else {
        &fm.yaml
    };
    let front: MdFront = match serde_yaml_ng::from_str(yaml) {
        Ok(front) => front,
        Err(e) => {
            return Some((
                e.location().map_or(1, |l| l.line() + 1),
                "slot-invalid",
                format!("{}の YAML front matter の形に合わない: {e}", def.label),
            ));
        }
    };
    mission_problem(def, front.mission.as_deref(), fm.line_of_key("mission"))
        .map(|(line, message)| (line, "slot-mission", message))
}

/// 分けた Slot のページを読む。読めないときは、その理由（行、メッセージ）。
fn parse_page(def: &Def, text: &str) -> Result<ParsedPage, (usize, String)> {
    let document = markdown::split(text);
    let Some(fm) = &document.frontmatter else {
        return Err((
            1,
            format!(
                "YAML front matter がない。ページに分けた{}には、どのページにも order と mission を書く",
                def.label
            ),
        ));
    };
    let front: PageFront = serde_yaml_ng::from_str(&fm.yaml).map_err(|e| {
        (
            e.location().map_or(1, |l| l.line() + 1),
            format!(
                "YAML front matter が{}のページの形に合わない: {e}",
                def.label
            ),
        )
    })?;
    let outline = markdown::outline(&document);
    let Some(h1) = outline.headings.iter().find(|h| h.level == 1) else {
        return Err((
            document.body_start_line,
            "h1 がない。h1 がページの名前になる".to_string(),
        ));
    };
    Ok(ParsedPage {
        page: Page {
            title: h1.text.clone(),
            order: front.order,
        },
        body: document.body.clone(),
        title_line: h1.line,
        mission: front.mission,
        mission_line: fm.line_of_key("mission"),
    })
}

/// Slot のディレクトリにあるファイル。
struct SlotFile {
    path: String,
    /// Slot。Slot の名前でなければ None。
    def: Option<&'static Def>,
    /// 分けた Slot のページか。
    page: bool,
}

impl SlotFile {
    /// 同じ Slot を、分けずに 1 つのファイルで置いているか。ページと両方あれば、1 つのファイルを読む（check が報告する）。
    fn has_whole(&self, files: &[SlotFile]) -> bool {
        let Some(def) = self.def else { return false };
        files
            .iter()
            .any(|f| !f.page && f.def.is_some_and(|d| d.id == def.id))
    }
}

/// 除外するファイルと、名前がドットで始まるファイルは含まない（Project::slot_dir_files）。
fn slot_files(project: &Project) -> Result<Vec<SlotFile>> {
    let dir = project.slot_dir();
    let prefix = format!("{dir}/");
    // adr/ は ADR のディレクトリ。Slot としては読まない（docs/spec/adr.md）。
    let adr_dir = format!("{prefix}{}/", crate::adr::DIR);
    Ok(project
        .slot_dir_files()?
        .into_iter()
        .filter(|path| !path.starts_with(&adr_dir))
        .map(|path| {
            let rel = path.strip_prefix(&prefix).unwrap_or(&path);
            let (def, page) = match (def_of_file(rel), def_of_page(rel)) {
                (Some(def), _) => (Some(def), false),
                (None, page) => (page, page.is_some()),
            };
            SlotFile { path, def, page }
        })
        .collect())
}

/// YAML の Slot ファイルのパス（ルートからの相対パス）。
pub fn yaml_paths(project: &Project) -> Result<Vec<String>> {
    Ok(slot_files(project)?
        .into_iter()
        .filter(|f| f.def.is_some_and(|d| d.format == Format::Yaml))
        .map(|f| f.path)
        .collect())
}

/// Slot を DEFS の順に読む。分けた Slot のページは、order とパスの順に並べる。読めない Slot は飛ばす（check が報告する）。
pub fn load(project: &Project) -> Result<Vec<Slot>> {
    let files = slot_files(project)?;
    let mut slots: Vec<Slot> = Vec::new();
    for file in &files {
        let Some(def) = file.def else { continue };
        if file.page && file.has_whole(&files) {
            continue;
        }
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        let parsed = if file.page {
            parse_page(def, &text)
                .ok()
                .map(|p| (Content::Markdown(p.body), Some(p.page)))
        } else {
            Content::parse(def, &text).ok().map(|c| (c, None))
        };
        if let Some((content, page)) = parsed {
            slots.push(Slot {
                def,
                path: file.path.clone(),
                content,
                page,
            });
        }
    }
    let key = |s: &Slot| {
        (
            DEFS.iter().position(|d| d.id == s.def.id),
            s.page.as_ref().map(|p| p.order),
        )
    };
    slots.sort_by(|a, b| key(a).cmp(&key(b)).then_with(|| a.path.cmp(&b.path)));
    Ok(slots)
}

/// Slot ファイルを確かめる。
pub fn check(project: &Project) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut push = |path: &str, line: usize, rule: &str, message: String| {
        diagnostics.push(Diagnostic {
            path: path.to_string(),
            line,
            rule: rule.to_string(),
            message,
        });
    };
    let files = slot_files(project)?;
    // 分けた Slot のページ。(パス, ページ, order の行, h1 の行)
    let mut pages: Vec<(&str, Page, usize, usize)> = Vec::new();
    for file in &files {
        let Some(def) = file.def else {
            let message = match suggest(&project.slot_dir(), &file.path) {
                Some(right) => format!("Slot ファイルは {right} に置く"),
                None => format!(
                    "Slot の名前ではない。Slot のディレクトリに置けるのは {}",
                    file_names()
                ),
            };
            push(&file.path, 1, "slot-unknown", message);
            continue;
        };
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        let line_of = |needle: &str| {
            text.lines()
                .position(|l| l.contains(needle))
                .map_or(1, |i| i + 1)
        };
        if file.page {
            match parse_page(def, &text) {
                Ok(p) => {
                    if let Some((line, message)) =
                        mission_problem(def, p.mission.as_deref(), p.mission_line)
                    {
                        push(&file.path, line, "slot-mission", message);
                    }
                    pages.push((&file.path, p.page, line_of("order:"), p.title_line));
                }
                Err((line, message)) => push(&file.path, line, "slot-invalid", message),
            }
            continue;
        }
        if def.format == Format::Markdown
            && let Some((line, rule, message)) = md_front_problem(def, &text)
        {
            push(&file.path, line, rule, message);
        }
        if let Some(dir) = def.pages_dir()
            && files
                .iter()
                .any(|f| f.page && f.def.is_some_and(|d| d.id == def.id))
        {
            push(
                &file.path,
                1,
                "slot-invalid",
                format!(
                    "{}が {} と {}/{dir}/ の両方にある。ページに分けるなら {} を消す",
                    def.label,
                    file.path,
                    project.slot_dir(),
                    file.path
                ),
            );
        }
        let content = match Content::parse(def, &text) {
            Ok(content) => content,
            Err(e) => {
                let line = e.location().map_or(1, |l| l.line());
                push(
                    &file.path,
                    line,
                    "slot-invalid",
                    format!("{}の Slot の形に合わない: {e}", def.label),
                );
                continue;
            }
        };
        for (needle, message) in content.problems() {
            push(&file.path, line_of(&needle), "slot-invalid", message);
        }
        for (field, path) in content.paths() {
            if !project.root.join(path).exists() {
                push(
                    &file.path,
                    line_of(path),
                    "slot-path-missing",
                    format!("{field} の {path} がない"),
                );
            }
        }
    }
    // ページどうしで、order と h1 を重ねない。後のパスのページに出す。
    for (i, (path, page, order_line, title_line)) in pages.iter().enumerate() {
        let earlier = &pages[..i];
        if let Some((other, ..)) = earlier.iter().find(|(_, p, ..)| p.order == page.order) {
            push(
                path,
                *order_line,
                "slot-invalid",
                format!("order {} が {other} と重なっている", page.order),
            );
        }
        if let Some((other, ..)) = earlier.iter().find(|(_, p, ..)| p.title == page.title) {
            push(
                path,
                *title_line,
                "slot-invalid",
                format!(
                    "h1「{}」が {other} と重なっている。h1 はページの名前で、重ねない",
                    page.title
                ),
            );
        }
    }
    Ok(diagnostics)
}

/// yaml の Slot のテンプレ。docv slot gen で書く、項目のサンプル。
pub fn template(id: &str) -> Option<&'static str> {
    Some(match id {
        "install" => include_str!("../assets/template/install.yaml"),
        "words" => include_str!("../assets/template/words.yaml"),
        "stack" => include_str!("../assets/template/stack.yaml"),
        "directory-tree" => include_str!("../assets/template/directory-tree.yaml"),
        "setup" => include_str!("../assets/template/setup.yaml"),
        "env" => include_str!("../assets/template/env.yaml"),
        "develop" => include_str!("../assets/template/develop.yaml"),
        "cost" => include_str!("../assets/template/cost.yaml"),
        _ => return None,
    })
}

/// md の Slot のテンプレ。docv slot gen で書く雛形。Doc ガイドの Slot（doc-guide）だけがガイドラインの雛形を持ち、ほかの md の Slot は、YAML front matter の mission だけを書いたファイルを作る。
pub fn md_template(id: &str) -> Option<&'static str> {
    match id {
        "doc-guide" => Some(include_str!("../assets/template/doc-guide.md")),
        _ => None,
    }
}

/// yaml の Slot の JSON Schema。docv slot schema で出し、Slot ファイルの 1 行目から参照する。md の Slot はスキーマを持たない。
pub fn schema(id: &str) -> Option<&'static str> {
    Some(match id {
        "install" => include_str!("../assets/schema/install.schema.json"),
        "words" => include_str!("../assets/schema/words.schema.json"),
        "stack" => include_str!("../assets/schema/stack.schema.json"),
        "directory-tree" => include_str!("../assets/schema/directory-tree.schema.json"),
        "setup" => include_str!("../assets/schema/setup.schema.json"),
        "env" => include_str!("../assets/schema/env.schema.json"),
        "develop" => include_str!("../assets/schema/develop.schema.json"),
        "cost" => include_str!("../assets/schema/cost.schema.json"),
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
        write(dir.path(), "docv.yaml", "{}\n");
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    /// md の Slot の中身。YAML front matter の mission に、docv が決めたミッションを書く。
    fn md(id: &str, body: &str) -> String {
        format!("---\nmission: {}\n---\n\n{body}", def(id).unwrap().mission)
    }

    /// 分けたサービス仕様のページの中身。
    fn page(order: i64, body: &str) -> String {
        format!(
            "---\norder: {order}\nmission: {}\n---\n\n{body}",
            def("service").unwrap().mission
        )
    }

    #[test]
    fn every_yaml_slot_has_a_template_that_parses() {
        for def in DEFS.iter().filter(|d| d.format == Format::Yaml) {
            let text = template(def.id).unwrap();
            Content::parse(def, text).unwrap_or_else(|e| panic!("{}: {e}", def.id));
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
            ("docv/system/api.md", &md("api", "API\n")),
            ("docv/system/api.yaml", "ref: openapi.yaml\n"),
            (
                "docv/service/install.yaml",
                "steps:\n  - id: binary\n    name: a\n    items:\n      - os: macos\n      - body_markdown: m\n",
            ),
            (
                "docv/guide/directory-tree.yaml",
                "entries:\n  - path: src/\n    desc_markdown: |\n      d\n",
            ),
            ("docs/a.md", "# A\n\n## 1. ある\n"),
        ]);
        let diagnostics = check(&project).unwrap();
        let result: Vec<String> = diagnostics
            .iter()
            .map(|d| format!("{}:{} {}", d.path, d.line, d.rule))
            .collect();
        assert_eq!(
            result,
            [
                "docv/guide/directory-tree.yaml:2 slot-path-missing",
                "docv/service/03-words.yaml:1 slot-unknown",
                "docv/service/install.yaml:2 slot-invalid",
                "docv/service/words.yaml:2 slot-invalid",
                "docv/system/api.yaml:1 slot-unknown",
                "docv/unknown.yaml:1 slot-unknown",
                "docv/words.yaml:1 slot-unknown",
            ]
        );
        // ディレクトリが違う名前、番号を付けた名前には、正しいパスを示す
        assert!(diagnostics[1].message.contains("docv/service/words.yaml"));
        assert!(diagnostics[6].message.contains("docv/service/words.yaml"));
        // API 仕様書は md だけ。yaml には、md のパスを示す
        assert!(diagnostics[4].message.contains("docv/system/api.md"));
    }

    #[test]
    fn reports_missions_of_md_slots() {
        let (_dir, project) = project(&[
            // docv が決めたミッションと同じなら、指摘しない
            ("docv/service/manual.md", &md("manual", "# M\n")),
            ("docv/system/api.md", "# API\n"),
            (
                "docv/system/db.md",
                "---\nmission: 保存しているデータ\n---\n\n# DB\n",
            ),
            ("docv/system/system.md", "---\ntags: [a]\n---\n\n# S\n"),
            ("docv/guide/doc-guide.md", "---\n---\n\n# G\n"),
            ("docv/operation/deploy.md", "---\nmission: x\n"),
            (
                "docv/service/service/a.md",
                "---\norder: 1\nmission: 機能\n---\n\n# A\n",
            ),
            ("docv/service/service/b.md", "---\norder: 2\n---\n\n# B\n"),
        ]);
        let diagnostics = check(&project).unwrap();
        let mut result: Vec<String> = diagnostics
            .iter()
            .map(|d| format!("{}:{} {} {}", d.path, d.line, d.rule, d.message))
            .collect();
        result.sort();
        let service = def("service").unwrap().mission;
        assert_eq!(
            result,
            [
                format!(
                    "docv/guide/doc-guide.md:1 slot-mission YAML front matter に mission がない。mission: {} を書く",
                    def("doc-guide").unwrap().mission
                ),
                "docv/operation/deploy.md:1 slot-invalid YAML front matter を閉じる --- がない".to_string(),
                format!(
                    "docv/service/service/a.md:3 slot-mission mission が、サービス仕様のミッションと違う。mission: {service} にする"
                ),
                format!(
                    "docv/service/service/b.md:1 slot-mission YAML front matter に mission がない。mission: {service} を書く"
                ),
                format!(
                    "docv/system/api.md:1 slot-mission YAML front matter がない。先頭に --- で囲んで mission: {} を書く",
                    def("api").unwrap().mission
                ),
                format!(
                    "docv/system/db.md:2 slot-mission mission が、DB 仕様書のミッションと違う。mission: {} にする",
                    def("db").unwrap().mission
                ),
                "docv/system/system.md:2 slot-invalid システム仕様の YAML front matter の形に合わない: unknown field `tags`, expected `mission`".to_string(),
            ]
        );
        // 指摘があっても、HTML には本文を出す
        let ids: Vec<&str> = load(&project).unwrap().iter().map(|s| s.def.id).collect();
        assert!(ids.contains(&"api") && ids.contains(&"db"), "{ids:?}");
    }

    #[test]
    fn loads_pages_of_a_split_slot_in_order() {
        let (_dir, project) = project(&[
            ("docv/service/manual.md", "# マニュアル\n"),
            (
                "docv/service/service/user.md",
                "---\norder: 2\n---\n\n# 利用者向け機能\n\n本文\n",
            ),
            (
                "docv/service/service/admin.md",
                "---\norder: 1\n---\n\n# 管理者向け機能\n",
            ),
            // 読めないページは飛ばす（check が報告する）
            ("docv/service/service/memo.md", "# メモ\n"),
        ]);
        let slots = load(&project).unwrap();
        let pages: Vec<(&str, Option<&str>)> = slots
            .iter()
            .map(|s| (s.def.id, s.page.as_ref().map(|p| p.title.as_str())))
            .collect();
        assert_eq!(
            pages,
            [
                ("service", Some("管理者向け機能")),
                ("service", Some("利用者向け機能")),
                ("manual", None),
            ]
        );
        // 本文から YAML front matter を除く
        assert!(
            matches!(&slots[1].content, Content::Markdown(text) if text.starts_with("\n# 利用者向け機能"))
        );
    }

    #[test]
    fn reads_the_whole_file_when_pages_also_exist() {
        let (_dir, project) = project(&[
            ("docv/service/service.md", "# サービス仕様\n"),
            (
                "docv/service/service/user.md",
                "---\norder: 1\n---\n\n# 利用者向け機能\n",
            ),
        ]);
        let slots = load(&project).unwrap();
        assert_eq!(slots.len(), 1);
        assert!(slots[0].page.is_none());
    }

    #[test]
    fn reports_problems_of_pages() {
        let (_dir, project) = project(&[
            (
                "docv/service/service.md",
                &md("service", "# サービス仕様\n"),
            ),
            ("docv/service/service/a.md", &page(1, "# A\n")),
            ("docv/service/service/b.md", &page(1, "# A\n")),
            ("docv/service/service/c.md", "# C\n"),
            (
                "docv/service/service/d.md",
                "---\norder: 4\nname: D\n---\n\n# D\n",
            ),
            ("docv/service/service/e.md", "---\norder: 5\n---\n\n本文\n"),
            ("docv/service/service/sub/f.md", "# F\n"),
            ("docv/system/system/a.md", "# A\n"),
        ]);
        let diagnostics = check(&project).unwrap();
        let mut result: Vec<String> = diagnostics
            .iter()
            .map(|d| format!("{}:{} {} {}", d.path, d.line, d.rule, d.message))
            .collect();
        result.sort();
        assert_eq!(
            result,
            [
                "docv/service/service.md:1 slot-invalid サービス仕様が docv/service/service.md と docv/service/service/ の両方にある。ページに分けるなら docv/service/service.md を消す",
                "docv/service/service/b.md:2 slot-invalid order 1 が docv/service/service/a.md と重なっている",
                "docv/service/service/b.md:6 slot-invalid h1「A」が docv/service/service/a.md と重なっている。h1 はページの名前で、重ねない",
                "docv/service/service/c.md:1 slot-invalid YAML front matter がない。ページに分けたサービス仕様には、どのページにも order と mission を書く",
                "docv/service/service/d.md:3 slot-invalid YAML front matter がサービス仕様のページの形に合わない: unknown field `name`, expected `order` or `mission` at line 2 column 1",
                "docv/service/service/e.md:4 slot-invalid h1 がない。h1 がページの名前になる",
                &format!(
                    "docv/service/service/sub/f.md:1 slot-unknown Slot の名前ではない。Slot のディレクトリに置けるのは {}",
                    file_names()
                ),
                &format!(
                    "docv/system/system/a.md:1 slot-unknown Slot の名前ではない。Slot のディレクトリに置けるのは {}",
                    file_names()
                ),
            ]
        );
        // ページに分けられるのはサービス仕様だけ。置ける名前の一覧にも、そのディレクトリを出す
        assert!(
            file_names().contains("service/service.md、service/service/*.md、service/manual.md")
        );
    }

    #[test]
    fn needs_ja_or_en_for_each_word() {
        let (_dir, project) = project(&[(
            "docv/service/words.yaml",
            "groups:\n  - name: g\n    items:\n      - en: Slot\n        desc_markdown: |\n          a\n      - desc_markdown: |\n          b\n",
        )]);
        let result: Vec<String> = check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{}:{} {}", d.path, d.line, d.rule))
            .collect();
        assert_eq!(result, ["docv/service/words.yaml:8 slot-invalid"]);
    }

    #[test]
    fn reads_one_item_or_many() {
        let install = def("install").unwrap();
        let one = Content::parse(
            install,
            "steps:\n  - id: cargo\n    name: c\n    items:\n      body_markdown: m\n",
        )
        .unwrap();
        let many = Content::parse(
            install,
            "steps:\n  - id: binary\n    name: b\n    items:\n      - os: windows\n      - os: macos\n",
        )
        .unwrap();
        let count = |c: &Content| match c {
            Content::Procedure(p) => p.steps[0].items.len(),
            _ => 0,
        };
        assert_eq!((count(&one), count(&many)), (1, 2));
    }
}
