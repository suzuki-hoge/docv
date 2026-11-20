//! docv fix。textlint の自動修正と整形で、管理対象のファイルと、Slot、ADR、PDR を書き換える。docv.yaml は、末尾の改行だけを整える。
//! docv/slot/service/manual.md の docv fix。

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

    // docv.yaml は、中身を変えず、末尾の改行だけを整える。
    if let Some(path) = project.config_target(targets) {
        let full = project.root.join(&path);
        let text = fs::read_to_string(&full).with_context(|| format!("{path} を読めない"))?;
        let formatted = format::final_newline(&text);
        if formatted != text {
            fs::write(&full, &formatted).with_context(|| format!("{path} を書けない"))?;
            report.changed.push(path);
        }
    }
    report.changed.sort();
    report.changed.dedup();
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
        write(root, "docv.yaml", "docv_exclude: [docv/free/tmp/**]\n\n");
        write(
            root,
            "docv/free/spec/a.md",
            "---\nmission: q\n---\n# A\n* x\n",
        );
        write(root, "docv/free/spec/b.md", "# B\n");
        write(root, "docv/free/spec/c.md", "---\nmission: q\n");
        write(root, "docv/free/tmp/e.md", "* x\n");
        write(root, "notes.md", "* x\n");
        write(
            root,
            "docv/slot/service/manual.md",
            "# M\n| a | bb |\n|---|---|\n| 種類 | c |\n",
        );
        // 残った YAML の Slot は、Slot ではないので書き換えない（check が slot-unknown にする）
        write(root, "docv/slot/service/words.yaml", "groups: []");
        let project = Project::open(&Locate::default(), root).unwrap();

        let report = run(&project, &[], true).unwrap();
        assert_eq!(
            report.changed,
            [
                "docv.yaml",
                "docv/free/spec/a.md",
                "docv/slot/service/manual.md"
            ]
        );
        // docv.yaml は、末尾の改行だけを整える
        assert_eq!(
            fs::read_to_string(root.join("docv.yaml")).unwrap(),
            "docv_exclude: [docv/free/tmp/**]\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("docv/slot/service/words.yaml")).unwrap(),
            "groups: []"
        );
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, "docv/free/spec/c.md");
        assert_eq!(
            fs::read_to_string(root.join("docv/free/spec/a.md")).unwrap(),
            "---\nmission: q\n---\n\n# A\n\n- x\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("docv/slot/service/manual.md")).unwrap(),
            "# M\n\n| a    | bb |\n| ---- | -- |\n| 種類 | c  |\n"
        );
        for path in ["docv/free/tmp/e.md", "notes.md"] {
            assert_eq!(fs::read_to_string(root.join(path)).unwrap(), "* x\n");
        }
        assert_eq!(
            run(&project, &[], true).unwrap().changed,
            Vec::<String>::new()
        );
    }
}
