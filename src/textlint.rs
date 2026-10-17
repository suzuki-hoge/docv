//! textlint を npx で実行する。使うルールとその値は docv が決め、docv の版ごとに固定する。docs/spec/config.md の 5 章。

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::config::Switch;
use crate::diagnostic::Diagnostic;
use crate::workspace::Project;

/// 使うパッケージ。ルールはプリセットのパッケージから読む。
const PACKAGES: &[&str] = &[
    "textlint@15.8.0",
    "textlint-rule-preset-ja-technical-writing@12.0.2",
    "textlint-rule-preset-ja-spacing@3.0.3",
];

/// docv が決めた textlint のルール 1 つ。
pub struct Rule {
    pub preset: &'static str,
    pub name: &'static str,
    /// ルールに渡す値（JSON）。false なら使わない。
    pub value: &'static str,
    /// 現在の設定のページに出す説明。使うルールは確かめること、使わないルールは確かめないこと。
    pub desc: &'static str,
}

impl Rule {
    pub fn enabled(&self) -> bool {
        self.value != "false"
    }
}

const fn rule(
    preset: &'static str,
    name: &'static str,
    value: &'static str,
    desc: &'static str,
) -> Rule {
    Rule {
        preset,
        name,
        value,
        desc,
    }
}

const TW: &str = "ja-technical-writing";
const SP: &str = "ja-spacing";

/// RULES のプリセット。現在の設定のページで、この順に表を分ける。
pub const PRESETS: &[&str] = &[TW, SP];

/// プリセットのルールを、使わないものも含めてすべて書く。プリセットの既定には頼らない。
pub const RULES: &[Rule] = &[
    rule(
        TW,
        "sentence-length",
        r#"{"max":100}"#,
        "一文を 100 文字以内にする",
    ),
    rule(
        TW,
        "max-comma",
        r#"{"max":3}"#,
        "一文のカンマ（,）を 3 つまでにする",
    ),
    rule(
        TW,
        "max-ten",
        r#"{"max":3}"#,
        "一文の読点（、）を 3 つまでにする",
    ),
    rule(TW, "max-kanji-continuous-len", "false", "漢字が続く長さ"),
    rule(
        TW,
        "no-mix-dearu-desumasu",
        r#"{"preferInHeader":"","preferInBody":"","preferInList":"","strict":false}"#,
        "「である」と「ですます」を混ぜない",
    ),
    rule(
        TW,
        "ja-no-mixed-period",
        r#"{"periodMark":"。"}"#,
        "文末を「。」で終える",
    ),
    rule(
        TW,
        "arabic-kanji-numbers",
        "true",
        "算用数字と漢数字を使い分ける",
    ),
    rule(TW, "no-doubled-conjunction", "true", "同じ接続詞を続けない"),
    rule(
        TW,
        "no-doubled-conjunctive-particle-ga",
        "true",
        "逆接の「が」を一文に 2 回使わない",
    ),
    rule(TW, "no-double-negative-ja", "true", "二重否定を使わない"),
    rule(
        TW,
        "no-doubled-joshi",
        r#"{"min_interval":1}"#,
        "同じ助詞を続けない",
    ),
    rule(TW, "no-dropping-the-ra", "true", "ら抜き言葉を使わない"),
    rule(TW, "no-nfd", "true", "UTF8-MAC の濁点を使わない"),
    rule(TW, "no-invalid-control-character", "false", "制御文字"),
    rule(
        TW,
        "no-zero-width-spaces",
        "true",
        "ゼロ幅スペースを使わない",
    ),
    rule(
        TW,
        "no-exclamation-question-mark",
        "false",
        "感嘆符と疑問符を使うこと",
    ),
    rule(TW, "no-hankaku-kana", "false", "半角カナ"),
    rule(
        TW,
        "ja-no-weak-phrase",
        "false",
        "「かもしれない」などの弱い表現",
    ),
    rule(TW, "ja-no-successive-word", "false", "同じ語の連続"),
    rule(TW, "ja-no-abusage", "true", "よくある誤用を避ける"),
    rule(
        TW,
        "ja-no-redundant-expression",
        "true",
        "冗長な表現を避ける",
    ),
    rule(
        TW,
        "ja-unnatural-alphabet",
        "true",
        "入力ミスで紛れたアルファベットを残さない",
    ),
    rule(TW, "no-unmatched-pair", "true", "括弧を対にする"),
    rule(
        SP,
        "ja-space-between-half-and-full-width",
        r#"{"space":"always"}"#,
        "半角の文字と全角の文字の間にスペースを入れる",
    ),
    rule(
        SP,
        "ja-space-around-code",
        r#"{"before":true,"after":true}"#,
        "インラインコードの前後にスペースを入れる",
    ),
    rule(
        SP,
        "ja-space-around-link",
        r#"{"before":true,"after":true}"#,
        "リンクの前後にスペースを入れる",
    ),
    rule(
        SP,
        "ja-space-around-emphasis",
        r#"{"before":true,"after":true}"#,
        "斜体の前後にスペースを入れる",
    ),
    rule(
        SP,
        "ja-space-around-strong",
        r#"{"before":true,"after":true}"#,
        "太字の前後にスペースを入れる",
    ),
    rule(
        SP,
        "ja-no-space-around-slash",
        "false",
        "スラッシュの前後のスペース",
    ),
    rule(
        SP,
        "ja-no-space-between-full-width",
        "false",
        "全角の文字どうしの間のスペース",
    ),
    rule(
        SP,
        "ja-no-space-around-parentheses",
        "true",
        "全角の括弧と [ ] の内側と外側にスペースを入れない",
    ),
    rule(
        SP,
        "ja-nakaguro-or-halfwidth-space-between-katakana",
        "false",
        "カタカナ語の区切りの文字",
    ),
    rule(
        SP,
        "ja-space-after-exclamation",
        "false",
        "文末の感嘆符の後のスペース",
    ),
    rule(
        SP,
        "ja-space-after-question",
        "false",
        "文末の疑問符の後のスペース",
    ),
];

