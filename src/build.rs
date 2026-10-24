//! docv build。人間向けの HTML と、設定で有効なら Slot のページの JPG を生成する。docv/system/system.md の「HTML の作り」。
//!
//! トップは 5 ページに分け、サイドバーで行き来する。
//! index.html（プロジェクト概要）に Slot ファイルのページへのカード、ADR.html に決定のつながりの図、
//! プロジェクト詳細.html に Free ファイルのツリー、現在の設定.html に docv.yaml のすべての項目と今の値、
//! 検索.html に Slot ファイル、ADR、Free ファイルの中身を探す入力欄を置く。
//! Slot は 1 つずつ独立したページにし、`<まとまり>/<Slot>.html` に置く。
//! 分けたサービス仕様は、ページごとに `<まとまり>/<Slot>/<ページの h1>.html` に置き、ページへのリンクをタブにして並べる。
//! 画像にしても読めるように、折りたたみは使わない。OS ごとの手順のタブは、印刷の表示（JPG）ではすべて並べる。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, HeadingLevel, Parser, Tag, TagEnd, html};
use serde::Serialize;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::adr::{self, Adr};
use crate::config_view::{self, Cell};
use crate::jpg;
use crate::markdown::{self, relative_path, slug};
use crate::meta;
use crate::slot::{self, Content, DEFS, Group, Slot};
use crate::textlint;
use crate::workspace::Project;

const STYLE: &str = include_str!("../assets/style.css");
const SEARCH: &str = include_str!("../assets/search.js");

/// docv が作った出力先であることを示すファイル。これがない出力先は消さない。
const MARKER: &str = ".docv";
const MARKER_TEXT: &str = "docv build の出力。このディレクトリは docv build のたびに作り直す。\n";

/// トップの 5 ページ。サイドバーに、この順で並べる。
pub const PAGE: &str = "index.html";
const ADR_PAGE: &str = "ADR.html";
const DETAIL_PAGE: &str = "プロジェクト詳細.html";
const CONFIG_PAGE: &str = "現在の設定.html";
const SEARCH_PAGE: &str = "検索.html";

/// サイドバーの項目。描いているページがどれに属するかを示すのにも使う。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Nav {
    Overview,
    Adr,
    Detail,
    Config,
    Search,
}

impl Nav {
    fn label(self) -> &'static str {
        match self {
            Nav::Overview => "プロジェクト概要",
            Nav::Adr => "ADR",
            Nav::Detail => "プロジェクト詳細",
            Nav::Config => "現在の設定",
            Nav::Search => "検索",
        }
    }

    fn path(self) -> &'static str {
        match self {
            Nav::Overview => PAGE,
            Nav::Adr => ADR_PAGE,
            Nav::Detail => DETAIL_PAGE,
            Nav::Config => CONFIG_PAGE,
            Nav::Search => SEARCH_PAGE,
        }
    }
}

/// Free ファイルのページを置くディレクトリ（出力先からの相対パス）。Free ファイルのパスの .md を .html にして、この下に置く。
const DETAIL_DIR: &str = "プロジェクト詳細";

/// ADR のページを置くディレクトリ（出力先からの相対パス）。決定 1 つを 1 ページにし、`ADR/<まとまり>/<題>.html` に置く。
const ADR_DIR: &str = "ADR";

/// 決定のページで、カーソルキーの ← と → を、前後の決定のカードへのリンクにする。
/// 入力欄にいるときと、修飾キーを押しているときは、ブラウザの動きのままにする。
const ADR_KEYS: &str = "document.addEventListener('keydown',function(e){if(e.altKey||e.ctrlKey||e.metaKey||e.shiftKey||/^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName))return;var side={ArrowLeft:'prev',ArrowRight:'next'}[e.key];var a=side&&document.querySelector('.adr-nav .'+side+' a');if(a)location.href=a.href;});";

/// プロジェクト詳細のタグのトグル。選んだタグのどれかを持つ Free ファイルの行と、その上のディレクトリの行だけを出す。
/// すべて選んでいるときに押したタグは、そのタグだけを選ぶ。それ以外のときは押したタグを切り替え、1 つも残らなければすべて選び直す。
/// 絞り込んでも列の幅が変わらないように、すべての行を出した状態のファイルとタグの列の幅で固定する。
const TAG_FILTER: &str = "(function(){var tb=document.querySelector('table.docs');var hs=tb.tHead.rows[0].cells;var w0=hs[0].offsetWidth,w2=hs[2].offsetWidth;hs[0].style.width=w0+'px';hs[2].style.width=w2+'px';tb.style.tableLayout='fixed';var bs=[].slice.call(document.querySelectorAll('.tag-filter button'));var files=[].slice.call(document.querySelectorAll('table.docs tr[data-path]'));var dirs=[].slice.call(document.querySelectorAll('table.docs tr[data-dir]'));function apply(){var on=bs.filter(function(b){return b.getAttribute('aria-pressed')==='true';}).map(function(b){return b.getAttribute('data-tag');});var all=on.length===bs.length;files.forEach(function(r){var t=JSON.parse(r.getAttribute('data-tags'));r.hidden=!all&&!t.some(function(x){return on.indexOf(x)>=0;});});dirs.forEach(function(d){var p=d.getAttribute('data-dir')+'/';d.hidden=!files.some(function(r){return !r.hidden&&r.getAttribute('data-path').indexOf(p)===0;});});}function pressed(b){return b.getAttribute('aria-pressed')==='true';}function set(b,on){b.setAttribute('aria-pressed',on?'true':'false');}bs.forEach(function(b){b.addEventListener('click',function(){if(bs.every(pressed)){bs.forEach(function(x){set(x,x===b);});}else{set(b,!pressed(b));if(!bs.some(pressed)){bs.forEach(function(x){set(x,true);});}}apply();});});})();";

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

/// docv build。out（dir.html）の中を作り直す。image（dir.image）を書いていれば、Slot のページを JPG にしてそこに置く。
/// JPG は Slot のページごとに 1 枚。見つけやすいように、まとまりと Slot の表示名をパスにする。
/// Markdown から作るページ（md の Slot、ADR、Free ファイル）は、GitHub で md をそのまま見られるので JPG にしない。
pub fn run(project: &Project, out: &Path, image: Option<&Path>) -> Result<()> {
    let out = prepare_out(out)?;
    let slots = render(project, &out)?;
    write(&out.join(MARKER), MARKER_TEXT)?;
    if let Some(image) = image {
        let image = prepare_out(image)?;
        let shots: Vec<(String, String)> = slots
            .iter()
            .filter(|s| !matches!(s.content, Content::Markdown(_)))
            .map(|s| (page_path(s), jpg_path(s)))
            .collect();
        jpg::capture(&out, &image, &shots)?;
        write(&image.join(MARKER), MARKER_TEXT)?;
    }
    Ok(())
}

