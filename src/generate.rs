//! docv gen。Slot ファイルを決まったパスに作る。docv/slot/service/manual.md の docv gen。
//!
//! パスは docv が Slot ごとに決めていて、選ぶ余地はない。Slot ファイルは、すべて Markdown。
//! 雛形のある Slot（用語集、技術スタック、環境変数、費用、インストール、環境構築、開発コマンド、Doc ガイド）には、雛形を書く。
//! ほかの Slot は、YAML front matter の mission にミッションを書き、中身の形は自由だと伝える。
//! どの Slot も、ファイルの先頭の mission でミッションが分かる。雛形にも書いてある。

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, bail};

use crate::slot::{self, Def};
use crate::workspace::Project;

pub fn find(slot: &str) -> Result<&'static Def> {
    match slot::def(slot) {
        Some(def) => Ok(def),
        None => bail!("{slot} は Slot の名前ではない。docv list で Slot の一覧を出す"),
    }
}

/// Slot ファイルに書く中身。雛形のない Slot は、YAML front matter の mission だけを書く。
fn text(def: &Def) -> String {
    match slot::template(def.id) {
        Some(template) => template.to_string(),
        None => format!("---\nmission: {}\n---\n", def.mission),
    }
}

/// 作ったあとに伝える文。雛形のない Slot は、中身の形は自由だと伝える。雛形のある Slot は、雛形が形を示すので伝えない。
/// ページに分けられる Slot は、分けるときの置き場所を伝える。どちらでもなければ None。
fn note(def: &Def) -> Option<String> {
    let mut note = String::new();
    if slot::template(def.id).is_none() {
        let _ = writeln!(
            note,
            "{}の中身の形は決めていない。ミッション「{}」に沿って、Markdown を自由に書く",
            def.label, def.mission
        );
    }
    if let Some(dir) = def.pages_dir() {
        let _ = writeln!(
            note,
            "ページに分けるときは、このファイルの代わりに、Slot のディレクトリ（document_root の下の slot/）の下の {dir}/ に 1 ページ 1 つの Markdown で置く。書き方は docv desc の「ページ分け」"
        );
    }
    (!note.is_empty()).then_some(note)
}

/// docv gen --print の出力。Slot ファイルを作らずに、書く中身だけを出す。
pub fn print(slot: &str) -> Result<String> {
    Ok(text(find(slot)?))
}

/// 作った Slot ファイル。
#[derive(Debug)]
pub struct Generated {
    /// ルートからの相対パス。
    pub path: String,
    /// 作ったあとに伝える文。中身の形が自由なことや、ページに分けるときの置き場所。
    pub note: Option<String>,
}

