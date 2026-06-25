#![allow(dead_code)]

use crate::cards::builders::{
    CardTextError, EffectAst, KeywordAction, LineAst, PredicateAst, StaticAbilityAst,
    SubjectVerbActionAst, TargetAst, TriggerSpec,
};
use crate::effect::{EventValueSpec, Value};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::activation_and_restrictions::keyword_action_costs::maybe_strip_leading_damage_subject_tokens;
use super::activation_and_restrictions::{
    parse_ability_phrase, parse_named_number, parse_single_word_keyword_action,
    parse_triggered_times_each_turn_lexed,
};
use super::grammar::primitives::{
    TokenWordView, find_token_index, split_lexed_slices_on_and,
    split_lexed_slices_on_commas_or_semicolons,
};
use super::grammar::structure::{
    find_trigger_effect_list_tail_split_lexed,
    rewrite_attached_controller_trigger_effect_tokens_lexed,
    split_first_time_each_turn_trigger_suffix_lexed, split_state_triggered_clause_lexed,
    split_triggered_conditional_clause_lexed,
};
use super::lex_patterns::{LexCaptureKind, LexPattern};
use super::lexer::{
    LexedClause, OwnedLexToken, TokenKind, render_token_slice, split_lexed_sentences,
};
use super::object_filters::parse_object_filter_lexed;
use super::util::{
    parse_card_type, parse_color, parse_filter_counter_constraint_words,
    parse_flashback_keyword_line, parse_subtype_flexible, strip_leading_word_refs_any, trim_commas,
};
use super::value_helpers::parse_filter_comparison_tokens;

const PROTECTION_FROM_COLORED_SPELLS_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&[
        "protection",
        "from",
        "spells",
        "that",
        "are",
        "one",
        "or",
        "more",
        "colors",
    ])]);
const PROTECTION_EACH_MANA_VALUE_AMONG_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["protection", "from", "each", "mana", "value", "among"]),
    LexPattern::object("filter", LexCaptureKind::Rest),
]);
const PROTECTION_FROM_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["protection", "from"])]);
const EACH_MANA_VALUE_AMONG_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["each", "mana", "value", "among"]),
    LexPattern::object("filter", LexCaptureKind::Rest),
]);
const CHOSEN_PLAYER_TAIL_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["the", "chosen", "player"])]);
const ALL_COLORS_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::any_phrase(&[
    &["all", "color"],
    &["all", "colors"],
])]);
const TWO_WORD_KEYWORD_ACTIONS: &[(&[&str], KeywordAction)] = &[
    (&["first", "strike"], KeywordAction::FirstStrike),
    (&["double", "strike"], KeywordAction::DoubleStrike),
    (&["battle", "cry"], KeywordAction::BattleCry),
    (&["split", "second"], KeywordAction::SplitSecond),
    (&["read", "ahead"], KeywordAction::ReadAhead),
    (&["for", "mirrodin"], KeywordAction::ForMirrodin),
    (&["living", "weapon"], KeywordAction::LivingWeapon),
    (&["umbra", "armor"], KeywordAction::UmbraArmor),
    (
        &["doctor", "companion"],
        KeywordAction::Marker("doctor companion"),
    ),
];

fn predicate_counts_creatures_died_this_turn(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::CreatureDiedThisTurn | PredicateAst::CreatureDiedThisTurnOrMore(_)
    )
}

fn bind_event_amount_to_creatures_died_this_turn(value: &mut Value) {
    match value {
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            *value = Value::CreaturesDiedThisTurn;
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            *value = Value::Add(
                Box::new(Value::CreaturesDiedThisTurn),
                Box::new(Value::Fixed(*offset)),
            );
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            bind_event_amount_to_creatures_died_this_turn(left);
            bind_event_amount_to_creatures_died_this_turn(right);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner)
        | Value::SurfaceHinted { value: inner, .. } => {
            bind_event_amount_to_creatures_died_this_turn(inner);
        }
        _ => {}
    }
}

fn bind_creatures_died_condition_amounts(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::GainLife { amount }
                | SubjectVerbActionAst::PutCounters { count: amount, .. } => {
                    bind_event_amount_to_creatures_died_this_turn(amount);
                }
                _ => {}
            },
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                bind_creatures_died_condition_amounts(if_true);
                bind_creatures_died_condition_amounts(if_false);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AttackedPlayerFilterKind {
    Any,
    Enchanted,
    Opponent,
    You,
}

const ATTACKED_PLAYER_FILTERS: &[(&[&str], AttackedPlayerFilterKind)] = &[
    (
        &["enchanted", "player"],
        AttackedPlayerFilterKind::Enchanted,
    ),
    (&["you"], AttackedPlayerFilterKind::You),
    (&["an", "opponent"], AttackedPlayerFilterKind::Opponent),
    (&["opponent"], AttackedPlayerFilterKind::Opponent),
    (&["a", "player"], AttackedPlayerFilterKind::Any),
    (&["player"], AttackedPlayerFilterKind::Any),
    (&["any", "player"], AttackedPlayerFilterKind::Any),
];
const CASUALTY_PLANESWALKER_COPY_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&[
        "casualty",
        "x",
        "the",
        "copy",
        "isnt",
        "legendary",
        "and",
        "has",
        "starting",
        "loyalty",
        "x",
    ])]);

fn two_word_keyword_action(words: &[&str]) -> Option<KeywordAction> {
    TWO_WORD_KEYWORD_ACTIONS
        .iter()
        .find_map(|(phrase, action)| (*phrase == words).then(|| action.clone()))
}

