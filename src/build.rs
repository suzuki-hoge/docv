//! docv build。人間向けの HTML を生成する。docs/spec/build.md。

use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use pulldown_cmark::{CowStr, Event, Parser, Tag, html};
use serde_yaml_ng::Value;

use crate::kind::{Kind, Time};
use crate::markdown::{self, Document};
use crate::workspace::{Class, Project};

const STYLE: &str = include_str!("../assets/style.css");

/// docv が作った出力先であることを示すファイル。これがない出力先は消さない。
const MARKER: &str = ".docv";
const MARKER_TEXT: &str = "docv build の出力。このディレクトリは docv build のたびに作り直す。\n";

/// 管理対象のファイル 1 つから作るページ。
struct Page {
    source: String,
    /// 出力先からの相対パス。
    page: String,
    title: String,
    question: Option<String>,
    topics: Vec<String>,
    status: Option<String>,
    kind: Kind,
    document: Document,
}

pub fn run(project: &Project, out: &Path) -> Result<()> {
    let files = project.files()?;
    let mut pages = Vec::new();
    let mut unmanaged = Vec::new();
    for file in &files {
        match file.class {
            Class::Managed(index) => pages.push(read_page(project, &file.path, index)?),
            Class::Unmanaged => unmanaged.push(file.path.clone()),
        }
    }

    let out = prepare_out(out)?;
    let site = Site {
        project,
        out: &out,
        pages: &pages,
    };

    write(&out.join("assets/style.css"), STYLE)?;
    for page in &pages {
        write(&out.join(&page.page), &site.render_page(page))?;
    }
    write(&out.join("index.html"), &site.render_index(&unmanaged))?;
    if !project.config.output.commit {
        write(&out.join(".gitignore"), "*\n")?;
    }
    write(&out.join(MARKER), MARKER_TEXT)?;
    Ok(())
}

