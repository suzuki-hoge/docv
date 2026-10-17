//! docv init。推奨する配置の docv.yaml を作る。docv/service/manual.md の docv init。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::schema;

/// 推奨する配置（docs/spec/config.md の 7 章）の docv.yaml。すべての項目を、既定の値でコメントに載せる。
pub fn config_text() -> String {
    format!(
        "{}# docv の設定。項目の型と意味は、1 行目の URL のスキーマにある。
# コメントにした項目は、使うときにコメントを外して書き換える。

# HTML に出すプロジェクト名。省略するとルートのディレクトリ名
# title: my-project

# docv が読み書きするディレクトリ。どれもルートからの相対パス
dir:
  # 管理対象の Markdown（Free ファイル）を置くディレクトリ。各ファイルの YAML front matter に mission を書く
  free: docs
  # Slot ファイルを置くディレクトリ
  slot: docv
  # docv build が HTML を出すディレクトリ
  html: output/docv/html
  # docv build が Slot のページの JPG を出すディレクトリ。書かなければ JPG を作らない。JPG を作るには Chrome が要る
  # image: output/docv/image

# 管理対象にも管理外にもしないファイルの glob
ignore:
  - docs/tmp/**

# 使うタグの一覧。YAML front matter の tags には、ここにあるタグだけを書ける
# color は HTML のタグの色（blue、green、yellow、red、purple、orange、teal、pink）
# tags:
#   - name: 仕様
#     color: blue
#   - name: 手順
#     color: green

lint:
  # 太字を使ってよいか（allow、deny）
  bold: allow
  # 斜体を使ってよいか（allow、deny）
  italic: allow
  # コードの外の 1 行に $ を 2 つ以上書いてよいか（allow、deny）。GitHub は $ で挟んだ部分を数式として表示する
  dollar: allow
  # textlint で文章を校正するか（enable、disable）。ルールは docv が決める。使うには Node.js が要る
  textlint: disable

format:
  # check で、整形が要るファイルを error にするか
  enabled: false
",
        schema::header("docv.schema.json")
    )
}

/// path は作る docv.yaml のパス。すでにあれば、force のときだけ書き直す。
pub fn run(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "{} はすでにある。書き直すなら --force を付ける",
            path.display()
        );
    }
    fs::write(path, config_text()).with_context(|| format!("{} を書けない", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn writes_valid_config_unless_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("docv.yaml");
        run(&path, false).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(&format!(
            "# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/",
            env!("CARGO_PKG_VERSION")
        )));
        let config = config::parse(&text).unwrap();
        assert_eq!(config.free_dir(), "docs");
        assert_eq!(config.slot_dir(), "docv");
        assert_eq!(config.image_dir(), None);
        let error = run(&path, false).unwrap_err().to_string();
        assert!(error.contains("すでにある"), "{error}");
        fs::write(&path, "dir:\n  free: notes\n").unwrap();
        run(&path, true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), config_text());
    }

    #[test]
    fn commented_items_are_valid_when_uncommented() {
        // 項目をコメントにした行（`# key: value` と `# - item`）を戻し、説明のコメントの行は捨てる
        let is_item = |line: &str| {
            let t = line.trim_start();
            let key = t.split_once(": ").map(|(k, _)| k).or(t.strip_suffix(':'));
            t.starts_with("- ")
                || key.is_some_and(|k| !k.is_empty() && !k.contains([' ', '-', '#']))
        };
        let text: String = config_text()
            .lines()
            .filter_map(|line| match line.strip_prefix("# ") {
                Some(rest) => is_item(rest).then_some(rest),
                None => Some(line),
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("title: my-project"), "{text}");
        config::parse(&text).unwrap_or_else(|e| panic!("{e:#}\n{text}"));
    }
}