fn attacked_player_filter_from_words(words: &[&str]) -> Option<PlayerFilter> {
    ATTACKED_PLAYER_FILTERS
        .iter()
        .find_map(|(phrase, filter)| (*phrase == words).then_some(*filter))
        .map(|filter| match filter {
            AttackedPlayerFilterKind::Any => PlayerFilter::Any,
            AttackedPlayerFilterKind::Enchanted => {
                PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))
            }
            AttackedPlayerFilterKind::Opponent => PlayerFilter::Opponent,
            AttackedPlayerFilterKind::You => PlayerFilter::You,
        })
}
const READ_AHEAD_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["read", "ahead"])]);
const SPLICE_ONTO_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["splice", "onto"])]);
const MONSTROUS_DAMAGE_HAND_TRIGGER_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&[
        "when",
        "this",
        "becomes",
        "monstrous",
        "it",
        "deals",
        "damage",
        "to",
        "each",
        "opponent",
        "equal",
        "to",
    ])]);
const MONSTROUS_DAMAGE_HAND_TRIGGER_MARKER_WORDS: &[&str] = &["number", "cards", "hand"];
const THIS_BECOMES_BLOCKED_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["this", "becomes", "blocked"])]);
const THIS_CREATURE_BECOMES_BLOCKED_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["this", "creature", "becomes", "blocked"]),
]);
const THIS_LEAVES_BATTLEFIELD_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["this", "leaves", "the", "battlefield"]),
]);
const THIS_CREATURE_LEAVES_BATTLEFIELD_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["this", "creature", "leaves", "the", "battlefield"]),
]);

fn is_and_word(word: &str) -> bool {
    word == "and"
}

fn is_from_word(word: &str) -> bool {
    word == "from"
}

fn is_with_word(word: &str) -> bool {
    word == "with"
}

fn is_permanent_word(word: &str) -> bool {
    matches!(word, "permanent" | "permanents")
}

fn is_trigger_intro_word(word: &str) -> bool {
    matches!(word, "whenever" | "at" | "when")
}

fn is_attack_word(word: &str) -> bool {
    matches!(word, "attack" | "attacks")
}

fn is_keyword_with_count_word(word: &str) -> bool {
    matches!(
        word,
        "ward"
            | "toxic"
            | "afflict"
            | "afterlife"
            | "fabricate"
            | "renown"
            | "backup"
            | "bushido"
            | "bloodthirst"
    )
}

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|_| token.parser_text() == expected)
}

fn clause_support_words_start_with_pattern<'a>(words: &[&str], pattern: LexPattern<'a>) -> bool {
    pattern.match_prefix_word_refs(words).is_some()
}

fn clause_support_words_contain_all(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .all(|expected_word| words.iter().any(|word| word == expected_word))
}

fn protection_from_colored_spells_action(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    if !PROTECTION_FROM_COLORED_SPELLS_PATTERN.matches(LexedClause::new(tokens)) {
        return None;
    }

    let mut filter = ObjectFilter::spell();
    filter.colors = Some(all_magic_colors());
    Some(KeywordAction::ProtectionFromFilter(filter))
}

fn all_magic_colors() -> crate::color::ColorSet {
    crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN)
}

fn protection_from_each_mana_value_among_action(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    protection_each_mana_value_among_filter(tokens, PROTECTION_EACH_MANA_VALUE_AMONG_PATTERN)
        .map(KeywordAction::ProtectionFromEachManaValueAmong)
}

fn protection_from_each_mana_value_among_tail_action(
    tokens: &[OwnedLexToken],
) -> Option<KeywordAction> {
    protection_each_mana_value_among_filter(tokens, EACH_MANA_VALUE_AMONG_PATTERN)
        .map(KeywordAction::ProtectionFromEachManaValueAmong)
}

fn protection_each_mana_value_among_filter(
    tokens: &[OwnedLexToken],
    pattern: LexPattern<'static>,
) -> Option<ObjectFilter> {
    let clause = LexedClause::new(tokens);
    let matched = pattern.match_clause(clause)?;
    let filter_clause = matched.capture_clause("filter", clause)?.trimmed();
    if filter_clause.is_empty() {
        return None;
    }
    let filter_tokens = trim_commas(filter_clause.tokens());
    parse_object_filter_lexed(&filter_tokens, false).ok()
}

