use ironsmith_core::static_ability_model::DeckConstructionRule;
use crate::util::is_source_reference_words;

use super::super::super::lexer::{OwnedLexToken, TokenKind, TokenWordView, parser_token_word_refs};
use super::super::keyword_static_lines::{self, AdditionalVoteKind};
use super::super::leaf;
use super::{
    any_word_is_present, every_phrase_is_present, phrase_is_exact, phrase_is_prefix,
    phrase_is_present, phrase_is_suffix, phrase_location,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticSpecialLineShape {
    BlackManaMayBePaidWithLife,
    BoastTwice,
    /// Oath of Teferi.
    LoyaltyTwice,
    EquipAtInstantSpeed,
    /// "During your turn, you may activate equip abilities any time you
    /// could cast an instant." (Forge Anew).
    EquipAtInstantSpeedDuringYourTurn,
    AdditionalVoteTime,
    AdditionalVote,
    DoesntUntap,
    DraftRule,
    HiddenAgenda,
    DoubleAgenda,
    StartingPlayer,
    DeckConstruction(DeckConstructionRule),
    /// "A deck with this commander can have ..." / "If ~ is your commander,
    /// ..." (Rulebreaker): see [`parse_commander_deck_allowance_words`].
    CommanderDeckAllowance,
    FirstEquipCostAlternative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombinedSpellAndActivationTax;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceKeywordTail<'a> {
    pub ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipKeywordActionProbe;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityWordMarker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelUpIntro;

fn is_black_mana_life_payment(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
            "for", "each", "b", "in", "a", "cost", "you", "may", "pay", "2", "life", "rather",
            "than", "pay", "that", "mana",
        ],
    )
}

fn is_boast_twice(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
            "creatures",
            "you",
            "control",
            "can",
            "boast",
            "twice",
            "during",
            "each",
            "of",
            "your",
            "turns",
            "rather",
            "than",
            "once",
        ],
    )
}

fn is_equip_at_instant_speed(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
            "you",
            "may",
            "activate",
            "equip",
            "abilities",
            "any",
            "time",
            "you",
            "could",
            "cast",
            "an",
            "instant",
        ],
    )
}

fn is_doesnt_untap(words: &[&str]) -> bool {
    phrase_is_suffix(words, &["untap", "during", "your", "untap", "step"])
        && any_word_is_present(words, &["doesnt", "doesn't"])
}

fn is_draft_rule(words: &[&str]) -> bool {
    phrase_is_exact(words, &["draft", "this", "card", "face", "up"])
        || [
            &["reveal", "this", "card", "as", "you", "draft", "it"][..],
            &["as", "you", "draft"],
            &["during", "the", "draft"],
            &["immediately", "after", "the", "draft"],
            &["instead", "of", "drafting"],
            &[
                "as", "long", "as", "this", "card", "is", "face", "up", "during", "the", "draft",
            ],
        ]
        .iter()
        .any(|prefix| phrase_is_prefix(words, prefix))
        || (phrase_is_prefix(words, &["each", "player", "passes"])
            && phrase_is_present(words, &["booster", "pack"]))
}

fn is_hidden_agenda(words: &[&str]) -> bool {
    phrase_is_exact(words, &["hidden", "agenda"])
}

fn is_double_agenda(words: &[&str]) -> bool {
    phrase_is_exact(words, &["double", "agenda"])
}

/// Power Play: "You are the starting player. If multiple players would be
/// the starting player, one of those players is chosen at random."
fn is_starting_player(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
            "you", "are", "the", "starting", "player", "if", "multiple", "players", "would",
            "be", "the", "starting", "player", "one", "of", "those", "players", "is", "chosen",
            "at", "random",
        ],
    )
}

/// One class head: card-type, supertype, and subtype words ("basic land",
/// "artifact creature", "equipment", "phyrexian").
fn deck_card_class_head(words: &[&str]) -> Option<ironsmith_core::DeckCardClass> {
    use crate::grammar::primitives::probe_shape;
    if words.is_empty() {
        return None;
    }
    let mut class = ironsmith_core::DeckCardClass::default();
    for word in words {
        if let Some(card_type) = probe_shape(leaf::parse_leaf_card_type_complete(word)) {
            class.card_types.push(card_type);
        } else if let Some(supertype) = probe_shape(leaf::parse_leaf_supertype_complete(word)) {
            class.supertypes.push(supertype);
        } else if let Some(subtype) = probe_shape(leaf::parse_leaf_subtype_complete(word)) {
            class.subtypes.push(subtype);
        } else {
            return None;
        }
    }
    Some(class)
}