/// YAML の Slot の Markdown の項目には使わないルール。項目の多くは HTML で表のセルに出す短い説明で、
/// Free ファイルの表のセルと同じく、文末の「。」を求めない（textlint は表のセルを段落として扱わない）。
pub const VIRTUAL_OFF: &[&str] = &["ja-no-mixed-period"];

pub struct Textlint<'a> {
    project: &'a Project,
}

impl<'a> Textlint<'a> {
    /// lint.textlint が enable のときだけ返す。
    pub fn new(project: &'a Project) -> Option<Self> {
        (project.config.lint.textlint == Switch::Enable).then_some(Textlint { project })
    }

    /// files はルートからの相対パス。virtual は、ファイルにない Markdown（YAML の Slot の Markdown の項目）と、
    /// 指摘に付けるパス。(パス, Markdown)。virtual には VIRTUAL_OFF のルールをかけない。
    pub fn lint(
        &self,
        files: &[String],
        virtual_files: &[(String, String)],
    ) -> Result<Vec<Diagnostic>> {
        let virtual_dir = VirtualDir::write(virtual_files)?;
        let mut diagnostics = Vec::new();
        for (paths, off) in [
            (files.iter().map(PathBuf::from).collect::<Vec<_>>(), &[][..]),
            (virtual_dir.paths.clone(), VIRTUAL_OFF),
        ] {
            if paths.is_empty() {
                continue;
            }
            let mut command = self.command(off)?;
            command.args(["--format", "json"]).args(&paths);
            let output = run(command)?;
            let results: Vec<FileResult> =
                serde_json::from_slice(&output.stdout).with_context(|| {
                    format!(
                        "textlint の結果を読めない\n{}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    )
                })?;
            for result in results {
                let path = virtual_dir
                    .original(&result.file_path)
                    .unwrap_or_else(|| self.relative(&result.file_path, files));
                for message in result.messages {
                    diagnostics.push(Diagnostic {
                        path: path.clone(),
                        line: message.line,
                        rule: format!(
                            "textlint/{}",
                            message.rule_id.as_deref().unwrap_or("unknown")
                        ),
                        message: message
                            .message
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" "),
                    });
                }
            }
        }
        Ok(diagnostics)
    }

