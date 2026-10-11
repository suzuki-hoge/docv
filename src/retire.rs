//! docv retire。章やファイルを archive に移す。docs/spec/retire.md。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use pulldown_cmark::{Event, Parser, Tag};

use crate::format;
use crate::kind::Time;
use crate::markdown::{self, relative_path, slash_path};
use crate::meta;
use crate::style::{self, Style};
use crate::workspace::{Class, Project};

pub struct Request<'a> {
    /// 移す元のファイル。ルートからの相対パス。
    pub path: &'a str,
    /// 章の見出し（章番号を除いた部分）か、章番号。None ならファイル全体を移す。
    pub section: Option<&'a str>,
    pub reason: &'a str,
    /// 章の見出しに書く日付（YYYY-MM-DD）。
    pub today: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// 移した先の archive のファイル。
    pub archive: String,
    /// 移した元のファイルや、移した章を指すリンク（`パス:行`）。
    pub links: Vec<String>,
}

/// 移す内容。
struct Moved {
    heading: String,
    question: Option<String>,
    body: String,
    /// 章を移したあとの、移した元のファイル。ファイル全体を移したときは None。
    rest: Option<String>,
}

pub fn run(project: &Project, request: &Request) -> Result<Outcome> {
    let path = request.path;
    let full = project.root.join(path);
    if !full.is_file() {
        bail!("{path} がない");
    }
    let index = match project.classify(path) {
        Some(Class::Managed(index)) => index,
        _ => bail!("{path} は管理対象ではない。移せるのは、管理対象の現在と未来のファイルだけ"),
    };
    let kind = project.kind_of(index);
    if !matches!(kind.time, Time::Present | Time::Future) {
        bail!(
            "{path} は {}。移せるのは、管理対象の現在と未来のファイルだけ",
            kind.id
        );
    }
    if request.reason.trim().is_empty() {
        bail!("--reason に、移す理由を書く");
    }

    let text = fs::read_to_string(&full).with_context(|| format!("{path} を読めない"))?;
    let moved = match request.section {
        Some(section) => take_section(&text, path, section)?,
        None => take_file(project, index, &text, path),
    };

    let archive = archive_path(&project.config.archive.dir, path);
    let archive_dir = parent(&archive);
    let link_base = Style::resolve(project.config.lint.style.as_ref(), None).link_text;
    let display = style::link_text(link_base.as_deref(), &archive, path);
    let body = rewrite_links(&moved.body, &parent(path), &archive_dir);

    let mut entry = format!(
        "## {} {}\n\n- 移した元: [{display}]({})\n- 理由: {}\n",
        request.today,
        moved.heading,
        relative_path(Path::new(&archive_dir), Path::new(path)),
        request.reason.trim()
    );
    if let Some(question) = &moved.question {
        entry.push_str(&format!("- 元の問い: {question}\n"));
    }
    if !body.is_empty() {
        entry.push_str(&format!("\n{body}"));
    }

    let archive_full = project.root.join(&archive);
    let archive_text = match fs::read_to_string(&archive_full) {
        Ok(existing) => insert_entry(&existing, &entry),
        Err(_) => format!(
            "---\nquestion: {path} から外した内容と、外した理由は何か\nsource: {path}\n---\n\n# {display} の経緯\n\n{entry}"
        ),
    };

    write(project, &archive_full, &archive_text)?;
    match &moved.rest {
        Some(rest) => write(project, &full, rest)?,
        None => fs::remove_file(&full).with_context(|| format!("{path} を消せない"))?,
    }

    Ok(Outcome {
        links: links_to(project, path, request.section.is_some())?,
        archive,
    })
}

fn write(project: &Project, path: &Path, text: &str) -> Result<()> {
    let text = if project.config.format.enabled {
        format::format(text).with_context(|| format!("{} を整形できない", path.display()))?
    } else {
        text.to_string()
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("{} を作れない", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("{} を書けない", path.display()))
}

/// archive.dir の下の、移した元と同じ相対パス。相対パスの起点は archive.dir の親ディレクトリ。
fn archive_path(archive_dir: &str, source: &str) -> String {
    let dir = archive_dir.trim_start_matches("./").trim_end_matches('/');
    let rel = match dir.rsplit_once('/') {
        Some((base, _)) => source
            .strip_prefix(base)
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(source),
        None => source,
    };
    format!("{dir}/{rel}")
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(dir, _)| dir.to_string())
}

