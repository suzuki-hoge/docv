//! docv update-guide。docv を更新したときに、版ごとの変更点と対応を出す。docv/service/manual.md の docv update-guide。

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::version::{self, Version};

/// 版ごとの変更点と対応。新しい版を先頭に書き、版を上げるときに足す（docv/operation/release.md）。
/// バイナリに埋め込むので、CLI の版と内容が一致する。
const SOURCE: &str = include_str!("../assets/update-guide.yaml");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    version: String,
    changes: Vec<Change>,
}

/// 変更 1 つ。change は何が変わったか、action はそれへの対応。対応がなければ「対応は要らない」と書く。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    change: String,
    action: String,
}

/// 埋め込んだ変更点を、書いた順（新しい版から）に返す。
fn releases() -> Vec<(Version, Vec<Change>)> {
    let releases: Vec<Release> =
        serde_yaml_ng::from_str(SOURCE).expect("埋め込みの update-guide.yaml は正しい");
    releases
        .into_iter()
        .map(|r| {
            let version = r
                .version
                .parse()
                .expect("update-guide.yaml の版は 1.8.0 の形");
            (version, r.changes)
        })
        .collect()
}

/// from より後の版から、今の版までの変更点と対応。古い版から順に並べ、最後に docv_version とスキーマの URL を上げる案内を置く。
pub fn text(from: &str) -> Result<String> {
    let current = Version::current();
    let Some(from) = Version::from_arg(from) else {
        bail!("--from には、1.4.0 の形で版を書く: {from}");
    };
    if from < version::FIRST {
        bail!(
            "--from には、最初の版の {} から、docv の版の {current} までを書く: {from}",
            version::FIRST
        );
    }
    if from > current {
        bail!("{from} は docv の版の {current} より新しい。docv を {from} 以上にしてから実行する");
    }
    if from == current {
        return Ok(format!("{current} からの変更点はない\n"));
    }
    let mut out = String::new();
    for (version, changes) in releases().iter().rev().filter(|(v, _)| *v > from) {
        out.push_str(&format!("{version} 変更点\n"));
        for c in changes {
            out.push_str(&format!("- {}: {}\n", c.change, c.action));
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "対応が終わったら、docv.yaml の docv_version を {current} にし、docv.yaml と YAML の Slot の 1 行目にあるスキーマの URL の版を v{current} にする。最後に docv check で、残りがないかを確かめる\n"
    ));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_release_is_this_version() {
        // 版を上げたら、その版の変更点を update-guide.yaml の先頭に足す
        assert_eq!(releases()[0].0, Version::current());
    }

    #[test]
    fn releases_are_written_from_the_newest() {
        let releases = releases();
        for pair in releases.windows(2) {
            assert!(pair[0].0 > pair[1].0, "{} と {} の順", pair[0].0, pair[1].0);
        }
        for (version, changes) in &releases {
            assert!(*version > version::FIRST, "{version}");
            assert!(!changes.is_empty(), "{version}");
            for c in changes {
                for text in [&c.change, &c.action] {
                    assert!(!text.trim().is_empty(), "{version}");
                    assert!(!text.contains('\n'), "{version}: {text}");
                    assert!(!text.ends_with('。'), "{version}: {text}");
                }
            }
        }
    }

    #[test]
    fn prints_releases_after_from_in_order() {
        let current = Version::current();
        let all = text("1.0.0").unwrap();
        let headings: Vec<&str> = all.lines().filter(|l| l.ends_with(" 変更点")).collect();
        assert_eq!(headings.len(), releases().len());
        assert_eq!(headings[0], "1.0.1 変更点");
        assert_eq!(*headings.last().unwrap(), format!("{current} 変更点"));
        assert!(all.contains("\n- "), "{all}");
        assert!(
            all.ends_with(&format!(
                "対応が終わったら、docv.yaml の docv_version を {current} にし、docv.yaml と YAML の Slot の 1 行目にあるスキーマの URL の版を v{current} にする。最後に docv check で、残りがないかを確かめる\n"
            )),
            "{all}"
        );
        // from の版は含めない。間の版（出していないパッチの版など）も受け付ける
        let some = text("v1.0.1").unwrap();
        assert!(!some.contains("1.0.1 変更点"), "{some}");
        assert!(some.starts_with("1.1.0 変更点\n"), "{some}");
        assert_eq!(text("1.0.5").unwrap(), some);
    }

    #[test]
    fn prints_nothing_to_do_for_this_version() {
        let current = Version::current();
        assert_eq!(
            text(&current.to_string()).unwrap(),
            format!("{current} からの変更点はない\n")
        );
    }

    #[test]
    fn rejects_unknown_versions() {
        for (from, message) in [
            ("1.4", "1.4.0 の形"),
            ("latest", "1.4.0 の形"),
            ("0.9.0", "最初の版の 1.0.0 から"),
            ("99.0.0", "より新しい"),
        ] {
            let error = text(from).unwrap_err().to_string();
            assert!(error.contains(message), "{from}: {error}");
        }
    }
}
