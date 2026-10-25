//! 現在の設定のページに並べる、docv.yaml のすべての項目と今の値。docv/service/service/html.md の「現在の設定のページ」。

use crate::config::{Config, Permission, Switch};

/// 項目 1 つ。key は docv.yaml のキーを `.` でつないだもの。
#[derive(Debug, PartialEq, Eq)]
pub struct Row {
    pub key: &'static str,
    pub value: Cell,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Cell {
    /// 値が 1 つの項目。
    Value(String),
    /// 配列の項目。空なら値がない。
    List(Vec<String>),
    /// タグの一覧。HTML ではタグの色で出す。空なら値がない。
    Tags(Vec<String>),
    /// 候補から選ぶ項目。選んでいる候補は true。
    Choice(Vec<(String, bool)>),
}

/// docv.yaml のすべての項目を、スキーマの順に返す。書いていない項目は、既定の値にする。
/// title は、省略したときの値（ルートのディレクトリ名）を受け取る。
pub fn rows(config: &Config, title: &str) -> Vec<Row> {
    let row = |key, value| Row { key, value };
    let list = |items: &[String]| Cell::List(items.to_vec());
    vec![
        row("title", Cell::Value(title.to_string())),
        row("dir.free", Cell::Value(config.free_dir())),
        row("dir.slot", Cell::Value(config.slot_dir())),
        row("dir.html", Cell::Value(config.html_dir())),
        // dir.image を書いていなければ、値のない項目として `-` を出す。
        row(
            "dir.image",
            config
                .image_dir()
                .map_or(Cell::List(Vec::new()), Cell::Value),
        ),
        row("ignore", list(&config.ignore)),
        row(
            "tags",
            Cell::Tags(config.tags.iter().map(|t| t.name.clone()).collect()),
        ),
        row("lint.bold", permission(config.lint.bold)),
        row("lint.italic", permission(config.lint.italic)),
        row("lint.dollar", permission(config.lint.dollar)),
        row(
            "lint.textlint",
            choice(
                &["enable", "disable"],
                config.lint.textlint == Switch::Enable,
            ),
        ),
        row("format.enabled", boolean(config.format.enabled)),
    ]
}

fn boolean(value: bool) -> Cell {
    choice(&["true", "false"], value)
}

fn permission(value: Permission) -> Cell {
    choice(&["allow", "deny"], value == Permission::Allow)
}

/// 2 つの候補。first が true なら 1 つ目を選んでいる。
fn choice(candidates: &[&str; 2], first: bool) -> Cell {
    Cell::Choice(vec![
        (candidates[0].to_string(), first),
        (candidates[1].to_string(), !first),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    /// 値を 1 行ずつ並べる。選ぶ項目は `[x]` か `[ ]` を付け、配列は `- ` を付ける。
    fn lines(rows: &[Row]) -> String {
        let mut out = String::new();
        for row in rows {
            out.push_str(&format!("{}\n", row.key));
            match &row.value {
                Cell::Value(text) => out.push_str(&format!("  {text}\n")),
                Cell::List(items) | Cell::Tags(items) => {
                    for item in items {
                        out.push_str(&format!("  - {item}\n"));
                    }
                }
                Cell::Choice(items) => {
                    for (text, selected) in items {
                        let mark = if *selected { "x" } else { " " };
                        out.push_str(&format!("  [{mark}] {text}\n"));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn lists_every_item_with_current_values() {
        let config = config::parse(
            "tags:
  - name: 仕様
    color: red
dir:
  image: out/jpg
lint:
  bold: deny
  textlint: enable
",
        )
        .unwrap();
        assert_eq!(
            lines(&rows(&config, "repo")),
            "\
title
  repo
dir.free
  docs
dir.slot
  docv
dir.html
  output/docv/html
dir.image
  out/jpg
ignore
tags
  - 仕様
lint.bold
  [ ] allow
  [x] deny
lint.italic
  [x] allow
  [ ] deny
lint.dollar
  [x] allow
  [ ] deny
lint.textlint
  [x] enable
  [ ] disable
format.enabled
  [ ] true
  [x] false
"
        );
    }
}
