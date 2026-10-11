//! コマンドラインの入口。コマンドの一覧は docs/spec/cli.md。

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

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
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let locate = Locate {
        config: cli.config,
        root: cli.root,
    };
    let result = std::env::current_dir()
        .map_err(anyhow::Error::from)
        .and_then(|cwd| Project::open(&locate, &cwd));
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("docv: {e:#}");
            ExitCode::from(EXIT_UNRUNNABLE)
        }
    }
}
