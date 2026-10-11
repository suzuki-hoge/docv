//! docv migrate-guide。後から docv を入れるときのガイドを出す。docs/spec/migrate-guide.md。

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::Result;

use crate::check::{self, Options};
use crate::diagnostic::Diagnostic;
use crate::workspace::{Class, Project};

/// ディレクトリ名に含まれる語と、推測する種類。上から順に見る。
const GUESSES: &[(&[&str], &str)] = &[
    (&["specification", "spec", "architecture", "design"], "spec"),
    (
        &["operation", "command", "manual", "setup", "runbook"],
        "operation",
    ),
    (&["pricing", "price", "cost"], "cost"),
    (&["guideline", "coding", "convention", "rule"], "rule"),
    (&["plan", "task", "todo"], "plan"),
    (&["proposal", "rfc", "draft"], "proposal"),
    (&["fixme", "question"], "fixme"),
    (&["archive", "history", "deprecated"], "archive"),
];

/// docv.yaml に足す案。
#[derive(Debug, PartialEq, Eq)]
enum Suggestion {
    Doc { path: String, kind: &'static str },
    Ignore(String),
}

pub fn render(project: &Project, options: Options) -> Result<String> {
    let files = project.files()?;
    let unmanaged: Vec<&str> = files
        .iter()
        .filter(|f| f.class == Class::Unmanaged)
        .map(|f| f.path.as_str())
        .collect();
    let suggestions = suggest(&unmanaged);
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
        let dir = path.rsplit_once('/').map_or("./", |(dir, _)| dir);
        let dir = if dir == "./" {
            dir.to_string()
        } else {
            format!("{dir}/")
        };
        *dirs.entry(dir).or_default() += 1;
    }
    let width = dirs.keys().map(|d| d.chars().count()).max().unwrap_or(0);
    for (dir, count) in &dirs {
        let pad = width - dir.chars().count();
        let _ = writeln!(out, "{dir}{}  {count} 件", " ".repeat(pad));
    }
    if !unmanaged.is_empty() {
        out.push('\n');
    }
    for path in &unmanaged {
        let _ = writeln!(out, "- {path}");
    }

    out.push_str("\n## 3. docs の案\n");
    let docs: Vec<&Suggestion> = suggestions
        .iter()
        .filter(|s| matches!(s, Suggestion::Doc { .. }))
        .collect();
    let ignore: Vec<&Suggestion> = suggestions
        .iter()
        .filter(|s| matches!(s, Suggestion::Ignore(_)))
        .collect();
    if suggestions.is_empty() {
        out.push_str("（なし）\n");
    }
    if !docs.is_empty() {
        out.push_str("docs:\n");
        for s in docs {
            if let Suggestion::Doc { path, kind } = s {
                let _ = writeln!(out, "  - path: {path}\n    kind: {kind}");
                if kind == &"overview" {
                    out.push_str("    question: （書く）\n");
                }
            }
        }
    }
    if !ignore.is_empty() {
        out.push_str("ignore:\n");
        for s in ignore {
            if let Suggestion::Ignore(path) = s {
                let _ = writeln!(out, "  - {path}");
            }
        }
    }

    out.push_str("\n## 4. ルールに沿っていないファイル\n");
    if !project.configured {
        out.push_str("（docv.yaml がないため、なし）\n");
    } else if diagnostics.is_empty() {
        out.push_str("（なし）\n");
    } else {
        render_diagnostics(&mut out, &diagnostics);
    }

    out.push_str("\n## 5. 次にやること\n");
    for (i, step) in next_steps(project.configured, &suggestions, &diagnostics)
        .iter()
        .enumerate()
    {
        let _ = writeln!(out, "{}. {step}", i + 1);
    }
    Ok(out)
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
    configured: bool,
    suggestions: &[Suggestion],
    diagnostics: &[Diagnostic],
) -> Vec<String> {
    if !configured {
        return vec![
            "docv init で docv.yaml を作り、3 の案を docs に足す".to_string(),
            "もう一度 docv migrate-guide を実行し、4 に出たファイルを直す".to_string(),
        ];
    }
    let mut steps = Vec::new();
    if !suggestions.is_empty() {
        steps.push("3 の案を docv.yaml に足し、もう一度 docv migrate-guide を実行する".to_string());
    }
    if diagnostics.iter().any(|d| d.rule == "format-diff") {
        steps.push(
            "ほかを直す前に docv fix で整形し、整形の差分だけでコミットする。整形の差分と内容の差分を混ぜないため"
                .to_string(),
        );
    }
    if diagnostics.iter().any(|d| d.rule != "format-diff") {
        steps.push(
            "4 に出たファイルを直す。この出力と docv desc を AI Agent に渡して直させてもよい"
                .to_string(),
        );
    }
    if steps.is_empty() {
        steps.push("なし。ドキュメントを変えたら docv check で確かめる".to_string());
    }
    steps
}

