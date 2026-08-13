use crate::runtime_backend::families::activation_and_restrictions::keyword_action_costs::{
    is_known_keyword_action_head, parse_single_word_keyword_action,
};
use crate::runtime_backend::front_end::grammar::{permission_shapes, primitives};
use crate::runtime_backend::lexer::{TokenWordView, lex_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SingleKeywordVerbSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeywordAbilityNameSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VoteChoiceSurface;

pub(crate) fn parse_single_keyword_verb(name: &str) -> Option<SingleKeywordVerbSurface> {
    let tokens = lex_line(name.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    let [word] = words.as_slice() else {
        return None;
    };
    matches!(
        *word,
        "add"
            | "move"
            | "deal"
            | "draw"
            | "counter"
            | "destroy"
            | "exile"
            | "untap"
            | "scry"
            | "discard"
            | "transform"
            | "regenerate"
            | "mill"
            | "get"
            | "reveal"
            | "lose"
            | "gain"
            | "put"
            | "sacrifice"
            | "create"
            | "investigate"
            | "remove"
            | "return"
            | "exchange"
            | "become"
            | "switch"
            | "skip"
            | "surveil"
            | "pay"
    )
    .then_some(SingleKeywordVerbSurface)
}

pub(crate) fn parse_keyword_ability_name(name: &str) -> Option<KeywordAbilityNameSurface> {
    let tokens = lex_line(name.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    if permission_shapes::exact_words(&words, &["first", "strike"])
        || permission_shapes::exact_words(&words, &["double", "strike"])
        || permission_shapes::exact_words(&words, &["ward"])
    {
        return Some(KeywordAbilityNameSurface);
    }
    let [word] = words.as_slice() else {
        return None;
    };
    (parse_single_word_keyword_action(word).is_some() || is_known_keyword_action_head(word))
        .then_some(KeywordAbilityNameSurface)
}

pub(crate) fn parse_vote_choice_surface(text: &str) -> Option<VoteChoiceSurface> {
    let tokens = lex_line(text.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    (permission_shapes::find_words(&words, &["vote", "for"]).is_some()
        || permission_shapes::find_words(&words, &["votes", "for"]).is_some())
    .then_some(VoteChoiceSurface)
}

pub(crate) fn parse_short_self_reference_name(name: &str) -> String {
    let trimmed = name.trim();
    let Ok(tokens) = lex_line(trimmed, 0) else {
        return trimmed.to_string();
    };
    if let Some((_, comma, _)) = primitives::find_prefix(&tokens, || primitives::comma()) {
        let alias = trimmed.get(..comma.span.start).unwrap_or(trimmed).trim();
        if !alias.is_empty() {
            return alias.to_string();
        }
    }

    let words = TokenWordView::new(&tokens);
    if words.len() <= 1 {
        return trimmed.to_string();
    }
    let Some(token_index) = words.token_start_indices().first().copied() else {
        return trimmed.to_string();
    };
    let alias = tokens[token_index]
        .slice
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '-');
    if alias.len() <= 2 || is_reserved_short_alias(alias) {
        trimmed.to_string()
    } else {
        alias.to_string()
    }
}

fn is_reserved_short_alias(alias: &str) -> bool {
    let lower = alias.to_ascii_lowercase();
    if let Ok(tokens) = lex_line(&lower, 0)
        && (super::super::keyword_dispatch::parse_keyword_dispatch_hint_tokens(&tokens).is_some()
            || super::super::line_families::parse_simple_document_line(&tokens).is_some())
    {
        return true;
    }
    if super::super::sentence_markers::parse_keyword_marker_text(&lower).is_some() {
        return true;
    }
    if matches!(
        lower.as_str(),
        "prototype" | "dredge" | "enchanted" | "equipped"
    ) {
        return true;
    }
    if super::super::leaf::parse_leaf_card_type_complete(&lower).is_ok() {
        return true;
    }
    if super::super::leaf::parse_leaf_supertype_complete(&lower).is_ok() {
        return true;
    }
    if super::super::leaf::parse_leaf_color_complete(&lower).is_ok() {
        return true;
    }
    if let Ok(subtype) = super::super::leaf::parse_leaf_subtype_flexible_complete(&lower) {
        // Planeswalker first names are both legitimate short source names and
        // planeswalker subtypes. Other subtype words in rules text are much
        // more likely to be characteristic selectors (for example,
        // "Skeleton or Pirate") than abbreviated card names.
        return !subtype.is_planeswalker_subtype();
    }
    matches!(
        lower.as_str(),
        "a" | "an"
            | "the"
            | "one"
            | "two"
            | "three"
            | "four"
            | "five"
            | "six"
            | "seven"
            | "eight"
            | "nine"
            | "ten"
            | "x"
            | "all"
            | "any"
            | "each"
            | "every"
            | "single"
            | "another"
            | "other"
            | "this"
            | "that"
            | "these"
            | "those"
            | "you"
            | "your"
            | "when"
            | "whenever"
            | "if"
            | "unless"
            | "then"
            | "at"
            | "for"
            | "from"
            | "until"
            | "during"
            | "turn"
            | "without"
            | "with"
            | "first"
            | "second"
            | "third"
            | "fourth"
            | "fifth"
            | "sixth"
            | "seventh"
            | "eighth"
            | "ninth"
            | "tenth"
            | "last"
            | "next"
            | "additional"
            | "alternative"
            | "target"
            | "targets"
            | "targeted"
            | "add"
            | "added"
            | "move"
            | "moved"
            | "deal"
            | "deals"
            | "dealt"
            | "damaged"
            | "draw"
            | "draws"
            | "drawn"
            | "counter"
            | "countered"
            | "double"
            | "doubles"
            | "doubled"
            | "destroy"
            | "destroyed"
            | "exile"
            | "untap"
            | "tapped"
            | "untapped"
            | "scry"
            | "discard"
            | "discarded"
            | "transform"
            | "transformed"
            | "regenerate"
            | "mill"
            | "milled"
            | "get"
            | "reveal"
            | "revealed"
            | "look"
            | "prevent"
            | "prevents"
            | "prevented"
            | "lose"
            | "lost"
            | "gain"
            | "gained"
            | "put"
            | "sacrifice"
            | "sacrificed"
            | "create"
            | "created"
            | "investigate"
            | "attach"
            | "attached"
            | "unattached"
            | "remove"
            | "removed"
            | "return"
            | "returned"
            | "exchange"
            | "become"
            | "became"
            | "switch"
            | "skip"
            | "surveil"
            | "shuffle"
            | "reorder"
            | "pay"
            | "paid"
            | "goad"
            | "goaded"
            | "exiled"
            | "blocked"
            | "blocking"
            | "attacking"
            | "top"
            | "bottom"
            | "same"
            | "different"
            | "villainous"
            | "chosen"
            | "named"
            | "counted"
            | "rounded"
            | "rest"
            | "source"
            | "color"
            | "copy"
            | "clash"
            | "coin"
            | "radiance"
            | "station"
            | "speed"
            | "historic"
            | "nonhistoric"
            | "power"
            | "toughness"
            | "mana"
            | "life"
            | "commander"
            | "player"
            | "opponent"
            | "creature"
            | "artifact"
            | "enchantment"
            | "land"
            | "spell"
            | "card"
            | "token"
            | "permanent"
            | "library"
            | "graveyard"
            | "hand"
            | "battlefield"
            | "controller"
            | "owner"
            | "planeswalker"
            | "battle"
            | "equipment"
            | "aura"
    ) || parse_keyword_ability_name(alias).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keyword_and_short_name_surfaces() {
        assert!(parse_single_keyword_verb("Mill").is_some());
        assert!(parse_keyword_ability_name("double strike").is_some());
        assert_eq!(
            parse_short_self_reference_name("Brago, King Eternal"),
            "Brago"
        );
        assert_eq!(
            parse_short_self_reference_name("Draw the Line"),
            "Draw the Line"
        );
        assert_eq!(
            parse_short_self_reference_name("Skeleton Crew"),
            "Skeleton Crew"
        );
        assert_eq!(
            parse_short_self_reference_name("Attached Count Anthem Variant"),
            "Attached Count Anthem Variant"
        );
        assert_eq!(
            parse_short_self_reference_name("Each Player Sacrifice Variant"),
            "Each Player Sacrifice Variant"
        );
        assert_eq!(
            parse_short_self_reference_name("Black Scarab"),
            "Black Scarab"
        );
        assert_eq!(
            parse_short_self_reference_name("Exiled Flashback Return Variant"),
            "Exiled Flashback Return Variant"
        );
        assert_eq!(
            parse_short_self_reference_name("Turn Static Boundary Variant"),
            "Turn Static Boundary Variant"
        );
        assert_eq!(parse_short_self_reference_name("Ajani Vengeant"), "Ajani");
        assert_eq!(
            parse_short_self_reference_name("Enchanted River's Grasp"),
            "Enchanted River's Grasp",
            "an attached-object adjective must not become a source alias"
        );

        for name in [
            "Craft Variant",
            "Prototype Probe",
            "Escalate Probe",
            "Rampage Variant",
            "Learn Test",
            "Echo Variant",
            "Morph Variant",
            "Adapt Variant",
            "Vanishing Parse Test",
            "Sunburst Parse Test",
            "Removed Counter Mana Variant",
            "Destroyed Draw Variant",
            "Tapped Damage Variant",
            "Bottom Library Exile",
            "Target Opponent Put",
            "Villainous Choice Variant",
            "Same Name Search Probe",
            "Double Counter Probe",
            "Prevent Combat Probe",
            "Blocked Variant",
            "Chosen Copy Probe",
            "Snow Untap Probe",
            "Additional Cost Probe",
            "Then If Probe",
            "Nonhistoric Probe",
        ] {
            assert_eq!(
                parse_short_self_reference_name(name),
                name,
                "mechanic heads must not become abbreviated source names"
            );
        }
    }
}
