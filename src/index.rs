//! docv index。AI Agent 向けに、管理対象のファイルの一覧と問いを出す。docs/spec/cli.md の 4 章。

use std::fmt::Write as _;
use std::fs;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::kind::Time;
use crate::markdown;
use crate::meta;
use crate::workspace::{Class, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Md,
    Json,
}

#[derive(Debug, Serialize)]
pub struct Index {
    pub title: String,
    /// 時間（現在、未来、過去）、種類、パスの順に並べる。過去は --all のときだけ入る。
    pub files: Vec<Entry>,
    pub archive: Archive,
    pub unmanaged: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Entry {
    pub path: String,
    pub time: &'static str,
    pub kind: String,
    pub question: Option<String>,
    pub topics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Archive {
    pub dir: String,
    /// 過去のファイルの数。
    pub count: usize,
}

pub fn collect(project: &Project, all: bool) -> Result<Index> {
    let mut entries = Vec::new();
    let mut unmanaged = Vec::new();
    let mut past = 0;
    for file in project.files()? {
        let index = match file.class {
            Class::Managed(index) => index,
            Class::Unmanaged => {
                unmanaged.push(file.path);
                continue;
            }
        };
        let kind = project.kind_of(index);
        match kind.time {
            Time::Untimed => continue,
            Time::Past => {
                past += 1;
                if !all {
                    continue;
                }
            }
            Time::Present | Time::Future => {}
        }
        let text = fs::read_to_string(project.root.join(&file.path))
            .with_context(|| format!("{} を読めない", file.path))?;
        let document = markdown::split(&text);
        let meta = meta::read(
            project.entry(index),
            &document,
            &markdown::outline(&document),
        );
        entries.push((
            kind.time,
            kind_order(project, &kind.id),
            Entry {
                path: file.path,
                time: time_id(kind.time),
                kind: kind.id.clone(),
                question: meta.question,
                topics: meta.topics,
                status: meta.status,
            },
        ));
    }
    entries.sort_by(|a, b| (a.0, a.1, &a.2.path).cmp(&(b.0, b.1, &b.2.path)));
    Ok(Index {
        title: project.title(),
        files: entries.into_iter().map(|(_, _, e)| e).collect(),
        archive: Archive {
            dir: project.config.archive.dir.trim_end_matches('/').to_string(),
            count: past,
        },
        unmanaged,
    })
}

fn kind_order(project: &Project, id: &str) -> usize {
    project
        .kinds
        .all()
        .iter()
        .position(|k| k.id == id)
        .unwrap_or(usize::MAX)
}

fn time_id(time: Time) -> &'static str {
    match time {
        Time::Present => "present",
        Time::Future => "future",
        Time::Past => "past",
        Time::Untimed => "untimed",
    }
}

pub fn render(index: &Index, format: Format) -> Result<String> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(index)? + "\n"),
        Format::Md => Ok(render_md(index)),
    }
}

fn render_md(index: &Index) -> String {
    let mut out = format!("# docv index: {}\n", index.title);
    for time in [Time::Present, Time::Future, Time::Past] {
        let entries: Vec<&Entry> = index
            .files
            .iter()
            .filter(|e| e.time == time_id(time))
            .collect();
        if time == Time::Past && entries.is_empty() {
            if index.archive.count > 0 {
                let _ = write!(
                    out,
                    "\n## 過去\n- {}/ に {} ファイル。経緯を聞かれたときだけ読む\n",
                    index.archive.dir, index.archive.count
                );
            }
            continue;
        }
        if entries.is_empty() {
            continue;
        }
        let _ = writeln!(out, "\n## {}", time.label());
        let mut kind = "";
        for entry in entries {
            if entry.kind != kind {
                kind = &entry.kind;
                let _ = writeln!(out, "### {kind}");
            }
            let _ = write!(out, "- {}", entry.path);
            if let Some(status) = &entry.status {
                let _ = write!(out, " ({status})");
            }
            if let Some(question) = &entry.question {
                let _ = write!(out, " — {question}");
            }
            if !entry.topics.is_empty() {
                let _ = write!(out, " [{}]", entry.topics.join(", "));
            }
            out.push('\n');
        }
    }
    if !index.unmanaged.is_empty() {
        out.push_str("\n## 管理外\n");
        for path in &index.unmanaged {
            let _ = writeln!(out, "- {path}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::workspace::Locate;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "docv.yaml",
            "version: 1\ntitle: demo\ntopics: [注文, 在庫]\ndocs:\n  - path: README.md\n    kind: overview\n    question: これは何か\n  - path: docs/spec/*.md\n    kind: spec\n  - path: docs/cost/*.md\n    kind: cost\n  - path: docs/plan/*.md\n    kind: plan\n  - path: docs/archive/**/*.md\n    kind: archive\n  - path: docs/tmp/*.md\n    kind: tmp\n",
        );
        write(root, "README.md", "# R\n");
        write(
            root,
            "docs/spec/order.md",
            "---\nquestion: 注文の処理はどう動くか\ntopics: [注文, 在庫]\n---\n# P\n",
        );
        write(
            root,
            "docs/cost/payment.md",
            "---\nquestion: 何にいくらかかるか\n---\n# L\n",
        );
        write(
            root,
            "docs/plan/011.md",
            "---\nquestion: 何をやるか\nstatus: doing\n---\n# 011\n",
        );
        write(
            root,
            "docs/archive/spec/a.md",
            "---\nquestion: 何を外したか\n---\n# A\n",
        );
        write(root, "docs/archive/spec/b.md", "# B\n");
        write(root, "docs/tmp/memo.md", "# M\n");
        write(root, "docs/memo.md", "# M\n");
        let project = Project::open(&Locate::default(), root).unwrap();
        (dir, project)
    }

    #[test]
    fn renders_markdown() {
        let (_dir, project) = project();
        let index = collect(&project, false).unwrap();
        assert_eq!(
            render(&index, Format::Md).unwrap(),
            "# docv index: demo

## 現在
### overview
- README.md — これは何か
### spec
- docs/spec/order.md — 注文の処理はどう動くか [注文, 在庫]
### cost
- docs/cost/payment.md — 何にいくらかかるか

## 未来
### plan
- docs/plan/011.md (doing) — 何をやるか

## 過去
- docs/archive/ に 2 ファイル。経緯を聞かれたときだけ読む

## 管理外
- docs/memo.md
"
        );
    }

    #[test]
    fn lists_past_files_with_all_and_renders_json() {
        let (_dir, project) = project();
        let index = collect(&project, true).unwrap();
        let md = render(&index, Format::Md).unwrap();
        assert!(
            md.contains("## 過去\n### archive\n- docs/archive/spec/a.md — 何を外したか\n- docs/archive/spec/b.md\n"),
            "{md}"
        );
        let json: serde_json::Value =
            serde_json::from_str(&render(&index, Format::Json).unwrap()).unwrap();
        assert_eq!(json["files"][3]["status"], "doing");
        assert_eq!(json["files"][0]["time"], "present");
        assert_eq!(json["archive"]["count"], 2);
        assert_eq!(json["unmanaged"][0], "docs/memo.md");
        assert!(json["files"][0].get("status").is_none());
    }
}