/// "<head> (and <head>)* cards [with mana value N or greater]" at the start
/// of `words`: the classes and the words read.
fn deck_card_class_list(words: &[&str]) -> Option<(Vec<ironsmith_core::DeckCardClass>, usize)> {
    let cards_at = words.iter().position(|word| matches!(*word, "cards" | "card"))?;
    let mut classes = Vec::new();
    for head in words[..cards_at].split(|word| *word == "and") {
        classes.push(deck_card_class_head(head)?);
    }
    let mut used = cards_at + 1;
    let rest = &words[used..];
    if phrase_is_prefix(rest, &["with", "mana", "value"]) {
        let (minimum, consumed) =
            leaf::parse_leaf_number_prefix_words(&rest[3..]).and_then(|number| number.into_fixed())?;
        if !phrase_is_prefix(&rest[3 + consumed..], &["or", "greater"]) {
            return None;
        }
        for class in &mut classes {
            class.min_mana_value = Some(minimum);
        }
        used += 3 + consumed + 2;
    }
    Some((classes, used))
}

/// "can have" objects: "any <classes>" and "<classes> of any color identity",
/// joined by "and". Every named class ignores the commander's color identity
/// (CR 903.5c).
fn deck_allowance_haves(mut words: &[&str], into: &mut Vec<ironsmith_core::DeckCardClass>) -> Option<()> {
    loop {
        if let Some(rest) = words.strip_prefix(&["any"]) {
            let (classes, used) = deck_card_class_list(rest)?;
            into.extend(classes);
            words = &rest[used..];
        } else {
            let (classes, used) = deck_card_class_list(words)?;
            words = words[used..].strip_prefix(&["of", "any", "color", "identity"])?;
            into.extend(classes);
        }
        match words {
            [] => return Some(()),
            ["and", rest @ ..] => words = rest,
            _ => return None,
        }
    }
}

/// Rulebreaker commander deck-construction allowances:
/// "A deck with this commander can have <classes> of any color identity and
/// any basic land cards." / "... has no maximum deck size." / "If ~ is your
/// commander, the color identity of <classes> in your deck can include N
/// color(s) of your choice not in your commander's color identity, and your
/// deck can have any basic land cards."
pub fn parse_commander_deck_allowance_words(
    words: &[&str],
) -> Option<ironsmith_core::CommanderDeckAllowance> {
    let mut allowance = ironsmith_core::CommanderDeckAllowance::default();
    if let Some(rest) = words.strip_prefix(&["a", "deck", "with", "this", "commander"]) {
        if let Some(haves) = rest.strip_prefix(&["can", "have"]) {
            deck_allowance_haves(haves, &mut allowance.any_color_identity)?;
        } else if rest == ["has", "no", "maximum", "deck", "size"] {
            allowance.no_maximum_deck_size = true;
        } else {
            return None;
        }
        return Some(allowance);
    }
    let rest = words.strip_prefix(&["if"])?;
    let is_at = rest.windows(3).position(|window| window == ["is", "your", "commander"])?;
    if is_at == 0 || !(is_source_reference_words(&rest[..is_at]) || rest[..is_at] == ["~"]) {
        return None;
    }
    let rest = rest[is_at + 3..].strip_prefix(&["the", "color", "identity", "of"])?;
    let (classes, used) = deck_card_class_list(rest)?;
    let rest = rest[used..].strip_prefix(&["in", "your", "deck", "can", "include"])?;
    let (count, consumed) =
        leaf::parse_leaf_number_prefix_words(rest).and_then(|number| number.into_fixed())?;
    let rest = &rest[consumed..];
    let rest = rest
        .strip_prefix(&["color"])
        .or_else(|| rest.strip_prefix(&["colors"]))?;
    let rest = rest.strip_prefix(&[
        "of", "your", "choice", "not", "in", "your", "commanders", "color", "identity",
    ])?;
    if count == 0 {
        return None;
    }
    allowance.extra_identity_colors = Some(ironsmith_core::ExtraIdentityColors { classes, count });
    match rest {
        [] => {}
        ["and", "your", "deck", "can", "have", haves @ ..] => {
            deck_allowance_haves(haves, &mut allowance.any_color_identity)?;
        }
        _ => return None,
    }
    Some(allowance)
}

