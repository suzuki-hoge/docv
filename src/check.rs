//! docv check のルール。Free ファイルのルールは docs/spec/free.md の 3 章、書式の規則と textlint は docs/spec/config.md。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_yaml_ng::Value;

use crate::diagnostic::{Diagnostic, Level};
use crate::markdown::{self, Document, Outline};
use crate::style::{self, Style};
use crate::textlint::Textlint;
use crate::workspace::{Project, Settings};
use crate::{adr, format, meta, slot};

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

/// 管理対象のファイルと Slot を確かめる。targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
pub fn run(project: &Project, targets: &[String], options: Options) -> Result<Vec<Diagnostic>> {
    let words = Words::new(project);
    let managed: HashSet<String> = project.managed()?.into_iter().collect();
    let mut diagnostics = Vec::new();
    let mut textlint_files = Vec::new();
    for path in project.targets(targets)? {
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        diagnostics.extend(check_file(project, &words, &managed, &path, &text));
        if project.settings_of(&path).textlint {
            textlint_files.push(path);
        }
    }
    // Slot のディレクトリの中の .md のファイルには、Free ファイルのルールはかけず、整形と textlint だけをかける。
    for path in project.slot_markdown()? {
        if !targets.is_empty() && !targets.contains(&path) {
            continue;
        }
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        let mut cx = Checker {
            project,
            path: &path,
            settings: project.settings_of(&path),
            diagnostics: Vec::new(),
        };
        check_format(&mut cx, &text);
        diagnostics.extend(cx.diagnostics);
        if project.settings_of(&path).textlint {
            textlint_files.push(path);
        }
    }
    if targets.is_empty() {
        diagnostics.extend(slot::check(project)?);
        diagnostics.extend(adr::check(project)?);
    }
    if options.textlint
        && let Some(textlint) = Textlint::new(project)
    {
        diagnostics.extend(textlint.lint(&textlint_files)?);
    }
    diagnostics.sort();
    Ok(diagnostics)
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
    path: &'a str,
    settings: Settings,
    diagnostics: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn push(&mut self, line: usize, rule: &str, level: Level, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            path: self.path.to_string(),
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
    path: &str,
    text: &str,
) -> Vec<Diagnostic> {
    let mut cx = Checker {
        project,
        path,
        settings: project.settings_of(path),
        diagnostics: Vec::new(),
    };
    let document = markdown::split(text);
    let outline = markdown::outline(&document);

    check_frontmatter(&mut cx, &document);
    check_structure(&mut cx, &document, &outline);

    let tags = meta::read(&document, &outline).tags;
    let rules: HashSet<&str> = project.config.rules_of(&tags).collect();
    if rules.contains("no-past-wording") {
        check_wording(&mut cx, words, &outline);
    }
    if rules.contains("no-done-task") {
        for &line in &outline.done_tasks {
            cx.push(
                line,
                "done-task",
                Level::Warn,
                "終わったタスクが残っている。終わったタスクは消す",
            );
        }
    }
    check_links(&mut cx, &outline);

    if cx.settings.style {
        let style = Style::resolve(project.config.lint.style.as_ref(), None);
        for found in style::check(&style, path, &document, managed) {
            cx.push(found.line, found.rule, Level::Error, found.message);
        }
    }

    check_format(&mut cx, text);
    cx.diagnostics
}

/// 整形できないファイルは、Free ファイルなら frontmatter-invalid、ADR なら adr-invalid が出ているので、ここでは出さない。
fn check_format(cx: &mut Checker, text: &str) {
    if cx.project.config.format.enabled
        && cx.settings.format
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
}

fn check_frontmatter(cx: &mut Checker, document: &Document) {
    let fm = document.frontmatter.as_ref();
    let mapping = match fm {
        None => serde_yaml_ng::Mapping::new(),
        Some(fm) if !fm.closed => {
            cx.push(
                1,
                "frontmatter-invalid",
                Level::Error,
                "YAML front matter を閉じる --- がない",
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
                    "YAML front matter はキーと値の組で書く",
                );
                return;
            }
            Err(e) => {
                cx.push(
                    1,
                    "frontmatter-invalid",
                    Level::Error,
                    format!("YAML front matter が読めない: {e}"),
                );
                return;
            }
        },
    };
    let line_of = |key: &str| fm.map_or(1, |fm| fm.line_of_key(key));

    // overrides で mission を optional にしたパスでは、mission を求めない。
    let required = cx.settings.mission_required;
    match mapping.get("mission") {
        Some(Value::String(m)) => {
            if m.trim_end().contains('\n') {
                cx.push(
                    line_of("mission"),
                    "mission-multiline",
                    Level::Error,
                    "mission は 1 行で書く",
                );
            }
        }
        Some(_) => cx.push(
            line_of("mission"),
            "frontmatter-invalid",
            Level::Error,
            "mission は文字列で書く",
        ),
        None if required => cx.push(
            1,
            "mission-missing",
            Level::Error,
            "mission がない。このファイルが答えることを 1 文で書く",
        ),
        None => {}
    }

    match mapping.get("tags") {
        None => {}
        Some(Value::Sequence(tags)) => {
            for tag in tags {
                match (tag, &cx.project.config.tags) {
                    (Value::String(t), Some(known)) if !known.contains(t) => cx.push(
                        line_of("tags"),
                        "tag-unknown",
                        Level::Error,
                        format!("タグ {t} は docv.yaml の tags にない"),
                    ),
                    (Value::String(_), _) => {}
                    _ => cx.push(
                        line_of("tags"),
                        "frontmatter-invalid",
                        Level::Error,
                        "tags は文字列の配列で書く",
                    ),
                }
            }
        }
        Some(_) => cx.push(
            line_of("tags"),
            "frontmatter-invalid",
            Level::Error,
            "tags は文字列の配列で書く",
        ),
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
                        "「{word}」は過去や実装状態の言い回し。今の姿だけを書き、決めた経緯は ADR に残す"
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
                    format!("見出しの「{word}」は過去を扱う語。決めた経緯は ADR に残す"),
                );
            }
        }
    }
}

