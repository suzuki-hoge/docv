//! docv build。人間向けの HTML を生成する。docv/slot/service/service/html.md。
//!
//! トップは 8 ページに分け、サイドバーで行き来する。ルートに README.md があれば、README のページをサイドバーの先頭に足す。
//! index.html（Slot）に Slot ファイルのページへのカード、プロジェクト詳細.html（Free）に Free ファイルのツリー、
//! ADR.html と PDR.html に決定のつながりの図、インデックス.html（Index）にインデックス（README.md）と同じ並びのページへのリンク、
//! レビューガイド.html（Review Guide）に docv review-guide と同じレビューの手順、
//! 現在の設定.html（Config）に docv.yaml のすべての項目と今の値、検索.html（Search）に Slot ファイル、ADR、PDR、Free ファイルの中身を探す入力欄を置く。
//! ADR と PDR のページは、決定があるときだけ作る。
//! Slot は 1 つずつ独立したページにし、`<まとまり>/<Slot>.html` に置く。
//! ページに分けた Slot は、ページごとに `<まとまり>/<Slot>/<ページの名前>.html` に置き、ページへのリンクをタブにして並べる。
//! Slot、ADR、PDR、Free ファイルのページには、h1 の上に元のファイルのパスと、それをコピーするボタンを置く。表は、横にスクロールできる包みに入れる。
//! 同じページの中で見出しの行が同じ Markdown の表は、列の幅をそろえる。
//! Markdown の中の `<details>` は閉じたまま出し、印刷の表示では CSS で開く。
//! document_root の直下に、Slot ファイルへのリンクを並べたインデックス（README.md）も書く（index.rs）。
//! 左のサイドバーの一番下に、作った docv の版を出す。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, HeadingLevel, LinkType, Parser, Tag, TagEnd, TextMergeStream,
    html,
};
use serde::Serialize;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use unicode_width::UnicodeWidthChar;

use crate::config_view::{self, Cell};
use crate::decision::{self, Decision, Kind, Record};
use crate::diff_open::{Change, Changes, Status};
use crate::index;
use crate::markdown::{self, relative_path, slug};
use crate::meta;
use crate::review_guide;
use crate::slot::{self, DEFS, Group, Slot};
use crate::textlint;
use crate::workspace::{Class, Project};

/// すべてのページが読む CSS と JS。出力先の直下に書き、各ページから相対パスで読む。
/// ページに埋め込まないのは、出力先をコミットするリポジトリで、CSS と JS を変えたときの差分を 1 ファイルに収めるため。
const CSS: &str = include_str!("../assets/docv.css");
const JS: &str = include_str!("../assets/docv.js");
const CSS_FILE: &str = "docv.css";
const JS_FILE: &str = "docv.js";

/// docv が作った出力先であることを示すファイル。これがない出力先は消さない。
const MARKER: &str = ".docv";
const MARKER_TEXT: &str = "docv build の出力。このディレクトリは docv build のたびに作り直す。\n";

/// ルートの README.md と、その HTML のページ。README.md があるときだけ、サイドバーの先頭に出す。
const README_SOURCE: &str = "README.md";
const README_PAGE: &str = "README.html";

/// トップの 8 ページ。サイドバーの並びは Nav::SIDEBAR。
pub const PAGE: &str = "index.html";
const INDEX_PAGE: &str = "インデックス.html";
const ADR_PAGE: &str = "ADR.html";
const PDR_PAGE: &str = "PDR.html";
pub(crate) const DETAIL_PAGE: &str = "プロジェクト詳細.html";
const REVIEW_PAGE: &str = "レビューガイド.html";
const CONFIG_PAGE: &str = "現在の設定.html";
const SEARCH_PAGE: &str = "検索.html";

/// サイドバーの項目。描いているページがどれに属するかを示すのにも使う。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Nav {
    Readme,
    Overview,
    Adr,
    Pdr,
    Detail,
    Index,
    Review,
    Config,
    Search,
}

/// サイドバーの段。ドキュメント、docv が作るもの、検索に分け、間に横線を引く。
const SIDEBAR: [&[Nav]; 3] = [
    &[Nav::Readme, Nav::Overview, Nav::Detail, Nav::Adr, Nav::Pdr],
    &[Nav::Index, Nav::Review, Nav::Config],
    &[Nav::Search],
];

impl Nav {
    /// ページの h1。ADR と PDR は、サイドバーでは略し、h1 では略さずに書く。ほかはサイドバーと同じ。
    fn heading(self) -> &'static str {
        match self {
            Nav::Adr => "Architecture Decision Record",
            Nav::Pdr => "Product Decision Record",
            other => other.label(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Nav::Readme => "README",
            Nav::Overview => "Slot",
            Nav::Adr => "ADR",
            Nav::Pdr => "PDR",
            Nav::Detail => "Free",
            Nav::Index => "Index",
            Nav::Review => "Review Guide",
            Nav::Config => "Config",
            Nav::Search => "Search",
        }
    }

    /// 決定の記録の種類の、一覧のページ。
    fn of(kind: Kind) -> Nav {
        match kind {
            Kind::Adr => Nav::Adr,
            Kind::Pdr => Nav::Pdr,
        }
    }

    fn path(self) -> &'static str {
        match self {
            Nav::Readme => README_PAGE,
            Nav::Overview => PAGE,
            Nav::Adr => ADR_PAGE,
            Nav::Pdr => PDR_PAGE,
            Nav::Detail => DETAIL_PAGE,
            Nav::Index => INDEX_PAGE,
            Nav::Review => REVIEW_PAGE,
            Nav::Config => CONFIG_PAGE,
            Nav::Search => SEARCH_PAGE,
        }
    }
}

/// ADR か PDR の一覧のページの、出力先からの相対パス。`ADR.html`、`PDR.html`。
pub(crate) fn decision_index_page(kind: Kind) -> &'static str {
    Nav::of(kind).path()
}

/// Free ファイルのページを置くディレクトリ（出力先からの相対パス）。Free ファイルのパスの .md を .html にして、この下に置く。
const DETAIL_DIR: &str = "プロジェクト詳細";

/// 管理対象のファイル 1 つ。HTML ではツリーの 1 行になる。
struct Page {
    source: String,
    title: String,
    mission: Option<String>,
    tags: Vec<String>,
    /// YAML front matter を除いた本文。
    body: String,
}

/// 検索の索引の、ファイル 1 つ。
#[derive(Serialize)]
struct SearchEntry {
    path: String,
    /// ページへの、出力先からの相対パス。
    href: String,
    /// タグと、その色（CSS の `.tag.c0`〜`.tag.c7`）。
    tags: Vec<(String, usize)>,
    lines: Vec<String>,
}

/// docv build。out（dir.output の下の html/）の中を作り直す。
/// インデックスも書く。書けないインデックスなら、何も書かずに error で終わる。
pub fn run(project: &Project, out: &Path) -> Result<()> {
    let index = index::path(project);
    index::ensure_writable(&index)?;
    let out = prepare_out(out)?;
    render(project, &out, Some(&index), None)?;
    write(&out.join(CSS_FILE), CSS)?;
    write(&out.join(JS_FILE), JS)?;
    write(&out.join(MARKER), MARKER_TEXT)?;
    Ok(())
}

/// docv diff-open。build と同じページを out に書き、変わったファイルに + と − の印を付ける。
/// --from にあって作業ツリーにないファイルも、カードや行にして、消したことを見せる。インデックスは作らない。
pub fn run_diff(project: &Project, out: &Path, changes: &Changes) -> Result<()> {
    let out = prepare_out(out)?;
    // 途中で止まっても、次の diff-open で作り直せるように、先に目印を書く。
    write(&out.join(MARKER), MARKER_TEXT)?;
    render(project, &out, None, Some(changes))?;
    write(&out.join(CSS_FILE), CSS)?;
    write(&out.join(JS_FILE), JS)?;
    Ok(())
}

/// diff-open で、--from にあって作業ツリーにないもの。
#[derive(Default)]
struct Deleted {
    /// 消した Slot ファイル（Slot の id、パス）。
    slots: Vec<(&'static str, String)>,
    decisions: Vec<Decision>,
    /// 消した Free ファイル。中身は --from のもの。
    pages: Vec<Page>,
}

impl Deleted {
    /// 消したファイルを、Slot、ADR と PDR、Free ファイルに分ける。どれでもないファイルは捨てる。
    fn collect(project: &Project, changes: &Changes) -> Deleted {
        let mut deleted = Deleted::default();
        let slot_dir = format!("{}/", project.slot_dir());
        for (path, text) in changes.deleted() {
            if let Some(rel) = path.strip_prefix(&slot_dir) {
                if let Some(def) = slot::def_of_file(rel).or_else(|| slot::def_of_page(rel)) {
                    deleted.slots.push((def.id, path.to_string()));
                }
                continue;
            }
            let decision = Kind::ALL.into_iter().find_map(|kind| {
                let name = path.strip_prefix(&format!("{}/", decision::dir(project, kind)))?;
                let id = name.strip_suffix(".md").filter(|id| !id.contains('/'))?;
                Some((kind, id))
            });
            if let Some((kind, id)) = decision {
                // 今の docv で読めない決定も、カードは出す。題はファイル名にする。
                let record = decision::parse(kind, id, text).unwrap_or_else(|_| Record {
                    id: id.to_string(),
                    title: id.to_string(),
                    group: None,
                    date: String::new(),
                    supersedes: Vec::new(),
                    body: String::new(),
                });
                deleted.decisions.push(Decision {
                    kind,
                    path: path.to_string(),
                    record,
                    superseded_by: Vec::new(),
                });
                continue;
            }
            if path.ends_with(".md") && project.classify(path) == Some(Class::Managed) {
                deleted.pages.push(page_of_text(path, text));
            }
        }
        deleted
    }
}

/// out にすべてのページを書き、index があればインデックスを書く。
/// changes があれば（diff-open）、変わったファイルに印を付け、消したファイルのページも書く。
fn render(
    project: &Project,
    out: &Path,
    index: Option<&Path>,
    changes: Option<&Changes>,
) -> Result<()> {
    let mut pages = Vec::new();
    for path in project.managed()? {
        pages.push(read_page(project, &path)?);
    }
    // README.md は管理外のファイルなので、docv_exclude に当てはまらなければ読む。
    let readme = match project.classify(README_SOURCE) {
        Some(_) if project.root.join(README_SOURCE).is_file() => {
            Some(read_page(project, README_SOURCE)?)
        }
        _ => None,
    };
    let slots = slot::load(project)?;
    let adrs = decision::load(project, Kind::Adr)?;
    let pdrs = decision::load(project, Kind::Pdr)?;
    let deleted = changes
        .map(|c| Deleted::collect(project, c))
        .unwrap_or_default();

    let site = Site {
        project,
        out,
        dir: RefCell::new(out.to_path_buf()),
        readme: readme.as_ref(),
        pages: &pages,
        slots: &slots,
        adrs: &adrs,
        pdrs: &pdrs,
        changes,
        deleted: &deleted,
    };
    if let Some(readme) = &readme {
        write(&out.join(README_PAGE), &site.render_readme(readme))?;
    }
    write(&out.join(PAGE), &site.render_index())?;
    let kinds: Vec<Kind> = Kind::ALL
        .into_iter()
        .filter(|kind| site.has_decisions(*kind))
        .collect();
    for kind in &kinds {
        write(
            &out.join(Nav::of(*kind).path()),
            &site.render_decision_index(*kind),
        )?;
    }
    write(&out.join(DETAIL_PAGE), &site.render_detail())?;
    write(
        &out.join(INDEX_PAGE),
        &site.render_index_page(!pages.is_empty(), &kinds),
    )?;
    write(&out.join(REVIEW_PAGE), &site.render_review_guide())?;
    write(&out.join(CONFIG_PAGE), &site.render_config())?;
    write(&out.join(SEARCH_PAGE), &site.render_search()?)?;
    for slot in &slots {
        write(&out.join(page_path(slot)), &site.render_slot(slot))?;
    }
    for d in adrs.iter().chain(&pdrs) {
        write(&out.join(decision_page(d)), &site.render_decision(d))?;
    }
    for page in &pages {
        write(&out.join(doc_page(&page.source)), &site.render_doc(page))?;
    }
    for (id, path) in &deleted.slots {
        let label = DEFS.iter().find(|d| d.id == *id).map_or("", |d| d.label);
        write(
            &out.join(deleted_page(path)),
            &site.render_deleted(path, label, Nav::Overview),
        )?;
    }
    for d in &deleted.decisions {
        write(
            &out.join(deleted_page(&d.path)),
            &site.render_deleted(&d.path, &d.record.title, Nav::of(d.kind)),
        )?;
    }
    for page in &deleted.pages {
        write(
            &out.join(deleted_page(&page.source)),
            &site.render_deleted(&page.source, &page.title, Nav::Detail),
        )?;
    }
    if let Some(index) = index {
        let text = index::render(project, index, &slots, !pages.is_empty(), &kinds);
        write(index, &text)?;
    }
    Ok(())
}

/// Slot のページの、出力先からの相対パス。`サービス/利用マニュアル.html`。名前の空白は除く（`開発者向けガイド/Docガイド.html`）。
/// 分けた Slot のページは、Slot の名前のディレクトリの下に、ページの名前で置く。`サービス/サービス仕様/管理者向け機能.html`。
fn page_path(slot: &Slot) -> String {
    match &slot.page {
        Some(page) => format!(
            "{}/{}/{}.html",
            file_name(slot.def.group.label()),
            file_name(slot.def.label),
            file_name(&page.title)
        ),
        None => format!(
            "{}/{}.html",
            file_name(slot.def.group.label()),
            file_name(slot.def.label)
        ),
    }
}

fn read_page(project: &Project, path: &str) -> Result<Page> {
    let text = fs::read_to_string(project.root.join(path))
        .with_context(|| format!("{path} を読めない"))?;
    Ok(page_of_text(path, &text))
}

fn page_of_text(path: &str, text: &str) -> Page {
    let document = markdown::split(text);
    let meta = meta::read(&document, &markdown::outline(&document));
    Page {
        source: path.to_string(),
        title: meta.title.unwrap_or_else(|| path.to_string()),
        mission: meta.mission,
        tags: meta.tags,
        body: document.body.clone(),
    }
}

/// diff-open で、消したファイルのページを置くディレクトリ（出力先からの相対パス）。
const DELETED_DIR: &str = "削除";

/// 消したファイルのページの、出力先からの相対パス。`削除/docv/adr/go.html`。今のページと重ならないように、分けて置く。
fn deleted_page(source: &str) -> String {
    let stem = source.rsplit_once('.').map_or(source, |(stem, _)| stem);
    format!("{DELETED_DIR}/{}.html", no_space(stem))
}

/// Free ファイルのページの、出力先からの相対パス。`プロジェクト詳細/docv/free/order.html`。
/// ディレクトリとファイルの名前の空白は除く（`docv/free/my notes/a b.md` は `プロジェクト詳細/docv/free/mynotes/ab.html`）。
fn doc_page(source: &str) -> String {
    format!(
        "{DETAIL_DIR}/{}.html",
        no_space(source.strip_suffix(".md").unwrap_or(source))
    )
}

/// 出力先を用意する。目印を除いて中を消す。目印を残すので、途中で止まっても、次の build で作り直せる。
/// docv が作った目印のない、空でないディレクトリには書かずにエラーにする。
fn prepare_out(out: &Path) -> Result<PathBuf> {
    if out.exists() {
        let is_empty = fs::read_dir(out)?.next().is_none();
        if !is_empty && !out.join(MARKER).is_file() {
            bail!(
                "出力先 {} は docv が作ったディレクトリではないので書けない",
                out.display()
            );
        }
        for entry in fs::read_dir(out)? {
            let path = entry?.path();
            if path.file_name().is_some_and(|n| n == MARKER) {
                continue;
            }
            let removed = if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
            removed.with_context(|| format!("{} を消せない", path.display()))?;
        }
    }
    fs::create_dir_all(out).with_context(|| format!("{} を作れない", out.display()))?;
    Ok(out.canonicalize()?)
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text).with_context(|| format!("{} を書けない", path.display()))
}

struct Site<'a> {
    project: &'a Project,
    out: &'a Path,
    /// 描いているページのディレクトリ。リンクの起点。
    dir: RefCell<PathBuf>,
    /// ルートの README.md。ないか、docv_exclude に当てはまれば None。
    readme: Option<&'a Page>,
    pages: &'a [Page],
    slots: &'a [Slot],
    adrs: &'a [Decision],
    pdrs: &'a [Decision],
    /// diff-open のときだけ、--from と作業ツリーの差分。
    changes: Option<&'a Changes>,
    /// diff-open で、--from にあって作業ツリーにないもの。build では空。
    deleted: &'a Deleted,
}