/// 管理外のファイルの場所から、docs と ignore に足す案を作る。
fn suggest(unmanaged: &[&str]) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    if unmanaged.contains(&"README.md") {
        suggestions.push(Suggestion::Doc {
            path: "README.md".to_string(),
            kind: "overview",
        });
    }
    // 推測できたディレクトリと、その下の深いところにファイルがあるか。
    let mut dirs: BTreeMap<String, (Option<&'static str>, bool)> = BTreeMap::new();
    for path in unmanaged {
        let parts: Vec<&str> = path.split('/').collect();
        let dir_parts = &parts[..parts.len() - 1];
        for (i, name) in dir_parts.iter().enumerate() {
            let name = name.to_lowercase();
            let guess = if name.contains("generated") {
                Some(None)
            } else {
                GUESSES
                    .iter()
                    .find(|(words, _)| words.iter().any(|w| name.contains(w)))
                    .map(|(_, kind)| Some(*kind))
            };
            if let Some(kind) = guess {
                let dir = dir_parts[..=i].join("/");
                let nested = i + 1 < dir_parts.len();
                let entry = dirs.entry(dir).or_insert((kind, false));
                entry.1 |= nested;
                break;
            }
        }
    }
    for (dir, (kind, nested)) in dirs {
        suggestions.push(match kind {
            None => Suggestion::Ignore(format!("{dir}/**")),
            Some(kind) => Suggestion::Doc {
                path: if nested || kind == "archive" {
                    format!("{dir}/**/*.md")
                } else {
                    format!("{dir}/*.md")
                },
                kind,
            },
        });
    }
    suggestions
}

#[cfg(test)]
mod tests {
    use std::fs;
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
    fn guesses_kinds_from_directories() {
        let suggestions = suggest(&[
            "README.md",
            "docs/Design/a.md",
            "docs/design/sub/b.md",
            "docs/manual/c.md",
            "docs/archive/spec/d.md",
            "docs/generated/e.md",
            "docs/misc/f.md",
            "notes.md",
        ]);
        let doc = |path: &str, kind| Suggestion::Doc {
            path: path.to_string(),
            kind,
        };
        assert_eq!(
            suggestions,
            [
                doc("README.md", "overview"),
                doc("docs/Design/*.md", "spec"),
                doc("docs/archive/**/*.md", "archive"),
                doc("docs/design/**/*.md", "spec"),
                Suggestion::Ignore("docs/generated/**".to_string()),
                doc("docs/manual/*.md", "operation"),
            ]
        );
    }

    #[test]
    fn guides_without_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "README.md", "# R\n");
        write(root, "docs/design/a.md", "# A\n");
        write(root, "docs/design/b.md", "# B\n");
        write(root, "docs/plan/c.md", "# C\n");
        let project = Project::unconfigured(root).unwrap();
        assert_eq!(
            render(&project, OPTIONS).unwrap(),
            "## 1. 設定
docv.yaml: なし。docv init で作る

## 2. 管理外のファイル（4 件）
./            1 件
docs/design/  2 件
docs/plan/    1 件

- README.md
- docs/design/a.md
- docs/design/b.md
- docs/plan/c.md

## 3. docs の案
docs:
  - path: README.md
    kind: overview
    question: （書く）
  - path: docs/design/*.md
    kind: spec
  - path: docs/plan/*.md
    kind: plan

## 4. ルールに沿っていないファイル
（docv.yaml がないため、なし）

## 5. 次にやること
1. docv init で docv.yaml を作り、3 の案を docs に足す
2. もう一度 docv migrate-guide を実行し、4 に出たファイルを直す
"
        );
    }

    #[test]
    fn guides_with_config() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "docv.yaml",
            "version: 1\ndocs:\n  - path: docs/spec/*.md\n    kind: spec\nformat:\n  enabled: true\n",
        );
        write(root, "docs/spec/a.md", "# A\n* x\n");
        let project = Project::open(&Locate::default(), root).unwrap();
        let out = render(&project, OPTIONS).unwrap();
        assert!(
            out.contains("## 4. ルールに沿っていないファイル\nformat-diff       1 件\nquestion-missing  1 件\n\ndocs/spec/a.md\n  1: error question-missing"),
            "{out}"
        );
        assert!(
            out.contains("## 5. 次にやること\n1. ほかを直す前に docv fix で整形し"),
            "{out}"
        );
        assert!(out.contains("\n2. 4 に出たファイルを直す"), "{out}");
    }
}
