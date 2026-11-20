//! docv check のルール。Free ファイルのルールは docv/slot/service/service/free.md の 3 章、書式の規則と textlint は docv/slot/service/service/config.md。
//! Free ファイルと Slot の YAML front matter の adr と pdr は、決定があるかと、打ち消されていないか（warning）を確かめる。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_yaml_ng::Value;

use crate::decision::{Decision, Kind};
use crate::diagnostic::Diagnostic;
use crate::markdown::{self, Document, Outline};
use crate::meta;
use crate::style::{self, Style};
use crate::textlint::Textlint;
use crate::workspace::Project;
use crate::{decision, format, layout, slot, version};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// lint.textlint が enable のときに textlint をかけるか。--no-textlint で false になる。
    pub textlint: bool,
}

/// 管理対象のファイル、Slot、ADR と PDR を確かめる。targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
/// targets で絞るのは確かめるファイルだけで、ファイルにかける検査は、targets があってもなくても同じ。
pub fn run(project: &Project, targets: &[String], options: Options) -> Result<Vec<Diagnostic>> {
    let style = Style::new(&project.config);
    let wanted = |path: &str| targets.is_empty() || targets.iter().any(|t| t == path);
    let read = |path: &str| {
        fs::read_to_string(project.root.join(path)).with_context(|| format!("{path} を読めない"))
    };
    let known = Known::load(project)?;
    let mut diagnostics = Vec::new();
    let mut textlint_files = Vec::new();
    for path in project.targets(targets)? {
        let text = read(&path)?;
        diagnostics.extend(check_markdown(
            project,
            &known,
            &style,
            &path,
            &text,
            Front::Free,
        ));
        textlint_files.push(path);
    }
    // Slot、ADR、PDR には、YAML front matter のほかは、Free ファイルと同じ検査をかける。
    // Slot の YAML front matter の形は slot::check で、ADR と PDR の YAML front matter は decision::check で確かめる。
    for path in project.docv_markdown()?.into_iter().filter(|p| wanted(p)) {
        let text = read(&path)?;
        let front = if slot::def_of_path(project, &path).is_some() {
            Front::Slot
        } else {
            Front::Other
        };
        diagnostics.extend(check_markdown(project, &known, &style, &path, &text, front));
        textlint_files.push(path);
    }
    // docv.yaml は、末尾の改行だけを整形と連動させる。
    if project.config.format.enabled
        && let Some(path) = project.config_target(targets)
    {
        let text = read(&path)?;
        let formatted = format::final_newline(&text);
        if formatted != text {
            diagnostics.push(Diagnostic::error(
                path,
                format::first_diff_line(&text, &formatted),
                "format-diff",
                "末尾の改行を 1 つにする。docv fix で整形する",
            ));
        }
    }
    // Slot、ADR、PDR は、ファイルどうしのつながり（supersedes など）も見るので、すべて読んでから絞る。
    diagnostics.extend(
        slot::check(project)?
            .into_iter()
            .filter(|d| wanted(&d.path)),
    );
    diagnostics.extend(
        decision::check(project)?
            .into_iter()
            .filter(|d| wanted(&d.path)),
    );
    // document_root の直下に置いたものと、画像の持ち主。
    diagnostics.extend(
        layout::check(project)?
            .into_iter()
            .filter(|d| wanted(&d.path)),
    );
    // docv.yaml の docv_version と、docv.yaml の 1 行目のスキーマの URL の版。
    diagnostics.extend(
        version::check(project)?
            .into_iter()
            .filter(|d| wanted(&d.path)),
    );
    if options.textlint
        && let Some(textlint) = Textlint::new(project)
    {
        diagnostics.extend(textlint.lint(&textlint_files)?);
    }
    diagnostics.sort();
    Ok(diagnostics)
}

/// ファイルの種類ごとの、YAML front matter の確かめ方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Front {
    /// Free ファイル。mission、tags、adr、pdr を確かめる。
    Free,
    /// Slot（分けたページも）。形は slot::check が確かめるので、adr と pdr の id だけを確かめる。
    Slot,
    /// ADR と PDR（decision::check が確かめる）と、Slot の名前でないファイル（slot::check が報告する）。ここでは確かめない。
    Other,
}

/// 関わる決定の id を確かめるための、ADR と PDR。
struct Known {
    adr: Records,
    pdr: Records,
}