impl Site<'_> {
    /// 1 つの種類の決定。日付と id の順。
    fn decisions(&self, kind: Kind) -> &[Decision] {
        match kind {
            Kind::Adr => self.adrs,
            Kind::Pdr => self.pdrs,
        }
    }

    /// 一覧のページを作る決定があるか。diff-open では、消した決定だけでも作る。
    fn has_decisions(&self, kind: Kind) -> bool {
        !self.decisions(kind).is_empty() || self.deleted.decisions.iter().any(|d| d.kind == kind)
    }

    // ---- diff-open の印 ----

    /// ファイルの差分。build では None。
    fn change(&self, path: &str) -> Option<Change> {
        self.changes.and_then(|c| c.change(path))
    }

    /// ファイルをまとめた印。すべて新しく足したファイルなら新規、すべて消したファイルなら削除、ほかに変わったファイルがあれば変更。
    /// 変わっていないファイルも渡す。今あるページに足しただけの Slot を、新規でなく変更にするため。
    fn look<'p>(&self, paths: impl IntoIterator<Item = &'p str>) -> Look {
        let changes: Vec<Option<Change>> = paths.into_iter().map(|p| self.change(p)).collect();
        let all = |status| {
            changes
                .iter()
                .all(|c| c.is_some_and(|c| c.status == status))
        };
        if changes.iter().all(Option::is_none) {
            Look::Plain
        } else if all(Status::Added) {
            Look::Added
        } else if all(Status::Deleted) {
            Look::Deleted
        } else {
            Look::Edited(lines_tip(changes.iter().flatten()))
        }
    }

    /// サイドバーの項目に属するファイル。消したファイルも含める。
    fn nav_paths(&self, nav: Nav) -> Vec<String> {
        match nav {
            Nav::Readme => vec![README_SOURCE.to_string()],
            Nav::Overview => self
                .slots
                .iter()
                .map(|s| s.path.clone())
                .chain(self.deleted.slots.iter().map(|(_, p)| p.clone()))
                .collect(),
            Nav::Detail => self
                .pages
                .iter()
                .chain(&self.deleted.pages)
                .map(|p| p.source.clone())
                .collect(),
            Nav::Adr | Nav::Pdr => {
                let kind = if nav == Nav::Adr {
                    Kind::Adr
                } else {
                    Kind::Pdr
                };
                self.decisions(kind)
                    .iter()
                    .chain(self.deleted.decisions.iter().filter(|d| d.kind == kind))
                    .map(|d| d.path.clone())
                    .collect()
            }
            Nav::Config => self.config_source().into_iter().collect(),
            Nav::Index | Nav::Review | Nav::Search => Vec::new(),
        }
    }

    /// サイドバーの項目の鉛筆。中のファイルが 1 つでも変わっていれば出し、マウスを乗せると変わったファイルの数を出す。
    /// README と Config は 1 つのファイルで、数がいつも 1 になるので、代わりに行数を出す。2 つにはカードがなく、ほかに行数を見る場所がないため。
    fn nav_pencil(&self, nav: Nav) -> String {
        let changes: Vec<Change> = self
            .nav_paths(nav)
            .iter()
            .filter_map(|p| self.change(p))
            .collect();
        if changes.is_empty() {
            return String::new();
        }
        if matches!(nav, Nav::Readme | Nav::Config) {
            return pencil_html(&lines_tip(&changes));
        }
        let count = |status| changes.iter().filter(|c| c.status == status).count();
        pencil_html(&format!(
            "差分ファイル数\n新規: {}\n変更: {}\n削除: {}",
            count(Status::Added),
            count(Status::Modified),
            count(Status::Deleted)
        ))
    }

    /// 元のファイルのパスの行。diff-open で変わったファイルは、パスの代わりに git diff のコマンドを出し、ボタンでコマンドをコピーする。
    fn source(&self, path: &str, mission: Option<&str>) -> String {
        match self.changes.and_then(|c| c.command(path)) {
            Some(command) => source_line(&command, mission, "コマンドをコピーする"),
            None => source(path, mission),
        }
    }

    /// README と Config のページの、git diff のコマンドの行。2 つは通常の build ではパスの行を出さないので、diff-open で変わったときだけ出す。
    fn command_line(&self, path: &str) -> String {
        self.changes
            .and_then(|c| c.command(path))
            .map(|command| source_line(&command, None, "コマンドをコピーする"))
            .unwrap_or_default()
    }

    /// docv.yaml のルートからの相対パス。ルートの外に置いていれば None。
    fn config_source(&self) -> Option<String> {
        self.project.config_source()
    }

    /// diff-open で、消したファイルのページ。元のファイルのパスの行に git diff のコマンドを出す。中身は出さない。
    fn render_deleted(&self, path: &str, title: &str, nav: Nav) -> String {
        let page = deleted_page(path);
        let dir = Path::new(&page).parent().unwrap_or(Path::new(""));
        *self.dir.borrow_mut() = self.out.join(dir);
        let from = self.changes.map_or("", |c| c.from.as_str());
        let body = format!(
            "{}<section class=\"doc\">\n<h1>{}</h1>\n<p class=\"status\">{} にあり、作業ツリーにはない。</p>\n</section>\n",
            self.source(path, None),
            escape(title),
            escape(from)
        );
        self.page(
            &format!(
                "{} - {} - {}",
                escape(title),
                nav.label(),
                escape(&self.project.title())
            ),
            nav,
            &body,
        )
    }

    /// 消したファイルのページへのリンク。描いているページのディレクトリからの相対パス。
    fn deleted_href(&self, path: &str) -> String {
        encode_path(&relative_path(
            &self.dir.borrow(),
            &self.out.join(deleted_page(path)),
        ))
    }

    /// README.html。ルートの README.md を Markdown のまま HTML にする。
    fn render_readme(&self, readme: &Page) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let html = self.markdown_with_ids(&readme.body, Path::new(""), true, 2);
        let body = format!(
            "{}<section class=\"doc\">\n<h1>{}</h1>\n{html}</section>\n",
            self.command_line(README_SOURCE),
            escape(&readme.title)
        );
        self.page(
            &format!(
                "{} - {}",
                Nav::Readme.label(),
                escape(&self.project.title())
            ),
            Nav::Readme,
            &body,
        )
    }

    /// index.html（Slot）。Slot のページへのカード。
    fn render_index(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = String::new();
        self.index(&mut out);
        self.page(&escape(&self.project.title()), Nav::Overview, &out)
    }

    /// インデックス.html。インデックス（README.md）と同じ並びで、md ではなく HTML のページへリンクする。
    /// Free ファイル、ADR、PDR は、それぞれの一覧のページへリンクする。
    fn render_index_page(&self, has_free: bool, kinds: &[Kind]) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = format!("<h1>{}</h1>\n", Nav::Index.heading());
        let link = |label: &str, page: &str| {
            format!(
                "<a href=\"{}\">{}</a>",
                escape(&encode_path(page)),
                escape(label)
            )
        };
        for (group, items) in index::outline(self.slots) {
            let _ = writeln!(out, "<h2>{}</h2>\n<ul>", group.label());
            for item in items {
                match item {
                    index::Item::Single(label, slot) => {
                        let _ = writeln!(out, "<li>{}</li>", link(label, &page_path(slot)));
                    }
                    index::Item::Split(label, pages) => {
                        let _ = writeln!(out, "<li>{}\n<ul>", escape(label));
                        for (title, slot) in pages {
                            let _ = writeln!(out, "<li>{}</li>", link(title, &page_path(slot)));
                        }
                        let _ = writeln!(out, "</ul>\n</li>");
                    }
                }
            }
            let _ = writeln!(out, "</ul>");
        }
        let mut lists: Vec<(&str, Nav)> = Vec::new();
        if has_free {
            lists.push(("Free ファイル", Nav::Detail));
        }
        for kind in kinds {
            lists.push((kind.label(), Nav::of(*kind)));
        }
        for (heading, nav) in lists {
            let _ = writeln!(
                out,
                "<h2>{heading}</h2>\n<ul>\n<li>{}</li>\n</ul>",
                link(nav.label(), nav.path())
            );
        }
        // インデックス（README.md）と同じく、末尾に作った docv の版を出す。
        let _ = writeln!(out, "<hr>\n<p class=\"note\">{}</p>", generated_by());
        self.page(
            &format!("{} - {}", Nav::Index.label(), escape(&self.project.title())),
            Nav::Index,
            &out,
        )
    }

    /// ADR.html と PDR.html。決定のつながりの図。
    fn render_decision_index(&self, kind: Kind) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = String::new();
        self.decision_lanes(&mut out, kind);
        self.page(
            &format!("{} - {}", kind.label(), escape(&self.project.title())),
            Nav::of(kind),
            &out,
        )
    }

    /// プロジェクト詳細.html（Free）。Free ファイルのツリー。
    fn render_detail(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = String::new();
        self.docs(&mut out);
        self.page(
            &format!(
                "{} - {}",
                Nav::Detail.label(),
                escape(&self.project.title())
            ),
            Nav::Detail,
            &out,
        )
    }

    /// レビューガイド.html。docv review-guide と同じファイルから作った、レビューの手順を出す。人間と AI Agent が同じものを読むため。
    /// 一覧は出さない（review_guide::page）。手順の h1 は、ページの h1（Review Guide）と重なるので除く。
    /// h1 の下に、docv review-guide が出す手順と同じであることを書く。
    fn render_review_guide(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let text = review_guide::page();
        let html = self.markdown_with_ids(&text, Path::new(""), true, 2);
        let body = format!(
            "<section class=\"doc\">\n<h1>{}</h1>\n<p class=\"note\"><code>docv review-guide</code> が出す手順と同じ内容。</p>\n{html}</section>\n",
            Nav::Review.heading()
        );
        self.page(
            &format!(
                "{} - {}",
                Nav::Review.label(),
                escape(&self.project.title())
            ),
            Nav::Review,
            &body,
        )
    }

    /// 現在の設定.html。docv.yaml のすべての項目と今の値を、1 つの表に並べる。
    fn render_config(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = self
            .config_source()
            .map(|rel| self.command_line(&rel))
            .unwrap_or_default();
        self.config(&mut out);
        self.page(
            &format!(
                "{} - {}",
                Nav::Config.label(),
                escape(&self.project.title())
            ),
            Nav::Config,
            &out,
        )
    }

    /// 検索.html。入力欄と、探す先の索引。索引は、Slot ファイル、ADR、PDR、Free ファイルの順に、中身を行ごとに持つ。
    /// 入力欄は JS が動くときだけ出し、結果は JS が入力欄の下に並べる。
    fn render_search(&self) -> Result<String> {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut entries = Vec::new();
        for s in self.slots {
            entries.push(self.search_entry(&s.path, &[])?);
        }
        for d in self.adrs.iter().chain(self.pdrs) {
            entries.push(self.search_entry(&d.path, &[])?);
        }
        for p in self.pages {
            entries.push(self.search_entry(&p.source, &p.tags)?);
        }
        // <script> の中に置くので、</script> などで閉じないように < を書き換える。
        let index = serde_json::to_string(&entries)?.replace('<', "\\u003c");
        let body = format!(
            "<h1>{}</h1>\n<form class=\"search\" role=\"search\" hidden><input type=\"search\" placeholder=\"入力した文字列をそのまま含む行を探す\" aria-label=\"検索\" autofocus></form>\n<noscript><p class=\"note\">検索には JavaScript が要る。</p></noscript>\n<p class=\"note\" id=\"search-count\"></p>\n<div id=\"search-results\"></div>\n<script>var DOCV_SEARCH={index};</script>\n",
            Nav::Search.label(),
        );
        Ok(self.page(
            &format!(
                "{} - {}",
                Nav::Search.label(),
                escape(&self.project.title())
            ),
            Nav::Search,
            &body,
        ))
    }

    /// 検索の索引の、ファイル 1 つ。path はルートからの相対パス。
    fn search_entry(&self, path: &str, tags: &[String]) -> Result<SearchEntry> {
        let text = fs::read_to_string(self.project.root.join(path))
            .with_context(|| format!("{path} を読めない"))?;
        Ok(SearchEntry {
            path: path.to_string(),
            href: self.link(path, "").unwrap_or_default(),
            tags: tags
                .iter()
                .map(|t| (t.clone(), self.tag_color(t)))
                .collect(),
            lines: text.lines().map(str::to_string).collect(),
        })
    }

    /// docv.yaml の項目の表と、docv が使う textlint のルールの表。
    /// 配列の項目は箇条書きにし、選ぶ項目は候補を横に並べて、選んでいるものに色を付ける。値のない項目は `-` を出す。
    fn config(&self, out: &mut String) {
        // ページの名前の h1 と重ならないように、項目の表の見出しは docv.yaml にする。
        let _ = writeln!(
            out,
            "<h1>{}</h1>\n<h2 class=\"group\">docv.yaml</h2>",
            Nav::Config.label()
        );
        let _ = writeln!(
            out,
            "<table class=\"config\">\n<thead><tr><th>項目</th><th>値</th></tr></thead>\n<tbody>"
        );
        for row in config_view::rows(&self.project.config, &self.project.title()) {
            let _ = write!(out, "<tr><td><code>{}</code></td><td>", escape(row.key));
            match &row.value {
                Cell::Value(text) => {
                    let _ = write!(out, "<code>{}</code>", escape(text));
                }
                Cell::Tags(tags) if tags.is_empty() => out.push('-'),
                Cell::Tags(tags) => {
                    for tag in tags {
                        out.push_str(&self.tag(tag));
                    }
                }
                Cell::List(items) if items.is_empty() => out.push('-'),
                Cell::List(items) => {
                    out.push_str("<ul>");
                    for item in items {
                        let _ = write!(out, "<li><code>{}</code></li>", escape(item));
                    }
                    out.push_str("</ul>");
                }
                Cell::Choice(items) => {
                    out.push_str("<div class=\"choices\">");
                    for (text, selected) in items {
                        let _ = write!(
                            out,
                            "<span class=\"choice{}\">{}</span>",
                            if *selected { " on" } else { "" },
                            escape(text)
                        );
                    }
                    out.push_str("</div>");
                }
            }
            out.push_str("</td></tr>\n");
        }
        out.push_str("</tbody>\n</table>\n");

        let _ = writeln!(
            out,
            "<h2 class=\"group\">textlint のルール</h2>\n<p class=\"note\">lint.textlint が enable のときに、docv check が textlint で検査するルール。個別の設定は不可。</p>"
        );
        for (title, enabled, column) in [
            ("検査項目", true, "確かめること"),
            ("非検査項目", false, "確かめないこと"),
        ] {
            let _ = writeln!(out, "<h3>{title}</h3>");
            for preset in textlint::PRESETS {
                let _ = writeln!(
                    out,
                    "<h4>{preset}</h4>\n<table class=\"textlint\">\n<thead><tr><th>ルール</th><th>{column}</th></tr></thead>\n<tbody>"
                );
                for rule in textlint::RULES
                    .iter()
                    .filter(|r| r.preset == *preset && r.enabled() == enabled)
                {
                    let _ = writeln!(
                        out,
                        "<tr><td><code>{}</code></td><td>{}</td></tr>",
                        rule.name,
                        escape(rule.desc)
                    );
                }
                out.push_str("</tbody>\n</table>\n");
            }
        }
    }

    /// Free ファイル 1 つのページ。Free ファイルを Markdown のまま HTML にする。
    fn render_doc(&self, page: &Page) -> String {
        let path = doc_page(&page.source);
        let dir = Path::new(&path).parent().unwrap_or(Path::new(""));
        *self.dir.borrow_mut() = self.out.join(dir);
        let base = Path::new(&page.source)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let html = self.markdown_with_ids(&page.body, &base, true, 2);
        let body = format!(
            "{}<section class=\"doc\">\n<h1>{}</h1>\n{html}</section>\n",
            self.source(&page.source, None),
            escape(&page.title)
        );
        self.page(
            &format!(
                "{} - {} - {}",
                escape(&page.title),
                Nav::Detail.label(),
                escape(&self.project.title())
            ),
            Nav::Detail,
            &body,
        )
    }

    /// HTML の全体。左に README とトップの 8 ページへのサイドバーを、右に h2 と h3 の目次を置く。印刷の表示では、どちらも出さない。
    /// 本文の Markdown の表は、見出しの行が同じものどうしで列の幅をそろえる（align_tables）。
    /// サイドバーは SIDEBAR の段ごとに並べ、段の間に横線を引く。
    /// README、ADR、PDR は、そのページがあるときだけ出す。サイドバーの一番下に、作った docv の版を出す。
    fn page(&self, title: &str, current: Nav, body: &str) -> String {
        let mut nav = format!(
            "<nav class=\"sidebar\">\n<p class=\"project\">{}</p>\n",
            escape(&self.project.title())
        );
        // diff-open では、何と比べたページかを、プロジェクトの名前の下に出す。
        if let Some(changes) = self.changes {
            let _ = writeln!(
                nav,
                "<p class=\"diff-from\"><code>{}</code> との差分</p>",
                escape(&changes.from)
            );
        }
        nav.push_str("<ul>\n");
        for (i, item) in SIDEBAR
            .iter()
            .enumerate()
            .flat_map(|(i, items)| items.iter().map(move |item| (i, *item)))
        {
            // 段の先頭の項目の前に、横線を引く。
            if i > 0 && SIDEBAR[i].first() == Some(&item) {
                nav.push_str("<li class=\"separator\" role=\"separator\"></li>\n");
            }
            if (item == Nav::Readme && self.readme.is_none())
                || (item == Nav::Adr && !self.has_decisions(Kind::Adr))
                || (item == Nav::Pdr && !self.has_decisions(Kind::Pdr))
            {
                continue;
            }
            let href = encode_path(&relative_path(
                &self.dir.borrow(),
                &self.out.join(item.path()),
            ));
            let _ = writeln!(
                nav,
                "<li><a href=\"{href}\"{}>{}{}</a></li>",
                if item == current {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                item.label(),
                self.nav_pencil(item)
            );
        }
        let _ = write!(
            nav,
            "</ul>\n<p class=\"generated\">{}</p>\n</nav>\n",
            generated_by()
        );
        let (body, aside) = outline(&wrap_tables(&align_tables(body)));
        let root = |name: &str| relative_path(&self.dir.borrow(), &self.out.join(name));
        page(title, &root(CSS_FILE), &root(JS_FILE), &nav, &body, &aside)
    }

    /// Slot のページ。分けた Slot のページの題には、ページの名前を前に付ける。
    fn render_slot(&self, slot: &Slot) -> String {
        let path = page_path(slot);
        let dir = Path::new(&path).parent().unwrap_or(Path::new(""));
        *self.dir.borrow_mut() = self.out.join(dir);
        let mut out = self.source(&slot.path, Some(slot.def.mission));
        self.slot(&mut out, slot);
        let title = match &slot.page {
            Some(page) => format!("{} - {}", escape(&page.title), slot.def.label),
            None => slot.def.label.to_string(),
        };
        self.page(
            &format!("{title} - {}", escape(&self.project.title())),
            Nav::Overview,
            &out,
        )
    }

    /// Slot のページへのカード。まとまりごとに、すべての Slot をカードで横に並べる。
    /// ファイルのない Slot は、リンクにせずグレーで出し、「未作成」と書く。
    /// ページに分けられる Slot は、「未作成」と同じ位置に、ページの数を「3 tabs」の形で書く。分けていなければ「1 tab」。
    fn index(&self, out: &mut String) {
        let _ = writeln!(out, "<h1>{}</h1>", Nav::Overview.label());
        if self.slots.is_empty() {
            let _ = writeln!(
                out,
                "<p class=\"note\">Slot ファイルがない。{} に Slot ファイルを置くと、カードから開ける。</p>",
                escape(&self.project.slot_dir())
            );
        }
        for group in Group::ALL {
            let _ = writeln!(
                out,
                "<h2 class=\"group\">{}</h2>\n<div class=\"cards\">",
                group.label()
            );
            for def in DEFS.iter().filter(|d| d.group == group) {
                let gone: Vec<&str> = self
                    .deleted
                    .slots
                    .iter()
                    .filter(|(id, _)| *id == def.id)
                    .map(|(_, p)| p.as_str())
                    .collect();
                let look = self.look(
                    self.slots
                        .iter()
                        .filter(|s| s.def.id == def.id)
                        .map(|s| s.path.as_str())
                        .chain(gone.iter().copied()),
                );
                let (class, pencil) = (look.class(), look.pencil());
                let Some(slot) = self.slots.iter().find(|s| s.def.id == def.id) else {
                    // diff-open で消した Slot のカードは、消したファイルのページへリンクする。
                    match gone.first() {
                        Some(path) => {
                            let _ = writeln!(
                                out,
                                "<a class=\"card missing{class}\" href=\"{}\">{}{}{pencil}<span class=\"state\">未作成</span></a>",
                                self.deleted_href(path),
                                icon(def.id),
                                def.label
                            );
                        }
                        None => {
                            let _ = writeln!(
                                out,
                                "<span class=\"card missing\">{}{}<span class=\"state\">未作成</span></span>",
                                icon(def.id),
                                def.label
                            );
                        }
                    }
                    continue;
                };
                let href = encode_path(&page_path(slot));
                let tabs = match self.slots.iter().filter(|s| s.def.id == def.id).count() {
                    _ if def.pages_dir().is_none() => String::new(),
                    1 => "<span class=\"state\">1 tab</span>".to_string(),
                    n => format!("<span class=\"state\">{n} tabs</span>"),
                };
                let _ = writeln!(
                    out,
                    "<a class=\"card{class}\" href=\"{href}\">{}{}{pencil}{tabs}</a>",
                    icon(def.id),
                    def.label
                );
            }
            let _ = writeln!(out, "</div>");
        }
    }

    /// ADR と PDR の図。まとまりごとに h2 を置き、決定のつながりを過去から未来へ、左から右に箱と矢印で並べる。
    /// つながりのない決定は、1 行に 1 つずつ並べず、つながりの後ろに箱を折り返して並べる。箱は、その決定のページへのリンク。
    fn decision_lanes(&self, out: &mut String, kind: Kind) {
        let decisions = self.decisions(kind);
        let _ = writeln!(out, "<h1>{}</h1>", Nav::of(kind).heading());
        // diff-open で消した決定は、まとまりのつながりの後ろに、箱を並べる。
        let deleted: Vec<&Decision> = self
            .deleted
            .decisions
            .iter()
            .filter(|d| d.kind == kind)
            .collect();
        let mut groups = decision::groups(decisions);
        for d in &deleted {
            if !groups.contains(&d.group()) {
                groups.push(d.group());
            }
        }
        for group in groups {
            let members: Vec<&Decision> = decisions.iter().filter(|a| a.group() == group).collect();
            let _ = writeln!(out, "<h2 class=\"group\">{}</h2>", escape(group));
            let (chains, singles): (Vec<_>, Vec<_>) = decision::lanes(&members)
                .into_iter()
                .partition(|l| l.len() > 1);
            if !chains.is_empty() {
                let _ = writeln!(out, "<div class=\"lanes\">");
                for lane in chains {
                    let boxes: Vec<String> = lane.iter().map(|a| self.decision_box(a)).collect();
                    let _ = writeln!(
                        out,
                        "<div class=\"lane\">{}</div>",
                        boxes.join("<span class=\"arrow\">→</span>")
                    );
                }
                let _ = writeln!(out, "</div>");
            }
            let gone: String = deleted
                .iter()
                .filter(|d| d.group() == group)
                .map(|d| self.deleted_box(d))
                .collect();
            if !singles.is_empty() || !gone.is_empty() {
                let boxes: String = singles.iter().map(|l| self.decision_box(l[0])).collect();
                let _ = writeln!(out, "<div class=\"decision-grid\">{boxes}{gone}</div>");
            }
        }
    }

    /// 決定 1 つのページ。
    fn render_decision(&self, a: &Decision) -> String {
        *self.dir.borrow_mut() = self.out.join(a.kind.label()).join(file_name(a.group()));
        let mut out = self.source(&a.path, None);
        let _ = writeln!(out, "<section class=\"slot\">");
        self.decision_detail(&mut out, a);
        let _ = writeln!(out, "</section>");
        self.page(
            &format!(
                "{} - {} - {}",
                escape(&a.record.title),
                a.kind.label(),
                escape(&self.project.title())
            ),
            Nav::of(a.kind),
            &out,
        )
    }

    /// 決定のページへのリンク。描いているページのディレクトリからの相対パス。
    fn decision_href(&self, a: &Decision) -> String {
        let to = self.out.join(decision_page(a));
        encode_path(&relative_path(&self.dir.borrow(), &to))
    }

    fn decision_box(&self, a: &Decision) -> String {
        let look = self.look([a.path.as_str()]);
        format!(
            "<a class=\"decision-box{}{}\" href=\"{}\">{}{}</a>",
            if a.superseded_by.is_empty() {
                ""
            } else {
                " superseded"
            },
            look.class(),
            self.decision_href(a),
            escape(&a.record.title),
            look.pencil()
        )
    }

    /// diff-open で消した決定の箱。消したファイルのページへリンクする。
    fn deleted_box(&self, a: &Decision) -> String {
        format!(
            "<a class=\"decision-box{}\" href=\"{}\">{}</a>",
            self.look([a.path.as_str()]).class(),
            self.deleted_href(&a.path),
            escape(&a.record.title)
        )
    }

    fn decision_detail(&self, out: &mut String, a: &Decision) {
        let r = &a.record;
        let decisions = self.decisions(a.kind);
        let _ = writeln!(out, "<h1>{}{}</h1>", icon(a.kind.dir()), escape(&r.title));
        // 打ち消された決定は、題の下に、打ち消されたことと新しい決定へのリンクを出す。下のカードまで読まなくても分かるように。
        let newer: Vec<String> = a
            .superseded_by
            .iter()
            .filter_map(|id| decisions.iter().find(|x| &x.record.id == id))
            .map(|x| {
                format!(
                    "<a href=\"{}\">{}</a>",
                    self.decision_href(x),
                    escape(&x.record.title)
                )
            })
            .collect();
        if !newer.is_empty() {
            let _ = writeln!(
                out,
                "<p class=\"status\">この決定は打ち消された。新しい決定は {}</p>",
                newer.join("、")
            );
        }
        // 本文は、h1（題）を除き、いちばん浅い見出しを h2 にして出す。
        let base = Path::new(&a.path)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let _ = writeln!(out, "{}", self.markdown(&r.body, &base, true, 2));
        // ページの下に、打ち消した過去の決定を左、打ち消された先の新しい決定を右に、見出しを付けたカードで出す。
        // カーソルキーの ← と → で、それぞれの先頭のカードのページへ移る。移れることも書いておく。
        let boxes = |ids: &[String]| -> String {
            ids.iter()
                .filter_map(|id| decisions.iter().find(|x| &x.record.id == id))
                .map(|x| self.decision_box(x))
                .collect()
        };
        let (prev, next) = (boxes(&r.supersedes), boxes(&a.superseded_by));
        if prev.is_empty() && next.is_empty() {
            return;
        }
        let side = |class: &str, label: &str, html: &str| -> String {
            if html.is_empty() {
                return format!("<div class=\"{class}\"></div>");
            }
            format!(
                "<div class=\"{class}\"><p class=\"decision-nav-label\">{label}</p><div class=\"boxes\">{html}</div></div>"
            )
        };
        let _ = writeln!(
            out,
            "<nav class=\"decision-nav\">{}{}</nav>\n<p class=\"note\">カーソルキーの ← と → でも移れる。</p>",
            side("prev", "← 過去の決定", &prev),
            side("next", "新しい決定 →", &next)
        );
    }

    fn slot(&self, out: &mut String, slot: &Slot) {
        let _ = writeln!(
            out,
            "<section class=\"slot\" id=\"{}\">\n<h1>{}{}</h1>",
            slot.def.id,
            icon(slot.def.id),
            slot.def.label
        );
        self.page_tabs(out, slot);
        out.push_str(&self.slot_body(slot));
        let _ = writeln!(out, "</section>");
    }

    /// 分けた Slot のページのタブ。同じ Slot のすべてのページへのリンクを order の順に並べ、描いているページに印を付ける。
    /// 分けていない Slot には出さない。
    fn page_tabs(&self, out: &mut String, slot: &Slot) {
        if slot.page.is_none() {
            return;
        }
        let _ = write!(out, "<nav class=\"page-tabs\">");
        for s in self.slots.iter().filter(|s| s.def.id == slot.def.id) {
            let Some(page) = &s.page else { continue };
            let look = self.look([s.path.as_str()]);
            let _ = write!(
                out,
                "<a{} href=\"{}\"{}>{}{}</a>",
                class_attr(look.class()),
                self.link(&s.path, "").unwrap_or_default(),
                if s.path == slot.path {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                escape(&page.title),
                look.pencil()
            );
        }
        // diff-open で消したページは、ファイルの名前のタブにして、消したファイルのページへリンクする。
        for (_, path) in self
            .deleted
            .slots
            .iter()
            .filter(|(id, _)| *id == slot.def.id)
        {
            let _ = write!(
                out,
                "<a{} href=\"{}\">{}</a>",
                class_attr(self.look([path.as_str()]).class()),
                self.deleted_href(path),
                escape(path.rsplit('/').next().unwrap_or(path))
            );
        }
        let _ = writeln!(out, "</nav>");
    }

    /// Slot の本文。h1 を除き、見出しは h2 から始まるように段を付け直す。リンクは、ファイルの場所からの相対パス。
    /// Slot はページ全体なので、Free ファイルと同じく見出しに id を付け、ページ内リンクで飛べるようにする。
    fn slot_body(&self, slot: &Slot) -> String {
        let base = Path::new(&slot.path)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        format!("{}\n", self.markdown_with_ids(&slot.body, &base, true, 2))
    }

    // ---- Free ファイルのツリー ----

    fn docs(&self, out: &mut String) {
        let _ = writeln!(
            out,
            "<h1>{}</h1>\n<h2 class=\"group\">ドキュメント</h2>",
            Nav::Detail.label()
        );
        if self.pages.is_empty() && self.deleted.pages.is_empty() {
            let _ = writeln!(out, "<p class=\"note\">管理対象のファイルがない。</p>");
            return;
        }
        let mut root = Dir::default();
        // diff-open で消した Free ファイルも、ツリーの行にする。
        for page in self.pages.iter().chain(&self.deleted.pages) {
            root.insert(page);
        }
        // すべての Free ファイルが 1 つのディレクトリ（docv/free/ など）の中にあるときは、そのディレクトリの行を出さない。
        let mut top = &root;
        let mut path = String::new();
        while top.files.is_empty() && top.dirs.len() == 1 {
            let (name, dir) = top.dirs.iter().next().expect("ディレクトリが 1 つある");
            path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}/{name}")
            };
            top = dir;
        }
        self.tag_filter(out);
        let _ = writeln!(
            out,
            "<table class=\"docs\">\n<thead><tr><th>ファイル</th><th>ミッション</th><th>タグ</th></tr></thead>\n<tbody>"
        );
        self.docs_tree(out, top, &path, 0);
        let _ = writeln!(out, "</tbody>\n</table>");
    }

    /// Free ファイルをディレクトリのツリーにして出す。ディレクトリの中は、ファイル、ディレクトリの順。
    fn docs_tree(&self, out: &mut String, dir: &Dir, path: &str, depth: usize) {
        let mut files = dir.files.clone();
        files.sort_by(|a, b| a.source.cmp(&b.source));
        for page in files {
            let mission = escape(page.mission.as_deref().unwrap_or(""));
            let gone = self.deleted.pages.iter().any(|p| std::ptr::eq(p, page));
            let href = if gone {
                self.deleted_href(&page.source)
            } else {
                self.link(&page.source, "").unwrap_or_default()
            };
            let look = self.look([page.source.as_str()]);
            let _ = writeln!(
                out,
                "<tr{} data-path=\"{}\" data-tags=\"{}\"><td style=\"padding-left:{}px\"><a href=\"{}\">{}</a>{}</td><td title=\"{mission}\">{mission}</td><td>{}</td></tr>",
                class_attr(look.class()),
                escape(&page.source),
                escape(&serde_json::to_string(&page.tags).unwrap_or_default()),
                10 + depth * 20,
                escape(&href),
                escape(&page.title),
                look.pencil(),
                self.tags(page)
            );
        }
        let dirs: Vec<(&String, &Dir, String)> = dir
            .dirs
            .iter()
            .map(|(name, d)| {
                let full = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}/{name}")
                };
                (name, d, full)
            })
            .collect();
        for (name, d, full) in dirs {
            let _ = writeln!(
                out,
                "<tr class=\"dir\" data-dir=\"{}\"><td colspan=\"3\"><div style=\"margin-left:{}px\">{}/</div></td></tr>",
                escape(&full),
                depth * 20,
                escape(name)
            );
            self.docs_tree(out, d, &full, depth + 1);
        }
    }

    /// Free ファイルで使っているタグ。docv.yaml の tags の順に並べる。tags にないタグ（check の error）は、後ろに出てきた順で並べる。
    fn tag_order(&self) -> Vec<&str> {
        let used = |tag: &str| self.pages.iter().any(|p| p.tags.iter().any(|t| t == tag));
        let mut order: Vec<&str> = self
            .project
            .config
            .tags
            .iter()
            .map(|t| t.name.as_str())
            .filter(|t| used(t))
            .collect();
        for tag in self.pages.iter().flat_map(|p| &p.tags) {
            if !order.contains(&tag.as_str()) {
                order.push(tag);
            }
        }
        order
    }

    /// タグのトグル。押すと、選んだタグのどれかを持つ Free ファイルだけを表に出す。すべて選んでいるときは絞り込まない。
    fn tag_filter(&self, out: &mut String) {
        let order = self.tag_order();
        if order.is_empty() {
            return;
        }
        let _ = writeln!(out, "<div class=\"tag-filter\">");
        for tag in &order {
            let count = self
                .pages
                .iter()
                .filter(|p| p.tags.iter().any(|t| t == tag))
                .count();
            let _ = writeln!(
                out,
                "<button type=\"button\" class=\"tag c{}\" data-tag=\"{}\" aria-pressed=\"true\">{}<span class=\"count\">{count}</span></button>",
                self.tag_color(tag),
                escape(tag),
                escape(tag)
            );
        }
        let _ = writeln!(out, "</div>");
    }

    /// ページのタグ。
    fn tags(&self, page: &Page) -> String {
        let mut out = String::new();
        for tag in &page.tags {
            out.push_str(&self.tag(tag));
        }
        out
    }

    fn tag(&self, tag: &str) -> String {
        format!(
            "<span class=\"tag c{}\">{}</span>",
            self.tag_color(tag),
            escape(tag)
        )
    }

    /// タグの色（CSS の `.tag.c0`〜`.tag.c7`）。docv.yaml の tags に書いた色。tags にないタグ（check の error）は c0 にする。
    fn tag_color(&self, tag: &str) -> usize {
        self.project
            .config
            .tag_color(tag)
            .map_or(0, |color| color.index())
    }

    // ---- Markdown ----

    /// Markdown を HTML にする。base はリンクの起点（ルートからの相対パス）。
    /// drop_h1 なら、最初の h1 を除く（ページの h1 と重なるため）。
    /// 見出しは、いちばん浅いものが top の段になるように付け直す。ページの中で h1 から h2、h3 と段を飛ばさないため。
    fn markdown(&self, text: &str, base: &Path, drop_h1: bool, top: usize) -> String {
        self.render_markdown(text, base, drop_h1, top, false)
    }

    /// markdown と同じく HTML にし、見出しに GitHub と同じ形の id を付ける。
    fn markdown_with_ids(&self, text: &str, base: &Path, drop_h1: bool, top: usize) -> String {
        self.render_markdown(text, base, drop_h1, top, true)
    }

    fn render_markdown(
        &self,
        text: &str,
        base: &Path,
        drop_h1: bool,
        top: usize,
        ids: bool,
    ) -> String {
        // 文字を 1 つにまとめてから、URL を探す。パーサーは、`_` などの前で文字を分けて出すことがある。
        let parsed = link_bare_urls(
            TextMergeStream::new(Parser::new_ext(text, markdown::OPTIONS)).collect(),
        );
        let mut dropped_h1 = !drop_h1;
        let mut skipping = false;
        let mut kept = Vec::new();
        for event in parsed {
            match event {
                Event::Start(Tag::Heading {
                    level: HeadingLevel::H1,
                    ..
                }) if !dropped_h1 => skipping = true,
                Event::End(TagEnd::Heading(HeadingLevel::H1)) if skipping => {
                    skipping = false;
                    dropped_h1 = true;
                }
                _ if skipping => {}
                other => kept.push(other),
            }
        }
        let shallowest = kept
            .iter()
            .filter_map(|e| match e {
                Event::Start(Tag::Heading { level, .. }) => Some(*level as usize),
                _ => None,
            })
            .min()
            .unwrap_or(top);
        let level_of = |level: HeadingLevel| {
            HeadingLevel::try_from((level as usize + top - shallowest).clamp(1, 6))
                .unwrap_or(HeadingLevel::H6)
        };
        // 見出しの文字。id に使う。
        let mut texts: Vec<String> = Vec::new();
        let mut in_heading = false;
        for event in &kept {
            match event {
                Event::Start(Tag::Heading { .. }) => {
                    in_heading = true;
                    texts.push(String::new());
                }
                Event::End(TagEnd::Heading(_)) => in_heading = false,
                Event::Text(t) | Event::Code(t) if in_heading => {
                    if let Some(last) = texts.last_mut() {
                        last.push_str(t);
                    }
                }
                _ => {}
            }
        }
        let mut used: BTreeMap<String, usize> = BTreeMap::new();
        let mut index = 0;
        let mut events = Vec::new();
        let mut unlinked = false;
        for event in kept {
            let event = match event {
                Event::Start(Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs,
                }) => {
                    let text = texts.get(index).cloned().unwrap_or_default();
                    index += 1;
                    let id = match id {
                        Some(id) => Some(id),
                        None if ids => {
                            // GitHub と同じく、同じ id が続けば -1、-2 を付ける。
                            let base_id = slug(&text);
                            let n = used.entry(base_id.clone()).or_insert(0);
                            let unique = if *n == 0 {
                                base_id
                            } else {
                                format!("{base_id}-{n}")
                            };
                            *n += 1;
                            Some(CowStr::from(unique))
                        }
                        None => None,
                    };
                    Event::Start(Tag::Heading {
                        level: level_of(level),
                        id,
                        classes,
                        attrs,
                    })
                }
                Event::End(TagEnd::Heading(level)) => Event::End(TagEnd::Heading(level_of(level))),
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => match self.rewrite(base, &dest_url, false) {
                    Some(dest_url) => Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }),
                    // ページのないファイルへのリンクは、文字だけを残す。リンクは入れ子にならない。
                    None => {
                        unlinked = true;
                        continue;
                    }
                },
                Event::End(TagEnd::Link) if unlinked => {
                    unlinked = false;
                    continue;
                }
                Event::Start(Tag::Image {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => Event::Start(Tag::Image {
                    link_type,
                    dest_url: self.rewrite(base, &dest_url, true).unwrap_or(dest_url),
                    title,
                    id,
                }),
                other => other,
            };
            events.push(event);
        }
        let mut body = String::new();
        html::push_html(&mut body, highlight_code_blocks(events).into_iter());
        body
    }

    /// Markdown のリンク先を、HTML から辿れるリンク先にする。ルートの中のファイルで、ページのないものは None（リンクにしない）。
    /// 画像は、元のファイルを出力先から辿れる相対パスで読む。
    fn rewrite<'a>(&self, base: &Path, dest: &CowStr<'a>, image: bool) -> Option<CowStr<'a>> {
        let Some((target, fragment)) = markdown::resolve_link(base, dest) else {
            return Some(dest.clone());
        };
        let target = markdown::slash_path(&target);
        if image {
            return Some(CowStr::from(self.file_path(&target)));
        }
        let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
        self.link(&target, &fragment).map(CowStr::from)
    }

    /// ルートからのパスのファイルの、出力先からのページのパス。ページのないファイルは None。
    fn page_of(&self, target: &str) -> Option<String> {
        // 設定ファイルのページは、現在の設定のページ。
        if self.config_source().as_deref() == Some(target) {
            return Some(CONFIG_PAGE.to_string());
        }
        if target == README_SOURCE && self.readme.is_some() {
            return Some(README_PAGE.to_string());
        }
        if self.pages.iter().any(|p| p.source == target) {
            return Some(doc_page(target));
        }
        if let Some(slot) = self.slots.iter().find(|s| s.path == target) {
            return Some(page_path(slot));
        }
        self.adrs
            .iter()
            .chain(self.pdrs)
            .find(|a| a.path == target)
            .map(decision_page)
    }

    /// ルートの中のファイルへのリンク先。README.md、Free ファイル、Slot ファイル、ADR、PDR だけ、そのページに出力先の中の相対パスで向ける。
    /// docv.yaml は、現在の設定のページに向ける。
    /// HTML は clone したリポジトリで見るので、ほかのファイルはエディタで開けばよく、リンクにしない（None）。
    fn link(&self, target: &str, fragment: &str) -> Option<String> {
        let to = self.out.join(self.page_of(target)?);
        Some(format!(
            "{}{fragment}",
            encode_path(&relative_path(&self.dir.borrow(), &to))
        ))
    }

    /// ルートの中のファイルの、出力先から辿れる相対パス。画像に使う。
    fn file_path(&self, target: &str) -> String {
        relative_path(&self.dir.borrow(), &self.project.root.join(target))
    }
}

