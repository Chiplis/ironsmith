//! Typed substitutions of authored words (CR 612.2).
//!
//! These values do not authorize rewriting every occurrence of a Rust domain
//! value. Owners must call the word operations only for authored rules/type
//! text. In particular, names, mana symbols, color indicators, inferred keyword
//! meanings and captured game choices are not authored word occurrences.

use crate::{Color, ColorSet, Subtype};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextWord {
    Color(Color),
    BasicLandType(Subtype),
    CreatureType(Subtype),
}

/// A checked, directed replacement. The source need not occur on the target;
/// choosing two different words in the declared family is sufficient.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "TextChangeModel"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextChange {
    from: TextWord,
    to: TextWord,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
struct TextChangeModel {
    from: TextWord,
    to: TextWord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextChangeError {
    DifferentFamilies,
    InvalidBasicLandType,
    InvalidCreatureType,
    IdenticalWords,
}

impl std::fmt::Display for TextChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DifferentFamilies => "text replacement words belong to different families",
            Self::InvalidBasicLandType => "text replacement requires a basic land type",
            Self::InvalidCreatureType => "text replacement requires a creature type",
            Self::IdenticalWords => "text replacement requires another word",
        })
    }
}
impl std::error::Error for TextChangeError {}

impl TryFrom<TextChangeModel> for TextChange {
    type Error = TextChangeError;
    fn try_from(model: TextChangeModel) -> Result<Self, Self::Error> {
        Self::new(model.from, model.to)
    }
}

impl TextChange {
    pub fn new(from: TextWord, to: TextWord) -> Result<Self, TextChangeError> {
        match (from, to) {
            (TextWord::Color(_), TextWord::Color(_)) => {},
            (TextWord::BasicLandType(a), TextWord::BasicLandType(b)) => {
                if !a.is_basic_land_type() || !b.is_basic_land_type() {
                    return Err(TextChangeError::InvalidBasicLandType);
                }
            }
            (TextWord::CreatureType(a), TextWord::CreatureType(b)) => {
                if !a.is_creature_type() || !b.is_creature_type() {
                    return Err(TextChangeError::InvalidCreatureType);
                }
            }
            _ => return Err(TextChangeError::DifferentFamilies),
        }
        if from == to { return Err(TextChangeError::IdenticalWords); }
        Ok(Self { from, to })
    }

    pub fn color(from: Color, to: Color) -> Result<Self, TextChangeError> {
        Self::new(TextWord::Color(from), TextWord::Color(to))
    }
    pub fn basic_land_type(from: Subtype, to: Subtype) -> Result<Self, TextChangeError> {
        Self::new(TextWord::BasicLandType(from), TextWord::BasicLandType(to))
    }
    pub fn creature_type(from: Subtype, to: Subtype) -> Result<Self, TextChangeError> {
        Self::new(TextWord::CreatureType(from), TextWord::CreatureType(to))
    }
    pub fn from(self) -> TextWord { self.from }
    pub fn to(self) -> TextWord { self.to }
    pub fn changes_type_words(self) -> bool {
        matches!(self.from, TextWord::BasicLandType(_) | TextWord::CreatureType(_))
    }

    pub fn replace_color_word(self, word: &mut Color) {
        if let (TextWord::Color(from), TextWord::Color(to)) = (self.from, self.to) {
            if *word == from { *word = to; }
        }
    }

    /// A set in an authored color predicate, including a negative predicate
    /// such as "nonblack". This must not be used for an object's actual colors.
    pub fn replace_color_words(self, words: &mut ColorSet) {
        if let (TextWord::Color(from), TextWord::Color(to)) = (self.from, self.to) {
            if words.contains(from) { *words = words.without(from).with(to); }
        }
    }

    pub fn replace_subtype_word(self, word: &mut Subtype) {
        match (self.from, self.to) {
            (TextWord::BasicLandType(from), TextWord::BasicLandType(to))
            | (TextWord::CreatureType(from), TextWord::CreatureType(to)) => {
                if *word == from { *word = to; }
            }
            _ => {},
        }
    }

    /// Type-line/predicate set semantics: replacing Human in Human Vampire
    /// with Vampire leaves one Vampire subtype, not two occurrences.
    pub fn replace_subtype_words(self, words: &mut Vec<Subtype>) {
        let mut changed = false;
        for word in words.iter_mut() {
            let old = *word;
            self.replace_subtype_word(word);
            changed |= *word != old;
        }
        if !changed { return; }
        let mut seen = Vec::new();
        words.retain(|word| {
            if seen.contains(word) { false } else { seen.push(*word); true }
        });
    }
}

