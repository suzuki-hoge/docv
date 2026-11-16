//! docv diff-open。--from のリビジョン（省略時 HEAD）と作業ツリーを git で比べ、新しく足したファイル、書き換えたファイル、消したファイルに印を付けた HTML を作って開く。
//! ページの中には差分を出さない。元のファイルのパスの代わりに git diff のコマンドを出し、詳しい差分は git で見てもらう。
//! HTML は OS のキャッシュのディレクトリに書き、リポジトリには何も書かない。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::build;
use crate::workspace::Project;

/// ファイルの変わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// --from になく、作業ツリーにある。
    Added,
    /// どちらにもあり、中身が違う。
    Modified,
    /// --from にあり、作業ツリーにない。
    Deleted,
}

/// 1 つのファイルの差分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub status: Status,
    /// 足した行数と消した行数。画像などのバイナリは None。
    pub lines: Option<(u64, u64)>,
}

/// --from と作業ツリーの差分。パスはルート（docv.yaml のあるディレクトリ）からの相対パス。
#[derive(Debug, Default)]
pub struct Changes {
    /// 比べたリビジョン。コマンドラインで受け取ったまま。
    pub from: String,
    files: BTreeMap<String, Change>,
    /// まだ git に追加していないファイル。git diff に出ないので、コマンドを出さない。
    untracked: BTreeSet<String>,
    /// --from にあって作業ツリーにないファイルと、--from での中身。
    deleted: BTreeMap<String, String>,
}

