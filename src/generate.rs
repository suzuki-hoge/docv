//! docv slot list と docv slot gen。Slot の一覧を出し、Slot ファイルを決まったパスに作る。docv/service/manual.md の docv slot。
//!
//! パスは docv が Slot ごとに決めていて、選ぶ余地はない。yaml の Slot には項目のサンプル（テンプレ）を書く。
//! md の Slot は空のファイルを作り、中身の形は自由だと伝える。Doc ガイドの Slot（doc-guide）だけは、ガイドラインの雛形を書く。

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
fn text(def: &Def) -> String {
    match def.format {
        Format::Yaml => format!(
            "{}{}",
            schema::header(&format!("{}.schema.json", def.id)),
            slot::template(def.id).expect("yaml の Slot にはテンプレがある")
        ),
        Format::Markdown => slot::md_template(def.id).unwrap_or("").to_string(),
    }
}

/// 雛形のない md の Slot で、中身の形は自由だと伝える文。
fn md_note(def: &Def) -> Option<String> {
    if def.format != Format::Markdown || slot::md_template(def.id).is_some() {
        return None;
    }
    Some(format!(
        "{}は md の Slot で、中身の形は決めていない。ミッション「{}」に沿って、Markdown を自由に書く\n",
        def.label, def.mission
    ))
}

/// docv slot gen --print の出力。Slot ファイルを作らずに、書く中身を出す。雛形のない md の Slot は、中身の形は自由だと伝える。
pub fn print(slot: &str) -> Result<String> {
    let def = find(slot)?;
    Ok(md_note(def).unwrap_or_else(|| text(def)))
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
pub fn run(project: &Project, slot: &str, force: bool) -> Result<Generated> {
    let def = find(slot)?;
    let rel = format!("{}/{}", project.slot_dir(), def.file_name());
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
        // md の Slot は空のファイルを作り、中身の形は自由だと伝える
        let api = run(&project, "api", false).unwrap();
        assert_eq!(api.path, "docv/system/api.md");
        assert_eq!(read(&dir, &api.path), "");
        assert!(api.note.unwrap().contains("API の一覧と"));
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
    fn prints_without_writing() {
        assert_eq!(
            print("doc-guide").unwrap(),
            include_str!("../assets/template/doc-guide.md")
        );
        assert_eq!(print("words").unwrap(), words_text());
        assert!(print("manual").unwrap().contains("中身の形は決めていない"));
        for name in ["api-md", "docv/service/words.yaml"] {
            let error = print(name).unwrap_err().to_string();
            assert!(error.contains("Slot の名前ではない"), "{name}: {error}");
        }
    }

    #[test]
    fn lists_every_slot_with_its_mission() {
        let list = list();
        assert!(list.starts_with("install "), "{list}");
        assert!(
            list.contains("service/manual.md            利用マニュアル: 利用者の使い方"),
            "{list}"
        );
        assert_eq!(list.lines().count(), DEFS.len());
    }
}