struct Records {
    /// ファイルのある決定の id。読めない決定（adr-invalid など）も含め、ないとは言わない。
    ids: Vec<String>,
    /// 読めた決定。打ち消されたかを見る。
    decisions: Vec<Decision>,
}

impl Known {
    fn load(project: &Project) -> Result<Known> {
        let records = |kind| -> Result<Records> {
            Ok(Records {
                ids: decision::ids(project, kind)?,
                decisions: decision::load(project, kind)?,
            })
        };
        Ok(Known {
            adr: records(Kind::Adr)?,
            pdr: records(Kind::Pdr)?,
        })
    }

    fn of(&self, kind: Kind) -> &Records {
        match kind {
            Kind::Adr => &self.adr,
            Kind::Pdr => &self.pdr,
        }
    }
}

struct Checker<'a> {
    project: &'a Project,
    known: &'a Known,
    path: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn push(&mut self, line: usize, rule: &str, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(self.path, line, rule, message));
    }

    fn warn(&mut self, line: usize, rule: &str, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::warning(self.path, line, rule, message));
    }
}

/// Markdown のファイル 1 つ。YAML front matter は、front に合わせて確かめる。
fn check_markdown(
    project: &Project,
    known: &Known,
    style: &Style,
    path: &str,
    text: &str,
    front: Front,
) -> Vec<Diagnostic> {
    let mut cx = Checker {
        project,
        known,
        path,
        diagnostics: Vec::new(),
    };
    let document = markdown::split(text);
    let outline = markdown::outline(&document);

    match front {
        Front::Free => check_frontmatter(&mut cx, &document),
        Front::Slot => check_related(&mut cx, &document),
        Front::Other => {}
    }
    check_structure(&mut cx, &document, &outline);

    let base = Path::new(path).parent().unwrap_or(Path::new(""));
    check_links(&mut cx, &outline, base);
    for found in style::check(style, &document) {
        cx.push(found.line, found.rule, found.message);
    }

    check_format(&mut cx, text);
    cx.diagnostics
}

/// 整形できないファイルは、Free ファイルなら frontmatter-invalid、ADR と PDR なら adr-invalid と pdr-invalid が出ているので、ここでは出さない。
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

    for kind in Kind::ALL {
        if let Some(message) = meta::related_type_problem(kind, mapping.get(kind.dir())) {
            cx.push(line_of(kind.dir()), "frontmatter-invalid", message);
        }
    }
    check_related(cx, document);
}