    /// textlint の自動修正でファイルを書き換える。virtual は lint と同じで、直した Markdown を同じ順に返す。
    pub fn fix(&self, files: &[String], virtual_files: &[(String, String)]) -> Result<Vec<String>> {
        let virtual_dir = VirtualDir::write(virtual_files)?;
        if !files.is_empty() {
            let mut command = self.command(&[])?;
            command.arg("--fix").args(files);
            run(command)?;
        }
        if !virtual_dir.paths.is_empty() {
            let mut command = self.command(VIRTUAL_OFF)?;
            command.arg("--fix").args(&virtual_dir.paths);
            run(command)?;
        }
        virtual_dir
            .paths
            .iter()
            .map(|p| fs::read_to_string(p).with_context(|| format!("{} を読めない", p.display())))
            .collect()
    }

    /// off は、RULES で使うルールのうち、この実行では使わないもの。
    fn command(&self, off: &[&str]) -> Result<Command> {
        let mut command = Command::new(if cfg!(windows) { "npx.cmd" } else { "npx" });
        command.current_dir(&self.project.root);
        command.arg("--yes");
        for package in PACKAGES {
            command.args(["-p", package]);
        }
        let rc = write_builtin_config(&builtin_config(off))?;
        command.arg("textlint").arg("--config").arg(rc);
        Ok(command)
    }

    /// textlint が出す絶対パスを、ルートからの相対パスにする。
    fn relative(&self, file_path: &str, files: &[String]) -> String {
        let path = Path::new(file_path);
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Ok(rel) = path.strip_prefix(&self.project.root) {
            return crate::markdown::slash_path(rel);
        }
        files
            .iter()
            .find(|f| file_path.replace('\\', "/").ends_with(f.as_str()))
            .cloned()
            .unwrap_or_else(|| file_path.to_string())
    }
}

/// ファイルにない Markdown を textlint に渡すために、OS のキャッシュのディレクトリに書いたファイル。
/// リポジトリにはファイルを作らない。使い終わったら消す。
struct VirtualDir {
    dir: Option<PathBuf>,
    /// 書いたファイル。virtual と同じ順。
    paths: Vec<PathBuf>,
    /// 書いたファイルと、指摘に付けるパス。
    originals: Vec<(PathBuf, String)>,
}

impl VirtualDir {
    fn write(virtual_files: &[(String, String)]) -> Result<Self> {
        if virtual_files.is_empty() {
            return Ok(VirtualDir {
                dir: None,
                paths: Vec::new(),
                originals: Vec::new(),
            });
        }
        let dir = dirs::cache_dir()
            .context("OS のキャッシュのディレクトリが分からない")?
            .join("docv")
            .join("textlint")
            .join(format!("virtual-{}", std::process::id()));
        fs::create_dir_all(&dir).with_context(|| format!("{} を作れない", dir.display()))?;
        let dir = dir.canonicalize().unwrap_or(dir);
        let mut paths = Vec::new();
        let mut originals = Vec::new();
        for (i, (path, text)) in virtual_files.iter().enumerate() {
            let file = dir.join(format!("{i}.md"));
            fs::write(&file, text).with_context(|| format!("{} を書けない", file.display()))?;
            paths.push(file.clone());
            originals.push((file, path.clone()));
        }
        Ok(VirtualDir {
            dir: Some(dir),
            paths,
            originals,
        })
    }