fn check_links(cx: &mut Checker, outline: &Outline) {
    let base = Path::new(cx.path)
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    for link in &outline.links {
        let Some((target, _)) = markdown::resolve_link(&base, &link.dest) else {
            continue;
        };
        if !cx.project.root.join(&target).exists() {
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

    const CONFIG: &str = "docs:
  - docs/**/*.md
ignore:
  - docs/ignored/**
tags: [仕様, 計画]
lint:
  tags:
    仕様: [no-past-wording]
    計画: [no-done-task]
  words:
    add: [いずれ]
    remove: [将来的に]
overrides:
  - paths: [docs/tmp/**]
    textlint: false
    style: false
    mission: optional
";

    /// mission を書いた YAML front matter。
    fn fm(rest: &str) -> String {
        format!("---\nmission: q\n{rest}---\n")
    }

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
            ("README.md", "no frontmatter\n"),
            (
                "docs/spec/a.md",
                &format!(
                    "{}\n# A\n\n`以前は` と書かない。[README](../../README.md#x)\n",
                    fm("tags: [仕様]\n")
                ),
            ),
            (
                "docs/plan/p.md",
                &format!("{}\n# P\n\n- [ ] t\n", fm("tags: [計画]\n")),
            ),
            ("docs/ignored/memo.md", "以前は TODO [x](missing.md)\n"),
            // overrides で YAML front matter を求めないパス
            ("docs/tmp/memo.md", "# メモ\n\n**太字**\n"),
            ("docv/service/service.md", "# サービス仕様\n"),
        ]);
        assert!(result.is_empty(), "{result:#?}");
    }

    #[test]
    fn reports_frontmatter_problems() {
        let result = check(&[
            ("docs/spec/a.md", "# A\n"),
            (
                "docs/spec/b.md",
                "---\nmission: |\n  1\n  2\ntags: [仕様, z]\n---\n# B\n",
            ),
            ("docs/spec/c.md", "---\nmission: [\n---\n# C\n"),
        ]);
        let without_c: Vec<&str> = result
            .iter()
            .filter(|r| !r.starts_with("docs/spec/c.md"))
            .map(String::as_str)
            .collect();
        assert_eq!(
            without_c,
            [
                "docs/spec/a.md:1: error mission-missing mission がない。このファイルが答えることを 1 文で書く".to_string(),
                "docs/spec/b.md:2: error mission-multiline mission は 1 行で書く".to_string(),
                "docs/spec/b.md:5: error tag-unknown タグ z は docv.yaml の tags にない".to_string(),
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
            ("docs/spec/a.md", &format!("{}# A\n\n# B\n", fm(""))),
            (
                "docs/plan/p.md",
                &format!("{}# P\n\n- [x] done\n", fm("tags: [計画]\n")),
            ),
        ]);
        assert_eq!(
            result,
            [
                "docs/plan/p.md:7: warn done-task 終わったタスクが残っている。終わったタスクは消す",
                "docs/spec/a.md:6: error title-count h1 が 2 個ある。1 つにする",
            ]
        );
    }

    #[test]
    fn reports_wording_only_in_files_with_the_rule() {
        let result = check(&[
            (
                "docs/spec/a.md",
                &format!(
                    "{}# A\n\n## 変更履歴\n\n以前は X だった。いずれ直す。将来的に直す。\n",
                    fm("tags: [仕様]\n")
                ),
            ),
            (
                "docs/plan/p.md",
                &format!("{}# P\n\n以前は\n", fm("tags: [計画]\n")),
            ),
        ]);
        assert_eq!(
            result,
            [
                "docs/spec/a.md:7: warn past-heading 見出しの「変更履歴」は過去を扱う語。決めた経緯は ADR に残す",
                "docs/spec/a.md:9: warn past-wording 「いずれ」は過去や実装状態の言い回し。今の姿だけを書き、決めた経緯は ADR に残す",
                "docs/spec/a.md:9: warn past-wording 「以前は」は過去や実装状態の言い回し。今の姿だけを書き、決めた経緯は ADR に残す",
            ]
        );
    }

    #[test]
    fn reports_broken_links_and_slot_and_adr_problems() {
        let result = check(&[
            (
                "docs/spec/a.md",
                &format!(
                    "{}# A\n\n[x](missing.md) [y](https://example.com) [z](#top) [w](%E3%81%82.md) [v](/README.md)\n",
                    fm("")
                ),
            ),
            ("docs/spec/あ.md", &format!("{}# あ\n", fm(""))),
            ("README.md", "# R\n"),
            ("docv/service/words.yaml", "groups: 1\n"),
            ("docv/adr/a.md", "# A\n"),
        ]);
        assert_eq!(result.len(), 3, "{result:#?}");
        assert_eq!(
            result[0],
            "docs/spec/a.md:6: error link-broken リンク先 missing.md がない"
        );
        assert!(
            result[1].starts_with("docv/adr/a.md:1: error adr-invalid"),
            "{result:#?}"
        );
        assert_eq!(
            result[2],
            "docv/service/words.yaml:1: error slot-invalid 用語集の Slot の形に合わない: groups: invalid type: integer `1`, expected a sequence at line 1 column 9"
        );
    }

    #[test]
    fn skips_excluded_and_dot_files_in_slots_dir() {
        let result = check(&[
            (".gitignore", "memo.txt\n"),
            ("docv/service/words.yaml", "groups: []\n"),
            // output.dir の既定は docv/build
            ("docv/build/.docv", ""),
            ("docv/build/index.html", "<html></html>\n"),
            ("docv/build/jpg/サービス仕様/用語集.jpg", ""),
            ("docv/service/.DS_Store", ""),
            ("docv/.cache/a.yaml", "a: 1\n"),
            ("docv/service/memo.txt", "メモ\n"),
            ("docv/adr/.draft.md", "# 下書き\n"),
        ]);
        assert!(result.is_empty(), "{result:#?}");
    }

    #[test]
    fn checks_only_format_in_slot_markdown() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "docv.yaml",
            "docs: [docs/**/*.md]\nformat:\n  enabled: true\n",
        );
        write(
            dir.path(),
            "docv/service/manual.md",
            "# M\n\n| a | b |\n| --- | --- |\n| 種類 | c |\n",
        );
        write(
            dir.path(),
            "docv/adr/a.md",
            "---\ndate: 2026-10-13\n---\n\n# A\n\n| 種類 | c |\n| ---- | - |\n",
        );
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        let result: Vec<String> = run(&project, &[], OPTIONS)
            .unwrap()
            .iter()
            .map(|d| d.to_string())
            .collect();
        assert_eq!(
            result,
            ["docv/service/manual.md:3: error format-diff 整形が必要。docv fix で整形する"]
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
        assert!(result.iter().all(|d| d.path == "docs/spec/b.md"));
    }
}