/// `3. 見出し` を、章番号と見出しに分ける。
fn split_number(title: &str) -> (Option<&str>, &str) {
    let digits = title.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(rest) = title[digits..].strip_prefix('.')
        && (rest.is_empty() || rest.starts_with(char::is_whitespace))
    {
        return (Some(&title[..digits]), rest.trim_start());
    }
    (None, title)
}

/// ATX の見出しの行から、見出しの文字列を取り出す。
fn heading_text(line: &str) -> &str {
    line.trim_end().trim_start_matches('#').trim()
}

fn take_section(text: &str, path: &str, section: &str) -> Result<Moved> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let document = markdown::split(text);
    let outline = markdown::outline(&document);
    let h2: Vec<usize> = outline
        .headings
        .iter()
        .filter(|h| h.level == 2)
        .map(|h| h.line - 1)
        .collect();
    let wanted = section.trim();
    let wanted_number = wanted.trim_end_matches('.');
    let found = h2.iter().position(|&i| {
        let (number, title) = split_number(heading_text(lines[i]));
        title == wanted || number == Some(wanted_number)
    });
    let Some(position) = found else {
        let titles: Vec<&str> = h2.iter().map(|&i| heading_text(lines[i])).collect();
        bail!("{path} に章「{wanted}」がない。章は {}", titles.join("、"));
    };
    let start = h2[position];
    let end = h2.get(position + 1).copied().unwrap_or(lines.len());
    let heading = split_number(heading_text(lines[start])).1.to_string();

    let body = trim_block(&lines[start + 1..end]);
    let mut before = lines[..start].to_vec();
    if end == lines.len() {
        // 最後の章を移したときは、その前の区切り線も消す。
        before = trim_block_end(&before).to_vec();
    }
    let mut rest: String = before.concat();
    if !rest.ends_with('\n') {
        rest.push('\n');
    }
    rest.push_str(&lines[end..].concat());
    Ok(Moved {
        heading,
        question: None,
        body,
        rest: Some(rest),
    })
}

fn take_file(project: &Project, index: usize, text: &str, path: &str) -> Moved {
    let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    let document = markdown::split(text);
    let outline = markdown::outline(&document);
    let meta = meta::read(project.entry(index), &document, &outline);
    let h1 = outline.headings.iter().find(|h| h.level == 1);
    let heading = h1.map_or_else(
        || path.to_string(),
        |h| heading_text(&lines[h.line - 1]).to_string(),
    );
    // 元の h2 以下の見出しは、archive の章の下に入るので 1 段ずつ下げる。
    for h in outline
        .headings
        .iter()
        .filter(|h| (2..6).contains(&h.level))
    {
        let line = &mut lines[h.line - 1];
        if line.starts_with('#') {
            line.insert(0, '#');
        }
    }
    let body: Vec<&str> = lines
        .iter()
        .enumerate()
        .skip(document.body_start_line - 1)
        .filter(|(i, _)| h1.is_none_or(|h| *i != h.line - 1))
        .map(|(_, l)| l.as_str())
        .collect();
    Moved {
        heading,
        question: meta.question,
        body: trim_block(&body),
        rest: None,
    }
}

/// 前後の空行と、末尾の区切り線を除いてつなぐ。空でなければ改行で終わる。
fn trim_block(lines: &[&str]) -> String {
    let lines = trim_block_end(lines);
    let start = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(lines.len());
    let mut text = lines[start..].concat();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn trim_block_end<'a, 'b>(lines: &'b [&'a str]) -> &'b [&'a str] {
    let end = lines
        .iter()
        .rposition(|l| {
            let l = l.trim();
            !l.is_empty() && l != "---"
        })
        .map_or(0, |i| i + 1);
    &lines[..end]
}

