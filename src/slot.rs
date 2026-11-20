//! Slot。どのプロジェクトにも要るドキュメントを、docv が決めたミッションで書くファイル。docv/slot/service/service/slot.md。
//!
//! Slot ファイルは、すべて Markdown。Slot のディレクトリ（document_root の下の slot/）の、まとまりのディレクトリに `<まとまり>/<ID>.md` の名前で置く。
//! サービス仕様、システム仕様、開発コマンド、アーキテクチャ、設計・実装、テスト方針、インフラと環境は、`<まとまり>/<ID>/` の直下に 1 ページ 1 つの Markdown で置いて、ページに分けられる。
//! ページの名前は h1、並びは YAML front matter の order。
//! Slot（分けたページも）は、YAML front matter の mission に、docv が決めたミッションをそのまま書く。違えば check が error にする。
//! YAML front matter には、ほかに関わる決定の adr と pdr（meta::Related）を書ける。分けたページは order も書く。
//! 中身の形は確かめない。雛形のある Slot は、docv gen が雛形を書く。

use std::fs;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::decision::Kind;
use crate::diagnostic::Diagnostic;
use crate::markdown;
use crate::meta::{self, Related};
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

    /// Slot ファイルを置くディレクトリの名前（Slot のディレクトリからの相対パス）。
    pub fn dir(self) -> &'static str {
        match self {
            Group::Service => "service",
            Group::System => "system",
            Group::Developer => "guide",
            Group::Operate => "operation",
        }
    }
}

#[derive(Debug)]
pub struct Def {
    pub id: &'static str,
    pub label: &'static str,
    pub group: Group,
    /// ミッション。この Slot を読むと何が分かるか。
    pub mission: &'static str,
}

/// Slot の一覧。並び順は HTML に出す順。
pub const DEFS: &[Def] = &[
    Def {
        id: "overview",
        label: "サービス概要",
        group: Group::Service,
        mission: "利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明",
    },
    Def {
        id: "service",
        label: "サービス仕様",
        group: Group::Service,
        mission: "外から見た機能と、それぞれの振る舞いと決まり",
    },
    Def {
        id: "install",
        label: "インストール",
        group: Group::Service,
        mission: "利用者がこのサービスをインストールする方法",
    },
    Def {
        id: "manual",
        label: "利用マニュアル",
        group: Group::Service,
        mission: "利用者の使い方（CLI のコマンド、GUI の操作など）",
    },
    Def {
        id: "words",
        label: "用語集",
        group: Group::Service,
        mission: "このプロジェクトの言葉の意味",
    },
    Def {
        id: "system-overview",
        label: "システム概要",
        group: Group::System,
        mission: "開発者に向けた、システムを作る部品と、そのつながりの全体像",
    },
    Def {
        id: "system",
        label: "システム仕様",
        group: Group::System,
        mission: "部品ごとの振る舞いと、部品の間の流れ",
    },
    Def {
        id: "stack",
        label: "技術スタック",
        group: Group::System,
        mission: "利用している外部サービス、言語、ライブラリなどの名前と用途",
    },
    Def {
        id: "api",
        label: "API 仕様書",
        group: Group::System,
        mission: "API の一覧と、それぞれが受け取るものと返すもの",
    },
    Def {
        id: "db",
        label: "DB 仕様書",
        group: Group::System,
        mission: "保存しているデータと、その保存先と形",
    },
    Def {
        id: "setup",
        label: "環境構築",
        group: Group::Developer,
        mission: "開発を始めるのに用意するものと、実行するコマンド",
    },
    Def {
        id: "develop",
        label: "開発コマンド",
        group: Group::Developer,
        mission: "テストやビルドなど、開発者が実行するコマンド",
    },
    Def {
        id: "architecture",
        label: "アーキテクチャ",
        group: Group::Developer,
        mission: "システム全体で採る方式と決まり（部品の分け方、通信の方式、非同期処理の実現方法など）",
    },
    Def {
        id: "design",
        label: "設計・実装",
        group: Group::Developer,
        mission: "コードを書くときに従う決まり（フレームワークの使い方、ディレクトリやコーディングの決まりなど）",
    },
    Def {
        id: "testing",
        label: "テスト方針",
        group: Group::Developer,
        mission: "テストの範囲と方法（自動テストの範囲、E2E テスト、手動テストについてなど）",
    },
    Def {
        id: "env",
        label: "環境変数",
        group: Group::Developer,
        mission: "システムを動かすのに要る環境変数と、それぞれを設定している場所",
    },
    Def {
        id: "doc-guide",
        label: "Doc ガイド",
        group: Group::Developer,
        mission: "このプロジェクトのドキュメントの置き場所と書き方",
    },
    Def {
        id: "infrastructure",
        label: "インフラと環境",
        group: Group::Operate,
        mission: "環境（本番、ステージング、開発など）の一覧と、それぞれの用途",
    },
    Def {
        id: "deploy",
        label: "デプロイ",
        group: Group::Operate,
        mission: "作ったものを環境に反映する手順",
    },
    Def {
        id: "release",
        label: "リリース",
        group: Group::Operate,
        mission: "版の決め方と、利用者への届け方",
    },
    Def {
        id: "cost",
        label: "費用",
        group: Group::Operate,
        mission: "このプロジェクトの費用（月額費や、費用を理由にやっていないことなど）",
    },
];

