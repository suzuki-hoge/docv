//! コマンドラインの入口。コマンドの一覧は docv/slot/service/manual.md。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::check::Options;
use crate::workspace::{Locate, Project};
use crate::{
    build, check, desc, diagnostic, diff_open, fix, generate, init, list, open, review_guide,
    update_guide,
};

/// 終了コード。docv/slot/service/service/cli.md の「終了コード」。
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
    /// Slot と Free ファイルの一覧を出す。Slot は、置いていないものも含めてすべて出し、置いたものに * を付ける
    List,
    /// Slot ファイルを、決まったパスに作る
    Gen {
        /// 作る Slot（docv list の名前）
        #[arg(value_name = "slot")]
        slot: String,
        /// Slot ファイルを作らずに、書く中身を出す
        #[arg(long, conflicts_with = "force")]
        print: bool,
        /// Slot ファイルがすでにあっても書き直す
        #[arg(long)]
        force: bool,
    },
    /// docv を更新したときに、版ごとの変更点と対応を出す
    UpdateGuide {
        /// 更新する前に使っていた docv の版（docv.yaml の docv_version）
        #[arg(long, value_name = "version")]
        from: String,
    },
    /// ドキュメントをレビューする手順と、ADR と PDR の一覧を出す。確かめて指摘するのは、これを読んだ AI Agent
    ReviewGuide,
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
    /// 人間向けの HTML とインデックスを作る
    Build,
    /// docv build が作った HTML（Slot のページ）を、既定のブラウザで開く
    Open,
    /// git のリビジョンと作業ツリーで変わったファイルに印を付けた HTML を作り、既定のブラウザで開く
    DiffOpen {
        /// 比べる git のリビジョン（ブランチ、タグ、コミット）
        #[arg(long, value_name = "rev", default_value = "HEAD")]
        from: String,
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
            println!("docv gen doc-guide で、ドキュメントのガイドラインの雛形を作る");
            Ok(ExitCode::SUCCESS)
        }
        // docv.yaml があれば読み、なければ Slot の一覧だけを出す。docv を入れる前にも、どんな Slot があるかを見られるように
        Command::List => {
            let project = if locate.config_path(&root).is_file() {
                Some(Project::open(&locate, &root)?)
            } else {
                None
            };
            print!("{}", list::text(project.as_ref())?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Gen {
            slot, print: true, ..
        } => {
            print!("{}", generate::print(&slot)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Desc => {
            print!("{}", desc::text());
            Ok(ExitCode::SUCCESS)
        }
        Command::UpdateGuide { from } => {
            print!("{}", update_guide::text(&from)?);
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
        Command::Gen { slot, force, .. } => {
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
            print!("{}", diagnostic::report(&diagnostics));
            Ok(failure_if(diagnostic::has_error(&diagnostics)))
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
        Command::Build => {
            let out = project.out_dir();
            build::run(project, &out)?;
            println!("{} に出力した", out.display());
            println!("{} を書いた", crate::index::path(project).display());
            Ok(ExitCode::SUCCESS)
        }
        Command::ReviewGuide => {
            print!("{}", review_guide::text(project)?);
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
        Command::DiffOpen { from } => {
            let Some(program) = open::opener() else {
                eprintln!(
                    "docv: docv diff-open は、この OS に対応していない（macOS と Windows だけ）"
                );
                return Ok(ExitCode::from(EXIT_FAILURE));
            };
            let Some(out) = diff_open::run(project, &from)? else {
                println!("{from} との差分がない");
                return Ok(ExitCode::SUCCESS);
            };
            println!("{} に出力した", out.display());
            open::run(program, &out)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Init { .. } | Command::Desc | Command::UpdateGuide { .. } | Command::List => {
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
