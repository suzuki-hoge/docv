//! docv add。Slot ファイルを、決まったパスに作る。docv/service/manual.md の docv add。
//!
//! パスは docv が Slot ごとに決めていて、選ぶ余地はない。yaml の Slot には項目のサンプル（テンプレ）を書き、md の Slot は空のファイルを作る。
//! Doc ガイドの Slot（doc-guide）だけは、ガイドラインの雛形を書く。

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, bail};

use crate::slot::{self, DEFS, Def};
use crate::workspace::Project;

/// 作れる Slot の名前と、Slot と拡張子の組を、Slot の一覧の順に返す。
pub fn pages() -> Vec<(String, &'static Def, &'static str)> {
    DEFS.iter()
        .map(|def| (def.id.to_string(), def, def.extension()))
        .collect()
}

/// docv add --list の出力。名前、作るファイル（slots.dir からの相対パス）、Slot の名前を 1 行ずつ並べる。
pub fn list() -> String {
    let pages = pages();
    let rows: Vec<(String, String, &str)> = pages
        .iter()
        .map(|(page, def, ext)| {
            (
                page.clone(),
                format!("{}/{}.{ext}", def.group.dir(), def.id),
                def.label,
            )
        })
        .collect();
    let page_width = rows.iter().map(|r| r.0.len()).max().unwrap_or(0);
    let file_width = rows.iter().map(|r| r.1.len()).max().unwrap_or(0);
    let mut out = String::new();
    for (page, file, label) in rows {
        let _ = writeln!(out, "{page:<page_width$}  {file:<file_width$}  {label}");
    }
    out
}

/// Slot ファイルに書く中身。
fn text(def: &Def, ext: &str) -> &'static str {
    match ext {
        "yaml" => slot::template(def.id).expect("yaml の Slot にはテンプレがある"),
        _ => slot::md_template(def.id).unwrap_or(""),
    }
}

/// docv add --print の出力。Slot ファイルを作らずに、書く中身を出す。新しい版の雛形と見比べるため。
pub fn print(slot: &str) -> Result<&'static str> {
    let Some((_, def, ext)) = pages().into_iter().find(|(p, ..)| p == slot) else {
        bail!("{slot} は Slot の名前ではない。docv add --list で作れる Slot の一覧を出す");
    };
    Ok(text(def, ext))
}

/// Slot ファイルを作り、ルートからの相対パスを返す。
pub fn run(project: &Project, slot: &str) -> Result<String> {
    let Some((_, def, ext)) = pages().into_iter().find(|(p, ..)| p == slot) else {
        bail!("{slot} は Slot の名前ではない。docv add --list で作れる Slot の一覧を出す");
    };
    let rel = format!("{}/{}", project.slots_dir(), def.file_name());
    if project.root.join(&rel).exists() {
        bail!("{rel} がすでにある");
    }
    let text = text(def, ext);
    let path = project.root.join(&rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("{} を作れない", parent.display()))?;
    }
    fs::write(&path, text).with_context(|| format!("{rel} を書けない"))?;
    Ok(rel)
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
        write(dir.path(), "docv.yaml", "docs: [docs/**/*.md]\n");
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn creates_slots_at_fixed_paths() {
        let (dir, project) = project();
        assert_eq!(run(&project, "words").unwrap(), "docv/service/words.yaml");
        assert_eq!(
            fs::read_to_string(dir.path().join("docv/service/words.yaml")).unwrap(),
            include_str!("../assets/template/words.yaml")
        );
        assert_eq!(run(&project, "service").unwrap(), "docv/service/service.md");
        assert_eq!(
            fs::read_to_string(dir.path().join("docv/service/service.md")).unwrap(),
            ""
        );
        assert_eq!(run(&project, "api").unwrap(), "docv/system/api.md");
        // Doc ガイドの Slot は、ガイドラインの雛形を書く
        assert_eq!(
            run(&project, "doc-guide").unwrap(),
            "docv/guide/doc-guide.md"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("docv/guide/doc-guide.md")).unwrap(),
            include_str!("../assets/template/doc-guide.md")
        );
    }

    #[test]
    fn prints_without_writing() {
        assert_eq!(
            print("doc-guide").unwrap(),
            include_str!("../assets/template/doc-guide.md")
        );
        assert_eq!(
            print("words").unwrap(),
            include_str!("../assets/template/words.yaml")
        );
        assert_eq!(print("manual").unwrap(), "");
        assert!(print("api-md").is_err());
    }

    #[test]
    fn rejects_unknown_and_existing_slots() {
        let (_dir, project) = project();
        run(&project, "db").unwrap();
        for (page, message) in [
            ("api-md", "Slot の名前ではない"),
            ("docv/service/words.yaml", "Slot の名前ではない"),
            ("db", "docv/system/db.md がすでにある"),
        ] {
            let error = run(&project, page).unwrap_err().to_string();
            assert!(error.contains(message), "{page}: {error}");
        }
    }

    #[test]
    fn lists_every_page() {
        let list = list();
        assert!(list.starts_with("install "), "{list}");
        assert!(list.contains("system/api.md"), "{list}");
        assert!(list.contains("service/manual.md"), "{list}");
        assert_eq!(list.lines().count(), DEFS.len());
    }
}