impl Def {
    /// Slot ファイルの、Slot のディレクトリからの相対パス。
    pub fn file_name(&self) -> String {
        format!("{}/{}.md", self.group.dir(), self.id)
    }

    /// ページに分けるときのディレクトリの、Slot のディレクトリからの相対パス。直下に 1 ページ 1 つの Markdown を置く。
    /// 分けられるのはサービス仕様、システム仕様、開発コマンド、アーキテクチャ、設計・実装、テスト方針、インフラと環境だけで、ほかの Slot は None。
    pub fn pages_dir(&self) -> Option<String> {
        matches!(
            self.id,
            "service"
                | "system"
                | "develop"
                | "architecture"
                | "design"
                | "testing"
                | "infrastructure"
        )
        .then(|| format!("{}/{}", self.group.dir(), self.id))
    }
}

/// Slot のディレクトリからの相対パス（`service/words.md`）から Slot を引く。
pub fn def_of_file(rel: &str) -> Option<&'static Def> {
    DEFS.iter().find(|d| d.file_name() == rel)
}

/// Slot のディレクトリからの相対パスが、分けた Slot のページ（`service/service/admin.md`）なら、その Slot を引く。
pub fn def_of_page(rel: &str) -> Option<&'static Def> {
    let (dir, name) = rel.rsplit_once('/')?;
    DEFS.iter()
        .find(|d| d.pages_dir().is_some_and(|pages| pages == dir) && name.ends_with(".md"))
}

/// ルートからの相対パスが、Slot ファイルか分けた Slot のページなら、その Slot を引く。
pub fn def_of_path(project: &Project, path: &str) -> Option<&'static Def> {
    let rel = path.strip_prefix(&format!("{}/", project.slot_dir()))?;
    def_of_file(rel).or_else(|| def_of_page(rel))
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

/// Slot の名前に近いパス（ディレクトリが違う `words.md`、番号が付いた `service/03-words.md`、
/// 拡張子が違う `service/service.txt`）なら、正しいパスを返す。
/// dir は Slot のディレクトリ。返すパスはルートからの相対パス。
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

/// 以前は YAML で書いた Slot。ファイルが残っていれば、雛形を見て Markdown に書き直すよう案内する。
const FORMER_YAML: [&str; 7] = [
    "install", "words", "stack", "setup", "develop", "env", "cost",
];

/// 残っている YAML の Slot。
struct FormerYaml {
    def: &'static Def,
    /// 書き直す先の、Slot のディレクトリからの相対パス。
    rel: String,
    /// 分けた開発コマンドのページか。
    page: bool,
}

/// Slot のディレクトリからの相対パスが、以前は YAML で書いた Slot のファイル（`service/words.yaml`）か、
/// その分けたページ（`guide/develop/web.yaml`）なら、Slot と書き直す先を返す。
fn former_yaml(rel: &str) -> Option<FormerYaml> {
    let stem = rel.strip_suffix(".yaml")?;
    DEFS.iter()
        .filter(|d| FORMER_YAML.contains(&d.id))
        .find_map(|def| {
            let page = match stem.rsplit_once('/') {
                _ if stem == format!("{}/{}", def.group.dir(), def.id) => false,
                Some((dir, name))
                    if !name.is_empty() && def.pages_dir().as_deref() == Some(dir) =>
                {
                    true
                }
                _ => return None,
            };
            Some(FormerYaml {
                def,
                rel: format!("{stem}.md"),
                page,
            })
        })
}

