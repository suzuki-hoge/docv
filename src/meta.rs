//! 管理対象のファイルのタイトル、問い、話題、status。frontmatter と docv.yaml の docs の項目から読む。

use serde_yaml_ng::Value;

use crate::config::DocEntry;
use crate::markdown::{Document, Outline};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    /// h1 の文字列。
    pub title: Option<String>,
    pub question: Option<String>,
    pub topics: Vec<String>,
    pub status: Option<String>,
}

/// docv.yaml の docs の項目に question や topics があれば、frontmatter より優先する。
pub fn read(entry: &DocEntry, document: &Document, outline: &Outline) -> Meta {
    let frontmatter = document
        .frontmatter
        .as_ref()
        .and_then(|fm| serde_yaml_ng::from_str::<Value>(&fm.yaml).ok());
    let field = |key: &str| frontmatter.as_ref().and_then(|v| v.get(key));
    let string = |key: &str| field(key).and_then(Value::as_str).map(str::to_string);

    let topics = entry.topics.clone().unwrap_or_else(|| {
        field("topics")
            .and_then(Value::as_sequence)
            .map(|seq| {
                seq.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    });
    Meta {
        title: outline
            .headings
            .iter()
            .find(|h| h.level == 1)
            .map(|h| h.text.clone()),
        question: entry.question.clone().or_else(|| string("question")),
        topics,
        status: string("status"),
    }
}
