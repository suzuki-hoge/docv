//! docv check のルール。Free ファイルのルールは docs/spec/free.md の 3 章、書式の規則と textlint は docs/spec/config.md。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_yaml_ng::Value;

use crate::diagnostic::Diagnostic;
use crate::markdown::{self, Document, Outline};
use crate::style::{self, Style};
use crate::textlint::Textlint;
use crate::workspace::Project;
use crate::{adr, format, slot};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// lint.textlint が enable のときに textlint をかけるか。--no-textlint で false になる。
    pub textlint: bool,
}

/// 管理対象のファイルと Slot を確かめる。targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
pub fn run(project: &Project, targets: &[String], options: Options) -> Result<Vec<Diagnostic>> {
    let style = Style::new(&project.config);
    let managed: HashSet<String> = project.managed()?.into_iter().collect();
    let mut diagnostics = Vec::new();
    let mut textlint_files = Vec::new();
    for path in project.targets(targets)? {
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        diagnostics.extend(check_file(project, &style, &managed, &path, &text));
        textlint_files.push(path);
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
            diagnostics: Vec::new(),
        };
        check_format(&mut cx, &text);
        diagnostics.extend(cx.diagnostics);
        textlint_files.push(path);
    }
    // docv.yaml と YAML の Slot は、末尾の改行だけを整形と連動させる。
    if project.config.format.enabled {
        for path in project.yaml_targets(targets)? {
            let text = fs::read_to_string(project.root.join(&path))
                .with_context(|| format!("{path} を読めない"))?;
            let formatted = format::final_newline(&text);
            if formatted != text {
                diagnostics.push(Diagnostic {
                    line: format::first_diff_line(&text, &formatted),
                    path,
                    rule: "format-diff".to_string(),
                    message: "末尾の改行を 1 つにする。docv fix で整形する".to_string(),
                });
            }
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

struct Checker<'a> {
    project: &'a Project,
    path: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn push(&mut self, line: usize, rule: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            path: self.path.to_string(),
            line,
            rule: rule.to_string(),
            message: message.into(),
        });
    }
}

fn check_file(
    project: &Project,
    style: &Style,
    managed: &HashSet<String>,
    path: &str,
    text: &str,
) -> Vec<Diagnostic> {
    let mut cx = Checker {
        project,
        path,
        diagnostics: Vec::new(),
    };
    let document = markdown::split(text);
    let outline = markdown::outline(&document);

    check_frontmatter(&mut cx, &document);
    check_structure(&mut cx, &document, &outline);

    check_links(&mut cx, &outline);
    for found in style::check(style, path, &document, managed) {
        cx.push(found.line, found.rule, found.message);
    }

    check_format(&mut cx, text);
    cx.diagnostics
}

/// 整形できないファイルは、Free ファイルなら frontmatter-invalid、ADR なら adr-invalid が出ているので、ここでは出さない。
fn check_format(cx: &mut Checker, text: &str) {
    if cx.project.config.format.enabled
        && let Ok(formatted) = format::format(text)
        && formatted != text
    {
        cx.push(
            format::first_diff_line(text, &formatted),
            "format-diff",
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
                    "YAML front matter はキーと値の組で書く",
                );
                return;
            }
            Err(e) => {
                cx.push(
                    1,
                    "frontmatter-invalid",
                    format!("YAML front matter が読めない: {e}"),
                );
                return;
            }
        },
    };
    let line_of = |key: &str| fm.map_or(1, |fm| fm.line_of_key(key));

    match mapping.get("mission") {
        Some(Value::String(m)) => {
            if m.trim_end().contains('\n') {
                cx.push(
                    line_of("mission"),
                    "mission-multiline",
                    "mission は 1 行で書く",
                );
            }
        }
        Some(_) => cx.push(
            line_of("mission"),
            "frontmatter-invalid",
            "mission は文字列で書く",
        ),
        None => cx.push(
            1,
            "mission-missing",
            "mission がない。このファイルを読むと何が分かるかを 1 文で書く",
        ),
    }

    match mapping.get("tags") {
        None => {}
        Some(Value::Sequence(tags)) => {
            for tag in tags {
                match tag {
                    Value::String(t) if cx.project.config.tag_color(t).is_none() => cx.push(
                        line_of("tags"),
                        "tag-unknown",
                        format!(
                            "タグ {t} は docv.yaml の tags にない。使うなら、docv.yaml の tags に name と color を書いて足す"
                        ),
                    ),
                    Value::String(_) => {}
                    _ => cx.push(
                        line_of("tags"),
                        "frontmatter-invalid",
                        "tags は文字列の配列で書く",
                    ),
                }
            }
        }
        Some(_) => cx.push(
            line_of("tags"),
            "frontmatter-invalid",
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
            format!("h1 が {} 個ある。1 つにする", h1.len()),
        );
    }
}