/// out にすべてのページを書き、読み込んだ Slot を返す。
fn render(project: &Project, out: &Path) -> Result<Vec<Slot>> {
    let mut pages = Vec::new();
    for path in project.managed()? {
        pages.push(read_page(project, &path)?);
    }
    let slots = slot::load(project)?;
    let adrs = adr::load(project)?;

    let site = Site {
        project,
        out,
        dir: RefCell::new(out.to_path_buf()),
        pages: &pages,
        slots: &slots,
        adrs: &adrs,
    };
    write(&out.join(PAGE), &site.render_index())?;
    if !adrs.is_empty() {
        write(&out.join(ADR_PAGE), &site.render_adr_index())?;
    }
    write(&out.join(DETAIL_PAGE), &site.render_detail())?;
    write(&out.join(CONFIG_PAGE), &site.render_config())?;
    write(&out.join(SEARCH_PAGE), &site.render_search()?)?;
    for slot in &slots {
        write(&out.join(page_path(slot)), &site.render_slot(slot))?;
    }
    for a in &adrs {
        write(&out.join(adr_page(a)), &site.render_adr(a))?;
    }
    for page in &pages {
        write(&out.join(doc_page(&page.source)), &site.render_doc(page))?;
    }
    Ok(slots)
}

/// Slot のページの、出力先からの相対パス。`サービス仕様/利用マニュアル.html`。
/// 分けた Slot のページは、Slot の名前のディレクトリの下に、ページの名前で置く。`サービス仕様/サービス仕様/管理者向け機能.html`。
fn page_path(slot: &Slot) -> String {
    match &slot.page {
        Some(page) => format!(
            "{}/{}/{}.html",
            slot.def.group.label(),
            slot.def.label,
            file_name(&page.title)
        ),
        None => format!("{}/{}.html", slot.def.group.label(), slot.def.label),
    }
}

/// Slot の JPG の、dir.image からの相対パス。`サービス仕様/用語集.jpg`。
fn jpg_path(slot: &Slot) -> String {
    format!("{}/{}.jpg", slot.def.group.label(), slot.def.label)
}

fn read_page(project: &Project, path: &str) -> Result<Page> {
    let text = fs::read_to_string(project.root.join(path))
        .with_context(|| format!("{path} を読めない"))?;
    let document = markdown::split(&text);
    let meta = meta::read(&document, &markdown::outline(&document));
    Ok(Page {
        source: path.to_string(),
        title: meta.title.unwrap_or_else(|| path.to_string()),
        mission: meta.mission,
        tags: meta.tags,
        body: document.body.clone(),
    })
}

