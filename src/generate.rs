//! docv slot list と docv slot gen。Slot の一覧を出し、Slot ファイルを決まったパスに作る。docv/service/manual.md の docv slot。
//!
//! パスは docv が Slot ごとに決めていて、選ぶ余地はない。yaml の Slot には項目のサンプル（テンプレ）を書く。
//! md の Slot は、YAML front matter の mission にミッションを書き、中身の形は自由だと伝える。Doc ガイドの Slot（doc-guide）だけは、ガイドラインの雛形を書く。
//! どの Slot も、ファイルの先頭でミッションが分かる。yaml の Slot はコメントで、md の Slot は check が確かめる mission で。yaml と Doc ガイドの雛形は、雛形の中に書いてある。

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, bail};

use crate::schema;
use crate::slot::{self, DEFS, Def, Format};
use crate::workspace::Project;

/// docv slot list の出力。名前、作るファイル（dir.slot からの相対パス）、Slot の名前とミッションを 1 行ずつ並べる。
pub fn list() -> String {
    let name_width = DEFS.iter().map(|d| d.id.len()).max().unwrap_or(0);
    let file_width = DEFS.iter().map(|d| d.file_name().len()).max().unwrap_or(0);
    let mut out = String::new();
    for def in DEFS {
        let _ = writeln!(
            out,
            "{:<name_width$}  {:<file_width$}  {}: {}",
            def.id,
            def.file_name(),
            def.label,
            def.mission
        );
    }
    out
}

pub fn find(slot: &str) -> Result<&'static Def> {
    match slot::def(slot) {
        Some(def) => Ok(def),
        None => bail!("{slot} は Slot の名前ではない。docv slot list で Slot の一覧を出す"),
    }
}

/// Slot ファイルに書く中身。yaml の Slot は、1 行目でスキーマの URL をエディタに伝える。
/// 雛形のない md の Slot は、YAML front matter の mission だけを書く。
fn text(def: &Def) -> String {
    match def.format {
        Format::Yaml => format!(
            "{}{}",
            schema::header(&format!("{}.schema.json", def.id)),
            slot::template(def.id).expect("yaml の Slot にはテンプレがある")
        ),
        Format::Markdown => match slot::md_template(def.id) {
            Some(template) => template.to_string(),
            None => format!("---\nmission: {}\n---\n", def.mission),
        },
    }
}

/// 雛形のない md の Slot で、中身の形は自由だと伝える文。ページに分けられる Slot は、分けるときの置き場所も伝える。
fn md_note(def: &Def) -> Option<String> {
    if def.format != Format::Markdown || slot::md_template(def.id).is_some() {
        return None;
    }
    let mut note = format!(
        "{}は md の Slot で、中身の形は決めていない。ミッション「{}」に沿って、Markdown を自由に書く\n",
        def.label, def.mission
    );
    if let Some(dir) = def.pages_dir() {
        let _ = writeln!(
            note,
            "ページに分けるときは、このファイルの代わりに、dir.slot の下の {dir}/ に 1 ページ 1 つの Markdown で置く。書き方は docv desc の「ページ分け」"
        );
    }
    Some(note)
}

/// docv slot gen --print の出力。Slot ファイルを作らずに、書く中身だけを出す。
pub fn print(slot: &str) -> Result<String> {
    Ok(text(find(slot)?))
}