/// 雛形。docv gen で書く。雛形のない Slot は None で、YAML front matter の mission だけを書いたファイルを作る。
/// 雛形にも mission を書いてある。作った雛形はプロジェクトのもので、docv の版を上げても書き換えない。
pub fn template(id: &str) -> Option<&'static str> {
    Some(match id {
        "install" => include_str!("../assets/template/install.md"),
        "words" => include_str!("../assets/template/words.md"),
        "stack" => include_str!("../assets/template/stack.md"),
        "setup" => include_str!("../assets/template/setup.md"),
        "develop" => include_str!("../assets/template/develop.md"),
        "env" => include_str!("../assets/template/env.md"),
        "doc-guide" => include_str!("../assets/template/doc-guide.md"),
        "cost" => include_str!("../assets/template/cost.md"),
        _ => return None,
    })
}

/// 読み込んだ Slot。ページに分けた Slot は、ページごとに 1 つ。
#[derive(Debug, Clone)]
pub struct Slot {
    pub def: &'static Def,
    /// ルートからの相対パス。
    pub path: String,
    /// YAML front matter を除いた本文。
    pub body: String,
    /// 分けた Slot のページなら、その名前と並び。分けていなければ None。
    pub page: Option<Page>,
    /// YAML front matter の adr と pdr。
    pub related: Related,
}

/// 分けた Slot の 1 ページ。
#[derive(Debug, Clone)]
pub struct Page {
    /// ページの名前。本文の h1。
    pub title: String,
    /// 並び。YAML front matter の order。
    pub order: i64,
}

/// 分けていない Slot の YAML front matter。mission は必ず書き、docv が決めたミッションと同じ文にする（check が確かめる）。
/// adr と pdr は、書けることだけをここで受ける。型は Free ファイルと同じく related_problem で確かめ、中身は meta::related で読む。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MdFront {
    mission: Option<String>,
    #[serde(default, rename = "adr")]
    _adr: IgnoredAny,
    #[serde(default, rename = "pdr")]
    _pdr: IgnoredAny,
}

/// 分けた Slot のページの YAML front matter。mission、adr、pdr は、分けていない Slot と同じ。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageFront {
    order: i64,
    mission: Option<String>,
    #[serde(default, rename = "adr")]
    _adr: IgnoredAny,
    #[serde(default, rename = "pdr")]
    _pdr: IgnoredAny,
}

/// YAML front matter の adr と pdr が、文字列の配列でなければ、最初の 1 つの行とメッセージ。
fn related_problem(fm: &markdown::Frontmatter) -> Option<(usize, String)> {
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&fm.yaml).ok()?;
    Kind::ALL.into_iter().find_map(|kind| {
        meta::related_type_problem(kind, value.get(kind.dir()))
            .map(|message| (fm.line_of_key(kind.dir()), message))
    })
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

/// Slot の mission を確かめる。ないか、docv が決めたミッションと違えば、その行とメッセージ。
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

/// 分けていない Slot の YAML front matter を確かめる。問題があれば、その行、ルール、メッセージ。
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
    if let Some((line, message)) = related_problem(fm) {
        return Some((line, "slot-invalid", message));
    }
    mission_problem(def, front.mission.as_deref(), fm.line_of_key("mission"))
        .map(|(line, message)| (line, "slot-mission", message))
}

