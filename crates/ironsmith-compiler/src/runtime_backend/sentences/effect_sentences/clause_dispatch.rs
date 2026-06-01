pub(crate) use self::become_clause::parse_become_clause;
use self::helpers::{
    has_counter_state_pronoun, parse_become_base_pt_tail, parse_become_creature_descriptor_words,
    parse_controller_or_owner_of_target_subject, render_lower_words,
    strip_base_power_toughness_subject_tokens, subject_references_base_power_toughness,
};
use self::next_turn_cant::parse_next_turn_cant_clause;
use super::super::activation_and_restrictions::{
    build_may_cast_tagged_effect, find_negation_span, parse_cant_restriction_clause,
    parse_cant_restrictions, parse_choose_card_type_phrase_words, parse_choose_color_phrase_words,
    parse_choose_creature_type_phrase_words, parse_choose_player_phrase_words,
    parse_may_cast_it_sentence, parse_single_word_keyword_action,
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause,
    parse_you_choose_player_clause, starts_with_target_indicator,
};
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::keyword_static::{
    keyword_action_to_static_ability, parse_ability_line, parse_pt_modifier,
    parse_pt_modifier_values,
};
use super::super::lexer::{
    LexedClause, OwnedLexToken, contains_token_word, token_slice_first_is,
    token_slice_first_is_any, word_slice_contains_phrase, word_slice_eq, word_slice_eq_any,
    word_slice_first_is_any, word_slice_starts_with,
};
use super::super::object_filters::parse_object_filter;
use super::super::permission_helpers::parse_cast_or_play_tagged_clause;
use super::super::token_primitives::find_index as find_token_index;
use super::super::util::{
    contains_until_end_of_turn, parse_card_type, parse_color, parse_counter_type_word,
    parse_number, parse_subject, parse_subtype_flexible, parse_target_phrase, parse_value,
    parser_trace, parser_trace_stack, span_from_tokens, starts_with_until_end_of_turn,
    token_index_for_word_index, trim_commas, word_refs_except,
};
use super::chain_carry::{parse_leading_player_may, remove_first_word, remove_through_first_word};
use super::clause_pattern_helpers::{ClauseShape, clause_shape, extract_subject_player};
use super::clause_primitives::run_clause_primitives;
use super::dispatch_inner::{
    parse_additional_phase_sentence, parse_prevent_damage_sentence, parse_take_extra_turn_sentence,
    trim_edge_punctuation,
};
use super::for_each_helpers::{
    has_demonstrative_object_reference, is_mana_replacement_clause_words,
    is_mana_trigger_additional_clause_words, is_target_player_dealt_damage_by_this_turn_subject,
    parse_for_each_object_subject, parse_get_for_each_count_value,
    parse_get_modifier_values_with_tail, parse_has_base_power_clause,
    parse_has_base_power_toughness_clause,
};
use super::search_library::parse_restriction_duration;
use super::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, find_unquoted_token_word, try_build_unless,
};
use super::verb_dispatch::parse_effect_with_verb;
use super::verb_handlers::parse_control_duration;
use super::zone_counter_helpers::{parse_half_starting_life_total_value, parse_put_counters};
use super::zone_handlers::{collapse_leading_signed_pt_modifier_tokens, parse_sacrifice};
use super::{
    Verb, bind_implicit_player_context, find_verb, parse_effect_chain_with_subject_verb_primitives,
    parse_simple_gain_ability_clause, parse_simple_lose_ability_clause, parse_subtype_word,
};
use crate::TagKey;
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction, PlayerAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod become_clause;
mod helpers;
mod next_turn_cant;

type ClauseDispatchCompatWords<'a> = TokenWordView<'a>;

const PREVENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["prevent"]);
const CONTROL_PLAYER_SUBJECT_PATTERNS: &[&[&str]] = &[
    &["you"],
    &["that", "player"],
    &["target", "player"],
    &["each", "opponent"],
];
const YOU_MAY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "may"]);
const CAST_ANY_NUMBER_OF_SPELLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["cast", "any", "number", "of", "spells"]);
const WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["without", "paying", "their", "mana", "costs"]);
const FROM_AMONG_NONLAND_EXILED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [&[
            "from", "among", "the", "nonland", "cards", "exiled", "this", "way"
        ]]
);
const WITH_MANA_VALUE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["with", "mana", "value"]);
const X_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["x"]);
const ALL_ABILITIES_AND_GAIN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["all", "abilities", "and"]; contains_any_words & [&["gain", "gains"]]);
const HEXPROOF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["hexproof"]);
const FLYING_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["flying"]);
const HASTE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["haste"]);
const RING_TEMPTS_YOU_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["the", "ring", "tempts", "you"]);
const TAKE_INITIATIVE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "take", "the", "initiative"]);
const FOR_EACH_OF_THOSE_CARDS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each", "of", "those", "cards"]);
const PAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["pay"]);
const OR_PUT_WORDS: &[&str] = &["or", "put"];
const OR_PUT_PATTERN: ClauseShape<'static> = clause_shape!(exact & OR_PUT_WORDS);
const TOP_LIBRARY_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["the", "card", "on", "top", "of", "your", "library"]);
const THEN_RETURN_WORDS: &[&str] = &["then", "return"];
const THEN_RETURN_PATTERN: ClauseShape<'static> = clause_shape!(exact & THEN_RETURN_WORDS);
const UNLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const ITS_CONTROLLER_HAS_YOU_DRAW_CARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["its", "controller", "has", "you", "draw", "a", "card"]);
const CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
const PRONOUN_TAGGED_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["they"], &["them"]]);
const DEMONSTRATIVE_SUBJECT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that"], &["those"]]);
const TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const TAGGED_OBJECT_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["they"],
            &["them"],
            &["that"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "permanents"],
            &["those", "objects"],
        ]
);
const OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const HAVE_OR_HAS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["have"], &["has"]]);
const THIS_SOURCE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this"]);
const EQUIPPED_OBJECT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["equipped", "creature"], &["equipped", "permanent"]]);
const ENCHANTED_OBJECT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted", "creature"], &["enchanted", "permanent"]]);
const FOR_EACH_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each", "opponent"]);
const CHOOSE_ODD_OR_EVEN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["choose", "odd", "or", "even"]);
const CHOOSE_OR_CHOOSES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["choose"], &["chooses"]]);
const CHOOSE_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["choose"], &["chooses"]]; contains_words & ["target"]);
const CHOOSE_TARGET_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any & [&["choose"], &["chooses"]];
    contains_words & ["target"];
    contains_any_words & [&["player", "players"]]
);
const ASSIGNS_NO_COMBAT_DAMAGE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["assigns", "no", "combat", "damage"]]);
const ASSIGN_OR_ASSIGNS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["assign"], &["assigns"]]);
const THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this", "turn"]);
const THIS_COMBAT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this", "combat"]);
const ATTACHED_SUBJECT_DURATION_PREFIXES: &[&[&str]] = &[
    &["until", "end", "of", "turn"],
    &["until", "your", "next", "turn"],
    &["until", "end", "of", "combat"],
];
const IT_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const IT_OR_THEM_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const THIS_OR_THIS_CREATURE_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["this", "creature"]]);
const CAST_TARGET_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["cast", "target"]);
const WITHOUT_PAYING_ITS_MANA_COST_WORDS: &[&str] = &["without", "paying", "its", "mana", "cost"];
const WITHOUT_PAYING_ITS_MANA_COST_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & WITHOUT_PAYING_ITS_MANA_COST_WORDS);
const COPY_CARD_EXILED_WITH_THIS_ARTIFACT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["copy", "a", "card", "exiled", "with", "this", "artifact"]);
const PLANESWALK_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["planeswalk"]);
const CHAOS_ENSUES_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["chaos", "ensues"]);
const PROTECTION_CHOICE_COLOR_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["protection", "choice"]; contains_any_words & [&["color", "colorless"]]);
const COLORLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["colorless"]);
const HEXPROOF_TARGETING_OVERRIDE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [
        &["can", "be", "the", "targets"],
        &["as", "though", "they", "didnt", "have", "hexproof"],
    ];
    contains_words & ["hexproof"]
);
const CREATURES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["creatures"]);
const CAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["can"]);
const IS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["is"]);
const GOADED_OR_GOAD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["goaded"], &["goad"]]);
const TARGET_PLAYER_CONTROLS_WORDS: &[&str] = &["target", "player", "controls"];
const TARGET_PLAYERS_CONTROL_WORDS: &[&str] = &["target", "players", "control"];
const TARGET_OPPONENT_CONTROLS_WORDS: &[&str] = &["target", "opponent", "controls"];
const TARGET_OPPONENTS_CONTROL_WORDS: &[&str] = &["target", "opponents", "control"];
const TARGET_CONTROLLER_PATTERNS: &[&[&str]] = &[
    TARGET_PLAYER_CONTROLS_WORDS,
    TARGET_PLAYERS_CONTROL_WORDS,
    TARGET_OPPONENT_CONTROLS_WORDS,
    TARGET_OPPONENTS_CONTROL_WORDS,
];
const FROM_AMONG_THEM_OR_THOSE_CARDS_PATTERNS: &[&[&str]] = &[
    &["from", "among", "them"],
    &["from", "among", "those", "cards"],
];
const TARGET_ONLY_RESTRICTION_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words
        & [&[
            "blocked", "except", "unless", "attack", "attacks", "block", "blocks",
        ]]
);
const COUNTED_COUNTER_OBJECTS_TAG: &str = "__ironsmith_counted_counter_objects";