/// Slot ファイルを作る。すでにあれば、force のときだけ書き直す。
/// ページに分けた Slot のページがあれば、force でも作らない。分けたページと 1 つのファイルの両方を置かないため。
pub fn run(project: &Project, slot: &str, force: bool) -> Result<Generated> {
    let def = find(slot)?;
    let rel = format!("{}/{}", project.slot_dir(), def.file_name());
    if let Some(dir) = def.pages_dir() {
        let prefix = format!("{}/", project.slot_dir());
        let paged = project.slot_dir_files()?.iter().any(|path| {
            path.strip_prefix(&prefix)
                .and_then(slot::def_of_page)
                .is_some_and(|d| d.id == def.id)
        });
        if paged {
            bail!(
                "{}/{dir}/ にページがある。ページに分けているときは、{rel} を作らない",
                project.slot_dir()
            );
        }
    }
    let path = project.root.join(&rel);
    if path.exists() && !force {
        bail!("{rel} がすでにある。書き直すなら --force を付ける");
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("{} を作れない", parent.display()))?;
    }
    fs::write(&path, text(def)).with_context(|| format!("{rel} を書けない"))?;
    Ok(Generated {
        path: rel,
        note: note(def),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::slot::DEFS;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", "{}\n");
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    fn read(dir: &tempfile::TempDir, path: &str) -> String {
        fs::read_to_string(dir.path().join(path)).unwrap()
    }

    /// 雛形のある Slot と、その雛形。
    const TEMPLATES: [(&str, &str, &str); 8] = [
        (
            "install",
            "docv/slot/service/install.md",
            include_str!("../assets/template/install.md"),
        ),
        (
            "words",
            "docv/slot/service/words.md",
            include_str!("../assets/template/words.md"),
        ),
        (
            "stack",
            "docv/slot/system/stack.md",
            include_str!("../assets/template/stack.md"),
        ),
        (
            "setup",
            "docv/slot/guide/setup.md",
            include_str!("../assets/template/setup.md"),
        ),
        (
            "develop",
            "docv/slot/guide/develop.md",
            include_str!("../assets/template/develop.md"),
        ),
        (
            "env",
            "docv/slot/guide/env.md",
            include_str!("../assets/template/env.md"),
        ),
        (
            "doc-guide",
            "docv/slot/guide/doc-guide.md",
            include_str!("../assets/template/doc-guide.md"),
        ),
        (
            "cost",
            "docv/slot/operation/cost.md",
            include_str!("../assets/template/cost.md"),
        ),
    ];

    #[test]
    fn creates_slots_at_fixed_paths() {
        let (dir, project) = project();
        // 雛形のない Slot は、YAML front matter の mission だけを書いたファイルを作り、中身の形は自由だと伝える
        let api = run(&project, "api", false).unwrap();
        assert_eq!(api.path, "docv/slot/system/api.md");
        assert_eq!(
            read(&dir, &api.path),
            "---\nmission: API の一覧と、それぞれが受け取るものと返すもの\n---\n"
        );
        assert_eq!(
            api.note.as_deref(),
            Some(
                "API 仕様書の中身の形は決めていない。ミッション「API の一覧と、それぞれが受け取るものと返すもの」に沿って、Markdown を自由に書く\n"
            )
        );
    }

    #[test]
    fn writes_each_template() {
        // 雛形のある Slot は、雛形をそのまま書く。形は雛形が示すので、中身の形は自由だとは伝えない
        let (dir, project) = project();
        for (id, path, template) in TEMPLATES {
            let generated = run(&project, id, false).unwrap();
            assert_eq!(generated.path, path);
            assert_eq!(read(&dir, path), template, "{id}");
            assert!(
                !generated
                    .note
                    .unwrap_or_default()
                    .contains("中身の形は決めていない"),
                "{id}"
            );
            assert_eq!(print(id).unwrap(), template, "{id}");
        }
        // 雛形があるのは、これらの Slot だけ
        let with_template: Vec<&str> = DEFS
            .iter()
            .filter(|d| slot::template(d.id).is_some())
            .map(|d| d.id)
            .collect();
        let mut expected: Vec<&str> = TEMPLATES.iter().map(|(id, ..)| *id).collect();
        expected.sort_by_key(|id| DEFS.iter().position(|d| d.id == *id));
        assert_eq!(with_template, expected);
    }

    #[test]
    fn rewrites_only_with_force() {
        let (dir, project) = project();
        write(dir.path(), "docv/slot/service/words.md", "# 用語集\n");
        let error = run(&project, "words", false).unwrap_err().to_string();
        assert!(
            error.contains("docv/slot/service/words.md がすでにある"),
            "{error}"
        );
        assert_eq!(read(&dir, "docv/slot/service/words.md"), "# 用語集\n");
        run(&project, "words", true).unwrap();
        assert_eq!(
            read(&dir, "docv/slot/service/words.md"),
            include_str!("../assets/template/words.md")
        );
    }

    #[test]
    fn does_not_create_the_whole_file_of_a_split_slot() {
        let (dir, paged) = project();
        write(
            dir.path(),
            "docv/slot/service/service/user.md",
            "---\norder: 1\n---\n\n# 利用者向け機能\n",
        );
        let error = run(&paged, "service", true).unwrap_err().to_string();
        assert!(
            error.contains("docv/slot/service/service/ にページがある"),
            "{error}"
        );
        assert!(!dir.path().join("docv/slot/service/service.md").exists());
        // 分けていなければ、1 つのファイルを作り、分けるときの置き場所を伝える
        let (_dir, whole) = project();
        let service = run(&whole, "service", false).unwrap();
        assert_eq!(service.path, "docv/slot/service/service.md");
        assert!(
            service
                .note
                .unwrap()
                .contains("Slot のディレクトリ（document_root の下の slot/）の下の service/service/ に 1 ページ 1 つの Markdown で置く")
        );
        // 雛形のある開発コマンドも、分けられるので置き場所を伝える。ページも Markdown
        let develop = run(&whole, "develop", false).unwrap();
        assert_eq!(
            develop.note.as_deref(),
            Some(
                "ページに分けるときは、このファイルの代わりに、Slot のディレクトリ（document_root の下の slot/）の下の guide/develop/ に 1 ページ 1 つの Markdown で置く。書き方は docv desc の「ページ分け」\n"
            )
        );
        write(
            dir.path(),
            "docv/slot/guide/develop/web.md",
            "---\norder: 1\n---\n\n# Web\n",
        );
        let error = run(&paged, "develop", true).unwrap_err().to_string();
        assert!(
            error.contains("docv/slot/guide/develop/ にページがある"),
            "{error}"
        );
    }

    #[test]
    fn prints_without_writing() {
        assert_eq!(
            print("manual").unwrap(),
            "---\nmission: 利用者の使い方（CLI のコマンド、GUI の操作など）\n---\n"
        );
        for name in ["api-md", "docv/slot/service/words.md"] {
            let error = print(name).unwrap_err().to_string();
            assert!(error.contains("Slot の名前ではない"), "{name}: {error}");
        }
    }

    #[test]
    fn every_slot_file_starts_with_its_mission() {
        // 雛形も、雛形のない Slot のファイルも、YAML front matter の mission から始まる
        for def in DEFS {
            assert!(
                text(def).starts_with(&format!("---\nmission: {}\n---\n", def.mission)),
                "{}",
                def.id
            );
        }
    }
}
