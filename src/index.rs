//! docv build が dir.index に書くインデックス（docv-index.md）。docv/service/service/html.md の「インデックス」。
//!
//! GitHub でリポジトリを開いた人が、clone しなくても Slot ファイルを読めるように、
//! HTML のプロジェクト概要と同じ順に、元のファイルへのリンクを並べる。
//! GitHub はシンボリックリンクの先を表示しないので、エイリアスやコピーは置かず、Markdown のリンクで辿らせる。

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::build;
use crate::markdown::relative_path;
use crate::slot::{Content, DEFS, Group, Slot};
use crate::workspace::Project;

pub const FILE_NAME: &str = "docv-index.md";

/// インデックスの 1 行目。docv が作ったことを示す。これで始まらない同じ名前のファイルは書き換えない。
const MARKER: &str = "<!-- docv build が作るファイル。手で書き換えず、docv build で作り直す -->";

/// インデックスのパス。dir.index を書いていなければ None。
pub fn path(project: &Project) -> Option<PathBuf> {
    let dir = project.config.index_dir()?;
    let dir = match dir.as_str() {
        "" | "." => project.root.clone(),
        dir => project.root.join(dir),
    };
    Some(dir.join(FILE_NAME))
}

/// path に書いてよいか。docv が作った印のない、同じ名前のファイルがあれば error。
/// 手で書いたファイルを消さないため。build は、出力先に何か書く前にこれを確かめる。
pub fn ensure_writable(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    if text.lines().next() != Some(MARKER) {
        bail!(
            "{} は docv が作ったファイルではないので書けない",
            path.display()
        );
    }
    Ok(())
}

/// インデックスの中身。index は書く先のパスで、リンクはそのディレクトリからの相対パスにする。
/// Slot は、HTML のプロジェクト概要と同じく、まとまりごとに DEFS の順で並べる。置いていない Slot は出さない。
/// md の Slot は元のファイルへ、YAML の Slot は dir.image を書いていれば JPG へリンクし、書いていなければ出さない。
/// JPG を出すかは、dir.image を書いたかで決める。--no-image で build しても、同じインデックスになる。
/// Free ファイルと ADR は、置いているディレクトリと、HTML で見るときのページを示す。
pub fn render(
    project: &Project,
    index: &Path,
    slots: &[Slot],
    has_free: bool,
    has_adr: bool,
) -> String {
    let from = index.parent().unwrap_or(&project.root);
    let image_dir = project.image_dir();
    let link = |label: &str, target: &Path| {
        format!(
            "[{}](<{}>)",
            escape_label(label),
            relative_path(from, target)
                .replace('<', "%3C")
                .replace('>', "%3E")
        )
    };

    let mut out = format!(
        "{MARKER}\n\n# {}\n\n本ページは、プロジェクトをローカルに取得しなくても、GitHub などのホスティングサービス上で docv のビジュアライズページとほぼ同じ内容を閲覧するためのもの。\n",
        project.title()
    );
    for group in Group::ALL {
        let mut lines = Vec::new();
        for def in DEFS.iter().filter(|d| d.group == group) {
            let members: Vec<&Slot> = slots.iter().filter(|s| s.def.id == def.id).collect();
            let Some(first) = members.first() else {
                continue;
            };
            if first.page.is_some() {
                // 分けた Slot は、Slot の名前の下に、ページを order の順に入れ子で並べる。
                lines.push(format!("- {}", def.label));
                for slot in &members {
                    let title = slot.page.as_ref().map_or(def.label, |p| p.title.as_str());
                    lines.push(format!("  - {}", link(title, &source(project, slot))));
                }
            } else if matches!(first.content, Content::Markdown(_)) {
                lines.push(format!("- {}", link(def.label, &source(project, first))));
            } else if let Some(image_dir) = &image_dir {
                lines.push(format!(
                    "- {}",
                    link(def.label, &image_dir.join(build::jpg_path(first)))
                ));
            }
        }
        if !lines.is_empty() {
            let _ = write!(out, "\n## {}\n\n{}\n", group.label(), lines.join("\n"));
        }
    }
    // Free ファイルと ADR は、ディレクトリへのリンクと、HTML で見るときのページを案内する。
    let html = |page: &str| relative_path(from, &project.out_dir().join(page));
    if has_free {
        let dir = project.config.free_dir();
        let _ = write!(
            out,
            "\n## Free ファイル\n\n- {} ディレクトリを参照\n\n検索やタグでの絞り込みをしたい場合は、リポジトリを取得し `docv open` するもしくは直接 `{}` を開いてください。\n",
            link(&dir, &project.root.join(&dir)),
            html(build::DETAIL_PAGE)
        );
    }
    if has_adr {
        let dir = format!("{}/{}", project.slot_dir(), crate::adr::DIR);
        let _ = write!(
            out,
            "\n## ADR\n\n- {} ディレクトリを参照\n\nADR の系譜をたどりたい場合は、リポジトリを取得し `docv open` するもしくは直接 `{}` を開いてください。\n",
            link(&dir, &project.root.join(&dir)),
            html(build::ADR_PAGE)
        );
    }
    out
}