fn dispatch_find_phrase_shape(
    words: &[&str],
    phrase_len: usize,
    shape: ClauseShape<'static>,
) -> Option<usize> {
    words
        .windows(phrase_len)
        .position(|window| shape.matches_words(window))
}

fn dispatch_find_any_phrase_start<'a>(
    words: &[&str],
    phrases: &'a [&'a [&'a str]],
) -> Option<(&'a [&'a str], usize)> {
    phrases.iter().find_map(|phrase| {
        words
            .windows(phrase.len())
            .position(|window| window == *phrase)
            .map(|idx| (*phrase, idx))
    })
}

fn dispatch_strip_prefix_value<'a>(
    words: &'a [&'a str],
    phrases: &[&[&str]],
) -> Option<&'a [&'a str]> {
    phrases.iter().find_map(|phrase| {
        if words.starts_with(phrase) {
            Some(&words[phrase.len()..])
        } else {
            None
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommonPlayerActionPattern {
    Amount,
    ObjectSelection,
    ZoneMovement,
    Choice,
    Payment,
    StateChange,
}

#[derive(Debug, Clone, Copy)]
struct PlayerAmountClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

fn rest_starts_all_abilities_shared_gain(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens);
    ALL_ABILITIES_AND_GAIN_PREFIX_PATTERN.matches(clause)
}

fn is_tagged_object_reference(words: &[&str]) -> bool {
    TAGGED_OBJECT_REFERENCE_PATTERN.matches_words(words)
}

fn parse_copular_base_pt_animation_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let words = clause.word_refs();
    let Some(copula_idx) = words.iter().position(|word| matches!(*word, "is" | "are")) else {
        return Ok(None);
    };
    if copula_idx == 0 || copula_idx + 1 >= words.len() {
        return Ok(None);
    }

    let rest_words = &words[copula_idx + 1..];
    if parse_pt_modifier(rest_words[0]).is_err()
        || !rest_words
            .iter()
            .any(|word| matches!(*word, "creature" | "creatures"))
        || !word_slice_contains_phrase(rest_words, &["in", "addition", "to"])
    {
        return Ok(None);
    }

    let Some(subject_clause) = clause.before_word(copula_idx) else {
        return Ok(None);
    };
    let Some(rest_clause) = clause.from_word(copula_idx + 1) else {
        return Ok(None);
    };
    let subject_tokens = subject_clause.trimmed_tokens();
    let rest_tokens = rest_clause.trimmed_tokens();
    if subject_tokens.is_empty() || rest_tokens.is_empty() {
        return Ok(None);
    }

    parse_become_clause(subject_tokens, rest_tokens).map(Some)
}

fn parse_count_counters_on_filter_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let words = clause.word_refs();
    if !word_slice_starts_with(&words, &["count"]) {
        return Ok(None);
    }

    let mut idx = 1usize;
    if words.get(idx).copied() == Some("the") {
        idx += 1;
    }
    if words.get(idx..idx + 2) != Some(&["number", "of"][..]) {
        return Ok(None);
    }
    idx += 2;

    if words.get(idx).is_some_and(|word| matches!(*word, "a" | "an" | "one")) {
        idx += 1;
    }

    let Some(counter_word) = words.get(idx).copied() else {
        return Ok(None);
    };
    let Some(counter_type) = parse_counter_type_word(counter_word) else {
        return Ok(None);
    };
    idx += 1;

    if !words
        .get(idx)
        .is_some_and(|word| matches!(*word, "counter" | "counters"))
        || words.get(idx + 1).copied() != Some("on")
    {
        return Ok(None);
    }
    idx += 2;

    let Some(filter_token_idx) = token_index_for_word_index(tokens, idx) else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&tokens[filter_token_idx..]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(&filter_tokens, false)?;
    filter.with_counter = Some(crate::filter::CounterConstraint::Typed(counter_type));
    Ok(Some(EffectAst::subject_verb_tag_matching_objects(
        filter,
        vec![Zone::Battlefield],
        TagKey::from(COUNTED_COUNTER_OBJECTS_TAG),
    )))
}

#[derive(Debug, Clone, Copy)]
struct PlayerObjectClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerZoneClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerChoiceClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerPaymentClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct PlayerStateClause<'a> {
    subject: SubjectAst,
    verb: Verb,
    action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
enum CommonPlayerActionClause<'a> {
    Amount(PlayerAmountClause<'a>),
    Object(PlayerObjectClause<'a>),
    Zone(PlayerZoneClause<'a>),
    Choice(PlayerChoiceClause<'a>),
    Payment(PlayerPaymentClause<'a>),
    State(PlayerStateClause<'a>),
}

impl<'a> PlayerAmountClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerObjectClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerZoneClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerChoiceClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerPaymentClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

impl<'a> PlayerStateClause<'a> {
    fn lower(self) -> Result<EffectAst, CardTextError> {
        parse_effect_with_verb(self.verb, Some(self.subject), self.action_tokens)
    }
}

fn common_player_action_pattern_for(
    verb: Verb,
    action_tokens: &[OwnedLexToken],
) -> Option<CommonPlayerActionPattern> {
    let words = TokenWordView::new(action_tokens);
    if matches!(verb, Verb::Pay) {
        return Some(CommonPlayerActionPattern::Payment);
    }
    if matches!(verb, Verb::Scry | Verb::Surveil) {
        return Some(CommonPlayerActionPattern::Choice);
    }
    if matches!(
        verb,
        Verb::Sacrifice | Verb::Discard | Verb::Reveal | Verb::Look
    ) {
        return Some(CommonPlayerActionPattern::ObjectSelection);
    }
    if matches!(
        verb,
        Verb::Shuffle | Verb::Move | Verb::Put | Verb::Return | Verb::Exile
    ) || words.word_refs().iter().any(|word| {
        matches!(
            *word,
            "library" | "graveyard" | "hand" | "battlefield" | "exile"
        )
    }) {
        return Some(CommonPlayerActionPattern::ZoneMovement);
    }
    if matches!(
        verb,
        Verb::Draw | Verb::Lose | Verb::Gain | Verb::Mill | Verb::Get | Verb::Add
    ) {
        return Some(CommonPlayerActionPattern::Amount);
    }
    if matches!(verb, Verb::Skip | Verb::Take | Verb::Become | Verb::End) {
        return Some(CommonPlayerActionPattern::StateChange);
    }
    None
}

fn parse_control_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let Some(control_word_idx) = clause.find_word_any(&["control", "controls"]) else {
        return Ok(None);
    };
    if control_word_idx == 0 {
        return Ok(None);
    }

    let subject_clause = clause
        .before_word(control_word_idx)
        .unwrap_or_else(|| clause.before(0))
        .trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let subject_words = subject_clause.word_refs();
    let Some((subject_phrase, _)) =
        dispatch_find_any_phrase_start(&subject_words, CONTROL_PLAYER_SUBJECT_PATTERNS)
    else {
        return Ok(None);
    };
    let player = match subject_phrase {
        ["you"] => PlayerAst::You,
        ["that", "player"] => PlayerAst::That,
        ["target", "player"] => PlayerAst::Target,
        ["each", "opponent"] => PlayerAst::Opponent,
        _ => unreachable!("matched subject-control player phrase"),
    };

    let Some(during_word_idx) = clause.find_word("during") else {
        return Ok(None);
    };
    if during_word_idx <= control_word_idx + 1 {
        return Ok(None);
    }

    let target_clause = clause.between_words_trimmed(control_word_idx + 1, during_word_idx);
    if target_clause.is_empty() {
        return Ok(None);
    }
    let TargetAst::Player(target_filter, _) = parse_target_phrase(target_clause.tokens())? else {
        return Ok(None);
    };

    let duration_clause = clause
        .from_word(during_word_idx)
        .unwrap_or_else(|| clause.from(clause.len()))
        .trimmed();
    let duration = parse_control_duration(duration_clause.tokens())?;
    Ok(Some(EffectAst::subject_verb_control_player(
        player,
        PlayerFilter::Target(Box::new(target_filter)),
        duration,
    )))
}

fn is_pronoun_top_or_bottom_library_choice_put_tail(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens);
    if !clause
        .first_word()
        .is_some_and(|word| IT_OR_THEM_SUBJECT_PATTERN.matches_word(word))
    {
        return false;
    }
    clause.contains_all_words(&["on", "choice", "top", "bottom", "library"])
}

impl<'a> CommonPlayerActionClause<'a> {
    fn recognize(
        subject: SubjectAst,
        verb: Verb,
        action_tokens: &'a [OwnedLexToken],
    ) -> Option<Self> {
        if !matches!(subject, SubjectAst::Player(_)) {
            return None;
        }
        let pattern = common_player_action_pattern_for(verb, action_tokens)?;
        Some(match pattern {
            CommonPlayerActionPattern::Amount => Self::Amount(PlayerAmountClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::ObjectSelection => Self::Object(PlayerObjectClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::ZoneMovement => Self::Zone(PlayerZoneClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::Choice => Self::Choice(PlayerChoiceClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::Payment => Self::Payment(PlayerPaymentClause {
                subject,
                verb,
                action_tokens,
            }),
            CommonPlayerActionPattern::StateChange => Self::State(PlayerStateClause {
                subject,
                verb,
                action_tokens,
            }),
        })
    }

    #[cfg(test)]
    fn pattern(&self) -> CommonPlayerActionPattern {
        match self {
            Self::Amount(_) => CommonPlayerActionPattern::Amount,
            Self::Object(_) => CommonPlayerActionPattern::ObjectSelection,
            Self::Zone(_) => CommonPlayerActionPattern::ZoneMovement,
            Self::Choice(_) => CommonPlayerActionPattern::Choice,
            Self::Payment(_) => CommonPlayerActionPattern::Payment,
            Self::State(_) => CommonPlayerActionPattern::StateChange,
        }
    }

    fn lower(self) -> Result<EffectAst, CardTextError> {
        match self {
            Self::Amount(clause) => clause.lower(),
            Self::Object(clause) => clause.lower(),
            Self::Zone(clause) => clause.lower(),
            Self::Choice(clause) => clause.lower(),
            Self::Payment(clause) => clause.lower(),
            Self::State(clause) => clause.lower(),
        }
    }
}

fn clause_may_contain_cast_or_play_permission(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(
                word,
                "may" | "cast" | "casts" | "casting" | "play" | "plays" | "playing" | "played"
            )
        })
}

fn parse_for_each_prevent_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(prevent_idx) =
        find_token_index(tokens, |token| PREVENT_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..prevent_idx]);
    let Some(filter) = parse_for_each_object_subject(&subject_tokens)? else {
        return Ok(None);
    };

    let unless_idx =
        crate::runtime_backend::lexer::find_token_word(&tokens[prevent_idx..], "unless")
            .map(|idx| prevent_idx + idx);
    let prevent_tokens = trim_commas(match unless_idx {
        Some(idx) => &tokens[prevent_idx..idx],
        None => &tokens[prevent_idx..],
    });
    let Some(prevent_effect) = parse_prevent_damage_sentence(&prevent_tokens)? else {
        return Ok(None);
    };

    let effects = if let Some(idx) = unless_idx {
        if let Some(unless_effect) = try_build_unless(
            vec![prevent_effect.clone()],
            SubjectVerbPrimitiveClause::new(tokens),
            idx,
        )? {
            vec![unless_effect]
        } else {
            vec![prevent_effect]
        }
    } else {
        vec![prevent_effect]
    };
    Ok(Some(EffectAst::ForEachObject { filter, effects }))
}

fn parse_cast_any_number_from_among_tagged_clause(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let words = if YOU_MAY_PREFIX_PATTERN.matches_words(&clause_words) {
        &clause_words[2..]
    } else {
        clause_words.as_slice()
    };

    if !CAST_ANY_NUMBER_OF_SPELLS_PREFIX_PATTERN.matches_words(words)
        || !WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN.matches_words(words)
    {
        return None;
    }

    let (_, from_idx) =
        dispatch_find_any_phrase_start(words, FROM_AMONG_THEM_OR_THOSE_CARDS_PATTERNS)?;

    let mut filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
        TagKey::from(IT_TAG),
        crate::target::TaggedOpbjectRelation::IsTaggedObject,
    );

    if let Some(mana_idx) = words.iter().enumerate().find_map(|(idx, _)| {
        WITH_MANA_VALUE_PREFIX_PATTERN
            .matches_words(&words[idx..])
            .then_some(idx)
    }) {
        let value_word_idx = mana_idx + 3;
        let Some(value_words) = words.get(value_word_idx..from_idx) else {
            return None;
        };
        filter.mana_value = Some(
            if value_words
                .first()
                .is_some_and(|word| X_WORD_PATTERN.matches_word(word))
            {
                if value_words != ["x", "or", "less"] {
                    return None;
                }
                crate::filter::Comparison::LessThanOrEqualExpr(Box::new(Value::X))
            } else {
                let quantity_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(value_words);
                let (value, used) =
                    crate::runtime_backend::util::parse_less_than_or_equal_quantity_prefix(
                        &quantity_tokens,
                        false,
                        false,
                        "mana value bound",
                    )
                    .ok()
                    .flatten()?;
                if used != value_words.len() {
                    return None;
                }
                crate::filter::Comparison::LessThanOrEqual(value as i32)
            },
        );
    } else if from_idx != 5 {
        return None;
    }

    Some(EffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                false,
                false,
                true,
                None,
            )],
        }],
    })
}