/// Free ファイルのページの、出力先からの相対パス。`プロジェクト詳細/docs/order.html`。
fn doc_page(source: &str) -> String {
    format!(
        "{DETAIL_DIR}/{}.html",
        source.strip_suffix(".md").unwrap_or(source)
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
    pages: &'a [Page],
    slots: &'a [Slot],
    adrs: &'a [Adr],
}

impl Site<'_> {
    /// index.html（プロジェクト概要）。Slot のページへのカード。
    fn render_index(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = String::new();
        self.index(&mut out);
        self.page(&escape(&self.project.title()), Nav::Overview, &out)
    }

    /// ADR.html。決定のつながりの図。
    fn render_adr_index(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = String::new();
        self.adr_lanes(&mut out);
        self.page(
            &format!("ADR - {}", escape(&self.project.title())),
            Nav::Adr,
            &out,
        )
    }

    /// プロジェクト詳細.html。Free ファイルのツリー。
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

    /// 現在の設定.html。docv.yaml のすべての項目と今の値を、1 つの表に並べる。
    fn render_config(&self) -> String {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut out = String::new();
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

    /// 検索.html。入力欄と、探す先の索引。索引は、Slot ファイル、ADR、Free ファイルの順に、中身を行ごとに持つ。
    /// 入力欄は JS が動くときだけ出し、結果は JS が入力欄の下に並べる。
    fn render_search(&self) -> Result<String> {
        *self.dir.borrow_mut() = self.out.to_path_buf();
        let mut entries = Vec::new();
        for s in self.slots {
            entries.push(self.search_entry(&s.path, &[])?);
        }
        for a in self.adrs {
            entries.push(self.search_entry(&a.path, &[])?);
        }
        for p in self.pages {
            entries.push(self.search_entry(&p.source, &p.tags)?);
        }
        // <script> の中に置くので、</script> などで閉じないように < を書き換える。
        let index = serde_json::to_string(&entries)?.replace('<', "\\u003c");
        let body = format!(
            "<h1>検索</h1>\n<form class=\"search\" role=\"search\" hidden><input type=\"search\" placeholder=\"入力した文字列をそのまま含む行を探す\" aria-label=\"検索\" autofocus></form>\n<noscript><p class=\"note\">検索には JavaScript が要る。</p></noscript>\n<p class=\"note\" id=\"search-count\"></p>\n<div id=\"search-results\"></div>\n<script>var DOCV_SEARCH={index};\n{SEARCH}</script>\n"
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
        let _ = writeln!(out, "<h2 class=\"group\">現在の設定</h2>");
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
            "<section class=\"doc\">\n<h1>{}</h1>\n{html}</section>\n",
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

    /// HTML の全体。左にトップの 5 ページへのサイドバーを、右に h2 と h3 の目次を置く。印刷の表示（JPG）では、どちらも出さない。
    fn page(&self, title: &str, current: Nav, body: &str) -> String {
        let mut nav = format!(
            "<nav class=\"sidebar\">\n<p class=\"project\">{}</p>\n<ul>\n",
            escape(&self.project.title())
        );
        for item in [
            Nav::Overview,
            Nav::Adr,
            Nav::Detail,
            Nav::Config,
            Nav::Search,
        ] {
            if item == Nav::Adr && self.adrs.is_empty() {
                continue;
            }
            let href = encode_path(&relative_path(
                &self.dir.borrow(),
                &self.out.join(item.path()),
            ));
            let _ = writeln!(
                nav,
                "<li><a href=\"{href}\"{}>{}</a></li>",
                if item == current {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                item.label()
            );
        }
        nav.push_str("</ul>\n</nav>\n");
        let (body, aside) = outline(body);
        page(title, &nav, &body, &aside)
    }

    /// Slot のページ。分けた Slot のページの題には、ページの名前を前に付ける。
    fn render_slot(&self, slot: &Slot) -> String {
        let path = page_path(slot);
        let dir = Path::new(&path).parent().unwrap_or(Path::new(""));
        *self.dir.borrow_mut() = self.out.join(dir);
        let mut out = String::new();
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
    /// ファイルのない Slot は、リンクにせずグレーで出す。
    fn index(&self, out: &mut String) {
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
                let Some(slot) = self.slots.iter().find(|s| s.def.id == def.id) else {
                    let _ = writeln!(
                        out,
                        "<span class=\"card missing\">{}{}</span>",
                        icon(def.id),
                        def.label
                    );
                    continue;
                };
                let href = encode_path(&page_path(slot));
                let _ = writeln!(
                    out,
                    "<a class=\"card\" href=\"{href}\">{}{}</a>",
                    icon(def.id),
                    def.label
                );
            }
            let _ = writeln!(out, "</div>");
        }
    }

    /// ADR の図。まとまりごとに h2 を置き、決定のつながりを過去から未来へ、左から右に箱と矢印で並べる。
    /// 箱は、その決定のページへのリンク。
    fn adr_lanes(&self, out: &mut String) {
        for group in adr::groups(self.adrs) {
            let members: Vec<&Adr> = self.adrs.iter().filter(|a| a.group() == group).collect();
            let _ = writeln!(
                out,
                "<h2 class=\"group\">{}</h2>\n<div class=\"lanes\">",
                escape(group)
            );
            for lane in adr::lanes(&members) {
                let boxes: Vec<String> = lane.iter().map(|a| self.adr_box(a)).collect();
                let _ = writeln!(
                    out,
                    "<div class=\"lane\">{}</div>",
                    boxes.join("<span class=\"arrow\">→</span>")
                );
            }
            let _ = writeln!(out, "</div>");
        }
    }

    /// 決定 1 つのページ。
    fn render_adr(&self, a: &Adr) -> String {
        *self.dir.borrow_mut() = self.out.join(ADR_DIR).join(file_name(a.group()));
        let mut out = String::new();
        let _ = writeln!(out, "<section class=\"slot\">");
        self.adr_detail(&mut out, a);
        let _ = writeln!(out, "</section>");
        self.page(
            &format!(
                "{} - ADR - {}",
                escape(&a.record.title),
                escape(&self.project.title())
            ),
            Nav::Adr,
            &out,
        )
    }

    /// 決定のページへのリンク。描いているページのディレクトリからの相対パス。
    fn adr_href(&self, a: &Adr) -> String {
        let to = self.out.join(adr_page(a));
        encode_path(&relative_path(&self.dir.borrow(), &to))
    }

    fn adr_box(&self, a: &Adr) -> String {
        format!(
            "<a class=\"adr-box{}\" href=\"{}\">{}</a>",
            if a.superseded_by.is_empty() {
                ""
            } else {
                " superseded"
            },
            self.adr_href(a),
            escape(&a.record.title)
        )
    }

    fn adr_detail(&self, out: &mut String, a: &Adr) {
        let r = &a.record;
        let _ = writeln!(out, "<h1>{}{}</h1>", icon("adr"), escape(&r.title));
        // 本文は、h1（題）を除き、いちばん浅い見出しを h2 にして出す。
        let base = Path::new(&a.path)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let _ = writeln!(out, "{}", self.markdown(&r.body, &base, true, 2));
        // ページの下に、打ち消した過去の決定を ← の左、打ち消された先の新しい決定を → の右に、カードで出す。
        // カーソルキーの ← と → で、それぞれの先頭のカードのページへ移る。
        let boxes = |ids: &[String]| -> String {
            ids.iter()
                .filter_map(|id| self.adrs.iter().find(|x| &x.record.id == id))
                .map(|x| self.adr_box(x))
                .collect()
        };
        let (prev, next) = (boxes(&r.supersedes), boxes(&a.superseded_by));
        if prev.is_empty() && next.is_empty() {
            return;
        }
        let side = |class: &str, html: &str, arrow_first: bool| -> String {
            if html.is_empty() {
                return format!("<div class=\"{class}\"></div>");
            }
            let arrow = format!(
                "<span class=\"arrow\">{}</span>",
                if arrow_first { "←" } else { "→" }
            );
            if arrow_first {
                format!("<div class=\"{class}\">{arrow}{html}</div>")
            } else {
                format!("<div class=\"{class}\">{html}{arrow}</div>")
            }
        };
        let _ = writeln!(
            out,
            "<nav class=\"adr-nav\">{}{}</nav>\n<script>{ADR_KEYS}</script>",
            side("prev", &prev, true),
            side("next", &next, false)
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
            let _ = write!(
                out,
                "<a href=\"{}\"{}>{}</a>",
                self.link(&s.path, "").unwrap_or_default(),
                if s.path == slot.path {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                escape(&page.title)
            );
        }
        let _ = writeln!(out, "</nav>");
    }

    /// Slot ごとの形の中身。
    fn slot_body(&self, slot: &Slot) -> String {
        let mut body = String::new();
        let out = &mut body;
        match &slot.content {
            Content::Procedure(c) => {
                self.tools(out, "前提とされるもの", &c.requirements);
                if slot.def.id == "develop" {
                    self.step_sections(out, slot.def.id, &c.steps);
                } else {
                    self.steps(out, slot.def.id, &c.steps);
                }
            }
            Content::Words(c) => self.words(out, &c.groups),
            Content::Stack(c) => self.stack(out, &c.stack),
            Content::Layout(c) => self.layout(out, &c.entries),
            Content::Env(c) => self.envs(out, &c.envs),
            Content::Cost(c) => {
                let _ = writeln!(out, "<h2>使っているもの</h2>");
                self.cost(out, &c.accepted_items);
                let _ = writeln!(out, "<h2>費用を理由にやっていないこと</h2>");
                self.cost(out, &c.rejected_items);
            }
            Content::Markdown(text) => {
                let base = Path::new(&slot.path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_path_buf();
                // md の Slot はページ全体なので、Free ファイルと同じく見出しに id を付け、ページ内リンクで飛べるようにする。
                let html = self.markdown_with_ids(text, &base, true, 2);
                let _ = writeln!(out, "{html}");
            }
        }
        // YAML の Slot の appendix_markdown は、項目のあとに続けて出す。見出しは h2 から。リンクはルートからの相対パス。
        if let Some(appendix) = slot.content.appendix() {
            let _ = writeln!(out, "{}", self.markdown(appendix, Path::new(""), false, 2));
        }
        body
    }

    fn tools(&self, out: &mut String, title: &str, tools: &[slot::Tool]) {
        if tools.is_empty() {
            return;
        }
        let _ = writeln!(out, "<h2>{title}</h2>");
        self.tool_table(out, tools);
    }

    fn tool_table(&self, out: &mut String, tools: &[slot::Tool]) {
        let _ = writeln!(
            out,
            "<table>\n<thead><tr><th>名前</th><th>版</th><th>何に使うか</th></tr></thead>\n<tbody>"
        );
        for t in tools {
            let _ = writeln!(
                out,
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                name_link(&t.name, t.url.as_deref()),
                escape(t.version.as_deref().unwrap_or("")),
                self.inline_opt(t.purpose.as_deref())
            );
        }
        let _ = writeln!(out, "</tbody>\n</table>");
    }

    /// 手順。番号を付けて上から並べる。
    fn steps(&self, out: &mut String, slot_id: &str, steps: &[slot::Step]) {
        if steps.is_empty() {
            return;
        }
        let _ = writeln!(out, "<h2>手順</h2>\n<ol class=\"steps\">");
        for step in steps {
            let _ = write!(out, "<li><p class=\"step-name\">{}</p>", escape(&step.name));
            self.step_items(out, slot_id, step);
            let _ = writeln!(out, "</li>");
        }
        let _ = writeln!(out, "</ol>");
    }

    /// 順番のない項目（開発コマンド）。先頭に目次を置き、項目ごとに h2 にする。番号は付けない。
    fn step_sections(&self, out: &mut String, slot_id: &str, steps: &[slot::Step]) {
        if steps.is_empty() {
            return;
        }
        let _ = writeln!(out, "<ul class=\"toc\">");
        for step in steps {
            let _ = writeln!(
                out,
                "<li><a href=\"#{}\">{}</a></li>",
                slug(&step.id),
                escape(&step.name)
            );
        }
        let _ = writeln!(out, "</ul>");
        for step in steps {
            let _ = writeln!(
                out,
                "<h2 id=\"{}\">{}</h2>",
                slug(&step.id),
                escape(&step.name)
            );
            self.step_items(out, slot_id, step);
        }
    }

    /// 手順の中身。items が複数あれば os ごとのタブにし、1 つならタブにしない。
    fn step_items(&self, out: &mut String, slot_id: &str, step: &slot::Step) {
        if let [item] = step.items.as_slice() {
            let _ = write!(out, "{}", self.step_markdown(item));
            return;
        }
        if step.items.is_empty() {
            return;
        }
        let name = format!("tab-{slot_id}-{}", slug(&step.id));
        let _ = write!(out, "<div class=\"tabs\">");
        for (i, item) in step.items.iter().enumerate() {
            let _ = write!(
                out,
                "<input type=\"radio\" name=\"{name}\" id=\"{name}-{i}\"{}><label for=\"{name}-{i}\">{}</label>",
                if i == 0 { " checked" } else { "" },
                escape(&os_label(item.os.as_deref().unwrap_or("")))
            );
        }
        // 印刷の表示（JPG）ではタブを隠して中身を並べるので、どの OS の中身かを中にも書く。
        for item in &step.items {
            let _ = write!(
                out,
                "<div class=\"panel\"><p class=\"panel-os\">{}</p>{}</div>",
                escape(&os_label(item.os.as_deref().unwrap_or(""))),
                self.step_markdown(item)
            );
        }
        let _ = write!(out, "</div>");
    }

    fn step_markdown(&self, item: &slot::StepItem) -> String {
        item.markdown
            .as_deref()
            .map(|m| self.md(m))
            .unwrap_or_default()
    }

    /// 用語集。まとまりごとに h2 と「日本語 | 英語 | 意味」の表。
    fn words(&self, out: &mut String, groups: &[slot::WordGroup]) {
        for group in groups {
            let _ = writeln!(
                out,
                "<h2>{}</h2>\n<table class=\"words\">\n<thead><tr><th>日本語</th><th>英語</th><th>意味</th></tr></thead>\n<tbody>",
                escape(&group.name)
            );
            for w in &group.items {
                let _ = writeln!(
                    out,
                    "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape(w.ja.as_deref().unwrap_or("")),
                    escape(w.en.as_deref().unwrap_or("")),
                    self.inline(&w.desc)
                );
            }
            let _ = writeln!(out, "</tbody>\n</table>");
        }
    }

    fn stack(&self, out: &mut String, groups: &[slot::StackGroup]) {
        for group in groups {
            let _ = writeln!(out, "<h2>{}</h2>", escape(&group.group));
            if let Some(image) = &group.image {
                let _ = writeln!(
                    out,
                    "<p><img src=\"{}\" alt=\"\"></p>",
                    escape(&encode_path(&self.file_path(image)))
                );
            }
            if !group.items.is_empty() {
                self.tool_table(out, &group.items);
            }
        }
    }

    /// ディレクトリの Slot。表は常に 1 段だけを出し、入れ子は表を分けて出す。
    /// いちばん上の段の表を先に置き、下の段を持つディレクトリごとに、その下の表を h2 とともに後ろへ並べる。
    /// 下の段を持つディレクトリの名前は、その表へのページ内リンクにする。折りたたまないのは、画像にしても読めるようにするため。
    fn layout(&self, out: &mut String, entries: &[slot::LayoutEntry]) {
        if entries.is_empty() {
            return;
        }
        let mut roots: Vec<TreeNode> = Vec::new();
        for entry in entries {
            let trimmed = entry.path.trim_end_matches('/');
            let names: Vec<&str> = trimmed.split('/').collect();
            let mut nodes = &mut roots;
            for (i, name) in names.iter().enumerate() {
                let at = match nodes.iter().position(|n| n.name == *name) {
                    Some(at) => at,
                    None => {
                        nodes.push(TreeNode {
                            name: name.to_string(),
                            path: names[..=i].join("/"),
                            dir: true,
                            description: None,
                            children: Vec::new(),
                        });
                        nodes.len() - 1
                    }
                };
                if i + 1 == names.len() {
                    let node = &mut nodes[at];
                    node.dir = slot::is_dir(self.project, &entry.path);
                    node.description
                        .get_or_insert_with(|| entry.description.clone());
                }
                nodes = &mut nodes[at].children;
            }
        }
        self.layout_table(out, &roots);
        let mut dirs = Vec::new();
        tree_dirs(&roots, &mut dirs);
        for dir in dirs {
            let _ = writeln!(
                out,
                "<h2 id=\"{}\">{}/</h2>",
                escape(&tree_anchor(&dir.path)),
                escape(&dir.path)
            );
            self.layout_table(out, &dir.children);
        }
    }

    /// ディレクトリの Slot の、1 段の表。
    fn layout_table(&self, out: &mut String, nodes: &[TreeNode]) {
        let _ = writeln!(
            out,
            "<table class=\"tree\">\n<thead><tr><th>名前</th><th>説明</th></tr></thead>\n<tbody>"
        );
        for node in nodes {
            let name = format!("<code>{}</code>", escape(&node.label()));
            let name = if node.children.is_empty() {
                name
            } else {
                format!(
                    "<a href=\"#{}\">{name}</a>",
                    escape(&tree_anchor(&node.path))
                )
            };
            let _ = writeln!(
                out,
                "<tr><td>{name}</td><td>{}</td></tr>",
                node.description
                    .as_deref()
                    .map(|d| self.inline(d))
                    .unwrap_or_default()
            );
        }
        let _ = writeln!(out, "</tbody>\n</table>");
    }

    fn envs(&self, out: &mut String, envs: &[slot::EnvVar]) {
        if envs.is_empty() {
            return;
        }
        let _ = writeln!(
            out,
            "<table>\n<thead><tr><th>キー</th><th>意味</th><th>秘密</th><th>ローカルでの設定場所</th><th>ステージング・本番での設定場所</th></tr></thead>\n<tbody>"
        );
        for e in envs {
            let _ = writeln!(
                out,
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape(&e.key),
                self.inline(&e.desc),
                if e.secret { "秘密" } else { "" },
                self.inline_opt(e.local_from.as_deref()),
                self.inline_opt(e.release_from.as_deref())
            );
        }
        let _ = writeln!(out, "</tbody>\n</table>");
    }

    fn cost(&self, out: &mut String, items: &[slot::CostItem]) {
        if items.is_empty() {
            let _ = writeln!(out, "<p class=\"note\">なし</p>");
            return;
        }
        let _ = writeln!(
            out,
            "<table>\n<thead><tr><th>項目</th><th>説明</th><th>月額</th><th>参考</th></tr></thead>\n<tbody>"
        );
        for item in items {
            let reference = item
                .reference
                .as_deref()
                .map(|r| match self.href(r) {
                    Some(href) => format!("<a href=\"{}\">{}</a>", escape(&href), escape(r)),
                    None => escape(r),
                })
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "<tr><td>{}</td><td>{}</td><td class=\"num\">{}</td><td>{reference}</td></tr>",
                escape(&item.name),
                self.inline_opt(item.desc.as_deref()),
                escape(&item.monthly.as_ref().map(slot::scalar).unwrap_or_default())
            );
        }
        let _ = writeln!(out, "</tbody>\n</table>");
    }

    // ---- Free ファイルのツリー ----

    fn docs(&self, out: &mut String) {
        let _ = writeln!(out, "<h2 class=\"group\">ドキュメント</h2>");
        if self.pages.is_empty() {
            let _ = writeln!(out, "<p class=\"note\">管理対象のファイルがない。</p>");
            return;
        }
        let mut root = Dir::default();
        for page in self.pages {
            root.insert(page);
        }
        // すべての Free ファイルが 1 つのディレクトリ（docs/ など）の中にあるときは、そのディレクトリの行を出さない。
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
        // トグルの動きは、表の行を読み込んだ後に付ける。
        if !self.tag_order().is_empty() {
            let _ = writeln!(out, "<script>{TAG_FILTER}</script>");
        }
    }

    /// Free ファイルをディレクトリのツリーにして出す。ディレクトリの中は、ファイル、ディレクトリの順。
    fn docs_tree(&self, out: &mut String, dir: &Dir, path: &str, depth: usize) {
        for page in &dir.files {
            let mission = escape(page.mission.as_deref().unwrap_or(""));
            let _ = writeln!(
                out,
                "<tr data-path=\"{}\" data-tags=\"{}\"><td style=\"padding-left:{}px\"><a href=\"{}\">{}</a></td><td title=\"{mission}\">{mission}</td><td>{}</td></tr>",
                escape(&page.source),
                escape(&serde_json::to_string(&page.tags).unwrap_or_default()),
                10 + depth * 20,
                escape(&self.link(&page.source, "").unwrap_or_default()),
                escape(&page.title),
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

    /// Slot の中の文字列（ルートを起点にした Markdown）を描く。h2 の下に置くので、見出しは h3 から。
    fn md(&self, text: &str) -> String {
        self.markdown(text, Path::new(""), false, 3)
    }

    /// 1 行の Markdown を描く。段落の `<p>` を外す。
    fn inline(&self, text: &str) -> String {
        strip_paragraph(&self.md(text))
    }

    fn inline_opt(&self, text: Option<&str>) -> String {
        text.map(|t| self.inline(t)).unwrap_or_default()
    }

    /// Markdown を HTML にする。base はリンクの起点（ルートからの相対パス）。
    /// drop_h1 なら、最初の h1 を除く（Markdown の Slot。ページの h1 と重なるため）。
    /// 見出しは、いちばん浅いものが top の段になるように付け直す。ページの中で h1 から h2、h3 と段を飛ばさないため。
    /// `<details>` は開いた状態にする。画像にしても中身が読めるように。
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
        let parsed: Vec<Event> = Parser::new_ext(text, markdown::OPTIONS).collect();
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
                Event::Html(html) if html.contains("<details>") => {
                    Event::Html(CowStr::from(open_details(&html)))
                }
                Event::InlineHtml(html) if html.contains("<details>") => {
                    Event::InlineHtml(CowStr::from(open_details(&html)))
                }
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
        if self
            .project
            .config_path
            .strip_prefix(&self.project.root)
            .is_ok_and(|rel| markdown::slash_path(rel) == target)
        {
            return Some(CONFIG_PAGE.to_string());
        }
        if self.pages.iter().any(|p| p.source == target) {
            return Some(doc_page(target));
        }
        if let Some(slot) = self.slots.iter().find(|s| s.path == target) {
            return Some(page_path(slot));
        }
        self.adrs.iter().find(|a| a.path == target).map(adr_page)
    }

    /// URL ならそのまま、ルートからのパスなら、ページのあるファイルだけリンク先にする。
    fn href(&self, target: &str) -> Option<String> {
        if slot::is_url(target) {
            Some(target.to_string())
        } else {
            self.link(target, "")
        }
    }

    /// ルートの中のファイルへのリンク先。Free ファイル、Slot ファイル、ADR だけ、そのページに出力先の中の相対パスで向ける。
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
        // フォルダ
        "directory-tree" => {
            r#"<path d="M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"/>"#
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
        // 旗（決定の記録）
        "adr" => r#"<path d="M5 21V4"/><path d="M5 4h12l-2.5 4 2.5 4H5"/>"#,
        _ => r#"<rect x="4" y="4" width="16" height="16" rx="2"/>"#,
    };
    format!(
        "<svg class=\"icon\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\">{shapes}</svg>"
    )
}

/// HTML の全体。CSS は中に書く。
fn page(title: &str, nav: &str, body: &str, aside: &str) -> String {
    format!(
        "<!DOCTYPE html>
<html lang=\"ja\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>{title}</title>
<style>
{STYLE}</style>
</head>
<body>
{nav}<main>
{body}</main>
{aside}</body>
</html>
"
    )
}

/// 本文の h2 と h3 から、右のサイドバーの目次を作る。id のない見出しには、GitHub と同じ形の id を付ける。
/// 見出しがなければ、目次は空にする。
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
    let mut aside = String::new();
    if !items.is_empty() {
        aside.push_str("<nav class=\"outline\">\n<p class=\"outline-title\">目次</p>\n<ul>\n");
        for (level, id, text) in items {
            let _ = writeln!(
                aside,
                "<li class=\"h{level}\"><a href=\"#{}\">{}</a></li>",
                escape(&id),
                text.trim()
            );
        }
        aside.push_str("</ul>\n</nav>\n");
    }
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

/// ディレクトリの Slot のツリーの、パス 1 つ。兄弟は書いた順に並べる。
struct TreeNode {
    name: String,
    /// ルートからのパス。末尾の `/` は付けない。
    path: String,
    dir: bool,
    /// 説明。entries に書いていない途中のディレクトリは None。
    description: Option<String>,
    children: Vec<TreeNode>,
}

impl TreeNode {
    /// 表に出す名前。ディレクトリには末尾に `/` を付ける。
    fn label(&self) -> String {
        format!("{}{}", self.name, if self.dir { "/" } else { "" })
    }
}

/// 下の段を持つディレクトリを、上から順に（親の次にその子を）集める。
fn tree_dirs<'a>(nodes: &'a [TreeNode], dirs: &mut Vec<&'a TreeNode>) {
    for node in nodes {
        if !node.children.is_empty() {
            dirs.push(node);
            tree_dirs(&node.children, dirs);
        }
    }
}

/// ディレクトリの Slot の、下の段の表のアンカー。
fn tree_anchor(path: &str) -> String {
    format!("dir-{}", encode_path(path))
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

/// 決定のページの、出力先からの相対パス。`ADR/<まとまり>/<題>.html`。
fn adr_page(a: &Adr) -> String {
    format!(
        "{ADR_DIR}/{}/{}.html",
        file_name(a.group()),
        file_name(&a.record.title)
    )
}

/// まとまりの名前を、ファイル名に使える形にする。
fn file_name(name: &str) -> String {
    name.replace(['/', '\\'], "-")
}

/// タブに出す OS の名前。
fn os_label(os: &str) -> String {
    match os {
        "macos" => "macOS".to_string(),
        "windows" => "Windows".to_string(),
        "linux" => "Linux".to_string(),
        other => other.to_string(),
    }
}

fn name_link(name: &str, url: Option<&str>) -> String {
    match url {
        Some(url) => format!("<a href=\"{}\">{}</a>", escape(url), escape(name)),
        None => escape(name),
    }
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

/// `<details>` を開いた状態にする。
fn open_details(html: &str) -> String {
    html.replace("<details>", "<details open>")
}

fn strip_paragraph(html: &str) -> String {
    let trimmed = html.trim();
    match trimmed
        .strip_prefix("<p>")
        .and_then(|s| s.strip_suffix("</p>"))
    {
        Some(inner) if !inner.contains("<p>") => inner.to_string(),
        _ => trimmed.to_string(),
    }
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
            "title: demo\nignore: [docs/tmp/**]\ntags:\n  - name: 仕様\n    color: blue\n",
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
                "docs/spec/a.md",
                "---\nmission: A は何か\ntags: [仕様]\n---\n\n# A <仕様>\n\n## 1. 章\n\n本文\n",
            ),
            ("docs/plan/b.md", "---\nmission: q\n---\n# B\n"),
            (
                "docv/adr/go.md",
                "---\ngroup: 言語\ndate: 2026-01-01\n---\n\n# Go にする\n\nGo で書く\n",
            ),
            (
                "docv/adr/rust.md",
                "---\ngroup: 言語\ndate: 2026-03-01\nsupersedes: [go]\n---\n\n# Rust にする\n\n## 選ばなかった案\n\n| 案 | 理由 |\n| --- | --- |\n| Zig | 情報が少ない |\n\n### 細部\n",
            ),
            ("docs/tmp/memo.md", "# Memo\n"),
            (
                "docv/service/install.yaml",
                "steps:\n  - id: binary\n    name: 取る\n    items:\n      - os: macos\n        body_markdown: \"`curl`\"\n      - os: windows\n  - id: cargo\n    name: cargo\n    items:\n      body_markdown: \"`cargo install`\"\nappendix_markdown: |\n  # 補足\n\n  後書き\n",
            ),
            (
                "docv/service/service.md",
                "# サービス仕様\n\n概要の本文\n\n## 部品\n\n[a](../../docs/spec/a.md)\n\n<details>\n\n細部\n\n</details>\n\n### 小見出し\n",
            ),
            (
                "docv/service/manual.md",
                "# 利用マニュアル\n\n| コマンド | 説明 |\n| --- | --- |\n| [demo run](#demo-run) | 動かす |\n\n## demo run\n\n詳しく\n",
            ),
            ("docv/system/db.md", "# DB 仕様書\n\n表\n"),
            (
                "docv/operation/cost.yaml",
                "accepted_items:\n  - name: サーバー\n    monthly: 3 ドル\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out, None).unwrap();

        // トップは 5 ページ。index.html（プロジェクト概要）には、Slot のページへのカードだけを置く
        let index = fs::read_to_string(out.join(PAGE)).unwrap();
        let detail = fs::read_to_string(out.join(DETAIL_PAGE)).unwrap();
        let adr_index = fs::read_to_string(out.join(ADR_PAGE)).unwrap();
        let config = fs::read_to_string(out.join(CONFIG_PAGE)).unwrap();
        // サイドバーで 5 ページを行き来する。描いているページの項目に印を付ける
        assert!(
            index.contains("<li><a href=\"index.html\" aria-current=\"page\">プロジェクト概要</a></li>\n<li><a href=\"ADR.html\">ADR</a></li>\n<li><a href=\"プロジェクト詳細.html\">プロジェクト詳細</a></li>\n<li><a href=\"現在の設定.html\">現在の設定</a></li>\n<li><a href=\"検索.html\">検索</a></li>"),
            "{index}"
        );
        // 検索.html に、Slot ファイル、ADR、Free ファイルの順で、中身を行ごとに索引にして書く。< は書き換え、</script> で閉じない
        let search = fs::read_to_string(out.join(SEARCH_PAGE)).unwrap();
        assert!(
            search.contains("<a href=\"検索.html\" aria-current=\"page\">検索</a>"),
            "{search}"
        );
        assert!(
            search.contains("{\"path\":\"docs/spec/a.md\",\"href\":\"プロジェクト詳細/docs/spec/a.html\",\"tags\":[[\"仕様\",0]],\"lines\":[\"---\",\"mission: A は何か\","),
            "{search}"
        );
        let slot_entry = search
            .find("\"path\":\"docv/service/install.yaml\"")
            .unwrap();
        let adr_entry = search.find("\"path\":\"docv/adr/go.md\"").unwrap();
        let free_entry = search.find("\"path\":\"docs/spec/a.md\"").unwrap();
        assert!(slot_entry < adr_entry && adr_entry < free_entry, "{search}");
        assert!(!search.contains("<nav class=\"outline\">"), "{search}");
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
            config.contains("<tr><td><code>dir.free</code></td><td><code>docs</code></td></tr>\n<tr><td><code>dir.slot</code></td><td><code>docv</code></td></tr>")
                && config.contains("<tr><td><code>dir.image</code></td><td>-</td></tr>"),
            "{config}"
        );
        // docv が決めた textlint のルールを、検査項目と非検査項目に分け、プリセットごとの表に並べる
        assert!(
            config.contains("<h3 id=\"検査項目\">検査項目</h3>\n<h4>ja-technical-writing</h4>\n<table class=\"textlint\">\n<thead><tr><th>ルール</th><th>確かめること</th></tr></thead>\n<tbody>\n<tr><td><code>sentence-length</code></td><td>一文を 100 文字以内にする</td></tr>"),
            "{config}"
        );
        assert!(
            config.contains("<h3 id=\"非検査項目\">非検査項目</h3>\n<h4>ja-technical-writing</h4>\n<table class=\"textlint\">\n<thead><tr><th>ルール</th><th>確かめないこと</th></tr></thead>\n<tbody>\n<tr><td><code>max-kanji-continuous-len</code></td><td>漢字が続く長さ</td></tr>"),
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
        assert!(!index.contains("class=\"adr-box"), "{index}");
        assert!(!index.contains(">spec/</td>"), "{index}");
        assert!(!index.contains("<h1>"), "{index}");
        assert!(!detail.contains("<h1>"), "{detail}");
        // サービス仕様のまとまりは、サービス概要、サービス仕様、利用マニュアル、インストール、用語集の順
        let overview = index.find(">サービス概要</span>").unwrap();
        let service = index.find(">サービス仕様</a>").unwrap();
        let manual = index.find(">利用マニュアル</a>").unwrap();
        let install = index.find(">インストール</a>").unwrap();
        let words = index.find(">用語集</span>").unwrap();
        assert!(
            overview < service && service < manual && manual < install && install < words,
            "{index}"
        );
        assert!(
            index.contains(
                "<a class=\"card\" href=\"サービス仕様/利用マニュアル.html\"><svg class=\"icon\""
            ),
            "{index}"
        );
        assert!(!index.contains("<p class=\"step-name\">"), "{index}");
        // ファイルのない Slot も、リンクにせずグレーで並べる
        assert!(
            index.contains("<span class=\"card missing\"><svg class=\"icon\""),
            "{index}"
        );
        assert!(index.contains("</svg>用語集</span>"), "{index}");
        // プロジェクト詳細.html に、Free ファイルのページへのリンクをツリーに並べる。管理外のファイルは出さない
        let plan = detail
            .find("<tr class=\"dir\" data-dir=\"docs/plan\"><td colspan=\"3\"><div style=\"margin-left:0px\">plan/</div></td></tr>")
            .unwrap();
        let spec = detail.find(">spec/</div>").unwrap();
        // すべての Free ファイルが入っている docs/ の行は出さず、ファイル名も出さない。タグには色を付ける
        assert!(!detail.contains(">docs/</div>"), "{detail}");
        assert!(
            detail.contains("<tr data-path=\"docs/spec/a.md\" data-tags=\"[&quot;仕様&quot;]\"><td style=\"padding-left:30px\"><a href=\"プロジェクト詳細/docs/spec/a.html\">A &lt;仕様&gt;</a></td><td title=\"A は何か\">A は何か</td><td><span class=\"tag c0\">仕様</span></td>"),
            "{detail}"
        );
        // 表の上に、タグのトグルを数と一緒に並べる。最初はすべて選んでいる
        assert!(
            detail.contains("<button type=\"button\" class=\"tag c0\" data-tag=\"仕様\" aria-pressed=\"true\">仕様<span class=\"count\">1</span></button>"),
            "{detail}"
        );
        assert!(detail.contains("<script>(function(){"), "{detail}");
        // Free ファイルは 1 つずつ HTML のページにする。見出しは h1 から段を飛ばさない
        let a = fs::read_to_string(out.join("プロジェクト詳細/docs/spec/a.html")).unwrap();
        assert!(a.contains("<h1>A &lt;仕様&gt;</h1>"), "{a}");
        // 見出しには GitHub と同じ形の id を付け、右のサイドバーに目次を置く
        assert!(a.contains("<h2 id=\"1-章\">1. 章</h2>"), "{a}");
        assert!(
            a.contains("<nav class=\"outline\">\n<p class=\"outline-title\">目次</p>\n<ul>\n<li class=\"h2\"><a href=\"#1-章\">1. 章</a></li>"),
            "{a}"
        );
        assert!(
            a.contains("<a href=\"../../../プロジェクト詳細.html\" aria-current=\"page\">"),
            "{a}"
        );
        assert_headings(&a);
        assert!(plan < spec, "{detail}");
        assert!(!detail.contains("tmp/"), "{detail}");

        // Slot は 1 つずつ独立したページ。同じ id の手順は os ごとのタブ、ほかは 1 つの手順
        let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();
        let install = read("サービス仕様/インストール.html");
        assert!(install.contains("</svg>インストール</h1>"), "{install}");
        // Slot のページのサイドバーは、1 つ上のトップのページを指す
        assert!(
            install
                .contains("<a href=\"../index.html\" aria-current=\"page\">プロジェクト概要</a>"),
            "{install}"
        );
        assert!(
            install.contains("<label for=\"tab-install-binary-0\">macOS</label>"),
            "{install}"
        );
        assert!(
            install.contains("<label for=\"tab-install-binary-1\">Windows</label>"),
            "{install}"
        );
        assert!(
            install.contains(
                "<li><p class=\"step-name\">cargo</p><p><code>cargo install</code></p>\n</li>"
            ),
            "{install}"
        );
        // appendix_markdown は、項目のあとに続けて出す。見出しは h2 から
        let appendix = install
            .find("<h2 id=\"補足\">補足</h2>\n<p>後書き</p>")
            .unwrap();
        assert!(
            install.find("cargo install").unwrap() < appendix,
            "{install}"
        );
        // Markdown の Slot は、h1 を除き、見出しを下げて、そのまま出す。見出しには id を付ける。details は開く
        let service = read("サービス仕様/サービス仕様.html");
        assert!(service.contains("<h2 id=\"部品\">部品</h2>"), "{service}");
        assert!(
            // Markdown の中のリンクは、pulldown-cmark が和名をパーセントエンコードする
            service.contains("href=\"../%E3%83%97%E3%83%AD%E3%82%B8%E3%82%A7%E3%82%AF%E3%83%88%E8%A9%B3%E7%B4%B0/docs/spec/a.html\">a</a>"),
            "{service}"
        );
        assert!(service.contains("<details open>"), "{service}");
        // md の Slot の見出しには id が付き、ページ内リンクで飛べる
        let manual = read("サービス仕様/利用マニュアル.html");
        assert!(
            manual.contains("<a href=\"#demo-run\">demo run</a>"),
            "{manual}"
        );
        assert!(
            manual.contains("<h2 id=\"demo-run\">demo run</h2>"),
            "{manual}"
        );
        // 名前に空白のある Slot のページは、リンクで空白を %20 にする
        assert!(
            index.contains("href=\"システム仕様/DB%20仕様書.html\""),
            "{index}"
        );
        let cost = read("運用/費用.html");
        assert!(cost.contains("<td class=\"num\">3 ドル</td>"), "{cost}");
        assert!(
            cost.contains("<h2 id=\"費用を理由にやっていないこと\">費用を理由にやっていないこと</h2>\n<p class=\"note\">なし</p>"),
            "{cost}"
        );
        // Slot のページにも、id を付けた見出しから右のサイドバーの目次を置く
        assert!(
            cost.contains("<li class=\"h2\"><a href=\"#費用を理由にやっていないこと\">費用を理由にやっていないこと</a></li>"),
            "{cost}"
        );

        // ADR は、ADR.html に、まとまりごとの h2 と決定のつながりの図を置く。
        // 箱をクリックすると、決定 1 つのページを開く
        assert!(
            adr_index.contains("<main>\n<h2 id=\"言語\" class=\"group\">言語</h2>"),
            "{adr_index}"
        );
        assert!(
            adr_index.contains("<div class=\"lane\"><a class=\"adr-box superseded\" href=\"ADR/言語/Go%20にする.html\">Go にする</a><span class=\"arrow\">→</span><a class=\"adr-box\" href=\"ADR/言語/Rust%20にする.html\">"),
            "{adr_index}"
        );
        let go = read("ADR/言語/Go にする.html");
        assert!(go.contains("</svg>Go にする</h1>"), "{go}");
        assert!(
            go.contains("<nav class=\"adr-nav\"><div class=\"prev\"></div><div class=\"next\"><a class=\"adr-box\" href=\"Rust%20にする.html\">Rust にする</a><span class=\"arrow\">→</span></div></nav>"),
            "{go}"
        );
        assert!(
            go.contains("<a href=\"../../ADR.html\" aria-current=\"page\">ADR</a>"),
            "{go}"
        );
        assert_headings(&go);
        let rust = read("ADR/言語/Rust にする.html");
        assert!(
            rust.contains("<h2 id=\"選ばなかった案\">選ばなかった案</h2>"),
            "{rust}"
        );
        assert!(rust.contains("<td>Zig</td>"), "{rust}");
        // 前後のカードは本文の後に置き、カーソルキーで移れるようにする。状態や id、日付は出さない
        assert!(
            rust.find("<td>Zig</td>").unwrap() < rust.find("<nav class=\"adr-nav\">").unwrap(),
            "{rust}"
        );
        assert!(
            rust.contains("<script>document.addEventListener('keydown'"),
            "{rust}"
        );
        assert!(!rust.contains("2026-03-01"), "{rust}");
        assert!(
            rust.contains("<nav class=\"adr-nav\"><div class=\"prev\"><span class=\"arrow\">←</span><a class=\"adr-box superseded\" href=\"Go%20にする.html\">Go にする</a></div><div class=\"next\"></div></nav>"),
            "{rust}"
        );
        assert_headings(&rust);

        // Slot のページは、h1 が 1 つで、見出しの段を飛ばさない
        for slot in &slot::load(&project).unwrap() {
            let html = read(&page_path(slot));
            assert_headings(&html);
        }

        run(&project, &out, None).unwrap();
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
    fn links_slot_and_adr_files_to_their_pages() {
        let (dir, project) = project(&[
            (
                "docs/a.md",
                "---\nmission: q\n---\n# A\n\n[s](../docv/system/system.md#部品) [d](../docv/adr/go.md)\n",
            ),
            ("docv/system/system.md", "# システム仕様\n"),
            (
                "docv/adr/go.md",
                "---\ngroup: 言語\ndate: 2026-01-01\n---\n\n# Go にする\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out, None).unwrap();
        let page = fs::read_to_string(out.join("プロジェクト詳細/docs/a.html")).unwrap();
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
            page.contains("href=\"../../システム仕様/システム仕様.html#部品\""),
            "{page}"
        );
        assert!(
            page.contains("href=\"../../ADR/言語/Go にする.html\""),
            "{page}"
        );
    }

    #[test]
    fn does_not_link_files_without_page() {
        let (dir, project) = project(&[
            ("docs/a.md", "---\nmission: q\n---\n# A\n"),
            (
                "docv/system/system.md",
                "# システム仕様\n\n[memo](../../notes.md) ![図](../../a.svg) [設定](../../docv.yaml)\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out, None).unwrap();
        let page = fs::read_to_string(out.join("システム仕様/システム仕様.html")).unwrap();
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
            detail.contains("href=\"プロジェクト詳細/docs/a.html\""),
            "{detail}"
        );
        // ADR がないときは、ADR.html を作らず、サイドバーにも出さない
        assert!(!out.join(ADR_PAGE).exists());
        assert!(!detail.contains("ADR.html"), "{detail}");
        // 出力先に .gitignore は置かない
        assert!(!out.join(".gitignore").exists());
    }

    #[test]
    fn splits_directory_tree_into_one_level_tables() {
        let (dir, project) = project(&[
            ("src/main.rs", ""),
            ("docs/a.md", "---\nmission: q\n---\n# A\n"),
            (
                "docv/guide/directory-tree.yaml",
                "entries:\n  - path: src/\n    desc_markdown: ソース\n  - path: docs/\n    desc_markdown: \"`docv` の文書\"\n  - path: src/main.rs\n    desc_markdown: 入口\n  - path: docs/rule/guide.md\n    desc_markdown: 書き方\n  - path: docs/spec/\n    desc_markdown: 規格\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out, None).unwrap();
        let page = fs::read_to_string(out.join("開発者向けガイド/ディレクトリ.html")).unwrap();
        let body: String = page
            .lines()
            .filter(|l| l.starts_with("<h2") || l.starts_with("<tr><td>"))
            .map(|l| format!("{l}\n"))
            .collect();
        // 表は 1 段だけ。下の段を持つディレクトリは、親の次にその子の順で、表を分けて後ろに並べ、名前からリンクする。表の名前は code にし、h2 は code にしない。
        // 書いていない途中のディレクトリ（docs/rule/）は説明なしで出す。兄弟は書いた順
        assert_eq!(
            body,
            "<tr><td><a href=\"#dir-src\"><code>src/</code></a></td><td>ソース</td></tr>
<tr><td><a href=\"#dir-docs\"><code>docs/</code></a></td><td><code>docv</code> の文書</td></tr>
<h2 id=\"dir-src\">src/</h2>
<tr><td><code>main.rs</code></td><td>入口</td></tr>
<h2 id=\"dir-docs\">docs/</h2>
<tr><td><a href=\"#dir-docs/rule\"><code>rule/</code></a></td><td></td></tr>
<tr><td><code>spec/</code></td><td>規格</td></tr>
<h2 id=\"dir-docs/rule\">docs/rule/</h2>
<tr><td><code>guide.md</code></td><td>書き方</td></tr>
"
        );
    }

    #[test]
    fn refuses_to_clear_foreign_directory() {
        let (dir, project) = project(&[]);
        let out = dir.path().join("docs");
        write_file(dir.path(), "docs/keep.txt", "keep");
        let error = run(&project, &out, None).unwrap_err();
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
                "docv/service/service/user.md",
                "---\norder: 1\n---\n\n# 利用者向け機能\n\n## 注文\n\n[取り消し](admin.md#取り消し)\n",
            ),
            (
                "docv/service/service/admin.md",
                "---\norder: 2\n---\n\n# 管理者向け機能\n\n## 取り消し\n\n本文\n",
            ),
            (
                "docs/a.md",
                "---\nmission: a\n---\n\n# A\n\n[管理](../docv/service/service/admin.md)\n",
            ),
        ]);
        let out = dir.path().join("out");
        run(&project, &out, None).unwrap();
        let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();

        // ページを 1 つずつ、Slot の名前のディレクトリの下に、ページの名前で置く。カードは先頭のページを開く
        let index = read(PAGE);
        assert!(
            index.contains(
                "<a class=\"card\" href=\"サービス仕様/サービス仕様/利用者向け機能.html\">"
            ),
            "{index}"
        );
        let user = read("サービス仕様/サービス仕様/利用者向け機能.html");
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
            user.contains(
                "<a href=\"../../index.html\" aria-current=\"page\">プロジェクト概要</a>"
            ),
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
        let admin = read("サービス仕様/サービス仕様/管理者向け機能.html");
        assert!(
            admin.contains(
                "<a href=\"管理者向け機能.html\" aria-current=\"page\">管理者向け機能</a>"
            ),
            "{admin}"
        );
        // Free ファイルからのリンクも、そのページに向く
        let a = read("プロジェクト詳細/docs/a.html");
        assert!(
            a.contains("href=\"../../%E3%82%B5%E3%83%BC%E3%83%93%E3%82%B9%E4%BB%95%E6%A7%98/%E3%82%B5%E3%83%BC%E3%83%93%E3%82%B9%E4%BB%95%E6%A7%98/%E7%AE%A1%E7%90%86%E8%80%85%E5%90%91%E3%81%91%E6%A9%9F%E8%83%BD.html\">管理</a>"),
            "{a}"
        );
        // 検索の索引も、ページごとに持つ
        let search = read(SEARCH_PAGE);
        assert!(
            search.contains("{\"path\":\"docv/service/service/admin.md\",\"href\":\"サービス仕様/サービス仕様/管理者向け機能.html\""),
            "{search}"
        );
    }

    #[test]
    fn rebuilds_everything_but_the_marker() {
        let (dir, project) = project(&[]);
        let out = dir.path().join("output/docv/html");
        run(&project, &out, None).unwrap();
        write_file(&out, "jpg/a.jpg", "jpg");
        write_file(&out, "old.html", "old");
        // 途中で止まって index.html などがない出力先でも、目印があれば作り直せる
        fs::remove_file(out.join(PAGE)).unwrap();
        run(&project, &out, None).unwrap();
        assert!(!out.join("jpg").exists());
        assert!(!out.join("old.html").exists());
        assert!(out.join(PAGE).is_file());
    }

    #[test]
    fn puts_jpg_under_group_and_label() {
        let slot = Slot {
            def: slot::def("words").unwrap(),
            path: "docv/service/words.yaml".to_string(),
            content: Content::Markdown(String::new()),
            page: None,
        };
        assert_eq!(jpg_path(&slot), "サービス仕様/用語集.jpg");
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
