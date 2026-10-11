//! ルートと設定ファイルの探索、ファイルの分類。docs/spec/cli.md の 2 章。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use globset::{GlobMatcher, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::{self, CheckLevel, Config, DocEntry};
use crate::kind::{Kind, Kinds};

/// コマンドラインで指定された、設定ファイルとルートの場所。
#[derive(Debug, Clone, Default)]
pub struct Locate {
    pub config: Option<PathBuf>,
    pub root: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub config: Config,
    pub kinds: Kinds,
    docs: Vec<GlobMatcher>,
    ignore: GlobSet,
}

/// ルートの中の .md のファイル。path はルートからの相対パスで、区切りは `/`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocFile {
    pub path: String,
    pub class: Class,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// docv.yaml の docs の何番目の項目に当てはまったか。
    Managed(usize),
    Unmanaged,
}

impl Project {
    pub fn open(locate: &Locate, cwd: &Path) -> Result<Self> {
        let (config_path, root) = match (&locate.config, &locate.root) {
            (Some(config), root) => (
                cwd.join(config),
                root.as_ref()
                    .map_or_else(|| cwd.to_path_buf(), |r| cwd.join(r)),
            ),
            (None, Some(root)) => (cwd.join(root).join(config::FILE_NAME), cwd.join(root)),
            (None, None) => {
                let root = find_root(cwd)?;
                (root.join(config::FILE_NAME), root)
            }
        };
        let root = root
            .canonicalize()
            .with_context(|| format!("ルート {} が見つからない", root.display()))?;
        let config = config::load(&config_path)?;
        let kinds = Kinds::new(&config.kinds);
        let docs = config
            .docs
            .iter()
            .map(|entry| config::glob(&entry.path))
            .collect::<Result<_>>()?;
        let mut ignore = GlobSetBuilder::new();
        for pattern in &config.ignore {
            ignore.add(
                globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()?,
            );
        }
        Ok(Project {
            root,
            config_path,
            config,
            kinds,
            docs,
            ignore: ignore.build()?,
        })
    }

    /// 除外したものを除く .md のファイルを、パスの辞書順で返す。
    pub fn files(&self) -> Result<Vec<DocFile>> {
        let output_dir = normalize_dir(&self.config.output.dir);
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&self.root)
            .require_git(false)
            .git_global(false)
            .ignore(false)
            .build();
        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let rel = relative(&self.root, path)?;
            if is_under(&rel, &output_dir) || self.ignore.is_match(&rel) {
                continue;
            }
            let class = self
                .docs
                .iter()
                .position(|m| m.is_match(&rel))
                .map_or(Class::Unmanaged, Class::Managed);
            files.push(DocFile { path: rel, class });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    pub fn entry(&self, index: usize) -> &DocEntry {
        &self.config.docs[index]
    }

    pub fn kind_of(&self, index: usize) -> &Kind {
        self.kinds
            .get(&self.entry(index).kind)
            .expect("docs の種類は設定の検証で確かめてある")
    }

    /// ルートからの相対パスが tmp の項目に当てはまるか。ファイルがあるかどうかは見ない。
    pub fn is_tmp_path(&self, rel: &str) -> bool {
        self.docs
            .iter()
            .position(|m| m.is_match(rel))
            .is_some_and(|index| self.kind_of(index).is_tmp())
    }

    pub fn check_level_of(&self, index: usize) -> CheckLevel {
        self.entry(index).check
    }

    pub fn title(&self) -> String {
        self.config.title.clone().unwrap_or_else(|| {
            self.root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }
}

fn find_root(cwd: &Path) -> Result<PathBuf> {
    for dir in cwd.ancestors() {
        if dir.join(config::FILE_NAME).is_file() {
            return Ok(dir.to_path_buf());
        }
    }
    bail!(
        "{} が見つからない。docv init で作るか、--config で指定する",
        config::FILE_NAME
    )
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("{} がルートの外にある", path.display()))?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    Ok(parts.join("/"))
}

fn normalize_dir(dir: &str) -> String {
    dir.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn is_under(path: &str, dir: &str) -> bool {
    !dir.is_empty()
        && path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    const CONFIG: &str = "version: 1
docs:
  - path: README.md
    kind: overview
    question: q
  - path: docs/spec/*.md
    kind: spec
  - path: docs/**/*.md
    kind: fixme
ignore:
  - tmp/**
";

    #[test]
    fn classifies_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "docv.yaml", CONFIG);
        write(root, ".gitignore", "secret.md\n");
        for path in [
            "README.md",
            "docs/spec/a.md",
            "docs/spec/sub/b.md",
            "docs/memo.txt",
            "notes.md",
            "secret.md",
            "tmp/work.md",
            "docv/index.md",
        ] {
            write(root, path, "# x\n");
        }

        let project = Project::open(&Locate::default(), &root.join("docs")).unwrap();
        let files = project.files().unwrap();
        let summary: Vec<(&str, Class)> =
            files.iter().map(|f| (f.path.as_str(), f.class)).collect();
        assert_eq!(
            summary,
            [
                ("README.md", Class::Managed(0)),
                ("docs/spec/a.md", Class::Managed(1)),
                ("docs/spec/sub/b.md", Class::Managed(2)),
                ("notes.md", Class::Unmanaged),
            ]
        );
        assert_eq!(project.kind_of(2).id, "fixme");
    }

    #[test]
    fn uses_config_outside_root() {
        let config_dir = tempfile::tempdir().unwrap();
        let root_dir = tempfile::tempdir().unwrap();
        write(config_dir.path(), "trial.yaml", CONFIG);
        write(root_dir.path(), "README.md", "# x\n");

        let locate = Locate {
            config: Some(config_dir.path().join("trial.yaml")),
            root: Some(root_dir.path().to_path_buf()),
        };
        let project = Project::open(&locate, Path::new("/")).unwrap();
        assert_eq!(project.files().unwrap().len(), 1);
        assert_eq!(
            project.title(),
            project.root.file_name().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn reports_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let error = Project::open(&Locate::default(), dir.path()).unwrap_err();
        assert!(error.to_string().contains("docv.yaml が見つからない"));
    }

    #[test]
    fn output_dir_prefix_does_not_match_sibling() {
        assert!(is_under("docv/index.md", "docv"));
        assert!(!is_under("docv.md", "docv"));
        assert!(!is_under("docvx/a.md", "docv"));
    }
}
