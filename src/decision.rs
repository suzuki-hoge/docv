//! 決定の記録（ADR と PDR）。docv/service/service/adr.md と docv/service/service/pdr.md。
//!
//! ADR（Architecture Decision Record）は技術の決定を、PDR（Product Decision Record）はプロダクトやサービスの決定を残す。
//! 2 つは、置き場所、check のルールの ID、HTML のページだけが違い、形と扱いは同じ。
//! dir.slot の下の adr/ か pdr/ に、決定 1 つを 1 つの Markdown で置く。ファイル名（.md を除く）が id。
//! システムが使う項目は YAML front matter に、決定の中身（背景、決めたこと、選ばなかった案など）は本文に書く。題は本文の h1。
//! 状態は持たない。打ち消されたかどうかは、つながりからわかる。
//! 過去の決定を打ち消すときは、新しい決定の supersedes から、同じ種類の過去の決定の id を指す（未来から過去を参照する）。
//! 過去から未来へのつながりは、読み込むときに組み立てる。

use std::collections::{BTreeMap, HashSet};
use std::fs;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::diagnostic::Diagnostic;
use crate::markdown;
use crate::workspace::Project;

/// 決定の記録の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 技術の決定（Architecture Decision Record）。
    Adr,
    /// プロダクトやサービスの決定（Product Decision Record）。
    Pdr,
}

impl Kind {
    /// HTML のサイドバーと、インデックスに並べる順。
    pub const ALL: [Kind; 2] = [Kind::Adr, Kind::Pdr];

    /// 表示名。HTML のページの名前と、出力先のディレクトリの名前にも使う。
    pub fn label(self) -> &'static str {
        match self {
            Kind::Adr => "ADR",
            Kind::Pdr => "PDR",
        }
    }

    /// 置くディレクトリの名前（dir.slot からの相対パス）。check のルールの ID の頭と、アイコンの名前にも使う。
    pub fn dir(self) -> &'static str {
        match self {
            Kind::Adr => "adr",
            Kind::Pdr => "pdr",
        }
    }

    /// check のルールの ID。`adr-invalid`、`pdr-invalid` など。
    fn rule(self, name: &str) -> String {
        format!("{}-{name}", self.dir())
    }
}

/// group を書かなかった決定のまとまり。
pub const NO_GROUP: &str = "その他";

/// 決定の YAML front matter。ADR と PDR で同じ。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Front {
    /// まとまり。同じ group の決定を、HTML で 1 つの図にまとめる。
    pub group: Option<String>,
    /// 決めた日（YYYY-MM-DD）。
    pub date: String,
    /// 打ち消す過去の決定の id。
    #[serde(default)]
    pub supersedes: Vec<String>,
}

/// 決定のファイルの中身。
#[derive(Debug, Clone)]
pub struct Record {
    /// ファイル名から .md を除いたもの。
    pub id: String,
    /// 本文の h1。
    pub title: String,
    pub group: Option<String>,
    pub date: String,
    pub supersedes: Vec<String>,
    /// YAML front matter を除いた本文（Markdown）。
    pub body: String,
}

/// 決定のファイルを読む。id はファイル名から .md を除いたもの。読めないときは、その理由（行、メッセージ）。
pub fn parse(kind: Kind, id: &str, text: &str) -> Result<Record, (usize, String)> {
    let document = markdown::split(text);
    let Some(fm) = &document.frontmatter else {
        return Err((1, "YAML front matter がない".to_string()));
    };
    let front: Front = serde_yaml_ng::from_str(&fm.yaml).map_err(|e| {
        (
            e.location().map_or(1, |l| l.line() + 1),
            format!("YAML front matter が {} の形に合わない: {e}", kind.label()),
        )
    })?;
    let outline = markdown::outline(&document);
    let Some(h1) = outline.headings.iter().find(|h| h.level == 1) else {
        return Err((document.body_start_line, "題（h1）がない".to_string()));
    };
    Ok(Record {
        id: id.to_string(),
        title: h1.text.clone(),
        group: front.group,
        date: front.date,
        supersedes: front.supersedes,
        body: document.body.clone(),
    })
}

/// 読み込んだ決定。superseded_by は、読み込むときに supersedes を逆にたどって組み立てる。
#[derive(Debug, Clone)]
pub struct Decision {
    pub kind: Kind,
    /// ルートからの相対パス。
    pub path: String,
    pub record: Record,
    /// この決定を打ち消した、新しい決定の id。
    pub superseded_by: Vec<String>,
}

impl Decision {
    pub fn group(&self) -> &str {
        self.record.group.as_deref().unwrap_or(NO_GROUP)
    }
}

/// 決定を置くディレクトリ（ルートからの相対パス）。
pub fn dir(project: &Project, kind: Kind) -> String {
    format!("{}/{}", project.slot_dir(), kind.dir())
}

