//! docv list。Slot と Free ファイルの一覧を出す。docv/slot/service/manual.md の docv list。
//!
//! AI Agent が、どこに何が書いてあり、何をどこに書くかを決めるのに使う。
//! Slot は、置いていないものも含めてすべて出す。Free ファイルを足す前に、当たる Slot がないかを確かめられるようにするため。

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result};
use unicode_width::UnicodeWidthStr;

use crate::config;
use crate::markdown;
use crate::meta;
use crate::slot::{self, DEFS};
use crate::workspace::Project;

/// 置いた Slot の行の頭。git branch と同じ見せ方にする。
const PLACED: &str = "* ";
/// 置いていない Slot と、Free ファイルの行の頭。
const BLANK: &str = "  ";
/// 列の間。
const GAP: &str = "  ";
/// タグを 1 行に並べるときの区切り。
const SEPARATOR: &str = " / ";
/// Slot の名前とミッションの区切り。名前を埋めて、縦に揃える。
const COLON: &str = ": ";

/// docv list の出力。project は、docv.yaml がなければ None で、Slot だけを出す。
pub fn text(project: Option<&Project>) -> Result<String> {
    let mut out = slots(project)?;
    if let Some(project) = project {
        out.push('\n');
        out.push_str(&free(project)?);
    }
    Ok(out)
}

/// 1 つの Slot の行。
struct Row {
    placed: bool,
    id: &'static str,
    /// ルートからの相対パス。ページに分けた Slot はディレクトリ。
    path: String,
    /// Slot の名前。
    label: &'static str,
    mission: &'static str,
    /// ページに分けた Slot のページ。ページのディレクトリからのファイル名と、ページの名前。order の順。
    pages: Vec<String>,
}

/// Slot を、規格の順にすべて並べる。
fn slots(project: Option<&Project>) -> Result<String> {
    let (slot_dir, loaded, placed) = match project {
        Some(p) => (p.slot_dir(), slot::load(p)?, slot::placed(p)?),
        None => (config::parse("{}")?.slot_dir(), Vec::new(), Vec::new()),
    };
    let rows: Vec<Row> = DEFS
        .iter()
        .map(|def| {
            // load は、ページを order とパスの順に並べ、1 つのファイルもあればページを読まない
            let pages: Vec<&slot::Slot> = loaded
                .iter()
                .filter(|s| s.def.id == def.id && s.page.is_some())
                .collect();
            let path = match def.pages_dir() {
                Some(dir) if !pages.is_empty() => format!("{slot_dir}/{dir}/"),
                _ => format!("{slot_dir}/{}", def.file_name()),
            };
            let pages = pages
                .iter()
                .filter_map(|s| {
                    let page = s.page.as_ref()?;
                    let file = s.path.strip_prefix(&path).unwrap_or(&s.path);
                    Some(format!("{file}（{}）", page.title))
                })
                .collect();
            Row {
                placed: placed.contains(&def.id),
                id: def.id,
                path,
                label: def.label,
                mission: def.mission,
                pages,
            }
        })
        .collect();
    let id_width = rows.iter().map(|r| width(r.id)).max().unwrap_or(0);
    let path_width = rows.iter().map(|r| width(&r.path)).max().unwrap_or(0);
    let label_width = rows.iter().map(|r| width(r.label)).max().unwrap_or(0);
    // ページの行は、ミッションの頭に揃える
    let page_indent =
        BLANK.len() + id_width + GAP.len() + path_width + GAP.len() + label_width + COLON.len();
    let mut out = String::from("Slot\n");
    for row in &rows {
        let mark = if row.placed { PLACED } else { BLANK };
        let _ = writeln!(
            out,
            "{mark}{}{GAP}{}{GAP}{}{COLON}{}",
            pad(row.id, id_width),
            pad(&row.path, path_width),
            pad(row.label, label_width),
            row.mission
        );
        for page in &row.pages {
            let _ = writeln!(out, "{:page_indent$}{page}", "");
        }
    }
    Ok(out)
}

/// Free ファイルを、パスの順に並べる。docv_exclude に当たるファイルは、管理対象ではないので出さない。
fn free(project: &Project) -> Result<String> {
    let mut rows = Vec::new();
    for path in project.managed()? {
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        let document = markdown::split(&text);
        let meta = meta::read(&document, &markdown::outline(&document));
        let mission = meta.mission.as_deref().unwrap_or("（mission がない）");
        let tags = if meta.tags.is_empty() {
            String::new()
        } else {
            format!("（タグ: {}）", meta.tags.join(SEPARATOR))
        };
        rows.push((path, format!("{mission}{tags}")));
    }
    let mut out = String::from("Free\n");
    if rows.is_empty() {
        let _ = writeln!(out, "{BLANK}なし");
        return Ok(out);
    }
    let path_width = rows.iter().map(|(p, _)| width(p)).max().unwrap_or(0);
    for (path, mission) in &rows {
        let _ = writeln!(out, "{BLANK}{}{GAP}{mission}", pad(path, path_width));
    }
    Ok(out)
}

/// 端末に出したときの幅。全角の文字は 2 と数え、日本語があっても列を揃える。
fn width(text: &str) -> usize {
    text.width()
}

