//! docv slot schema。yaml の Slot のスキーマを出す。docv/service/manual.md の docv slot schema。
//!
//! md の Slot はスキーマを持たない。スキーマは JSON Schema で書き、GitHub の版のタグの URL で公開する。
//! 同じスキーマと `docv.yaml` のスキーマを docs/schema/ の Markdown に写して公開する。
//! 写しが元と揃っていることは、このファイルのテストで確かめる。

use anyhow::{Result, bail};

use crate::generate;
use crate::slot::{self, Format};

/// 実行した CLI の版の、assets/schema/ のファイルの URL。docv.yaml と YAML の Slot の 1 行目から参照する。
pub fn url(file: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/assets/schema/{file}",
        env!("CARGO_PKG_VERSION")
    )
}

/// エディタ（yaml-language-server）にスキーマを伝える、YAML のファイルの 1 行目。
pub fn header(file: &str) -> String {
    format!("# yaml-language-server: $schema={}\n", url(file))
}

/// yaml の Slot のスキーマ（JSON）を返す。
pub fn text(name: &str) -> Result<&'static str> {
    let def = generate::find(name)?;
    if def.format != Format::Yaml {
        // ページに分けられる Slot は、分けるときの YAML front matter の書き方を docv desc で案内する。
        let pages = if def.pages_dir().is_some() {
            format!(
                "。ページに分けるときの YAML front matter は、docv desc の「{}のページ分け」で確かめる",
                def.label
            )
        } else {
            String::new()
        };
        bail!("{name} は md の Slot で、スキーマはない。中身の形は決めていない{pages}");
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
        let schema: Value = serde_json::from_str(text).unwrap();
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
            let schema: Value = serde_json::from_str(text).unwrap();
            assert_eq!(
                schema["title"],
                format!("{}（{}）", def.label, def.file_name()),
                "{}",
                def.id
            );
            assert_eq!(schema["description"], def.mission, "{}", def.id);
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
            "steps:\n  - id: a\n    name: A\n    items:\n      body_markdown: x\n"
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
        // ページに分けられる Slot だけ、分けるときの YAML front matter の書き方を案内する
        let service = text("service").unwrap_err().to_string();
        assert!(
            service.contains("docv desc の「サービス仕様のページ分け」"),
            "{service}"
        );
        assert!(!text("api").unwrap_err().to_string().contains("ページ分け"));
    }

    #[test]
    fn docs_publish_copies_of_the_schemas() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut copies = vec![("config", crate::config::SCHEMA)];
        for def in yaml_defs() {
            copies.push((def.id, text(def.id).unwrap()));
        }
        for (name, schema) in copies {
            let path = root.join(format!("docs/schema/{name}.md"));
            let published = fs::read_to_string(&path).unwrap_or_default();
            assert!(
                published.contains(&format!("```json\n{schema}```\n")),
                "{} が {name} のスキーマと揃っていない",
                path.display()
            );
        }
    }
}