fn parse_protection_chain(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    let first_word_idx = if words.first().is_some_and(|word| is_and_word(word)) {
        1
    } else {
        0
    };
    if words.len().saturating_sub(first_word_idx) < 3 {
        return None;
    }
    if !clause_support_words_start_with_pattern(
        &words[first_word_idx..],
        PROTECTION_FROM_PREFIX_PATTERN,
    ) {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        if let Some(target_start) = words_view.token_index_for_word_index(idx + 1)
            && let Some(action) =
                protection_from_each_mana_value_among_tail_action(&tokens[target_start..])
        {
            return Some(action);
        }
        if value == "spells" || value == "spell" {
            return Some(KeywordAction::ProtectionFromFilter(ObjectFilter::spell()));
        }
        if matches!(value, "permanent" | "permanents")
            && words.get(idx + 2..idx + 7) == Some(&["that", "were", "cast", "this", "turn"][..])
        {
            let mut filter = ObjectFilter::permanent();
            filter.cast_this_turn = true;
            return Some(KeywordAction::ProtectionFromFilter(filter));
        }
        if value == "mana" && words.get(idx + 2).copied() == Some("value") {
            let comparison_tail = words.get(idx + 3..)?;
            let (comparison, consumed) =
                parse_filter_comparison_tokens("mana value", comparison_tail, words).ok()??;
            if consumed == comparison_tail.len() {
                let mut filter = ObjectFilter::default();
                filter.mana_value = Some(comparison);
                return Some(KeywordAction::ProtectionFromFilter(filter));
            }
        }
        if is_permanent_word(value) && words.get(idx + 2).is_some_and(|word| is_with_word(word)) {
            let counter_words = &words[idx + 3..];
            if let Some((with_counter, consumed)) =
                parse_filter_counter_constraint_words(counter_words)
                && consumed == counter_words.len()
            {
                let mut filter = ObjectFilter::permanent();
                filter.with_counter = Some(with_counter);
                return Some(KeywordAction::ProtectionFromFilter(filter));
            }
        }
        match value {
            _ if clause_support_words_start_with_pattern(
                words.get(idx + 1..)?,
                CHOSEN_PLAYER_TAIL_PATTERN,
            ) =>
            {
                Some(KeywordAction::ProtectionFromChosenPlayer)
            }
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "everything" => Some(KeywordAction::ProtectionFromEverything),
            _ if clause_support_words_start_with_pattern(
                words.get(idx + 1..)?,
                ALL_COLORS_PATTERN,
            ) =>
            {
                Some(KeywordAction::ProtectionFromAllColors)
            }
            _ => parse_color(value)
                .map(KeywordAction::ProtectionFrom)
                .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
                .or_else(|| {
                    parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)
                }),
        }
    };

    let mut from_count = 0usize;
    let mut parsed_count = 0usize;
    for idx in first_word_idx..words.len().saturating_sub(1) {
        if !is_from_word(words[idx]) {
            continue;
        }
        from_count += 1;
        if let Some(action) = parse_from_target(&words, idx) {
            parsed_count += 1;
            crate::slice_primitives::push_unique(&mut actions, action);
        }
    }

    if actions.is_empty() || parsed_count < from_count {
        None
    } else {
        Some(actions)
    }
}

fn color_only_hexproof_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    if matches!(words, ["each", "color"]) {
        let mut filter = ObjectFilter::default();
        filter.colors = Some(all_magic_colors());
        return Some(filter);
    }

    let mut filters = Vec::new();
    for word in words {
        if is_and_word(word) || is_from_word(word) {
            continue;
        }
        let color = crate::color::Color::from_name(word)?;
        let mut filter = ObjectFilter::default();
        filter.colors = Some(crate::color::ColorSet::from_color(color));
        filters.push(filter);
    }

    match filters.len() {
        0 => None,
        1 => filters.pop(),
        _ => {
            let mut filter = ObjectFilter::default();
            filter.any_of = filters;
            Some(filter)
        }
    }
}

fn parse_hexproof_from_chain(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words_view = TokenWordView::new(tokens);
    let words = words_view.word_refs();
    let first_word_idx = if words.first().is_some_and(|word| is_and_word(word)) {
        1
    } else {
        0
    };
    if words.len().saturating_sub(first_word_idx) < 3
        || words.get(first_word_idx).copied() != Some("hexproof")
        || words.get(first_word_idx + 1).copied() != Some("from")
    {
        return None;
    }

    color_only_hexproof_filter_words(&words[first_word_idx + 2..])
        .map(|filter| vec![KeywordAction::HexproofFrom(filter)])
}

