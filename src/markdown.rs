//! Markdown のファイルを、YAML front matter と、check が見る要素（見出し、リンク、地の文、タスク）に分ける。

use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// docv が Markdown を読むときの拡張。
pub const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_STRIKETHROUGH);

/// YAML front matter と本文に分けたファイル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub frontmatter: Option<Frontmatter>,
    pub body: String,
    /// 本文の 1 行目が、ファイルの何行目か（1 始まり）。
    pub body_start_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter {
    pub yaml: String,
    /// 閉じる `---` があるか。
    pub closed: bool,
}

impl Frontmatter {
    /// `key:` で始まる行が、ファイルの何行目か。見つからなければ 1 行目。
    pub fn line_of_key(&self, key: &str) -> usize {
        let prefix = format!("{key}:");
        self.yaml
            .lines()
            .position(|l| l.starts_with(&prefix))
            .map_or(1, |i| i + 2)
    }
}

pub fn split(text: &str) -> Document {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let is_delimiter = |line: &str, closing: bool| {
        let line = line.trim_end_matches(['\n', '\r']);
        line == "---" || (closing && line == "...")
    };
    if lines.first().is_none_or(|l| !is_delimiter(l, false)) {
        return Document {
            frontmatter: None,
            body: text.to_string(),
            body_start_line: 1,
        };
    }
    match lines.iter().skip(1).position(|l| is_delimiter(l, true)) {
        Some(i) => {
            let end = i + 1;
            Document {
                frontmatter: Some(Frontmatter {
                    yaml: lines[1..end].concat(),
                    closed: true,
                }),
                body: lines[end + 1..].concat(),
                body_start_line: end + 2,
            }
        }
        None => Document {
            frontmatter: Some(Frontmatter {
                yaml: lines[1..].concat(),
                closed: false,
            }),
            body: String::new(),
            body_start_line: lines.len() + 1,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    /// インラインコードも含めた見出しの文字列。
    pub text: String,
    /// インラインコードを除いた見出しの文字列。
    pub prose: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub dest: String,
    pub line: usize,
    /// 本文の中での、リンク全体（`[文字](先)`）の位置。
    pub span: Range<usize>,
}

/// コードブロックとインラインコードと見出しの外にある地の文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prose {
    pub text: String,
    pub line: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outline {
    pub headings: Vec<Heading>,
    /// リンクと画像の参照先。
    pub links: Vec<Link>,
    pub prose: Vec<Prose>,
    /// 終わったタスク（`- [x]`）の行。
    pub done_tasks: Vec<usize>,
}

/// 本文の中の位置から、ファイルの何行目かを引く。
pub struct LineIndex {
    starts: Vec<usize>,
    first_line: usize,
}

impl LineIndex {
    pub fn new(document: &Document) -> Self {
        let starts = std::iter::once(0)
            .chain(document.body.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        LineIndex {
            starts,
            first_line: document.body_start_line,
        }
    }

    pub fn line(&self, offset: usize) -> usize {
        self.first_line + self.starts.partition_point(|&s| s <= offset) - 1
    }
}

pub fn outline(document: &Document) -> Outline {
    let body = &document.body;
    let index = LineIndex::new(document);
    let line_of = |offset: usize| index.line(offset);

    let mut outline = Outline::default();
    let mut heading: Option<Heading> = None;
    let mut in_code_block = false;

    for (event, range) in Parser::new_ext(body, OPTIONS).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(Heading {
                    level: heading_level(level),
                    text: String::new(),
                    prose: String::new(),
                    line: line_of(range.start),
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut h) = heading.take() {
                    h.text = h.text.trim().to_string();
                    h.prose = h.prose.trim().to_string();
                    outline.headings.push(h);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                outline.links.push(Link {
                    dest: dest_url.to_string(),
                    line: line_of(range.start),
                    span: range.clone(),
                });
            }
            Event::Text(text) if !in_code_block => match heading.as_mut() {
                Some(h) => {
                    h.text.push_str(&text);
                    h.prose.push_str(&text);
                }
                None => outline.prose.push(Prose {
                    text: text.to_string(),
                    line: line_of(range.start),
                }),
            },
            Event::Code(code) => {
                if let Some(h) = heading.as_mut() {
                    h.text.push_str(&code);
                }
            }
            Event::TaskListMarker(true) => outline.done_tasks.push(line_of(range.start)),
            _ => {}
        }
    }
    outline
}

/// リンク先を、ルートからの相対パスと `#` 以降に分ける。ルートの中を指す相対リンクでなければ None。
/// base はリンク元のファイルがあるディレクトリ（ルートからの相対パス）。
pub fn resolve_link(base: &Path, dest: &str) -> Option<(PathBuf, Option<String>)> {
    if dest.is_empty()
        || dest.starts_with('#')
        || dest.contains("://")
        || dest.starts_with("mailto:")
    {
        return None;
    }
    let (path, fragment) = match dest.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment.to_string())),
        None => (dest, None),
    };
    let path = percent_decode(path.split('?').next().unwrap_or_default());
    let joined = match path.strip_prefix('/') {
        Some(rest) => PathBuf::from(rest),
        None => base.join(&path),
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::Normal(c) => normalized.push(c),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            _ => {}
        }
    }
    Some((normalized, fragment))
}