/// Resolution-time choice vocabulary. The fixed destination is deliberately
/// distinct from “another”: selecting that same source is a legal no-change
/// outcome, whereas TextChange itself only stores nonidentity substitutions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "TextChangeSelectionModel"))]
#[derive(Debug, Clone, PartialEq, Eq, crate::tag::TagKeyWalk)]
pub enum TextChangeSelection {
    Color,
    BasicLand,
    ColorOrBasicLand,
    Creature { excluded_new: Vec<Subtype> },
    CreatureTo(Subtype),
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
enum TextChangeSelectionModel {
    Color,
    BasicLand,
    ColorOrBasicLand,
    Creature { excluded_new: Vec<Subtype> },
    CreatureTo(Subtype),
}

impl TryFrom<TextChangeSelectionModel> for TextChangeSelection {
    type Error = TextChangeError;
    fn try_from(model: TextChangeSelectionModel) -> Result<Self, Self::Error> {
        let selection = match model {
            TextChangeSelectionModel::Color => Self::Color,
            TextChangeSelectionModel::BasicLand => Self::BasicLand,
            TextChangeSelectionModel::ColorOrBasicLand => Self::ColorOrBasicLand,
            TextChangeSelectionModel::Creature { excluded_new } => Self::Creature { excluded_new },
            TextChangeSelectionModel::CreatureTo(subtype) => Self::CreatureTo(subtype),
        };
        selection.validate()?;
        Ok(selection)
    }
}

impl TextChangeSelection {
    pub fn validate(&self) -> Result<(), TextChangeError> {
        match self {
            Self::Creature { excluded_new } if excluded_new.iter().any(|subtype| !subtype.is_creature_type()) =>
                Err(TextChangeError::InvalidCreatureType),
            Self::CreatureTo(subtype) if !subtype.is_creature_type() => Err(TextChangeError::InvalidCreatureType),
            _ => Ok(()),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, crate::tag::TagKeyWalk)]
pub struct ChangeTextEffect {
    pub target: crate::ChooseSpec,
    pub selection: TextChangeSelection,
    pub duration: crate::Until,
}

impl ChangeTextEffect {
    pub fn new(target: crate::ChooseSpec, selection: TextChangeSelection, duration: crate::Until) -> Self {
        Self { target, selection, duration }
    }
}

crate::tag_key_leaves!(TextWord, TextChange);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_is_directed_and_family_checked() {
        assert_eq!(TextChange::new(TextWord::Color(Color::Blue), TextWord::CreatureType(Subtype::Elf)),
            Err(TextChangeError::DifferentFamilies));
        assert_eq!(TextChange::basic_land_type(Subtype::Desert, Subtype::Forest),
            Err(TextChangeError::InvalidBasicLandType));
        assert_eq!(TextChange::creature_type(Subtype::Equipment, Subtype::Elf),
            Err(TextChangeError::InvalidCreatureType));
        assert_eq!(TextChange::color(Color::Red, Color::Red), Err(TextChangeError::IdenticalWords));
        // Wall is a valid creature type; Artificial Evolution's exclusion
        // belongs to that instruction's choice contract, not all text changes.
        assert!(TextChange::creature_type(Subtype::Elf, Subtype::Wall).is_ok());
    }

    #[test]
    fn words_are_replaced_once_and_sets_collapse_duplicates() {
        let red_to_blue = TextChange::color(Color::Red, Color::Blue).unwrap();
        let mut words = ColorSet::RED.union(ColorSet::BLUE);
        red_to_blue.replace_color_words(&mut words);
        assert_eq!(words, ColorSet::BLUE);
        let mut absent = ColorSet::GREEN;
        red_to_blue.replace_color_words(&mut absent);
        assert_eq!(absent, ColorSet::GREEN);
        let mut types = vec![Subtype::Human, Subtype::Vampire, Subtype::Wizard];
        TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap()
            .replace_subtype_words(&mut types);
        assert_eq!(types, vec![Subtype::Vampire, Subtype::Wizard]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn wire_round_trip_checks_the_same_domain() {
        let value = TextChange::basic_land_type(Subtype::Island, Subtype::Swamp).unwrap();
        let wire = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<TextChange>(&wire).unwrap(), value);
        assert!(serde_json::from_str::<TextChange>(
            r#"{"from":{"CreatureType":"Equipment"},"to":{"CreatureType":"Wall"}}"#).is_err());
    }
    #[cfg(feature = "serde")]
    #[test]
    fn selection_wire_rejects_noncreature_fixed_or_excluded_destinations() {
        let fixed = TextChangeSelection::CreatureTo(Subtype::Vampire);
        let wire = serde_json::to_value(&fixed).unwrap();
        assert_eq!(serde_json::from_value::<TextChangeSelection>(wire).unwrap(), fixed);
        assert!(serde_json::from_str::<TextChangeSelection>(r#"{"CreatureTo":"Equipment"}"#).is_err());
        assert!(serde_json::from_str::<TextChangeSelection>(r#"{"Creature":{"excluded_new":["Forest"]}}"#).is_err());
    }

}
