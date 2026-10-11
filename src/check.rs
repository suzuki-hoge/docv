//! docv check のルール。docs/spec/check.md。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_yaml_ng::Value;

use crate::config::CheckLevel;
use crate::diagnostic::{Diagnostic, Level};
use crate::format;
use crate::kind::{Kind, Time};
use crate::markdown::{self, Document, Outline};
use crate::style::{self, Style};
use crate::textlint::Textlint;
use crate::workspace::{Class, DocFile, Project};

pub const PAST_WORDING: &[&str] = &[
    "以前は",
    "従来は",
    "旧仕様",
    "旧方式",
    "に変更した",
    "に変更しました",
    "を廃止した",
    "を廃止しました",
    "未実装",
    "実装予定",
    "実装済み",
    "今後は",
    "将来的に",
    "TODO",
    "FIXME",
];

pub const PAST_HEADING: &[&str] = &[
    "変更履歴",
    "更新履歴",
    "経緯",
    "廃案",
    "不採用",
    "未実装",
    "今後の予定",
];

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// lint.textlint.enabled が true のときに textlint をかけるか。--no-textlint で false になる。
    pub textlint: bool,
}

/// 管理対象のファイルを確かめる。targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
pub fn run(project: &Project, targets: &[String], options: Options) -> Result<Vec<Diagnostic>> {
    let words = Words::new(project);
    let managed = managed_paths(project)?;
    let mut diagnostics = Vec::new();
    let mut levels = HashMap::new();
    let mut textlint_files = Vec::new();
    for (file, index) in project.targets(targets)? {
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        diagnostics.extend(check_file(project, &words, &managed, &file, index, &text));
        levels.insert(file.path.clone(), project.check_level_of(index));
        if !project.kind_of(index).is_archive() {
            textlint_files.push(file.path);
        }
    }
    if options.textlint
        && let Some(textlint) = Textlint::new(project)
    {
        diagnostics.extend(textlint.lint(&textlint_files)?);
    }
    for d in &mut diagnostics {
        if levels.get(&d.path) == Some(&CheckLevel::Warn) {
            d.level = Level::Warn;
        }
    }
    diagnostics.sort();
    Ok(diagnostics)
}

/// 管理対象のファイル（tmp を除く）のパス。link_text の規則で使う。
fn managed_paths(project: &Project) -> Result<HashSet<String>> {
    Ok(project
        .files()?
        .into_iter()
        .filter_map(|file| match file.class {
            Class::Managed(index) if !project.kind_of(index).is_tmp() => Some(file.path),
            _ => None,
        })
        .collect())
}

struct Words {
    wording: Vec<String>,
    heading: Vec<String>,
}

impl Words {
    /// lint.words.add は past-wording に足し、lint.words.remove は両方から外す。
    fn new(project: &Project) -> Self {
        let config = &project.config.lint.words;
        let keep = |words: &[&str]| -> Vec<String> {
            words
                .iter()
                .filter(|w| !config.remove.iter().any(|r| r == *w))
                .map(|w| w.to_string())
                .collect()
        };
        let mut wording = keep(PAST_WORDING);
        wording.extend(config.add.iter().cloned());
        let heading = keep(PAST_HEADING);
        Words { wording, heading }
    }
}

struct Checker<'a> {
    project: &'a Project,
    file: &'a DocFile,
    kind: &'a Kind,
    diagnostics: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn push(&mut self, line: usize, rule: &str, level: Level, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            path: self.file.path.clone(),
            line,
            rule: rule.to_string(),
            level,
            message: message.into(),
        });
    }
}