pub(crate) fn rewrite_parse_ability_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }

    let segments = split_lexed_slices_on_commas_or_semicolons(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(hexproof_actions) = parse_hexproof_from_chain(segment) {
            actions.extend(hexproof_actions);
            continue;
        }

        if let Some(action) = parse_ability_phrase(segment) {
            actions.push(action);
        } else {
            let and_parts = split_lexed_slices_on_and(segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in &and_parts {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(action) = parse_ability_phrase(part) {
                        actions.push(action);
                    } else {
                        all_ok = false;
                        break;
                    }
                }
                if !all_ok {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn parse_ability_line_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    fn parse_simple_keyword_phrase_lexed(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
        let words_view = TokenWordView::new(tokens);
        let words = words_view.word_refs();
        let words = strip_leading_word_refs_any(&words, &["and"]);
        if words.is_empty() {
            return None;
        }

        if clause_support_words_start_with_pattern(&words, CASUALTY_PLANESWALKER_COPY_PATTERN) {
            return Some(KeywordAction::VariableCasualtyPlaneswalkerCopy);
        }

        if words.len() == 1 {
            return parse_single_word_keyword_action(words[0]);
        }

        let parse_count_keyword =
            |expected: &str, ctor: fn(u32) -> KeywordAction| -> Option<KeywordAction> {
                if !words
                    .first()
                    .is_some_and(|word| is_keyword_with_count_word(word))
                    || !is_keyword_with_count_word(expected)
                    || !words.first().is_some_and(|word| *word == expected)
                {
                    return None;
                }
                words
                    .get(1)
                    .and_then(|word| parse_named_number(word))
                    .map(ctor)
            };

        if let Some(action) = parse_count_keyword("ward", KeywordAction::Ward) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("toxic", KeywordAction::Toxic) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("afflict", KeywordAction::Afflict) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("afterlife", KeywordAction::Afterlife) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("fabricate", KeywordAction::Fabricate) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("renown", KeywordAction::Renown) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("backup", KeywordAction::Backup) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("bushido", KeywordAction::Bushido) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("bloodthirst", KeywordAction::Bloodthirst) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("tribute", KeywordAction::Tribute) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("rampage", KeywordAction::Rampage) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("annihilator", KeywordAction::Annihilator) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("soulshift", KeywordAction::Soulshift) {
            return Some(action);
        }
        if let Some(action) = super::activation_and_restrictions::keyword_action_costs::parse_dynamic_soulshift_keyword_action(&words)
        {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("modular", KeywordAction::Modular) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("graft", KeywordAction::Graft) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("fading", KeywordAction::Fading) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("vanishing", KeywordAction::Vanishing) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("mobilize", KeywordAction::Mobilize) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("casualty", KeywordAction::Casualty) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("amplify", KeywordAction::Amplify) {
            return Some(action);
        }
        if let Some(action) = parse_count_keyword("devour", KeywordAction::Devour) {
            return Some(action);
        }
        if words.first().is_some_and(|word| *word == "dredge")
            && let Some(amount) = words.get(1)
            && parse_named_number(amount).is_some()
        {
            return Some(KeywordAction::MarkerText(format!("Dredge {amount}")));
        }

        if clause_support_words_start_with_pattern(&words, READ_AHEAD_PATTERN) {
            return Some(KeywordAction::ReadAhead);
        }

        if let Some(action) = two_word_keyword_action(words) {
            return Some(action);
        }

        None
    }

    fn parse_flashback_keyword_line_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        if !tokens
            .first()
            .is_some_and(|token| token_is_word(token, "flashback"))
        {
            return None;
        }
        let mut idx = 1usize;
        let mut cost = String::new();
        while let Some(token) = tokens.get(idx) {
            if token.kind != TokenKind::ManaGroup {
                break;
            }
            cost.push_str(token.slice.as_str());
            idx += 1;
        }
        if cost.is_empty() {
            return None;
        }

        let tail_view = TokenWordView::new(&tokens[idx..]);
        let tail = tail_view.word_refs();
        let mut text = format!("Flashback {cost}");
        if !tail.is_empty() {
            let mut tail_text = tail.join(" ");
            if let Some(first) = tail_text.chars().next() {
                let upper = first.to_ascii_uppercase().to_string();
                let rest = &tail_text[first.len_utf8()..];
                tail_text = format!("{upper}{rest}");
            }
            text.push_str(", ");
            text.push_str(&tail_text);
        }
        Some(vec![KeywordAction::MarkerText(text)])
    }

    fn parse_splice_keyword_line_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        let words = TokenWordView::new(tokens);
        if !clause_support_words_start_with_pattern(&words.word_refs(), SPLICE_ONTO_PATTERN) {
            return None;
        }

        let mut text = render_token_slice(tokens).trim().to_string();
        if let Some(reminder_start) = text.find(" (") {
            text.truncate(reminder_start);
        }
        (!text.is_empty()).then_some(vec![KeywordAction::MarkerText(text)])
    }

    fn parse_protection_chain_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        let words_view = TokenWordView::new(tokens);
        let words = words_view.word_refs();
        if let Some(action) = protection_from_colored_spells_action(tokens) {
            return Some(vec![action]);
        }
        if let Some(action) = protection_from_each_mana_value_among_action(tokens) {
            return Some(vec![action]);
        }
        let first_word_idx = if words.first().is_some_and(|word| is_and_word(word)) {
            1
        } else {
            0
        };
        if words.len().saturating_sub(first_word_idx) < 3
            || !clause_support_words_start_with_pattern(
                &words[first_word_idx..],
                PROTECTION_FROM_PREFIX_PATTERN,
            )
        {
            return None;
        }

        let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
            let value = *words.get(idx + 1)?;
            if let Some(target_start) = words_view.token_index_for_word_index(idx + 1)
                && let Some(action) =
                    protection_from_each_mana_value_among_tail_action(&tokens[target_start..])
            {
                return Some(action);
            }
            if value == "spells" || value == "spell" {
                return Some(KeywordAction::ProtectionFromFilter(ObjectFilter::spell()));
            }
            if matches!(value, "permanent" | "permanents")
                && words.get(idx + 2..idx + 7)
                    == Some(&["that", "were", "cast", "this", "turn"][..])
            {
                let mut filter = ObjectFilter::permanent();
                filter.cast_this_turn = true;
                return Some(KeywordAction::ProtectionFromFilter(filter));
            }
            if value == "mana" && words.get(idx + 2).copied() == Some("value") {
                let comparison_tail = words.get(idx + 3..)?;
                let (comparison, consumed) =
                    parse_filter_comparison_tokens("mana value", comparison_tail, &words)
                        .ok()??;
                if consumed == comparison_tail.len() {
                    let mut filter = ObjectFilter::default();
                    filter.mana_value = Some(comparison);
                    return Some(KeywordAction::ProtectionFromFilter(filter));
                }
            }
            if is_permanent_word(value) && words.get(idx + 2).is_some_and(|word| is_with_word(word))
            {
                let counter_words = &words[idx + 3..];
                if let Some((with_counter, consumed)) =
                    parse_filter_counter_constraint_words(counter_words)
                    && consumed == counter_words.len()
                {
                    let mut filter = ObjectFilter::permanent();
                    filter.with_counter = Some(with_counter);
                    return Some(KeywordAction::ProtectionFromFilter(filter));
                }
            }
            match value {
                _ if clause_support_words_start_with_pattern(
                    words.get(idx + 1..)?,
                    CHOSEN_PLAYER_TAIL_PATTERN,
                ) =>
                {
                    Some(KeywordAction::ProtectionFromChosenPlayer)
                }
                "colorless" => Some(KeywordAction::ProtectionFromColorless),
                "everything" => Some(KeywordAction::ProtectionFromEverything),
                _ if clause_support_words_start_with_pattern(
                    words.get(idx + 1..)?,
                    ALL_COLORS_PATTERN,
                ) =>
                {
                    Some(KeywordAction::ProtectionFromAllColors)
                }
                _ => parse_color(value)
                    .map(KeywordAction::ProtectionFrom)
                    .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
                    .or_else(|| {
                        parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)
                    }),
            }
        };

        let mut actions = Vec::new();
        let mut from_count = 0usize;
        let mut parsed_count = 0usize;
        for idx in first_word_idx..words.len().saturating_sub(1) {
            if !is_from_word(words[idx]) {
                continue;
            }
            from_count += 1;
            if let Some(action) = parse_from_target(&words, idx) {
                parsed_count += 1;
                crate::slice_primitives::push_unique(&mut actions, action);
            }
        }

        if actions.is_empty() || parsed_count < from_count {
            None
        } else {
            Some(actions)
        }
    }

    fn parse_hexproof_from_chain_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
        parse_hexproof_from_chain(tokens)
    }

    fn split_on_lexed_comma_or_semicolon(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
        let mut segments = Vec::new();
        let mut start = 0usize;
        for (idx, token) in tokens.iter().enumerate() {
            if matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
                if start < idx {
                    segments.push(&tokens[start..idx]);
                }
                start = idx + 1;
            }
        }
        if start < tokens.len() {
            segments.push(&tokens[start..]);
        }
        segments
    }

    fn split_on_lexed_and(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
        let mut segments = Vec::new();
        let mut start = 0usize;
        for (idx, token) in tokens.iter().enumerate() {
            if token_is_word(token, "and") {
                let segment = &tokens[start..idx];
                if !segment.is_empty() {
                    segments.push(segment);
                }
                start = idx + 1;
            }
        }
        let tail = &tokens[start..];
        if !tail.is_empty() {
            segments.push(tail);
        }
        segments
    }

    if let Some(actions) = parse_flashback_keyword_line_lexed(tokens) {
        return Some(actions);
    }
    if let Some(actions) = parse_splice_keyword_line_lexed(tokens) {
        return Some(actions);
    }
    let words = TokenWordView::new(tokens).word_refs();
    if let Some(action) =
        super::activation_and_restrictions::keyword_action_costs::parse_dynamic_soulshift_keyword_action(&words)
    {
        return Some(vec![action]);
    }
    if let Some(action @ KeywordAction::CumulativeUpkeep { .. }) = parse_ability_phrase(tokens) {
        return Some(vec![action]);
    }

    let segments = split_on_lexed_comma_or_semicolon(tokens);
    let mut actions = Vec::new();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        if let Some(protection_actions) = parse_protection_chain_lexed(segment) {
            actions.extend(protection_actions);
            continue;
        }

        if let Some(hexproof_actions) = parse_hexproof_from_chain_lexed(segment) {
            actions.extend(hexproof_actions);
            continue;
        }

        if let Some(action) = parse_simple_keyword_phrase_lexed(segment) {
            actions.push(action);
            continue;
        }

        let and_parts = split_on_lexed_and(segment);
        if and_parts.len() > 1 {
            let mut all_ok = true;
            for part in &and_parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(action) = parse_simple_keyword_phrase_lexed(part) {
                    actions.push(action);
                    continue;
                }
                if let Some(action) = parse_ability_phrase(part) {
                    actions.push(action);
                } else {
                    all_ok = false;
                    break;
                }
            }
            if !all_ok {
                return None;
            }
            continue;
        }

        if let Some(action) = parse_ability_phrase(segment) {
            actions.push(action);
            continue;
        }

        let and_parts = split_on_lexed_and(segment);
        if and_parts.len() > 1 {
            let mut all_ok = true;
            for part in &and_parts {
                if part.is_empty() {
                    continue;
                }
                if let Some(action) = parse_ability_phrase(part) {
                    actions.push(action);
                } else {
                    all_ok = false;
                    break;
                }
            }
            if !all_ok {
                return None;
            }
        } else {
            return None;
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::effect_sentences::parse_effect_sentences_lexed(tokens)
}