fn check_links(cx: &mut Checker, outline: &Outline) {
    let base = Path::new(cx.path)
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    for link in &outline.links {
        // ページ内のリンクは、このファイルの見出しを指す。
        if let Some(fragment) = link.dest.strip_prefix('#') {
            if !markdown::anchors(outline).contains(&markdown::percent_decode(fragment)) {
                cx.push(
                    link.line,
                    "link-anchor",
                    format!("リンク先の見出し #{fragment} がない"),
                );
            }
            continue;
        }
        let Some((target, fragment)) = markdown::resolve_link(&base, &link.dest) else {
            continue;
        };
        let path = cx.project.root.join(&target);
        if !path.exists() {
            cx.push(
                link.line,
                "link-broken",
                format!("リンク先 {} がない", link.dest),
            );
            continue;
        }
        // アンカーは、Markdown のファイルの見出しを指すときだけ確かめる。
        let Some(fragment) = fragment else {
            continue;
        };
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let anchors = markdown::anchors(&markdown::outline(&markdown::split(&text)));
        if !anchors.contains(&markdown::percent_decode(&fragment)) {
            cx.push(
                link.line,
                "link-anchor",
                format!(
                    "リンク先 {} に見出し #{fragment} がない",
                    markdown::slash_path(&target)
                ),
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

    const CONFIG: &str = "ignore:
  - docs/ignored/**
tags:
  - name: 仕様
    color: blue
  - name: 計画
    color: green
lint:
  bold: deny
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
            ("README.md", "no frontmatter\n\n## X\n"),
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
            ("docs/ignored/memo.md", "**太字** [x](missing.md)\n"),
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
                "docs/spec/a.md:1: mission-missing mission がない。このファイルを読むと何が分かるかを 1 文で書く".to_string(),
                "docs/spec/b.md:2: mission-multiline mission は 1 行で書く".to_string(),
                "docs/spec/b.md:5: tag-unknown タグ z は docv.yaml の tags にない。使うなら、docv.yaml の tags に name と color を書いて足す".to_string(),
            ]
        );
        assert!(
            result
                .iter()
                .any(|r| r.starts_with("docs/spec/c.md:1: frontmatter-invalid"))
        );
    }

    #[test]
    fn reports_structure_problems() {
        let result = check(&[
            ("docs/spec/a.md", &format!("{}# A\n\n# B\n", fm(""))),
            // 終わったタスクは確かめない
            ("docs/plan/p.md", &format!("{}# P\n\n- [x] done\n", fm(""))),
            // 書式の規則は、lint で deny にしたものと、章番号とリンクの表示文字を確かめる
            (
                "docs/spec/b.md",
                &format!("{}# B\n\n## 1. 一\n\n**b** [a](a.md)\n\n## 二\n", fm("")),
            ),
        ]);
        assert_eq!(
            result,
            [
                "docs/spec/a.md:6: title-count h1 が 2 個ある。1 つにする",
                "docs/spec/b.md:8: link-text リンクの表示文字を spec/a.md にする",
                "docs/spec/b.md:8: style-bold 太字を使わない",
                "docs/spec/b.md:10: heading-number h2 の見出しを「2. 」で始める。章番号は 1 から順に振る",
            ]
        );
    }

    #[test]
    fn reports_broken_links_and_slot_and_adr_problems() {
        let result = check(&[
            (
                "docs/spec/a.md",
                &format!(
                    "{}# A\n\n[x](missing.md) [y](https://example.com) [z](#a) [spec/あ.md](%E3%81%82.md) [v](/README.md)\n",
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
            "docs/spec/a.md:6: link-broken リンク先 missing.md がない"
        );
        assert!(
            result[1].starts_with("docv/adr/a.md:1: adr-invalid"),
            "{result:#?}"
        );
        assert_eq!(
            result[2],
            "docv/service/words.yaml:1: slot-invalid 用語集の Slot の形に合わない: groups: invalid type: integer `1`, expected a sequence at line 1 column 9"
        );
    }

    #[test]
    fn reports_missing_anchors() {
        let result = check(&[
            (
                "docs/spec/a.md",
                &format!(
                    "{}# A\n\n## 一\n\n## 一\n\n[x](#一-1) [x](#%E4%B8%80) [x](#%E4%BA%8C) [spec/b.md](b.md#2-二) [spec/b.md](b.md#1-一) [y](/docv.yaml#x)\n",
                    fm("")
                ),
            ),
            ("docs/spec/b.md", &format!("{}# B\n\n## 1. 一\n", fm(""))),
        ]);
        assert_eq!(
            result,
            [
                "docs/spec/a.md:10: link-anchor リンク先 docs/spec/b.md に見出し #2-二 がない",
                "docs/spec/a.md:10: link-anchor リンク先の見出し #%E4%BA%8C がない",
            ]
        );
    }

    #[test]
    fn skips_excluded_and_dot_files_in_slots_dir() {
        let result = check(&[
            (".gitignore", "memo.txt\n"),
            ("docv/service/words.yaml", "groups: []\n"),
            // dir.html の既定は docv/build
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
        write(dir.path(), "docv.yaml", "format:\n  enabled: true\n");
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
        write(dir.path(), "docv/service/words.yaml", "groups: []");
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        let result: Vec<String> = run(&project, &[], OPTIONS)
            .unwrap()
            .iter()
            .map(|d| d.to_string())
            .collect();
        assert_eq!(
            result,
            [
                "docv/service/manual.md:3: format-diff 整形が必要。docv fix で整形する",
                "docv/service/words.yaml:1: format-diff 末尾の改行を 1 つにする。docv fix で整形する",
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
        assert!(result.iter().all(|d| d.path == "docs/spec/b.md"));
    }
}