fn check_file(
    project: &Project,
    words: &Words,
    managed: &HashSet<String>,
    file: &DocFile,
    index: usize,
    text: &str,
) -> Vec<Diagnostic> {
    let mut cx = Checker {
        project,
        file,
        kind: project.kind_of(index),
        diagnostics: Vec::new(),
    };
    let document = markdown::split(text);
    let outline = markdown::outline(&document);

    check_frontmatter(&mut cx, index, &document);
    check_structure(&mut cx, &document, &outline);
    if cx.kind.time == Time::Present {
        check_wording(&mut cx, words, &outline);
    }
    if !cx.kind.is_archive() {
        check_links(&mut cx, &outline);
    }

    let entry = project.entry(index);
    let mut style = Style::resolve(project.config.lint.style.as_ref(), entry.style.as_ref());
    if cx.kind.is_archive() {
        style = style.for_archive();
    }
    for found in style::check(&style, &file.path, &document, managed) {
        cx.push(found.line, found.rule, Level::Error, found.message);
    }

    // 整形できないファイルは frontmatter-invalid が出ているので、ここでは出さない。
    if project.config.format.enabled
        && let Ok(formatted) = format::format(text)
        && formatted != text
    {
        cx.push(
            format::first_diff_line(text, &formatted),
            "format-diff",
            Level::Error,
            "整形が必要。docv fix で整形する",
        );
    }
    cx.diagnostics
}

fn check_frontmatter(cx: &mut Checker, index: usize, document: &Document) {
    let entry = cx.project.entry(index);
    let fm = document.frontmatter.as_ref();
    let mapping = match fm {
        None => serde_yaml_ng::Mapping::new(),
        Some(fm) if !fm.closed => {
            cx.push(
                1,
                "frontmatter-invalid",
                Level::Error,
                "frontmatter を閉じる --- がない",
            );
            return;
        }
        Some(fm) => match serde_yaml_ng::from_str::<Value>(&fm.yaml) {
            Ok(Value::Mapping(m)) => m,
            Ok(Value::Null) => serde_yaml_ng::Mapping::new(),
            Ok(_) => {
                cx.push(
                    1,
                    "frontmatter-invalid",
                    Level::Error,
                    "frontmatter はキーと値の組で書く",
                );
                return;
            }
            Err(e) => {
                cx.push(
                    1,
                    "frontmatter-invalid",
                    Level::Error,
                    format!("frontmatter が YAML として読めない: {e}"),
                );
                return;
            }
        },
    };
    let line_of = |key: &str| fm.map_or(1, |fm| fm.line_of_key(key));

    match (mapping.get("question"), &entry.question) {
        (Some(_), Some(_)) => cx.push(
            line_of("question"),
            "question-duplicated",
            Level::Error,
            "question が frontmatter と docv.yaml の両方にある",
        ),
        (Some(Value::String(q)), None) => {
            if q.trim_end().contains('\n') {
                cx.push(
                    line_of("question"),
                    "question-multiline",
                    Level::Error,
                    "question は 1 行で書く",
                );
            }
        }
        (Some(_), None) => cx.push(
            line_of("question"),
            "frontmatter-invalid",
            Level::Error,
            "question は文字列で書く",
        ),
        (None, Some(_)) => {}
        (None, None) => cx.push(
            1,
            "question-missing",
            Level::Error,
            "question がない。このファイルが答える問いを 1 文で書く",
        ),
    }

    match mapping.get("topics") {
        None => {}
        Some(Value::Sequence(topics)) => {
            for topic in topics {
                match topic {
                    Value::String(t) if cx.project.config.topics.contains(t) => {}
                    Value::String(t) => cx.push(
                        line_of("topics"),
                        "topic-unknown",
                        Level::Error,
                        format!("話題 {t} は docv.yaml の topics にない"),
                    ),
                    _ => cx.push(
                        line_of("topics"),
                        "frontmatter-invalid",
                        Level::Error,
                        "topics は文字列の配列で書く",
                    ),
                }
            }
        }
        Some(_) => cx.push(
            line_of("topics"),
            "frontmatter-invalid",
            Level::Error,
            "topics は文字列の配列で書く",
        ),
    }

    if cx.kind.is_plan() {
        let valid =
            matches!(mapping.get("status"), Some(Value::String(s)) if s == "todo" || s == "doing");
        if !valid {
            cx.push(
                line_of("status"),
                "status-invalid",
                Level::Error,
                "status は todo か doing",
            );
        }
    }

    if cx.kind.is_archive() && !matches!(mapping.get("source"), Some(Value::String(_))) {
        cx.push(
            line_of("source"),
            "source-missing",
            Level::Error,
            "source（移した元のファイルのパス）がない",
        );
    }
}

