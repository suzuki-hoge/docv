//! docv init。推奨する配置の docv.yaml を作る。docs/spec/cli.md の 4 章。

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

/// 推奨する配置（docs/spec/config.md の 5 章）の docv.yaml。name はプロジェクト名で、README.md の問いに使う。
pub fn config_text(name: &str) -> String {
    let mut text = format!(
        "# yaml-language-server: $schema={}\nversion: 1\n\ndocs:\n  - path: README.md\n    kind: overview\n    question: {name} は何で、どこから見ればいいか\n",
        schema_url()
    );
    for kind in [
        "spec",
        "operation",
        "cost",
        "rule",
        "plan",
        "proposal",
        "fixme",
    ] {
        text.push_str(&format!("  - path: docs/{kind}/*.md\n    kind: {kind}\n"));
    }
    text.push_str(
        "  - path: docs/archive/**/*.md\n    kind: archive\n  - path: docs/tmp/**/*.md\n    kind: tmp\n\narchive:\n  dir: docs/archive\n",
    );
    text
}

/// path は作る docv.yaml のパス。
pub fn run(path: &Path) -> Result<()> {
    if path.exists() {
        bail!("{} はすでにある", path.display());
    }
    let name = path
        .parent()
        .and_then(|dir| dir.canonicalize().ok())
        .and_then(|dir| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "このプロジェクト".to_string());
    fs::write(path, config_text(&name)).with_context(|| format!("{} を書けない", path.display()))
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
        assert_eq!(config.docs.len(), 10);
        assert_eq!(config.docs[9].kind, "tmp");
        let name = dir.path().canonicalize().unwrap();
        let name = name.file_name().unwrap().to_string_lossy();
        assert_eq!(
            config.docs[0].question.as_deref(),
            Some(format!("{name} は何で、どこから見ればいいか").as_str())
        );
        assert!(run(&path).unwrap_err().to_string().contains("すでにある"));
    }
}
