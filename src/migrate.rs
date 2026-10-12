//! docv migrate-guide。後から docv を入れるときのガイドを出す。docv/service/manual.yaml の docv migrate-guide。

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

use anyhow::Result;

use crate::check::{self, Options};
use crate::diagnostic::Diagnostic;
use crate::markdown;
use crate::slot;
use crate::version::{self, Version};
use crate::workspace::{Class, Project};

/// ディレクトリ名に含まれる語と、推測するタグ。上から順に見る。
const GUESSES: &[(&[&str], &str)] = &[
    (&["specification", "spec", "architecture", "design"], "仕様"),
    (
        &["operation", "command", "manual", "setup", "runbook"],
        "手順",
    ),
    (&["pricing", "price", "cost"], "費用"),
    (&["guideline", "coding", "convention", "rule"], "規約"),
];

/// README の見出しに含まれる語と、埋め込める枠。
const SLOT_HEADINGS: &[(&[&str], &str)] = &[
    (&["install", "インストール", "入れ方"], "install"),
    (
        &["setup", "development", "contributing", "環境構築", "開発"],
        "setup",
    ),
    (&["usage", "使い方", "コマンド"], "manual"),
    (&["environment", "env", "環境変数"], "env"),
];

pub fn render(project: &Project, options: Options) -> Result<String> {
    let files = project.files()?;
    let unmanaged: Vec<&str> = files
        .iter()
        .filter(|f| f.class == Class::Unmanaged)
        .map(|f| f.path.as_str())
        .collect();
    let diagnostics = if project.configured {
        check::run(project, &[], options)?
    } else {
        Vec::new()
    };

    let mut out = String::new();
    out.push_str("## 1. 設定\n");
    if project.configured {
        let _ = writeln!(out, "docv.yaml: {}", project.config_path.display());
    } else {
        out.push_str("docv.yaml: なし。docv init で作る\n");
    }

    let _ = writeln!(out, "\n## 2. 管理外のファイル（{} 件）", unmanaged.len());
    let mut dirs: BTreeMap<String, usize> = BTreeMap::new();
    for path in &unmanaged {
        let dir = path
            .rsplit_once('/')
            .map_or("./".to_string(), |(dir, _)| format!("{dir}/"));
        *dirs.entry(dir).or_default() += 1;
    }
    let width = dirs.keys().map(|d| d.chars().count()).max().unwrap_or(0);
    for (dir, count) in &dirs {
        let pad = width - dir.chars().count();
        let _ = writeln!(out, "{dir}{}  {count} 件", " ".repeat(pad));
    }

    out.push_str("\n## 3. タグの案\n");
    let tags = guess_tags(&unmanaged);
    if tags.is_empty() {
        out.push_str("（なし）\n");
    }
    for (dir, tag) in &tags {
        let _ = writeln!(out, "- {dir}/ のファイル: tags: [{tag}]");
    }

    out.push_str("\n## 4. 枠\n");
    let present: Vec<&str> = slot::load(project)?.iter().map(|s| s.def.id).collect();
    let readme = readme_headings(project);
    for def in slot::DEFS {
        if present.contains(&def.id) {
            let _ = writeln!(out, "- [x] {}（{}）", def.label, def.file_name());
            continue;
        }
        let _ = write!(
            out,
            "- [ ] {}（{}）: {}",
            def.label,
            def.file_name(),
            def.question
        );
        if let Some((_, heading)) = readme.iter().find(|(id, _)| *id == def.id) {
            let _ = write!(
                out,
                "。include: [README.md#{heading}] で README の章を埋め込める"
            );
        }
        out.push('\n');
    }

    out.push_str("\n## 5. ルールに沿っていないファイル\n");
    if !project.configured {
        out.push_str("（docv.yaml がないため、なし）\n");
    } else if diagnostics.is_empty() {
        out.push_str("（なし）\n");
    } else {
        render_diagnostics(&mut out, &diagnostics);
    }

    out.push_str("\n## 6. frontmatter の版\n");
    let versions = file_versions(project)?;
    let current = version::current();
    let older = render_versions(&mut out, &versions, current);

    out.push_str("\n## 7. 次にやること\n");
    for (i, step) in next_steps(project, &present, &diagnostics, older)
        .iter()
        .enumerate()
    {
        let _ = writeln!(out, "{}. {step}", i + 1);
    }
    Ok(out)
}

