//! Markdown の整形。整形の形は docv/service/service/config.md の「整形との連動」。

use anyhow::{Result, anyhow, bail};
use dprint_plugin_markdown::configuration::{
    Configuration, ConfigurationBuilder, EmphasisKind, HeadingKind, ListUnorderedMarker,
    StrongKind, TextWrap,
};

use crate::markdown;

fn configuration() -> Configuration {
    ConfigurationBuilder::new()
        .text_wrap(TextWrap::Maintain)
        .emphasis_kind(EmphasisKind::Asterisks)
        .strong_kind(StrongKind::Asterisks)
        .heading_kind(HeadingKind::Atx)
        .list_unordered_marker(ListUnorderedMarker::Dashes)
        .max_blank_lines(1)
        .build()
}

/// ファイル全体を整形する。YAML front matter は中身を変えず、本文だけを dprint-plugin-markdown で整形する。
pub fn format(text: &str) -> Result<String> {
    let text = text.replace("\r\n", "\n");
    let document = markdown::split(&text);
    let mut head = match &document.frontmatter {
        None => String::new(),
        Some(fm) if !fm.closed => bail!("YAML front matter を閉じる --- がない"),
        Some(_) => text
            .split_inclusive('\n')
            .take(document.body_start_line - 1)
            .collect(),
    };
    if !head.is_empty() && !head.ends_with('\n') {
        head.push('\n');
    }
    let body =
        dprint_plugin_markdown::format_text(&document.body, &configuration(), |_, _, _| Ok(None))
            .map_err(|e| anyhow!("Markdown として整形できない: {e}"))?
            .unwrap_or_else(|| document.body.clone());
    let body = body.trim_start_matches('\n');
    Ok(match (head.is_empty(), body.trim().is_empty()) {
        (true, _) => body.to_string(),
        (false, true) => head,
        (false, false) => format!("{head}\n{body}"),
    })
}

/// YAML のファイル（docv.yaml と YAML の Slot）の整形。末尾の改行を 1 つにする。中身は変えない。
pub fn final_newline(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let body = text.trim_end_matches('\n');
    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

/// 2 つの文字列で、最初に違う行（1 始まり）。
pub fn first_diff_line(a: &str, b: &str) -> usize {
    let mut a_lines = a.split_inclusive('\n');
    let mut b_lines = b.split_inclusive('\n');
    let mut line = 1;
    loop {
        match (a_lines.next(), b_lines.next()) {
            (Some(x), Some(y)) if x == y => line += 1,
            _ => return line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_body_and_keeps_frontmatter() {
        let text = "---\r\nmission:  q  \r\ntopics: [a]\r\n---\r\n# T\r\nあいう _em_ __strong__\r\n\r\n\r\n\r\n* a\r\n* b\r\n\r\n| 種類 | x |\r\n|---|---|\r\n| ああ | b |\r\n\r\nSetext\r\n===\r\n\r\n```rust\r\nfn  main(){}\r\n```\r\n長い 行は 折り返さない。長い 行は 折り返さない。長い 行は 折り返さない。長い 行は 折り返さない。";
        let expected = "---\nmission:  q  \ntopics: [a]\n---\n\n# T\n\nあいう *em* **strong**\n\n- a\n- b\n\n| 種類 | x |\n| ---- | - |\n| ああ | b |\n\n# Setext\n\n```rust\nfn  main(){}\n```\n\n長い 行は 折り返さない。長い 行は 折り返さない。長い 行は 折り返さない。長い 行は 折り返さない。\n";
        let formatted = format(text).unwrap();
        assert_eq!(formatted, expected);
        assert_eq!(format(&formatted).unwrap(), formatted);
    }

    #[test]
    fn handles_empty_body_and_unclosed_frontmatter() {
        assert_eq!(
            format("---\nmission: q\n---").unwrap(),
            "---\nmission: q\n---\n"
        );
        assert_eq!(format("").unwrap(), "");
        assert!(format("---\nmission: q\n").is_err());
    }

    #[test]
    fn finds_first_different_line() {
        assert_eq!(first_diff_line("a\nb\nc\n", "a\nb\nd\n"), 3);
        assert_eq!(first_diff_line("a\n", "a\nb\n"), 2);
    }

    #[test]
    fn keeps_one_final_newline() {
        assert_eq!(final_newline("a: 1"), "a: 1\n");
        assert_eq!(final_newline("a: 1\r\n\n\n"), "a: 1\n");
        assert_eq!(final_newline(""), "");
    }
}