fn read_page(project: &Project, path: &str, index: usize) -> Result<Page> {
    let text = fs::read_to_string(project.root.join(path))
        .with_context(|| format!("{path} を読めない"))?;
    let document = markdown::split(&text);
    let outline = markdown::outline(&document);
    let frontmatter = document
        .frontmatter
        .as_ref()
        .and_then(|fm| serde_yaml_ng::from_str::<Value>(&fm.yaml).ok());
    let field = |key: &str| frontmatter.as_ref().and_then(|v| v.get(key));
    let string = |key: &str| field(key).and_then(Value::as_str).map(str::to_string);

    let entry = project.entry(index);
    let topics = entry.topics.clone().unwrap_or_else(|| {
        field("topics")
            .and_then(Value::as_sequence)
            .map(|seq| {
                seq.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    });
    let title = outline
        .headings
        .iter()
        .find(|h| h.level == 1)
        .map_or_else(|| path.to_string(), |h| h.text.clone());
    Ok(Page {
        source: path.to_string(),
        page: page_path(path),
        title,
        question: entry.question.clone().or_else(|| string("question")),
        topics,
        status: string("status"),
        kind: project.kind_of(index).clone(),
        document,
    })
}

fn page_path(source: &str) -> String {
    let stem = source.strip_suffix(".md").unwrap_or(source);
    format!("{stem}.html")
}

/// 出力先を空にして作る。docv が作ったものでない、空でないディレクトリは消さない。
fn prepare_out(out: &Path) -> Result<PathBuf> {
    if out.exists() {
        let is_empty = fs::read_dir(out)?.next().is_none();
        if !is_empty && !out.join(MARKER).is_file() {
            bail!(
                "出力先 {} は docv build が作ったディレクトリではないので消せない",
                out.display()
            );
        }
        fs::remove_dir_all(out).with_context(|| format!("{} を消せない", out.display()))?;
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
    pages: &'a [Page],
}

impl Site<'_> {
    fn render_page(&self, page: &Page) -> String {
        let dir = parent(&page.page);
        let base = Path::new(&page.source)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let parser =
            Parser::new_ext(&page.document.body, markdown::OPTIONS).map(|event| match event {
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => Event::Start(Tag::Link {
                    link_type,
                    dest_url: self.rewrite(&base, &dir, &dest_url, false),
                    title,
                    id,
                }),
                Event::Start(Tag::Image {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => Event::Start(Tag::Image {
                    link_type,
                    dest_url: self.rewrite(&base, &dir, &dest_url, true),
                    title,
                    id,
                }),
                other => other,
            });
        let mut body = String::new();
        html::push_html(&mut body, parser);

        let mut main = String::new();
        let _ = writeln!(
            main,
            "<div class=\"meta\">{}<span class=\"path\">{}</span></div>",
            badges(page),
            escape(&page.source)
        );
        if let Some(q) = &page.question {
            let _ = writeln!(main, "<p class=\"question\">{}</p>", escape(q));
        }
        let _ = writeln!(main, "<article class=\"doc\">\n{body}</article>");
        self.layout(&page.title, &dir, &main)
    }

    fn render_index(&self, unmanaged: &[String]) -> String {
        let mut main = String::new();
        let _ = writeln!(main, "<h1>{}</h1>", escape(&self.project.title()));

        let mut times: Vec<Time> = self.pages.iter().map(|p| p.kind.time).collect();
        times.sort();
        times.dedup();
        for time in times {
            let count = self.pages.iter().filter(|p| p.kind.time == time).count();
            let (open, close) = if time == Time::Past {
                (
                    format!(
                        "<details class=\"time-group\">\n<summary>{}（{count} 件）</summary>",
                        time.label()
                    ),
                    "</details>",
                )
            } else {
                (
                    format!("<section class=\"time-group\">\n<h2>{}</h2>", time.label()),
                    "</section>",
                )
            };
            let _ = writeln!(main, "{open}");
            for kind in self.project.kinds.all().iter().filter(|k| k.time == time) {
                let pages: Vec<&Page> =
                    self.pages.iter().filter(|p| p.kind.id == kind.id).collect();
                if pages.is_empty() {
                    continue;
                }
                let _ = writeln!(
                    main,
                    "<section class=\"kind-group\">\n<h3>{} <span class=\"count\">{}</span></h3>\n<ul class=\"docs\">",
                    escape(&kind.label),
                    pages.len()
                );
                for page in pages {
                    let _ = write!(
                        main,
                        "<li><a href=\"{}\">{}</a>",
                        escape(&page.page),
                        escape(&page.title)
                    );
                    if let Some(status) = &page.status {
                        let _ = write!(main, " <span class=\"status\">{}</span>", escape(status));
                    }
                    for topic in &page.topics {
                        let _ = write!(main, " <span class=\"topic\">{}</span>", escape(topic));
                    }
                    if let Some(q) = &page.question {
                        let _ = write!(main, "<p class=\"question\">{}</p>", escape(q));
                    }
                    let _ = writeln!(main, "</li>");
                }
                let _ = writeln!(main, "</ul>\n</section>");
            }
            let _ = writeln!(main, "{close}");
        }

        if !unmanaged.is_empty() {
            let _ = writeln!(
                main,
                "<section class=\"time-group\">\n<h2>その他</h2>\n<ul class=\"unmanaged\">"
            );
            for path in unmanaged {
                let href = self.outside_link(path, "", false);
                let _ = writeln!(
                    main,
                    "<li><a href=\"{}\">{}</a></li>",
                    escape(&href),
                    escape(path)
                );
            }
            let _ = writeln!(main, "</ul>\n</section>");
        }
        self.layout(&self.project.title(), "", &main)
    }

    fn layout(&self, title: &str, dir: &str, main: &str) -> String {
        let up = up_to_root(dir);
        let project = escape(&self.project.title());
        let heading = if title == self.project.title() {
            project.clone()
        } else {
            format!("{} - {project}", escape(title))
        };
        format!(
            "<!DOCTYPE html>
<html lang=\"ja\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>{heading}</title>
<link rel=\"stylesheet\" href=\"{up}assets/style.css\">
</head>
<body>
<header class=\"site\"><a href=\"{up}index.html\">{project}</a></header>
<main>
{main}</main>
</body>
</html>
"
        )
    }

    /// Markdown のリンク先を、HTML から辿れるリンク先にする。
    fn rewrite<'a>(
        &self,
        base: &Path,
        page_dir: &str,
        dest: &CowStr<'a>,
        image: bool,
    ) -> CowStr<'a> {
        let Some((target, fragment)) = markdown::resolve_link(base, dest) else {
            return dest.clone();
        };
        let target = slash_path(&target);
        let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
        if let Some(page) = self.pages.iter().find(|p| p.source == target) {
            let href = format!("{}{}{fragment}", up_to_root(page_dir), page.page);
            return CowStr::from(href);
        }
        CowStr::from(self.outside_link_from(page_dir, &target, &fragment, image))
    }

    fn outside_link(&self, target: &str, fragment: &str, image: bool) -> String {
        self.outside_link_from("", target, fragment, image)
    }

    /// 管理対象でないファイルへのリンク。repository.url があれば GitHub に、なければ元のファイルに向ける。
    fn outside_link_from(
        &self,
        page_dir: &str,
        target: &str,
        fragment: &str,
        image: bool,
    ) -> String {
        if let Some(repo) = &self.project.config.repository {
            let mode = if image { "raw" } else { "blob" };
            let url = repo.url.trim_end_matches('/');
            return format!("{url}/{mode}/{}/{target}{fragment}", repo.branch);
        }
        let from = self.out.join(page_dir);
        let to = self.project.root.join(target);
        format!("{}{fragment}", relative_path(&from, &to))
    }
}

fn badges(page: &Page) -> String {
    let time_class = match page.kind.time {
        Time::Present => "present",
        Time::Future => "future",
        Time::Past => "past",
    };
    let mut s = format!(
        "<span class=\"time time-{time_class}\">{}</span><span class=\"kind\">{}</span>",
        page.kind.time.label(),
        escape(&page.kind.label)
    );
    if let Some(status) = &page.status {
        let _ = write!(s, "<span class=\"status\">{}</span>", escape(status));
    }
    for topic in &page.topics {
        let _ = write!(s, "<span class=\"topic\">{}</span>", escape(topic));
    }
    s
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(String::new, |(dir, _)| dir.to_string())
}

/// 出力先の中のディレクトリから、出力先のルートへの相対パス（末尾に `/` が付く）。
fn up_to_root(dir: &str) -> String {
    if dir.is_empty() {
        return String::new();
    }
    "../".repeat(dir.split('/').count())
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// 絶対パス同士の相対パス。区切りは `/`。
fn relative_path(from_dir: &Path, to: &Path) -> String {
    let from: Vec<Component> = from_dir.components().collect();
    let to: Vec<Component> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
    parts.extend(
        to[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
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
            "version: 1\ntitle: demo\ntopics: [t]\ndocs:\n  - path: README.md\n    kind: overview\n    question: 何か\n  - path: docs/spec/*.md\n    kind: spec\n  - path: docs/archive/**/*.md\n    kind: archive\n",
        );
        for (path, text) in files {
            write_file(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn builds_pages_and_index_deterministically() {
        let (dir, project) = project(&[
            (
                "README.md",
                "# Demo\n\n[a](docs/spec/a.md) [memo](notes.md) ![img](docs/img.svg)\n",
            ),
            (
                "docs/spec/a.md",
                "---\nquestion: A は何か\ntopics: [t]\n---\n\n# A <仕様>\n\n[readme](../../README.md#top) [x](https://example.com)\n",
            ),
            (
                "docs/archive/spec/a.md",
                "---\nquestion: q\nsource: s\n---\n# 経緯\n",
            ),
            ("notes.md", "# Notes\n"),
        ]);
        let out = dir.path().join("docv");
        run(&project, &out).unwrap();

        let readme = fs::read_to_string(out.join("README.html")).unwrap();
        assert!(readme.contains("href=\"docs/spec/a.html\""), "{readme}");
        assert!(readme.contains("href=\"../notes.md\""), "{readme}");
        assert!(readme.contains("src=\"../docs/img.svg\""), "{readme}");

        let page = fs::read_to_string(out.join("docs/spec/a.html")).unwrap();
        assert!(page.contains("href=\"../../README.html#top\""), "{page}");
        assert!(page.contains("href=\"https://example.com\""), "{page}");
        assert!(
            page.contains("<p class=\"question\">A は何か</p>"),
            "{page}"
        );
        assert!(
            page.contains("<title>A &lt;仕様&gt; - demo</title>"),
            "{page}"
        );
        assert!(page.contains("href=\"../../assets/style.css\""), "{page}");

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        let present = index.find("<h2>現在</h2>").unwrap();
        let past = index.find("<summary>過去（1 件）</summary>").unwrap();
        let other = index.find("<h2>その他</h2>").unwrap();
        assert!(present < past && past < other, "{index}");
        assert!(index.contains("<span class=\"topic\">t</span>"), "{index}");
        assert!(index.contains("href=\"../notes.md\""), "{index}");

        run(&project, &out).unwrap();
        assert_eq!(fs::read_to_string(out.join("index.html")).unwrap(), index);
        assert!(out.join(MARKER).is_file());
        assert!(!out.join(".gitignore").exists());
    }

    #[test]
    fn links_outside_files_to_repository() {
        let (dir, mut project) =
            project(&[("README.md", "# Demo\n\n[memo](notes.md) ![img](a.svg)\n")]);
        project.config.repository = Some(crate::config::Repository {
            url: "https://github.com/o/r/".to_string(),
            branch: "main".to_string(),
        });
        project.config.output.commit = false;
        let out = dir.path().join("docv");
        run(&project, &out).unwrap();
        let readme = fs::read_to_string(out.join("README.html")).unwrap();
        assert!(
            readme.contains("href=\"https://github.com/o/r/blob/main/notes.md\""),
            "{readme}"
        );
        assert!(
            readme.contains("src=\"https://github.com/o/r/raw/main/a.svg\""),
            "{readme}"
        );
        assert_eq!(fs::read_to_string(out.join(".gitignore")).unwrap(), "*\n");
    }

    #[test]
    fn refuses_to_clear_foreign_directory() {
        let (dir, project) = project(&[("README.md", "# Demo\n")]);
        let out = dir.path().join("docs");
        write_file(dir.path(), "docs/keep.txt", "keep");
        let error = run(&project, &out).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("docv build が作ったディレクトリではない")
        );
        assert!(out.join("keep.txt").is_file());
    }

    #[test]
    fn computes_relative_paths() {
        assert_eq!(
            relative_path(Path::new("/a/out/docs"), Path::new("/a/notes.md")),
            "../../notes.md"
        );
        assert_eq!(up_to_root("docs/spec"), "../../");
        assert_eq!(up_to_root(""), "");
    }
}