    /// textlint が出したパスが、書いたファイルなら、指摘に付けるパスを返す。
    fn original(&self, file_path: &str) -> Option<String> {
        let path = Path::new(file_path);
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.originals
            .iter()
            .find(|(file, _)| *file == path)
            .map(|(_, original)| original.clone())
    }
}

impl Drop for VirtualDir {
    fn drop(&mut self) {
        if let Some(dir) = &self.dir {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

#[derive(Deserialize)]
struct FileResult {
    #[serde(rename = "filePath")]
    file_path: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(rename = "ruleId")]
    rule_id: Option<String>,
    message: String,
    line: usize,
}

/// textlint の設定ファイルの中身。RULES をプリセットごとにまとめる。off のルールは使わない。
fn builtin_config(off: &[&str]) -> Value {
    let mut rules = Map::new();
    for r in RULES {
        let value: Value = if off.contains(&r.name) {
            Value::Bool(false)
        } else {
            serde_json::from_str(r.value).expect("RULES の値は JSON として正しい")
        };
        let preset = rules
            .entry(format!("preset-{}", r.preset))
            .or_insert_with(|| Value::Object(Map::new()));
        preset
            .as_object_mut()
            .expect("プリセットの値はオブジェクト")
            .insert(r.name.to_string(), value);
    }
    json!({ "rules": rules })
}

/// 設定ファイルを OS のキャッシュのディレクトリに書く。リポジトリにはファイルを作らない。
fn write_builtin_config(value: &Value) -> Result<PathBuf> {
    let text = serde_json::to_string_pretty(value)?;
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let dir = dirs::cache_dir()
        .context("OS のキャッシュのディレクトリが分からない")?
        .join("docv")
        .join("textlint");
    let path = dir.join(format!("{:016x}.json", hasher.finish()));
    if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
        fs::create_dir_all(&dir).with_context(|| format!("{} を作れない", dir.display()))?;
        fs::write(&path, text).with_context(|| format!("{} を書けない", path.display()))?;
    }
    Ok(path)
}

/// textlint は、問題があると終了コード 1 で終わる。それより大きい終了コードは失敗として扱う。
fn run(mut command: Command) -> Result<Output> {
    let output = command.output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            anyhow!(
                "lint.textlint が enable だが、npx が見つからない。Node.js を入れるか、--no-textlint で textlint を飛ばす"
            )
        } else {
            anyhow!(e).context("npx を実行できない")
        }
    })?;
    match output.status.code() {
        Some(0 | 1) => Ok(output),
        _ => bail!(
            "textlint が失敗した\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_builtin_config() {
        let config = builtin_config(&[]);
        let rules = &config["rules"];
        assert_eq!(
            rules["preset-ja-technical-writing"]["sentence-length"],
            json!({ "max": 100 })
        );
        assert_eq!(
            rules["preset-ja-technical-writing"]["no-exclamation-question-mark"],
            json!(false)
        );
        assert_eq!(
            rules["preset-ja-spacing"]["ja-space-around-code"],
            json!({ "before": true, "after": true })
        );
        assert_eq!(rules.as_object().unwrap().len(), 2);
        assert!(RULES.iter().all(|r| !r.desc.is_empty()));
        let virtual_config = builtin_config(VIRTUAL_OFF);
        assert_eq!(
            virtual_config["rules"]["preset-ja-technical-writing"]["ja-no-mixed-period"],
            json!(false)
        );
    }

    #[test]
    fn reads_textlint_json() {
        let results: Vec<FileResult> = serde_json::from_str(
            r#"[{"filePath":"/repo/a.md","messages":[{"type":"lint","ruleId":"ja-spacing/x","message":"m\n n","line":3,"severity":2}]}]"#,
        )
        .unwrap();
        assert_eq!(
            results[0].messages[0].rule_id.as_deref(),
            Some("ja-spacing/x")
        );
        assert_eq!(results[0].messages[0].line, 3);
    }
}