pub fn parse_static_special_line_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StaticSpecialLineShape> {
    let words = parser_token_word_refs(tokens);
    let shape = parse_static_special_line_words(tokens, &words)?;
    // These shapes are complete fixed lines, matched word for word.
    if matches!(
        shape,
        StaticSpecialLineShape::BlackManaMayBePaidWithLife
            | StaticSpecialLineShape::BoastTwice
            | StaticSpecialLineShape::LoyaltyTwice
            | StaticSpecialLineShape::EquipAtInstantSpeed
            | StaticSpecialLineShape::EquipAtInstantSpeedDuringYourTurn
            | StaticSpecialLineShape::HiddenAgenda
            | StaticSpecialLineShape::DoubleAgenda
            | StaticSpecialLineShape::StartingPlayer
    ) {
        crate::word_claims::claim_words(&words);
    }
    Some(shape)
}

fn parse_static_special_line_words(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<StaticSpecialLineShape> {
    let words = words.to_vec();
    if is_black_mana_life_payment(&words) {
        Some(StaticSpecialLineShape::BlackManaMayBePaidWithLife)
    } else if is_boast_twice(&words) {
        Some(StaticSpecialLineShape::BoastTwice)
    } else if phrase_is_exact(
        &words,
        &[
            "you", "may", "activate", "the", "loyalty", "abilities", "of", "planeswalkers",
            "you", "control", "twice", "each", "turn", "rather", "than", "only", "once",
        ],
    ) {
        Some(StaticSpecialLineShape::LoyaltyTwice)
    } else if is_equip_at_instant_speed(&words) {
        Some(StaticSpecialLineShape::EquipAtInstantSpeed)
    } else if words.len() > 3
        && words[..3] == ["during", "your", "turn"]
        && is_equip_at_instant_speed(&words[3..])
    {
        Some(StaticSpecialLineShape::EquipAtInstantSpeedDuringYourTurn)
    } else if let Some(kind) = keyword_static_lines::parse_additional_vote_tokens(tokens) {
        Some(match kind {
            AdditionalVoteKind::OptionalTime => StaticSpecialLineShape::AdditionalVoteTime,
            AdditionalVoteKind::MandatoryVote => StaticSpecialLineShape::AdditionalVote,
        })
    } else if is_doesnt_untap(&words) {
        Some(StaticSpecialLineShape::DoesntUntap)
    } else if is_hidden_agenda(&words) {
        Some(StaticSpecialLineShape::HiddenAgenda)
    } else if is_double_agenda(&words) {
        Some(StaticSpecialLineShape::DoubleAgenda)
    } else if is_starting_player(&words) {
        Some(StaticSpecialLineShape::StartingPlayer)
    } else if is_draft_rule(&words) {
        Some(StaticSpecialLineShape::DraftRule)
    } else if parse_commander_deck_allowance_words(&words).is_some() {
        Some(StaticSpecialLineShape::CommanderDeckAllowance)
    } else if let Some(rule) = DeckConstructionRule::from_words(&words) {
        Some(StaticSpecialLineShape::DeckConstruction(rule))
    } else if super::parse_first_keyword_cost_alternative_tokens(tokens).is_some() {
        Some(StaticSpecialLineShape::FirstEquipCostAlternative)
    } else if let Some(prefix) =
        crate::grammar::abilities::split_as_long_as_condition_prefix_lexed(tokens)
        && super::parse_first_keyword_cost_alternative_tokens(prefix.remainder_tokens).is_some()
    {
        // "As long as you have an enduring story, you may pay {0} rather than
        // pay the equip cost …" (Kíli the Resourceful).
        Some(StaticSpecialLineShape::FirstEquipCostAlternative)
    } else {
        None
    }
}

pub fn parse_combined_spell_and_activation_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CombinedSpellAndActivationTax> {
    let words = parser_token_word_refs(tokens);
    (every_phrase_is_present(
        &words,
        &[
            &["and", "abilities"],
            &["activate", "cost"],
            &["more", "to", "activate"],
        ],
    ) && any_word_is_present(&words, &["spell", "spells"]))
    .then_some(CombinedSpellAndActivationTax)
}

pub fn parse_source_keyword_tail_tokens(tokens: &[OwnedLexToken]) -> Option<SourceKeywordTail<'_>> {
    let words = parser_token_word_refs(tokens);
    let has_word =
        phrase_location(&words, &["has"]).or_else(|| phrase_location(&words, &["have"]))?;
    if has_word == 0 || !is_source_reference_words(&words[..has_word]) {
        return None;
    }
    let ability_start = has_word + 1;
    if phrase_is_present(&words[ability_start..], &["as", "long", "as"])
        || phrase_is_present(&words[ability_start..], &["during", "your", "turn"])
    {
        return None;
    }
    let view = TokenWordView::new(tokens);
    let range = view.token_span_for_words(ability_start, words.len())?;
    let ability_tokens = super::super::super::lexer::trim_lexed_commas(&tokens[range]);
    (!ability_tokens.is_empty()).then_some(SourceKeywordTail { ability_tokens })
}

