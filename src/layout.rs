//! document_root の配置と、画像の置き場所の check。
//! docv/slot/service/service/config.md の「配置」と、docv/slot/service/service/free.md の「画像」。
//!
//! document_root の直下には、free/、slot/、adr/、pdr/ と、build が作る README.md だけを置ける。
//! 画像は、使うファイルと同じディレクトリに `{使うファイルの名前}_{画像の名前}.{拡張子}` の名前で置く。

use std::collections::BTreeSet;

use anyhow::Result;

use crate::config::{FREE_DIR, INDEX_FILE, SLOT_DIR};
use crate::decision::Kind;
use crate::diagnostic::Diagnostic;
use crate::workspace::Project;

/// 画像として扱う拡張子（小文字）。
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "svg", "webp"];

/// 持ち主になれるファイルの拡張子。
const OWNER_EXTENSIONS: [&str; 3] = ["md", "yaml", "yml"];

/// パスが画像か。拡張子の大文字と小文字は区別しない。
pub fn is_image(path: &str) -> bool {
    extension(path).is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

fn extension(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map(|(_, ext)| ext)
}

/// document_root の直下に置けるもの。
fn allowed_entries() -> Vec<&'static str> {
    let mut entries = vec![INDEX_FILE, FREE_DIR, SLOT_DIR];
    entries.extend(Kind::ALL.iter().map(|kind| kind.dir()));
    entries
}

/// root-unknown と image-owner を確かめる。
pub fn check(project: &Project) -> Result<Vec<Diagnostic>> {
    let root = project.config.document_root();
    let files = project.document_root_files()?;
    let mut diagnostics = Vec::new();

    // 直下に置けないものは、直下の名前ごとに 1 つだけ出す。
    let allowed = allowed_entries();
    let mut unknown = BTreeSet::new();
    for path in &files {
        let Some(rest) = path.strip_prefix(&format!("{root}/")) else {
            continue;
        };
        let (entry, nested) = match rest.split_once('/') {
            Some((entry, _)) => (entry, true),
            None => (rest, false),
        };
        // README.md はファイルだけ、ほかはディレクトリだけを置ける。
        let ok = if entry == INDEX_FILE {
            !nested
        } else {
            nested && allowed.contains(&entry)
        };
        if !ok {
            unknown.insert(format!("{root}/{entry}"));
        }
    }
    for path in unknown {
        diagnostics.push(Diagnostic {
            path,
            line: 1,
            rule: "root-unknown".to_string(),
            message: format!(
                "{root}/ の直下に置けるのは {} だけ。docv に読ませないなら docv.yaml の docv_exclude に書く",
                allowed
                    .iter()
                    .map(|e| if *e == INDEX_FILE {
                        e.to_string()
                    } else {
                        format!("{e}/")
                    })
                    .collect::<Vec<_>>()
                    .join("、")
            ),
        });
    }

    // 画像の持ち主は、同じディレクトリの Markdown か YAML のファイル。
    let areas: Vec<String> = [FREE_DIR, SLOT_DIR]
        .into_iter()
        .chain(Kind::ALL.iter().map(|kind| kind.dir()))
        .map(|dir| format!("{root}/{dir}/"))
        .collect();
    let owners: BTreeSet<(&str, &str)> = files
        .iter()
        .filter(|path| {
            extension(path)
                .is_some_and(|ext| OWNER_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        })
        .map(|path| {
            let (dir, name) = split_dir(path);
            (dir, name.rsplit_once('.').map_or(name, |(stem, _)| stem))
        })
        .collect();
    for path in files
        .iter()
        .filter(|path| is_image(path) && areas.iter().any(|area| path.starts_with(area)))
    {
        let (dir, name) = split_dir(path);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        let owned = stem
            .match_indices('_')
            .any(|(i, _)| owners.contains(&(dir, &stem[..i])));
        if !owned {
            diagnostics.push(Diagnostic {
                path: path.clone(),
                line: 1,
                rule: "image-owner".to_string(),
                message: "画像の持ち主がない。画像は、使うファイルと同じディレクトリに {使うファイルの名前}_{画像の名前}.{拡張子} の名前で置く".to_string(),
            });
        }
    }
    Ok(diagnostics)
}

/// パスを、ディレクトリと名前に分ける。
fn split_dir(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "x\n").unwrap();
    }

    fn found(config: &str, paths: &[&str]) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("docv.yaml"), config).unwrap();
        for path in paths {
            write(dir.path(), path);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{} {}", d.path, d.rule))
            .collect()
    }

    #[test]
    fn reports_unknown_entries_in_document_root() {
        let found = found(
            "docv_exclude:\n  - docv/draft/**\n",
            &[
                "docv/README.md",
                "docv/free/a.md",
                "docv/slot/service/manual.md",
                "docv/adr/a.md",
                "docv/pdr/a.md",
                "docv/tmp/a.md",
                "docv/tmp/b.md",
                "docv/memo.md",
                "docv/free.md",
                "docv/README.md.bak",
                "docv/.DS_Store",
                "docv/draft/a.md",
                "README.md",
            ],
        );
        assert_eq!(
            found,
            [
                "docv/README.md.bak root-unknown",
                "docv/free.md root-unknown",
                "docv/memo.md root-unknown",
                "docv/tmp root-unknown",
            ]
        );
    }

    #[test]
    fn reports_images_without_owners() {
        let found = found(
            "{}\n",
            &[
                "docv/free/order.md",
                "docv/free/order_flow.svg",
                "docv/free/order_flow_detail.PNG",
                "docv/free/orders_flow.svg",
                "docv/free/flow.svg",
                "docv/free/sub/order_flow.svg",
                "docv/free/schema/config.md",
                "docv/free/schema/config.json",
                "docv/slot/system/system-overview.md",
                "docv/slot/system/system-overview_parts.svg",
                "docv/slot/system/stack.yaml",
                "docv/slot/system/stack_backend.png",
                "docv/slot/service/service/cli.md",
                "docv/slot/service/service/cli_flow.svg",
                "docv/slot/service/service_flow.svg",
                "docv/adr/use_rust.md",
                "docv/adr/use_rust_compare.svg",
                "docv/adr/img/use_rust_compare.svg",
                "docv/pdr/free.md",
                "docv/pdr/paid_plan.svg",
            ],
        );
        assert_eq!(
            found,
            [
                "docv/adr/img/use_rust_compare.svg image-owner",
                "docv/free/flow.svg image-owner",
                "docv/free/orders_flow.svg image-owner",
                "docv/free/sub/order_flow.svg image-owner",
                "docv/pdr/paid_plan.svg image-owner",
                "docv/slot/service/service_flow.svg image-owner",
            ]
        );
    }

    #[test]
    fn tells_images_apart() {
        assert!(is_image("docv/free/a.SVG"));
        assert!(is_image("a.webp"));
        assert!(!is_image("docv/free/a.json"));
        assert!(!is_image("docv/free/svg"));
    }
}
