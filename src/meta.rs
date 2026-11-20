//! 管理対象のファイルのタイトル、ミッション、タグと、ページに関わる決定。YAML front matter と h1 から読む。

use serde_yaml_ng::Value;

use crate::decision::Kind;
use crate::markdown::{Document, Outline};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    /// h1 の文字列。
    pub title: Option<String>,
    /// YAML front matter の mission。このファイルを読むと何が分かるか。
    pub mission: Option<String>,
    pub tags: Vec<String>,
    pub related: Related,
}

/// YAML front matter の adr と pdr に書いた、ページに関わる決定の id。docv/slot/service/service/free.md の「関わる決定」。
/// Free ファイルにも Slot（分けたページも）にも書ける。key は Kind::dir と同じ。
/// それぞれ書いた順に持ち、重なりは除く。型の誤りは check が報告するので、文字列でないものは飛ばす。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Related {
    pub adr: Vec<String>,
    pub pdr: Vec<String>,
}

impl Related {
    pub fn ids(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::Adr => &self.adr,
            Kind::Pdr => &self.pdr,
        }
    }
}

pub fn read(document: &Document, outline: &Outline) -> Meta {
    let frontmatter = frontmatter(document);
    let field = |key: &str| frontmatter.as_ref().and_then(|v| v.get(key));
    Meta {
        title: outline
            .headings
            .iter()
            .find(|h| h.level == 1)
            .map(|h| h.text.clone()),
        mission: field("mission").and_then(Value::as_str).map(str::to_string),
        tags: strings(field("tags")),
        related: related_of(frontmatter.as_ref()),
    }
}

/// YAML front matter の adr か pdr の値（value）が文字列の配列でなければ、直し方を書いたメッセージ。なければ None。
/// check が、Free ファイルなら frontmatter-invalid に、Slot なら slot-invalid にする。
/// 数や真偽値の要素も誤りにする。YAML のパーサーによっては文字列として読めるが、id はファイル名なので文字列で書く。
pub fn related_type_problem(kind: Kind, value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Sequence(ids) if ids.iter().all(Value::is_string) => None,
        _ => Some(format!(
            "{} は {} の id（ファイル名から .md を除いたもの）の配列で書く",
            kind.dir(),
            kind.label()
        )),
    }
}

/// YAML front matter の adr と pdr。YAML front matter がないか読めなければ、空。
pub fn related(document: &Document) -> Related {
    related_of(frontmatter(document).as_ref())
}

fn frontmatter(document: &Document) -> Option<Value> {
    document
        .frontmatter
        .as_ref()
        .and_then(|fm| serde_yaml_ng::from_str::<Value>(&fm.yaml).ok())
}

fn related_of(frontmatter: Option<&Value>) -> Related {
    let ids = |kind: Kind| {
        let mut ids: Vec<String> = Vec::new();
        for id in strings(frontmatter.and_then(|v| v.get(kind.dir()))) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    };
    Related {
        adr: ids(Kind::Adr),
        pdr: ids(Kind::Pdr),
    }
}

/// 配列の中の文字列。配列でなければ空。
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown;

    fn related_in(text: &str) -> Related {
        related(&markdown::split(text))
    }

    #[test]
    fn reads_related_decisions_in_the_written_order() {
        let text =
            "---\nmission: q\ntags: [仕様]\nadr: [rust, go, rust]\npdr:\n  - paid\n---\n\n# A\n";
        let document = markdown::split(text);
        let meta = read(&document, &markdown::outline(&document));
        assert_eq!(meta.tags, ["仕様"]);
        // 重なりは除く
        assert_eq!(meta.related.adr, ["rust", "go"]);
        assert_eq!(meta.related.ids(Kind::Pdr), ["paid"]);
        assert_eq!(related_in(text), meta.related);
    }

    #[test]
    fn skips_related_decisions_of_wrong_types() {
        // 型の誤りは check が報告するので、読めるものだけを読む
        let related = related_in("---\nadr: rust\npdr: [paid, 1]\n---\n");
        assert!(related.adr.is_empty());
        assert_eq!(related.pdr, ["paid"]);
        assert_eq!(related_in("# A\n"), Related::default());
        assert_eq!(related_in("---\nadr: [\n---\n"), Related::default());
    }
}
