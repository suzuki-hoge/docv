//! コマンドラインの入口。コマンドの一覧は docv/service/manual.yaml。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::check::Options;
use crate::diagnostic::Level;
use crate::workspace::{Locate, Project};
use crate::{add, build, check, desc, fix, init, migrate};

/// 終了コード。docv/system/system.md の「終了コード」。
pub const EXIT_FAILURE: u8 = 1;
pub const EXIT_UNRUNNABLE: u8 = 2;

#[derive(Debug, Parser)]
#[command(
    name = "docv",
    version,
    about = "ドキュメントを、AI Agent が読む Markdown と人間が見る HTML の 2 つの形で扱う"
)]
struct Cli {
    /// docv.yaml の代わりに読む設定ファイル。テストのためのもので、公開しない
    #[arg(long, global = true, hide = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// docv.yaml を作る
    Init,
    /// 枠のファイルを、決まったパスに作る
    Add {
        /// 作る枠（docv add --list で一覧を出す）
        #[arg(required_unless_present = "list", value_name = "slot")]
        slot: Option<String>,
        /// 作れる枠の一覧を出す
        #[arg(long, conflicts_with = "slot")]
        list: bool,
    },
    /// 管理対象のファイルと枠がルールに沿っているかを確かめる
    Check {
        /// 確かめるファイル。省略すると管理対象のファイルをすべて確かめる
        #[arg(value_name = "path")]
        paths: Vec<PathBuf>,
        /// textlint だけを飛ばす
        #[arg(long)]
        no_textlint: bool,
    },
    /// 管理対象のファイルを整形し、textlint で自動修正する
    Fix {
        /// 直すファイル。省略すると管理対象のファイルをすべて直す
        #[arg(value_name = "path")]
        paths: Vec<PathBuf>,
        /// textlint の自動修正を飛ばし、整形だけをする
        #[arg(long)]
        no_textlint: bool,
    },
    /// 人間向けの HTML を作る
    BuildHtml,
    /// 枠のページを JPG にする
    BuildImage,
    /// フレームワークのルールファイルを出す
    Desc,
    /// 後から docv を入れるときのガイドを出す
    MigrateGuide,
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
    // docv は docv.yaml のある場所で実行する。ルートは常にカレントディレクトリ。
    let root = std::env::current_dir()?;
    let locate = Locate { config: cli.config };
    match cli.command {
        Command::Init => {
            let path = locate.config_path(&root);
            init::run(&path)?;
            println!("{} を作った", path.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Add { list: true, .. } => {
            print!("{}", add::list());
            Ok(ExitCode::SUCCESS)
        }
        Command::Desc => {
            print!("{}", desc::RULES);
            Ok(ExitCode::SUCCESS)
        }
        Command::MigrateGuide => {
            let project = if locate.config_path(&root).is_file() {
                Project::open(&locate, &root)?
            } else {
                Project::unconfigured(&root)?
            };
            print!("{}", migrate::render(&project, Options { textlint: true })?);
            Ok(ExitCode::SUCCESS)
        }
        command => {
            let project = Project::open(&locate, &root)?;
            execute_in_project(&project, command)
        }
    }
}

fn execute_in_project(project: &Project, command: Command) -> Result<ExitCode> {
    match command {
        Command::Add {
            slot: Some(slot), ..
        } => {
            let rel = add::run(project, &slot)?;
            println!("{rel} を作った");
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { paths, no_textlint } => {
            let targets = relative_targets(project, &paths);
            if !paths.is_empty() && targets.is_empty() {
                return Ok(ExitCode::SUCCESS);
            }
            let options = Options {
                textlint: !no_textlint,
            };
            let diagnostics = check::run(project, &targets, options)?;
            for d in &diagnostics {
                println!("{d}");
            }
            let errors = diagnostics
                .iter()
                .filter(|d| d.level == Level::Error)
                .count();
            let warns = diagnostics.len() - errors;
            println!("error {errors} 件、warn {warns} 件");
            Ok(failure_if(errors > 0))
        }
        Command::Fix { paths, no_textlint } => {
            let targets = relative_targets(project, &paths);
            if !paths.is_empty() && targets.is_empty() {
                return Ok(ExitCode::SUCCESS);
            }
            let report = fix::run(project, &targets, !no_textlint)?;
            for path in &report.changed {
                println!("{path} を書き換えた");
            }
            for (path, reason) in &report.failed {
                eprintln!("{path}: 整形できない: {reason}");
            }
            println!("{} ファイルを書き換えた", report.changed.len());
            Ok(failure_if(!report.failed.is_empty()))
        }
        Command::BuildHtml => {
            let out = project.out_dir();
            build::html(project, &out)?;
            println!("{} に出力した", out.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::BuildImage => {
            let out = project.out_dir();
            build::image(project, &out)?;
            println!("{} に出力した", out.join(build::JPG_DIR).display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Init | Command::Add { .. } | Command::Desc | Command::MigrateGuide => {
            unreachable!("docv.yaml を読まずに実行するコマンド")
        }
    }
}

fn failure_if(failed: bool) -> ExitCode {
    if failed {
        ExitCode::from(EXIT_FAILURE)
    } else {
        ExitCode::SUCCESS
    }
}

/// コマンドラインのパスを、ルートからの相対パスにする。ルートの外や、存在しないパスは捨てる。
fn relative_targets(project: &Project, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|p| project.root.join(p).canonicalize().ok())
        .filter_map(|p| {
            p.strip_prefix(&project.root)
                .ok()
                .map(crate::markdown::slash_path)
        })
        .collect()
}
