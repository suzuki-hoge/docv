//! 書式の規則。lint の bold、italic、dollar（docs/spec/config.md の 4 章）と、
//! 常に確かめる章番号とリンクの表示文字（docs/spec/free.md の 3 章）。

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::config::{Config, Permission};
use crate::markdown::{self, Document, LineIndex};

/// 1 つのファイルにかける書式の規則。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub dollar: bool,
    /// リンクの表示文字のパスの起点（docs のディレクトリ）。None ならルートからのパス。
    pub link_base: Option<String>,
}

impl Style {
    pub fn new(config: &Config) -> Style {
        let deny = |p: Permission| p == Permission::Deny;
        Style {
            bold: deny(config.lint.bold),
            italic: deny(config.lint.italic),
            dollar: deny(config.lint.dollar),
            link_base: Some(config.free_dir()),
        }
    }
}

/// リンクの表示文字として求めるパス。from と to はルートからの相対パス。
/// リンク元とリンク先がどちらも base の中にあれば base からのパス、そうでなければルートからのパス。
pub fn link_text(base: Option<&str>, from: &str, to: &str) -> String {
    let inside = |path: &str, base: &str| {
        path.strip_prefix(base)
            .and_then(|rest| rest.strip_prefix('/'))
            .map(str::to_string)
    };
    match base {
        Some(base) if inside(from, base).is_some() => {
            inside(to, base).unwrap_or_else(|| to.to_string())
        }
        _ => to.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub line: usize,
    pub rule: &'static str,
    pub message: String,
}

/// path はルートからの相対パス。managed は管理対象のファイルのパスで、リンクの表示文字に使う。
pub fn check(
    style: &Style,
    path: &str,
    document: &Document,
    managed: &HashSet<String>,
) -> Vec<Found> {
    let body = &document.body;
    let index = LineIndex::new(document);
    let base_dir = Path::new(path).parent().unwrap_or(Path::new(""));
    let mut found = Vec::new();
    let mut push = |line: usize, rule: &'static str, message: String| {
        found.push(Found {
            line,
            rule,
            message,
        });
    };

    let mut in_code_block = false;
    // 見出しとリンクの中の文字を集める。（行、文字、求める表示文字）
    let mut heading: Option<(usize, String)> = None;
    let mut h2: Vec<(usize, String)> = Vec::new();
    let mut link: Option<(usize, String, String)> = None;
    let mut dollars: BTreeMap<usize, usize> = BTreeMap::new();

    for (event, range) in Parser::new_ext(body, markdown::OPTIONS).into_offset_iter() {
        let line = index.line(range.start);
        match &event {
            Event::Start(tag) => match tag {
                Tag::Heading {
                    level: HeadingLevel::H2,
                    ..
                } => heading = Some((line, String::new())),
                Tag::CodeBlock(_) => in_code_block = true,
                Tag::Strong if style.bold => push(line, "style-bold", "太字を使わない".to_string()),
                Tag::Emphasis if style.italic => {
                    push(line, "style-italic", "斜体を使わない".to_string())
                }
                Tag::Link { dest_url, .. } => {
                    if let Some(expected) = expected_text(style, path, base_dir, dest_url, managed)
                    {
                        link = Some((line, String::new(), expected));
                    }
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(HeadingLevel::H2) => h2.extend(heading.take()),
                TagEnd::CodeBlock => in_code_block = false,
                TagEnd::Link => {
                    if let Some((line, text, expected)) = link.take()
                        && text.trim() != expected
                    {
                        push(
                            line,
                            "link-text",
                            format!("リンクの表示文字を {expected} にする"),
                        );
                    }
                }
                _ => {}
            },
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, collected)) = heading.as_mut() {
                    collected.push_str(text);
                }
                if let Some((_, collected, _)) = link.as_mut() {
                    collected.push_str(text);
                }
                if matches!(event, Event::Text(_)) && !in_code_block && style.dollar {
                    count_dollars(body, range.clone(), &index, &mut dollars);
                }
            }
            _ => {}
        }
    }

    // h2 のどれかが章番号で始まっていれば、すべての h2 に 1 から順に章番号を求める。
    if h2.iter().any(|(_, text)| has_number(text)) {
        for (i, (line, text)) in h2.iter().enumerate() {
            let number = format!("{}. ", i + 1);
            if !text.trim().starts_with(&number) {
                push(
                    *line,
                    "heading-number",
                    format!("h2 の見出しを「{number}」で始める。章番号は 1 から順に振る"),
                );
            }
        }
    }
    for (line, count) in dollars {
        if count >= 2 {
            push(
                line,
                "style-dollar",
                "1 行に $ が 2 つ以上ある。GitHub は $ で挟んだ部分を数式として表示する"
                    .to_string(),
            );
        }
    }
    found.sort_by_key(|f| f.line);
    found
}

