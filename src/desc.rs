//! docv desc。フレームワークのルールファイルを出す。docs/spec/cli.md の 4 章。

/// AI Agent 向けのルールファイル。バイナリに埋め込むので、CLI の版と内容が一致する。
pub const RULES: &str = include_str!("../assets/desc.md");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_commands_for_ai_agents() {
        for command in [
            "docv index",
            "docv check",
            "docv new",
            "docv retire",
            "docv fix",
        ] {
            assert!(RULES.contains(command), "{command}");
        }
    }
}
