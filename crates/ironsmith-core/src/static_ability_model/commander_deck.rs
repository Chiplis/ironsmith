//! Commander deck-construction allowances printed on a commander
//! ("Rulebreaker — A deck with this commander can have Angel cards of any
//! color identity and any basic land cards."). They have no in-game effect;
//! deck validation reads them from the deck's commander(s).

use crate::tag::TagKeyWalk;

use super::*;

/// One class of cards a deck-construction rule names: "creature cards with
/// mana value 7 or greater", "basic land cards", "Phyrexian cards". A card
/// belongs to the class when it has every listed card type, supertype, and
/// subtype and at least the minimum mana value.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default, TagKeyWalk)]
pub struct DeckCardClass {
    pub card_types: Vec<CardType>,
    pub supertypes: Vec<Supertype>,
    pub subtypes: Vec<Subtype>,
    /// "with mana value N or greater".
    pub min_mana_value: Option<u32>,
}

impl DeckCardClass {
    pub fn matches(
        &self,
        card_types: &[CardType],
        supertypes: &[Supertype],
        subtypes: &[Subtype],
        mana_value: u32,
    ) -> bool {
        self.card_types.iter().all(|card_type| card_types.contains(card_type))
            && self.supertypes.iter().all(|supertype| supertypes.contains(supertype))
            && self.subtypes.iter().all(|subtype| subtypes.contains(subtype))
            && self.min_mana_value.is_none_or(|minimum| mana_value >= minimum)
    }

    /// The class's noun phrase, singular head omitted: "creature cards with
    /// mana value 7 or greater", "basic land cards".
    pub fn plural_noun(&self) -> String {
        let mut words: Vec<String> = Vec::new();
        for supertype in &self.supertypes {
            words.push(supertype.name().to_ascii_lowercase());
        }
        for card_type in &self.card_types {
            words.push(card_type.name().to_string());
        }
        for subtype in &self.subtypes {
            words.push(subtype.to_string());
        }
        words.push("cards".to_string());
        let mut phrase = words.join(" ");
        if let Some(minimum) = self.min_mana_value {
            phrase.push_str(&format!(" with mana value {minimum} or greater"));
        }
        phrase
    }
}

/// "the color identity of <classes> in your deck can include N color(s) of
/// your choice not in your commander's color identity" (Tolabow, Loch
/// Rascal). Validation accepts the deck when the colors those cards add
/// outside the commander's color identity number at most `count`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default, TagKeyWalk)]
pub struct ExtraIdentityColors {
    pub classes: Vec<DeckCardClass>,
    pub count: u32,
}

/// What a commander's deck-construction rule relaxes for the deck it leads
/// (CR 903.5): cards of some classes ignore the commander's color identity
/// (CR 903.5c), some classes may add chosen colors, and the deck may exceed
/// 100 cards (CR 903.5a).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default, TagKeyWalk)]
pub struct CommanderDeckAllowance {
    /// "can have <classes> of any color identity", "can have any land cards".
    pub any_color_identity: Vec<DeckCardClass>,
    pub extra_identity_colors: Option<ExtraIdentityColors>,
    /// "has no maximum deck size": at least 100 cards instead of exactly 100.
    pub no_maximum_deck_size: bool,
}

impl CommanderDeckAllowance {
    pub fn is_empty(&self) -> bool {
        self.any_color_identity.is_empty()
            && self.extra_identity_colors.is_none()
            && !self.no_maximum_deck_size
    }

    /// Fold another commander's allowance into this one (a partner pair).
    pub fn merge(&mut self, other: &CommanderDeckAllowance) {
        for class in &other.any_color_identity {
            if !self.any_color_identity.contains(class) {
                self.any_color_identity.push(class.clone());
            }
        }
        if let Some(extra) = &other.extra_identity_colors {
            match &mut self.extra_identity_colors {
                Some(mine) => {
                    for class in &extra.classes {
                        if !mine.classes.contains(class) {
                            mine.classes.push(class.clone());
                        }
                    }
                    mine.count = mine.count.max(extra.count);
                }
                None => self.extra_identity_colors = Some(extra.clone()),
            }
        }
        self.no_maximum_deck_size |= other.no_maximum_deck_size;
    }

    pub fn ignores_color_identity(
        &self,
        card_types: &[CardType],
        supertypes: &[Supertype],
        subtypes: &[Subtype],
        mana_value: u32,
    ) -> bool {
        self.any_color_identity
            .iter()
            .any(|class| class.matches(card_types, supertypes, subtypes, mana_value))
    }

    /// How many chosen colors outside the commander's identity a card of
    /// this shape may carry (0 when no extra-color clause covers it).
    pub fn extra_identity_color_count(
        &self,
        card_types: &[CardType],
        supertypes: &[Supertype],
        subtypes: &[Subtype],
        mana_value: u32,
    ) -> u32 {
        self.extra_identity_colors
            .as_ref()
            .filter(|extra| {
                extra
                    .classes
                    .iter()
                    .any(|class| class.matches(card_types, supertypes, subtypes, mana_value))
            })
            .map_or(0, |extra| extra.count)
    }

    /// The rule read back from its typed fields, in the printed wording.
    pub fn describe(&self) -> String {
        fn join_and(items: &[String]) -> String {
            match items {
                [] => String::new(),
                [one] => one.clone(),
                [first, second] => format!("{first} and {second}"),
                [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
            }
        }
        let mut clauses = Vec::new();
        let basic_land = DeckCardClass {
            card_types: vec![CardType::Land],
            supertypes: vec![Supertype::Basic],
            ..DeckCardClass::default()
        };
        let land = DeckCardClass { card_types: vec![CardType::Land], ..DeckCardClass::default() };
        let mut any_identity = Vec::new();
        let mut any_cards = Vec::new();
        for class in &self.any_color_identity {
            if *class == basic_land || *class == land {
                any_cards.push(format!("any {}", class.plural_noun()));
            } else {
                any_identity.push(class.plural_noun());
            }
        }
        if let Some(extra) = &self.extra_identity_colors {
            let classes = extra
                .classes
                .iter()
                .map(|class| class.plural_noun().trim_end_matches(" cards").to_string())
                .collect::<Vec<_>>();
            clauses.push(format!(
                "the color identity of {} cards in your deck can include {} of your choice not in your commander's color identity",
                join_and(&classes),
                if extra.count == 1 {
                    "one color".to_string()
                } else {
                    format!("{} colors", extra.count)
                }
            ));
        }
        let mut haves = Vec::new();
        if !any_identity.is_empty() {
            // "artifact creature and Equipment cards of any color identity"
            let heads = any_identity
                .iter()
                .map(|noun| noun.strip_suffix(" cards").unwrap_or(noun).to_string())
                .collect::<Vec<_>>();
            let single_tail = any_identity.iter().all(|noun| noun.ends_with(" cards"));
            haves.push(if single_tail {
                format!("{} cards of any color identity", join_and(&heads))
            } else {
                format!("{} of any color identity", join_and(&any_identity))
            });
        }
        haves.extend(any_cards);
        if !haves.is_empty() {
            clauses.push(format!("a deck with this commander can have {}", join_and(&haves)));
        }
        if self.no_maximum_deck_size {
            clauses.push("a deck with this commander has no maximum deck size".to_string());
        }
        let text = clauses.join(", and ");
        let mut chars = text.chars();
        match chars.next() {
            Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
            None => text,
        }
    }
}