/// Slot のアイコン。線で描いた 24 × 24 の SVG。外部の素材に頼らず、バイナリに持つ。
fn icon(id: &str) -> String {
    let shapes = match id {
        // 円の中の i（案内）
        "overview" => {
            r#"<circle cx="12" cy="12" r="9"/><path d="M12 11v6"/><path d="M12 7.5v.01"/>"#
        }
        // 下向きの矢印と受け皿
        "install" => r#"<path d="M12 3v12"/><path d="M7 10l5 5 5-5"/><path d="M5 21h14"/>"#,
        // 箱
        "service" => {
            r#"<path d="M12 2l9 5v10l-9 5-9-5V7z"/><path d="M3 7l9 5 9-5"/><path d="M12 12v10"/>"#
        }
        // 開いた本
        "manual" => {
            r#"<path d="M3 5h6a3 3 0 0 1 3 3v12a2 2 0 0 0-2-2H3z"/><path d="M21 5h-6a3 3 0 0 0-3 3v12a2 2 0 0 1 2-2h7z"/>"#
        }
        // 背のある厚い本と A（辞書）
        "words" => {
            r#"<path d="M5 5a2 2 0 0 1 2-2h12v15H7a2 2 0 0 0-2 2z"/><path d="M5 20a2 2 0 0 0 2 2h12v-4"/><path d="M9.5 13l2.5-6 2.5 6"/><path d="M10.4 11h3.2"/>"#
        }
        // 線でつないだ 3 つの箱（部品の構成図）
        "system-overview" => {
            r#"<rect x="3" y="3" width="7" height="5" rx="1"/><rect x="14" y="3" width="7" height="5" rx="1"/><rect x="8.5" y="16" width="7" height="5" rx="1"/><path d="M6.5 8v3.5h11V8"/><path d="M12 11.5V16"/>"#
        }
        // 重なった層
        "stack" => {
            r#"<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 12.5l9 5 9-5"/><path d="M3 17l9 5 9-5"/>"#
        }
        // 半導体のチップ
        "system" => {
            r#"<rect x="6" y="6" width="12" height="12" rx="1"/><rect x="9.5" y="9.5" width="5" height="5"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4"/>"#
        }
        // 行き来する 2 本の矢印（通信）
        "api" => {
            r#"<path d="M4 8h15"/><path d="M15 4l4 4-4 4"/><path d="M20 16H5"/><path d="M9 12l-4 4 4 4"/>"#
        }
        // 円柱
        "db" => {
            r#"<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5"/><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"/>"#
        }
        // スパナ
        "setup" => {
            r#"<path d="M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.5 2.5-2.4-.6-.6-2.4z"/>"#
        }
        // 錠
        "env" => {
            r#"<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/><path d="M12 15v2"/>"#
        }
        // 端末
        "develop" => {
            r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3"/><path d="M13 15h4"/>"#
        }
        // 屋根と柱（建物の作り）
        "architecture" => {
            r#"<path d="M3 9l9-5 9 5z"/><path d="M6 12v6M10 12v6M14 12v6M18 12v6"/><path d="M3 21h18"/>"#
        }
        // 山括弧と斜線（コード）
        "design" => r#"<path d="M8 7l-5 5 5 5"/><path d="M16 7l5 5-5 5"/><path d="M14 4l-4 16"/>"#,
        // フラスコ（試験）
        "testing" => {
            r#"<path d="M9 3h6"/><path d="M10 3v6l-5.6 9.7A1.5 1.5 0 0 0 5.7 21h12.6a1.5 1.5 0 0 0 1.3-2.3L14 9V3"/><path d="M7 15h10"/>"#
        }
        // 鉛筆
        "doc-guide" => r#"<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M13 7l4 4"/>"#,
        // 雲
        "infrastructure" => {
            r#"<path d="M7 19a4.5 4.5 0 0 1-.6-9A6 6 0 0 1 18 9.5a4.5 4.5 0 0 1-.5 9.5z"/>"#
        }
        // 上向きの矢印と台（環境へ上げる）
        "deploy" => {
            r#"<path d="M12 15V3"/><path d="M7 8l5-5 5 5"/><path d="M4 15v4a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4"/>"#
        }
        // 紙飛行機
        "release" => r#"<path d="M22 2L11 13"/><path d="M22 2l-7 20-4-9-9-4z"/>"#,
        // 円の中の円記号
        "cost" => {
            r#"<circle cx="12" cy="12" r="9"/><path d="M8 7l4 5 4-5"/><path d="M12 12v6"/><path d="M9 13.5h6M9 16.5h6"/>"#
        }
        // 旗（技術の決定の記録）
        "adr" => r#"<path d="M5 21V4"/><path d="M5 4h12l-2.5 4 2.5 4H5"/>"#,
        // 左右を指す道しるべ（プロダクトやサービスの決定の記録）
        "pdr" => {
            r#"<path d="M12 22V3"/><path d="M5 5h11l3 3-3 3H5z"/><path d="M19 13H8l-3 3 3 3h11z"/>"#
        }
        _ => r#"<rect x="4" y="4" width="16" height="16" rx="2"/>"#,
    };
    format!(
        "<svg class=\"icon\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">{shapes}</svg>"
    )
}