fn parse_cast_single_spell_from_among_hand_cards_clause(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let mut words = clause_words.as_slice();
    if word_slice_starts_with(words, &["if", "you", "do"]) {
        words = &words[3..];
        if word_slice_first_is_any(words, &["then", "and"]) {
            words = &words[1..];
        }
    }
    let words = if word_slice_starts_with(words, &["you", "may"]) {
        &words[2..]
    } else {
        words
    };

    if !word_slice_eq(
        words,
        &[
            "cast", "a", "spell", "from", "among", "those", "cards", "without", "paying", "its",
            "mana", "cost",
        ],
    ) {
        return None;
    }

    Some(
        EffectAst::may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
            PlayerAst::You,
            PlayerAst::That,
            ObjectFilter::nonland().in_zone(Zone::Hand),
            Zone::Hand,
        ),
    )
}

pub(crate) fn parse_effect_clause(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    let stripped_instead = super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);

    if clause_may_contain_cast_or_play_permission(tokens) {
        if let Some(spec) = parse_may_cast_it_sentence(tokens) {
            return Ok(build_may_cast_tagged_effect(&spec));
        }

        if let Some(effect) = parse_cast_or_play_tagged_clause(tokens)? {
            return Ok(effect);
        }

        if let Some(effect) = parse_cast_any_number_from_among_tagged_clause(tokens) {
            return Ok(effect);
        }

        if let Some(effect) = parse_cast_single_spell_from_among_hand_cards_clause(tokens) {
            return Ok(effect);
        }
    }

    if let Some(player) = parse_leading_player_may(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if token_slice_first_is_any(&stripped, &["have", "has"]) {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain_with_subject_verb_primitives(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        return Ok(EffectAst::MayByPlayer { player, effects });
    }

    if token_slice_first_is(tokens, "may") {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_with_subject_verb_primitives(&stripped)?;
        return Ok(EffectAst::May { effects });
    }

    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();

    if let Some(effect) = parse_for_each_prevent_damage_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_count_counters_on_filter_clause(tokens)? {
        return Ok(effect);
    }

    if CAST_ANY_NUMBER_OF_SPELLS_PREFIX_PATTERN.matches_words(&clause_words)
        && FROM_AMONG_NONLAND_EXILED_THIS_WAY_PATTERN.matches_words(&clause_words)
        && WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN.matches_words(&clause_words)
    {
        let cast_filter = ObjectFilter::nonland()
            .in_zone(crate::zone::Zone::Exile)
            .match_tagged(
                TagKey::from(IT_TAG),
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            );
        return Ok(EffectAst::ForEachObject {
            filter: cast_filter,
            effects: vec![EffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    TagKey::from(IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                )],
            }],
        });
    }

    if ALL_ABILITIES_AND_GAIN_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some(gain_idx) =
            crate::runtime_backend::lexer::find_token_any_word(tokens, &["gain", "gains"])
    {
        let ability_words = &clause_words[gain_idx + 1..];
        let mut abilities = Vec::new();
        if HEXPROOF_WORD_PATTERN.matches_words(ability_words) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Hexproof));
        }
        if FLYING_WORD_PATTERN.matches_words(ability_words) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Flying));
        }
        if HASTE_WORD_PATTERN.matches_words(ability_words) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
        }
        if !abilities.is_empty() {
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                TargetAst::Tagged(
                    TagKey::from(IT_TAG),
                    Some(crate::cards::builders::TextSpan::synthetic()),
                ),
                abilities,
                Until::Forever,
            ));
        }
    }
    if RING_TEMPTS_YOU_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_ring_tempts_you(
            crate::cards::builders::PlayerAst::You,
        ));
    }
    if TAKE_INITIATIVE_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_take_initiative(
            crate::cards::builders::PlayerAst::You,
        ));
    }
    if let Some(effect) = parse_take_extra_turn_sentence(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(effect);
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
            clause_words.join(" ")
        )));
    }

    if FOR_EACH_OF_THOSE_CARDS_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some(pay_idx) = PAY_WORD_PATTERN.find_word(&clause_words)
        && let Some(or_put_idx) =
            dispatch_find_phrase_shape(&clause_words, OR_PUT_WORDS.len(), OR_PUT_PATTERN)
        && or_put_idx > pay_idx
        && clause_words
            .get(or_put_idx + 2..)
            .is_some_and(|tail| TOP_LIBRARY_CARD_TAIL_PATTERN.matches_words(tail))
    {
        let pay_token_idx = token_index_for_word_index(tokens, pay_idx).unwrap_or(tokens.len());
        let Some((life_amount, _used)) = parse_number(&tokens[pay_token_idx + 1..]) else {
            return Err(CardTextError::ParseError(format!(
                "missing life payment amount in for-each card choice (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let mut filter = ObjectFilter::default();
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(EffectAst::ForEachObject {
            filter,
            effects: vec![EffectAst::UnlessAction {
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    crate::zone::Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                alternative: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LoseLife {
                        amount: Value::Fixed(life_amount as i32),
                    },
                )],
                player: PlayerAst::You,
            }],
        });
    }

    if FOR_EACH_OPPONENT_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some(then_return_idx) =
            dispatch_find_phrase_shape(&clause_words, THEN_RETURN_WORDS.len(), THEN_RETURN_PATTERN)
        && let Some(unless_idx) = UNLESS_WORD_PATTERN.find_word(&clause_words)
        && unless_idx > then_return_idx
        && clause_words
            .get(unless_idx + 1..unless_idx + 8)
            .is_some_and(|tail| ITS_CONTROLLER_HAS_YOU_DRAW_CARD_PATTERN.matches_words(tail))
    {
        let choose_start = 3usize;
        if CHOOSE_WORD_PATTERN.matches_word_at(&clause_words, choose_start) {
            let target_token_start =
                token_index_for_word_index(tokens, choose_start + 1).unwrap_or(tokens.len());
            let target_token_end =
                token_index_for_word_index(tokens, then_return_idx).unwrap_or(tokens.len());
            let target_tokens = trim_commas(&tokens[target_token_start..target_token_end]);
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::ForEachOpponent {
                effects: vec![
                    EffectAst::subject_verb_target_only(target),
                    EffectAst::UnlessAction {
                        effects: vec![EffectAst::subject_verb_return_to_hand(
                            TargetAst::Tagged(TagKey::from(IT_TAG), None),
                            false,
                        )],
                        alternative: vec![EffectAst::subject_verb(
                            SubjectVerbRoleAst::AffectedPlayer,
                            PlayerAst::You,
                            SubjectVerbActionAst::Draw {
                                count: Value::Fixed(1),
                            },
                        )],
                        player: PlayerAst::ItsController,
                    },
                ],
            });
        }
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    let clause = SubjectVerbPrimitiveClause::new(tokens);
    if let Some(unless_idx) = find_unquoted_token_word(clause, "unless") {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        if !main_tokens.is_empty()
            && let Ok(main_effect) = parse_effect_clause(&main_tokens)
            && let Some(unless_effect) = try_build_unless(vec![main_effect], clause, unless_idx)?
        {
            return Ok(unless_effect);
        }
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_copular_base_pt_animation_clause(tokens)? {
        return Ok(effect);
    }

    let choice_words = if YOU_WORD_PATTERN.matches_word_at(&clause_words, 0) {
        &clause_words[1..]
    } else {
        &clause_words[..]
    };

    if let Some((consumed, excluded_color)) = parse_choose_color_phrase_words(choice_words)?
        && consumed == choice_words.len()
        && excluded_color.is_none()
    {
        return Ok(EffectAst::subject_verb_choose_color(
            crate::cards::builders::PlayerAst::Implicit,
        ));
    }

    if CHOOSE_ODD_OR_EVEN_PATTERN.matches_words(choice_words) {
        return Ok(EffectAst::subject_verb_choose_named_option(
            crate::cards::builders::PlayerAst::Implicit,
            vec!["odd".to_string(), "even".to_string()],
        ));
    }

    if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_creature_type(
            crate::cards::builders::PlayerAst::Implicit,
            excluded_subtypes,
        ));
    }

    if let Some((consumed, options)) = parse_choose_card_type_phrase_words(choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_card_type(
            crate::cards::builders::PlayerAst::Implicit,
            options,
        ));
    }

    if let Some(consumed) = parse_choose_player_phrase_words(choice_words)
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_player(
            crate::cards::builders::PlayerAst::Implicit,
            PlayerFilter::Any,
            TagKey::from(IT_TAG),
            false,
            0,
        ));
    }

    if CHOOSE_TARGET_PLAYER_PATTERN.matches_words(&clause_words)
        && let Ok(target) = parse_target_phrase(&tokens[1..])
    {
        let is_player_target = match &target {
            TargetAst::Player(_, _) => true,
            TargetAst::WithCount(inner, _) => matches!(inner.as_ref(), TargetAst::Player(_, _)),
            _ => false,
        };
        if is_player_target {
            return Ok(EffectAst::subject_verb_target_only(target));
        }
    }

    if CHOOSE_TARGET_PATTERN.matches_words(&clause_words) {
        let target_tokens = trim_commas(&tokens[1..]);
        if !target_tokens.is_empty()
            && starts_with_target_indicator(&target_tokens)
            && find_verb(&target_tokens).is_none()
        {
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::subject_verb_target_only(target));
        }
    }

    if clause_words == ["choose", "left", "or", "right"] {
        return Ok(EffectAst::subject_verb_choose_named_option(
            PlayerAst::You,
            vec!["left".to_string(), "right".to_string()],
        ));
    }

    if let Some((chooser, choose_filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(tokens)?
    {
        return Ok(EffectAst::subject_verb_choose_player(
            chooser,
            choose_filter,
            TagKey::from(IT_TAG),
            random,
            exclude_previous_choices,
        ));
    }

    if let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some((chooser, choose_filter, choose_count)) = parse_you_choose_objects_clause(tokens)? {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if ASSIGNS_NO_COMBAT_DAMAGE_PATTERN.matches_words(&clause_words) {
        let assigns_idx = find_token_index(tokens, |token| {
            ASSIGN_OR_ASSIGNS_WORD_PATTERN.matches_token(token)
        })
        .unwrap_or(0);
        let subject_tokens = trim_commas(&tokens[..assigns_idx]);
        let tail_tokens = trim_commas(&tokens[assigns_idx + 1..]);
        let tail_word_view = ClauseDispatchCompatWords::new(&tail_tokens);
        let tail_words = tail_word_view.to_word_refs();
        if grammar::words_match_prefix(&tail_tokens, &["no", "combat", "damage"]).is_none() {
            return Err(CardTextError::ParseError(format!(
                "unsupported assigns-no-combat-damage clause (clause: '{}') [rule=assigns-no-combat-damage]",
                clause_words.join(" ")
            )));
        }
        let mut idx = 3usize;
        if THIS_TURN_PREFIX_PATTERN.matches_words(&tail_words[idx..]) {
            idx += 2;
        } else if THIS_COMBAT_PREFIX_PATTERN.matches_words(&tail_words[idx..]) {
            idx += 2;
        }
        if idx != tail_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
                clause_words.join(" ")
            )));
        }

        let subject_word_view = ClauseDispatchCompatWords::new(&subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let source = if IT_SUBJECT_PATTERN.matches_words(&subject_words) {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&subject_tokens))
        } else if subject_words.is_empty()
            || THIS_OR_THIS_CREATURE_SUBJECT_PATTERN.matches_words(&subject_words)
        {
            TargetAst::Source(None)
        } else {
            parse_target_phrase(&subject_tokens)?
        };

        return Ok(
            EffectAst::subject_verb_prevent_all_combat_damage_from_source(source, Until::EndOfTurn),
        );
    }

    if token_slice_first_is(tokens, "target")
        && find_negation_span(tokens).is_some()
        && let (duration, clause_tokens) =
            parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()))
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && let Some(target) = parsed.target.clone()
    {
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_cant(parsed.restriction.clone(), duration, None),
            ],
        });
    }

    if token_slice_first_is(tokens, "target") && find_verb(tokens).is_none() {
        let looks_like_restriction_clause = find_negation_span(tokens).is_some()
            || TARGET_ONLY_RESTRICTION_WORD_PATTERN.matches_words(&clause_words);
        if looks_like_restriction_clause {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(tokens)?;
        return Ok(EffectAst::subject_verb_target_only(target));
    }

    if let Some(choose_word_idx) = clause_words
        .iter()
        .position(|word| CHOOSE_OR_CHOOSES_WORD_PATTERN.matches_word(word))
        && choose_word_idx > 0
        && TARGET_WORD_PATTERN.matches_word_at(&clause_words, choose_word_idx + 1)
        && let Some(target_token_idx) = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| token.as_word().is_some())
            .nth(choose_word_idx + 1)
            .map(|(idx, _)| idx)
    {
        let target = parse_target_phrase(&tokens[target_token_idx..])?;
        return Ok(EffectAst::subject_verb_target_only(target));
    }

    if let Some(effect) = parse_next_turn_cant_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)?
        && find_negation_span(&clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(EffectAst::subject_verb_cant(
            parsed.restriction.clone(),
            duration,
            None,
        ));
    }

    if let Some(effect) = parse_hexproof_targeting_override_clause(tokens)? {
        return Ok(effect);
    }

    if matches!(
        clause_words.as_slice(),
        [
            "all",
            "suspected",
            "creatures",
            "are",
            "no",
            "longer",
            "suspected"
        ]
    ) {
        return Ok(EffectAst::subject_verb_clear_suspected(None));
    }

    if CAST_TARGET_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some(without_word_idx) = dispatch_find_phrase_shape(
            &clause_words,
            WITHOUT_PAYING_ITS_MANA_COST_WORDS.len(),
            WITHOUT_PAYING_ITS_MANA_COST_PATTERN,
        )
        && let Some(target_token_end) = token_index_for_word_index(tokens, without_word_idx)
    {
        let _ = parse_target_phrase(&tokens[1..target_token_end])?;
        return Ok(EffectAst::SubjectVerb(
            crate::runtime_backend::ast::SubjectVerbEffectAst {
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::Implicit,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: TagKey::from(IT_TAG),
                    player: PlayerAst::Implicit,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                },
            },
        ));
    }

    if let Some(effect) = parse_passive_goad_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_control_player_clause(tokens)? {
        return Ok(effect);
    }

    if COPY_CARD_EXILED_WITH_THIS_ARTIFACT_PATTERN.matches_words(&clause_words) {
        let filter = ObjectFilter::default().in_zone(Zone::Exile).match_tagged(
            TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        );
        return Ok(EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            zones: vec![Zone::Exile],
            search_mode: None,
        });
    }

    if matches!(
        clause_words.as_slice(),
        [
            "this", "creature", "enters", "with", "a", "+1/+1", "counter", "on", "it"
        ] | [
            "this",
            "permanent",
            "enters",
            "with",
            "a",
            "+1/+1",
            "counter",
            "on",
            "it"
        ] | ["it", "enters", "with", "a", "+1/+1", "counter", "on", "it"]
    ) {
        return Ok(EffectAst::subject_verb_put_counters(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1),
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
            None,
            false,
        ));
    }

    if PLANESWALK_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_emit_keyword_action(
            crate::events::KeywordActionKind::Planeswalk,
            1,
        ));
    }

    if CHAOS_ENSUES_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_emit_keyword_action(
            crate::events::KeywordActionKind::ChaosEnsues,
            1,
        ));
    }

    let (verb, verb_idx) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
            "suspect",
            "end",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if verb_idx > 0 { "explicit" } else { "implicit" }
    ));

    if matches!(verb, Verb::Counter)
        && verb_idx > 0
        && contains_token_word(tokens, "on")
        && let Ok(effect) = parse_put_counters(tokens)
    {
        parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
        return Ok(effect);
    }

    if matches!(verb, Verb::Get) {
        let subject_tokens = &tokens[..verb_idx];
        if !subject_tokens.is_empty() {
            let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            let collapsed_modifier_tail =
                collapse_leading_signed_pt_modifier_tokens(&tokens[verb_idx + 1..]);
            let modifier_tail = collapsed_modifier_tail
                .as_deref()
                .unwrap_or(&tokens[verb_idx + 1..]);
            if let Some(mod_token) = modifier_tail.first().and_then(OwnedLexToken::as_word)
                && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
            {
                let count = parse_get_for_each_count_value(modifier_tail)?.or_else(|| {
                    let tail_after_modifier = modifier_tail.get(1..).unwrap_or_default();
                    if grammar::words_match_prefix(
                        tail_after_modifier,
                        &["until", "end", "of", "turn", "for", "each"],
                    )
                    .is_some()
                    {
                        parse_get_for_each_count_value(&tail_after_modifier[4..])
                            .ok()
                            .flatten()
                    } else {
                        None
                    }
                });
                if let Some(count) = count {
                    let modifier_word_view = ClauseDispatchCompatWords::new(modifier_tail);
                    let modifier_words = modifier_word_view.to_word_refs();
                    let duration = if starts_with_until_end_of_turn(&modifier_words)
                        || contains_until_end_of_turn(&modifier_words)
                    {
                        Until::EndOfTurn
                    } else {
                        Until::EndOfTurn
                    };
                    let target = parse_target_phrase(subject_tokens)?;
                    let power_per = match power {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each power modifier (clause: '{}')",
                                render_lower_words(tokens)
                            )));
                        }
                    };
                    let toughness_per = match toughness {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each toughness modifier (clause: '{}')",
                                render_lower_words(tokens)
                            )));
                        }
                    };
                    return Ok(EffectAst::subject_verb_pump_for_each(
                        power_per,
                        toughness_per,
                        target,
                        count,
                        duration,
                    ));
                }

                let (power, toughness, duration, condition) =
                    parse_get_modifier_values_with_tail(modifier_tail, power, toughness)?;

                let mut normalized_subject_words = word_refs_except(&subject_words, &["each"]);
                if OF_WORD_PATTERN.matches_word_at(&normalized_subject_words, 0) {
                    normalized_subject_words.remove(0);
                }
                if PRONOUN_TAGGED_SUBJECT_PATTERN.matches_words(&normalized_subject_words) {
                    return Ok(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens)),
                        duration,
                        condition,
                    ));
                }

                let is_demonstrative_subject = normalized_subject_words
                    .first()
                    .is_some_and(|word| DEMONSTRATIVE_SUBJECT_WORD_PATTERN.matches_word(word));
                if is_demonstrative_subject {
                    let target = parse_target_phrase(subject_tokens)?;
                    return Ok(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        target,
                        duration,
                        condition,
                    ));
                }

                let target_controller_phrase =
                    dispatch_find_any_phrase_start(&subject_words, TARGET_CONTROLLER_PATTERNS);
                if let Some((_, target_word_idx)) = target_controller_phrase
                    && let Some(target_token_idx) = find_token_index(subject_tokens, |token| {
                        TARGET_WORD_PATTERN.matches_token(token)
                    })
                    && let Ok(mut filter) = parse_object_filter(
                        &trim_commas(&subject_tokens[..target_token_idx]),
                        false,
                    )
                    && filter != ObjectFilter::default()
                {
                    filter.controller = if matches!(
                        subject_words.get(target_word_idx + 1).copied(),
                        Some("opponent" | "opponents")
                    ) {
                        Some(PlayerFilter::target_opponent())
                    } else {
                        Some(PlayerFilter::target_player())
                    };
                    return Ok(EffectAst::subject_verb_pump_all(
                        filter,
                        power.clone(),
                        toughness.clone(),
                        duration,
                    ));
                }

                if subject_words
                    .iter()
                    .any(|word| TARGET_WORD_PATTERN.matches_word(word))
                {
                    let target_tokens = if subject_tokens
                        .first()
                        .is_some_and(|token| HAVE_OR_HAS_WORD_PATTERN.matches_token(token))
                    {
                        &subject_tokens[1..]
                    } else {
                        subject_tokens
                    };
                    let target = parse_target_phrase(target_tokens)?;
                    return Ok(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        target,
                        duration,
                        condition,
                    ));
                }

                let attached_subject_words =
                    dispatch_strip_prefix_value(&subject_words, ATTACHED_SUBJECT_DURATION_PREFIXES)
                        .unwrap_or(&subject_words);
                if EQUIPPED_OBJECT_SUBJECT_PATTERN.matches_words(attached_subject_words) {
                    return Ok(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        TargetAst::Tagged(
                            TagKey::from("equipped"),
                            span_from_tokens(subject_tokens),
                        ),
                        duration,
                        condition,
                    ));
                }
                if ENCHANTED_OBJECT_SUBJECT_PATTERN.matches_words(attached_subject_words) {
                    return Ok(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        TargetAst::Tagged(
                            TagKey::from("enchanted"),
                            span_from_tokens(subject_tokens),
                        ),
                        duration,
                        condition,
                    ));
                }

                let has_counter_state_pronoun = has_counter_state_pronoun(&subject_words);
                let has_disallowed_pronoun_reference = subject_words
                    .iter()
                    .any(|word| PRONOUN_TAGGED_SUBJECT_PATTERN.matches_words(&[*word]))
                    && !has_counter_state_pronoun;
                if !subject_words
                    .iter()
                    .any(|word| THIS_SOURCE_PATTERN.matches_word(word))
                    && !has_disallowed_pronoun_reference
                    && !has_demonstrative_object_reference(&subject_words)
                    && let Ok(filter) = parse_object_filter(subject_tokens, false)
                    && filter != ObjectFilter::default()
                {
                    return Ok(EffectAst::subject_verb_pump_all(
                        filter,
                        power.clone(),
                        toughness.clone(),
                        duration,
                    ));
                }
            }
        }
    }

    let subject_tokens = &tokens[..verb_idx];
    if matches!(verb, Verb::Sacrifice)
        && let Some((subject, target)) = parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        let rest = &tokens[verb_idx + 1..];
        return parse_sacrifice(rest, Some(subject), Some(target));
    }
    if matches!(verb, Verb::Put)
        && let Some((SubjectAst::Player(PlayerAst::ItsOwner), target)) =
            parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        let rest = &tokens[verb_idx + 1..];
        if is_pronoun_top_or_bottom_library_choice_put_tail(rest) {
            return Ok(EffectAst::subject_verb_move_to_library_top_or_bottom_choice(target));
        }
    }
    let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        let rest_word_view = ClauseDispatchCompatWords::new(&tokens[verb_idx + 1..]);
        let rest_words = rest_word_view.to_word_refs();
        if PROTECTION_CHOICE_COLOR_PATTERN.matches_words(&rest_words) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::subject_verb_grant_protection_choice(
                target,
                rest_words
                    .iter()
                    .any(|word| COLORLESS_WORD_PATTERN.matches_word(word)),
            ));
        }
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let rest_word_view = ClauseDispatchCompatWords::new(&tokens[verb_idx + 1..]);
        let rest_words = rest_word_view.to_word_refs();
        let duration_phrase = super::gain_ability::parse_simple_ability_duration(&rest_words);
        let duration = duration_phrase
            .as_ref()
            .map(|(_, _, duration)| duration.clone())
            .unwrap_or(Until::Forever);
        let ability_end_word_idx = duration_phrase
            .as_ref()
            .map(|(start, _, _)| verb_idx + 1 + *start)
            .unwrap_or(clause_words.len());
        let ability_end_token_idx =
            token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
        let ability_tokens = trim_commas(&tokens[verb_idx + 1..ability_end_token_idx]);
        let trailing_tokens = trim_commas(&tokens[ability_end_token_idx..]);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_words
                .first()
                .is_some_and(|word| TARGET_WORD_PATTERN.matches_word(word))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                target, abilities, duration,
            ));
        }
    }
    if matches!(verb, Verb::Lose) && rest_starts_all_abilities_shared_gain(&tokens[verb_idx + 1..])
    {
        let target = if THIS_SOURCE_PATTERN.matches_words(&subject_words) {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else if PRONOUN_TAGGED_SUBJECT_PATTERN.matches_words(&subject_words) {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        return Ok(EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            Until::EndOfTurn,
        ));
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) {
        let rest_word_view = ClauseDispatchCompatWords::new(&tokens[verb_idx + 1..]);
        let rest_words = rest_word_view.to_word_refs();
        let duration_phrase = super::gain_ability::parse_simple_ability_duration(&rest_words);
        let duration = duration_phrase
            .as_ref()
            .map(|(_, _, duration)| duration.clone())
            .unwrap_or(Until::Forever);
        let ability_end_word_idx = duration_phrase
            .as_ref()
            .map(|(start, _, _)| verb_idx + 1 + *start)
            .unwrap_or(clause_words.len());
        let ability_end_token_idx =
            token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
        let ability_token_storage = trim_commas(&tokens[verb_idx + 1..ability_end_token_idx]);
        let ability_tokens = trim_edge_punctuation(&ability_token_storage);
        let trailing_tokens = trim_edge_punctuation(&trim_commas(&tokens[ability_end_token_idx..]));
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && TARGET_WORD_PATTERN.matches_word_at(&subject_words, 0)
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            ));
        }
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let rest = &tokens[verb_idx + 1..];
    let mut effect = if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = parse_subject(subject_tokens);
        if let Some(clause) = CommonPlayerActionClause::recognize(subject, verb, rest) {
            clause.lower()?
        } else {
            parse_effect_with_verb(verb, Some(subject), rest)?
        }
    };
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

