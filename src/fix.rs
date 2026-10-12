//! docv fix。textlint の自動修正と整形で、管理対象のファイルを書き換える。docv/service/manual.yaml の docv fix。

use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};

use crate::format;
use crate::textlint::Textlint;
use crate::workspace::Project;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// 書き換えたファイル。
    pub changed: Vec<String>,
    /// 整形できなかったファイルと、その理由。
    pub failed: Vec<(String, String)>,
}

/// targets が空なら check と同じ対象をすべて、そうでなければそのパス（ルートからの相対パス）だけ。
/// textlint が false なら、textlint の自動修正を飛ばす。
pub fn run(project: &Project, targets: &[String], textlint: bool) -> Result<Report> {
    let files = project.targets(targets)?;
    let mut original = BTreeMap::new();
    for path in &files {
        let text = fs::read_to_string(project.root.join(path))
            .with_context(|| format!("{path} を読めない"))?;
        original.insert(path.clone(), text);
    }

    // textlint の自動修正で文を直してから整形する。overrides で textlint や整形を外したパスは飛ばす。
    if textlint && let Some(textlint) = Textlint::new(project) {
        let paths: Vec<String> = files
            .iter()
            .filter(|path| project.settings_of(path).textlint)
            .cloned()
            .collect();
        textlint.fix(&paths)?;
    }

    let mut report = Report::default();
    for (path, before) in original {
        if !project.settings_of(&path).format {
            continue;
        }
        let full = project.root.join(&path);
        let text = fs::read_to_string(&full).with_context(|| format!("{path} を読めない"))?;
        match format::format(&text) {
            Ok(formatted) => {
                if formatted != text {
                    fs::write(&full, &formatted).with_context(|| format!("{path} を書けない"))?;
                }
                if formatted != before {
                    report.changed.push(path);
                }
            }
            Err(e) => report.failed.push((path, format!("{e:#}"))),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn formats_targets_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "docv.yaml",
            "version: 1\ndocs: [docs/spec/*.md]\nignore: [docs/tmp/**]\n",
        );
        write(root, "docs/spec/a.md", "---\nquestion: q\n---\n# A\n* x\n");
        write(root, "docs/spec/b.md", "# B\n");
        write(root, "docs/spec/c.md", "---\nquestion: q\n");
        write(root, "docs/tmp/e.md", "* x\n");
        write(root, "notes.md", "* x\n");
        let project = Project::open(&Locate::default(), root).unwrap();

        let report = run(&project, &[], true).unwrap();
        assert_eq!(report.changed, ["docs/spec/a.md"]);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "docs/spec/c.md");
        assert_eq!(
            fs::read_to_string(root.join("docs/spec/a.md")).unwrap(),
            "---\nquestion: q\n---\n\n# A\n\n- x\n"
        );
        for path in ["docs/tmp/e.md", "notes.md"] {
            assert_eq!(fs::read_to_string(root.join(path)).unwrap(), "* x\n");
        }
        assert_eq!(
            run(&project, &[], true).unwrap().changed,
            Vec::<String>::new()
        );
    }
}