/// HTML の全体。CSS と JS は、出力先の直下の docv.css と docv.js を、ページからの相対パス（css、js）で読む。
/// JS は defer で読み、ページを読み終えて CSS が効いてから動かす。file:// でも読めるように、module にはしない。
fn page(title: &str, css: &str, js: &str, nav: &str, body: &str, aside: &str) -> String {
    format!(
        "<!DOCTYPE html>
<html lang=\"ja\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>{title}</title>
<link rel=\"stylesheet\" href=\"{css}\">
<script src=\"{js}\" defer></script>
</head>
<body>
{nav}<main>
{body}</main>
{aside}</body>
</html>
"
    )
}

/// Slot、ADR、PDR、Free ファイルのページの h1 の上に置く、元のファイルのルートからのパスと、それをコピーするボタン。
/// 編集したいときに、元のファイルを見つけられるように。ボタンは JS が動くときだけ出し、印刷の表示には出さない。
/// Slot のページでは、同じ行の左に、docv が決めたミッション（mission）を置き、パスを右に寄せる。何を書くページかを、ページの中で分かるようにするため。
/// Slot の YAML front matter の mission は、check で同じ文だと確かめるので読まない。
fn source(path: &str, mission: Option<&str>) -> String {
    source_line(path, mission, "パスをコピーする")
}

/// 元のファイルのパスの行。text をボタンでコピーする。diff-open では、パスの代わりに git diff のコマンドを出す。
fn source_line(text: &str, mission: Option<&str>, label: &str) -> String {
    let path = escape(text);
    let mission = mission
        .map(|m| format!("<span class=\"mission\">{}</span>", escape(m)))
        .unwrap_or_default();
    format!(
        "<p class=\"source\">{mission}<code>{path}</code><button type=\"button\" class=\"copy\" data-path=\"{path}\" title=\"{label}\" aria-label=\"{label}\" hidden>\
<svg class=\"ready\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><rect x=\"9\" y=\"9\" width=\"11\" height=\"11\" rx=\"2\"/><path d=\"M5 15V6a2 2 0 0 1 2-2h9\"/></svg>\
<svg class=\"done\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"M5 12.5l4.5 4.5L19 7.5\"/></svg>\
</button></p>\n"
    )
}

/// diff-open の印。新しく足したものは緑、消したものは赤で、カードと箱は枠、タブと Free の行は背景に色を付ける。
/// 書き換えたものは鉛筆を出し、マウスを乗せると行数を出す。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Look {
    Plain,
    Added,
    Deleted,
    /// マウスを乗せたときに出す文。
    Edited(String),
}

impl Look {
    /// 色を付ける class。前に空白を付ける。
    fn class(&self) -> &'static str {
        match self {
            Look::Added => " added",
            Look::Deleted => " deleted",
            Look::Plain | Look::Edited(_) => "",
        }
    }

    /// 書き換えたときの鉛筆。
    fn pencil(&self) -> String {
        match self {
            Look::Edited(tip) => pencil_html(tip),
            _ => String::new(),
        }
    }
}

/// class の値（前に空白を付けたもの）を、class 属性にする。空なら空。
fn class_attr(class: &str) -> String {
    if class.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", class.trim_start())
    }
}

/// 変わったファイルの、足した行数と消した行数の合計。見出しの下に 1 行ずつ出す。行数のないバイナリだけなら「バイナリ」。
fn lines_tip<'c>(changes: impl IntoIterator<Item = &'c Change>) -> String {
    let counted: Vec<(u64, u64)> = changes.into_iter().filter_map(|c| c.lines).collect();
    if counted.is_empty() {
        return "差分行数\nバイナリ".to_string();
    }
    let (added, removed) = counted.iter().fold((0, 0), |(a, r), (x, y)| (a + x, r + y));
    format!("差分行数\n追加: {added}\n削除: {removed}")
}

/// 書き換えた印の鉛筆。tip は、マウスを乗せたときに docv.js が鉛筆の右に出す文。改行で行を分ける。
/// 鉛筆は、芯と削った先を線で描き、持ち手を塗り、消しゴムを線で描く。
fn pencil_html(tip: &str) -> String {
    let label = escape(&tip.replace('\n', "、"));
    let tip = escape(tip).replace('\n', "&#10;");
    format!(
        "<span class=\"diff-edit\" data-tip=\"{tip}\" role=\"img\" aria-label=\"{label}\">\
<svg viewBox=\"0 0 24 24\" aria-hidden=\"true\"><g transform=\"rotate(-45 12 12)\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linejoin=\"round\">\
<path d=\"M2.5 12L7 9v6z\" fill=\"none\"/><path d=\"M2.5 12l1.8-1.2v2.4z\" fill=\"currentColor\"/>\
<rect x=\"7\" y=\"9\" width=\"11\" height=\"6\" fill=\"currentColor\"/>\
<path d=\"M18 9h2a1.5 1.5 0 0 1 1.5 1.5v3a1.5 1.5 0 0 1-1.5 1.5h-2z\" fill=\"none\"/></g></svg></span>"
    )
}

