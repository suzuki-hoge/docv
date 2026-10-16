//! docv slot schema。yaml の Slot のスキーマを出す。docv/service/manual.md の docv slot schema。
//!
//! md の Slot はスキーマを持たない。同じスキーマと `docv.yaml` のスキーマを docs/schema/ の Markdown に写して公開する。
//! 写しが元と揃っていることは、このファイルのテストで確かめる。

use anyhow::{Result, bail};

use crate::generate;
use crate::slot::{self, Format};

/// yaml の Slot のスキーマ（YAML）を返す。
pub fn text(name: &str) -> Result<&'static str> {
    let def = generate::find(name)?;
    if def.format != Format::Yaml {
        bail!("{name} は md の Slot で、スキーマはない。中身の形は決めていない");
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

    /// yaml の Slot の定義。
    fn yaml_defs() -> Vec<&'static slot::Def> {
        slot::DEFS
            .iter()
            .filter(|d| d.format == Format::Yaml)
            .collect()
    }

    #[test]
    fn every_yaml_slot_has_a_schema_named_after_it() {
        for def in yaml_defs() {
            let text = text(def.id).unwrap();
            let schema: Value = serde_yaml_ng::from_str(text).unwrap();
            assert_eq!(
                schema["title"],
                format!("{}（{}）", def.label, def.file_name()),
                "{}",
                def.id
            );
            assert_eq!(schema["description"], def.question, "{}", def.id);
            validator(text);
        }
    }

    #[test]
    fn templates_and_this_repository_follow_the_schemas() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for def in yaml_defs() {
            let schema = text(def.id).unwrap();
            let template = slot::template(def.id).unwrap();
            assert!(is_valid(schema, template), "assets/template: {}", def.id);
            if let Ok(own) = fs::read_to_string(root.join("docv").join(def.file_name())) {
                assert!(is_valid(schema, &own), "docv/: {}", def.id);
            }
        }
    }

    #[test]
    fn rejects_what_the_slot_does_not_accept() {
        let words = text("words").unwrap();
        assert!(!is_valid(
            words,
            "groups:\n  - name: a\n    items:\n      - desc: x\n"
        ));
        assert!(!is_valid(words, "unknown: 1\n"));
        let install = text("install").unwrap();
        assert!(is_valid(
            install,
            "steps:\n  - id: a\n    name: A\n    items:\n      markdown: x\n"
        ));
        assert!(!is_valid(install, "steps:\n  - id: a\n"));
    }

    #[test]
    fn rejects_md_and_unknown_slots() {
        for (name, message) in [
            ("service", "md の Slot"),
            ("api", "md の Slot"),
            ("api-yaml", "Slot の名前ではない"),
        ] {
            let error = text(name).unwrap_err().to_string();
            assert!(error.contains(message), "{name}: {error}");
        }
    }

    #[test]
    fn docs_publish_copies_of_the_schemas() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut copies = vec![("config", "json", crate::config::SCHEMA)];
        for def in yaml_defs() {
            copies.push((def.id, "yaml", text(def.id).unwrap()));
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