impl Changes {
    /// root で git を呼び、--from と作業ツリーの差分のうち、keep が true を返すファイルのものを集める。
    /// git の管理下にないか、--from が解けなければ error。
    pub fn collect(root: &Path, from: &str, keep: impl Fn(&str) -> bool) -> Result<Changes> {
        if git(root, &["rev-parse", "--is-inside-work-tree"]).is_err() {
            bail!("{} は git の管理下にない", root.display());
        }
        let commit = format!("{from}^{{commit}}");
        if git(root, &["rev-parse", "--verify", "--quiet", &commit]).is_err() {
            bail!("{from} は git のリビジョン（ブランチ、タグ、コミット）ではない");
        }
        // --relative で、パスをルートからの相対にし、ルートの外を除く。-z で、パスを引用符で囲ませない。
        let diff = |format: &str| {
            git(
                root,
                &[
                    "diff",
                    format,
                    "--no-renames",
                    "--relative",
                    "-z",
                    from,
                    "--",
                ],
            )
        };
        let name_status = diff("--name-status")?;
        let numstat = diff("--numstat")?;
        let untracked = git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;

        let mut changes = Changes {
            from: from.to_string(),
            files: files_of(&name_status, &numstat),
            ..Changes::default()
        };
        changes.files.retain(|p, _| keep(p));
        let deleted: Vec<String> = changes
            .files
            .iter()
            .filter(|(_, c)| c.status == Status::Deleted)
            .map(|(p, _)| p.clone())
            .collect();
        for path in deleted {
            // <rev>:./<path> は、カレントディレクトリ（ルート）からの相対パスで読む。
            let text = git(root, &["show", &format!("{from}:./{path}")]).unwrap_or_default();
            changes.deleted.insert(path, text);
        }
        for path in untracked.split('\0').filter(|p| !p.is_empty() && keep(p)) {
            // git の行数がないので、ファイルの行を数える。読めなければバイナリとみなす。
            let lines = std::fs::read_to_string(root.join(path))
                .ok()
                .map(|text| (text.lines().count() as u64, 0));
            changes.files.insert(
                path.to_string(),
                Change {
                    status: Status::Added,
                    lines,
                },
            );
            changes.untracked.insert(path.to_string());
        }
        Ok(changes)
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// ファイルの差分。変わっていなければ None。
    pub fn change(&self, path: &str) -> Option<Change> {
        self.files.get(path).copied()
    }

    /// --from にあって作業ツリーにないファイルと、--from での中身。パスの順。
    pub fn deleted(&self) -> impl Iterator<Item = (&str, &str)> {
        self.deleted.iter().map(|(p, t)| (p.as_str(), t.as_str()))
    }

    /// ファイルの差分を見る git のコマンド。docv ルートで実行する。変わっていないか、まだ git に追加していなければ None。
    pub fn command(&self, path: &str) -> Option<String> {
        if !self.files.contains_key(path) || self.untracked.contains(path) {
            return None;
        }
        Some(format!(
            "git diff {} -- {}",
            shell_quote(&self.from),
            shell_quote(path)
        ))
    }
}

/// `git diff --name-status -z` と `git diff --numstat -z` の出力から、ファイルごとの差分を作る。
/// モードだけが変わったファイル（行数がどちらも 0）は、変わっていないとみなす。
fn files_of(name_status: &str, numstat: &str) -> BTreeMap<String, Change> {
    let lines = parse_numstat(numstat);
    let mut files = BTreeMap::new();
    let mut fields = name_status.split('\0').filter(|f| !f.is_empty());
    while let (Some(letter), Some(path)) = (fields.next(), fields.next()) {
        let status = match letter {
            "A" => Status::Added,
            "D" => Status::Deleted,
            // M のほか、T（ファイルとシンボリックリンクの入れ替わり）と U（マージの途中）も、書き換えとみなす。
            _ => Status::Modified,
        };
        let lines = lines.get(path).copied().flatten();
        if status == Status::Modified && lines == Some((0, 0)) {
            continue;
        }
        files.insert(path.to_string(), Change { status, lines });
    }
    files
}

/// `git diff --numstat -z` の出力を読む。1 つのファイルは「足した行数 TAB 消した行数 TAB パス NUL」。
/// 画像などのバイナリは行数の代わりに `-` が出るので、None にする。
fn parse_numstat(text: &str) -> BTreeMap<String, Option<(u64, u64)>> {
    let mut lines = BTreeMap::new();
    for record in text.split('\0').filter(|r| !r.is_empty()) {
        let mut parts = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let count = added.parse::<u64>().ok().zip(removed.parse::<u64>().ok());
        lines.insert(path.to_string(), count);
    }
    lines
}

/// シェルにそのまま貼れるように、英数字と / . _ - と日本語のほかを含めば、単一引用符で囲む。
fn shell_quote(text: &str) -> String {
    let plain = !text.is_empty()
        && text.chars().all(|c| {
            !c.is_ascii() || c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-')
        });
    if plain {
        text.to_string()
    } else {
        format!("'{}'", text.replace('\'', "'\\''"))
    }
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("git を実行できない")?;
    if !output.status.success() {
        bail!(
            "git {} が失敗した: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 差分の HTML の出力先。OS のキャッシュのディレクトリの下。毎回、中を消して作り直す。
pub fn out_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .context("OS のキャッシュのディレクトリが分からない")?
        .join("docv")
        .join("diff-open"))
}

/// 差分を集め、印を付けた HTML を出力先に書く。差分がなければ何も書かずに None。
pub fn run(project: &Project, from: &str) -> Result<Option<PathBuf>> {
    // コードなど、docv が扱わないファイルの差分は見ない。
    let changes = Changes::collect(&project.root, from, |path| project.shows_changes_of(path))?;
    if changes.is_empty() {
        return Ok(None);
    }
    let out = out_dir()?;
    build::run_diff(project, &out, &changes)?;
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_status_and_lines() {
        let files = files_of(
            "A\0a.md\0M\0b c.md\0D\0d.md\0M\0e.png\0M\0f.md\0T\0g.md\0",
            "3\t0\ta.md\x001\t2\tb c.md\x000\t4\td.md\x00-\t-\te.png\x000\t0\tf.md\x001\t1\tg.md\x00",
        );
        let change = |status, lines| Some(Change { status, lines });
        assert_eq!(
            files.get("a.md").copied(),
            change(Status::Added, Some((3, 0)))
        );
        assert_eq!(
            files.get("b c.md").copied(),
            change(Status::Modified, Some((1, 2)))
        );
        assert_eq!(
            files.get("d.md").copied(),
            change(Status::Deleted, Some((0, 4)))
        );
        // バイナリは、行数を None にする
        assert_eq!(files.get("e.png").copied(), change(Status::Modified, None));
        // モードだけが変わったファイルは、変わっていないとみなす
        assert_eq!(files.get("f.md"), None);
        assert_eq!(
            files.get("g.md").copied(),
            change(Status::Modified, Some((1, 1)))
        );
    }

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(
            shell_quote("docv/slot/guide/design.md"),
            "docv/slot/guide/design.md"
        );
        // ~ と ^ は zsh の glob で意味を持つので囲む
        assert_eq!(shell_quote("HEAD~3"), "'HEAD~3'");
        assert_eq!(shell_quote("docv/free/設計.md"), "docv/free/設計.md");
        assert_eq!(
            shell_quote("docv/free/my notes.md"),
            "'docv/free/my notes.md'"
        );
        assert_eq!(shell_quote("it's.md"), "'it'\\''s.md'");
    }

    #[test]
    fn command_needs_tracked_change() {
        let mut changes = Changes {
            from: "main".to_string(),
            ..Changes::default()
        };
        let added = Change {
            status: Status::Added,
            lines: Some((1, 0)),
        };
        changes.files.insert("a.md".to_string(), added);
        changes.files.insert("new.md".to_string(), added);
        changes.untracked.insert("new.md".to_string());
        assert_eq!(
            changes.command("a.md").as_deref(),
            Some("git diff main -- a.md")
        );
        assert_eq!(changes.command("new.md"), None);
        assert_eq!(changes.command("same.md"), None);
    }
}
