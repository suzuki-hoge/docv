//! docv add。枠のファイルを、決まったパスに作る。docv/service/manual.yaml の docv add。
//!
//! パスは docv が枠ごとに決めていて、選ぶ余地はない。yaml の枠には項目のサンプル（テンプレ）を書き、md の枠は空のファイルを作る。
//! yaml と md のどちらでも置ける枠（API 仕様書、DB 仕様書）は、`api-yaml` と `api-md` のように、名前で形を選ぶ。

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result, bail};

use crate::slot::{self, DEFS, Def, Format};
use crate::workspace::Project;

/// 作れる枠の名前と、枠と拡張子の組を、枠の一覧の順に返す。
fn pages() -> Vec<(String, &'static Def, &'static str)> {
    DEFS.iter()
        .flat_map(|def| {
            def.extensions().iter().map(move |ext| {
                let page = match def.format {
                    Format::Either => format!("{}-{ext}", def.id),
                    _ => def.id.to_string(),
                };
                (page, def, *ext)
            })
        })
        .collect()
}

/// docv add --list の出力。名前、作るファイル（slots.dir からの相対パス）、枠の名前を 1 行ずつ並べる。
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

/// 枠のファイルを作り、ルートからの相対パスを返す。
pub fn run(project: &Project, slot: &str) -> Result<String> {
    let Some((_, def, ext)) = pages().into_iter().find(|(p, ..)| p == slot) else {
        bail!("{slot} は枠の名前ではない。docv add --list で作れる枠の一覧を出す");
    };
    let dir = project.slots_dir();
    if let Some(existing) = def
        .file_names()
        .into_iter()
        .map(|f| format!("{dir}/{f}"))
        .find(|f| project.root.join(f).exists())
    {
        bail!("{existing} がすでにある");
    }
    let rel = format!("{dir}/{}/{}.{ext}", def.group.dir(), def.id);
    let text = match ext {
        "yaml" => slot::template(def.id).expect("yaml の枠にはテンプレがある"),
        _ => "",
    };
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
        write(
            dir.path(),
            "docv.yaml",
            "version: 1\ndocs: [docs/**/*.md]\n",
        );
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    #[test]
    fn creates_slots_at_fixed_paths() {
        let (dir, project) = project();
        assert_eq!(run(&project, "manual").unwrap(), "docv/service/manual.yaml");
        assert_eq!(
            fs::read_to_string(dir.path().join("docv/service/manual.yaml")).unwrap(),
            include_str!("../templates/slot/manual.yaml")
        );
        assert_eq!(run(&project, "service").unwrap(), "docv/service/service.md");
        assert_eq!(
            fs::read_to_string(dir.path().join("docv/service/service.md")).unwrap(),
            ""
        );
        assert_eq!(run(&project, "api-md").unwrap(), "docv/system/api.md");
    }

    #[test]
    fn rejects_unknown_and_existing_slots() {
        let (_dir, project) = project();
        run(&project, "db-yaml").unwrap();
        for (page, message) in [
            ("api", "枠の名前ではない"),
            ("docv/service/manual.yaml", "枠の名前ではない"),
            ("db-yaml", "docv/system/db.yaml がすでにある"),
            ("db-md", "docv/system/db.yaml がすでにある"),
        ] {
            let error = run(&project, page).unwrap_err().to_string();
            assert!(error.contains(message), "{page}: {error}");
        }
    }

    #[test]
    fn lists_every_page() {
        let list = list();
        assert!(list.starts_with("install "), "{list}");
        assert!(list.contains("api-yaml"), "{list}");
        assert!(list.contains("api-md"), "{list}");
        assert!(list.contains("service/manual.yaml"), "{list}");
        assert_eq!(list.lines().count(), DEFS.len() + 2);
    }
}