/// 管理対象のファイルの frontmatter の version。書いていないファイルは None。
fn file_versions(project: &Project) -> Result<Vec<(String, Option<Version>)>> {
    let mut versions = Vec::new();
    for path in project.managed()? {
        let text = fs::read_to_string(project.root.join(&path)).unwrap_or_default();
        let version = markdown::split(&text)
            .frontmatter
            .and_then(|fm| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&fm.yaml).ok())
            .and_then(|v| v.get("version").and_then(Version::parse));
        versions.push((path, version));
    }
    Ok(versions)
}

/// 版ごとのファイルの数と、いちばん古い版からこの CLI の版までの手順を、1 版ずつ出す。
/// CLI は「1 つ前の版から」の手順だけを持ち、古いファイルには手順をつないで示す。古い版のファイルがあれば true。
fn render_versions(
    out: &mut String,
    versions: &[(String, Option<Version>)],
    current: Version,
) -> bool {
    let _ = writeln!(out, "この docv の版: {current}");
    let mut counts: BTreeMap<Option<Version>, usize> = BTreeMap::new();
    for (_, v) in versions {
        *counts.entry(*v).or_default() += 1;
    }
    for (v, count) in &counts {
        let label = v.map_or("version なし".to_string(), |v| v.to_string());
        let _ = writeln!(out, "- {label}: {count} ファイル");
    }
    let Some(oldest) = counts
        .keys()
        .flatten()
        .copied()
        .filter(|v| *v < current)
        .min()
    else {
        out.push_str("（古い版のファイルはない）\n");
        return false;
    };
    let steps = version::path_from(oldest);
    if steps.is_empty() {
        let _ = writeln!(
            out,
            "\n{oldest} → {current}: 直すことはない。version を {current} に上げる"
        );
    }
    for m in steps {
        let _ = writeln!(out, "\n{} → {}", m.from, m.to);
        for step in m.steps {
            let _ = writeln!(out, "- {step}");
        }
    }
    true
}

fn render_diagnostics(out: &mut String, diagnostics: &[Diagnostic]) {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for d in diagnostics {
        *counts.entry(&d.rule).or_default() += 1;
    }
    let mut counts: Vec<(&str, usize)> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let width = counts.iter().map(|(r, _)| r.len()).max().unwrap_or(0);
    for (rule, count) in counts {
        let _ = writeln!(out, "{rule:<width$}  {count} 件");
    }
    let mut path = "";
    for d in diagnostics {
        if d.path != path {
            path = &d.path;
            let _ = writeln!(out, "\n{path}");
        }
        let _ = writeln!(out, "  {}: {} {} {}", d.line, d.level, d.rule, d.message);
    }
}

fn next_steps(
    project: &Project,
    present: &[&str],
    diagnostics: &[Diagnostic],
    older: bool,
) -> Vec<String> {
    let mut steps = Vec::new();
    if !project.configured {
        steps.push(
            "docv init で docv.yaml を作り、docs に管理するファイルの glob を書く".to_string(),
        );
    }
    if present.len() < slot::DEFS.len() {
        steps.push(
            "4 の枠のうち、このプロジェクトに当てはまるものを docv add <slot> で作る（docv add --list で一覧を出す）"
                .to_string(),
        );
    }
    if older {
        steps.push(format!(
            "6 の手順を古い版から順にあて、frontmatter の version を {} にする",
            version::current()
        ));
    }
    if diagnostics.iter().any(|d| d.rule == "format-diff") {
        steps.push(
            "ほかを直す前に docv fix で整形し、整形の差分だけでコミットする。整形の差分と内容の差分を混ぜないため"
                .to_string(),
        );
    }
    if diagnostics.iter().any(|d| d.rule != "format-diff") {
        steps.push(
            "5 に出たファイルを直す。この出力と docv desc を AI Agent に渡して直させてもよい"
                .to_string(),
        );
    }
    if steps.is_empty() {
        steps.push("なし。ドキュメントを変えたら docv check で確かめる".to_string());
    }
    steps
}

/// 管理外のファイルのディレクトリ名から、タグを推測する。
fn guess_tags(unmanaged: &[&str]) -> BTreeMap<String, &'static str> {
    let mut tags = BTreeMap::new();
    for path in unmanaged {
        let parts: Vec<&str> = path.split('/').collect();
        let dir_parts = &parts[..parts.len() - 1];
        for (i, name) in dir_parts.iter().enumerate() {
            let name = name.to_lowercase();
            if let Some((_, tag)) = GUESSES
                .iter()
                .find(|(words, _)| words.iter().any(|w| name.contains(w)))
            {
                tags.insert(dir_parts[..=i].join("/"), *tag);
                break;
            }
        }
    }
    tags
}