fn parse_effect_sentences_or_single_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_sentences_lexed(tokens).or_else(|original_err| {
        let sentences = split_lexed_sentences(tokens);
        if sentences.len() == 1 {
            super::effect_sentences::parse_effect_sentence_lexed(sentences[0])
        } else {
            Err(original_err)
        }
    })
}

pub(crate) fn parse_triggered_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.word_refs();
    if clause_support_words_start_with_pattern(
        &clause_words,
        MONSTROUS_DAMAGE_HAND_TRIGGER_PREFIX_PATTERN,
    ) && clause_support_words_contain_all(
        &clause_words,
        MONSTROUS_DAMAGE_HAND_TRIGGER_MARKER_WORDS,
    ) {
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::ThisBecomesMonstrous,
            effects: vec![EffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_damage(
                    Value::CardsInHand(PlayerFilter::IteratedPlayer),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                )],
            }],
            max_triggers_per_turn: None,
        });
    }

    fn parse_triggered_times_each_turn_lexed_from_sentences(
        tokens: &[OwnedLexToken],
    ) -> Option<u32> {
        split_lexed_sentences(tokens)
            .iter()
            .find_map(|sentence| parse_triggered_times_each_turn_lexed(sentence))
    }

    let token_words = TokenWordView::new(tokens).word_refs();
    let start_idx = if token_words
        .first()
        .is_some_and(|word| is_trigger_intro_word(word))
    {
        1
    } else {
        0
    };

    let normalized_token_words: Vec<String> = token_words
        .iter()
        .map(|word| word.replace(['\'', '’'], ""))
        .collect();
    let contains_ordered_phrase = |phrase: &[&str]| -> bool {
        normalized_token_words
            .windows(phrase.len())
            .any(|window| window == phrase)
    };
    if contains_ordered_phrase(&[
        "you", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate", "an", "ability",
    ]) && contains_ordered_phrase(&[
        "that",
        "spells",
        "mana",
        "cost",
        "or",
        "that",
        "abilitys",
        "activation",
        "cost",
        "contains",
    ]) && let Some(copy_word_idx) = normalized_token_words
        .windows(5)
        .position(|window| window == ["copy", "that", "spell", "or", "ability"])
        && let Some(effect_start) =
            TokenWordView::new(tokens).token_index_for_word_index(copy_word_idx)
    {
        let mut spell_filter = ObjectFilter::instant_or_sorcery();
        spell_filter.has_x_in_cost = true;
        let mut ability_filter = ObjectFilter::default();
        ability_filter.has_x_in_cost = true;
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::Either(
                Box::new(TriggerSpec::SpellCast {
                    filter: Some(spell_filter),
                    caster: PlayerFilter::You,
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                }),
                Box::new(TriggerSpec::AbilityActivated {
                    activator: PlayerFilter::You,
                    filter: ability_filter,
                    non_mana_only: false,
                    loyalty_only: false,
                    activation_cost_has_tap: None,
                }),
            ),
            effects: parse_effect_sentences_lexed(&tokens[effect_start..])?,
            max_triggers_per_turn: None,
        });
    }

    if start_idx < tokens.len() {
        let trigger_body = &tokens[start_idx..];
        let trigger_body_view = TokenWordView::new(trigger_body);
        let trigger_body_words = trigger_body_view.word_refs();
        let blocked_prefix_len = if clause_support_words_start_with_pattern(
            &trigger_body_words,
            THIS_CREATURE_BECOMES_BLOCKED_PREFIX_PATTERN,
        ) {
            Some(4usize)
        } else if clause_support_words_start_with_pattern(
            &trigger_body_words,
            THIS_BECOMES_BLOCKED_PREFIX_PATTERN,
        ) {
            Some(3usize)
        } else {
            None
        };
        if let Some(prefix_len) = blocked_prefix_len
            && let Some(effect_start_rel) =
                trigger_body_view.token_index_after_words_or_end(prefix_len)
        {
            let split_idx = start_idx + effect_start_rel;
            let effects_tokens = trim_commas(&tokens[split_idx..]);
            let effect_words = TokenWordView::new(&effects_tokens).word_refs();
            if effect_words.as_slice()
                == [
                    "it",
                    "deals",
                    "2",
                    "damage",
                    "to",
                    "each",
                    "attacking",
                    "creature",
                    "and",
                    "each",
                    "blocking",
                    "creature",
                ]
            {
                let attacking = ObjectFilter {
                    zone: Some(Zone::Battlefield),
                    card_types: vec![CardType::Creature],
                    attacking: true,
                    ..ObjectFilter::default()
                };
                let blocking = ObjectFilter {
                    zone: Some(Zone::Battlefield),
                    card_types: vec![CardType::Creature],
                    blocking: true,
                    ..ObjectFilter::default()
                };
                return Ok(LineAst::Triggered {
                    trigger: TriggerSpec::ThisBecomesBlocked,
                    effects: vec![
                        EffectAst::subject_verb_damage_each(Value::Fixed(2), attacking),
                        EffectAst::subject_verb_damage_each(Value::Fixed(2), blocking),
                    ],
                    max_triggers_per_turn: None,
                });
            }
            if !effects_tokens.is_empty()
                && let Ok(effects) = parse_effect_sentences_lexed(&effects_tokens)
            {
                return Ok(LineAst::Triggered {
                    trigger: TriggerSpec::ThisBecomesBlocked,
                    effects,
                    max_triggers_per_turn: None,
                });
            }
        }

        let leaves_prefix_len = if clause_support_words_start_with_pattern(
            &trigger_body_words,
            THIS_LEAVES_BATTLEFIELD_PREFIX_PATTERN,
        ) {
            Some(4usize)
        } else if clause_support_words_start_with_pattern(
            &trigger_body_words,
            THIS_CREATURE_LEAVES_BATTLEFIELD_PREFIX_PATTERN,
        ) {
            Some(5usize)
        } else {
            None
        };
        if let Some(prefix_len) = leaves_prefix_len
            && let Some(effect_start_rel) =
                trigger_body_view.token_index_after_words_or_end(prefix_len)
        {
            let split_idx = start_idx + effect_start_rel;
            let trigger_tokens = trim_commas(&tokens[start_idx..split_idx]);
            let effects_tokens = trim_commas(&tokens[split_idx..]);
            if !effects_tokens.is_empty()
                && let Ok(trigger) = parse_trigger_clause_lexed(&trigger_tokens)
                && let Ok(effects) = parse_effect_sentences_lexed(&effects_tokens)
            {
                return Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: None,
                });
            }
        }
    }

    if let Some(spec) = split_triggered_conditional_clause_lexed(tokens, start_idx) {
        let (trigger_tokens, max_triggers_from_trigger_clause) =
            split_first_time_each_turn_trigger_suffix_lexed(spec.trigger_tokens);
        if let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) {
            let rewritten_effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                spec.effects_tokens,
            );
            if let Ok(mut effects) =
                parse_effect_sentences_or_single_sentence_lexed(&rewritten_effects_tokens)
            {
                if predicate_counts_creatures_died_this_turn(&spec.predicate) {
                    bind_creatures_died_condition_amounts(&mut effects);
                }
                let mut max_triggers_per_turn =
                    parse_triggered_times_each_turn_lexed_from_sentences(&rewritten_effects_tokens);
                if let Some(max) = max_triggers_from_trigger_clause {
                    max_triggers_per_turn =
                        Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                }
                return Ok(LineAst::Triggered {
                    trigger,
                    effects: vec![EffectAst::Conditional {
                        predicate: spec.predicate,
                        if_true: effects,
                        if_false: Vec::new(),
                    }],
                    max_triggers_per_turn,
                });
            }
        }
    }

    if let Some(split_idx) = find_token_index(tokens, |token| token.kind == TokenKind::Comma) {
        let trigger_tokens = &tokens[start_idx..split_idx];
        let trigger_word_view = TokenWordView::new(trigger_tokens);
        let trigger_words = trigger_word_view.word_refs();
        let mut attack_idx = None;
        let mut word_idx = 0usize;
        while word_idx < trigger_words.len() {
            if is_attack_word(trigger_words[word_idx]) {
                attack_idx = Some(word_idx);
                break;
            }
            word_idx += 1;
        }
        if let Some(attack_idx) = attack_idx
            && trigger_words
                .get(attack_idx + 1)
                .is_some_and(|word| is_with_word(word))
        {
            let subject_words = &trigger_words[..attack_idx];
            if let Some(player) =
                super::activation_and_restrictions::parse_trigger_subject_player_filter(
                    subject_words,
                )
            {
                let Some(with_object_start) =
                    trigger_word_view.token_index_for_word_index(attack_idx + 2)
                else {
                    return Err(CardTextError::ParseError(format!(
                        "missing attacking-object filter in trigger clause (clause: '{}')",
                        trigger_words.join(" ")
                    )));
                };
                let mut object_tokens = &trigger_tokens[with_object_start..];
                let mut min_total_attackers = None;
                let mut exact_total_attackers = None;
                let mut one_or_more = false;
                if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_or_more_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    object_tokens = stripped;
                    if count > 1 {
                        min_total_attackers = Some(count);
                    }
                } else if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_exactly_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    exact_total_attackers = Some(count);
                    object_tokens = stripped;
                }
                if !object_tokens.is_empty() {
                    let mut filter = super::object_filters::parse_object_filter_lexed(
                        object_tokens,
                        false,
                    )
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported attacking-object filter in trigger clause (clause: '{}')",
                            trigger_words.join(" ")
                        ))
                    })?;
                    if filter.controller.is_none() {
                        filter.controller = Some(player);
                    }
                    let trigger = if let Some(total_attackers) = exact_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithExactTotal {
                            filter,
                            total_attackers,
                        }
                    } else if let Some(min_total_attackers) = min_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithMinTotal {
                            filter,
                            min_total_attackers,
                        }
                    } else if one_or_more {
                        TriggerSpec::AttacksOneOrMore(filter)
                    } else {
                        TriggerSpec::Attacks(filter)
                    };
                    let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                        trigger_tokens,
                        &tokens[split_idx + 1..],
                    );
                    let effects = parse_effect_sentences_lexed(&effects_tokens)?;
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: None,
                    });
                }
            }
        }

        if let Some(attack_idx) = attack_idx
            && let Some(with_idx) = trigger_words[attack_idx + 1..]
                .iter()
                .position(|word| is_with_word(word))
                .map(|rel| attack_idx + 1 + rel)
            && with_idx > attack_idx + 1
        {
            let subject_words = &trigger_words[..attack_idx];
            let attacked_words = &trigger_words[attack_idx + 1..with_idx];
            if let (Some(player), Some(attacked_player)) = (
                super::activation_and_restrictions::parse_trigger_subject_player_filter(
                    subject_words,
                ),
                attacked_player_filter_from_words(attacked_words),
            ) {
                let Some(with_object_start) =
                    trigger_word_view.token_index_for_word_index(with_idx + 1)
                else {
                    return Err(CardTextError::ParseError(format!(
                        "missing attacking-object filter in trigger clause (clause: '{}')",
                        trigger_words.join(" ")
                    )));
                };
                let mut object_tokens = &trigger_tokens[with_object_start..];
                let mut min_total_attackers = None;
                let mut exact_total_attackers = None;
                let mut one_or_more = false;
                if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_or_more_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    object_tokens = stripped;
                    if count > 1 {
                        min_total_attackers = Some(count);
                    }
                } else if let Some((count, stripped)) =
                    super::activation_and_restrictions::parse_leading_exactly_quantifier(
                        object_tokens,
                    )
                {
                    one_or_more = true;
                    exact_total_attackers = Some(count);
                    object_tokens = stripped;
                }
                if !object_tokens.is_empty() {
                    let mut filter = super::object_filters::parse_object_filter_lexed(
                        object_tokens,
                        false,
                    )
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported attacking-object filter in trigger clause (clause: '{}')",
                            trigger_words.join(" ")
                        ))
                    })?;
                    if filter.controller.is_none() {
                        filter.controller = Some(player);
                    }
                    filter.attacking_player_or_planeswalker_controlled_by = Some(attacked_player);
                    filter.targets_only_player = Some(PlayerFilter::Any);
                    let trigger = if let Some(total_attackers) = exact_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithExactTotal {
                            filter,
                            total_attackers,
                        }
                    } else if let Some(min_total_attackers) = min_total_attackers {
                        TriggerSpec::AttacksOneOrMoreWithMinTotal {
                            filter,
                            min_total_attackers,
                        }
                    } else if one_or_more {
                        TriggerSpec::AttacksOneOrMore(filter)
                    } else {
                        TriggerSpec::Attacks(filter)
                    };
                    let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                        trigger_tokens,
                        &tokens[split_idx + 1..],
                    );
                    let effects = parse_effect_sentences_lexed(&effects_tokens)?;
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: None,
                    });
                }
            }
        }
    }

    if let Some(mut split_idx) = find_token_index(tokens, |token| token.kind == TokenKind::Comma)
        .or_else(|| find_token_index(tokens, |token| token_is_word(token, "then")))
    {
        let first_split_idx = split_idx;
        if tokens
            .get(split_idx)
            .is_some_and(|token| token.kind == TokenKind::Comma)
            && tokens.first().is_some_and(|token| {
                token
                    .as_word()
                    .is_some_and(|_| is_trigger_intro_word(token.parser_text()))
                    && token.parser_text() != "at"
            })
        {
            let trigger_prefix_tokens = &tokens[start_idx..split_idx];
            let tail = &tokens[split_idx + 1..];
            if let Some(next_comma_rel) =
                find_trigger_effect_list_tail_split_lexed(trigger_prefix_tokens, tail)
            {
                let candidate_idx = split_idx + 1 + next_comma_rel;
                if candidate_idx > start_idx && candidate_idx + 1 < tokens.len() {
                    split_idx = candidate_idx;
                }
            }
        }

        let (trigger_tokens, max_triggers_from_trigger_clause) =
            split_first_time_each_turn_trigger_suffix_lexed(&tokens[start_idx..split_idx]);
        if let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) {
            let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                &tokens[split_idx + 1..],
            );
            match parse_effect_sentences_lexed(&effects_tokens) {
                Ok(effects) => {
                    let mut max_triggers_per_turn =
                        parse_triggered_times_each_turn_lexed_from_sentences(&effects_tokens);
                    if let Some(max) = max_triggers_from_trigger_clause {
                        max_triggers_per_turn =
                            Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                    }
                    return Ok(LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn,
                    });
                }
                Err(err) => return Err(err),
            }
        }

        let state_split =
            split_state_triggered_clause_lexed(tokens, start_idx, split_idx).or_else(|| {
                (first_split_idx != split_idx)
                    .then(|| split_state_triggered_clause_lexed(tokens, start_idx, first_split_idx))
                    .flatten()
            });
        if let Some(spec) = state_split {
            let effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                spec.trigger_tokens,
                spec.effects_tokens,
            );
            let effects = parse_effect_sentences_lexed(&effects_tokens)?;
            return Ok(LineAst::Triggered {
                trigger: TriggerSpec::StateBased {
                    condition: spec.predicate,
                    display: spec
                        .display_tokens
                        .iter()
                        .map(|token| token.slice.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                },
                effects,
                max_triggers_per_turn: None,
            });
        }
    }

    // Try every comma as a trigger/effects split point.  Prefer the split
    // that produces the **longest** effects (earliest split_idx) rather than
    // the shortest (latest split_idx).  This prevents silent truncation where
    // only the last sentence parses and the rest is absorbed into the trigger.
    let mut best_result: Option<(usize, LineAst)> = None;
    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let (trigger_tokens, max_triggers_from_trigger_clause) =
            split_first_time_each_turn_trigger_suffix_lexed(&tokens[start_idx..split_idx]);
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        if let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) {
            let rewritten_effects_tokens = rewrite_attached_controller_trigger_effect_tokens_lexed(
                trigger_tokens,
                &effects_tokens,
            );
            let effects = parse_effect_sentences_lexed(&rewritten_effects_tokens).or_else(|_| {
                let Some(stripped) =
                    maybe_strip_leading_damage_subject_tokens(&rewritten_effects_tokens)
                else {
                    return Err(CardTextError::ParseError(String::new()));
                };
                parse_effect_sentences_lexed(stripped)
            });
            if let Ok(effects) = effects {
                let mut max_triggers_per_turn =
                    parse_triggered_times_each_turn_lexed_from_sentences(&rewritten_effects_tokens);
                if let Some(max) = max_triggers_from_trigger_clause {
                    max_triggers_per_turn =
                        Some(max_triggers_per_turn.map_or(max, |existing| existing.min(max)));
                }
                let effect_token_count = effects_tokens.len();
                let line_ast = LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                };
                // Keep the split that produces the most effect tokens
                // (earliest split point = most effects).
                if best_result
                    .as_ref()
                    .map_or(true, |(prev_count, _)| effect_token_count > *prev_count)
                {
                    best_result = Some((effect_token_count, line_ast));
                }
            }
        }
    }
    if let Some((effect_count, line_ast)) = best_result {
        // Reject splits where the effects cover too little of the total line
        // AND there are multiple sentences (periods) in the line.  A single-
        // sentence triggered ability can legitimately have a long trigger and
        // short effect; the truncation problem only arises when multi-sentence
        // lines silently lose entire sentences.
        let total_token_count = tokens.len().saturating_sub(start_idx);
        let period_count = tokens
            .iter()
            .filter(|t| t.kind == crate::runtime_backend::lexer::TokenKind::Period)
            .count();
        if period_count >= 2 && total_token_count > 15 && effect_count * 4 < total_token_count {
            return Err(CardTextError::ParseError(format!(
                "triggered line effects cover too few tokens ({effect_count}/{total_token_count}), \
                 likely missing unsupported clauses (line: '{}')",
                TokenWordView::new(tokens).word_refs().join(" ")
            )));
        }
        return Ok(line_ast);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported triggered line (clause: '{}')",
        TokenWordView::new(tokens).word_refs().join(" ")
    )))
}

pub(crate) fn parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    super::activation_and_restrictions::parse_trigger_clause_lexed(tokens)
}

pub(crate) fn parse_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    super::keyword_static::parse_static_ability_ast_line_lexed(tokens)
}