/// YAML front matter の adr と pdr に書いた id の決定があるかと、打ち消されていないかを確かめる。
/// ADR と PDR は、同じ id でも別の決定なので、adr は ADR だけ、pdr は PDR だけから探す。
/// 打ち消された決定は、直さなくても困らないので warning にする。HTML のページの末尾には出さない。
/// 型の誤りは、Free ファイルなら frontmatter-invalid、Slot なら slot-invalid で出しているので、読める id だけを確かめる。
fn check_related(cx: &mut Checker, document: &Document) {
    let Some(fm) = document.frontmatter.as_ref().filter(|fm| fm.closed) else {
        return;
    };
    let related = meta::related(document);
    for kind in Kind::ALL {
        let key = kind.dir();
        let records = cx.known.of(kind);
        for id in related.ids(kind) {
            let line = fm.line_of_item(key, id);
            if !records.ids.contains(id) {
                // 種類を取り違えていれば、もう一方の項目を案内する。
                let hint = Kind::ALL
                    .into_iter()
                    .find(|other| *other != kind && cx.known.of(*other).ids.contains(id))
                    .map(|other| format!("。{} の {id} なら {} に書く", other.label(), other.dir()))
                    .unwrap_or_default();
                cx.push(
                    line,
                    "decision-unknown",
                    format!("{key} の {id} という {} がない{hint}", kind.label()),
                );
                continue;
            }
            if let Some(d) = records
                .decisions
                .iter()
                .find(|d| &d.record.id == id && !d.superseded_by.is_empty())
            {
                let newer = d.superseded_by.join("、");
                cx.warn(
                    line,
                    "decision-superseded",
                    format!("{key} の {id} は打ち消された。新しい決定は {newer}"),
                );
            }
        }
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

/// base は、相対リンクの起点（ルートからの相対パス）。
fn check_links(cx: &mut Checker, outline: &Outline, base: &Path) {
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
        let Some((target, fragment)) = markdown::resolve_link(base, &link.dest) else {
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

    const CONFIG: &str = concat!(
        "docv_version: ",
        env!("CARGO_PKG_VERSION"),
        "
docv_exclude:
  - docv/free/ignored/**
tags:
  - name: 仕様
    color: blue
  - name: 計画
    color: green
lint:
  bold: deny
"
    );

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
                "docv/free/spec/a.md",
                &format!(
                    "{}\n# A\n\n`以前は` と書かない。[README](../../../README.md#x)\n",
                    fm("tags: [仕様]\n")
                ),
            ),
            (
                "docv/free/plan/p.md",
                &format!("{}\n# P\n\n- [ ] t\n", fm("tags: [計画]\n")),
            ),
            ("docv/free/ignored/memo.md", "**太字** [x](missing.md)\n"),
            (
                "docv/slot/service/service.md",
                "---\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# サービス仕様\n",
            ),
        ]);
        assert!(result.is_empty(), "{result:#?}");
    }

    #[test]
    fn reports_frontmatter_problems() {
        let result = check(&[
            ("docv/free/spec/a.md", "# A\n"),
            (
                "docv/free/spec/b.md",
                "---\nmission: |\n  1\n  2\ntags: [仕様, z]\n---\n# B\n",
            ),
            ("docv/free/spec/c.md", "---\nmission: [\n---\n# C\n"),
        ]);
        let without_c: Vec<&str> = result
            .iter()
            .filter(|r| !r.starts_with("docv/free/spec/c.md"))
            .map(String::as_str)
            .collect();
        assert_eq!(
            without_c,
            [
                "docv/free/spec/a.md:1: mission-missing mission がない。このファイルを読むと何が分かるかを 1 文で書く".to_string(),
                "docv/free/spec/b.md:2: mission-multiline mission は 1 行で書く".to_string(),
                "docv/free/spec/b.md:5: tag-unknown タグ z は docv.yaml の tags にない。使うなら、docv.yaml の tags に name と color を書いて足す".to_string(),
            ]
        );
        assert!(
            result
                .iter()
                .any(|r| r.starts_with("docv/free/spec/c.md:1: frontmatter-invalid"))
        );
    }

    /// adr と pdr を書いた Free ファイルと Slot、指される決定。
    const RELATED_FILES: &[(&str, &str)] = &[
        (
            "docv/adr/go.md",
            "---\ndate: 2026-01-01\n---\n\n# Go にする\n",
        ),
        (
            "docv/adr/rust.md",
            "---\ndate: 2026-02-01\nsupersedes: [go]\n---\n\n# Rust にする\n",
        ),
        ("docv/pdr/paid.md", "---\ndate: 2026-01-01\n---\n\n# 有料\n"),
        // 読めない決定も、ないとは言わない
        ("docv/adr/broken.md", "# 壊れた\n"),
        (
            "docv/free/spec/a.md",
            "---\nmission: q\nadr: [rust, go, broken, none]\npdr:\n  - paid\n  - rust\n---\n\n# A\n",
        ),
        // 型の違う adr と pdr は frontmatter-invalid にし、読める id だけを確かめる
        (
            "docv/free/spec/b.md",
            "---\nmission: q\nadr: go\npdr: [paid, 1]\n---\n\n# B\n",
        ),
        (
            "docv/slot/service/manual.md",
            "---\nmission: 利用者の使い方（CLI のコマンド、GUI の操作など）\nadr: [go]\n---\n\n# M\n",
        ),
        (
            "docv/slot/service/service/a.md",
            "---\norder: 1\nmission: 外から見た機能と、それぞれの振る舞いと決まり\npdr: [none]\n---\n\n# A\n",
        ),
    ];

    #[test]
    fn checks_related_decisions_of_free_files_and_slots() {
        assert_eq!(
            check(RELATED_FILES),
            [
                "docv/adr/broken.md:1: adr-invalid YAML front matter がない",
                // 見つからない id は error、打ち消された決定は warning。どちらも id の行を指す
                "docv/free/spec/a.md:3: decision-unknown adr の none という ADR がない",
                "docv/free/spec/a.md:3: warning decision-superseded adr の go は打ち消された。新しい決定は rust",
                // ADR と PDR は別の決定。種類を取り違えていれば、もう一方の項目を案内する
                "docv/free/spec/a.md:6: decision-unknown pdr の rust という PDR がない。ADR の rust なら adr に書く",
                "docv/free/spec/b.md:3: frontmatter-invalid adr は ADR の id（ファイル名から .md を除いたもの）の配列で書く",
                "docv/free/spec/b.md:4: frontmatter-invalid pdr は PDR の id（ファイル名から .md を除いたもの）の配列で書く",
                // Slot と分けたページも、Free ファイルと同じく確かめる
                "docv/slot/service/manual.md:3: warning decision-superseded adr の go は打ち消された。新しい決定は rust",
                "docv/slot/service/service/a.md:4: decision-unknown pdr の none という PDR がない",
            ]
        );
    }

    #[test]
    fn checks_related_decisions_only_in_targets() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        for (path, text) in RELATED_FILES {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        let run_on = |path: &str| run(&project, &[path.to_string()], OPTIONS).unwrap();
        // warning だけなら、error はない（終了コードは 0）
        let manual = run_on("docv/slot/service/manual.md");
        assert_eq!(
            manual.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
            [
                "docv/slot/service/manual.md:3: warning decision-superseded adr の go は打ち消された。新しい決定は rust"
            ]
        );
        assert!(!crate::diagnostic::has_error(&manual));
        let a = run_on("docv/free/spec/a.md");
        assert_eq!(a.len(), 3, "{a:#?}");
        assert!(a.iter().all(|d| d.path == "docv/free/spec/a.md"));
        assert!(crate::diagnostic::has_error(&a));
    }

    #[test]
    fn reports_structure_problems() {
        let result = check(&[
            ("docv/free/spec/a.md", &format!("{}# A\n\n# B\n", fm(""))),
            // 終わったタスクは確かめない
            (
                "docv/free/plan/p.md",
                &format!("{}# P\n\n- [x] done\n", fm("")),
            ),
            // 書式の規則は、lint で deny にしたものと、章番号を確かめる
            (
                "docv/free/spec/b.md",
                &format!("{}# B\n\n## 1. 一\n\n**b** [a](a.md)\n\n## 二\n", fm("")),
            ),
        ]);
        assert_eq!(
            result,
            [
                "docv/free/spec/a.md:6: title-count h1 が 2 個ある。1 つにする",
                "docv/free/spec/b.md:8: style-bold 太字を使わない",
                "docv/free/spec/b.md:10: heading-number h2 の見出しを「2. 」で始める。章番号は 1 から順に振る",
            ]
        );
    }

    #[test]
    fn reports_broken_links_and_slot_adr_and_pdr_problems() {
        let result = check(&[
            (
                "docv/free/spec/a.md",
                &format!(
                    "{}# A\n\n[x](missing.md) [y](https://example.com) [z](#a) [spec/あ.md](%E3%81%82.md) [v](/README.md)\n",
                    fm("")
                ),
            ),
            ("docv/free/spec/あ.md", &format!("{}# あ\n", fm(""))),
            ("README.md", "# R\n"),
            // 残った YAML の Slot は、Markdown に書き直すよう案内する
            ("docv/slot/service/words.yaml", "groups: 1\n"),
            ("docv/adr/a.md", "# A\n"),
            // PDR も ADR と同じく決定として読み、Slot の名前でないファイルとはしない。Markdown には Free ファイルと同じ検査をかける
            (
                "docv/pdr/b.md",
                "---\ndate: 2026-01-01\n---\n\n# B\n\n[x](missing.md)\n",
            ),
        ]);
        assert_eq!(result.len(), 4, "{result:#?}");
        assert!(
            result[0].starts_with("docv/adr/a.md:1: adr-invalid"),
            "{result:#?}"
        );
        assert_eq!(
            result[1],
            "docv/free/spec/a.md:6: link-broken リンク先 missing.md がない"
        );
        assert_eq!(
            result[2],
            "docv/pdr/b.md:7: link-broken リンク先 missing.md がない"
        );
        assert_eq!(
            result[3],
            "docv/slot/service/words.yaml:1: slot-unknown YAML の Slot はなくなった。docv gen words --print の雛形を見て、docv/slot/service/words.md に Markdown で書き直す"
        );
    }

    /// Slot と ADR に、Free ファイルと同じ検査をかける（YAML front matter を除く）。
    const SLOT_FILES: &[(&str, &str)] = &[
        (
            "docv/slot/service/manual.md",
            "---\nmission: 利用者の使い方（CLI のコマンド、GUI の操作など）\n---\n\n# 利用マニュアル\n\n**太字** [切れた](none.md) [見出し](#ない)\n\n# 二つ目\n",
        ),
        (
            "docv/adr/a.md",
            "---\ngroup: g\ndate: 2026-01-01\nsupersedes: [none]\n---\n\n# A\n\n[切れた](none.md)\n",
        ),
        ("docv/free/spec/b.md", "---\nmission: q\n---\n\n# B\n"),
    ];

    const SLOT_RESULTS: &[&str] = &[
        "docv/adr/a.md:4: adr-unknown-ref supersedes の none という ADR がない",
        "docv/adr/a.md:9: link-broken リンク先 none.md がない",
        "docv/slot/service/manual.md:7: link-anchor リンク先の見出し #ない がない",
        "docv/slot/service/manual.md:7: link-broken リンク先 none.md がない",
        "docv/slot/service/manual.md:7: style-bold 太字を使わない",
        "docv/slot/service/manual.md:9: title-count h1 が 2 個ある。1 つにする",
    ];

    #[test]
    fn checks_markdown_in_slots_and_adrs() {
        assert_eq!(check(SLOT_FILES), SLOT_RESULTS);
    }

    #[test]
    fn checks_the_same_with_paths() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        for (path, text) in SLOT_FILES {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        // パスを渡すと、そのファイルだけを、パスを渡さないときと同じ検査で確かめる
        for path in ["docv/slot/service/manual.md", "docv/adr/a.md"] {
            let result: Vec<String> = run(&project, &[path.to_string()], OPTIONS)
                .unwrap()
                .iter()
                .map(|d| d.to_string())
                .collect();
            let expected: Vec<&str> = SLOT_RESULTS
                .iter()
                .copied()
                .filter(|r| r.starts_with(&format!("{path}:")))
                .collect();
            assert_eq!(result, expected, "{path}");
        }
    }

    #[test]
    fn reports_missing_anchors() {
        let result = check(&[
            (
                "docv/free/spec/a.md",
                &format!(
                    "{}# A\n\n## 一\n\n## 一\n\n[x](#一-1) [x](#%E4%B8%80) [x](#%E4%BA%8C) [spec/b.md](b.md#2-二) [spec/b.md](b.md#1-一) [y](/docv.yaml#x)\n",
                    fm("")
                ),
            ),
            (
                "docv/free/spec/b.md",
                &format!("{}# B\n\n## 1. 一\n", fm("")),
            ),
        ]);
        assert_eq!(
            result,
            [
                "docv/free/spec/a.md:10: link-anchor リンク先 docv/free/spec/b.md に見出し #2-二 がない",
                "docv/free/spec/a.md:10: link-anchor リンク先の見出し #%E4%BA%8C がない",
            ]
        );
    }

    #[test]
    fn skips_excluded_and_dot_files_in_slot_dir() {
        let result = check(&[
            (".gitignore", "memo.txt\n"),
            (
                "docv/slot/service/words.md",
                "---\nmission: このプロジェクトの言葉の意味\n---\n\n# 用語集\n",
            ),
            ("docv/slot/service/.DS_Store", ""),
            ("docv/.cache/a.yaml", "a: 1\n"),
            ("docv/slot/service/memo.txt", "メモ\n"),
            ("docv/adr/.draft.md", "# 下書き\n"),
        ]);
        assert!(result.is_empty(), "{result:#?}");
    }

    #[test]
    fn checks_format_of_slots_and_the_final_newline_of_the_config() {
        let dir = tempfile::tempdir().unwrap();
        // docv.yaml は、末尾の改行だけを確かめる
        write(
            dir.path(),
            "docv.yaml",
            concat!(
                "docv_version: ",
                env!("CARGO_PKG_VERSION"),
                "\nformat:\n  enabled: true\n\n"
            ),
        );
        write(
            dir.path(),
            "docv/slot/service/manual.md",
            "---\nmission: 利用者の使い方（CLI のコマンド、GUI の操作など）\n---\n\n# M\n\n| a | b |\n| --- | --- |\n| 種類 | c |\n",
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
            [
                "docv.yaml:4: format-diff 末尾の改行を 1 つにする。docv fix で整形する",
                "docv/slot/service/manual.md:7: format-diff 整形が必要。docv fix で整形する",
            ]
        );
    }

    #[test]
    fn checks_only_targets() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        write(dir.path(), "docv/free/spec/a.md", "# A\n");
        write(dir.path(), "docv/free/spec/b.md", "# B\n");
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        let result = run(&project, &["docv/free/spec/b.md".to_string()], OPTIONS).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.iter().all(|d| d.path == "docv/free/spec/b.md"));
    }
}