/// 見出しが `1. ` のような章番号で始まっているか。
fn has_number(text: &str) -> bool {
    let text = text.trim();
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && text[digits..].starts_with(". ")
}

/// 管理対象のファイルを指すリンクなら、求める表示文字を返す。
fn expected_text(
    style: &Style,
    path: &str,
    base_dir: &Path,
    dest: &str,
    managed: &HashSet<String>,
) -> Option<String> {
    let (target, _) = markdown::resolve_link(base_dir, dest)?;
    let target = markdown::slash_path(&target);
    managed
        .contains(&target)
        .then(|| link_text(style.link_base.as_deref(), path, &target))
}

/// テキストの元の文字列から、`\` でエスケープしていない `$` を行ごとに数える。
fn count_dollars(
    body: &str,
    range: std::ops::Range<usize>,
    index: &LineIndex,
    dollars: &mut BTreeMap<usize, usize>,
) {
    let source = &body[range.clone()];
    let mut escaped = false;
    for (i, c) in source.char_indices() {
        if c == '$' && !escaped {
            *dollars.entry(index.line(range.start + i)).or_default() += 1;
        }
        escaped = c == '\\' && !escaped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(style: &Style, path: &str, text: &str) -> Vec<String> {
        let managed: HashSet<String> = ["docs/spec/a.md", "docs/spec/b.md", "README.md"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        check(style, path, &markdown::split(text), &managed)
            .into_iter()
            .map(|f| format!("{}: {} {}", f.line, f.rule, f.message))
            .collect()
    }

    fn all() -> Style {
        Style {
            bold: true,
            italic: true,
            dollar: true,
            link_base: Some("docs".to_string()),
        }
    }

    #[test]
    fn reads_lint_and_docs() {
        let config = crate::config::parse("lint:\n  bold: deny\n  dollar: allow\n").unwrap();
        assert_eq!(
            Style::new(&config),
            Style {
                bold: true,
                italic: false,
                dollar: false,
                link_base: Some("docs".to_string()),
            }
        );
    }

    #[test]
    fn passes_valid_text() {
        let text = "---\nmission: q\n---\n\n# A\n\n## 1. 一\n\n[spec/b.md](b.md) [README.md](../../README.md) [外](https://example.com) `$a $b` 3 ドル\n\n<b>x</b> ![](x.png)\n\n```\n**$x$**\n```\n\n## 2. 二\n";
        assert_eq!(run(&all(), "docs/spec/a.md", text), Vec::<String>::new());
    }

    #[test]
    fn reports_each_rule() {
        let text = "# A\n\n## 一\n\n**b** _i_ $1 と $2\n\n[b](b.md) [docs/spec/b.md](b.md#x)\n\n## 3. 三\n";
        assert_eq!(
            run(&all(), "docs/spec/a.md", text),
            [
                "3: heading-number h2 の見出しを「1. 」で始める。章番号は 1 から順に振る",
                "5: style-bold 太字を使わない",
                "5: style-italic 斜体を使わない",
                "5: style-dollar 1 行に $ が 2 つ以上ある。GitHub は $ で挟んだ部分を数式として表示する",
                "7: link-text リンクの表示文字を spec/b.md にする",
                "7: link-text リンクの表示文字を spec/b.md にする",
                "9: heading-number h2 の見出しを「2. 」で始める。章番号は 1 から順に振る",
            ]
        );
    }

    #[test]
    fn uses_root_path_outside_base() {
        assert_eq!(
            run(&all(), "README.md", "# R\n\n[spec/a.md](docs/spec/a.md)\n"),
            ["3: link-text リンクの表示文字を docs/spec/a.md にする"]
        );
        assert_eq!(
            link_text(Some("docs"), "docs/spec/a.md", "README.md"),
            "README.md"
        );
        assert_eq!(link_text(None, "docs/a.md", "docs/b.md"), "docs/b.md");
    }

    #[test]
    fn checks_only_fixed_rules_by_default() {
        // 章番号のない h2 は確かめない。リンクの表示文字は、起点がなければルートからのパスを求める
        let text = "# A\n\n## x\n\n**b** _i_ $1 $2\n\n[b](b.md) [docs/spec/b.md](b.md)\n\n## y\n";
        assert_eq!(
            run(&Style::default(), "docs/spec/a.md", text),
            ["7: link-text リンクの表示文字を docs/spec/b.md にする"]
        );
    }
}
