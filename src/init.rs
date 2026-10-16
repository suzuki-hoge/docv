//! docv init。推奨する配置の docv.yaml を作る。docv/service/manual.md の docv init。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// 実行した CLI の版のスキーマの URL。
pub fn schema_url() -> String {
    format!(
        "https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/assets/schema/docv.schema.json",
        env!("CARGO_PKG_VERSION")
    )
}

/// 推奨する配置（docs/spec/config.md の 9 章）の docv.yaml。すべての項目を、既定の値でコメントに載せる。
pub fn config_text() -> String {
    format!(
        "# yaml-language-server: $schema={}
# docv の設定。項目の型と意味は、1 行目の URL のスキーマにある。
# コメントにした項目は、使うときにコメントを外して書き換える。

# HTML に出すプロジェクト名。省略するとルートのディレクトリ名
# title: my-project

# 管理対象の Markdown（Free ファイル）の glob。各ファイルの YAML front matter に mission を書く
docs:
  - docs/**/*.md

# 管理対象にも管理外にもしないファイルの glob
ignore:
  - docs/tmp/**

# タグの一覧。書いたときだけ、YAML front matter の tags をこの中に限る
# tags: [仕様, 手順, 規約]

slots:
  # Slot ファイルを置くディレクトリ
  dir: docv

output:
  # docv build の出力先
  dir: docv/build
  # 出力を git で管理するか。false なら出力先に .gitignore を置く
  commit: true
  # docv build で、Slot のページを JPG にするか。true なら Chrome が要る
  image: false

# lint:
#   # タグごとに、そのタグを持つファイルにかけるルール（no-past-wording、no-done-task）
#   tags:
#     仕様: [no-past-wording]
#   # 過去の言い回しとして警告する語の足し引き
#   words:
#     add: []
#     remove: []
#   # 書式の規則。書いた規則だけを確かめる
#   style:
#     bold: deny
#     italic: deny
#     html: [details, summary]
#     dollar: deny
#     heading_number: require
#     section_separator: require
#     link_text: {{ base: docs }}
#     image_alt: require
#   # textlint。使うには Node.js が要る
#   textlint:
#     enabled: true
#     level: warn
#     presets: [ja-technical-writing]
#     rules: {{}}
#     prh: []

format:
  # check で、整形が要るファイルを error にするか
  enabled: false

# パスごとに、道具の設定を上書きする。後に書いたものが優先する
# overrides:
#   - paths: [docs/tmp/**]
#     textlint: false
#     style: false
#     format: false
#     mission: optional
",
        schema_url()
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
        assert_eq!(config.docs, ["docs/**/*.md"]);
        assert_eq!(config.slots.dir, "docv");
        assert!(!config.output.image);
        let error = run(&path, false).unwrap_err().to_string();
        assert!(error.contains("すでにある"), "{error}");
        fs::write(&path, "docs: []\n").unwrap();
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
        assert!(text.contains("  style:\n    bold: deny"), "{text}");
        config::parse(&text).unwrap_or_else(|e| panic!("{e:#}\n{text}"));
    }
}
