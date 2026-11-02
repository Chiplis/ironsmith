//! Grant a lasting player designation when a permanent-count condition holds.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "DesignationWire"))]
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct AscendEffect {
    pub minimum: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub history_requirements: Vec<DesignationHistoryRequirement>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub tapped_only: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub named_designation: Option<String>,
    pub historic_only: bool,
    pub enduring_story: bool,
    pub descriptive: bool,
}

impl Default for AscendEffect { fn default() -> Self { Self::new() } }
impl AscendEffect {
    pub const fn new() -> Self {
        Self { minimum: 10, history_requirements: Vec::new(), tapped_only: false, named_designation: None, historic_only: false, enduring_story: false, descriptive: false }
    }
    pub const fn designation_if_permanent_count(minimum: u32, historic_only: bool, enduring_story: bool) -> Self {
        Self { minimum, history_requirements: Vec::new(), tapped_only: false, named_designation: None, historic_only, enduring_story, descriptive: true }
    }
}

// Older artifacts encoded the native unit effect as null.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum DesignationWire {
    Legacy(()),
    Count { #[serde(default)] history_requirements: Vec<DesignationHistoryRequirement>, minimum: u32, historic_only: bool, enduring_story: bool, descriptive: bool, #[serde(default)] tapped_only: bool, #[serde(default)] named_designation: Option<String> },
}
#[cfg(feature = "serde")]
impl From<DesignationWire> for AscendEffect {
    fn from(value: DesignationWire) -> Self {
        match value {
            DesignationWire::Legacy(()) => Self::new(),
            DesignationWire::Count { minimum, historic_only, enduring_story, descriptive, tapped_only, named_designation, history_requirements } => Self { minimum, historic_only, enduring_story, descriptive, tapped_only, named_designation, history_requirements },
        }
    }
}

/// Independent completed turn actions. A conjunction preserves each actor and
/// its historical receipt even after the originating object changes zones.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub enum DesignationHistoryRequirement {
    CastSpellMatchingTypes(Vec<crate::types::CardType>),
    ActivatedAbility { nonmana_only: bool },
    Attacked,
}
impl DesignationHistoryRequirement {
    pub fn clause(&self) -> String {
        match self {
            Self::CastSpellMatchingTypes(types) => {
                let names = types.iter().map(|ty| ty.to_string().to_ascii_lowercase()).collect::<Vec<_>>();
                if names.is_empty() { "cast a spell this turn".into() }
                else { format!("cast {} {} spell this turn", if names[0].as_bytes().first().is_some_and(|letter| b"aeiou".contains(letter)) { "an" } else { "a" }, names.join(" or ")) }
            }
            Self::ActivatedAbility { nonmana_only } => format!("activated {} ability this turn", if *nonmana_only { "a nonmana" } else { "an" }),
            Self::Attacked => "attacked this turn".into(),
        }
    }
    pub fn read_clause(clause: &str) -> Option<Self> {
        let clause = clause.trim().strip_prefix("and ").unwrap_or(clause.trim());
        if clause == "attacked this turn" { return Some(Self::Attacked); }
        if clause == "activated a nonmana ability this turn" { return Some(Self::ActivatedAbility { nonmana_only: true }); }
        if clause == "activated an ability this turn" { return Some(Self::ActivatedAbility { nonmana_only: false }); }
        let quality = clause.strip_prefix("cast ")?.strip_suffix(" spell this turn")?;
        let quality = quality.strip_prefix("an ").or_else(|| quality.strip_prefix("a "))?;
        let types = quality.split(" or ").map(|word| match word {
            "instant" => Some(crate::types::CardType::Instant),
            "sorcery" => Some(crate::types::CardType::Sorcery),
            "creature" => Some(crate::types::CardType::Creature),
            "artifact" => Some(crate::types::CardType::Artifact),
            "enchantment" => Some(crate::types::CardType::Enchantment),
            "planeswalker" => Some(crate::types::CardType::Planeswalker),
            "battle" => Some(crate::types::CardType::Battle),
            _ => None,
        }).collect::<Option<Vec<_>>>()?;
        Some(Self::CastSpellMatchingTypes(types))
    }
}
