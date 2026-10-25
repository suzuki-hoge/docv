//! docv desc。docv の概要を出す。docv/service/manual.md の docv desc。

use crate::markdown;

/// docv の概要の元のファイル。docv のサービス仕様（docv/service/service.md）と同じファイル。バイナリに埋め込むので、CLI の版と内容が一致する。
const SOURCE: &str = include_str!("../assets/desc.md");

/// docv の概要。元のファイルは md の Slot として YAML front matter の mission を持つので、それを除いた本文を出す。
pub fn text() -> String {
    markdown::split(SOURCE)
        .body
        .trim_start_matches('\n')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_the_body_without_frontmatter() {
        assert!(text().starts_with("# docv\n"), "{}", text());
    }
}
