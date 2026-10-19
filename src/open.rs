//! docv open。docv build が作った index.html（プロジェクト概要）を、OS の既定のブラウザで開く。
//! docv/service/manual.md の「docv open」。

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::build::PAGE;

/// out（dir.html）の index.html を、program（opener の返すコマンド）で開く。ファイルは書かない。
pub fn run(program: &str, out: &Path) -> Result<()> {
    let page = page(out)?;
    let status = Command::new(program)
        .arg(&page)
        .status()
        .with_context(|| format!("{program} を実行できない"))?;
    // explorer は、開けたときも 0 以外で終わるので、終了コードを見ない
    if !status.success() && !cfg!(windows) {
        bail!("{program} が {} を開けなかった", page.display());
    }
    Ok(())
}

/// 開くページのパス。まだ build していなければ error。
fn page(out: &Path) -> Result<PathBuf> {
    let page = out.join(PAGE);
    if !page.is_file() {
        bail!("{} がない。先に docv build で HTML を作る", page.display());
    }
    Ok(page)
}

/// OS ごとの、ファイルを既定のアプリで開くコマンド。macOS と Windows のほかは対応しないので None。
pub fn opener() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("open")
    } else if cfg!(windows) {
        Some("explorer")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn page_needs_build() {
        let dir = tempfile::tempdir().unwrap();
        let error = page(dir.path()).unwrap_err().to_string();
        assert!(error.contains("docv build"), "{error}");
        fs::write(dir.path().join(PAGE), "").unwrap();
        assert_eq!(page(dir.path()).unwrap(), dir.path().join(PAGE));
    }
}
