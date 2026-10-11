//! コマンドラインの入口。コマンドの一覧は docs/spec/cli.md。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::check;
use crate::diagnostic::Level;
use crate::workspace::{Locate, Project};

/// 終了コード。docs/spec/cli.md の 2 章。
pub const EXIT_FAILURE: u8 = 1;
pub const EXIT_UNRUNNABLE: u8 = 2;

#[derive(Debug, Parser)]
#[command(
    name = "docv",
    version,
    about = "ドキュメントを、AI Agent が読む Markdown と人間が見る HTML の 2 つの形で扱う"
)]
struct Cli {
    /// docv.yaml を直接指定する
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// ルートを指定する。--config と一緒に使う
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 管理対象のファイルがルールに沿っているかを確かめる
    Check {
        /// 確かめるファイル。省略すると管理対象のファイルをすべて確かめる
        paths: Vec<PathBuf>,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match execute(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("docv: {e:#}");
            ExitCode::from(EXIT_UNRUNNABLE)
        }
    }
}

fn execute(cli: Cli) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let locate = Locate {
        config: cli.config,
        root: cli.root,
    };
    let project = Project::open(&locate, &cwd)?;
    match cli.command {
        Command::Check { paths } => {
            let targets = relative_targets(&project, &cwd, &paths);
            if !paths.is_empty() && targets.is_empty() {
                return Ok(ExitCode::SUCCESS);
            }
            let diagnostics = check::run(&project, &targets)?;
            for d in &diagnostics {
                println!("{d}");
            }
            let errors = diagnostics
                .iter()
                .filter(|d| d.level == Level::Error)
                .count();
            let warns = diagnostics.len() - errors;
            println!("error {errors} 件、warn {warns} 件");
            Ok(if errors > 0 {
                ExitCode::from(EXIT_FAILURE)
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

/// コマンドラインのパスを、ルートからの相対パスにする。ルートの外や、存在しないパスは捨てる。
fn relative_targets(project: &Project, cwd: &Path, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|p| cwd.join(p).canonicalize().ok())
        .filter_map(|p| {
            p.strip_prefix(&project.root).ok().map(|rel| {
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            })
        })
        .collect()
}
