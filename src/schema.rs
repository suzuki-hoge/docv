//! docv.yaml のスキーマの URL。docv/slot/system/system.md の「スキーマの公開」。
//!
//! スキーマは JSON Schema で書き（assets/schema/docv.schema.json）、GitHub の版のタグの URL で公開する。
//! docv init は、docv.yaml の 1 行目に、その URL を書く。
//! 同じスキーマを docv/free/schema/config.md に写して公開する。写しが元と揃っていることは、このファイルのテストで確かめる。

/// スキーマの URL の、版の手前まで。版のタグ（v1.8.0 など）の assets/schema/ を指す。
pub const URL_BASE: &str = "https://raw.githubusercontent.com/suzuki-hoge/docv/v";

/// docv.yaml のスキーマのファイルの名前。
const FILE: &str = "docv.schema.json";

/// 実行した CLI の版の、docv.yaml のスキーマの URL。
pub fn url() -> String {
    format!(
        "{URL_BASE}{}/assets/schema/{FILE}",
        env!("CARGO_PKG_VERSION")
    )
}

/// エディタ（yaml-language-server）にスキーマを伝える、docv.yaml の 1 行目。
pub fn header() -> String {
    format!("# yaml-language-server: $schema={}\n", url())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    #[test]
    fn points_to_the_schema_of_this_version() {
        assert_eq!(
            header(),
            format!(
                "# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v{}/assets/schema/docv.schema.json\n",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn docs_publish_a_copy_of_the_schema() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = root.join("docv/free/schema/config.md");
        let published = fs::read_to_string(&path).unwrap_or_default();
        assert!(
            published.contains(&format!("```json\n{}```\n", crate::config::SCHEMA)),
            "{} が docv.yaml のスキーマと揃っていない",
            path.display()
        );
    }
}
