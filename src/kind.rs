//! ドキュメントの種類。組み込みの種類は docs/spec/kind.md の 1 章。

use serde::Deserialize;

use crate::config::CustomKind;

/// 種類の時間。並び順は HTML とインデックスで見せる順（現在、未来、過去）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Time {
    Present,
    Future,
    Past,
}

impl Time {
    pub fn label(self) -> &'static str {
        match self {
            Time::Present => "現在",
            Time::Future => "未来",
            Time::Past => "過去",
        }
    }
}

pub struct Builtin {
    pub id: &'static str,
    pub label: &'static str,
    pub time: Time,
    /// この語を含む h2 が必要。
    pub required_sections: &'static [&'static str],
}

pub const BUILTIN: &[Builtin] = &[
    Builtin {
        id: "overview",
        label: "概要",
        time: Time::Present,
        required_sections: &[],
    },
    Builtin {
        id: "spec",
        label: "仕様",
        time: Time::Present,
        required_sections: &[],
    },
    Builtin {
        id: "operation",
        label: "手順",
        time: Time::Present,
        required_sections: &[],
    },
    Builtin {
        id: "cost",
        label: "費用",
        time: Time::Present,
        required_sections: &["単価", "見積もり"],
    },
    Builtin {
        id: "rule",
        label: "規約",
        time: Time::Present,
        required_sections: &[],
    },
    Builtin {
        id: "plan",
        label: "計画",
        time: Time::Future,
        required_sections: &[],
    },
    Builtin {
        id: "proposal",
        label: "検討中",
        time: Time::Future,
        required_sections: &["案", "決めること"],
    },
    Builtin {
        id: "fixme",
        label: "未決",
        time: Time::Future,
        required_sections: &[],
    },
    Builtin {
        id: "archive",
        label: "経緯",
        time: Time::Past,
        required_sections: &[],
    },
];

pub fn is_builtin(id: &str) -> bool {
    BUILTIN.iter().any(|k| k.id == id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub id: String,
    pub label: String,
    pub time: Time,
    pub required_sections: Vec<String>,
}

impl Kind {
    pub fn is_archive(&self) -> bool {
        self.id == "archive"
    }

    pub fn is_plan(&self) -> bool {
        self.id == "plan"
    }
}

/// 組み込みの種類と、docv.yaml の kinds で足した種類。
#[derive(Debug, Clone)]
pub struct Kinds(Vec<Kind>);

impl Kinds {
    pub fn new(custom: &[CustomKind]) -> Self {
        let builtin = BUILTIN.iter().map(|k| Kind {
            id: k.id.to_string(),
            label: k.label.to_string(),
            time: k.time,
            required_sections: k.required_sections.iter().map(|s| s.to_string()).collect(),
        });
        let custom = custom.iter().map(|k| Kind {
            id: k.id.clone(),
            label: k.label.clone().unwrap_or_else(|| k.id.clone()),
            time: k.time,
            required_sections: Vec::new(),
        });
        Kinds(builtin.chain(custom).collect())
    }

    pub fn get(&self, id: &str) -> Option<&Kind> {
        self.0.iter().find(|k| k.id == id)
    }

    pub fn all(&self) -> &[Kind] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_kind_uses_id_as_label_when_omitted() {
        let kinds = Kinds::new(&[CustomKind {
            id: "research".to_string(),
            label: None,
            time: Time::Future,
        }]);
        let kind = kinds.get("research").unwrap();
        assert_eq!(kind.label, "research");
        assert_eq!(kind.time, Time::Future);
        assert_eq!(
            kinds.get("cost").unwrap().required_sections,
            ["単価", "見積もり"]
        );
    }
}