/// README の h2 のうち、枠に埋め込めそうなもの。(枠の ID, 見出し)
fn readme_headings(project: &Project) -> Vec<(&'static str, String)> {
    let Ok(text) = fs::read_to_string(project.root.join("README.md")) else {
        return Vec::new();
    };
    let outline = markdown::outline(&markdown::split(&text));
    let mut found = Vec::new();
    for heading in outline.headings.iter().filter(|h| h.level == 2) {
        let lower = heading.text.to_lowercase();
        if let Some((_, id)) = SLOT_HEADINGS
            .iter()
            .find(|(words, _)| words.iter().any(|w| lower.contains(w)))
            && !found.iter().any(|(f, _)| f == id)
        {
            found.push((*id, slot::strip_number(&heading.text).to_string()));
        }
    }
    found
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

    const OPTIONS: Options = Options { textlint: false };

    #[test]
    fn guesses_tags_from_directories() {
        let tags = guess_tags(&[
            "README.md",
            "docs/Design/a.md",
            "docs/design/sub/b.md",
            "docs/manual/c.md",
            "docs/misc/f.md",
        ]);
        let tags: Vec<(&str, &str)> = tags.iter().map(|(d, t)| (d.as_str(), *t)).collect();
        assert_eq!(
            tags,
            [
                ("docs/Design", "仕様"),
                ("docs/design", "仕様"),
                ("docs/manual", "手順"),
            ]
        );
    }

    #[test]
    fn guides_without_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "README.md", "# R\n\n## Installation\n\n手順\n");
        write(root, "docs/design/a.md", "# A\n");
        write(root, "docs/plan/c.md", "# C\n");
        let project = Project::unconfigured(root).unwrap();
        let out = render(&project, OPTIONS).unwrap();
        assert!(
            out.starts_with(
                "## 1. 設定
docv.yaml: なし。docv init で作る

## 2. 管理外のファイル（3 件）
./            1 件
docs/design/  1 件
docs/plan/    1 件

## 3. タグの案
- docs/design/ のファイル: tags: [仕様]

## 4. 枠
- [ ] インストール（service/install.yaml）"
            ),
            "{out}"
        );
        assert!(
            out.contains("- [ ] インストール（service/install.yaml）: 利用者はどう入れるか。include: [README.md#Installation] で README の章を埋め込める\n"),
            "{out}"
        );
        assert!(out.contains("## 7. 次にやること\n1. docv init で"), "{out}");
    }

    #[test]
    fn guides_with_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "docv.yaml",
            "version: 1\ndocs: [docs/spec/*.md]\nformat:\n  enabled: true\n",
        );
        write(root, "docs/spec/a.md", "# A\n* x\n");
        write(root, "docv/service/service.md", "# サービス仕様\n");
        let project = Project::open(&Locate::default(), root).unwrap();
        let out = render(&project, OPTIONS).unwrap();
        assert!(
            out.contains("- [x] サービス仕様（service/service.md）\n"),
            "{out}"
        );
        assert!(
            out.contains("## 5. ルールに沿っていないファイル\nformat-diff      1 件\nmission-missing  1 件\nversion-missing  1 件\n\ndocs/spec/a.md\n  1: error mission-missing"),
            "{out}"
        );
        assert!(
            out.contains("\n2. ほかを直す前に docv fix で整形し"),
            "{out}"
        );
    }

    #[test]
    fn guides_from_old_frontmatter_versions() {
        let current = version::current();
        let mut out = String::new();
        let old = Version {
            major: current.major,
            minor: current.minor.saturating_sub(1),
        };
        let versions = vec![
            ("docs/a.md".to_string(), Some(current)),
            ("docs/b.md".to_string(), None),
        ];
        assert!(!render_versions(&mut out, &versions, current));
        assert!(out.contains("（古い版のファイルはない）"), "{out}");
        if old < current {
            let mut out = String::new();
            let versions = vec![("docs/a.md".to_string(), Some(old))];
            assert!(render_versions(&mut out, &versions, current));
            assert!(out.contains(&format!("{old} → {current}")), "{out}");
        }
    }
}