/// 「Generated by docv <版>」。docv はリポジトリへ、版はその版のタグのソースへリンクする。
fn generated_by() -> String {
    let (docv, version) = index::generated_by_links();
    format!(
        "Generated by <a href=\"{docv}\">docv</a> <a href=\"{version}\">{}</a>",
        env!("CARGO_PKG_VERSION")
    )
}

/// 表を、横にスクロールできる包みに入れる。幅の狭い画面で表がはみ出しても、ページを横に広げないため。
fn wrap_tables(body: &str) -> String {
    body.replace("<table", "<div class=\"table-scroll\"><table")
        .replace("</table>", "</table></div>")
}

/// 列の幅の見積もりの単位は 0.1em。全角の文字（East Asian Width が W か F。どちらとも取れる A も含める）を 1em、ほかの文字を 0.6em とみなす。
const WIDE: u32 = 10;
const NARROW: u32 = 6;
/// インラインコードの左右の余白（CSS の `:not(pre) > code` の padding）。
const CODE_PADDING: u32 = 6;
/// セルの左右の余白と罫線（CSS の `th, td` の padding と border）。
const CELL_PADDING: u32 = 14;

/// 本文の Markdown の表（属性のない `<table>`）。docv が作る表は class を持つので含まない。
struct MarkdownTable {
    /// 本文の中の位置。`<table>` の始まりから `</table>` の終わりまで。
    start: usize,
    end: usize,
    /// 見出しの行のセルの文字。
    header: Vec<String>,
    /// 列ごとの、セル（見出しも）の中身の幅の見積もりのうち、いちばん広いもの。0.1em 単位。
    widths: Vec<u32>,
}

/// 同じページの中で、見出しの行が同じ Markdown の表が 2 つ以上あれば、列の幅をそろえる。並んだ表を、1 つの表のように縦に読めるようにするため。
/// 列ごとに、そろえる表のすべてのセルの幅を見積もり、いちばん広いものをその列の幅にする。見積もりは build のときにするので、JS は使わない。
/// 幅のいちばん広い列は、幅を決めずに残りの幅を使わせる。ほかの列（短い列）は、幅を `<col>` に書き、セルを折り返さない（class="short"）。
/// 見出しの行が同じ表のない表は変えない。
fn align_tables(body: &str) -> String {
    let tables = markdown_tables(body);
    let mut groups: BTreeMap<&[String], Vec<usize>> = BTreeMap::new();
    for (i, table) in tables.iter().enumerate() {
        groups.entry(&table.header).or_default().push(i);
    }
    // 表ごとの、列の幅。いちばん広い列は None。そろえない表は None。
    let mut columns: Vec<Option<Vec<Option<u32>>>> = vec![None; tables.len()];
    for members in groups.values().filter(|members| members.len() > 1) {
        let widths: Vec<u32> = (0..tables[members[0]].widths.len())
            .map(|c| {
                members
                    .iter()
                    .map(|&i| tables[i].widths[c])
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        // 同じ幅なら、右の列に残りの幅を使わせる。説明の列は、右に置くことが多いため。
        let flexible = widths
            .iter()
            .enumerate()
            .max_by_key(|(i, w)| (**w, *i))
            .map(|(i, _)| i);
        let cols: Vec<Option<u32>> = widths
            .iter()
            .enumerate()
            .map(|(i, w)| (Some(i) != flexible).then_some(*w))
            .collect();
        for &i in members {
            columns[i] = Some(cols.clone());
        }
    }
    let mut out = String::with_capacity(body.len());
    let mut last = 0;
    for (table, cols) in tables.iter().zip(&columns) {
        let Some(cols) = cols else { continue };
        out.push_str(&body[last..table.start]);
        out.push_str(&aligned_table(&body[table.start..table.end], cols));
        last = table.end;
    }
    out.push_str(&body[last..]);
    out
}

/// 本文の Markdown の表を、出てくる順に返す。
fn markdown_tables(body: &str) -> Vec<MarkdownTable> {
    const OPEN: &str = "<table>";
    const CLOSE: &str = "</table>";
    let mut tables = Vec::new();
    let mut from = 0;
    while let Some(i) = body[from..].find(OPEN) {
        let start = from + i;
        let Some(j) = body[start..].find(CLOSE) else {
            break;
        };
        let end = start + j + CLOSE.len();
        from = end;
        let html = &body[start..end];
        // 表の中に表がある（HTML で書いた）ものは、そろえない。
        if html[OPEN.len()..].contains("<table") {
            continue;
        }
        let rows = table_rows(html);
        let Some(header) = rows.first().filter(|cells| !cells.is_empty()) else {
            continue;
        };
        let mut widths = vec![0; header.len()];
        for row in &rows {
            for (width, cell) in widths.iter_mut().zip(row) {
                *width = (*width).max(cell_width(cell));
            }
        }
        tables.push(MarkdownTable {
            start,
            end,
            header: header
                .iter()
                .map(|cell| unescape(&strip_tags(cell)).trim().to_string())
                .collect(),
            widths,
        });
    }
    tables
}

/// 表の行ごとの、セルの中身（HTML）。見出しの行が先頭。
fn table_rows(html: &str) -> Vec<Vec<&str>> {
    let mut rows = Vec::new();
    for (i, _) in html.match_indices("<tr>") {
        let row = &html[i..];
        let row = &row[..row.find("</tr>").unwrap_or(row.len())];
        let mut cells = Vec::new();
        for (j, _) in row.match_indices('<') {
            let tag = &row[j..];
            let Some(name) = cell_tag(tag) else {
                continue;
            };
            let Some(open) = tag.find('>') else {
                continue;
            };
            let inner = &tag[open + 1..];
            let close = inner.find(&format!("</{name}>")).unwrap_or(inner.len());
            cells.push(&inner[..close]);
        }
        rows.push(cells);
    }
    rows
}

/// セルの開くタグ（`<th>`、`<td style="...">` など）なら、その名前。`<thead>` などは None。
fn cell_tag(tag: &str) -> Option<&'static str> {
    ["th", "td"].into_iter().find(|name| {
        tag.strip_prefix('<')
            .and_then(|t| t.strip_prefix(name))
            .is_some_and(|t| t.starts_with('>') || t.starts_with(' '))
    })
}

/// セルの中身の幅の見積もり（0.1em 単位）。`<br>` で分けた行のうち、いちばん広いものに、セルの余白を足す。
fn cell_width(inner: &str) -> u32 {
    let lines = inner
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    let widest = lines
        .split('\n')
        .map(|line| {
            let code = line.matches("<code").count() as u32 * CODE_PADDING;
            let text: u32 = unescape(&strip_tags(line))
                .trim()
                .chars()
                .map(|c| match UnicodeWidthChar::width_cjk(c) {
                    Some(2) => WIDE,
                    Some(0) | None => 0,
                    _ => NARROW,
                })
                .sum();
            code + text
        })
        .max()
        .unwrap_or(0);
    widest + CELL_PADDING
}

/// そろえる表。短い列には、見積もった幅を `<col>` に書き、セルに class="short" を付けて折り返さない。
/// cols は列ごとの幅（0.1em 単位）。いちばん広い列は None で、幅を決めずに残りの幅を使う。
fn aligned_table(html: &str, cols: &[Option<u32>]) -> String {
    let mut out = String::from("<table class=\"aligned\"><colgroup>");
    for col in cols {
        match col {
            Some(width) => {
                let _ = write!(
                    out,
                    "<col style=\"width: {}.{}em\">",
                    width / 10,
                    width % 10
                );
            }
            None => out.push_str("<col>"),
        }
    }
    out.push_str("</colgroup>");
    let rest = &html["<table>".len()..];
    let mut last = 0;
    let mut column = 0;
    for (i, _) in rest.match_indices('<') {
        let tag = &rest[i..];
        if tag.starts_with("<tr>") {
            column = 0;
            continue;
        }
        if cell_tag(tag).is_none() {
            continue;
        }
        if let (Some(Some(_)), Some(open)) = (cols.get(column), tag.find('>')) {
            out.push_str(&rest[last..i + open]);
            out.push_str(" class=\"short\"");
            last = i + open;
        }
        column += 1;
    }
    out.push_str(&rest[last..]);
    out
}

/// 本文の h2 と h3 から、右のサイドバーの目次を作る。id のない見出しには、GitHub と同じ形の id を付ける。
/// 見出しがなければ、目次の代わりに同じ幅の空の枠を置く。ページを移っても本文の位置が変わらないようにするため。
fn outline(body: &str) -> (String, String) {
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut rest = body;
    while let Some(i) = rest.find(" id=\"") {
        rest = &rest[i + 5..];
        if let Some(end) = rest.find('"') {
            used.insert(rest[..end].to_string());
        }
    }
    let mut out = String::with_capacity(body.len());
    let mut items = Vec::new();
    let mut rest = body;
    while let Some(i) = ["<h2", "<h3"].iter().filter_map(|t| rest.find(t)).min() {
        let level = &rest[i + 2..i + 3];
        let after = &rest[i + 3..];
        // <header> などを見出しと取り違えない。
        if !after.starts_with('>') && !after.starts_with(' ') {
            out.push_str(&rest[..i + 3]);
            rest = after;
            continue;
        }
        let (Some(open), Some(close)) = (after.find('>'), after.find(&format!("</h{level}>")))
        else {
            break;
        };
        let attrs = &after[..open];
        let inner = &after[open + 1..close];
        let text = strip_tags(inner);
        let id = match attrs.find(" id=\"") {
            Some(j) => attrs[j + 5..].split('"').next().unwrap_or("").to_string(),
            None => {
                let base = slug(&unescape(&text));
                let mut id = base.clone();
                let mut n = 1;
                while used.contains(&id) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                used.insert(id.clone());
                id
            }
        };
        out.push_str(&rest[..i + 3]);
        if !attrs.contains(" id=\"") {
            let _ = write!(out, " id=\"{}\"", escape(&id));
        }
        out.push_str(&after[..close]);
        rest = &after[close..];
        items.push((level.to_string(), id, text));
    }
    out.push_str(rest);
    if items.is_empty() {
        return (out, "<div class=\"outline\"></div>\n".to_string());
    }
    let mut aside =
        String::from("<nav class=\"outline\">\n<p class=\"outline-title\">目次</p>\n<ul>\n");
    for (level, id, text) in items {
        let _ = writeln!(
            aside,
            "<li class=\"h{level}\"><a href=\"#{}\">{}</a></li>",
            escape(&id),
            text.trim()
        );
    }
    aside.push_str("</ul>\n</nav>\n");
    (out, aside)
}

/// HTML のタグを除いた文字。実体参照は、そのまま残す。
fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// escape の逆。
fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// リンクに書くパス。空白などを、URL で使える形にする。
fn encode_path(path: &str) -> String {
    let mut out = String::new();
    for c in path.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '%' => out.push_str("%25"),
            _ => out.push(c),
        }
    }
    out
}

/// Free ファイルのツリーの、ディレクトリ 1 つ。
#[derive(Default)]
struct Dir<'a> {
    files: Vec<&'a Page>,
    dirs: BTreeMap<String, Dir<'a>>,
}

impl<'a> Dir<'a> {
    fn insert(&mut self, page: &'a Page) {
        let mut dir = self;
        let mut parts: Vec<&str> = page.source.split('/').collect();
        parts.pop();
        for part in parts {
            dir = dir.dirs.entry(part.to_string()).or_default();
        }
        dir.files.push(page);
    }
}

/// 決定のページの、出力先からの相対パス。`ADR/<まとまり>/<題>.html` か `PDR/<まとまり>/<題>.html`。
/// 名前の空白は除く（`ADR/言語/Goにする.html`）。
fn decision_page(a: &Decision) -> String {
    format!(
        "{}/{}/{}.html",
        a.kind.label(),
        file_name(a.group()),
        file_name(&a.record.title)
    )
}

/// 名前を、ファイル名に使える形にする。区切りの文字は - にし、空白は除く。
fn file_name(name: &str) -> String {
    no_space(&name.replace(['/', '\\'], "-"))
}

/// 出力するファイルとディレクトリの名前から、空白（全角も含む）を除く。CLI や git で、パスを引用符で囲まずに扱えるように。
fn no_space(name: &str) -> String {
    name.chars().filter(|c| !c.is_whitespace()).collect()
}

/// フェンスのコードブロックに、言語に合わせて色を付ける。色は `<span style>` で埋め込む。
/// 言語がないブロックと、docv が知らない言語のブロックは、そのまま出す。
fn highlight_code_blocks(events: Vec<Event>) -> Vec<Event> {
    let mut out = Vec::with_capacity(events.len());
    let mut iter = events.into_iter();
    while let Some(event) = iter.next() {
        let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = &event else {
            out.push(event);
            continue;
        };
        let lang = info.split_whitespace().next().unwrap_or("").to_string();
        let mut inner = Vec::new();
        let mut code = String::new();
        for e in iter.by_ref() {
            if matches!(e, Event::End(TagEnd::CodeBlock)) {
                break;
            }
            if let Event::Text(t) = &e {
                code.push_str(t);
            }
            inner.push(e);
        }
        match highlight(&lang, &code) {
            Some(html) => out.push(Event::Html(CowStr::from(format!(
                "<pre><code class=\"language-{}\">{html}</code></pre>\n",
                escape(&lang)
            )))),
            None => {
                out.push(event);
                out.extend(inner);
                out.push(Event::End(TagEnd::CodeBlock));
            }
        }
    }
    out
}

/// コードを、言語の構文に沿って色付きの HTML にする。知らない言語なら None。
fn highlight(lang: &str, code: &str) -> Option<String> {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME: OnceLock<Theme> = OnceLock::new();
    if lang.is_empty() {
        return None;
    }
    let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let theme = THEME.get_or_init(|| {
        ThemeSet::load_defaults()
            .themes
            .remove("InspiredGitHub")
            .expect("syntect の既定のテーマにある")
    });
    let syntax = syntaxes.find_syntax_by_token(lang)?;
    let mut lines = HighlightLines::new(syntax, theme);
    let mut html = String::new();
    for line in LinesWithEndings::from(code) {
        let ranges = lines.highlight_line(line, syntaxes).ok()?;
        html.push_str(&styled_line_to_highlighted_html(&ranges, IncludeBackground::No).ok()?);
    }
    Some(html)
}

/// 本文の文字の中の、http:// と https:// で始まる URL をリンクにする。`[文字](URL)` の形で書かなくても、HTML から辿れるように。
/// リンク、画像、コードブロックの中と、HTML で書いた `<a>` の中の文字は変えない。
fn link_bare_urls(events: Vec<Event>) -> Vec<Event> {
    let mut out = Vec::with_capacity(events.len());
    // リンクにしない範囲の深さ。
    let mut inside = 0usize;
    for event in events {
        match &event {
            Event::Start(Tag::Link { .. } | Tag::Image { .. } | Tag::CodeBlock(_)) => inside += 1,
            Event::End(TagEnd::Link | TagEnd::Image | TagEnd::CodeBlock) => {
                inside = inside.saturating_sub(1);
            }
            Event::InlineHtml(html) if html.starts_with("<a ") || html.starts_with("<a>") => {
                inside += 1;
            }
            Event::InlineHtml(html) if html.starts_with("</a>") => {
                inside = inside.saturating_sub(1)
            }
            Event::Text(text) if inside == 0 => {
                let urls = bare_urls(text);
                if !urls.is_empty() {
                    let mut last = 0;
                    for (start, end) in urls {
                        if last < start {
                            out.push(Event::Text(CowStr::from(text[last..start].to_string())));
                        }
                        let url = &text[start..end];
                        out.push(Event::Start(Tag::Link {
                            link_type: LinkType::Autolink,
                            dest_url: CowStr::from(url.to_string()),
                            title: CowStr::from(""),
                            id: CowStr::from(""),
                        }));
                        out.push(Event::Text(CowStr::from(url.to_string())));
                        out.push(Event::End(TagEnd::Link));
                        last = end;
                    }
                    if last < text.len() {
                        out.push(Event::Text(CowStr::from(text[last..].to_string())));
                    }
                    continue;
                }
            }
            _ => {}
        }
        out.push(event);
    }
    out
}