fn parse_passive_goad_clause(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let words = ClauseDispatchCompatWords::new(tokens).to_word_refs();
    let Some(is_word_idx) = IS_WORD_PATTERN.find_word(&words) else {
        return Ok(None);
    };
    if !GOADED_OR_GOAD_WORD_PATTERN.matches_word_at(&words, is_word_idx + 1) {
        return Ok(None);
    }

    let duration_tail = &words[is_word_idx + 2..];
    let duration_ok = duration_tail.is_empty()
        || matches!(
            duration_tail,
            ["for", "the", "rest", "of", "the", "game"]
                | ["for", "the", "rest", "of", "this", "game"]
        );
    if !duration_ok {
        return Ok(None);
    }

    let Some(is_token_idx) = token_index_for_word_index(tokens, is_word_idx) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..is_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = ClauseDispatchCompatWords::new(&subject_tokens).to_word_refs();
    let target = if matches!(
        subject_words.as_slice(),
        ["the", "token"] | ["the", "tokens"]
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&subject_tokens))
    } else {
        parse_target_phrase(&subject_tokens)?
    };
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(EffectAst::subject_verb_goad(target)))
}

fn parse_hexproof_targeting_override_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let (duration, clause_tokens) =
        parse_restriction_duration(tokens)?.unwrap_or((Until::Forever, tokens.to_vec()));
    let clause_words = ClauseDispatchCompatWords::new(&clause_tokens).to_word_refs();
    if !HEXPROOF_TARGETING_OVERRIDE_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let Some(creatures_idx) = CREATURES_WORD_PATTERN.find_word(&clause_words) else {
        return Ok(None);
    };
    let Some(can_idx) = CAN_WORD_PATTERN
        .find_word(&clause_words[creatures_idx..])
        .map(|idx| creatures_idx + idx)
    else {
        return Ok(None);
    };
    let Some(creatures_token_idx) = token_index_for_word_index(&clause_tokens, creatures_idx)
    else {
        return Ok(None);
    };
    let Some(can_token_idx) = token_index_for_word_index(&clause_tokens, can_idx) else {
        return Ok(None);
    };

    let filter_tokens = trim_commas(&clause_tokens[creatures_token_idx..can_token_idx]);
    let filter = parse_object_filter(&filter_tokens, false)?;
    Ok(Some(EffectAst::subject_verb_remove_abilities_all(
        filter,
        vec![GrantedAbilityAst::KeywordAction(KeywordAction::Hexproof)],
        duration,
    )))
}