/// 分けた Slot のページを読む。読めないときは、その理由（行、メッセージ）。
fn parse_md_page(def: &Def, text: &str) -> Result<ParsedPage, (usize, String)> {
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
    if let Some(problem) = related_problem(fm) {
        return Err(problem);
    }
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
/// 画像は Slot ではないので含まない。置き場所は layout::check が確かめる（docv/slot/service/service/free.md の「画像」）。
fn slot_files(project: &Project) -> Result<Vec<SlotFile>> {
    let dir = project.slot_dir();
    let prefix = format!("{dir}/");
    Ok(project
        .slot_dir_files()?
        .into_iter()
        .filter(|path| !crate::layout::is_image(path))
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

/// 置いた Slot の名前。1 つのファイルか、ページのどちらかがあれば、置いたとみなす。
/// 読めないファイルや形の合わないファイルも、置いたことに数える。形の誤りは check が報告する。
pub fn placed(project: &Project) -> Result<Vec<&'static str>> {
    Ok(slot_files(project)?
        .into_iter()
        .filter_map(|f| f.def.map(|d| d.id))
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
        let document = markdown::split(&text);
        let parsed = if file.page {
            parse_md_page(def, &text)
                .ok()
                .map(|p| (p.body, Some(p.page)))
        } else {
            Some((document.body.clone(), None))
        };
        if let Some((body, page)) = parsed {
            slots.push(Slot {
                def,
                path: file.path.clone(),
                body,
                page,
                related: meta::related(&document),
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

/// Slot ファイルを確かめる。中身の形は確かめない。
pub fn check(project: &Project) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut push = |path: &str, line: usize, rule: &str, message: String| {
        diagnostics.push(Diagnostic::error(path, line, rule, message));
    };
    let slot_dir = project.slot_dir();
    let prefix = format!("{slot_dir}/");
    let files = slot_files(project)?;
    // 分けた Slot のページ。(Slot, パス, ページ, order の行, h1 の行)
    let mut pages: Vec<(&Def, &str, Page, usize, usize)> = Vec::new();
    for file in &files {
        let Some(def) = file.def else {
            let rel = file.path.strip_prefix(&prefix).unwrap_or(&file.path);
            push(
                &file.path,
                1,
                "slot-unknown",
                unknown_message(&slot_dir, rel),
            );
            continue;
        };
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        if file.page {
            match parse_md_page(def, &text) {
                Ok(p) => {
                    if let Some((line, message)) =
                        mission_problem(def, p.mission.as_deref(), p.mission_line)
                    {
                        push(&file.path, line, "slot-mission", message);
                    }
                    let order_line = text
                        .lines()
                        .position(|l| l.contains("order:"))
                        .map_or(1, |i| i + 1);
                    pages.push((def, &file.path, p.page, order_line, p.title_line));
                }
                Err((line, message)) => push(&file.path, line, "slot-invalid", message),
            }
            continue;
        }
        if let Some((line, rule, message)) = md_front_problem(def, &text) {
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
                    "{}が {} と {slot_dir}/{dir}/ の両方にある。ページに分けるなら {} を消す",
                    def.label, file.path, file.path
                ),
            );
        }
    }
    // 同じ Slot のページどうしで、order と h1 を重ねない。後のパスのページに出す。
    // ほかの Slot のページとは比べない。並びも名前も、Slot ごとに決まるため。
    for (i, (def, path, page, order_line, title_line)) in pages.iter().enumerate() {
        let earlier: Vec<_> = pages[..i]
            .iter()
            .filter(|(d, ..)| d.id == def.id)
            .map(|(_, other, p, ..)| (other, p))
            .collect();
        if let Some((other, _)) = earlier.iter().find(|(_, p)| p.order == page.order) {
            push(
                path,
                *order_line,
                "slot-invalid",
                format!("order {} が {other} と重なっている", page.order),
            );
        }
        if let Some((other, _)) = earlier.iter().find(|(_, p)| p.title == page.title) {
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

/// Slot の名前でないファイルの slot-unknown のメッセージ。rel は Slot のディレクトリからの相対パス。
/// 以前は YAML で書いた Slot のファイルが残っていれば、雛形を見て Markdown に書き直すよう案内する。
/// Slot の名前に近いパスなら正しいパスを、どれでもなければ置ける名前の一覧を示す。
fn unknown_message(slot_dir: &str, rel: &str) -> String {
    if let Some(yaml) = former_yaml(rel) {
        let page = if yaml.page {
            "。order と mission は YAML front matter に、title は h1 に書く"
        } else {
            ""
        };
        return format!(
            "YAML の Slot はなくなった。docv gen {} --print の雛形を見て、{slot_dir}/{} に Markdown で書き直す{page}",
            yaml.def.id, yaml.rel
        );
    }
    match suggest(slot_dir, rel) {
        Some(right) => format!("Slot ファイルは {right} に置く"),
        None => format!(
            "Slot の名前ではない。Slot のディレクトリに置けるのは {}",
            file_names()
        ),
    }
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

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", "{}\n");
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    /// Slot の中身。YAML front matter の mission に、docv が決めたミッションを書く。
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
    fn every_template_starts_with_its_mission() {
        // 雛形にも mission を書いてある。check の slot-mission に当たらない
        for def in DEFS {
            if let Some(text) = template(def.id) {
                assert!(
                    text.starts_with(&format!("---\nmission: {}\n---\n", def.mission)),
                    "{}",
                    def.id
                );
            }
        }
    }

    #[test]
    fn loads_slots_in_definition_order() {
        let (_dir, project) = project(&[
            ("docv/slot/service/manual.md", "# マニュアル\n"),
            ("docv/slot/service/service.md", "# サービス\n\n本文\n"),
            ("docv/slot/system/api.md", "API\n"),
        ]);
        let slots = load(&project).unwrap();
        let ids: Vec<&str> = slots.iter().map(|s| s.def.id).collect();
        assert_eq!(ids, ["service", "manual", "api"]);
        assert!(slots[0].body.contains("本文"));
    }

    #[test]
    fn reports_problems_of_slot_files() {
        let (_dir, project) = project(&[
            ("docv/slot/unknown.md", "# A\n"),
            ("docv/slot/words.md", &md("words", "# 用語集\n")),
            ("docv/slot/service/03-words.md", &md("words", "# 用語集\n")),
            ("docv/slot/system/api.md", &md("api", "API\n")),
            ("docv/slot/system/api.yaml", "ref: openapi.yaml\n"),
            ("docv/slot/service/service.txt", "メモ\n"),
            ("docv/free/a.md", "# A\n\n## 1. ある\n"),
        ]);
        let diagnostics = check(&project).unwrap();
        let result: Vec<String> = diagnostics
            .iter()
            .map(|d| format!("{}:{} {}", d.path, d.line, d.rule))
            .collect();
        assert_eq!(
            result,
            [
                "docv/slot/service/03-words.md:1 slot-unknown",
                "docv/slot/service/service.txt:1 slot-unknown",
                "docv/slot/system/api.yaml:1 slot-unknown",
                "docv/slot/unknown.md:1 slot-unknown",
                "docv/slot/words.md:1 slot-unknown",
            ]
        );
        // ディレクトリが違う名前、番号を付けた名前、拡張子が違う名前には、正しいパスを示す
        for i in [0, 4] {
            assert_eq!(
                diagnostics[i].message,
                "Slot ファイルは docv/slot/service/words.md に置く"
            );
        }
        assert_eq!(
            diagnostics[1].message,
            "Slot ファイルは docv/slot/service/service.md に置く"
        );
        // 以前から Markdown だった Slot（API 仕様書）の yaml には、書き直しではなく、md のパスを示す
        assert_eq!(
            diagnostics[2].message,
            "Slot ファイルは docv/slot/system/api.md に置く"
        );
        assert!(diagnostics[3].message.starts_with("Slot の名前ではない。"));
    }

    #[test]
    fn reports_leftover_yaml_slots() {
        let (_dir, project) = project(&[
            ("docv/slot/service/words.yaml", "groups: []\n"),
            ("docv/slot/operation/cost.yaml", "accepted_items: []\n"),
            ("docv/slot/guide/develop/web.yaml", "order: 1\ntitle: Web\n"),
            // まとまりのディレクトリが違えば、ほかの名前と同じく、正しいパスを示す
            ("docv/slot/system/words.yaml", "groups: []\n"),
        ]);
        let mut result: Vec<String> = check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{}:{} {} {}", d.path, d.line, d.rule, d.message))
            .collect();
        result.sort();
        assert_eq!(
            result,
            [
                "docv/slot/guide/develop/web.yaml:1 slot-unknown YAML の Slot はなくなった。docv gen develop --print の雛形を見て、docv/slot/guide/develop/web.md に Markdown で書き直す。order と mission は YAML front matter に、title は h1 に書く",
                "docv/slot/operation/cost.yaml:1 slot-unknown YAML の Slot はなくなった。docv gen cost --print の雛形を見て、docv/slot/operation/cost.md に Markdown で書き直す",
                "docv/slot/service/words.yaml:1 slot-unknown YAML の Slot はなくなった。docv gen words --print の雛形を見て、docv/slot/service/words.md に Markdown で書き直す",
                "docv/slot/system/words.yaml:1 slot-unknown Slot ファイルは docv/slot/service/words.md に置く",
            ]
        );
        // 残った YAML は、読まず、置いたことにも数えない
        assert!(load(&project).unwrap().is_empty());
        assert!(placed(&project).unwrap().is_empty());
    }

    #[test]
    fn reports_missions_of_md_slots() {
        let (_dir, project) = project(&[
            // docv が決めたミッションと同じなら、指摘しない
            ("docv/slot/service/manual.md", &md("manual", "# M\n")),
            ("docv/slot/system/api.md", "# API\n"),
            (
                "docv/slot/system/db.md",
                "---\nmission: 保存しているデータ\n---\n\n# DB\n",
            ),
            ("docv/slot/system/system.md", "---\ntags: [a]\n---\n\n# S\n"),
            ("docv/slot/guide/doc-guide.md", "---\n---\n\n# G\n"),
            ("docv/slot/operation/deploy.md", "---\nmission: x\n"),
            (
                "docv/slot/service/service/a.md",
                "---\norder: 1\nmission: 機能\n---\n\n# A\n",
            ),
            (
                "docv/slot/service/service/b.md",
                "---\norder: 2\n---\n\n# B\n",
            ),
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
                    "docv/slot/guide/doc-guide.md:1 slot-mission YAML front matter に mission がない。mission: {} を書く",
                    def("doc-guide").unwrap().mission
                ),
                "docv/slot/operation/deploy.md:1 slot-invalid YAML front matter を閉じる --- がない".to_string(),
                format!(
                    "docv/slot/service/service/a.md:3 slot-mission mission が、サービス仕様のミッションと違う。mission: {service} にする"
                ),
                format!(
                    "docv/slot/service/service/b.md:1 slot-mission YAML front matter に mission がない。mission: {service} を書く"
                ),
                format!(
                    "docv/slot/system/api.md:1 slot-mission YAML front matter がない。先頭に --- で囲んで mission: {} を書く",
                    def("api").unwrap().mission
                ),
                format!(
                    "docv/slot/system/db.md:2 slot-mission mission が、DB 仕様書のミッションと違う。mission: {} にする",
                    def("db").unwrap().mission
                ),
                "docv/slot/system/system.md:2 slot-invalid システム仕様の YAML front matter の形に合わない: unknown field `tags`, expected one of `mission`, `adr`, `pdr`".to_string(),
            ]
        );
        // 指摘があっても、HTML には本文を出す
        let ids: Vec<&str> = load(&project).unwrap().iter().map(|s| s.def.id).collect();
        assert!(ids.contains(&"api") && ids.contains(&"db"), "{ids:?}");
    }

    #[test]
    fn reads_related_decisions_of_slots_and_pages() {
        let service = def("service").unwrap().mission;
        let (_dir, project) = project(&[
            (
                "docv/slot/service/manual.md",
                &format!(
                    "---\nmission: {}\nadr: [go]\npdr: [paid, free]\n---\n\n# M\n",
                    def("manual").unwrap().mission
                ),
            ),
            (
                "docv/slot/service/service/a.md",
                &format!("---\norder: 1\nmission: {service}\nadr:\n  - rust\n---\n\n# A\n"),
            ),
            // 型の違う adr と pdr は、ほかの YAML front matter の形の誤りと同じく slot-invalid にする
            (
                "docv/slot/service/service/b.md",
                &format!("---\norder: 2\nmission: {service}\nadr: rust\n---\n\n# B\n"),
            ),
            (
                "docv/slot/system/db.md",
                &format!(
                    "---\nmission: {}\npdr: [1]\n---\n\n# DB\n",
                    def("db").unwrap().mission
                ),
            ),
        ]);
        let result: Vec<String> = check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{}:{} {} {}", d.path, d.line, d.rule, d.message))
            .collect();
        // 数の要素も、文字列として読まずに誤りにする
        assert_eq!(
            result,
            [
                "docv/slot/service/service/b.md:4 slot-invalid adr は ADR の id（ファイル名から .md を除いたもの）の配列で書く",
                "docv/slot/system/db.md:3 slot-invalid pdr は PDR の id（ファイル名から .md を除いたもの）の配列で書く",
            ]
        );
        let slots = load(&project).unwrap();
        let related: Vec<(&str, &[String], &[String])> = slots
            .iter()
            .map(|s| (s.path.as_str(), &s.related.adr[..], &s.related.pdr[..]))
            .collect();
        assert_eq!(
            related,
            [
                (
                    "docv/slot/service/service/a.md",
                    &["rust".to_string()][..],
                    &[][..]
                ),
                (
                    "docv/slot/service/manual.md",
                    &["go".to_string()][..],
                    &["paid".to_string(), "free".to_string()][..]
                ),
                // 形の合わない YAML front matter でも、分けていない Slot は本文を出す。読める id だけを読む
                ("docv/slot/system/db.md", &[][..], &[][..]),
            ]
        );
    }

    #[test]
    fn splits_the_system_spec_like_the_service_spec() {
        let system = def("system").unwrap().mission;
        let (_dir, project) = project(&[
            (
                "docv/slot/system/system/web.md",
                &format!("---\norder: 1\nmission: {system}\n---\n\n# web\n"),
            ),
            (
                "docv/slot/system/system/batch.md",
                &format!("---\norder: 2\nmission: {system}\n---\n\n# batch\n"),
            ),
        ]);
        let diagnostics = check(&project).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let slots = load(&project).unwrap();
        let pages: Vec<(&str, Option<&str>)> = slots
            .iter()
            .map(|s| (s.def.id, s.page.as_ref().map(|p| p.title.as_str())))
            .collect();
        assert_eq!(pages, [("system", Some("web")), ("system", Some("batch"))]);
    }

    #[test]
    fn splits_the_infrastructure_like_the_service_spec() {
        let infra = def("infrastructure").unwrap().mission;
        let (_dir, project) = project(&[
            (
                "docv/slot/operation/infrastructure/aws.md",
                &format!("---\norder: 1\nmission: {infra}\n---\n\n# AWS\n"),
            ),
            (
                "docv/slot/operation/infrastructure/local.md",
                &format!("---\norder: 2\nmission: {infra}\n---\n\n# ローカル\n"),
            ),
        ]);
        let diagnostics = check(&project).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let slots = load(&project).unwrap();
        let pages: Vec<(&str, Option<&str>)> = slots
            .iter()
            .map(|s| (s.def.id, s.page.as_ref().map(|p| p.title.as_str())))
            .collect();
        assert_eq!(
            pages,
            [
                ("infrastructure", Some("AWS")),
                ("infrastructure", Some("ローカル"))
            ]
        );
    }

    #[test]
    fn splits_the_guides_for_developers_like_the_service_spec() {
        let mut files = Vec::new();
        for id in ["architecture", "design", "testing"] {
            let mission = def(id).unwrap().mission;
            for (order, name) in [(1, "web"), (2, "batch")] {
                files.push((
                    format!("docv/slot/guide/{id}/{name}.md"),
                    format!("---\norder: {order}\nmission: {mission}\n---\n\n# {name}\n"),
                ));
            }
        }
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let (_dir, project) = project(&files);
        let diagnostics = check(&project).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let pages: Vec<(&str, String)> = load(&project)
            .unwrap()
            .iter()
            .map(|s| (s.def.id, s.page.as_ref().unwrap().title.clone()))
            .collect();
        assert_eq!(
            pages,
            [
                ("architecture", "web".to_string()),
                ("architecture", "batch".to_string()),
                ("design", "web".to_string()),
                ("design", "batch".to_string()),
                ("testing", "web".to_string()),
                ("testing", "batch".to_string()),
            ]
        );
    }

    #[test]
    fn splits_the_develop_commands_like_the_service_spec() {
        let develop = def("develop").unwrap().mission;
        let (_dir, project) = project(&[
            (
                "docv/slot/guide/develop/web.md",
                &format!(
                    "---\norder: 2\nmission: {develop}\n---\n\n# Web\n\n## テストを実行する\n"
                ),
            ),
            (
                "docv/slot/guide/develop/api.md",
                &format!("---\norder: 1\nmission: {develop}\n---\n\n# API\n"),
            ),
        ]);
        let diagnostics = check(&project).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let slots = load(&project).unwrap();
        let pages: Vec<(&str, Option<&str>)> = slots
            .iter()
            .map(|s| (s.def.id, s.page.as_ref().map(|p| p.title.as_str())))
            .collect();
        assert_eq!(pages, [("develop", Some("API")), ("develop", Some("Web"))]);
        assert!(slots[1].body.contains("## テストを実行する"));
    }

    #[test]
    fn compares_order_and_h1_only_within_the_same_slot() {
        let system = def("system").unwrap().mission;
        let (_dir, project) = project(&[
            ("docv/slot/service/service/a.md", &page(1, "# 概要\n")),
            (
                "docv/slot/system/system/a.md",
                &format!("---\norder: 1\nmission: {system}\n---\n\n# 概要\n"),
            ),
        ]);
        let diagnostics = check(&project).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn loads_pages_of_a_split_slot_in_order() {
        let (_dir, project) = project(&[
            ("docv/slot/service/manual.md", "# マニュアル\n"),
            (
                "docv/slot/service/service/user.md",
                "---\norder: 2\n---\n\n# 利用者向け機能\n\n本文\n",
            ),
            (
                "docv/slot/service/service/admin.md",
                "---\norder: 1\n---\n\n# 管理者向け機能\n",
            ),
            // 読めないページは飛ばす（check が報告する）
            ("docv/slot/service/service/memo.md", "# メモ\n"),
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
        assert!(slots[1].body.starts_with("\n# 利用者向け機能"));
    }

    #[test]
    fn reads_the_whole_file_when_pages_also_exist() {
        let (_dir, project) = project(&[
            ("docv/slot/service/service.md", "# サービス仕様\n"),
            (
                "docv/slot/service/service/user.md",
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
                "docv/slot/service/service.md",
                &md("service", "# サービス仕様\n"),
            ),
            ("docv/slot/service/service/a.md", &page(1, "# A\n")),
            ("docv/slot/service/service/b.md", &page(1, "# A\n")),
            ("docv/slot/service/service/c.md", "# C\n"),
            (
                "docv/slot/service/service/d.md",
                "---\norder: 4\nname: D\n---\n\n# D\n",
            ),
            (
                "docv/slot/service/service/e.md",
                "---\norder: 5\n---\n\n本文\n",
            ),
            ("docv/slot/service/service/sub/f.md", "# F\n"),
            ("docv/slot/system/api/a.md", "# A\n"),
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
                "docv/slot/service/service.md:1 slot-invalid サービス仕様が docv/slot/service/service.md と docv/slot/service/service/ の両方にある。ページに分けるなら docv/slot/service/service.md を消す",
                "docv/slot/service/service/b.md:2 slot-invalid order 1 が docv/slot/service/service/a.md と重なっている",
                "docv/slot/service/service/b.md:6 slot-invalid h1「A」が docv/slot/service/service/a.md と重なっている。h1 はページの名前で、重ねない",
                "docv/slot/service/service/c.md:1 slot-invalid YAML front matter がない。ページに分けたサービス仕様には、どのページにも order と mission を書く",
                "docv/slot/service/service/d.md:3 slot-invalid YAML front matter がサービス仕様のページの形に合わない: unknown field `name`, expected one of `order`, `mission`, `adr`, `pdr` at line 2 column 1",
                "docv/slot/service/service/e.md:4 slot-invalid h1 がない。h1 がページの名前になる",
                &format!(
                    "docv/slot/service/service/sub/f.md:1 slot-unknown Slot の名前ではない。Slot のディレクトリに置けるのは {}",
                    file_names()
                ),
                &format!(
                    "docv/slot/system/api/a.md:1 slot-unknown Slot の名前ではない。Slot のディレクトリに置けるのは {}",
                    file_names()
                ),
            ]
        );
        // ページに分けられる Slot だけ、置ける名前の一覧にそのディレクトリを出す
        let names = file_names();
        assert!(names.contains("service/service.md、service/service/*.md、service/install.md"));
        assert!(names.contains("guide/develop.md、guide/develop/*.md、guide/architecture.md"));
        assert!(names.contains(
            "guide/architecture/*.md、guide/design.md、guide/design/*.md、guide/testing.md、guide/testing/*.md、guide/env.md、guide/doc-guide.md"
        ));
        assert!(!names.contains(".yaml"));
        assert!(names.contains("operation/infrastructure.md、operation/infrastructure/*.md"));
        assert!(!names.contains("api/*"));
    }
}
