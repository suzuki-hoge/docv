//! YAML の Slot の中の Markdown。docs/spec/slot.md の 3 章。
//!
//! キーが `_markdown` で終わる項目の値を Markdown として扱い、`|` のブロックで書く。それ以外の文字列は 1 行で書く。
//! `|` のブロックは中身を 1 行ずつそのまま持つので、Markdown の N 行目は、YAML の「中身の 1 行目 + N」行目になる。

use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
use yaml_rust2::scanner::{Marker, TScalarStyle};

/// Markdown の項目のキーの終わり。
pub const SUFFIX: &str = "_markdown";

/// `|` で書いた Markdown の項目の値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// 中身の 1 行目の、YAML の行番号（1 から）。
    pub line: usize,
    /// 中身の行の字下げ（空白の数）。
    pub indent: usize,
    /// 字下げを除いた中身。
    pub text: String,
}

impl Block {
    /// 中身の行数。
    fn lines(&self) -> usize {
        self.text.lines().count()
    }
}

/// 書き方のルールに合わない項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub line: usize,
    pub rule: &'static str,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub blocks: Vec<Block>,
    pub problems: Vec<Problem>,
}

/// YAML の Slot の Markdown の項目を集め、書き方を確かめる。YAML として読めなければ None（slot-invalid が出る）。
pub fn scan(text: &str) -> Option<Scan> {
    let mut receiver = Receiver {
        source: text.lines().collect(),
        frames: Vec::new(),
        scan: Scan::default(),
    };
    Parser::new_from_str(text).load(&mut receiver, false).ok()?;
    Some(receiver.scan)
}

