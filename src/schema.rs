//! docv schema。docv.yaml と、yaml の Slot のスキーマを出す。docv/service/manual.md の docv schema。
//!
//! Slot の名前は docv add と同じ（API 仕様書と DB 仕様書は `api-yaml`、`db-yaml`）。md の Slot はスキーマを持たない。
//! 同じスキーマを docs/schema/ の Markdown に写して公開する。写しが元と揃っていることは、このファイルのテストで確かめる。

use anyhow::{Result, bail};

use crate::{add, config, slot};

/// slot を省略したら docv.yaml のスキーマ（JSON）、指定したらその Slot のスキーマ（YAML）を返す。
pub fn text(slot: Option<&str>) -> Result<&'static str> {
    let Some(name) = slot else {
        return Ok(config::SCHEMA);
    };
    let Some((_, def, ext)) = add::pages().into_iter().find(|(p, ..)| p == name) else {
        bail!("{name} は Slot の名前ではない。docv add --list で Slot の一覧を出す");
    };
    if ext != "yaml" {
        bail!("{name} は md の Slot で、スキーマはない。中身の形は決まっていない");
    }
    Ok(slot::schema(def.id).expect("yaml の Slot にはスキーマがある"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::Value;

    use super::*;

    fn validator(text: &str) -> jsonschema::Validator {
        let schema: Value = serde_yaml_ng::from_str(text).unwrap();
        jsonschema::validator_for(&schema).unwrap()
    }

    fn is_valid(schema: &str, yaml: &str) -> bool {
        let value: Value = serde_yaml_ng::from_str(yaml).unwrap();
        validator(schema).is_valid(&value)
    }

    /// yaml の Slot の、docv schema に渡す名前と定義。
    fn yaml_pages() -> Vec<(String, &'static slot::Def)> {
        add::pages()
            .into_iter()
            .filter(|(_, _, ext)| *ext == "yaml")
            .map(|(page, def, _)| (page, def))
            .collect()
    }

    #[test]
    fn every_yaml_slot_has_a_schema_named_after_it() {
        for (page, def) in yaml_pages() {
            let text = text(Some(&page)).unwrap();
            let schema: Value = serde_yaml_ng::from_str(text).unwrap();
            assert_eq!(
                schema["title"],
                format!("{}（{}）", def.label, def.file_name()),
                "{page}"
            );
            assert_eq!(schema["description"], def.question, "{page}");
            validator(text);
        }
    }

    #[test]
    fn templates_and_this_repository_follow_the_schemas() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (page, def) in yaml_pages() {
            let schema = text(Some(&page)).unwrap();
            let template = slot::template(def.id).unwrap();
            assert!(is_valid(schema, template), "templates: {page}");
            if let Ok(own) = fs::read_to_string(root.join("docv").join(def.file_name())) {
                assert!(is_valid(schema, &own), "docv/: {page}");
            }
        }
    }

    #[test]
    fn rejects_what_the_slot_does_not_accept() {
        let words = text(Some("words")).unwrap();
        assert!(!is_valid(
            words,
            "groups:\n  - name: a\n    items:\n      - desc: x\n"
        ));
        assert!(!is_valid(words, "unknown: 1\n"));
        let install = text(Some("install")).unwrap();
        assert!(is_valid(
            install,
            "steps:\n  - id: a\n    name: A\n    items:\n      markdown: x\n"
        ));
        assert!(!is_valid(install, "steps:\n  - id: a\n"));
    }

    #[test]
    fn rejects_md_and_unknown_slots() {
        assert!(text(None).unwrap().contains("\"docs\""));
        for (name, message) in [
            ("service", "md の Slot"),
            ("api-md", "md の Slot"),
            ("manual", "md の Slot"),
            ("api", "Slot の名前ではない"),
        ] {
            let error = text(Some(name)).unwrap_err().to_string();
            assert!(error.contains(message), "{name}: {error}");
        }
        assert!(text(Some("api-yaml")).is_ok());
    }

    #[test]
    fn docs_publish_copies_of_the_schemas() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut copies = vec![("config".to_string(), "json", config::SCHEMA)];
        for (page, _) in yaml_pages() {
            let schema = text(Some(&page)).unwrap();
            copies.push((page, "yaml", schema));
        }
        for (name, lang, schema) in copies {
            let path = root.join(format!("docs/schema/{name}.md"));
            let published = fs::read_to_string(&path).unwrap_or_default();
            assert!(
                published.contains(&format!("```{lang}\n{schema}```\n")),
                "{} が {name} のスキーマと揃っていない",
                path.display()
            );
        }
    }
}
