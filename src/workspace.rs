//! ルートと設定ファイルの探索、ファイルの分類。docv/system/system.md の「処理の流れ」と「ファイルの分類」。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use globset::{GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::{self, Config};

/// コマンドラインで指定された設定ファイル。ルートは常にカレントディレクトリ。
/// config は、テストのための非公開のオプション（--config）でだけ指定する。
#[derive(Debug, Clone, Default)]
pub struct Locate {
    pub config: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub config: Config,
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
    Managed,
    Unmanaged,
}

impl Locate {
    /// 設定ファイルの場所。--config がなければ、ルート（カレントディレクトリ）の docv.yaml。
    pub fn config_path(&self, root: &Path) -> PathBuf {
        self.config
            .as_ref()
            .map_or_else(|| root.join(config::FILE_NAME), |c| root.join(c))
    }
}

impl Project {
    /// root はカレントディレクトリ。docv は docv.yaml のある場所で実行する。
    pub fn open(locate: &Locate, root: &Path) -> Result<Self> {
        let config_path = locate.config_path(root);
        if !config_path.is_file() {
            bail!(
                "{} が見つからない。docv.yaml のあるディレクトリで実行するか、docv init で作る",
                config_path.display()
            );
        }
        let config = config::load(&config_path)?;
        Self::new(config_path, root, config)
    }

