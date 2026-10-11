//! コマンドラインの入口。コマンドの一覧は docs/spec/cli.md。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::check::Options;
use crate::diagnostic::Level;
use crate::workspace::{Locate, Project};
use crate::{build, check, config, desc, fix, index, init, migrate, new, retire};

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
    /// docv.yaml を作る
    Init,
    /// テンプレからファイルを作る
    New {
        /// 作るファイル。docv.yaml の docs の項目から種類を決める
        path: PathBuf,
    },
    /// 管理対象のファイルがルールに沿っているかを確かめる
    Check {
        /// 確かめるファイル。省略すると管理対象のファイルをすべて確かめる
        paths: Vec<PathBuf>,
        /// textlint だけを飛ばす
        #[arg(long)]
        no_textlint: bool,
    },
    /// 章やファイルを archive に移す
    Retire {
        /// 移す元のファイル
        file: PathBuf,
        /// 移す章の見出し（章番号を除く）か章番号。省略するとファイル全体を移す
        #[arg(long)]
        section: Option<String>,
        /// 移す理由
        #[arg(long)]
        reason: String,
    },
    /// AI Agent 向けのインデックスを出す
    Index {
        /// 過去のファイルも並べる
        #[arg(long)]
        all: bool,
        /// 出力の形
        #[arg(long, value_enum, default_value = "md")]
        format: index::Format,
    },
    /// フレームワークのルールファイルを出す
    Desc,
    /// 管理対象のファイルを整形し、textlint で自動修正する
    Fix {
        /// 直すファイル。省略すると管理対象のファイルをすべて直す
        paths: Vec<PathBuf>,
        /// textlint の自動修正を飛ばし、整形だけをする
        #[arg(long)]
        no_textlint: bool,
    },
    /// 人間向けの HTML を作る
    Build {
        /// 出力先。省略すると docv.yaml の output.dir
        #[arg(long)]
        out: Option<PathBuf>,
    },
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
    let cwd = std::env::current_dir()?;
    let locate = Locate {
        config: cli.config,
        root: cli.root,
    };
    match cli.command {
        Command::Init => {
            let path = match &locate.config {
                Some(config) => cwd.join(config),
                None => locate
                    .root
                    .as_ref()
                    .map_or_else(|| cwd.clone(), |root| cwd.join(root))
                    .join(config::FILE_NAME),
            };
            init::run(&path)?;
            println!("{} を作った", path.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Desc => {
            print!("{}", desc::RULES);
            Ok(ExitCode::SUCCESS)
        }
        Command::MigrateGuide => {
            let project = match locate.resolve(&cwd) {
                Some((config_path, _)) if locate.config.is_some() || config_path.is_file() => {
                    Project::open(&locate, &cwd)?
                }
                Some((_, root)) => Project::unconfigured(&root)?,
                None => Project::unconfigured(&cwd)?,
            };
            print!("{}", migrate::render(&project, Options { textlint: true })?);
            Ok(ExitCode::SUCCESS)
        }
        command => {
            let project = Project::open(&locate, &cwd)?;
            execute_in_project(&project, &cwd, command)
        }
    }
}

fn execute_in_project(project: &Project, cwd: &Path, command: Command) -> Result<ExitCode> {
    match command {
        Command::New { path } => {
            let rel = project.relative_path(cwd, &path)?;
            new::run(project, &rel)?;
            println!("{rel} を作った");
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { paths, no_textlint } => {
            let targets = relative_targets(project, cwd, &paths);
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
        Command::Retire {
            file,
            section,
            reason,
        } => {
            let rel = project.relative_path(cwd, &file)?;
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let outcome = retire::run(
                project,
                &retire::Request {
                    path: &rel,
                    section: section.as_deref(),
                    reason: &reason,
                    today: &today,
                },
            )?;
            println!("{} に移した", outcome.archive);
            if outcome.links.is_empty() {
                println!("移した元を指すリンクはない");
            } else {
                println!("移した元を指すリンクがある。必要なら直す");
                for link in &outcome.links {
                    println!("  {link}");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Index { all, format } => {
            let index = index::collect(project, all)?;
            print!("{}", index::render(&index, format)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Fix { paths, no_textlint } => {
            let targets = relative_targets(project, cwd, &paths);
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
        Command::Build { out } => {
            let out = out.map_or_else(
                || project.root.join(&project.config.output.dir),
                |out| cwd.join(out),
            );
            build::run(project, &out)?;
            println!("{} に出力した", out.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Init | Command::Desc | Command::MigrateGuide => {
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
fn relative_targets(project: &Project, cwd: &Path, paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|p| cwd.join(p).canonicalize().ok())
        .filter_map(|p| {
            p.strip_prefix(&project.root)
                .ok()
                .map(crate::markdown::slash_path)
        })
        .collect()
}
