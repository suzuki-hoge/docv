//! 書式の規則（lint.style）。docs/spec/config.md の 5 章。

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde_json::Value;

use crate::markdown::{self, Document, LineIndex};

/// 1 つのファイルに効く書式の規則。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    /// 使ってよい HTML タグ。None なら確かめない。
    pub html: Option<Vec<String>>,
    pub dollar: bool,
    pub heading_number: bool,
    pub section_separator: bool,
    /// link_text の base。None なら確かめない。
    pub link_text: Option<String>,
    pub image_alt: bool,
}

impl Style {
    /// lint.style に、docs の項目の style を上書きする。false を書いた規則は使わない。
    pub fn resolve(base: Option<&Value>, over: Option<&Value>) -> Style {
        let mut rules = serde_json::Map::new();
        for value in [base, over].into_iter().flatten() {
            if let Value::Object(map) = value {
                rules.extend(map.clone());
            }
        }
        let is = |key: &str, word: &str| rules.get(key).and_then(Value::as_str) == Some(word);
        Style {
            bold: is("bold", "deny"),
            italic: is("italic", "deny"),
            html: rules.get("html").and_then(Value::as_array).map(|tags| {
                tags.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_ascii_lowercase)
                    .collect()
            }),
            dollar: is("dollar", "deny"),
            heading_number: is("heading_number", "require"),
            section_separator: is("section_separator", "require"),
            link_text: rules
                .get("link_text")
                .and_then(|v| v.get("base"))
                .and_then(Value::as_str)
                .map(|base| {
                    base.trim_start_matches("./")
                        .trim_end_matches('/')
                        .to_string()
                }),
            image_alt: is("image_alt", "require"),
        }
    }
}

/// link_text の規則で求める表示文字。from と to はルートからの相対パス。
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