/// 右を空白で埋めて、幅を width にする。
fn pad(text: &str, width: usize) -> String {
    format!(
        "{text}{}",
        " ".repeat(width.saturating_sub(self::width(text)))
    )
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

    fn project(config: &str, files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", config);
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    /// Slot の行のうち、名前が id のもの（ページの行を含む）。
    fn slot_lines(list: &str, id: &str) -> Vec<String> {
        let lines: Vec<&str> = list.lines().collect();
        let start = lines
            .iter()
            .position(|l| l.get(2..).is_some_and(|l| l.starts_with(&format!("{id} "))))
            .unwrap_or_else(|| panic!("{id} がない: {list}"));
        let mut out = vec![lines[start].trim_end().to_string()];
        out.extend(
            lines[start + 1..]
                .iter()
                .take_while(|l| l.starts_with("    "))
                .map(|l| l.trim().to_string()),
        );
        out
    }

    #[test]
    fn lists_every_slot_and_marks_the_placed_ones() {
        let (_dir, project) = project(
            "docv_exclude:\n  - docv/free/tmp/**\n",
            &[
                (
                    "docv/slot/service/overview.md",
                    "---\nmission: 利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明\n---\n\n# サービス概要\n",
                ),
                (
                    "docv/slot/service/service/b.md",
                    "---\norder: 2\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 管理\n",
                ),
                (
                    "docv/slot/service/service/a.md",
                    "---\norder: 1\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 利用者向け\n",
                ),
                (
                    "docv/free/order.md",
                    "---\nmission: 注文を確定するタイミング\ntags: [ 仕様, 注文 ]\n---\n\n# 注文\n",
                ),
                ("docv/free/note.md", "# メモ\n"),
                ("docv/free/tmp/draft.md", "---\nmission: 下書き\n---\n"),
            ],
        );
        let list = text(Some(&project)).unwrap();
        // Slot は規格の順にすべて出し、置いたものに印を付ける
        assert!(list.starts_with("Slot\n* overview "), "{list}");
        let ids: Vec<&str> = list
            .lines()
            .skip(1)
            .take_while(|l| !l.is_empty())
            .filter(|l| !l.starts_with("    "))
            .map(|l| l[2..].split(' ').next().unwrap())
            .collect();
        assert_eq!(ids, DEFS.iter().map(|d| d.id).collect::<Vec<_>>());
        // Slot の名前は、いちばん広い名前（利用マニュアルなど）の幅に埋め、: を縦に揃える
        assert_eq!(
            slot_lines(&list, "overview"),
            [format!(
                "* {}  {}  {}: 利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明",
                pad("overview", 15),
                pad("docv/slot/service/overview.md", 37),
                pad("サービス概要", 14)
            )]
        );
        // ページに分けた Slot は、ディレクトリを出し、その下にページを order の順に出す
        let service = format!(
            "* {}  {}  {}: 外から見た機能と、それぞれの振る舞いと決まり",
            pad("service", 15),
            pad("docv/slot/service/service/", 37),
            pad("サービス仕様", 14)
        );
        assert_eq!(
            slot_lines(&list, "service"),
            [
                service.clone(),
                "a.md（利用者向け）".to_string(),
                "b.md（管理）".to_string(),
            ]
        );
        // ページの行は、ミッションの頭に揃える
        let indent = " ".repeat(2 + 15 + 2 + 37 + 2 + 14 + 2);
        assert!(
            list.contains(&format!(
                "{service}\n{indent}a.md（利用者向け）\n{indent}b.md（管理）\n"
            )),
            "{list}"
        );
        // 置いていない Slot は、印を付けず、分けないときのファイルを出す
        assert_eq!(
            slot_lines(&list, "system"),
            [format!(
                "  {}  {}  {}: 部品ごとの振る舞いと、部品の間の流れ",
                pad("system", 15),
                pad("docv/slot/system/system.md", 37),
                pad("システム仕様", 14)
            )]
        );
        // Free ファイルは、パス、ミッション、タグを出し、docv_exclude に当たるものは出さない
        assert!(
            list.ends_with(
                "\nFree\n\
                 \x20 docv/free/note.md   （mission がない）\n\
                 \x20 docv/free/order.md  注文を確定するタイミング（タグ: 仕様 / 注文）\n"
            ),
            "{list}"
        );
    }

    #[test]
    fn counts_a_slot_that_cannot_be_read_as_placed() {
        // 形の合わない Slot も、置いたことに数える。形の誤りは check が報告する
        let (_dir, project) = project(
            "dir:\n  document_root: docs\n",
            &[("docs/slot/service/words.yaml", "groups: 1\n")],
        );
        let list = text(Some(&project)).unwrap();
        assert!(
            slot_lines(&list, "words")[0].starts_with(&format!(
                "* {}  docs/slot/service/words.yaml",
                pad("words", 15)
            )),
            "{list}"
        );
        assert!(list.ends_with("\nFree\n  なし\n"), "{list}");
    }

    #[test]
    fn lists_only_slots_without_a_config() {
        let list = text(None).unwrap();
        assert!(list.starts_with("Slot\n  overview "), "{list}");
        assert!(!list.contains("* "), "{list}");
        assert!(!list.contains("Free"), "{list}");
        assert_eq!(list.lines().count(), 1 + DEFS.len());
    }
}
