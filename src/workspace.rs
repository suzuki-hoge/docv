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
    /// docv.yaml を読んだか。migrate-guide だけが、docv.yaml がなくても動く。
    pub configured: bool,
    pub config: Config,
    docs: GlobSet,
    ignore: GlobSet,
}

/// ルートの中の .md のファイル。path はルートからの相対パスで、区切りは `/`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocFile {
    pub path: String,
    pub class: Class,
}

/// overrides を当てた、ファイルごとの道具の設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub textlint: bool,
    pub style: bool,
    pub format: bool,
    /// frontmatter（mission と version）を必須にするか。
    pub mission_required: bool,
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
        Self::new(config_path, root, config, true)
    }

    /// docv.yaml がないときのプロジェクト。すべてのファイルが管理外になる。
    pub fn unconfigured(root: &Path) -> Result<Self> {
        Self::new(
            root.join(config::FILE_NAME),
            root,
            Config::unconfigured(),
            false,
        )
    }

    fn new(config_path: PathBuf, root: &Path, config: Config, configured: bool) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("ルート {} が見つからない", root.display()))?;
        Ok(Project {
            root,
            config_path,
            configured,
            docs: glob_set(&config.docs)?,
            ignore: glob_set(&config.ignore)?,
            config,
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
            if let Some(class) = self.classify_with(&rel, &output_dir) {
                files.push(DocFile { path: rel, class });
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    /// ルートからの相対パスを分類する。output.dir の中と ignore に当てはまるものは None。
    /// .gitignore は見ない。ファイルがあるかどうかも見ない。
    pub fn classify(&self, rel: &str) -> Option<Class> {
        self.classify_with(rel, &normalize_dir(&self.config.output.dir))
    }

    /// 枠のディレクトリの中の Markdown は、枠として読むので、原本にも管理外にもしない。
    fn classify_with(&self, rel: &str, output_dir: &str) -> Option<Class> {
        if is_under(rel, output_dir)
            || is_under(rel, &self.slots_dir())
            || self.ignore.is_match(rel)
        {
            return None;
        }
        Some(if self.docs.is_match(rel) {
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

    /// overrides を当てた、そのファイルへの道具の設定。後に書いた overrides が優先する。
    pub fn settings_of(&self, rel: &str) -> Settings {
        let mut settings = Settings {
            textlint: true,
            style: true,
            format: true,
            mission_required: true,
        };
        for o in &self.config.overrides {
            let hit = o
                .paths
                .iter()
                .filter_map(|p| config::glob(p).ok())
                .any(|g| g.is_match(rel));
            if !hit {
                continue;
            }
            if let Some(v) = o.textlint {
                settings.textlint = v;
            }
            if let Some(v) = o.style {
                settings.style = v;
            }
            if let Some(v) = o.format {
                settings.format = v;
            }
            if let Some(v) = o.mission {
                settings.mission_required = v == config::Requirement::Required;
            }
        }
        settings
    }

    /// 枠のディレクトリ（ルートからの相対パス）。
    pub fn slots_dir(&self) -> String {
        normalize_dir(&self.config.slots.dir)
    }

    /// build-html と build-image の出力先。
    pub fn out_dir(&self) -> PathBuf {
        self.root.join(&self.config.output.dir)
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
  - README.md
  - docs/**/*.md
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
            "docv/build/index.md",
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
                ("README.md", Class::Managed),
                ("docs/spec/a.md", Class::Managed),
                ("docs/spec/sub/b.md", Class::Managed),
                ("notes.md", Class::Unmanaged),
            ]
        );
    }

    #[test]
    fn applies_overrides_in_order() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "docv.yaml",
            "version: 1\ndocs: [docs/**/*.md]\noverrides:\n  - paths: [docs/tmp/**]\n    textlint: false\n    mission: optional\n  - paths: [docs/tmp/keep.md]\n    textlint: true\n",
        );
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        let memo = project.settings_of("docs/tmp/memo.md");
        assert!(!memo.textlint && !memo.mission_required && memo.style && memo.format);
        assert!(project.settings_of("docs/tmp/keep.md").textlint);
        assert!(project.settings_of("docs/spec/a.md").mission_required);
    }

    #[test]
    fn uses_config_outside_root() {
        let config_dir = tempfile::tempdir().unwrap();
        let root_dir = tempfile::tempdir().unwrap();
        write(config_dir.path(), "trial.yaml", CONFIG);
        write(root_dir.path(), "README.md", "# x\n");

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