    fn new(config_path: PathBuf, root: &Path, config: Config) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("ルート {} が見つからない", root.display()))?;
        // ルートと比べられるように、設定ファイルのパスもシンボリックリンクを解く。
        let config_path = config_path.canonicalize().unwrap_or(config_path);
        Ok(Project {
            root,
            config_path,
            ignore: glob_set(&config.ignore)?,
            config,
        })
    }

    /// 除外したものを除く .md のファイルを、パスの辞書順で返す。
    pub fn files(&self) -> Result<Vec<DocFile>> {
        let output_dirs = self.output_dirs();
        let mut files = Vec::new();
        for rel in self.markdown_under(&self.root)? {
            if let Some(class) = self.classify_with(&rel, &output_dirs) {
                files.push(DocFile { path: rel, class });
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    /// Slot のディレクトリの中の .md のファイル（md の Slot と ADR）を、パスの辞書順で返す。
    /// Free ファイルのルールはかけず、整形と textlint だけをかける。シンボリックリンクは書き換えないので除く。
    pub fn slot_markdown(&self) -> Result<Vec<String>> {
        Ok(self
            .walk_slot_dir()?
            .into_iter()
            .filter(|(rel, symlink)| !symlink && rel.ends_with(".md"))
            .map(|(rel, _)| rel)
            .collect())
    }

    /// Slot のディレクトリの中のファイル（Slot と ADR の候補）を、パスの辞書順で返す。シンボリックリンクも含む。
    /// 除外するもの（.gitignore に書かれている、ignore に当てはまる）と、
    /// 名前がドットで始まるファイルとディレクトリ（.DS_Store など）は除く。
    pub fn slot_dir_files(&self) -> Result<Vec<String>> {
        Ok(self
            .walk_slot_dir()?
            .into_iter()
            .map(|(rel, _)| rel)
            .collect())
    }

    /// 末尾の改行を整える YAML のファイル（ルートの中の設定ファイルと、YAML の Slot）。シンボリックリンクは書き換えないので除く。
    /// targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
    pub fn yaml_targets(&self, targets: &[String]) -> Result<Vec<String>> {
        let mut files: Vec<String> = self
            .walk_slot_dir()?
            .into_iter()
            .filter(|(rel, symlink)| !symlink && (rel.ends_with(".yaml") || rel.ends_with(".yml")))
            .map(|(rel, _)| rel)
            .collect();
        if let Ok(rel) = self.config_path.strip_prefix(&self.root) {
            files.push(crate::markdown::slash_path(rel));
        }
        files.retain(|path| targets.is_empty() || targets.contains(path));
        files.sort();
        Ok(files)
    }

    /// Slot のディレクトリの中のファイルと、それがシンボリックリンクか。
    fn walk_slot_dir(&self) -> Result<Vec<(String, bool)>> {
        let dir = self.root.join(self.slot_dir());
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        for entry in walker(&dir) {
            let entry = entry?;
            let Some(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                continue;
            }
            let rel = relative(&self.root, entry.path())?;
            if self.ignore.is_match(&rel) {
                continue;
            }
            files.push((rel, file_type.is_symlink()));
        }
        files.sort();
        Ok(files)
    }

    /// dir の下の .md のファイルの、ルートからの相対パス。.gitignore に書かれたものと、シンボリックリンクは除く。
    fn markdown_under(&self, dir: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        for entry in walker(dir) {
            let entry = entry?;
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            files.push(relative(&self.root, path)?);
        }
        Ok(files)
    }

    /// ルートからの相対パスを分類する。docv build の出力先の中と ignore に当てはまるものは None。
    /// .gitignore は見ない。ファイルがあるかどうかも見ない。
    pub fn classify(&self, rel: &str) -> Option<Class> {
        self.classify_with(rel, &self.output_dirs())
    }

    /// docv build の出力先（dir.html と dir.image）。
    fn output_dirs(&self) -> Vec<String> {
        let mut dirs = vec![self.config.html_dir()];
        dirs.extend(self.config.image_dir());
        dirs
    }

    /// Slot のディレクトリの中の Markdown は、Slot ファイルとして読むので、管理対象にも管理外にもしない。
    fn classify_with(&self, rel: &str, output_dirs: &[String]) -> Option<Class> {
        if output_dirs.iter().any(|d| is_under(rel, d))
            || is_under(rel, &self.slot_dir())
            || self.ignore.is_match(rel)
        {
            return None;
        }
        Some(if is_under(rel, &self.config.free_dir()) {
            Class::Managed
        } else {
            Class::Unmanaged
        })
    }

    /// 管理対象のファイルのパス。
    pub fn managed(&self) -> Result<Vec<String>> {
        Ok(self
            .files()?
            .into_iter()
            .filter(|file| file.class == Class::Managed)
            .map(|file| file.path)
            .collect())
    }

    /// check と fix の対象の管理対象のファイル。targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
    pub fn targets(&self, targets: &[String]) -> Result<Vec<String>> {
        Ok(self
            .managed()?
            .into_iter()
            .filter(|path| targets.is_empty() || targets.contains(path))
            .collect())
    }

    /// fix で書き換えるファイル。管理対象のファイルと、Slot のディレクトリの中の .md のファイル。
    /// targets が空ならすべて、そうでなければそのパス（ルートからの相対パス）だけ。
    pub fn fix_targets(&self, targets: &[String]) -> Result<Vec<String>> {
        let mut files = self.managed()?;
        files.extend(self.slot_markdown()?);
        files.retain(|path| targets.is_empty() || targets.contains(path));
        files.sort();
        Ok(files)
    }

    /// Slot のディレクトリ（ルートからの相対パス）。
    pub fn slot_dir(&self) -> String {
        self.config.slot_dir()
    }

    /// docv build の HTML の出力先。
    pub fn out_dir(&self) -> PathBuf {
        self.root.join(self.config.html_dir())
    }

    /// docv build の JPG の出力先。dir.image を書いていなければ None。
    pub fn image_dir(&self) -> Option<PathBuf> {
        self.config.image_dir().map(|dir| self.root.join(dir))
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

/// dir の下を辿る。.gitignore に書かれたものと、名前がドットで始まるものは辿らない。
fn walker(dir: &Path) -> ignore::Walk {
    WalkBuilder::new(dir)
        .require_git(false)
        .git_global(false)
        .ignore(false)
        .build()
}

fn glob_set(patterns: &[String]) -> Result<GlobSet> {
    let mut set = GlobSetBuilder::new();
    for pattern in patterns {
        set.add(
            globset::GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()?,
        );
    }
    Ok(set.build()?)
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    let Ok(rel) = path.strip_prefix(root) else {
        bail!("{} がルートの外にある", path.display());
    };
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    Ok(parts.join("/"))
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

    const CONFIG: &str = "dir:
  free: docs
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
            "output/docv/html/index.md",
            "docv/adr/rust.md",
            "docv/service/secret.md",
        ] {
            write(root, path, "# x\n");
        }

        let project = Project::open(&Locate::default(), root).unwrap();
        let files = project.files().unwrap();
        let summary: Vec<(&str, Class)> =
            files.iter().map(|f| (f.path.as_str(), f.class)).collect();
        assert_eq!(
            summary,
            [
                ("README.md", Class::Unmanaged),
                ("docs/spec/a.md", Class::Managed),
                ("docs/spec/sub/b.md", Class::Managed),
                ("notes.md", Class::Unmanaged),
            ]
        );
        assert_eq!(project.slot_markdown().unwrap(), ["docv/adr/rust.md"]);
        assert_eq!(
            project.fix_targets(&[]).unwrap(),
            ["docs/spec/a.md", "docs/spec/sub/b.md", "docv/adr/rust.md",]
        );
        assert_eq!(
            project
                .fix_targets(&["docv/adr/rust.md".to_string()])
                .unwrap(),
            ["docv/adr/rust.md"]
        );
    }

    #[test]
    fn uses_config_outside_root() {
        let config_dir = tempfile::tempdir().unwrap();
        let root_dir = tempfile::tempdir().unwrap();
        write(config_dir.path(), "trial.yaml", CONFIG);
        write(root_dir.path(), "docs/a.md", "# x\n");

        let locate = Locate {
            config: Some(config_dir.path().join("trial.yaml")),
        };
        let project = Project::open(&locate, root_dir.path()).unwrap();
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
    fn classifies_paths() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", CONFIG);
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        assert_eq!(project.classify("tmp/x.md"), None);
        assert_eq!(project.classify("docs/x.md"), Some(Class::Managed));
    }

    #[test]
    fn output_dir_prefix_does_not_match_sibling() {
        assert!(is_under("docv/index.md", "docv"));
        assert!(!is_under("docv.md", "docv"));
        assert!(!is_under("docvx/a.md", "docv"));
    }
}