/// 文の中の URL の位置（始まり、終わり）。URL は、空白、`<`、`>`、ASCII でない文字の前で終わる。日本語の文に続けて書いても、URL に文が混ざらないように。
/// 末尾の句読点と、対になっていない `)` は URL に含めない。英数字の直後から始まるものは URL と見なさない。
fn bare_urls(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(start) = ["https://", "http://"]
        .iter()
        .filter_map(|scheme| text[from..].find(scheme).map(|i| from + i))
        .min()
    {
        let scheme = if text[start..].starts_with("https://") {
            "https://".len()
        } else {
            "http://".len()
        };
        let mut end = text[start..]
            .find(|c: char| c.is_whitespace() || c == '<' || c == '>' || !c.is_ascii())
            .map_or(text.len(), |i| start + i);
        loop {
            let url = &text[start..end];
            match url.chars().last() {
                Some('.' | ',' | ':' | ';' | '!' | '?' | '*' | '_' | '~' | '\'' | '"') => end -= 1,
                Some(')') if url.matches(')').count() > url.matches('(').count() => end -= 1,
                _ => break,
            }
        }
        let after_word = text[..start]
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric());
        if end > start + scheme && !after_word {
            found.push((start, end));
        }
        from = end.max(start + scheme);
    }
    found
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Locate;

    fn write_file(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            dir.path(),
            "docv.yaml",
            "title: demo\ndocv_exclude: [docv/free/tmp/**]\ntags:\n  - name: 仕様\n    color: blue\n",
        );
        for (path, text) in files {
            write_file(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn builds_index_and_slot_pages_deterministically() {
        let (dir, project) = project(&[
            ("README.md", "# Demo\n"),
            (
                "docv/free/spec/a.md",
                "---\nmission: A は何か\ntags: [仕様]\n---\n\n# A <仕様>\n\n## 1. 章\n\n本文\n",
            ),
            ("docv/free/plan/b.md", "---\nmission: q\n---\n# B\n"),
            (
                "docv/adr/go.md",
                "---\ngroup: 言語\ndate: 2026-01-01\n---\n\n# Go にする\n\nGo で書く\n",
            ),
            (
                "docv/adr/rust.md",
                "---\ngroup: 言語\ndate: 2026-03-01\nsupersedes: [go]\n---\n\n# Rust にする\n\n## 選ばなかった案\n\n| 案 | 理由 |\n| --- | --- |\n| Zig | 情報が少ない |\n\n### 細部\n",
            ),
            (
                "docv/adr/react.md",
                "---\ngroup: 言語\ndate: 2026-02-01\n---\n\n# React にする\n\nReact で書く\n",
            ),
            ("docv/free/tmp/memo.md", "# Memo\n"),
            (
                "docv/slot/service/install.md",
                "---\nmission: 利用者がこのサービスをインストールする方法\n---\n\n# インストール\n\n## 手順\n\n### 取る\n\n#### macOS\n\n`curl`\n\n### cargo\n\n`cargo install`\n",
            ),
            // 残った YAML の Slot は、ページにしない
            (
                "docv/slot/guide/setup.yaml",
                "steps:\n  - id: clone\n    name: 取ってくる\n",
            ),
            (
                "docv/slot/service/service.md",
                "# サービス仕様\n\n概要の本文\n\n## 部品\n\n[a](../../free/spec/a.md)\n\n<details>\n\n細部\n\n</details>\n\n### 小見出し\n",
            ),
            (
                "docv/slot/service/manual.md",
                "# 利用マニュアル\n\n| コマンド | 説明 |\n| --- | --- |\n| [demo run](#demo-run) | 動かす |\n\n## demo run\n\n詳しく\n",
            ),
            ("docv/slot/system/db.md", "# DB 仕様書\n\n表\n"),
            (
                "docv/slot/operation/cost.md",
                "---\nmission: かかっている費用と、費用を理由にやっていないこと\n---\n\n# 費用\n\n## かかっている費用\n\n| 説明 | 月額 |\n| --- | --: |\n| サーバーでは、API を動かす | 3 ドル |\n\n## 費用を理由にやっていないこと\n\n| 説明 | 月額 |\n| --- | --: |\n| 監視は入れない | 20 ドル |\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();

        // index.html（Slot）には、Slot のページへのカードだけを置く
        let index = fs::read_to_string(out.join(PAGE)).unwrap();
        let detail = fs::read_to_string(out.join(DETAIL_PAGE)).unwrap();
        let adr_index = fs::read_to_string(out.join(ADR_PAGE)).unwrap();
        let config = fs::read_to_string(out.join(CONFIG_PAGE)).unwrap();
        // サイドバーで README とトップのページを行き来する。描いているページの項目に印を付ける。段の間に横線を引く
        assert!(
            index.contains("<li><a href=\"README.html\">README</a></li>\n<li><a href=\"index.html\" aria-current=\"page\">Slot</a></li>\n<li><a href=\"プロジェクト詳細.html\">Free</a></li>\n<li><a href=\"ADR.html\">ADR</a></li>\n<li class=\"separator\" role=\"separator\"></li>\n<li><a href=\"インデックス.html\">Index</a></li>\n<li><a href=\"レビューガイド.html\">Review Guide</a></li>\n<li><a href=\"現在の設定.html\">Config</a></li>\n<li class=\"separator\" role=\"separator\"></li>\n<li><a href=\"検索.html\">Search</a></li>"),
            "{index}"
        );
        // インデックス.html に、インデックスと同じ並びで、md ではなく HTML のページへのリンクを並べる
        let index_page = fs::read_to_string(out.join(INDEX_PAGE)).unwrap();
        assert!(
            index_page.contains("<main>\n<h1>Index</h1>\n<h2 id=\"サービス\">サービス</h2>"),
            "{index_page}"
        );
        assert!(
            index_page.contains("<li><a href=\"サービス/インストール.html\">インストール</a></li>"),
            "{index_page}"
        );
        assert!(
            index_page.contains("<h2 id=\"free-ファイル\">Free ファイル</h2>\n<ul>\n<li><a href=\"プロジェクト詳細.html\">Free</a></li>\n</ul>\n<h2 id=\"adr\">ADR</h2>\n<ul>\n<li><a href=\"ADR.html\">ADR</a></li>"),
            "{index_page}"
        );
        // レビューガイド.html に、docv review-guide と同じ手順を出し、一覧は出さない。手順の h1 は除き、ページの h1 を Review Guide にする
        let review = fs::read_to_string(out.join(REVIEW_PAGE)).unwrap();
        assert!(
            review
                .contains("<a href=\"レビューガイド.html\" aria-current=\"page\">Review Guide</a>"),
            "{review}"
        );
        assert!(
            review.contains("<main>\n<section class=\"doc\">\n<h1>Review Guide</h1>\n<p class=\"note\"><code>docv review-guide</code> が出す手順と同じ内容。</p>\n"),
            "{review}"
        );
        assert!(!review.contains("<h1>レビューガイド</h1>"), "{review}");
        assert!(
            review.contains("<h2 id=\"レビュー手順\">レビュー手順</h2>"),
            "{review}"
        );
        assert!(
            review.contains("<h2 id=\"一覧\">一覧</h2>\n<p>一覧は、<code>docv review-guide</code> を実行したときに、その時点のファイルから作る。</p>\n</section>"),
            "{review}"
        );
        assert!(!review.contains("install.md"), "{review}");
        assert_headings(&review);
        // 検索.html に、Slot ファイル、ADR、Free ファイルの順で、中身を行ごとに索引にして書く。< は書き換え、</script> で閉じない
        let search = fs::read_to_string(out.join(SEARCH_PAGE)).unwrap();
        assert!(
            search.contains("<a href=\"検索.html\" aria-current=\"page\">Search</a>"),
            "{search}"
        );
        assert!(
            search.contains("{\"path\":\"docv/free/spec/a.md\",\"href\":\"プロジェクト詳細/docv/free/spec/a.html\",\"tags\":[[\"仕様\",0]],\"lines\":[\"---\",\"mission: A は何か\","),
            "{search}"
        );
        let slot_entry = search
            .find("\"path\":\"docv/slot/service/install.md\"")
            .unwrap();
        // 残った YAML の Slot は、索引にも入れない
        assert!(!search.contains("setup.yaml"), "{search}");
        let adr_entry = search.find("\"path\":\"docv/adr/go.md\"").unwrap();
        let free_entry = search.find("\"path\":\"docv/free/spec/a.md\"").unwrap();
        assert!(slot_entry < adr_entry && adr_entry < free_entry, "{search}");
        // 見出しのないページは、目次の代わりに同じ幅の空の枠を置く
        assert!(!search.contains("<nav class=\"outline\">"), "{search}");
        assert!(
            search.contains("</main>\n<div class=\"outline\"></div>\n</body>"),
            "{search}"
        );
        // 現在の設定.html に、docv.yaml のすべての項目を 1 つの表で並べる。選ぶ項目は、すべての候補を横に並べる
        assert!(
            config.contains("<a href=\"現在の設定.html\" aria-current=\"page\">"),
            "{config}"
        );
        assert_eq!(
            config.matches("<table class=\"config\"").count(),
            1,
            "{config}"
        );
        assert!(
            config.contains("<tr><td><code>format.enabled</code></td><td><div class=\"choices\"><span class=\"choice\">true</span><span class=\"choice on\">false</span></div></td></tr>"),
            "{config}"
        );
        assert!(
            config.contains("<tr><td><code>dir.document_root</code></td><td><code>docv</code></td></tr>\n<tr><td><code>dir.output</code></td><td><code>output/docv</code></td></tr>"),
            "{config}"
        );
        assert!(!config.contains("generate.image"), "{config}");
        // docv が作る表は、見出しの行が同じでも、列の幅をそろえない
        assert!(!config.contains("class=\"aligned\""), "{config}");
        // docv が決めた textlint のルールを、検査項目と非検査項目に分け、プリセットごとの表に並べる
        assert!(
            config.contains("<h3 id=\"検査項目\">検査項目</h3>\n<h4>ja-technical-writing</h4>\n<div class=\"table-scroll\"><table class=\"textlint\">\n<thead><tr><th>ルール</th><th>確かめること</th></tr></thead>\n<tbody>\n<tr><td><code>sentence-length</code></td><td>一文を 100 文字以内にする</td></tr>"),
            "{config}"
        );
        assert!(
            config.contains("<h3 id=\"非検査項目\">非検査項目</h3>\n<h4>ja-technical-writing</h4>\n<div class=\"table-scroll\"><table class=\"textlint\">\n<thead><tr><th>ルール</th><th>確かめないこと</th></tr></thead>\n<tbody>\n<tr><td><code>max-kanji-continuous-len</code></td><td>漢字が続く長さ</td></tr>"),
            "{config}"
        );
        assert!(
            config.contains(
                "<tr><td><code>tags</code></td><td><span class=\"tag c0\">仕様</span></td></tr>"
            ),
            "{config}"
        );
        assert!(
            detail.contains("<a href=\"プロジェクト詳細.html\" aria-current=\"page\">"),
            "{detail}"
        );
        assert!(!index.contains("class=\"decision-box"), "{index}");
        assert!(!index.contains(">spec/</td>"), "{index}");
        // トップのページにも、ページの名前の h1 を置く。現在の設定の項目の表の見出しは、h1 と重ならないように docv.yaml にする
        assert!(index.contains("<main>\n<h1>Slot</h1>\n"), "{index}");
        assert!(detail.contains("<main>\n<h1>Free</h1>\n"), "{detail}");
        assert!(
            config.contains(
                "<main>\n<h1>Config</h1>\n<h2 id=\"docvyaml\" class=\"group\">docv.yaml</h2>"
            ),
            "{config}"
        );
        for html in [&index, &detail, &config, &adr_index] {
            assert_headings(html);
        }
        // サービスのまとまりは、サービス概要、サービス仕様、インストール、利用マニュアル、用語集の順
        let overview = index.find(">サービス概要<span class=\"state\">").unwrap();
        // ページに分けられるサービス仕様は、1 つのファイルでも 1 tab と書く
        let service = index
            .find(">サービス仕様<span class=\"state\">1 tab</span></a>")
            .unwrap();
        let manual = index.find(">利用マニュアル</a>").unwrap();
        let install = index.find(">インストール</a>").unwrap();
        let words = index.find(">用語集<span class=\"state\">").unwrap();
        assert!(
            overview < service && service < install && install < manual && manual < words,
            "{index}"
        );
        // システムのまとまりは、システム概要、システム仕様、技術スタック、API 仕様書、DB 仕様書の順
        let at = |label: &str| index.find(&format!(">{label}<")).unwrap();
        let system = [
            "システム概要",
            "システム仕様",
            "技術スタック",
            "API 仕様書",
            "DB 仕様書",
        ]
        .map(at);
        assert!(system.is_sorted(), "{index}");
        assert!(
            index.contains(
                "<a class=\"card\" href=\"サービス/利用マニュアル.html\"><svg class=\"icon\""
            ),
            "{index}"
        );
        // ページに分けていない Slot のカードには、ページの数を書かない
        assert!(index.contains("</svg>利用マニュアル</a>"), "{index}");
        // ファイルのない Slot も、リンクにせずグレーで並べ、色だけに頼らず「未作成」と書く
        assert!(
            index.contains("<span class=\"card missing\"><svg class=\"icon\""),
            "{index}"
        );
        assert!(
            index.contains("</svg>用語集<span class=\"state\">未作成</span></span>"),
            "{index}"
        );
        // プロジェクト詳細.html に、Free ファイルのページへのリンクをツリーに並べる。管理外のファイルは出さない
        let plan = detail
            .find("<tr class=\"dir\" data-dir=\"docv/free/plan\"><td colspan=\"3\"><div style=\"margin-left:0px\">plan/</div></td></tr>")
            .unwrap();
        let spec = detail.find(">spec/</div>").unwrap();
        // すべての Free ファイルが入っている docv/free/ の行は出さず、ファイル名も出さない。タグには色を付ける
        assert!(!detail.contains(">docv/free/</div>"), "{detail}");
        assert!(
            detail.contains("<tr data-path=\"docv/free/spec/a.md\" data-tags=\"[&quot;仕様&quot;]\"><td style=\"padding-left:30px\"><a href=\"プロジェクト詳細/docv/free/spec/a.html\">A &lt;仕様&gt;</a></td><td title=\"A は何か\">A は何か</td><td><span class=\"tag c0\">仕様</span></td>"),
            "{detail}"
        );
        // 表の上に、タグのトグルを数と一緒に並べる。最初はすべて選んでいる
        assert!(
            detail.contains("<button type=\"button\" class=\"tag c0\" data-tag=\"仕様\" aria-pressed=\"true\">仕様<span class=\"count\">1</span></button>"),
            "{detail}"
        );
        // Free ファイルは 1 つずつ HTML のページにする。見出しは h1 から段を飛ばさない
        let a = fs::read_to_string(out.join("プロジェクト詳細/docv/free/spec/a.html")).unwrap();
        assert!(a.contains("<h1>A &lt;仕様&gt;</h1>"), "{a}");
        // 見出しには GitHub と同じ形の id を付け、右のサイドバーに目次を置く
        assert!(a.contains("<h2 id=\"1-章\">1. 章</h2>"), "{a}");
        assert!(
            a.contains("<nav class=\"outline\">\n<p class=\"outline-title\">目次</p>\n<ul>\n<li class=\"h2\"><a href=\"#1-章\">1. 章</a></li>"),
            "{a}"
        );
        assert!(
            a.contains("<a href=\"../../../../プロジェクト詳細.html\" aria-current=\"page\">"),
            "{a}"
        );
        // h1 の上に、元のファイルのルートからのパスと、それをコピーするボタンを置く。ボタンは JS が動くときだけ出す
        assert!(
            a.contains("<main>\n<p class=\"source\"><code>docv/free/spec/a.md</code><button type=\"button\" class=\"copy\" data-path=\"docv/free/spec/a.md\" title=\"パスをコピーする\" aria-label=\"パスをコピーする\" hidden>"),
            "{a}"
        );
        // トップのページには、元のファイルのパスを出さない
        assert!(!index.contains("class=\"source\""), "{index}");
        assert!(!detail.contains("class=\"source\""), "{detail}");
        assert_headings(&a);
        assert!(plan < spec, "{detail}");
        assert!(!detail.contains("tmp/"), "{detail}");

        // Slot は 1 つずつ独立したページ
        let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();
        let install = read("サービス/インストール.html");
        // Slot のページにも、h1 の上に元のファイルのパスを置く。同じ行の左に、docv が決めたミッションを置く
        assert!(
            install.contains("<main>\n<p class=\"source\"><span class=\"mission\">利用者がこのサービスをインストールする方法</span><code>docv/slot/service/install.md</code><button type=\"button\" class=\"copy\" data-path=\"docv/slot/service/install.md\" title=\"パスをコピーする\" aria-label=\"パスをコピーする\" hidden>"),
            "{install}"
        );
        // Slot のページのサイドバーは、1 つ上のトップのページを指す
        assert!(
            install.contains("<a href=\"../index.html\" aria-current=\"page\">Slot</a>"),
            "{install}"
        );
        // 本文は、h1 を除き、見出しは h2 から始まるように段を付け直して、そのまま出す。OS ごとのタブは作らない
        assert!(
            install.contains("<h2 id=\"手順\">手順</h2>\n<h3 id=\"取る\">取る</h3>\n<h4 id=\"macos\">macOS</h4>\n<p><code>curl</code></p>\n<h3 id=\"cargo\">cargo</h3>"),
            "{install}"
        );
        assert!(!install.contains("class=\"tabs\""), "{install}");
        // 残った YAML の Slot は、ページにせず、カードも未作成にする
        assert!(!out.join("開発者向けガイド/環境構築.html").exists());
        assert!(
            index.contains("</svg>環境構築<span class=\"state\">未作成</span></span>"),
            "{index}"
        );
        // Slot は、h1 を除き、見出しを下げて、そのまま出す。見出しには id を付ける。details は閉じたまま出す
        let service = read("サービス/サービス仕様.html");
        assert!(service.contains("<h2 id=\"部品\">部品</h2>"), "{service}");
        assert!(
            // Markdown の中のリンクは、pulldown-cmark が和名をパーセントエンコードする
            service.contains("href=\"../%E3%83%97%E3%83%AD%E3%82%B8%E3%82%A7%E3%82%AF%E3%83%88%E8%A9%B3%E7%B4%B0/docv/free/spec/a.html\">a</a>"),
            "{service}"
        );
        assert!(service.contains("<details>"), "{service}");
        assert!(!service.contains("<details open>"), "{service}");
        // Slot の見出しには id が付き、ページ内リンクで飛べる
        let manual = read("サービス/利用マニュアル.html");
        assert!(
            manual.contains("<a href=\"#demo-run\">demo run</a>"),
            "{manual}"
        );
        assert!(
            manual.contains("<h2 id=\"demo-run\">demo run</h2>"),
            "{manual}"
        );
        // 名前に空白のある Slot のページは、ファイル名から空白を除く
        assert!(index.contains("href=\"システム/DB仕様書.html\""), "{index}");
        assert!(out.join("システム/DB仕様書.html").is_file());
        let cost = read("運用/費用.html");
        // 見出しの行が同じ表は、列の幅をそろえる。短い列（月額）は折り返さず、いちばん長い列（説明）が残りの幅を使う
        assert_eq!(
            cost.matches("<table class=\"aligned\">").count(),
            2,
            "{cost}"
        );
        assert!(
            cost.contains("<div class=\"table-scroll\"><table class=\"aligned\"><colgroup><col><col style=\"width: 5.2em\"></colgroup><thead><tr><th>説明</th><th style=\"text-align: right\" class=\"short\">月額</th></tr></thead><tbody>\n<tr><td>サーバーでは、API を動かす</td><td style=\"text-align: right\" class=\"short\">3 ドル</td></tr>"),
            "{cost}"
        );
        // 見出しの行が同じ表のない表は、そのまま出す
        assert!(!manual_has_aligned(&read("サービス/利用マニュアル.html")));
        // Slot のページにも、id を付けた見出しから右のサイドバーの目次を置く
        assert!(
            cost.contains("<li class=\"h2\"><a href=\"#費用を理由にやっていないこと\">費用を理由にやっていないこと</a></li>"),
            "{cost}"
        );

        // ADR は、ADR.html に、まとまりごとの h2 と決定のつながりの図を置く。
        // 箱をクリックすると、決定 1 つのページを開く
        assert!(
            adr_index.contains("<main>\n<h1>Architecture Decision Record</h1>\n<h2 id=\"言語\" class=\"group\">言語</h2>\n<div class=\"lanes\">"),
            "{adr_index}"
        );
        assert!(
            adr_index.contains("<div class=\"lane\"><a class=\"decision-box superseded\" href=\"ADR/言語/Goにする.html\">Go にする</a><span class=\"arrow\">→</span><a class=\"decision-box\" href=\"ADR/言語/Rustにする.html\">"),
            "{adr_index}"
        );
        // つながりのない決定は、つながりの後ろに、箱を折り返して並べる
        assert!(
            adr_index.contains("</div>\n<div class=\"decision-grid\"><a class=\"decision-box\" href=\"ADR/言語/Reactにする.html\">React にする</a></div>"),
            "{adr_index}"
        );
        let go = read("ADR/言語/Goにする.html");
        assert!(go.contains("</svg>Go にする</h1>"), "{go}");
        // ADR のページにも、h1 の上に元のファイルのパスを置く
        assert!(
            go.contains("<main>\n<p class=\"source\"><code>docv/adr/go.md</code><button type=\"button\" class=\"copy\" data-path=\"docv/adr/go.md\" title=\"パスをコピーする\" aria-label=\"パスをコピーする\" hidden>"),
            "{go}"
        );
        // 打ち消された決定は、h1 の下に、打ち消されたことと新しい決定へのリンクを出す
        assert!(
            go.contains("</svg>Go にする</h1>\n<p class=\"status\">この決定は打ち消された。新しい決定は <a href=\"Rustにする.html\">Rust にする</a></p>"),
            "{go}"
        );
        assert!(
            go.contains("<nav class=\"decision-nav\"><div class=\"prev\"></div><div class=\"next\"><p class=\"decision-nav-label\">新しい決定 →</p><div class=\"boxes\"><a class=\"decision-box\" href=\"Rustにする.html\">Rust にする</a></div></div></nav>\n<p class=\"note\">カーソルキーの ← と → でも移れる。</p>"),
            "{go}"
        );
        assert!(
            go.contains("<a href=\"../../ADR.html\" aria-current=\"page\">ADR</a>"),
            "{go}"
        );
        assert_headings(&go);
        let rust = read("ADR/言語/Rustにする.html");
        assert!(
            rust.contains("<h2 id=\"選ばなかった案\">選ばなかった案</h2>"),
            "{rust}"
        );
        assert!(rust.contains("<td>Zig</td>"), "{rust}");
        // 表は、横にスクロールできる包みに入れる
        assert!(
            rust.contains("<div class=\"table-scroll\"><table>"),
            "{rust}"
        );
        // 打ち消されていない決定には、打ち消されたことを出さない
        assert!(!rust.contains("class=\"status\""), "{rust}");
        // 前後のカードは本文の後に置き、カーソルキーで移れるようにする。id と日付は出さない
        assert!(
            rust.find("<td>Zig</td>").unwrap() < rust.find("<nav class=\"decision-nav\">").unwrap(),
            "{rust}"
        );
        assert!(!rust.contains("2026-03-01"), "{rust}");
        assert!(
            rust.contains("<nav class=\"decision-nav\"><div class=\"prev\"><p class=\"decision-nav-label\">← 過去の決定</p><div class=\"boxes\"><a class=\"decision-box superseded\" href=\"Goにする.html\">Go にする</a></div></div><div class=\"next\"></div></nav>"),
            "{rust}"
        );
        assert_headings(&rust);

        // Slot のページは、h1 が 1 つで、見出しの段を飛ばさない
        for slot in &slot::load(&project).unwrap() {
            let html = read(&page_path(slot));
            assert_headings(&html);
        }

        run(&project, &out).unwrap();
        assert_eq!(read(PAGE), index);
        assert_eq!(read(DETAIL_PAGE), detail);
        assert_eq!(read(ADR_PAGE), adr_index);
        assert_eq!(read("運用/費用.html"), cost);
    }

    /// h1 が 1 つだけで先頭にあり、見出しが段を飛ばさない（h1 の次に h3 などが来ない）こと。
    fn assert_headings(html: &str) {
        let levels: Vec<u8> = html
            .match_indices("<h")
            .filter_map(|(i, _)| {
                let c = html[i + 2..].chars().next()?;
                let next = html[i + 3..].chars().next()?;
                (c.is_ascii_digit() && (next == '>' || next == ' ')).then(|| c as u8 - b'0')
            })
            .collect();
        assert_eq!(levels.first(), Some(&1), "{html}");
        assert_eq!(levels.iter().filter(|l| **l == 1).count(), 1, "{html}");
        for pair in levels.windows(2) {
            assert!(pair[1] <= pair[0] + 1, "{levels:?}: {html}");
        }
    }

    #[test]
    fn builds_pdr_pages_like_adr_pages() {
        let (dir, project) = project(&[
            (
                "docv/adr/go.md",
                "---\ngroup: 言語\ndate: 2026-01-01\n---\n\n# Go にする\n",
            ),
            (
                "docv/pdr/free.md",
                "---\ngroup: 料金\ndate: 2026-01-01\n---\n\n# 無料で始める\n",
            ),
            (
                "docv/pdr/paid.md",
                "---\ngroup: 料金\ndate: 2026-02-01\nsupersedes: [free]\n---\n\n# 有料のプランを足す\n\n## 見直す条件\n\n解約が増えたら見直す\n",
            ),
            (
                "docv/free/a.md",
                "---\nmission: a\n---\n\n# A\n\n[料金](../pdr/paid.md)\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();

        // サイドバーの ADR の下に PDR を置く
        let index = read(PAGE);
        assert!(
            index.contains("<li><a href=\"ADR.html\">ADR</a></li>\n<li><a href=\"PDR.html\">PDR</a></li>\n<li class=\"separator\""),
            "{index}"
        );
        // PDR.html も、まとまりごとに決定のつながりの図を置く
        let pdr_index = read("PDR.html");
        assert!(
            pdr_index.contains("<title>PDR - demo</title>"),
            "{pdr_index}"
        );
        assert!(
            pdr_index.contains("<main>\n<h1>Product Decision Record</h1>\n<h2 id=\"料金\" class=\"group\">料金</h2>"),
            "{pdr_index}"
        );
        assert!(
            pdr_index.contains("<a class=\"decision-box superseded\" href=\"PDR/料金/無料で始める.html\">無料で始める</a><span class=\"arrow\">→</span><a class=\"decision-box\" href=\"PDR/料金/有料のプランを足す.html\">"),
            "{pdr_index}"
        );
        assert!(
            pdr_index.contains("<a href=\"PDR.html\" aria-current=\"page\">PDR</a>"),
            "{pdr_index}"
        );
        // 決定 1 つのページは PDR/<group>/<title>.html。題の左に、PDR のアイコンを付ける
        let paid = read("PDR/料金/有料のプランを足す.html");
        assert!(
            paid.contains("<title>有料のプランを足す - PDR - demo</title>"),
            "{paid}"
        );
        assert!(
            paid.contains("<a href=\"../../PDR.html\" aria-current=\"page\">PDR</a>"),
            "{paid}"
        );
        assert!(paid.contains("<path d=\"M12 22V3\"/>"), "{paid}");
        assert!(
            paid.contains("<p class=\"decision-nav-label\">← 過去の決定</p><div class=\"boxes\"><a class=\"decision-box superseded\" href=\"無料で始める.html\">"),
            "{paid}"
        );
        // Free ファイルからのリンクと検索の索引も、PDR のページに向く。索引は ADR の後に PDR を並べる
        let a = read("プロジェクト詳細/docv/free/a.html");
        assert!(a.contains("href=\"../../../PDR/"), "{a}");
        let search = read(SEARCH_PAGE);
        let adr = search.find("\"path\":\"docv/adr/go.md\"").unwrap();
        let pdr = search
            .find("\"path\":\"docv/pdr/free.md\",\"href\":\"PDR/料金/無料で始める.html\"")
            .unwrap();
        assert!(adr < pdr, "{search}");
    }

    #[test]
    fn does_not_build_pdr_page_without_pdr() {
        // PDR がなければ、サイドバーに出さず、PDR.html も作らない
        let (dir, project) = project(&[(
            "docv/adr/go.md",
            "---\ndate: 2026-01-01\n---\n\n# Go にする\n",
        )]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        assert!(!out.join("PDR.html").exists());
        assert!(!fs::read_to_string(out.join(PAGE)).unwrap().contains("PDR"));
    }

    #[test]
    fn links_slot_and_adr_files_to_their_pages() {
        let (dir, project) = project(&[
            (
                "docv/free/a.md",
                "---\nmission: q\n---\n# A\n\n[s](../slot/system/system.md#部品) [d](../adr/go.md) [n](<my notes/b c.md>)\n",
            ),
            ("docv/free/my notes/b c.md", "---\nmission: n\n---\n# B\n"),
            ("docv/slot/system/system.md", "# システム仕様\n"),
            (
                "docv/adr/go.md",
                "---\ngroup: 言語\ndate: 2026-01-01\n---\n\n# Go にする\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        // つながりのない決定だけなら、レーンを作らない
        let adr_index = fs::read_to_string(out.join(ADR_PAGE)).unwrap();
        assert!(!adr_index.contains("class=\"lanes\""), "{adr_index}");
        assert!(
            adr_index.contains("<div class=\"decision-grid\"><a class=\"decision-box\""),
            "{adr_index}"
        );
        let page = fs::read_to_string(out.join("プロジェクト詳細/docv/free/a.html")).unwrap();
        // Markdown のリンクは、パーセントエンコードして出る。読めるように戻して比べる
        let bytes = page.as_bytes();
        let mut decoded = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            match (bytes[i], page.get(i + 1..i + 3)) {
                (b'%', Some(hex)) if u8::from_str_radix(hex, 16).is_ok() => {
                    decoded.push(u8::from_str_radix(hex, 16).unwrap());
                    i += 3;
                }
                (b, _) => {
                    decoded.push(b);
                    i += 1;
                }
            }
        }
        let page = String::from_utf8(decoded).unwrap();
        // ページのあるファイルは、出力先の中の相対パスで向ける
        assert!(
            page.contains("href=\"../../../システム/システム仕様.html#部品\""),
            "{page}"
        );
        assert!(
            page.contains("href=\"../../../ADR/言語/Goにする.html\""),
            "{page}"
        );
        // Free ファイルのページは、ディレクトリとファイルの名前から空白を除いて置く
        assert!(page.contains("href=\"mynotes/bc.html\""), "{page}");
        assert!(
            out.join("プロジェクト詳細/docv/free/mynotes/bc.html")
                .is_file()
        );
        let detail = fs::read_to_string(out.join(DETAIL_PAGE)).unwrap();
        assert!(
            detail.contains("<a href=\"プロジェクト詳細/docv/free/mynotes/bc.html\">B</a>"),
            "{detail}"
        );
    }

    #[test]
    fn does_not_link_files_without_page() {
        let (dir, project) = project(&[
            ("docv/free/a.md", "---\nmission: q\n---\n# A\n"),
            (
                "docv/slot/system/system.md",
                "# システム仕様\n\n[memo](../../../notes.md) ![図](../../../a.svg) [設定](../../../docv.yaml)\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        let page = fs::read_to_string(out.join("システム/システム仕様.html")).unwrap();
        let detail = fs::read_to_string(out.join(DETAIL_PAGE)).unwrap();
        // ページのないファイルは、リンクにせず文字だけを出す。画像は元のファイルを相対パスで読む
        assert!(page.contains("<p>memo <img src=\"../../a.svg\""), "{page}");
        assert!(!page.contains("notes.md"), "{page}");
        // docv.yaml へのリンクは、現在の設定のページに向ける
        assert!(
            page.contains(
                "<a href=\"../%E7%8F%BE%E5%9C%A8%E3%81%AE%E8%A8%AD%E5%AE%9A.html\">設定</a>"
            ),
            "{page}"
        );
        assert!(
            detail.contains("href=\"プロジェクト詳細/docv/free/a.html\""),
            "{detail}"
        );
        // ADR がないときは、ADR.html を作らず、サイドバーにも出さない。README.md がないときも同じ
        assert!(!out.join(ADR_PAGE).exists());
        assert!(!detail.contains("ADR.html"), "{detail}");
        assert!(!out.join(README_PAGE).exists());
        assert!(!detail.contains("README.html"), "{detail}");
        // 出力先に .gitignore は置かない
        assert!(!out.join(".gitignore").exists());
    }

    #[test]
    fn builds_readme_page_at_top_of_sidebar() {
        let (dir, project) = project(&[
            (
                "README.md",
                "# Demo\n\n概要\n\n## 使い方\n\n[仕様](docv/slot/system/system.md) [索引](docv/README.md) [コード](src/main.rs)\n",
            ),
            (
                "docv/slot/system/system.md",
                "# システム仕様\n\n[README](../../../README.md#使い方)\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        let readme = fs::read_to_string(out.join(README_PAGE)).unwrap();
        // README.md の h1 をページの h1 にし、見出しは h2 から。GitHub と同じ形のアンカーを付ける
        assert!(
            readme.contains(
                "<section class=\"doc\">\n<h1>Demo</h1>\n<p>概要</p>\n<h2 id=\"使い方\">使い方</h2>"
            ),
            "{readme}"
        );
        assert!(
            readme.contains("<li><a href=\"README.html\" aria-current=\"page\">README</a></li>\n<li><a href=\"index.html\">Slot</a></li>"),
            "{readme}"
        );
        // サイドバーの一番下に、作った docv の版を出す
        assert!(
            readme.contains(&format!(
                "</ul>\n<p class=\"generated\">Generated by <a href=\"https://github.com/suzuki-hoge/docv\">docv</a> <a href=\"https://github.com/suzuki-hoge/docv/tree/v{v}\">{v}</a></p>\n</nav>",
                v = env!("CARGO_PKG_VERSION")
            )),
            "{readme}"
        );
        // README のページには、元のファイルのパスを出さない
        assert!(!readme.contains("class=\"source\""), "{readme}");
        // 本文のリンクは、ほかのページと同じく、ページのあるファイルにだけ張る
        assert!(
            readme.contains("<a href=\"%E3%82%B7%E3%82%B9%E3%83%86%E3%83%A0/%E3%82%B7%E3%82%B9%E3%83%86%E3%83%A0%E4%BB%95%E6%A7%98.html\">仕様</a> 索引 コード"),
            "{readme}"
        );
        // ほかのページからの README.md へのリンクは、README のページに向ける
        let system = fs::read_to_string(out.join("システム/システム仕様.html")).unwrap();
        assert!(
            system.contains("<a href=\"../README.html#%E4%BD%BF%E3%81%84%E6%96%B9\">README</a>"),
            "{system}"
        );
        assert!(
            system.contains("<li><a href=\"../README.html\">README</a></li>"),
            "{system}"
        );
    }

    #[test]
    fn refuses_to_clear_foreign_directory() {
        let (dir, project) = project(&[]);
        let out = dir.path().join("docs");
        write_file(dir.path(), "docs/keep.txt", "keep");
        let error = run(&project, &out).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("docv が作ったディレクトリではない")
        );
        assert!(out.join("keep.txt").is_file());
    }

    #[test]
    fn builds_each_page_of_a_split_slot_with_tabs() {
        let (dir, project) = project(&[
            (
                "docv/slot/service/service/user.md",
                "---\norder: 1\n---\n\n# 利用者向け機能\n\n## 注文\n\n[取り消し](admin.md#取り消し)\n",
            ),
            (
                "docv/slot/service/service/admin.md",
                "---\norder: 2\n---\n\n# 管理者向け機能\n\n## 取り消し\n\n本文\n",
            ),
            (
                "docv/free/a.md",
                "---\nmission: a\n---\n\n# A\n\n[管理](../slot/service/service/admin.md)\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();

        // ページを 1 つずつ、Slot の名前のディレクトリの下に、ページの名前で置く。カードは先頭のページを開く
        let index = read(PAGE);
        assert!(
            index.contains("<a class=\"card\" href=\"サービス/サービス仕様/利用者向け機能.html\">"),
            "{index}"
        );
        // ページに分けた Slot のカードには、「未作成」と同じ位置に、ページの数を書く
        assert!(
            index.contains("</svg>サービス仕様<span class=\"state\">2 tabs</span></a>"),
            "{index}"
        );
        let user = read("サービス/サービス仕様/利用者向け機能.html");
        assert!(
            user.contains("<title>利用者向け機能 - サービス仕様 - demo</title>"),
            "{user}"
        );
        // Slot の名前の下に、すべてのページへのタブを order の順に並べ、開いているページに印を付ける
        assert!(
            user.contains("</svg>サービス仕様</h1>\n<nav class=\"page-tabs\"><a href=\"利用者向け機能.html\" aria-current=\"page\">利用者向け機能</a><a href=\"管理者向け機能.html\">管理者向け機能</a></nav>"),
            "{user}"
        );
        assert!(
            user.contains("<a href=\"../../index.html\" aria-current=\"page\">Slot</a>"),
            "{user}"
        );
        // 分けたページには、そのページの元のファイルのパスを出す
        assert!(
            user.contains("<p class=\"source\"><span class=\"mission\">外から見た機能と、それぞれの振る舞いと決まり</span><code>docv/slot/service/service/user.md</code><button type=\"button\" class=\"copy\" data-path=\"docv/slot/service/service/user.md\" title=\"パスをコピーする\" aria-label=\"パスをコピーする\" hidden>"),
            "{user}"
        );
        // ページの h1 は本文に出さない。目次と、ほかのページへのリンクは、ページごとに効く
        assert!(!user.contains("<h1>利用者向け機能"), "{user}");
        assert!(
            user.contains("<li class=\"h2\"><a href=\"#注文\">注文</a></li>"),
            "{user}"
        );
        assert!(
            user.contains("<a href=\"%E7%AE%A1%E7%90%86%E8%80%85%E5%90%91%E3%81%91%E6%A9%9F%E8%83%BD.html#%E5%8F%96%E3%82%8A%E6%B6%88%E3%81%97\">取り消し</a>"),
            "{user}"
        );
        let admin = read("サービス/サービス仕様/管理者向け機能.html");
        assert!(
            admin.contains(
                "<a href=\"管理者向け機能.html\" aria-current=\"page\">管理者向け機能</a>"
            ),
            "{admin}"
        );
        // Free ファイルからのリンクも、そのページに向く
        let a = read("プロジェクト詳細/docv/free/a.html");
        assert!(
            a.contains("href=\"../../../%E3%82%B5%E3%83%BC%E3%83%93%E3%82%B9/%E3%82%B5%E3%83%BC%E3%83%93%E3%82%B9%E4%BB%95%E6%A7%98/%E7%AE%A1%E7%90%86%E8%80%85%E5%90%91%E3%81%91%E6%A9%9F%E8%83%BD.html\">管理</a>"),
            "{a}"
        );
        // 検索の索引も、ページごとに持つ
        let search = read(SEARCH_PAGE);
        assert!(
            search.contains("{\"path\":\"docv/slot/service/service/admin.md\",\"href\":\"サービス/サービス仕様/管理者向け機能.html\""),
            "{search}"
        );
    }

    #[test]
    fn links_css_and_js_files_from_every_page() {
        let (dir, project) = project(&[
            ("docv/free/spec/a.md", "---\nmission: q\n---\n# A\n"),
            ("docv/slot/service/install.md", "# インストール\n"),
            (
                "docv/adr/go.md",
                "---\ngroup: 言語\ndate: 2026-01-01\n---\n\n# Go にする\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        // CSS と JS は、出力先の直下に、assets と同じ中身で書く
        assert_eq!(fs::read_to_string(out.join("docv.css")).unwrap(), CSS);
        assert_eq!(fs::read_to_string(out.join("docv.js")).unwrap(), JS);
        // 各ページは、ページからの相対パスで読む
        for (page, prefix) in [
            ("index.html", ""),
            ("サービス/インストール.html", "../"),
            ("ADR/言語/Goにする.html", "../../"),
            ("プロジェクト詳細/docv/free/spec/a.html", "../../../../"),
        ] {
            let html = fs::read_to_string(out.join(page)).unwrap();
            assert!(
                html.contains(&format!("<link rel=\"stylesheet\" href=\"{prefix}docv.css\">\n<script src=\"{prefix}docv.js\" defer></script>\n</head>")),
                "{page}: {html}"
            );
        }
        // ページの中には CSS を書かない。JS も、検索のページの索引のほかは書かない
        let mut dirs = vec![out.clone()];
        while let Some(d) = dirs.pop() {
            for entry in fs::read_dir(&d).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|e| e == "html") {
                    let html = fs::read_to_string(&path).unwrap();
                    assert!(!html.contains("<style"), "{}", path.display());
                    let inline = html.matches("<script>").count();
                    let expected = usize::from(path.ends_with(SEARCH_PAGE));
                    assert_eq!(inline, expected, "{}", path.display());
                }
            }
        }
        let search = fs::read_to_string(out.join(SEARCH_PAGE)).unwrap();
        assert!(search.contains("<script>var DOCV_SEARCH=[{"), "{search}");
    }

    #[test]
    fn rebuilds_everything_but_the_marker() {
        let (dir, project) = project(&[]);
        let out = dir.path().join("output/docv/html");
        run(&project, &out).unwrap();
        write_file(&out, "old/a.html", "old");
        write_file(&out, "old.html", "old");
        // 途中で止まって index.html などがない出力先でも、目印があれば作り直せる
        fs::remove_file(out.join(PAGE)).unwrap();
        run(&project, &out).unwrap();
        assert!(!out.join("old").exists());
        assert!(!out.join("old.html").exists());
        assert!(out.join(PAGE).is_file());
    }

    #[test]
    fn writes_one_tab_on_the_card_of_a_slot_with_one_page() {
        let mission = slot::def("architecture").unwrap().mission;
        let (dir, project) = project(&[
            (
                "docv/slot/guide/architecture/web.md",
                &format!("---\norder: 1\nmission: {mission}\n---\n\n# Web\n"),
            ),
            // ページに分けられる Slot は、分けていなくても 1 tab と書く
            ("docv/slot/guide/design.md", "# 設計・実装\n"),
            // ページに分けられない Slot には書かない
            ("docv/slot/guide/doc-guide.md", "# Doc ガイド\n"),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        let index = fs::read_to_string(out.join(PAGE)).unwrap();
        assert!(
            index.contains("<a class=\"card\" href=\"開発者向けガイド/アーキテクチャ/Web.html\">"),
            "{index}"
        );
        assert!(
            index.contains("</svg>アーキテクチャ<span class=\"state\">1 tab</span></a>"),
            "{index}"
        );
        assert!(
            index.contains("</svg>設計・実装<span class=\"state\">1 tab</span></a>"),
            "{index}"
        );
        assert!(index.contains("</svg>Doc ガイド</a>"), "{index}");
    }

    #[test]
    fn builds_each_page_of_the_develop_commands_like_other_split_slots() {
        let mission = slot::def("develop").unwrap().mission;
        let (dir, project) = project(&[
            (
                "docv/slot/guide/develop/web.md",
                &format!(
                    "---\norder: 2\nmission: {mission}\n---\n\n# Web\n\n| すること | コマンド |\n| --- | --- |\n| テストを実行する | `npm test` |\n\n## テストを実行する\n\n```sh\nnpm test\n```\n"
                ),
            ),
            (
                "docv/slot/guide/develop/api.md",
                &format!("---\norder: 1\nmission: {mission}\n---\n\n# API\n"),
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out).unwrap();
        let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();

        // カードは先頭のページを開く
        let index = read(PAGE);
        assert!(
            index.contains("<a class=\"card\" href=\"開発者向けガイド/開発コマンド/API.html\">"),
            "{index}"
        );
        // h1 をページの名前にして、タブを order の順に並べる
        let web = read("開発者向けガイド/開発コマンド/Web.html");
        assert!(
            web.contains("<title>Web - 開発コマンド - demo</title>"),
            "{web}"
        );
        assert!(
            web.contains("</svg>開発コマンド</h1>\n<nav class=\"page-tabs\"><a href=\"API.html\">API</a><a href=\"Web.html\" aria-current=\"page\">Web</a></nav>"),
            "{web}"
        );
        // 中身は、ほかの Slot と同じく Markdown のまま出す。先頭に目次の箇条書きは作らない
        assert!(
            web.contains("<h2 id=\"テストを実行する\">テストを実行する</h2>"),
            "{web}"
        );
        assert!(!web.contains("class=\"toc\""), "{web}");
        assert!(
            web.contains("<p class=\"source\"><span class=\"mission\">テストやビルドなど、開発者が実行するコマンド</span><code>docv/slot/guide/develop/web.md</code>"),
            "{web}"
        );
    }

    /// 本文の Markdown の表を、見出しの行で 1 つずつ作る。
    fn table(header: &str, rows: &[&str]) -> String {
        let mut text = format!(
            "{header}\n|{}\n",
            " --- |".repeat(header.matches('|').count() - 1)
        );
        for row in rows {
            text.push_str(row);
            text.push('\n');
        }
        text.push('\n');
        text
    }

    fn markdown_html(text: &str) -> String {
        let mut out = String::new();
        html::push_html(&mut out, Parser::new_ext(text, markdown::OPTIONS));
        out
    }

    fn manual_has_aligned(html: &str) -> bool {
        html.contains("class=\"aligned\"")
    }

    #[test]
    fn aligns_columns_of_tables_with_the_same_header() {
        let body = markdown_html(&format!(
            "{}{}{}",
            table(
                "| 用語 | 英語 | 意味 |",
                &["| 注文 | Order | 利用者が、カートの商品を買うと決めたもの |"]
            ),
            table(
                "| 用語 | 英語 | 意味 |",
                &[
                    "| 決済 | Payment | 注文の代金を受け取ること |",
                    "| 返金 | `refund`<br>Refund | 決済を取り消すこと |"
                ]
            ),
            table("| 名前 | 用途 |", &["| Rust | CLI |"]),
        ));
        let aligned = align_tables(&body);
        // 短い列は、どちらの表のセルも折り返さない幅にする。全角は 1em、ほかは 0.6em、コードは余白の分を足し、セルの余白 1.4em を足す
        // 用語: 全角 2 文字 = 2.0em。英語: `refund` は 6 × 0.6 + 0.6 = 4.2em、Payment は 7 × 0.6 = 4.2em
        let colgroup = "<table class=\"aligned\"><colgroup><col style=\"width: 3.4em\"><col style=\"width: 5.6em\"><col></colgroup>";
        assert_eq!(aligned.matches(colgroup).count(), 2, "{aligned}");
        assert!(
            aligned.contains("<thead><tr><th class=\"short\">用語</th><th class=\"short\">英語</th><th>意味</th></tr></thead>"),
            "{aligned}"
        );
        assert!(
            aligned.contains("<tr><td class=\"short\">返金</td><td class=\"short\"><code>refund</code><br>Refund</td><td>決済を取り消すこと</td></tr>"),
            "{aligned}"
        );
        // 見出しの行が同じ表のない表は、そのまま出す
        assert!(
            aligned.contains("<table><thead><tr><th>名前</th><th>用途</th></tr></thead>"),
            "{aligned}"
        );
        // 1 つしかなければ、どれも変えない。毎回同じ出力にする
        let one = markdown_html(&table("| 用語 | 英語 | 意味 |", &["| a | b | c |"]));
        assert_eq!(align_tables(&one), one);
        assert_eq!(align_tables(&body), aligned);
    }

    #[test]
    fn leaves_the_widest_column_flexible() {
        // いちばん広い列が残りの幅を使う。同じ幅なら右の列
        let body = markdown_html(&format!(
            "{}{}",
            table("| a | b | c |", &["| とても長い説明の文 | x | y |"]),
            table("| a | b | c |", &["| z | 同じ幅 | 同じ幅 |"]),
        ));
        let aligned = align_tables(&body);
        assert!(
            aligned.contains(
                "<colgroup><col><col style=\"width: 4.4em\"><col style=\"width: 4.4em\"></colgroup>"
            ),
            "{aligned}"
        );
        let body = markdown_html(&format!(
            "{}{}",
            table("| a | b |", &["| 同じ | 同じ |"]),
            table("| a | b |", &["| x | y |"]),
        ));
        assert!(
            align_tables(&body).contains("<colgroup><col style=\"width: 3.4em\"><col></colgroup>"),
            "{}",
            align_tables(&body)
        );
    }

    #[test]
    fn links_bare_urls() {
        let html = |text: &str| {
            let mut out = String::new();
            let events = link_bare_urls(
                TextMergeStream::new(Parser::new_ext(text, markdown::OPTIONS)).collect(),
            );
            html::push_html(&mut out, events.into_iter());
            out
        };
        let a = |url: &str| format!("<a href=\"{url}\">{url}</a>");
        let url = "https://github.com/suzuki-hoge/docv/blob/main/assets/desc.md";
        assert_eq!(
            html(&format!("- 詳細は {url} を参照 ( 内容は同じ )\n")),
            format!(
                "<ul>\n<li>詳細は {} を参照 ( 内容は同じ )</li>\n</ul>\n",
                a(url)
            )
        );
        // URL は、ASCII でない文字の前で終わる。末尾の句読点と、対になっていない ) は含めない
        assert_eq!(
            html(
                "詳細はhttps://example.com/a_b_c。(http://example.com/x(1)) と https://example.com.\n"
            ),
            format!(
                "<p>詳細は{}。({}) と {}.</p>\n",
                a("https://example.com/a_b_c"),
                a("http://example.com/x(1)"),
                a("https://example.com")
            )
        );
        // すでにリンクのもの、コード、英数字に続くもの、scheme だけのものは変えない
        let kept = "[https://example.com](https://example.com) <https://example.com/b> `https://example.com/c` xhttps://example.com https://\n\n```\nhttps://example.com/d\n```\n";
        assert_eq!(
            html(kept),
            format!(
                "<p>{} {} <code>https://example.com/c</code> xhttps://example.com https://</p>\n<pre><code>https://example.com/d\n</code></pre>\n",
                a("https://example.com"),
                a("https://example.com/b")
            )
        );
        assert_eq!(
            html("<a href=\"https://example.com\">https://example.com</a>\n"),
            "<p><a href=\"https://example.com\">https://example.com</a></p>\n"
        );
    }

    #[test]
    fn highlights_fenced_code_blocks() {
        let html = |text: &str| {
            let mut out = String::new();
            let events: Vec<Event> = Parser::new_ext(text, markdown::OPTIONS).collect();
            html::push_html(&mut out, highlight_code_blocks(events).into_iter());
            out
        };
        let yaml = html("```yaml\nkey: 1\n```\n");
        assert!(
            yaml.starts_with("<pre><code class=\"language-yaml\"><span style=\"color:"),
            "{yaml}"
        );
        // 言語がないブロックと、知らない言語のブロックは、色を付けない
        assert_eq!(
            html("```\nkey: 1\n```\n"),
            "<pre><code>key: 1\n</code></pre>\n"
        );
        assert_eq!(
            html("```nolang\n<a>\n```\n"),
            "<pre><code class=\"language-nolang\">&lt;a&gt;\n</code></pre>\n"
        );
    }
}