/// path はルートからの相対パス。managed は管理対象のファイル（tmp を除く）のパスで、link_text に使う。
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

    let mut depth = 0usize;
    let mut previous_is_rule = false;
    let mut h2_count = 0usize;
    let mut in_code_block = false;
    let mut in_comment = false;
    // 見出し・リンク・画像の中の文字を集める。（行、文字、求める表示文字）
    let mut heading: Option<(usize, String)> = None;
    let mut link: Option<(usize, String, String)> = None;
    let mut image: Option<(usize, String)> = None;
    let mut dollars: BTreeMap<usize, usize> = BTreeMap::new();

    for (event, range) in Parser::new_ext(body, markdown::OPTIONS).into_offset_iter() {
        let line = index.line(range.start);
        match &event {
            Event::Start(tag) => {
                if depth == 0 {
                    if matches!(
                        tag,
                        Tag::Heading {
                            level: HeadingLevel::H2,
                            ..
                        }
                    ) {
                        h2_count += 1;
                        if style.section_separator && h2_count > 1 && !previous_is_rule {
                            push(
                                line,
                                "style-section-separator",
                                "章（h2）の直前に --- を置く".to_string(),
                            );
                        }
                    }
                    previous_is_rule = false;
                }
                depth += 1;
                match tag {
                    Tag::Heading {
                        level: HeadingLevel::H2,
                        ..
                    } => heading = Some((line, String::new())),
                    Tag::CodeBlock(_) => in_code_block = true,
                    Tag::Strong if style.bold => {
                        push(line, "style-bold", "太字を使わない".to_string())
                    }
                    Tag::Emphasis if style.italic => {
                        push(line, "style-italic", "斜体を使わない".to_string())
                    }
                    Tag::Link { dest_url, .. } => {
                        if let Some(expected) =
                            expected_text(style, path, base_dir, dest_url, managed)
                        {
                            link = Some((line, String::new(), expected));
                        }
                    }
                    Tag::Image { .. } if style.image_alt => image = Some((line, String::new())),
                    _ => {}
                }
            }
            Event::End(tag) => {
                depth = depth.saturating_sub(1);
                match tag {
                    TagEnd::Heading(HeadingLevel::H2) => {
                        if let Some((line, text)) = heading.take()
                            && style.heading_number
                        {
                            let number = format!("{h2_count}. ");
                            if !text.trim().starts_with(&number) {
                                push(
                                    line,
                                    "style-heading-number",
                                    format!(
                                        "h2 の見出しを「{number}」で始める。章番号は 1 から順に振る"
                                    ),
                                );
                            }
                        }
                    }
                    TagEnd::CodeBlock => in_code_block = false,
                    TagEnd::Link => {
                        if let Some((line, text, expected)) = link.take()
                            && text.trim() != expected
                        {
                            push(
                                line,
                                "style-link-text",
                                format!("リンクの表示文字を {expected} にする"),
                            );
                        }
                    }
                    TagEnd::Image => {
                        if let Some((line, alt)) = image.take()
                            && alt.trim().is_empty()
                        {
                            push(
                                line,
                                "style-image-alt",
                                "画像に代替テキストを書く".to_string(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            Event::Rule if depth == 0 => previous_is_rule = true,
            Event::Text(text) | Event::Code(text) => {
                for (_, collected) in [&mut heading, &mut image].into_iter().flatten() {
                    collected.push_str(text);
                }
                if let Some((_, collected, _)) = link.as_mut() {
                    collected.push_str(text);
                }
                if matches!(event, Event::Text(_)) && !in_code_block && style.dollar {
                    count_dollars(body, range.clone(), &index, &mut dollars);
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                if let Some(allowed) = &style.html {
                    for (offset, tag) in html_tags(html, &mut in_comment) {
                        if !allowed.contains(&tag) {
                            push(
                                index.line(range.start + offset),
                                "style-html",
                                format!("HTML タグ <{tag}> を使わない"),
                            );
                        }
                    }
                }
            }
            _ => {}
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

/// 管理対象のファイルを指すリンクなら、求める表示文字を返す。
fn expected_text(
    style: &Style,
    path: &str,
    base_dir: &Path,
    dest: &str,
    managed: &HashSet<String>,
) -> Option<String> {
    let base = style.link_text.as_deref()?;
    let (target, _) = markdown::resolve_link(base_dir, dest)?;
    let target = markdown::slash_path(&target);
    managed
        .contains(&target)
        .then(|| link_text(Some(base), path, &target))
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

/// HTML の中の開きタグの名前と、その位置。コメントの中は見ない。in_comment はイベントをまたいで持ち越す。
fn html_tags(html: &str, in_comment: &mut bool) -> Vec<(usize, String)> {
    let mut tags = Vec::new();
    let mut offset = 0;
    let mut rest = html;
    loop {
        if *in_comment {
            match rest.find("-->") {
                Some(i) => {
                    offset += i + 3;
                    rest = &rest[i + 3..];
                    *in_comment = false;
                }
                None => break,
            }
        }
        let Some(i) = rest.find('<') else {
            break;
        };
        let after = &rest[i..];
        if after.starts_with("<!--") {
            *in_comment = true;
            offset += i + 4;
            rest = &rest[i + 4..];
            continue;
        }
        let name: String = after[1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
            tags.push((offset + i, name.to_ascii_lowercase()));
        }
        offset += i + 1;
        rest = &rest[i + 1..];
    }
    tags
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
        Style::resolve(
            Some(&json!({
                "bold": "deny",
                "italic": "deny",
                "html": ["details", "summary"],
                "dollar": "deny",
                "heading_number": "require",
                "section_separator": "require",
                "link_text": { "base": "docs" },
                "image_alt": "require",
            })),
            None,
        )
    }

    #[test]
    fn resolves_overrides() {
        let style = Style::resolve(
            Some(&json!({ "bold": "deny", "heading_number": "require" })),
            Some(&json!({ "heading_number": false, "html": [] })),
        );
        assert!(style.bold);
        assert!(!style.heading_number);
        assert_eq!(style.html, Some(Vec::new()));
    }

    #[test]
    fn passes_valid_text() {
        let text = "---\nmission: q\n---\n\n# A\n\n## 1. 一\n\n[spec/b.md](b.md) [README.md](../../README.md) [外](https://example.com) `$a $b` 3 ドル\n\n<details><summary>s</summary>\n\n<!-- <b>x</b>\n<i> -->\n\n</details>\n\n![図](x.png)\n\n```\n**$x$**\n```\n\n---\n\n## 2. 二\n";
        assert_eq!(run(&all(), "docs/spec/a.md", text), Vec::<String>::new());
    }

    #[test]
    fn reports_each_rule() {
        let text = "# A\n\n## 一\n\n**b** _i_ <b>x</b> $1 と $2\n\n[b](b.md) [docs/spec/b.md](b.md#x) ![](x.png)\n\n## 3. 三\n";
        assert_eq!(
            run(&all(), "docs/spec/a.md", text),
            [
                "3: style-heading-number h2 の見出しを「1. 」で始める。章番号は 1 から順に振る",
                "5: style-bold 太字を使わない",
                "5: style-italic 斜体を使わない",
                "5: style-html HTML タグ <b> を使わない",
                "5: style-dollar 1 行に $ が 2 つ以上ある。GitHub は $ で挟んだ部分を数式として表示する",
                "7: style-link-text リンクの表示文字を spec/b.md にする",
                "7: style-link-text リンクの表示文字を spec/b.md にする",
                "7: style-image-alt 画像に代替テキストを書く",
                "9: style-section-separator 章（h2）の直前に --- を置く",
                "9: style-heading-number h2 の見出しを「2. 」で始める。章番号は 1 から順に振る",
            ]
        );
    }

    #[test]
    fn uses_root_path_outside_base() {
        assert_eq!(
            run(&all(), "README.md", "# R\n\n[spec/a.md](docs/spec/a.md)\n"),
            ["3: style-link-text リンクの表示文字を docs/spec/a.md にする"]
        );
        assert_eq!(
            link_text(Some("docs"), "docs/spec/a.md", "README.md"),
            "README.md"
        );
        assert_eq!(link_text(None, "docs/a.md", "docs/b.md"), "docs/b.md");
    }

    #[test]
    fn checks_nothing_by_default() {
        let text = "# A\n\n## x\n\n**b** <b>x</b> $1 $2\n\n## y\n";
        assert!(run(&Style::default(), "docs/spec/a.md", text).is_empty());
    }
}
