//! docv fix。textlint の自動修正と整形で、管理対象のファイルと Slot のディレクトリの中の .md のファイルを書き換える。
//! YAML の Slot は、Markdown の項目（`_markdown` で終わる項目）の中身を直す。docv.yaml と YAML の Slot の末尾の改行も整える。
//! docv/service/manual.md の docv fix。

use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};

use crate::textlint::Textlint;
use crate::workspace::Project;
use crate::{format, slot, slot_markdown};

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

    // YAML の Slot の Markdown の項目。シンボリックリンクは書き換えない（yaml_targets が除く）。
    let yaml_files = project.yaml_targets(targets)?;
    let mut slots = Vec::new();
    for path in slot::yaml_paths(project)? {
        if !yaml_files.contains(&path) {
            continue;
        }
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        if let Some(scan) = slot_markdown::scan(&text) {
            let virtual_text = slot_markdown::virtual_markdown(&text, &scan.blocks);
            slots.push((path, text, scan.blocks, virtual_text));
        }
    }
    let virtual_files: Vec<(String, String)> = slots
        .iter()
        .map(|(path, _, _, v)| (path.clone(), v.clone()))
        .collect();

    // textlint の自動修正で文を直してから整形する。
    let mut fixed_virtual = Vec::new();
    if textlint && let Some(textlint) = Textlint::new(project) {
        fixed_virtual = textlint.fix(&files, &virtual_files)?;
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

    for (i, (path, text, blocks, virtual_text)) in slots.iter().enumerate() {
        let texts = match fixed_virtual.get(i) {
            Some(fixed) => match slot_markdown::from_virtual(virtual_text, fixed, blocks) {
                Some(texts) => texts,
                None => {
                    report.failed.push((
                        path.clone(),
                        "textlint の自動修正で行の数が変わった".to_string(),
                    ));
                    continue;
                }
            },
            None => blocks.iter().map(|b| b.text.clone()).collect(),
        };
        let mut replaced = Vec::new();
        for (block, new) in blocks.iter().zip(texts) {
            match format::format(&new) {
                Ok(formatted) if formatted != block.text => replaced.push((block, formatted)),
                Ok(_) => {}
                Err(e) => report.failed.push((path.clone(), format!("{e:#}"))),
            }
        }
        if !replaced.is_empty() {
            let full = project.root.join(path);
            fs::write(&full, slot_markdown::replace(text, &replaced))
                .with_context(|| format!("{path} を書けない"))?;
            report.changed.push(path.clone());
        }
    }

    for path in yaml_files {
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

    #[test]
    fn formats_markdown_in_yaml_slots() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "docv.yaml", "title: t\n");
        write(
            root,
            "docv/service/words.yaml",
            "groups:\n  - name: g\n    items:\n      - ja: a\n        desc_markdown: |\n          * x\n          * y\n      - ja: b\n        desc_markdown: 1 行\n",
        );
        let project = Project::open(&Locate::default(), root).unwrap();
        let report = run(&project, &[], true).unwrap();
        assert_eq!(report.changed, ["docv/service/words.yaml"]);
        // | のブロックの中身だけを整形し、字下げを付け直す。| でない項目は check が指摘するので、触らない
        assert_eq!(
            fs::read_to_string(root.join("docv/service/words.yaml")).unwrap(),
            "groups:\n  - name: g\n    items:\n      - ja: a\n        desc_markdown: |\n          - x\n          - y\n      - ja: b\n        desc_markdown: 1 行\n"
        );
    }
}