/// Markdown の項目の中身を差し替えた YAML。replaced は (項目, 新しい中身)。中身は字下げを付け直して書く。
pub fn replace(text: &str, replaced: &[(&Block, String)]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut sorted: Vec<&(&Block, String)> = replaced.iter().collect();
    // 後ろの項目から差し替え、前の項目の行番号をずらさない。
    sorted.sort_by_key(|(block, _)| std::cmp::Reverse(block.line));
    for (block, new) in sorted {
        let start = block.line - 1;
        let end = (start + block.lines()).min(lines.len());
        let indent = " ".repeat(block.indent);
        let new_lines = new.lines().map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{indent}{l}")
            }
        });
        lines.splice(start..end, new_lines);
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// YAML と同じ行数の Markdown。Markdown の項目の中身を同じ行に置き、ほかの行は空にする。
/// textlint の指摘の行番号が、そのまま YAML の行番号になる。項目どうしの間には必ず空の行が入る（キーの行）。
pub fn virtual_markdown(text: &str, blocks: &[Block]) -> String {
    let mut lines = vec![""; text.lines().count()];
    for block in blocks {
        for (i, line) in block.text.lines().enumerate() {
            if let Some(slot) = lines.get_mut(block.line - 1 + i) {
                *slot = line;
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// virtual_markdown を書き換えた結果から、項目ごとの新しい中身を取り出す。行数が変わっていたら None。
pub fn from_virtual(original: &str, fixed: &str, blocks: &[Block]) -> Option<Vec<String>> {
    let fixed: Vec<&str> = fixed.lines().collect();
    if fixed.len() != original.lines().count() {
        return None;
    }
    Some(
        blocks
            .iter()
            .map(|block| {
                let start = block.line - 1;
                let mut text = fixed[start..start + block.lines()].join("\n");
                text.push('\n');
                text
            })
            .collect(),
    )
}

enum Frame {
    /// key は、いま読んでいる値のキー。expecting_key なら、次のスカラーはキー。
    Map {
        key: Option<String>,
        expecting_key: bool,
    },
    /// 配列の要素は、配列を値に持つキーで確かめる。
    Seq { key: Option<String> },
}

struct Receiver<'a> {
    source: Vec<&'a str>,
    frames: Vec<Frame>,
    scan: Scan,
}

/// これから読む値の役割。
enum Role {
    Key,
    Value(Option<String>),
}

impl Receiver<'_> {
    fn role(&mut self) -> Role {
        match self.frames.last_mut() {
            Some(Frame::Map { expecting_key, .. }) if *expecting_key => Role::Key,
            Some(Frame::Map { key, expecting_key }) => {
                *expecting_key = true;
                Role::Value(key.take())
            }
            Some(Frame::Seq { key }) => Role::Value(key.clone()),
            None => Role::Value(None),
        }
    }

    fn scalar(&mut self, value: String, style: TScalarStyle, mark: Marker) {
        match self.role() {
            Role::Key => {
                if let Some(Frame::Map { key, expecting_key }) = self.frames.last_mut() {
                    *key = Some(value);
                    *expecting_key = false;
                }
            }
            Role::Value(key) => {
                let key = key.unwrap_or_default();
                if key.ends_with(SUFFIX) {
                    self.markdown(&key, value, style, mark);
                } else if matches!(style, TScalarStyle::Literal | TScalarStyle::Folded)
                    || value.contains('\n')
                {
                    self.problem(
                        mark.line(),
                        "slot-text-multiline",
                        format!("{key} は 1 行で書く。複数行の文章は、{SUFFIX} で終わる項目に書く"),
                    );
                }
            }
        }
    }

    fn markdown(&mut self, key: &str, value: String, style: TScalarStyle, mark: Marker) {
        if style != TScalarStyle::Literal {
            self.problem(
                mark.line(),
                "slot-markdown-style",
                format!("{key} は Markdown なので、| のブロックで書く"),
            );
            return;
        }
        // スカラーの位置は、中身の最初の空でない行。その前の空の行と、| の行を遡る。
        let blank = value.chars().take_while(|c| *c == '\n').count();
        let line = mark.line() - blank;
        let indicator = line - 1;
        let header = self.source.get(indicator - 1).copied().unwrap_or("");
        let header = header.split(" #").next().unwrap_or("").trim_end();
        if !header.ends_with(": |") && header != "|" && !header.ends_with(" |") {
            self.problem(
                indicator,
                "slot-markdown-style",
                format!("{key} は | だけで書く。|- や |+ や字下げの数は付けない"),
            );
            return;
        }
        if value.trim().is_empty() {
            return;
        }
        self.scan.blocks.push(Block {
            line,
            indent: mark.col(),
            text: value,
        });
    }

    fn problem(&mut self, line: usize, rule: &'static str, message: String) {
        self.scan.problems.push(Problem {
            line,
            rule,
            message,
        });
    }
}

impl MarkedEventReceiver for Receiver<'_> {
    fn on_event(&mut self, event: Event, mark: Marker) {
        match event {
            Event::Scalar(value, style, ..) => self.scalar(value, style, mark),
            Event::MappingStart(..) => {
                self.role();
                self.frames.push(Frame::Map {
                    key: None,
                    expecting_key: true,
                });
            }
            Event::SequenceStart(..) => {
                let key = match self.role() {
                    Role::Value(key) => key,
                    Role::Key => None,
                };
                self.frames.push(Frame::Seq { key });
            }
            Event::MappingEnd | Event::SequenceEnd => {
                self.frames.pop();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = "\
groups:
  - name: 基本
    items:
      - ja: 用語
        desc_markdown: |

          1 行目

          - 2
      - ja: 二つ目
        desc_markdown: |  # 注
          短い
";

    #[test]
    fn collects_markdown_blocks_with_lines() {
        let scan = scan(YAML).unwrap();
        assert_eq!(scan.problems, []);
        assert_eq!(
            scan.blocks,
            [
                Block {
                    line: 6,
                    indent: 10,
                    text: "\n1 行目\n\n- 2\n".to_string()
                },
                Block {
                    line: 12,
                    indent: 10,
                    text: "短い\n".to_string()
                },
            ]
        );
    }

    #[test]
    fn reports_markdown_not_in_literal_block_and_multiline_text() {
        let scan = scan(
            "a_markdown: 1 行\nb_markdown: |-\n  x\nc_markdown: >\n  y\nname: |\n  z\nlist:\n  - \"p\\nq\"\nok: 1 行\n",
        )
        .unwrap();
        let found: Vec<(usize, &str)> = scan.problems.iter().map(|p| (p.line, p.rule)).collect();
        assert_eq!(
            found,
            [
                (1, "slot-markdown-style"),
                (2, "slot-markdown-style"),
                (5, "slot-markdown-style"),
                (7, "slot-text-multiline"),
                (9, "slot-text-multiline"),
            ]
        );
        assert!(scan.blocks.is_empty());
        assert!(super::scan("a: [\n").is_none());
    }

    #[test]
    fn replaces_blocks_and_maps_virtual_markdown() {
        let scan = scan(YAML).unwrap();
        let virtual_md = virtual_markdown(YAML, &scan.blocks);
        assert_eq!(virtual_md.lines().count(), YAML.lines().count());
        assert_eq!(virtual_md.lines().nth(6), Some("1 行目"));
        let fixed = virtual_md.replace("短い", "短くない");
        let texts = from_virtual(&virtual_md, &fixed, &scan.blocks).unwrap();
        assert_eq!(texts[1], "短くない\n");
        let replaced = replace(
            YAML,
            &[
                (&scan.blocks[0], "1 行目\n\n- 2\n- 3\n".to_string()),
                (&scan.blocks[1], texts[1].clone()),
            ],
        );
        assert!(
            replaced.contains("desc_markdown: |\n          1 行目\n\n          - 2\n          - 3\n      - ja: 二つ目\n        desc_markdown: |  # 注\n          短くない\n"),
            "{replaced}"
        );
    }
}
