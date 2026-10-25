//! コマンドラインの入口。コマンドの一覧は docv/service/manual.md。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::check::Options;
use crate::workspace::{Locate, Project};
use crate::{build, check, desc, fix, generate, init, open, schema};

/// 終了コード。docv/service/service/cli.md の「終了コード」。
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
    Init {
        /// docv.yaml を作らずに、書く中身を出す
        #[arg(long, conflicts_with = "force")]
        print: bool,
        /// docv.yaml がすでにあっても書き直す
        #[arg(long)]
        force: bool,
    },
    /// docv の概要を出す
    Desc,
    /// 管理対象のファイルと Slot がルールに沿っているかを確かめる
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
    /// 人間向けの HTML を作る。dir.image を書いていれば、Slot のページを JPG にする
    Build {
        /// dir.image を書いていても JPG を作らない。dir.image の中には触れない
        #[arg(long)]
        no_image: bool,
    },
    /// docv build が作った HTML（プロジェクト概要）を、既定のブラウザで開く
    Open,
    /// Slot の一覧を出し、Slot ファイルを作り、YAML の Slot のスキーマを出す
    #[command(subcommand)]
    Slot(SlotCommand),
}

#[derive(Debug, Subcommand)]
enum SlotCommand {
    /// Slot の名前、ファイル、ミッションの一覧を出す
    List,
    /// Slot ファイルを、決まったパスに作る
    Gen {
        /// 作る Slot（docv slot list の名前）
        #[arg(value_name = "slot")]
        slot: String,
        /// Slot ファイルを作らずに、書く中身を出す
        #[arg(long, conflicts_with = "force")]
        print: bool,
        /// Slot ファイルがすでにあっても書き直す
        #[arg(long)]
        force: bool,
    },
    /// YAML の Slot のスキーマを出す
    Schema {
        /// スキーマを出す YAML の Slot（docv slot list の名前）
        #[arg(value_name = "slot")]
        slot: String,
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
    // docv は docv.yaml のある場所で実行する。ルートは常にカレントディレクトリ。
    let root = std::env::current_dir()?;
    let locate = Locate { config: cli.config };
    match cli.command {
        Command::Init { print: true, .. } => {
            print!("{}", init::config_text());
            Ok(ExitCode::SUCCESS)
        }
        Command::Init { force, .. } => {
            let path = locate.config_path(&root);
            init::run(&path, force)?;
            println!("{} を作った", path.display());
            println!("docv slot gen doc-guide で、ドキュメントのガイドラインの雛形を作る");
            Ok(ExitCode::SUCCESS)
        }
        Command::Slot(SlotCommand::List) => {
            print!("{}", generate::list());
            Ok(ExitCode::SUCCESS)
        }
        Command::Slot(SlotCommand::Gen {
            slot, print: true, ..
        }) => {
            print!("{}", generate::print(&slot)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Slot(SlotCommand::Schema { slot }) => {
            print!("{}", schema::text(&slot)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Desc => {
            print!("{}", desc::text());
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
        Command::Slot(SlotCommand::Gen { slot, force, .. }) => {
            let generated = generate::run(project, &slot, force)?;
            println!("{} を作った", generated.path);
            if let Some(note) = generated.note {
                print!("{note}");
            }
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
            println!("error {} 件", diagnostics.len());
            Ok(failure_if(!diagnostics.is_empty()))
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
        Command::Build { no_image } => {
            let out = project.out_dir();
            let image = project.image_dir().filter(|_| !no_image);
            build::run(project, &out, image.as_deref())?;
            println!("{} に出力した", out.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Open => {
            let Some(program) = open::opener() else {
                eprintln!("docv: docv open は、この OS に対応していない（macOS と Windows だけ）");
                return Ok(ExitCode::from(EXIT_FAILURE));
            };
            open::run(program, &project.out_dir())?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Init { .. }
        | Command::Desc
        | Command::Slot(SlotCommand::List | SlotCommand::Schema { .. }) => {
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