/// Slot ファイルの、リンクに使うパス。シンボリックリンクなら、ルートの中にある先のファイルにする。
/// GitHub は、シンボリックリンクを開いても先の中身を表示しないため。
fn source(project: &Project, slot: &Slot) -> PathBuf {
    let path = project.root.join(&slot.path);
    match fs::canonicalize(&path) {
        Ok(real) if real.starts_with(&project.root) => real,
        _ => path,
    }
}

/// リンクの文字の、Markdown として意味を持つ角括弧とバックスラッシュを escape する。
fn escape_label(label: &str) -> String {
    let mut out = String::new();
    for c in label.chars() {
        if matches!(c, '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
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

    const FILES: &[(&str, &str)] = &[
        ("docv/service/overview.md", "# サービス概要\n"),
        (
            "docv/service/service/b.md",
            "---\norder: 2\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 管理 [β]\n",
        ),
        (
            "docv/service/service/a.md",
            "---\norder: 1\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 概要\n",
        ),
        (
            "docv/service/install.yaml",
            "steps:\n  - id: cargo\n    name: cargo\n    items:\n      body_markdown: \"`cargo install`\"\n",
        ),
        ("docv/guide/doc-guide.md", "# Doc ガイド\n"),
        (
            "docv/adr/go.md",
            "---\ndate: 2026-01-01\n---\n\n# Go にする\n",
        ),
        ("docs/a.md", "---\nmission: q\n---\n# A\n"),
    ];

    #[test]
    fn lists_slots_in_html_order_with_jpg_when_image_is_set() {
        let (dir, project) = project(
            "title: demo\ndir:\n  html: out/html\n  image: out/image\n  index: .\n",
            FILES,
        );
        // JPG を作らなくても（--no-image）、dir.image を書いていれば JPG へリンクする
        build::run(&project, &dir.path().join("out/html"), None).unwrap();
        let text = fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        assert_eq!(
            text,
            "\
<!-- docv build が作るファイル。手で書き換えず、docv build で作り直す -->

# demo

本ページは、プロジェクトをローカルに取得しなくても、GitHub などのホスティングサービス上で docv のビジュアライズページとほぼ同じ内容を閲覧するためのもの。

## サービス

- [サービス概要](<docv/service/overview.md>)
- サービス仕様
  - [概要](<docv/service/service/a.md>)
  - [管理 \\[β\\]](<docv/service/service/b.md>)
- [インストール](<out/image/サービス/インストール.jpg>)

## 開発者向けガイド

- [Doc ガイド](<docv/guide/doc-guide.md>)

## Free ファイル

- [docs](<docs>) ディレクトリを参照

検索やタグでの絞り込みをしたい場合は、リポジトリを取得し `docv open` するもしくは直接 `out/html/プロジェクト詳細.html` を開いてください。

## ADR

- [docv/adr](<docv/adr>) ディレクトリを参照

ADR の系譜をたどりたい場合は、リポジトリを取得し `docv open` するもしくは直接 `out/html/ADR.html` を開いてください。
"
        );
    }

    #[test]
    fn lists_only_md_slots_without_image() {
        let (dir, project) = project("dir:\n  index: .\n", &FILES[..5]);
        build::run(&project, &dir.path().join("out"), None).unwrap();
        let text = fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        assert!(!text.contains("インストール"), "{text}");
        assert!(
            text.contains("- [Doc ガイド](<docv/guide/doc-guide.md>)\n"),
            "{text}"
        );
        // Free ファイルも ADR もなければ、その章を出さない
        assert!(!text.contains("## Free ファイル"), "{text}");
        assert!(!text.contains("## ADR"), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn links_real_file_of_symlinked_slot() {
        let (dir, project) = project(
            "dir:\n  index: .\n",
            &[(
                "assets/desc.md",
                "---\norder: 1\nmission: 外から見た機能と、それぞれの振る舞いと決まり\n---\n\n# 概要\n",
            )],
        );
        fs::create_dir_all(dir.path().join("docv/service/service")).unwrap();
        std::os::unix::fs::symlink(
            "../../../assets/desc.md",
            dir.path().join("docv/service/service/desc.md"),
        )
        .unwrap();
        build::run(&project, &dir.path().join("out"), None).unwrap();
        let text = fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        assert!(text.contains("  - [概要](<assets/desc.md>)\n"), "{text}");
    }

    #[test]
    fn writes_nothing_without_index_or_over_foreign_file() {
        let (dir, off) = project("{}\n", FILES);
        build::run(&off, &dir.path().join("out"), None).unwrap();
        assert!(!dir.path().join(FILE_NAME).exists());

        // docv が作った印のない同じ名前のファイルがあれば、何も書かずに error で終わる
        let (dir, project) = project("dir:\n  index: .\n", FILES);
        fs::write(dir.path().join(FILE_NAME), "# 手で書いた\n").unwrap();
        let error = build::run(&project, &dir.path().join("out"), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("docv が作ったファイルではない"), "{error}");
        assert_eq!(
            fs::read_to_string(dir.path().join(FILE_NAME)).unwrap(),
            "# 手で書いた\n"
        );
        assert!(!dir.path().join("out").exists());
    }

    #[test]
    fn escapes_brackets_in_label() {
        assert_eq!(escape_label("a [b] \\c"), "a \\[b\\] \\\\c");
    }
}