pub fn parse_skip_keyword_action_probe_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SkipKeywordActionProbe> {
    let words = parser_token_word_refs(tokens);
    ((phrase_is_suffix(&words, &["can't", "be", "blocked"])
        || phrase_is_suffix(&words, &["cant", "be", "blocked"]))
        && !(phrase_is_prefix(&words, &["this"]) || phrase_is_prefix(&words, &["it"])))
    .then_some(SkipKeywordActionProbe)
}

pub fn parse_additional_land_play_count_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    let words = parser_token_word_refs(tokens);
    if !phrase_is_prefix(&words, &["you", "may", "play"]) {
        return None;
    }
    if phrase_is_exact(
        words.get(3..)?,
        &["any", "number", "of", "lands", "on", "each", "of", "your", "turns"],
    ) {
        crate::word_claims::claim_words(&words);
        return Some(u32::MAX);
    }
    let (number, used) = leaf::parse_leaf_number_prefix_words(words.get(3..)?)?.into_fixed()?;
    let tail = words.get(3 + used..)?;
    if phrase_is_exact(
        tail,
        &["additional", "land", "on", "each", "of", "your", "turns"],
    ) || phrase_is_exact(
        tail,
        &["additional", "lands", "on", "each", "of", "your", "turns"],
    ) {
        // The line matched word for word.
        crate::word_claims::claim_words(&words);
        Some(number)
    } else {
        None
    }
}

pub fn parse_ability_word_marker_tokens(tokens: &[OwnedLexToken]) -> Option<AbilityWordMarker> {
    if tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Period
                | TokenKind::Colon
                | TokenKind::Dash
                | TokenKind::EmDash
                | TokenKind::Comma
                | TokenKind::Semicolon
        )
    }) {
        return None;
    }
    let word_count = TokenWordView::new(tokens).len();
    (word_count > 0 && word_count <= 4).then_some(AbilityWordMarker)
}

pub fn parse_level_up_intro_tokens(tokens: &[OwnedLexToken]) -> Option<LevelUpIntro> {
    phrase_is_prefix(&parser_token_word_refs(tokens), &["level", "up"]).then_some(LevelUpIntro)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_special_static_shapes() {
        let equip = lex_line(
            "You may activate equip abilities any time you could cast an instant.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_static_special_line_tokens(&equip),
            Some(StaticSpecialLineShape::EquipAtInstantSpeed)
        );

        let deck = lex_line(
            "A deck can have any number of cards named Relentless Rats.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_static_special_line_tokens(&deck),
            Some(StaticSpecialLineShape::DeckConstruction(DeckConstructionRule::CopyLimit(None)))
        );
    }

    #[test]
    fn parses_source_tail_and_land_count() {
        let source = lex_line("This creature has flying and vigilance.", 0).unwrap();
        let tail = parse_source_keyword_tail_tokens(&source).unwrap();
        assert_eq!(
            parser_token_word_refs(tail.ability_tokens),
            vec!["flying", "and", "vigilance"]
        );

        let timed = lex_line("This creature has first strike during your turn.", 0).unwrap();
        assert!(
            parse_source_keyword_tail_tokens(&timed).is_none(),
            "a timed source keyword must reach the typed conditional static parser"
        );

        let land = lex_line(
            "You may play two additional lands on each of your turns.",
            0,
        )
        .unwrap();
        assert_eq!(parse_additional_land_play_count_tokens(&land), Some(2));
    }
}
