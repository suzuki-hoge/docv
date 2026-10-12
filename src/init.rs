//! docv init。推奨する配置の docv.yaml を作る。docv/service/manual.yaml の docv init。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// 実行した CLI の版のスキーマの URL。
pub fn schema_url() -> String {
    format!(
        "https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/schema/docv.schema.json",
        env!("CARGO_PKG_VERSION")
    )
}

/// 推奨する配置（docs/spec/config.md の 5 章）の docv.yaml。
pub fn config_text() -> String {
    format!(
        "# yaml-language-server: $schema={}
version: 1

# 管理対象の Markdown。各ファイルの frontmatter に mission、version、tags を書く。
docs:
  - docs/**/*.md
ignore:
  - docs/tmp/**

# docv の規格のファイル（枠。概要、コマンドなど）を置くディレクトリ。docs/ はプロジェクトが自由に書く。
slots:
  dir: docv
",
        schema_url()
    )
}

/// path は作る docv.yaml のパス。
pub fn run(path: &Path) -> Result<()> {
    if path.exists() {
        bail!("{} はすでにある", path.display());
    }
    fs::write(path, config_text()).with_context(|| format!("{} を書けない", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn writes_valid_config_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("docv.yaml");
        run(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with(&format!(
            "# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/",
            env!("CARGO_PKG_VERSION")
        )));
        let config = config::parse(&text).unwrap();
        assert_eq!(config.docs, ["docs/**/*.md"]);
        assert_eq!(config.slots.dir, "docv");
        assert!(run(&path).unwrap_err().to_string().contains("すでにある"));
    }
}