/// 移した本文の中の相対リンクを、archive のファイルの位置から辿れるように書き換える。
fn rewrite_links(body: &str, from_dir: &str, to_dir: &str) -> String {
    let mut replacements = Vec::new();
    for (event, range) in Parser::new_ext(body, markdown::OPTIONS).into_offset_iter() {
        let (Event::Start(Tag::Link { dest_url, .. }) | Event::Start(Tag::Image { dest_url, .. })) =
            event
        else {
            continue;
        };
        if dest_url.starts_with('/') {
            continue;
        }
        let Some((target, fragment)) = markdown::resolve_link(Path::new(from_dir), &dest_url)
        else {
            continue;
        };
        let mut new = relative_path(Path::new(to_dir), Path::new(&slash_path(&target)));
        if let Some(fragment) = fragment {
            new = format!("{new}#{fragment}");
        }
        let old = format!("]({dest_url}");
        if let Some(i) = body[range.clone()].find(&old) {
            let at = range.start + i + 2;
            replacements.push((at..at + dest_url.len(), new));
        }
    }
    let mut text = body.to_string();
    for (range, new) in replacements.into_iter().rev() {
        text.replace_range(range, &new);
    }
    text
}

/// archive のファイルの h1 のすぐ下に章を足す。
fn insert_entry(existing: &str, entry: &str) -> String {
    let lines: Vec<&str> = existing.split_inclusive('\n').collect();
    let document = markdown::split(existing);
    let outline = markdown::outline(&document);
    let at = outline
        .headings
        .iter()
        .find(|h| h.level == 1)
        .map_or(document.body_start_line - 1, |h| h.line);
    let mut text: String = lines[..at.min(lines.len())].concat();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(entry);
    let after = lines[at.min(lines.len())..].concat();
    let after = after.trim_start_matches('\n');
    if !after.is_empty() {
        text.push('\n');
        text.push_str(after);
    }
    text
}