/// 決定のパス（ルートからの相対パス）から id を取る。
fn id_of(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

/// adr/ か pdr/ の直下の .md のパス（ルートからの相対パス）を、名前の順に返す。
/// 除外するファイルと、名前がドットで始まるファイルは含まない（Project::slot_markdown）。
fn files(project: &Project, kind: Kind) -> Result<Vec<String>> {
    let prefix = format!("{}/", dir(project, kind));
    Ok(project
        .slot_markdown()?
        .into_iter()
        .filter(|path| {
            path.strip_prefix(&prefix)
                .is_some_and(|name| !name.contains('/'))
        })
        .collect())
}

/// 1 つの種類の決定を、日付と id の順に読む。読めないものは飛ばす（check が報告する）。
/// supersedes は、同じ種類の決定だけをたどる。
pub fn load(project: &Project, kind: Kind) -> Result<Vec<Decision>> {
    let mut adrs: Vec<Decision> = Vec::new();
    for path in files(project, kind)? {
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        let Ok(record) = parse(kind, id_of(&path), &text) else {
            continue;
        };
        adrs.push(Decision {
            kind,
            path,
            record,
            superseded_by: Vec::new(),
        });
    }
    adrs.sort_by(|a, b| (&a.record.date, &a.record.id).cmp(&(&b.record.date, &b.record.id)));
    let mut later: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for adr in &adrs {
        for past in &adr.record.supersedes {
            later
                .entry(past.clone())
                .or_default()
                .push(adr.record.id.clone());
        }
    }
    for adr in &mut adrs {
        adr.superseded_by = later.remove(&adr.record.id).unwrap_or_default();
    }
    Ok(adrs)
}

/// まとまりごとに、決定のつながりを左から右（過去から未来）へ並べた列にする。
/// 1 つの列は、つながりのない決定から始め、未来の決定をたどる。分かれたつながりは、新しい列にする。
pub fn lanes<'a>(adrs: &[&'a Decision]) -> Vec<Vec<&'a Decision>> {
    let ids: HashSet<&str> = adrs.iter().map(|a| a.record.id.as_str()).collect();
    let has_past = |a: &Decision| a.record.supersedes.iter().any(|p| ids.contains(p.as_str()));
    let mut placed: HashSet<&str> = HashSet::new();
    let mut lanes = Vec::new();
    let starts = adrs
        .iter()
        .filter(|a| !has_past(a))
        .chain(adrs.iter().filter(|a| has_past(a)));
    for start in starts {
        if placed.contains(start.record.id.as_str()) {
            continue;
        }
        let mut lane = vec![*start];
        placed.insert(&start.record.id);
        let mut current = *start;
        while let Some(next) = adrs.iter().find(|a| {
            current.superseded_by.contains(&a.record.id) && !placed.contains(a.record.id.as_str())
        }) {
            placed.insert(&next.record.id);
            lane.push(*next);
            current = next;
        }
        lanes.push(lane);
    }
    lanes
}

/// まとまりの名前を、いちばん古い決定の順に返す。
pub fn groups(adrs: &[Decision]) -> Vec<&str> {
    let mut groups: Vec<&str> = Vec::new();
    for adr in adrs {
        if !groups.contains(&adr.group()) {
            groups.push(adr.group());
        }
    }
    groups
}

/// ADR と PDR のファイルを確かめる。
pub fn check(project: &Project) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    for kind in Kind::ALL {
        diagnostics.extend(check_kind(project, kind)?);
    }
    Ok(diagnostics)
}

/// 1 つの種類の決定のファイルを確かめる。supersedes は、同じ種類の決定を指しているかを確かめる。
fn check_kind(project: &Project, kind: Kind) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut push = |path: &str, line: usize, rule: &str, message: String| {
        diagnostics.push(Diagnostic {
            path: path.to_string(),
            line,
            rule: rule.to_string(),
            message,
        });
    };
    let mut records: Vec<(String, String, Record)> = Vec::new();
    for path in files(project, kind)? {
        let text = fs::read_to_string(project.root.join(&path))
            .with_context(|| format!("{path} を読めない"))?;
        match parse(kind, id_of(&path), &text) {
            Ok(record) => records.push((path, text, record)),
            Err((line, message)) => push(&path, line, &kind.rule("invalid"), message),
        }
    }
    let line_of = |text: &str, needle: &str| {
        text.lines()
            .position(|l| l.contains(needle))
            .map_or(1, |i| i + 1)
    };
    for (path, text, record) in &records {
        if !is_date(&record.date) {
            push(
                path,
                line_of(text, "date:"),
                &kind.rule("invalid"),
                "date は YYYY-MM-DD で書く".to_string(),
            );
        }
        for past in &record.supersedes {
            let line = line_of(text, past);
            match records.iter().find(|(_, _, r)| &r.id == past) {
                None => push(
                    path,
                    line,
                    &kind.rule("unknown-ref"),
                    format!("supersedes の {past} という {} がない", kind.label()),
                ),
                Some((_, _, r)) if r.date > record.date || r.id == record.id => push(
                    path,
                    line,
                    &kind.rule("unknown-ref"),
                    format!(
                        "supersedes には、この決定より前の決定を書く（{past} は {}）",
                        r.date
                    ),
                ),
                Some(_) => {}
            }
        }
    }
    Ok(diagnostics)
}

fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
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

    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docv.yaml", "{}\n");
        for (path, text) in files {
            write(dir.path(), path, text);
        }
        let project = Project::open(&Locate::default(), dir.path()).unwrap();
        (dir, project)
    }

    fn adr(id: &str, group: &str, date: &str, supersedes: &[&str]) -> String {
        format!(
            "---\ngroup: {group}\ndate: {date}\nsupersedes: [{}]\n---\n\n# {id} にする\n\n{id} で書く\n",
            supersedes.join(", ")
        )
    }

    #[test]
    fn links_past_to_future_and_makes_lanes() {
        let (_dir, project) = project(&[
            ("docv/adr/go.md", &adr("go", "言語", "2026-01-01", &[])),
            (
                "docv/adr/rust.md",
                &adr("rust", "言語", "2026-03-01", &["go"]),
            ),
            (
                "docv/adr/react.md",
                &adr("react", "言語", "2026-02-01", &[]),
            ),
            (
                "docv/adr/gcp.md",
                &adr("gcp", "インフラ", "2026-01-15", &[]),
            ),
        ]);
        let adrs = load(&project, Kind::Adr).unwrap();
        assert_eq!(groups(&adrs), ["言語", "インフラ"]);
        let go = adrs.iter().find(|a| a.record.id == "go").unwrap();
        assert_eq!(go.superseded_by, ["rust"]);
        assert_eq!(go.record.title, "go にする");
        let language: Vec<&Decision> = adrs.iter().filter(|a| a.group() == "言語").collect();
        let lanes: Vec<Vec<&str>> = lanes(&language)
            .iter()
            .map(|l| l.iter().map(|a| a.record.id.as_str()).collect())
            .collect();
        assert_eq!(lanes, [vec!["go", "rust"], vec!["react"]]);
    }

    #[test]
    fn reports_problems() {
        let (_dir, project) = project(&[
            (
                "docv/adr/a.md",
                &adr("a", "g", "2026-01-01", &["b", "none"]),
            ),
            ("docv/adr/b.md", &adr("b", "g", "2026-02-01", &[])),
            (
                "docv/adr/c.md",
                "---\nid: c\ndate: 2026-01-01\n---\n\n# C\n",
            ),
            ("docv/adr/d.md", "---\ngroup: g\n---\n\n# D\n"),
            ("docv/adr/e.md", "---\ndate: 2026-01-01\n---\n\n本文だけ\n"),
        ]);
        let result: Vec<String> = check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{}:{} {}", d.path, d.line, d.rule))
            .collect();
        assert_eq!(
            result,
            [
                "docv/adr/c.md:2 adr-invalid",
                "docv/adr/d.md:2 adr-invalid",
                "docv/adr/e.md:4 adr-invalid",
                "docv/adr/a.md:4 adr-unknown-ref",
                "docv/adr/a.md:4 adr-unknown-ref",
            ]
        );
    }

    #[test]
    fn reads_pdr_apart_from_adr() {
        let (_dir, project) = project(&[
            ("docv/adr/go.md", &adr("go", "言語", "2026-01-01", &[])),
            ("docv/pdr/free.md", &adr("free", "料金", "2026-01-01", &[])),
            (
                "docv/pdr/paid.md",
                &adr("paid", "料金", "2026-02-01", &["free"]),
            ),
            // 同じ id でも、ADR と PDR は別の決定
            ("docv/pdr/go.md", &adr("go", "料金", "2026-03-01", &[])),
        ]);
        let pdrs = load(&project, Kind::Pdr).unwrap();
        let ids: Vec<(&str, Kind)> = pdrs
            .iter()
            .map(|p| (p.record.id.as_str(), p.kind))
            .collect();
        assert_eq!(
            ids,
            [("free", Kind::Pdr), ("paid", Kind::Pdr), ("go", Kind::Pdr)]
        );
        assert_eq!(pdrs[0].superseded_by, ["paid"]);
        let adrs = load(&project, Kind::Adr).unwrap();
        assert_eq!(adrs.len(), 1);
        assert!(adrs[0].superseded_by.is_empty());
    }

    #[test]
    fn reports_problems_of_pdr_with_its_own_rules() {
        let (_dir, project) = project(&[
            ("docv/adr/go.md", &adr("go", "言語", "2026-01-01", &[])),
            // PDR の supersedes は PDR だけを指す。ADR の id は見つからない
            ("docv/pdr/a.md", &adr("a", "料金", "2026-02-01", &["go"])),
            (
                "docv/pdr/b.md",
                "---
group: 料金
---

# B
",
            ),
        ]);
        let result: Vec<String> = check(&project)
            .unwrap()
            .iter()
            .map(|d| format!("{}:{} {} {}", d.path, d.line, d.rule, d.message))
            .collect();
        assert_eq!(
            result,
            [
                "docv/pdr/b.md:2 pdr-invalid YAML front matter が PDR の形に合わない: missing field `date`",
                "docv/pdr/a.md:4 pdr-unknown-ref supersedes の go という PDR がない",
            ]
        );
    }
}
