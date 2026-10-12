//! 管理対象のファイルのタイトル、ミッション、タグ。frontmatter と h1 から読む。

use serde_yaml_ng::Value;

use crate::markdown::{Document, Outline};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    /// h1 の文字列。
    pub title: Option<String>,
    /// frontmatter の mission。このファイルが答えること。
    pub mission: Option<String>,
    pub tags: Vec<String>,
}

pub fn read(document: &Document, outline: &Outline) -> Meta {
    let frontmatter = document
        .frontmatter
        .as_ref()
        .and_then(|fm| serde_yaml_ng::from_str::<Value>(&fm.yaml).ok());
    let field = |key: &str| frontmatter.as_ref().and_then(|v| v.get(key));
    Meta {
        title: outline
            .headings
            .iter()
            .find(|h| h.level == 1)
            .map(|h| h.text.clone()),
        mission: field("mission").and_then(Value::as_str).map(str::to_string),
        tags: field("tags")
            .and_then(Value::as_sequence)
            .map(|seq| {
                seq.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    }
}
