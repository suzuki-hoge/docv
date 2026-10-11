//! docv new。種類のテンプレからファイルを作る。docs/spec/cli.md の 4 章。

use std::fs;

use anyhow::{Context, Result, bail};

use crate::format;
use crate::markdown;
use crate::workspace::{Class, Project};

/// 組み込みの種類のテンプレ。templates/ をバイナリに埋め込む。
const TEMPLATES: &[(&str, &str)] = &[
    ("overview", include_str!("../templates/overview.md")),
    ("spec", include_str!("../templates/spec.md")),
    ("operation", include_str!("../templates/operation.md")),
    ("cost", include_str!("../templates/cost.md")),
    ("rule", include_str!("../templates/rule.md")),
    ("plan", include_str!("../templates/plan.md")),
    ("proposal", include_str!("../templates/proposal.md")),
    ("fixme", include_str!("../templates/fixme.md")),
    ("tmp", include_str!("../templates/tmp.md")),
];

/// docv.yaml の templates がない、独自の種類のテンプレ。
const CUSTOM_TEMPLATE: &str = "---\nquestion: 〇〇は何か\n---\n\n# 〇〇\n";

/// rel はルートからの相対パス。
pub fn run(project: &Project, rel: &str) -> Result<()> {
    if !rel.ends_with(".md") {
        bail!("{rel} は .md のファイルではない。docs の対象は .md のファイルだけ");
    }
    let index = match project.classify(rel) {
        Some(Class::Managed(index)) => index,
        Some(Class::Unmanaged) => bail!(
            "{rel} は docv.yaml の docs のどの項目にも当てはまらない。管理外のファイルは docv を使わずに作る"
        ),
        None => bail!("{rel} は docv.yaml の ignore か output.dir で除外されている"),
    };
    let kind = project.kind_of(index);
    if kind.is_archive() {
        bail!("{rel} は archive。archive は docv new では作らない。docv retire で移す");
    }
    let path = project.root.join(rel);
    if path.exists() {
        bail!("{rel} はすでにある");
    }

    let template = match project.config.templates.get(&kind.id) {
        Some(template) => fs::read_to_string(project.root.join(template))
            .with_context(|| format!("テンプレ {template} を読めない"))?,
        None => TEMPLATES
            .iter()
            .find(|(id, _)| *id == kind.id)
            .map_or(CUSTOM_TEMPLATE, |(_, text)| text)
            .to_string(),
    };
    let mut text = if project.entry(index).question.is_some() {
        without_question(&template)
    } else {
        template
    };
    if project.config.format.enabled {
        text = format::format(&text).context("テンプレを整形できない")?;
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("{} を作れない", parent.display()))?;
    }
    fs::write(&path, text).with_context(|| format!("{rel} を書けない"))
}

/// docv.yaml で question を与えるファイルのために、テンプレの frontmatter から question を外す。
/// frontmatter が空になれば、frontmatter ごと外す。
fn without_question(template: &str) -> String {
    let document = markdown::split(template);
    let Some(fm) = document.frontmatter.filter(|fm| fm.closed) else {
        return template.to_string();
    };
    let rest: String = fm
        .yaml
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("question:"))
        .collect();
    let body = document.body.trim_start_matches('\n');
    if rest.trim().is_empty() {
        body.to_string()
    } else {
        format!("---\n{rest}---\n\n{body}")
    }
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

    fn project(extra: &str) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "docv.yaml",
            &format!(
                "version: 1\nkinds:\n  - id: research\n    time: future\ndocs:\n  - path: README.md\n    kind: overview\n    question: q\n  - path: docs/spec/*.md\n    kind: spec\n  - path: docs/plan/*.md\n    kind: plan\n  - path: docs/research/*.md\n    kind: research\n  - path: docs/archive/**/*.md\n    kind: archive\n{extra}"
            ),
        );
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn creates_file_from_template() {
        let (dir, project) = project("");
        run(&project, "docs/plan/001-x.md").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("docs/plan/001-x.md")).unwrap(),
            include_str!("../templates/plan.md")
        );
        run(&project, "docs/research/a.md").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("docs/research/a.md")).unwrap(),
            CUSTOM_TEMPLATE
        );
        run(&project, "README.md").unwrap();
        let readme = fs::read_to_string(dir.path().join("README.md")).unwrap();
        assert!(readme.starts_with("# 〇〇\n"), "{readme}");
    }

    #[test]
    fn rejects_invalid_paths() {
        let (dir, project) = project("");
        write(dir.path(), "docs/spec/a.md", "# a\n");
        for (path, message) in [
            ("docs/spec/a.md", "すでにある"),
            ("docs/memo.md", "どの項目にも当てはまらない"),
            ("docs/archive/a.md", "docv retire で移す"),
            ("docs/spec/a.txt", ".md のファイルではない"),
        ] {
            let error = run(&project, path).unwrap_err().to_string();
            assert!(error.contains(message), "{path}: {error}");
        }
    }

    #[test]
    fn uses_project_template_and_formats() {
        let (dir, project) = project("templates:\n  spec: t/spec.md\nformat:\n  enabled: true\n");
        write(
            dir.path(),
            "t/spec.md",
            "---\nquestion: q\n---\n# 独自\n* x\n",
        );
        run(&project, "docs/spec/b.md").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("docs/spec/b.md")).unwrap(),
            "---\nquestion: q\n---\n\n# 独自\n\n- x\n"
        );
    }

    #[test]
    fn removes_question_from_frontmatter() {
        assert_eq!(
            without_question("---\nquestion: q\nstatus: todo\n---\n\n# T\n"),
            "---\nstatus: todo\n---\n\n# T\n"
        );
        assert_eq!(without_question("---\nquestion: q\n---\n\n# T\n"), "# T\n");
    }
}