fn check_structure(cx: &mut Checker, document: &Document, outline: &Outline) {
    let h1: Vec<_> = outline.headings.iter().filter(|h| h.level == 1).collect();
    if h1.len() != 1 {
        let line = h1.get(1).map_or(document.body_start_line, |h| h.line);
        cx.push(
            line,
            "title-count",
            Level::Error,
            format!("h1 が {} 個ある。1 つにする", h1.len()),
        );
    }

    let h2: Vec<_> = outline.headings.iter().filter(|h| h.level == 2).collect();
    for word in cx.kind.required_sections.clone() {
        if !h2.iter().any(|h| h.text.contains(&word)) {
            cx.push(
                1,
                "section-required",
                Level::Error,
                format!("「{word}」を含む章（h2）がない"),
            );
        }
    }

    if cx.kind.is_archive() {
        check_archive_entries(cx, document, outline);
    }

    if cx.kind.is_plan() {
        for &line in &outline.done_tasks {
            cx.push(
                line,
                "plan-done-task",
                Level::Warn,
                "終わったタスクが残っている。終わったタスクは消す",
            );
        }
    }
}

fn check_archive_entries(cx: &mut Checker, document: &Document, outline: &Outline) {
    let lines: Vec<&str> = document.body.lines().collect();
    let h2: Vec<_> = outline.headings.iter().filter(|h| h.level == 2).collect();
    for (i, heading) in h2.iter().enumerate() {
        if !is_dated(&heading.text) {
            cx.push(
                heading.line,
                "archive-entry-format",
                Level::Error,
                "章の見出しを「YYYY-MM-DD 見出し」にする",
            );
        }
        let start = heading.line - document.body_start_line + 1;
        let end = h2
            .get(i + 1)
            .map_or(lines.len(), |next| next.line - document.body_start_line);
        let has_reason = lines[start.min(lines.len())..end.min(lines.len())]
            .iter()
            .any(|l| {
                let l = l.trim_start();
                ["- 理由:", "- 理由：", "* 理由:", "* 理由："]
                    .iter()
                    .any(|p| l.starts_with(p))
            });
        if !has_reason {
            cx.push(
                heading.line,
                "archive-entry-format",
                Level::Error,
                "章に「- 理由:」の行がない",
            );
        }
    }
}

fn is_dated(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() > 11
        && bytes[..10].iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        })
        && bytes[10] == b' '
}

fn check_wording(cx: &mut Checker, words: &Words, outline: &Outline) {
    for prose in &outline.prose {
        for word in &words.wording {
            if prose.text.contains(word.as_str()) {
                cx.push(
                    prose.line,
                    "past-wording",
                    Level::Warn,
                    format!(
                        "「{word}」は過去や実装状態の言い回し。経緯は docv retire で archive に移す"
                    ),
                );
            }
        }
    }
    for heading in &outline.headings {
        for word in &words.heading {
            if heading.prose.contains(word.as_str()) {
                cx.push(
                    heading.line,
                    "past-heading",
                    Level::Warn,
                    format!("見出しの「{word}」は過去を扱う語。経緯は archive に移す"),
                );
            }
        }
    }
}