/// パスを `/` 区切りの文字列にする。
pub fn slash_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// from_dir から to への相対パス。どちらも絶対パスか、どちらもルートからの相対パス。区切りは `/`。
pub fn relative_path(from_dir: &Path, to: &Path) -> String {
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

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(b) = s
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_links() {
        let base = Path::new("docs/spec");
        assert_eq!(
            resolve_link(base, "../plan/a%20b.md#x"),
            Some((PathBuf::from("docs/plan/a b.md"), Some("x".to_string())))
        );
        assert_eq!(
            resolve_link(base, "/README.md"),
            Some((PathBuf::from("README.md"), None))
        );
        assert_eq!(resolve_link(base, "../../../outside.md"), None);
        assert_eq!(resolve_link(base, "https://example.com"), None);
        assert_eq!(resolve_link(base, "#top"), None);
    }

    #[test]
    fn splits_frontmatter() {
        let doc = split("---\nmission: q\ntopics: [a]\n---\n\n# T\n");
        let fm = doc.frontmatter.as_ref().unwrap();
        assert!(fm.closed);
        assert_eq!(fm.yaml, "mission: q\ntopics: [a]\n");
        assert_eq!(fm.line_of_key("topics"), 3);
        assert_eq!(doc.body, "\n# T\n");
        assert_eq!(doc.body_start_line, 5);
    }

    #[test]
    fn keeps_whole_text_without_frontmatter() {
        let doc = split("# T\n---\n");
        assert_eq!(doc.frontmatter, None);
        assert_eq!(doc.body_start_line, 1);
    }

    #[test]
    fn marks_unclosed_frontmatter() {
        let doc = split("---\nmission: q\n");
        assert!(!doc.frontmatter.unwrap().closed);
    }

    #[test]
    fn collects_outline_with_file_lines() {
        let text = "---\nmission: q\n---\n\n# T `code`\n\n以前は [a](a.md) `以前は`\n\n```\n以前は\n```\n\n- [x] done\n- [ ] todo\n\n![img](i.png)\n";
        let outline = outline(&split(text));
        assert_eq!(
            outline.headings,
            [Heading {
                level: 1,
                text: "T code".to_string(),
                prose: "T".to_string(),
                line: 5
            }]
        );
        let prose: Vec<(&str, usize)> = outline
            .prose
            .iter()
            .map(|p| (p.text.as_str(), p.line))
            .collect();
        assert_eq!(
            prose,
            [
                ("以前は ", 7),
                ("a", 7),
                (" ", 7),
                ("done", 13),
                ("todo", 14),
                ("img", 16)
            ]
        );
        let links: Vec<(&str, usize)> = outline
            .links
            .iter()
            .map(|l| (l.dest.as_str(), l.line))
            .collect();
        assert_eq!(links, [("a.md", 7), ("i.png", 16)]);
        assert_eq!(outline.done_tasks, [13]);
    }
}