/// 作った Slot ファイル。
#[derive(Debug)]
pub struct Generated {
    /// ルートからの相対パス。
    pub path: String,
    /// 雛形のない md の Slot で、中身の形は自由だと伝える文。
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
        note: md_note(def),
    })
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

    fn project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", "{}\n");
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    fn read(dir: &tempfile::TempDir, path: &str) -> String {
        fs::read_to_string(dir.path().join(path)).unwrap()
    }

    /// 1 行目でスキーマの URL を伝える、用語集の Slot。
    fn words_text() -> String {
        format!(
            "# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/assets/schema/words.schema.json\n{}",
            env!("CARGO_PKG_VERSION"),
            include_str!("../assets/template/words.yaml")
        )
    }

    #[test]
    fn creates_slots_at_fixed_paths() {
        let (dir, project) = project();
        let words = run(&project, "words", false).unwrap();
        assert_eq!(words.path, "docv/service/words.yaml");
        assert!(words.note.is_none());
        assert_eq!(read(&dir, &words.path), words_text());
        // md の Slot は、YAML front matter の mission だけを書いたファイルを作り、中身の形は自由だと伝える
        let api = run(&project, "api", false).unwrap();
        assert_eq!(api.path, "docv/system/api.md");
        assert_eq!(
            read(&dir, &api.path),
            "---\nmission: API の一覧と、それぞれが受け取るものと返すもの\n---\n"
        );
        assert!(api.note.unwrap().contains("中身の形は決めていない"));
        // Doc ガイドの Slot は、ガイドラインの雛形を書く
        let guide = run(&project, "doc-guide", false).unwrap();
        assert_eq!(guide.path, "docv/guide/doc-guide.md");
        assert!(guide.note.is_none());
        assert_eq!(
            read(&dir, &guide.path),
            include_str!("../assets/template/doc-guide.md")
        );
    }

    #[test]
    fn rewrites_only_with_force() {
        let (dir, project) = project();
        write(dir.path(), "docv/service/words.yaml", "groups: []\n");
        let error = run(&project, "words", false).unwrap_err().to_string();
        assert!(
            error.contains("docv/service/words.yaml がすでにある"),
            "{error}"
        );
        assert_eq!(read(&dir, "docv/service/words.yaml"), "groups: []\n");
        run(&project, "words", true).unwrap();
        assert_eq!(read(&dir, "docv/service/words.yaml"), words_text());
    }

    #[test]
    fn does_not_create_the_whole_file_of_a_split_slot() {
        let (dir, paged) = project();
        write(
            dir.path(),
            "docv/service/service/user.md",
            "---\norder: 1\n---\n\n# 利用者向け機能\n",
        );
        let error = run(&paged, "service", true).unwrap_err().to_string();
        assert!(
            error.contains("docv/service/service/ にページがある"),
            "{error}"
        );
        assert!(!dir.path().join("docv/service/service.md").exists());
        // 分けていなければ、1 つのファイルを作り、分けるときの置き場所を伝える
        let (_dir, whole) = project();
        let service = run(&whole, "service", false).unwrap();
        assert_eq!(service.path, "docv/service/service.md");
        assert!(
            service
                .note
                .unwrap()
                .contains("dir.slot の下の service/service/ に 1 ページ 1 つの Markdown で置く")
        );
    }

    #[test]
    fn prints_without_writing() {
        assert_eq!(
            print("doc-guide").unwrap(),
            include_str!("../assets/template/doc-guide.md")
        );
        assert_eq!(print("words").unwrap(), words_text());
        assert_eq!(
            print("manual").unwrap(),
            "---\nmission: 利用者の使い方（CLI のコマンド、GUI の操作など）\n---\n"
        );
        for name in ["api-md", "docv/service/words.yaml"] {
            let error = print(name).unwrap_err().to_string();
            assert!(error.contains("Slot の名前ではない"), "{name}: {error}");
        }
    }

    #[test]
    fn every_slot_file_starts_with_its_mission() {
        // yaml の Slot はスキーマの URL の次の行に Slot の名前とミッションを、md の Slot は YAML front matter の mission を書く
        for def in DEFS {
            let text = text(def);
            match def.format {
                Format::Yaml => assert_eq!(
                    text.lines().nth(1),
                    Some(format!("# {}: {}", def.label, def.mission).as_str()),
                    "{}",
                    def.id
                ),
                Format::Markdown => assert!(
                    text.starts_with(&format!("---\nmission: {}\n---\n", def.mission)),
                    "{}",
                    def.id
                ),
            }
        }
    }

    #[test]
    fn lists_every_slot_with_its_mission() {
        let list = list();
        assert!(list.starts_with("overview "), "{list}");
        assert!(
            list.contains("service/manual.md            利用マニュアル: 利用者の使い方"),
            "{list}"
        );
        assert_eq!(list.lines().count(), DEFS.len());
    }
}