fn check_links(cx: &mut Checker, outline: &Outline) {
    let root = cx.project.root.clone();
    let base = Path::new(&cx.file.path)
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    for link in &outline.links {
        let Some((target, _)) = markdown::resolve_link(&base, &link.dest) else {
            continue;
        };
        let rel = markdown::slash_path(&target);
        if cx.project.is_tmp_path(&rel) {
            cx.push(
                link.line,
                "link-to-tmp",
                Level::Error,
                format!(
                    "tmp のファイル {} にリンクしている。tmp はいつ消えるか分からないので、リンクしない",
                    link.dest
                ),
            );
        } else if !root.join(&target).exists() {
            cx.push(
                link.line,
                "link-broken",
                Level::Error,
                format!("リンク先 {} がない", link.dest),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    const OPTIONS: Options = Options { textlint: true };

    const CONFIG: &str = "version: 1
topics: [a]
docs:
  - path: README.md
    kind: overview
    question: q
  - path: docs/spec/*.md
    kind: spec
  - path: docs/cost/*.md
    kind: cost
  - path: docs/plan/*.md
    kind: plan
  - path: docs/archive/**/*.md
    kind: archive
  - path: docs/loose/*.md
    kind: spec
    check: warn
  - path: docs/off/*.md
    kind: spec
    check: off
  - path: docs/tmp/**/*.md
    kind: tmp
lint:
  words:
    add: [いずれ]
    remove: [将来的に]
";

    fn check(files: &[(&str, &str)]) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        run(&project, &[], OPTIONS)
            .unwrap()
            .iter()
            .map(|d| d.to_string())
            .collect()
    }

    #[test]
    fn passes_valid_files() {
        let result = check(&[
            ("README.md", "# R\n\n[spec](docs/spec/a.md)\n"),
            (
                "docs/spec/a.md",
                "---\nquestion: q\ntopics: [a]\n---\n\n# A\n\n`以前は` と書かない。[README](../../README.md#x)\n",
            ),
            (
                "docs/cost/c.md",
                "---\nquestion: q\n---\n\n# C\n\n## 1. 単価\n\n## 2. 見積もり\n",
            ),
            (
                "docs/plan/p.md",
                "---\nquestion: q\nstatus: doing\n---\n\n# P\n\n- [ ] t\n",
            ),
            (
                "docs/archive/spec/a.md",
                "---\nquestion: q\nsource: docs/spec/a.md\n---\n\n# A の経緯\n\n## 2026-10-11 案\n\n- 移した元: [x](gone.md)\n- 理由: r\n\n以前は\n",
            ),
            ("docs/off/x.md", "no frontmatter\n"),
        ]);
        assert!(result.is_empty(), "{result:#?}");
    }

    #[test]
    fn reports_frontmatter_problems() {
        let result = check(&[
            ("README.md", "---\nquestion: dup\n---\n# R\n"),
            ("docs/spec/a.md", "# A\n"),
            (
                "docs/spec/b.md",
                "---\nquestion: |\n  1\n  2\ntopics: [a, z]\n---\n# B\n",
            ),
            ("docs/spec/c.md", "---\nquestion: [\n---\n# C\n"),
            (
                "docs/plan/p.md",
                "---\nquestion: q\nstatus: done\n---\n# P\n",
            ),
            ("docs/archive/a.md", "---\nquestion: q\n---\n# A\n"),
        ]);
        let without_c: Vec<&str> = result
            .iter()
            .filter(|r| !r.starts_with("docs/spec/c.md"))
            .map(String::as_str)
            .collect();
        assert_eq!(
            without_c,
            [
                "README.md:2: error question-duplicated question が frontmatter と docv.yaml の両方にある",
                "docs/archive/a.md:1: error source-missing source（移した元のファイルのパス）がない",
                "docs/plan/p.md:3: error status-invalid status は todo か doing",
                "docs/spec/a.md:1: error question-missing question がない。このファイルが答える問いを 1 文で書く",
                "docs/spec/b.md:2: error question-multiline question は 1 行で書く",
                "docs/spec/b.md:5: error topic-unknown 話題 z は docv.yaml の topics にない",
            ]
        );
        assert!(
            result
                .iter()
                .any(|r| r.starts_with("docs/spec/c.md:1: error frontmatter-invalid"))
        );
    }

    #[test]
    fn reports_structure_problems() {
        let result = check(&[
            ("docs/spec/a.md", "---\nquestion: q\n---\n# A\n\n# B\n"),
            (
                "docs/cost/c.md",
                "---\nquestion: q\n---\n# C\n\n## 1. 単価\n",
            ),
            (
                "docs/plan/p.md",
                "---\nquestion: q\nstatus: todo\n---\n# P\n\n- [x] done\n",
            ),
            (
                "docs/archive/a.md",
                "---\nquestion: q\nsource: s\n---\n# A\n\n## 案\n\n本文\n",
            ),
        ]);
        assert_eq!(
            result,
            [
                "docs/archive/a.md:7: error archive-entry-format 章に「- 理由:」の行がない",
                "docs/archive/a.md:7: error archive-entry-format 章の見出しを「YYYY-MM-DD 見出し」にする",
                "docs/cost/c.md:1: error section-required 「見積もり」を含む章（h2）がない",
                "docs/plan/p.md:7: warn plan-done-task 終わったタスクが残っている。終わったタスクは消す",
                "docs/spec/a.md:6: error title-count h1 が 2 個ある。1 つにする",
            ]
        );
    }

    #[test]
    fn reports_wording_only_in_present_kinds() {
        let result = check(&[
            (
                "docs/spec/a.md",
                "---\nquestion: q\n---\n# A\n\n## 変更履歴\n\n以前は X だった。いずれ直す。将来的に直す。\n",
            ),
            (
                "docs/plan/p.md",
                "---\nquestion: q\nstatus: todo\n---\n# P\n\n以前は\n",
            ),
        ]);
        assert_eq!(
            result,
            [
                "docs/spec/a.md:6: warn past-heading 見出しの「変更履歴」は過去を扱う語。経緯は archive に移す",
                "docs/spec/a.md:8: warn past-wording 「いずれ」は過去や実装状態の言い回し。経緯は docv retire で archive に移す",
                "docs/spec/a.md:8: warn past-wording 「以前は」は過去や実装状態の言い回し。経緯は docv retire で archive に移す",
            ]
        );
    }

    #[test]
    fn reports_broken_links_and_lowers_level() {
        let result = check(&[
            (
                "docs/spec/a.md",
                "---\nquestion: q\n---\n# A\n\n[x](missing.md) [y](https://example.com) [z](#top) [w](%E3%81%82.md) [v](/README.md)\n",
            ),
            ("docs/spec/あ.md", "---\nquestion: q\n---\n# あ\n"),
            ("README.md", "# R\n"),
            ("docs/loose/l.md", "# L\n"),
        ]);
        assert_eq!(
            result,
            [
                "docs/loose/l.md:1: warn question-missing question がない。このファイルが答える問いを 1 文で書く",
                "docs/spec/a.md:6: error link-broken リンク先 missing.md がない",
            ]
        );
    }

    #[test]
    fn reports_links_to_tmp_and_skips_tmp_files() {
        let result = check(&[
            (
                "docs/spec/a.md",
                "---\nquestion: q\n---\n# A\n\n[memo](../tmp/memo.md) [gone](../tmp/gone.md)\n",
            ),
            ("docs/tmp/memo.md", "以前は TODO [x](missing.md)\n"),
        ]);
        assert_eq!(
            result,
            [
                "docs/spec/a.md:6: error link-to-tmp tmp のファイル ../tmp/gone.md にリンクしている。tmp はいつ消えるか分からないので、リンクしない",
                "docs/spec/a.md:6: error link-to-tmp tmp のファイル ../tmp/memo.md にリンクしている。tmp はいつ消えるか分からないので、リンクしない",
            ]
        );
    }

    #[test]
    fn checks_only_targets() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        write(dir.path(), "docs/spec/a.md", "# A\n");
        write(dir.path(), "docs/spec/b.md", "# B\n");
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        let result = run(&project, &["docs/spec/b.md".to_string()], OPTIONS).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "docs/spec/b.md");
    }
}
