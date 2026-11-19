//! docv review-guide。ドキュメントをレビューする手順と、ADR と PDR の一覧を出す。docv/slot/service/manual.md の docv review-guide。
//!
//! docv は指摘を出さない。一覧には、読んでも分からないことか、AI Agent が自分で集めると間違えやすいことだけを出す。
//! 打ち消されていない決定と、docv_exclude である。レビューする Slot と Free ファイルは、手順の中で docv list を実行させる。
//! 同じ一覧を、2 つのコマンドで作らないため。
//! 見出しと行数は出さない。対象はすべて読ませ、大きさでは判断させないため（docv/pdr/review-by-reader.md）。

use anyhow::Result;

use crate::decision::{self, Kind};
use crate::workspace::Project;

/// レビューの手順。末尾は一覧の見出しで終わり、その後にファイルの一覧を続ける。
/// バイナリに埋め込むので、CLI の版と内容が一致する。
const SOURCE: &str = include_str!("../assets/review-guide.md");

/// 一覧の見出し。手順の末尾にあり、CLI はこの後にファイルの一覧を続ける。
const INVENTORY: &str = "\n## 一覧\n";

/// HTML の Review Guide のページで、一覧の代わりに書く 1 行。
const INVENTORY_NOTE: &str =
    "一覧は、`docv review-guide` を実行したときに、その時点のファイルから作る。\n";

/// レビューの手順と、ADR と PDR の一覧。
pub fn text(project: &Project) -> Result<String> {
    let mut sections = Vec::new();
    for kind in Kind::ALL {
        sections.push(decisions(project, kind)?);
    }
    sections.push(exclude(project));
    Ok(format!("{SOURCE}\n{}", sections.join("\n")))
}

/// HTML の Review Guide のページに出す、レビューの手順。一覧は出さない。
/// build した時点の一覧になってずれ、HTML を git で管理していると ADR や PDR を足すたびに差分になるため（docv/pdr/review-by-reader.md）。
pub fn page() -> String {
    let guide = SOURCE
        .rsplit_once(INVENTORY)
        .map_or(SOURCE, |(guide, _)| guide);
    format!("{guide}{INVENTORY}\n{INVENTORY_NOTE}")
}

/// 見出しと、その下の行。行がなければ「なし」と書く。
fn section(title: &str, lines: &[String]) -> String {
    let body = if lines.is_empty() {
        "なし\n".to_string()
    } else {
        lines.concat()
    };
    format!("### {title}\n\n{body}")
}

/// 打ち消されていない ADR か PDR を、日付と id の順に並べる。
/// レビューの対象ではない。Slot や Free ファイルにある経緯が決定にも書いてあるかと、今の決定と食い違っていないかを確かめるために読ませる。
/// 打ち消された決定は履歴で、今の姿ではないので出さない。今の決定として読まれると、誤った食い違いを指摘させてしまう。
fn decisions(project: &Project, kind: Kind) -> Result<String> {
    let lines: Vec<String> = decision::load(project, kind)?
        .iter()
        .filter(|d| d.superseded_by.is_empty())
        .map(|d| format!("- `{}`: {}\n", d.path, d.record.title))
        .collect();
    Ok(section(kind.label(), &lines))
}

/// docv_exclude の glob。解決した予定やメモが残っていないかを、AI Agent が確かめる場所。
fn exclude(project: &Project) -> String {
    let lines: Vec<String> = project
        .config
        .docv_exclude
        .iter()
        .map(|glob| format!("- `{glob}`\n"))
        .collect();
    section("docv_exclude", &lines)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::workspace::Locate;

    fn project(config: &str, files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let write = |path: &str, text: &str| {
            let path = dir.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        write("docv.yaml", config);
        for (path, text) in files {
            write(path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    /// 出力のうち、手順の後に続ける一覧。
    fn inventory(project: &Project) -> String {
        let all = text(project).unwrap();
        all.strip_prefix(SOURCE)
            .unwrap_or_else(|| panic!("手順から始まる: {all}"))
            .to_string()
    }

    #[test]
    fn the_guide_ends_with_the_heading_of_the_inventory() {
        // 一覧は、手順の末尾の見出しの下に続ける
        assert!(SOURCE.starts_with("# レビューガイド\n"), "{SOURCE}");
        assert!(SOURCE.contains(INVENTORY), "{SOURCE}");
        let (_, last) = SOURCE.rsplit_once("\n## ").unwrap();
        assert!(last.starts_with("一覧\n"), "{last}");
    }

    #[test]
    fn the_page_leaves_out_the_inventory() {
        // HTML のページは、一覧の見出しの下に、一覧を作るタイミングだけを書く
        let page = page();
        assert!(page.starts_with("# レビューガイド\n"), "{page}");
        assert!(
            page.ends_with(
                "\n## 一覧\n\n一覧は、`docv review-guide` を実行したときに、その時点のファイルから作る。\n"
            ),
            "{page}"
        );
    }

    #[test]
    fn lists_what_reading_the_files_does_not_tell() {
        let (_dir, project) = project(
            "docv_exclude:\n  - docv/free/tmp/**\n",
            &[
                (
                    "docv/slot/service/overview.md",
                    "---\nmission: 利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明\n---\n\n# サービス概要\n\n## 解決すること\n",
                ),
                (
                    "docv/slot/service/service/b.md",
                    "---\norder: 2\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 管理\n",
                ),
                (
                    "docv/slot/service/service/a.md",
                    "---\norder: 1\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 利用者向け\n\n## 注文\n",
                ),
                (
                    "docv/slot/service/install.yaml",
                    "steps:\n  - id: cargo\n    name: cargo\n    items:\n      body_markdown: |\n        cargo install\n",
                ),
                (
                    "docv/free/order.md",
                    "---\nmission: 注文を確定するタイミング\ntags: [ 仕様, 注文 ]\n---\n\n# 注文\n\n## 確定\n",
                ),
                ("docv/free/note.md", "# メモ\n"),
                ("docv/free/tmp/draft.md", "---\nmission: 下書き\n---\n"),
                (
                    "docv/adr/rust.md",
                    "---\ngroup: 言語\ndate: 2026-03-01\nsupersedes: [ go ]\n---\n\n# Rust で書く\n",
                ),
                (
                    "docv/adr/go.md",
                    "---\ndate: 2026-01-01\n---\n\n# Go で書く\n",
                ),
                (
                    "docv/adr/cli.md",
                    "---\ndate: 2026-02-01\n---\n\n# CLI にする\n",
                ),
            ],
        );
        // Slot と Free ファイルは出さない（docv list で出す）。打ち消された決定（go）も出さない
        assert_eq!(
            inventory(&project),
            "\n### ADR\n\n\
             - `docv/adr/cli.md`: CLI にする\n\
             - `docv/adr/rust.md`: Rust で書く\n\
             \n### PDR\n\nなし\n\
             \n### docv_exclude\n\n\
             - `docv/free/tmp/**`\n"
        );
    }

    #[test]
    fn writes_none_for_an_empty_project() {
        let (_dir, project) = project("title: demo\n", &[]);
        assert_eq!(
            inventory(&project),
            "\n### ADR\n\nなし\n\n### PDR\n\nなし\n\n### docv_exclude\n\nなし\n"
        );
    }
}