pub(crate) fn parse_effect_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn lex_tail(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex test tail")
    }

    #[test]
    fn common_player_action_clause_classifies_core_shapes() {
        let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
        for (verb, tail, expected) in [
            (
                Verb::Draw,
                "X cards where X is their devotion to black",
                CommonPlayerActionPattern::Amount,
            ),
            (
                Verb::Sacrifice,
                "a creature they control",
                CommonPlayerActionPattern::ObjectSelection,
            ),
            (
                Verb::Shuffle,
                "their graveyard into their library",
                CommonPlayerActionPattern::ZoneMovement,
            ),
            (Verb::Pay, "{2}", CommonPlayerActionPattern::Payment),
            (Verb::Scry, "X", CommonPlayerActionPattern::Choice),
        ] {
            let tail = lex_tail(tail);
            let clause = CommonPlayerActionClause::recognize(subject.clone(), verb, &tail)
                .expect("common player clause should be recognized");
            assert_eq!(clause.pattern(), expected, "{verb:?} {tail:?}");
        }
    }

    #[test]
    fn common_player_action_clause_recognizes_typed_clause_variants() {
        let subject = SubjectAst::Player(PlayerAst::TargetOpponent);
        for (verb, tail, assert_variant) in [
            (
                Verb::Draw,
                "X cards where X is their devotion to black",
                matches_amount as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Sacrifice,
                "a creature they control",
                matches_object as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Shuffle,
                "their graveyard into their library",
                matches_zone as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Scry,
                "X",
                matches_choice as fn(CommonPlayerActionClause<'_>),
            ),
            (
                Verb::Pay,
                "{2}",
                matches_payment as fn(CommonPlayerActionClause<'_>),
            ),
        ] {
            let tail = lex_tail(tail);
            let clause = CommonPlayerActionClause::recognize(subject.clone(), verb, &tail)
                .expect("common player clause should be recognized");
            assert_variant(clause);
        }
    }

    fn matches_amount(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Amount(_)));
    }

    fn matches_object(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Object(_)));
    }

    fn matches_zone(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Zone(_)));
    }

    fn matches_choice(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Choice(_)));
    }

    fn matches_payment(clause: CommonPlayerActionClause<'_>) {
        assert!(matches!(clause, CommonPlayerActionClause::Payment(_)));
    }

    #[test]
    fn common_player_action_clause_delegates_to_effect_parser() {
        for text in [
            "Target opponent draws a card",
            "Target opponent sacrifices a creature they control",
            "Target opponent shuffles their library",
            "Target opponent pays {2}",
            "Each opponent scries 1",
        ] {
            let tokens = lex_line(text, 0).expect("lex clause");
            parse_effect_clause(&tokens)
                .unwrap_or_else(|err| panic!("common player clause should parse: {text}: {err:?}"));
        }
    }

    #[test]
    fn parses_control_target_player_during_next_turn_clause() {
        let tokens = lex_line(
            "You control target player during that player's next turn.",
            0,
        )
        .expect("lex clause");
        let effect = parse_effect_clause(&tokens)
            .expect("control target player during next turn should parse");
        let debug = format!("{effect:?}").to_ascii_lowercase();
        assert!(
            debug.contains("controlplayer") && debug.contains("nextturn"),
            "expected control-player-next-turn effect, got {debug}"
        );
    }
}