/// 移した元のファイルを指すリンク。章を移したときは、`#` の付いたリンクだけ。archive のファイルは見ない。
fn links_to(project: &Project, source: &str, section: bool) -> Result<Vec<String>> {
    let mut links = Vec::new();
    for file in project.files()? {
        if let Class::Managed(index) = file.class
            && project.kind_of(index).time == Time::Past
        {
            continue;
        }
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        let outline = markdown::outline(&markdown::split(&text));
        let base = parent(&file.path);
        for link in &outline.links {
            if let Some((target, fragment)) = markdown::resolve_link(Path::new(&base), &link.dest)
                && slash_path(&target) == source
                && (!section || fragment.is_some())
            {
                links.push(format!("{}:{}", file.path, link.line));
            }
        }
    }
    Ok(links)
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

    fn read(root: &Path, path: &str) -> String {
        fs::read_to_string(root.join(path)).unwrap()
    }

    const CONFIG: &str = "version: 1
docs:
  - path: README.md
    kind: overview
    question: q
  - path: docs/spec/*.md
    kind: spec
  - path: docs/proposal/*.md
    kind: proposal
  - path: docs/archive/**/*.md
    kind: archive
lint:
  style:
    link_text: { base: docs }
";

    const ORDER: &str = "---
question: 注文の処理はどう動くか
---

# 注文

[README.md](../../README.md)

---

## 1. 選び方

[spec/other.md](other.md#x) を見る。

---

## 2. 注文履歴

履歴は [spec/other.md](other.md) に書く。

### 列

---

## 3. 失敗

```
## 3. コードの中
```
";

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    fn request<'a>(path: &'a str, section: Option<&'a str>) -> Request<'a> {
        Request {
            path,
            section,
            reason: "監査ログにまとめた",
            today: "2026-10-11",
        }
    }

    #[test]
    fn moves_section_to_top_of_archive() {
        let (dir, project) = project(&[
            ("docs/spec/order.md", ORDER),
            (
                "docs/spec/other.md",
                "# O\n\n[spec/order.md](order.md#2-注文履歴) [spec/order.md](order.md)\n",
            ),
            (
                "docs/archive/spec/order.md",
                "---\nquestion: q\nsource: docs/spec/order.md\n---\n\n# spec/order.md の経緯\n\n## 2026-01-01 古い章\n\n- 理由: r\n",
            ),
        ]);
        let root = dir.path();
        let outcome = run(&project, &request("docs/spec/order.md", Some("注文履歴"))).unwrap();
        assert_eq!(
            outcome,
            Outcome {
                archive: "docs/archive/spec/order.md".to_string(),
                links: vec!["docs/spec/other.md:3".to_string()],
            }
        );
        assert_eq!(
            read(root, "docs/spec/order.md"),
            "---
question: 注文の処理はどう動くか
---

# 注文

[README.md](../../README.md)

---

## 1. 選び方

[spec/other.md](other.md#x) を見る。

---

## 3. 失敗

```
## 3. コードの中
```
"
        );
        assert_eq!(
            read(root, "docs/archive/spec/order.md"),
            "---
question: q
source: docs/spec/order.md
---

# spec/order.md の経緯

## 2026-10-11 注文履歴

- 移した元: [spec/order.md](../../spec/order.md)
- 理由: 監査ログにまとめた

履歴は [spec/other.md](../../spec/other.md) に書く。

### 列

## 2026-01-01 古い章

- 理由: r
"
        );
    }

    #[test]
    fn moves_last_section_by_number() {
        let (dir, project) = project(&[("docs/spec/order.md", ORDER)]);
        run(&project, &request("docs/spec/order.md", Some("3"))).unwrap();
        let rest = read(dir.path(), "docs/spec/order.md");
        assert!(rest.ends_with("### 列\n"), "{rest}");
        let archive = read(dir.path(), "docs/archive/spec/order.md");
        assert!(archive.contains("## 2026-10-11 失敗\n"), "{archive}");
        assert!(
            archive.contains("```\n## 3. コードの中\n```\n"),
            "{archive}"
        );
    }

    #[test]
    fn moves_whole_file() {
        let (dir, project) = project(&[
            (
                "docs/proposal/coupon.md",
                "---\nquestion: クーポンを配るか\n---\n\n# クーポン\n\n## 案\n\n[README.md](../../README.md)\n\n## 決めること\n",
            ),
            ("README.md", "# R\n\n[x](docs/proposal/coupon.md)\n"),
        ]);
        let root = dir.path();
        let outcome = run(&project, &request("docs/proposal/coupon.md", None)).unwrap();
        assert_eq!(outcome.links, ["README.md:3"]);
        assert!(!root.join("docs/proposal/coupon.md").exists());
        assert_eq!(
            read(root, "docs/archive/proposal/coupon.md"),
            "---
question: docs/proposal/coupon.md から外した内容と、外した理由は何か
source: docs/proposal/coupon.md
---

# proposal/coupon.md の経緯

## 2026-10-11 クーポン

- 移した元: [proposal/coupon.md](../../proposal/coupon.md)
- 理由: 監査ログにまとめた
- 元の問い: クーポンを配るか

### 案

[README.md](../../../README.md)

### 決めること
"
        );
    }

    #[test]
    fn rejects_what_cannot_be_moved() {
        let (_dir, project) = project(&[
            ("docs/spec/order.md", ORDER),
            ("docs/archive/a.md", "# A\n"),
            ("notes.md", "# N\n"),
        ]);
        for (path, section, message) in [
            ("docs/spec/order.md", Some("ない章"), "章「ない章」がない"),
            ("docs/archive/a.md", None, "現在と未来のファイルだけ"),
            ("notes.md", None, "管理対象ではない"),
            ("docs/spec/none.md", None, "がない"),
        ] {
            let error = run(&project, &request(path, section))
                .unwrap_err()
                .to_string();
            assert!(error.contains(message), "{path}: {error}");
        }
    }

    #[test]
    fn computes_archive_path() {
        assert_eq!(
            archive_path("docs/archive", "docs/spec/a.md"),
            "docs/archive/spec/a.md"
        );
        assert_eq!(
            archive_path("docs/archive/", "README.md"),
            "docs/archive/README.md"
        );
        assert_eq!(archive_path("archive", "docs/a.md"), "archive/docs/a.md");
        assert_eq!(split_number("12. 見出し"), (Some("12"), "見出し"));
        assert_eq!(split_number("2026 年"), (None, "2026 年"));
    }
}
