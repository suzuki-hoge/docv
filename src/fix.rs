//! docv fix。textlint の自動修正と整形で、管理対象のファイルと Slot のディレクトリの中の .md のファイルを書き換える。
//! docv.yaml と YAML の Slot は、末尾の改行だけを整える。docv/service/manual.md の docv fix。

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

/// targets が空なら対象をすべて、そうでなければそのパス（ルートからの相対パス）だけ。
/// textlint が false なら、textlint の自動修正を飛ばす。
pub fn run(project: &Project, targets: &[String], textlint: bool) -> Result<Report> {
    let files = project.fix_targets(targets)?;
    let mut original = BTreeMap::new();
    for path in &files {
        let text = fs::read_to_string(project.root.join(path))
            .with_context(|| format!("{path} を読めない"))?;
        original.insert(path.clone(), text);
    }

    // textlint の自動修正で文を直してから整形する。
    if textlint && let Some(textlint) = Textlint::new(project) {
        textlint.fix(&files)?;
    }

    let mut report = Report::default();
    for (path, before) in original {
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

    for path in project.yaml_targets(targets)? {
        let full = project.root.join(&path);
        let text = fs::read_to_string(&full).with_context(|| format!("{path} を読めない"))?;
        let formatted = format::final_newline(&text);
        if formatted != text {
            fs::write(&full, &formatted).with_context(|| format!("{path} を書けない"))?;
            report.changed.push(path);
        }
    }
    report.changed.sort();
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
        write(root, "docv.yaml", "ignore: [docs/tmp/**]\n\n");
        write(root, "docs/spec/a.md", "---\nmission: q\n---\n# A\n* x\n");
        write(root, "docs/spec/b.md", "# B\n");
        write(root, "docs/spec/c.md", "---\nmission: q\n");
        write(root, "docs/tmp/e.md", "* x\n");
        write(root, "notes.md", "* x\n");
        write(
            root,
            "docv/service/manual.md",
            "# M\n| a | bb |\n|---|---|\n| 種類 | c |\n",
        );
        write(root, "docv/service/words.yaml", "groups: []");
        write(root, "docv/service/install.yaml", "steps: []\n");
        let project = Project::open(&Locate::default(), root).unwrap();

        let report = run(&project, &[], true).unwrap();
        assert_eq!(
            report.changed,
            [
                "docs/spec/a.md",
                "docv.yaml",
                "docv/service/manual.md",
                "docv/service/words.yaml"
            ]
        );
        assert_eq!(
            fs::read_to_string(root.join("docv/service/words.yaml")).unwrap(),
            "groups: []\n"
        );
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "docs/spec/c.md");
        assert_eq!(
            fs::read_to_string(root.join("docs/spec/a.md")).unwrap(),
            "---\nmission: q\n---\n\n# A\n\n- x\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("docv/service/manual.md")).unwrap(),
            "# M\n\n| a    | bb |\n| ---- | -- |\n| 種類 | c  |\n"
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
