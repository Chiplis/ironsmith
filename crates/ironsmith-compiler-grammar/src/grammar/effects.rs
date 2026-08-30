use super::super::activation_and_restrictions::{
    normalize_cant_words, parse_cant_restriction_clause, parse_cant_restrictions,
};
use super::super::grammar::structure::{
    IfClausePredicateSpec, split_if_clause_lexed, split_leading_result_prefix_lexed,
};
use super::super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, lex_line, parser_token_word_refs,
    render_token_slice, split_lexed_sentences, token_slice_all_are_kind, token_slice_at_is,
    token_slice_first_is, token_word_refs, trim_lexed_commas,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::search_library_support::{
    apply_search_library_mana_constraint, extract_search_library_mana_constraint,
    normalize_search_library_filter, parse_restriction_duration_lexed,
    parse_search_library_disjunction_filter, split_search_different_name_reference_filter,
    split_search_library_count_value_clause_lexed, split_search_same_name_reference_filter,
    word_slice_mentions_nth_from_top,
};
use super::super::util::{
    helper_tag_for_tokens, is_article, parse_card_type, parse_choice_count_token_prefix_consumed,
    parse_color, parse_number, parse_subject, parse_subtype_word, parse_target_phrase,
    span_from_tokens, trim_commas,
};
use super::primitives;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IfResultPredicate, PlayerAst, PredicateAst,
    ReturnControllerAst, SearchLibrarySlotAst, SubjectAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::static_abilities::StaticAbilityId;
use crate::target::{
    ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_core::Value;
use winnow::combinator::{alt, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
#[path = "effects/effect_composition.rs"]
mod bundle_rules;
pub use bundle_rules::*;
#[path = "effects/become_shapes.rs"]
pub mod become_shapes;
#[path = "effects/chain_carry.rs"]
pub mod chain_carry;
#[path = "effects/chain_splitting.rs"]
pub mod chain_splitting;
#[path = "effects/combat_damage_family_shapes.rs"]
pub mod combat_damage_family_shapes;
#[path = "effects/combat_shapes.rs"]
pub mod combat_shapes;
#[path = "effects/control_copy_attach_shapes.rs"]
pub mod control_copy_attach_shapes;
#[path = "effects/control_flow.rs"]
pub mod control_flow;
#[path = "effects/coordination.rs"]
pub mod coordination;
#[path = "effects/damage.rs"]
mod damage;
pub use damage::*;
#[path = "effects/delayed.rs"]
mod delayed;
pub use delayed::*;
#[path = "effects/delayed_sentence_shapes.rs"]
pub mod delayed_sentence_shapes;
#[path = "effects/delayed_step_shapes.rs"]
pub mod delayed_step_shapes;
#[path = "effects/delegated_partition_shapes.rs"]
pub mod delegated_partition_shapes;
#[path = "effects/dispatch_entry_shapes.rs"]
pub mod dispatch_entry_shapes;
#[path = "effects/divvy_shapes.rs"]
pub mod divvy_shapes;
#[path = "effects/fixed_mana_output.rs"]
mod fixed_mana_output;
pub use fixed_mana_output::*;
#[path = "effects/emblem_shapes.rs"]
pub mod emblem_shapes;
#[path = "effects/exile_shapes.rs"]
mod exile_shapes;
#[path = "effects/fanout_shapes.rs"]
pub mod fanout_shapes;
#[path = "effects/optional_companion_shapes.rs"]
pub mod optional_companion_shapes;
pub use exile_shapes::*;
#[path = "effects/exile_permission_followups.rs"]
mod exile_permission_followups;
#[path = "effects/followup_shapes.rs"]
pub mod followup_shapes;
pub use exile_permission_followups::*;
#[path = "effects/for_each_shapes.rs"]
pub mod for_each_shapes;
#[path = "effects/composition_shapes.rs"]
mod generic_program_shapes;
pub use generic_program_shapes::*;
#[path = "effects/gain_life_shapes.rs"]
mod gain_life_shapes;
#[path = "effects/generic_sequence_shapes.rs"]
pub mod generic_sequence_shapes;
pub use gain_life_shapes::*;
#[path = "effects/gain_ability_shapes.rs"]
pub mod gain_ability_shapes;
#[path = "effects/instead.rs"]
mod instead;
pub use instead::*;
#[path = "effects/control.rs"]
mod control;
#[path = "effects/labeled_dispatch.rs"]
pub mod labeled_dispatch;
#[path = "effects/looked_card_shapes.rs"]
pub mod looked_card_shapes;
pub use control::*;
pub use looked_card_shapes::{
    LookedCardDestinationShape, RevealedCardChooserShape, ThreeWayLookedCardDispositionShape,
    parse_counted_looked_hand_remainder_shape, parse_exact_looked_card_move_shape,
    parse_revealed_card_choice_shape, parse_three_way_looked_card_disposition_shape,
};
#[path = "effects/conditional_shapes.rs"]
mod conditional_shapes;
pub use conditional_shapes::*;
#[path = "effects/kicked_counter_replacement.rs"]
mod kicked_counter_replacement;
pub use kicked_counter_replacement::*;
#[path = "effects/creation_shapes.rs"]
mod creation_shapes;
pub use creation_shapes::*;
#[path = "effects/return_exchange.rs"]
mod return_exchange;
pub use return_exchange::*;
#[path = "effects/replacement_prevention_shapes.rs"]
mod replacement_prevention_shapes;
pub use replacement_prevention_shapes::*;
#[path = "effects/token_copy_control_shapes.rs"]
mod token_copy_control_shapes;
pub use token_copy_control_shapes::*;
#[path = "effects/rewrite_shapes.rs"]
mod rewrite_shapes;
pub use rewrite_shapes::*;
#[path = "effects/mana_replacement.rs"]
mod mana_replacement;
pub use mana_replacement::*;
#[path = "effects/misc_action_shapes.rs"]
pub mod misc_action_shapes;
#[path = "effects/next_spell_grants.rs"]
mod next_spell_grants;
#[path = "effects/remove_destroy_shapes.rs"]
pub mod remove_destroy_shapes;
#[path = "effects/resource_shapes.rs"]
pub mod resource_shapes;
#[path = "effects/sacrifice_discard_shapes.rs"]
pub mod sacrifice_discard_shapes;
#[path = "effects/four_clause_shapes.rs"]
pub mod sequence_quad_shapes;
pub use next_spell_grants::*;
#[path = "effects/search_library.rs"]
mod search_library;
pub use search_library::*;
#[path = "effects/source_linked_exile_sequences.rs"]
mod source_linked_exile_sequences;
pub use source_linked_exile_sequences::*;
#[path = "effects/choice_damage_shapes.rs"]
pub mod choice_damage_shapes;
#[path = "effects/clause_dispatch_shapes.rs"]
pub mod clause_dispatch_shapes;
#[path = "effects/clause_pattern_shapes.rs"]
pub mod clause_pattern_shapes;
#[path = "effects/clause_primitive_shapes.rs"]
pub mod clause_primitive_shapes;
#[path = "effects/counter_marker_shapes.rs"]
pub mod counter_marker_shapes;
#[path = "effects/counter_stat_shapes.rs"]
pub mod counter_stat_shapes;
#[path = "effects/sentence_predicate_shapes.rs"]
pub mod sentence_predicate_shapes;
#[path = "effects/linked_clauses.rs"]
mod sequence_pairs;
#[path = "effects/special_sentence_shapes.rs"]
pub mod special_sentence_shapes;
#[path = "effects/subject_verb_registry_shapes.rs"]
pub mod subject_verb_registry_shapes;
#[path = "effects/three_clause_shapes.rs"]
pub mod triple_sequence_shapes;
#[path = "effects/typed_clause_heads.rs"]
pub mod typed_clause_heads;
pub use sequence_pairs::*;
#[path = "effects/sentence_prelude.rs"]
mod sentence_prelude;
pub use sentence_prelude::*;
#[path = "effects/tap_shapes.rs"]
mod tap_shapes;
pub use tap_shapes::*;
#[path = "effects/unsupported_shapes.rs"]
mod unsupported_shapes;
pub use unsupported_shapes::*;
#[path = "effects/unless_clause.rs"]
mod unless_clause;
pub use unless_clause::*;
#[path = "effects/zone_counter_shapes.rs"]
pub mod zone_counter_shapes;
#[path = "effects/zone_move_shapes.rs"]
pub mod zone_move_shapes;
const IF_YOU_PHRASE: &[&str] = &["if", "you"];
const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
const THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const MAX_SPEED_LABEL: &[&str] = &["max", "speed"];
const SPLIT_NEGATED_ACTION_PHRASES: &[&[&str]] = &[
    &["do", "not"],
    &["did", "not"],
    &["can", "not"],
    &["can", "t"],
    &["doesn", "t"],
    &["didn", "t"],
    &["don", "t"],
];
const THAT_WOULD_BE_DEALT_PHRASE: &[&str] = &["that", "would", "be", "dealt"];
const PREVENT_ALL_COMBAT_DAMAGE_PREFIX: &[&str] = &["prevent", "all", "combat", "damage"];
const LOSE_MANA_STEPS_PHASES_END_WORDS: &[&str] = &["lose", "mana", "steps", "phases", "end"];
const THAT_MANY_PREFIX: &[&str] = &["that", "many"];
const TRAILING_THAT_PLAYER_SHUFFLE_PHRASES: &[&[&str]] = &[
    &["then", "that", "player", "shuffle"],
    &["then", "that", "player", "shuffles"],
    &["that", "player", "shuffle"],
    &["that", "player", "shuffles"],
];
const LABELED_ABILITY_EXACT_PHRASES: &[&[&str]] = &[
    &["spell", "mastery"],
    &["totem", "armor"],
    &["fateful", "hour"],
    &["join", "forces"],
    &["pack", "tactics"],
    &["max", "speed"],
    &["leading", "from", "the", "front"],
    &["summary", "execution"],
    &["will", "of", "the", "council"],
    &["guardian", "protocols"],
    &["jolly", "gutpipes"],
    &["protection", "fighting", "style"],
    &["relentless", "march"],
    &["secret", "of", "the", "soul"],
    &["secrets", "of", "the", "soul"],
    &["flurry", "of", "blows"],
    &["gust", "of", "wind"],
    &["reverberating", "summons"],
];
const LABELED_ABILITY_FIRST_WORDS: &[&str] = &[
    "adamant",
    "addendum",
    "alliance",
    "ascend",
    "battalion",
    "enrage",
    "boast",
    "buyback",
    "cycling",
    "bloodrush",
    "channel",
    "chroma",
    "cohort",
    "constellation",
    "converge",
    "corrupted",
    "coven",
    "eerie",
    "equip",
    "escape",
    "exhaust",
    "flashback",
    "harmonize",
    "delirium",
    "domain",
    "ferocious",
    "flurry",
    "formidable",
    "hellbent",
    "heroic",
    "imprint",
    "inspired",
    "landfall",
    "lieutenant",
    "magecraft",
    "metalcraft",
    "morbid",
    "parley",
    "partner",
    "protector",
    "radiance",
    "raid",
    "renew",
    "replicate",
    "revolt",
    "suspend",
    "spectacle",
    "strive",
    "surge",
    "threshold",
    "undergrowth",
    "ward",
];

fn token_is_any_word(token: &OwnedLexToken, words: &[&str]) -> bool {
    token.as_word().is_some_and(|_| {
        let text = token.parser_text();
        let mut idx = 0usize;
        while idx < words.len() {
            if words[idx] == text {
                return true;
            }
            idx += 1;
        }
        false
    })
}

fn search_put_attachment_target(
    search_tokens: &[OwnedLexToken],
    put_idx: Option<usize>,
) -> Result<Option<TargetAst>, CardTextError> {
    let Some(put_idx) = put_idx else {
        return Ok(None);
    };
    let put_tokens = &search_tokens[put_idx..];
    let Some((_, _, target_tokens)) = primitives::find_prefix(put_tokens, || {
        use winnow::Parser as _;
        alt((
            primitives::phrase(&["attached", "to"]),
            primitives::phrase(&["attach", "it", "to"]),
            primitives::phrase(&["attach", "them", "to"]),
            primitives::phrase(&["attach", "that", "card", "to"]),
        ))
        .void()
    }) else {
        return Ok(None);
    };
    let target_tokens = primitives::split_lexed_once_on_separator(target_tokens, || {
        use winnow::Parser as _;
        alt((
            primitives::comma().void(),
            primitives::kw("and").void(),
            primitives::kw("then").void(),
        ))
        .void()
    })
    .map(|(before, _)| before)
    .unwrap_or(target_tokens);
    let target_tokens = trim_commas(target_tokens);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = parser_token_word_refs(&target_tokens);
    let search_words = parser_token_word_refs(search_tokens);
    if target_words.len() == 2
        && target_words[0].eq_ignore_ascii_case("that")
        && target_words[1].eq_ignore_ascii_case("player")
        && crate::word_primitives::sequence_occurs(&search_words, &["enchanted", "player"])
    {
        return Ok(Some(TargetAst::Player(
            PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::Enchanted.key()),
            span_from_tokens(&target_tokens),
        )));
    }
    parse_target_phrase(&target_tokens).map(Some)
}

fn words_contain_all(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|required_word| words.iter().any(|word| word == required_word))
}

fn tokens_contain_any_non_article_word(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    let source_words = crate::util::non_article_token_word_refs(tokens);
    source_words.iter().any(|word| {
        let mut idx = 0usize;
        while idx < words.len() {
            if words[idx] == *word {
                return true;
            }
            idx += 1;
        }
        false
    })
}

fn zones_have(zones: &[Zone], expected: Zone) -> bool {
    let mut idx = 0usize;
    while idx < zones.len() {
        if zones[idx] == expected {
            return true;
        }
        idx += 1;
    }
    false
}

fn is_cant_negation_word(word: &str) -> bool {
    matches!(word, "can't" | "cant" | "cannot")
}

fn is_dont_negation_word(word: &str) -> bool {
    matches!(word, "doesn't" | "doesnt" | "don't" | "dont")
}

fn is_does_do_can_word(word: &str) -> bool {
    matches!(word, "does" | "do" | "can")
}

fn is_does_or_do_word(word: &str) -> bool {
    matches!(word, "does" | "do")
}

fn is_control_or_own_word(word: &str) -> bool {
    matches!(word, "control" | "controls" | "own" | "owns")
}

fn is_compact_negated_action_word(word: &str) -> bool {
    matches!(
        word,
        "cant" | "can't" | "cannot" | "doesnt" | "didnt" | "doesn't" | "didn't"
    )
}

fn is_prevent_damage_source_head_word(word: &str) -> bool {
    matches!(word, "target" | "that" | "this" | "it")
}

fn is_prevent_damage_explicit_target_source(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    crate::word_primitives::sequence_occurs(&words, &["target"])
        && !crate::word_primitives::sequence_occurs(&words, &["other", "than", "target"])
}

fn is_prevent_damage_explicit_reference_word(word: &str) -> bool {
    matches!(word, "this" | "that" | "it")
}

pub fn cant_sentence_clause_tokens_for_restriction_scan_lexed(
    clause_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    split_lexed_sentences(clause_tokens)
        .into_iter()
        .next()
        .unwrap_or(clause_tokens)
        .to_vec()
}

pub fn cant_sentence_has_supported_negation_gate_lexed(clause_tokens: &[OwnedLexToken]) -> bool {
    let Some((neg_start, _)) = find_cant_sentence_negation_span_lexed(clause_tokens) else {
        return false;
    };

    !clause_tokens[..neg_start]
        .iter()
        .enumerate()
        .any(|(index, token)| {
            token_is_any_word(token, &["and"])
                && !clause_tokens
                    .get(index + 1)
                    .is_some_and(|next| token_is_any_word(next, &["each", "every"]))
        })
}

pub fn find_cant_sentence_negation_span_lexed(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.as_word().is_some_and(is_cant_negation_word) {
            return Some((cursor, cursor + 1));
        }
        if token.as_word().is_some_and(is_dont_negation_word) {
            if cursor >= 2
                && token_word_refs(&tokens[cursor - 2..cursor]).as_slice() == IF_YOU_PHRASE
            {
                cursor += 1;
                continue;
            }
            if tokens
                .get(cursor + 1)
                .is_some_and(|next| next.as_word().is_some_and(is_control_or_own_word))
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 1));
        }
        if token.as_word().is_some_and(is_does_do_can_word)
            && tokens
                .get(cursor + 1)
                .is_some_and(|next| token_is_any_word(next, &["not"]))
        {
            if cursor >= 2
                && token_word_refs(&tokens[cursor - 2..cursor]).as_slice() == IF_YOU_PHRASE
            {
                cursor += 2;
                continue;
            }
            if token.as_word().is_some_and(is_does_or_do_word)
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|next| next.as_word().is_some_and(is_control_or_own_word))
            {
                cursor += 1;
                continue;
            }
            return Some((cursor, cursor + 2));
        }
        cursor += 1;
    }

    None
}

fn cant_sentence_next_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["during", "that", "players", "next", "turn"]),
        primitives::phrase(&["during", "that", "player's", "next", "turn"]),
        primitives::phrase(&["during", "that", "player", "s", "next", "turn"]),
    ))
    .void()
    .parse_next(input)
}

fn cant_sentence_for_as_long_as_marker<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

pub fn split_cant_sentence_next_turn_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let mut cursor = 0usize;

    while cursor < tokens.len() {
        let Some((_, rest)) =
            primitives::parse_prefix(&tokens[cursor..], cant_sentence_next_turn_suffix)
        else {
            cursor += 1;
            continue;
        };
        if token_slice_all_are_kind(rest, TokenKind::Period) {
            return Some(tokens[..cursor].to_vec());
        }
        cursor += 1;
    }

    None
}

#[derive(Debug, Clone)]
pub struct CantSentencePreparedClause {
    pub duration: crate::effect::Until,
    pub duration_surface: crate::effect::RestrictionDurationSurface,
    pub clause_tokens: Vec<OwnedLexToken>,
}

pub fn prepare_cant_sentence_restriction_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<CantSentencePreparedClause>, CardTextError> {
    let duration_surface = parse_search_restriction_duration_shape_lexed(tokens)?
        .filter(|shape| shape.placement == SearchRestrictionDurationPlacement::Prefix)
        .map(|shape| match shape.duration {
            crate::effect::Until::EndOfTurn => {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            }
            crate::effect::Until::YourNextTurn => {
                crate::effect::RestrictionDurationSurface::LeadingUntilYourNextTurn
            }
            _ => crate::effect::RestrictionDurationSurface::Default,
        })
        .unwrap_or_default();
    let Some((duration, clause_tokens)) = parse_restriction_duration_lexed(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if split_leading_result_prefix_lexed(&clause_tokens).is_some()
        || clause_tokens
            .first()
            .is_some_and(|token| token_is_any_word(token, &["when", "whenever"]))
    {
        // A leading result/trigger intro owns the sentence ("If the player
        // doesn't, creatures they control can't attack you this turn" or
        // "When you discard a creature card this way, target creature you
        // control can't be blocked this turn"). The bare restriction rule
        // must not reinterpret the intro words as a subject filter. A leading
        // state condition which is not an IfResult stays here: conditional
        // restrictions ("If you have no cards in hand, this spell can't be
        // countered..." — Demonfire) are this rule's own shape.
        return Ok(None);
    }

    let clause_tokens = cant_sentence_clause_tokens_for_restriction_scan_lexed(&clause_tokens);
    if !cant_sentence_has_supported_negation_gate_lexed(&clause_tokens) {
        return Ok(None);
    }

    let coordinated_members =
        chain_splitting::split_effect_chain_on_and_tokens(&clause_tokens, true);
    if coordinated_members.len() > 1
        && coordinated_members.iter().any(|member| {
            super::activation_restrictions::parse_activation_negation_span_tokens(member).is_none()
                && (chain_splitting::find_chain_verb_tokens(member).is_some()
                    || chain_splitting::has_extended_effect_head_tokens(member))
        })
    {
        // A complete affirmative member is not part of the restriction.
        // Leave mixed action/restriction coordination to the typed chain
        // grammar so no member can be silently discarded by cant lowering.
        return Ok(None);
    }

    Ok(Some(CantSentencePreparedClause {
        duration,
        duration_surface,
        clause_tokens,
    }))
}

fn conditional_label_delimiter<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::token_kind(TokenKind::Dash).void(),
        primitives::token_kind(TokenKind::EmDash).void(),
    ))
    .parse_next(input)
}

fn labeled_effect_prefix<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    (conditional_label_phrase, conditional_label_delimiter)
        .void()
        .parse_next(input)
}

pub fn split_labeled_effect_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, labeled_effect_prefix)?;
    Some(rest)
}

fn labeled_prefix_tokens(prefix: &str) -> Option<Vec<OwnedLexToken>> {
    lex_line(prefix.trim(), 0).ok()
}

pub fn is_labeled_ability_prefix_text(prefix: &str) -> bool {
    let Some(tokens) = labeled_prefix_tokens(prefix) else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    is_labeled_ability_prefix_words(&words)
}

fn is_labeled_ability_prefix_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    if let Some(rest) = primitives::parse_word_sequence_prefix(words, &["descend"])
        && rest.len() == 1
        && rest[0].chars().all(|ch| ch.is_ascii_digit())
    {
        return true;
    }

    if LABELED_ABILITY_EXACT_PHRASES
        .iter()
        .any(|expected| primitives::parse_word_sequence_complete(words, expected).is_some())
    {
        return true;
    }

    LABELED_ABILITY_FIRST_WORDS
        .iter()
        .any(|word| primitives::parse_word_sequence_prefix(words, &[*word]).is_some())
}

pub fn preserve_labeled_ability_prefix_for_parse_text(prefix: &str) -> bool {
    let Some(tokens) = labeled_prefix_tokens(prefix) else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    let Some(first) = words.first().copied() else {
        return false;
    };
    if parser_token_word_refs(&tokens).as_slice() == MAX_SPEED_LABEL {
        return true;
    }

    matches!(
        first,
        "buyback"
            | "bestow"
            | "cumulative"
            | "cycling"
            | "echo"
            | "equip"
            | "escape"
            | "flashback"
            | "harmonize"
            | "boast"
            | "modular"
            | "partner"
            | "replicate"
            | "reinforce"
            | "renew"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

fn is_generic_ability_label_prefix_text(prefix: &str) -> bool {
    let Some(tokens) = labeled_prefix_tokens(prefix) else {
        return false;
    };
    let words = parser_token_word_refs(&tokens);
    if words.is_empty() || words.len() > 4 {
        return false;
    }

    words.iter().all(|word| {
        word.chars().all(|ch| ch.is_ascii_alphanumeric())
            && word.chars().any(|ch| ch.is_ascii_alphabetic())
    })
}

fn starts_with_if_clause_text(text: &str) -> bool {
    let Some(tokens) = lex_line(text.trim_start(), 0).ok() else {
        return false;
    };
    parser_token_word_refs(&tokens)
        .first()
        .is_some_and(|word| *word == "if")
}

pub fn should_strip_labeled_ability_prefix_text(prefix: &str, remainder: &str) -> bool {
    is_labeled_ability_prefix_text(prefix)
        || (starts_with_if_clause_text(remainder) && is_generic_ability_label_prefix_text(prefix))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseNewTargetsClauseSplit<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub count: Option<ChoiceCount>,
    pub explicit_target: bool,
    pub reference_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeTargetClauseSplit {
    pub target_tokens: Vec<OwnedLexToken>,
    pub fixed_to_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForEachDoesntClauseSplit<'a> {
    pub inner_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
    pub negation_idx: usize,
    pub negation_len: usize,
}

const CHOOSE_NEW_TARGET_PREFIXES: &[&[&str]] = &[
    &["choose", "new", "targets", "for"],
    &["chooses", "new", "targets", "for"],
    &["choose", "a", "new", "target", "for"],
    &["chooses", "a", "new", "target", "for"],
];
const CHOOSE_NEW_TARGET_REFERENCE_PREFIXES: &[&[&str]] = &[
    &["it"],
    &["them"],
    &["the", "copy"],
    &["the", "copies"],
    &["that", "copy"],
    &["those", "copies"],
    &["the", "spell"],
    &["that", "spell"],
];
const CHANGE_TARGET_PREFIXES: &[&[&str]] = &[
    &["change", "the", "target", "of"],
    &["change", "the", "targets", "of"],
    &["change", "a", "target", "of"],
];
const FOR_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
    &["each", "opponent"],
    &["each", "opponents"],
];
const FOR_EACH_PLAYER_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["each", "player"],
    &["each", "players"],
];

pub fn split_choose_new_targets_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ChooseNewTargetsClauseSplit<'_>> {
    let (_, mut tail_tokens) =
        primitives::strip_lexed_prefix_phrases(tokens, CHOOSE_NEW_TARGET_PREFIXES)?;
    if tail_tokens.is_empty() {
        return None;
    }

    if let Some((before_if, _)) = primitives::split_lexed_once_on_separator(tail_tokens, || {
        use winnow::Parser as _;
        primitives::kw("if").void()
    }) {
        tail_tokens = before_if;
    }
    if tail_tokens.is_empty() {
        return None;
    }

    if primitives::starts_with_any_phrase(tail_tokens, CHOOSE_NEW_TARGET_REFERENCE_PREFIXES) {
        return Some(ChooseNewTargetsClauseSplit {
            target_tokens: tail_tokens,
            count: None,
            explicit_target: false,
            reference_target: true,
        });
    }

    if let Some((prefix, rest)) = primitives::strip_lexed_prefix_phrases(
        tail_tokens,
        &[&["any", "number", "of"], &["target"]],
    ) {
        return Some(ChooseNewTargetsClauseSplit {
            target_tokens: rest,
            count: (prefix.len() == 3).then_some(ChoiceCount::any_number()),
            explicit_target: prefix.len() != 3,
            reference_target: false,
        });
    }

    Some(ChooseNewTargetsClauseSplit {
        target_tokens: tail_tokens,
        count: None,
        explicit_target: false,
        reference_target: false,
    })
}

pub fn split_change_target_unless_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    primitives::split_lexed_once_on_separator(tokens, || {
        use winnow::Parser as _;
        primitives::kw("unless").void()
    })
    .map(|(main, unless)| (trim_lexed_commas(main), trim_lexed_commas(unless)))
}

pub fn split_change_target_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ChangeTargetClauseSplit> {
    let (_, after_prefix_tokens) =
        primitives::strip_lexed_prefix_phrases(tokens, CHANGE_TARGET_PREFIXES)?;
    if after_prefix_tokens.is_empty() {
        return None;
    }

    let mut tail_tokens = trim_commas(after_prefix_tokens).to_vec();
    let mut fixed_to_source = false;
    if let Some((before_to, to_tail)) =
        primitives::split_lexed_once_on_separator(&tail_tokens, || {
            use winnow::Parser as _;
            primitives::kw("to").void()
        })
        && token_slice_first_is(to_tail, "this")
    {
        fixed_to_source = true;
        tail_tokens.truncate(before_to.len());
    }

    Some(ChangeTargetClauseSplit {
        target_tokens: tail_tokens,
        fixed_to_source,
    })
}

pub fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    let mut idx = 0usize;
    while idx < words.len() {
        if is_compact_negated_action_word(words[idx]) {
            return Some((idx, 1));
        }
        idx += 1;
    }
    let mut phrase_idx = 0usize;
    while phrase_idx < SPLIT_NEGATED_ACTION_PHRASES.len() {
        let phrase = SPLIT_NEGATED_ACTION_PHRASES[phrase_idx];
        let mut start = 0usize;
        while start + phrase.len() <= words.len() {
            let mut offset = 0usize;
            let mut matched = true;
            while offset < phrase.len() {
                if words[start + offset] != phrase[offset] {
                    matched = false;
                    break;
                }
                offset += 1;
            }
            if matched {
                debug_assert_eq!(phrase.len(), 2);
                return Some((start, 2));
            }
            start += 1;
        }
        phrase_idx += 1;
    }
    None
}

fn split_for_each_doesnt_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
    prefixes: &'static [&'static [&'static str]],
) -> Option<ForEachDoesntClauseSplit<'a>> {
    let mut clause_tokens = tokens;
    if token_word_refs(clause_tokens)
        .first()
        .is_some_and(|word| *word == "then")
    {
        clause_tokens = &clause_tokens[1..];
    }
    let (_, rest_tokens) = primitives::strip_lexed_prefix_phrases(clause_tokens, prefixes)?;
    let inner_tokens = trim_lexed_commas(rest_tokens);
    let inner_clause = LexedClause::new(inner_tokens);
    let inner_words = token_word_refs(inner_tokens);
    if inner_words.first().is_none_or(|word| *word != "who") {
        return None;
    }
    let (negation_idx, negation_len) = negated_action_word_index(&inner_words)?;
    // In `who can't <effect>`, `can't` refers to the preceding per-player
    // action, so the effect begins immediately after it. A later comma can
    // belong to that effect's value surface (`half their life, rounded up`)
    // and must not be mistaken for the predicate/effect boundary used by
    // `who doesn't <predicate>, <effect>` clauses.
    let effect_token_start = if is_cant_negation_word(inner_words[negation_idx]) {
        inner_clause
            .after_words(negation_idx + negation_len)
            .map(|tail| inner_tokens.len() - tail.tokens().len())
            .unwrap_or(inner_tokens.len())
    } else if let Some((_, after_comma)) = primitives::split_lexed_once_on_comma(inner_tokens) {
        inner_tokens.len() - after_comma.len()
    } else if let Some(this_way) =
        primitives::parse_word_sequence_span(&inner_words, THIS_WAY_PHRASE)
    {
        inner_clause
            .after_words(this_way.start + this_way.len)
            .map(|tail| inner_tokens.len() - tail.tokens().len())
            .unwrap_or(inner_tokens.len())
    } else {
        inner_clause
            .after_words(negation_idx + negation_len)
            .map(|tail| inner_tokens.len() - tail.tokens().len())
            .unwrap_or(inner_tokens.len())
    };
    let effect_tokens = trim_lexed_commas(&inner_tokens[effect_token_start..]);
    (!effect_tokens.is_empty()).then_some(ForEachDoesntClauseSplit {
        inner_tokens,
        effect_tokens,
        negation_idx,
        negation_len,
    })
}

pub fn split_for_each_opponent_doesnt_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDoesntClauseSplit<'_>> {
    split_for_each_doesnt_clause_lexed(tokens, FOR_EACH_OPPONENT_PREFIXES)
}

pub fn split_for_each_player_doesnt_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ForEachDoesntClauseSplit<'_>> {
    split_for_each_doesnt_clause_lexed(tokens, FOR_EACH_PLAYER_PREFIXES)
}

pub fn split_negated_who_this_way_filter_tokens_lexed(
    inner_tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let inner_clause = LexedClause::new(inner_tokens);
    let inner_words = token_word_refs(inner_tokens);
    if inner_words.first().is_none_or(|word| *word != "who") {
        return None;
    }
    let this_way_idx = primitives::parse_word_sequence_span(&inner_words, THIS_WAY_PHRASE)?.start;
    let (negation_idx, negation_len) = negated_action_word_index(&inner_words)?;
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !matches!(verb, "discard" | "discarded") || this_way_idx <= verb_idx + 1 {
        return None;
    }

    let filter_clause = inner_clause
        .between_word_range(verb_idx + 1, this_way_idx)?
        .trimmed();
    let filter_tokens = filter_clause.tokens();
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}

const PREVENT_DAMAGE_BY_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "by"]];
const PREVENT_DAMAGE_TO_AND_BY_PREFIXES: &[&[&str]] =
    &[&["that", "would", "be", "dealt", "to", "and", "dealt", "by"]];
const PREVENT_DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "to"]];

fn parse_prevent_damage_source_excluding_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::target::ObjectFilter, TargetAst)>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    let Some(marker) = primitives::parse_word_sequence_span(&words, &["other", "than"]) else {
        return Ok(None);
    };
    let Some(source_clause) = clause.between_word_range(0, marker.start) else {
        return Ok(None);
    };
    let Some(excluded_clause) = clause.after_words(marker.start + marker.len) else {
        return Ok(None);
    };
    if source_clause.trimmed().tokens().is_empty() || excluded_clause.trimmed().tokens().is_empty()
    {
        return Ok(None);
    }
    let source_filter = parse_object_filter(source_clause.trimmed().tokens(), false)?;
    let excluded_target = parse_target_phrase(excluded_clause.trimmed().tokens())?;
    Ok(Some((source_filter, excluded_target)))
}

pub fn parse_prevent_damage_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if primitives::parse_prefix(tokens, primitives::phrase(PREVENT_ALL_COMBAT_DAMAGE_PREFIX))
        .is_none()
    {
        return Ok(None);
    }

    let clause = LexedClause::new(tokens);
    let Some(this_turn) = primitives::parse_word_sequence_span(&words, THIS_TURN_PHRASE) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    };
    let this_turn_idx = this_turn.start;
    if clause.after_words(this_turn_idx + 2).is_some_and(|tail| {
        primitives::parse_prefix(tail.tokens(), primitives::phrase(THIS_TURN_PHRASE))
            .is_some_and(|(_, rest)| rest.is_empty())
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }
    if this_turn_idx < PREVENT_ALL_COMBAT_DAMAGE_PREFIX.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }

    let before_duration = clause
        .between_word_range(PREVENT_ALL_COMBAT_DAMAGE_PREFIX.len(), this_turn_idx)
        .map(|part| part.tokens())
        .unwrap_or(&[]);
    let after_duration = clause
        .after_words(this_turn_idx + THIS_TURN_PHRASE.len())
        .map(|part| part.tokens())
        .unwrap_or(&[]);
    let mut core_tokens = Vec::with_capacity(before_duration.len() + after_duration.len());
    core_tokens.extend_from_slice(before_duration);
    core_tokens.extend_from_slice(after_duration);
    let core_clause = LexedClause::new(&core_tokens);
    let core_words = core_clause.word_refs();

    if primitives::parse_prefix(&core_tokens, primitives::phrase(THAT_WOULD_BE_DEALT_PHRASE))
        .is_some_and(|(_, rest)| rest.is_empty())
    {
        return Ok(Some(EffectAst::subject_verb_prevent_all_combat_damage(
            crate::effect::Until::EndOfTurn,
        )));
    }

    if let Some((_, source_tokens)) =
        primitives::strip_lexed_prefix_phrases(&core_tokens, PREVENT_DAMAGE_BY_PREFIXES)
    {
        // A target-relative source set must retain both identity arms. Parsing
        // this as an ordinary object filter makes "that creature" resolve to
        // the most recent collected set and silently drops the spell target.
        if crate::word_primitives::parse_sequence_complete(
            &parser_token_word_refs(source_tokens),
            &[
                "that", "creature", "and", "each", "creature", "blocking", "it",
            ],
        ) {
            let mut target_creature = ObjectFilter::creature();
            target_creature.is_target_object = true;

            let mut blockers = ObjectFilter::creature();
            blockers.blocking = true;
            blockers.in_combat_with = Some(ObjectRef::Target);

            let mut source_filter = ObjectFilter::default();
            source_filter.any_of = vec![target_creature, blockers];
            source_filter.set_conjunctive_set_surface(true);
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
                    source_filter,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        if let Some((source_filter, excluded_target)) =
            parse_prevent_damage_source_excluding_target(source_tokens)?
        {
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter_excluding_target(
                    source_filter,
                    excluded_target,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        if source_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_prevent_damage_source_head_word)
            || is_prevent_damage_explicit_target_source(source_tokens)
        {
            let (source, has_color_condition) =
                parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
            return Ok(Some(prevent_damage_effect_with_optional_condition(
                source,
                has_color_condition,
                false,
            )));
        }
        if let Ok(source_filter) = parse_object_filter(source_tokens, false) {
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
                    source_filter,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
            false,
        )));
    }

    if let Some((_, source_tokens)) =
        primitives::strip_lexed_prefix_phrases(&core_tokens, PREVENT_DAMAGE_TO_AND_BY_PREFIXES)
    {
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
            false,
        )));
    }

    if let Some((_, target_tokens)) =
        primitives::strip_lexed_prefix_phrases(&core_tokens, PREVENT_DAMAGE_TO_PREFIXES)
    {
        return parse_prevent_damage_target_scope_lexed(target_tokens, &words);
    }

    if let Some(would_deal) = primitives::parse_word_sequence_span(&core_words, &["would", "deal"])
    {
        let would_idx = would_deal.start;
        let Some(source_clause) = core_clause.before_word(would_idx) else {
            return Ok(None);
        };
        let source_tokens = source_clause.tokens();
        if !source_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(is_prevent_damage_source_head_word)
            && !is_prevent_damage_explicit_target_source(source_tokens)
            && let Ok(source_filter) = parse_object_filter(source_tokens, false)
        {
            return Ok(Some(
                EffectAst::subject_verb_prevent_all_combat_damage_from_source_filter(
                    source_filter,
                    crate::effect::Until::EndOfTurn,
                ),
            ));
        }
        let (source, has_color_condition) =
            parse_prevent_damage_source_target_lexed(source_tokens, &words)?;
        let damage_tail = core_clause
            .after_words(would_idx + 2)
            .map(|part| part.tokens())
            .unwrap_or(&[]);
        let has_color_condition =
            has_color_condition || prevent_damage_shares_color_clause_lexed(damage_tail);
        return Ok(Some(prevent_damage_effect_with_optional_condition(
            source,
            has_color_condition,
            true,
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all-combat-damage clause tail (clause: '{}')",
        words.join(" ")
    )))
}

pub fn parse_prevent_damage_source_target_lexed(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(TargetAst, bool), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all source target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (tokens, has_color_condition) = strip_prevent_damage_shares_color_clause_lexed(tokens);
    let source_words = crate::util::non_article_token_word_refs(tokens);
    let is_explicit_reference =
        tokens_contain_any_non_article_word(tokens, &["target", "this", "that", "it"])
            || source_words
                .first()
                .is_some_and(|word| is_prevent_damage_explicit_reference_word(word));
    if !is_explicit_reference {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            source_words.join(" ")
        )));
    }

    let source = parse_target_phrase(tokens)?;
    match source {
        TargetAst::Source(_)
        | TargetAst::Object(_, _, _)
        | TargetAst::Tagged(_, _)
        | TargetAst::WithCount(_, _)
        | TargetAst::WithCountValue(_, _, _) => Ok((source, has_color_condition)),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            token_word_refs(tokens).join(" ")
        ))),
    }
}

fn prevent_damage_effect_with_optional_condition(
    source: TargetAst,
    has_color_condition: bool,
    source_would_deal_surface: bool,
) -> EffectAst {
    let condition_filter = match &source {
        TargetAst::Object(filter, _, _) => Some(filter.clone()),
        _ => None,
    };
    let prevent = if source_would_deal_surface {
        EffectAst::subject_verb_prevent_all_combat_damage_source_would_deal(
            source,
            crate::effect::Until::EndOfTurn,
        )
    } else {
        EffectAst::subject_verb_prevent_all_combat_damage_from_source(
            source,
            crate::effect::Until::EndOfTurn,
        )
    };
    if has_color_condition {
        let predicate = condition_filter.map_or_else(
            || {
                PredicateAst::TargetMatches(
                    ObjectFilter::default()
                        .shares_color_with_tagged(crate::tag::CompilerReferenceTag::It.key()),
                )
            },
            |filter| {
                PredicateAst::TargetMatches(
                    filter.shares_color_with_tagged(crate::tag::CompilerReferenceTag::It.key()),
                )
            },
        );
        EffectAst::Conditional {
            predicate,
            if_true: vec![prevent],
            if_false: Vec::new(),
        }
    } else {
        prevent
    }
}

fn prevent_damage_shares_color_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::util::non_article_token_word_refs(tokens);
    [
        &["if", "it", "shares", "color", "with", "that", "permanent"][..],
        &["if", "it", "shares", "color", "with", "that", "object"][..],
        &["if", "it", "shares", "color", "with", "that", "creature"][..],
        &["if", "it", "shares", "color", "with", "it"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&words, expected).is_some())
}

fn strip_prevent_damage_shares_color_clause_lexed(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], bool) {
    let clause = LexedClause::new(tokens);
    let Some(if_idx) = clause.parse_last_word_position("if") else {
        return (tokens, false);
    };
    let Some(if_clause) = clause.from_word(if_idx) else {
        return (tokens, false);
    };
    if prevent_damage_shares_color_clause_lexed(if_clause.tokens())
        && let Some(head) = clause.before_word(if_idx)
    {
        return (head.tokens(), true);
    }
    (tokens, false)
}

pub fn parse_prevent_damage_target_scope_lexed(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = crate::util::non_article_token_word_refs(tokens);
    if target_words.len() == 1
        && target_words
            .first()
            .is_some_and(|word| matches!(*word, "player" | "players"))
    {
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_combat_damage_to_players(
                crate::effect::Until::EndOfTurn,
            ),
        ));
    }
    if target_words.len() == 1 && target_words.first().is_some_and(|word| *word == "you") {
        return Ok(Some(
            EffectAst::subject_verb_prevent_all_combat_damage_to_you(
                crate::effect::Until::EndOfTurn,
            ),
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all target scope '{}'",
        token_word_refs(tokens).join(" ")
    )))
}

fn conditional_sentence_family_head<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        primitives::phrase(&["then", "if"]),
        (
            conditional_label_phrase,
            opt(conditional_label_delimiter),
            primitives::kw("if"),
        )
            .void(),
        primitives::kw("if").void(),
    ))
    .parse_next(input)
}

pub fn split_conditional_sentence_family_head_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (_, rest) = primitives::parse_prefix(tokens, conditional_sentence_family_head)?;
    let consumed = tokens.len().checked_sub(rest.len())?;
    consumed.checked_sub(1).map(|if_idx| &tokens[if_idx..])
}

pub fn parse_conditional_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let split = split_if_clause_lexed(tokens, parse_effect_chain_lexed)?;

    Ok(vec![match split.predicate {
        IfClausePredicateSpec::Conditional(predicate) => EffectAst::Conditional {
            predicate,
            if_true: split.effects,
            if_false: Vec::new(),
        },
        IfClausePredicateSpec::Result(predicate) => EffectAst::IfResult {
            predicate,
            effects: split.effects,
        },
    }])
}

pub fn parse_conditional_sentence_family_lexed(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(conditional_tokens) = split_conditional_sentence_family_head_lexed(tokens) else {
        return Ok(None);
    };

    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        conditional_tokens,
        parse_effect_chain_lexed,
    )
    .map(Some)
}

/// Parse the player subject of a resolving rule that removes maximum hand
/// size for the rest of the game.
///
/// The duration is intentionally part of this grammar. A bare "You have no
/// maximum hand size" is a static ability, while the same words followed by
/// "for the rest of the game" establish a rule as a resolving effect.
pub fn parse_persistent_no_maximum_hand_size_player_lexed(
    tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let words = token_word_refs(tokens);
    let (player, subject_words) = if crate::word_primitives::parse_sequence_prefix(&words, &["you"])
    {
        (PlayerFilter::You, 1)
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["each", "player"]) {
        (PlayerFilter::Any, 2)
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["each", "opponent"]) {
        (PlayerFilter::Opponent, 2)
    } else {
        return None;
    };
    crate::word_primitives::parse_choice_sequence_complete(
        &words[subject_words..],
        &[
            &["have", "has"],
            &["no"],
            &["maximum"],
            &["hand"],
            &["size"],
            &["for"],
            &["the"],
            &["rest"],
            &["of"],
            &["the"],
            &["game"],
        ],
    )
    .then_some(player)
}

pub fn parse_cant_effect_sentence_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = token_word_refs(tokens);
    let parser_words = parser_token_word_refs(tokens);
    if crate::word_primitives::parse_choice_sequence_complete(
        &parser_words,
        &[
            &["this"],
            &["creature"],
            &["cant", "can't"],
            &["be"],
            &["blocked"],
            &["this"],
            &["combat"],
        ],
    ) {
        return Ok(Some(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::be_blocked(ObjectFilter::source_with_surface(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            )),
            crate::effect::Until::EndOfCombat,
            None,
        )]));
    }
    if crate::word_primitives::parse_choice_sequence_complete(
        &parser_words,
        &[
            &["that"],
            &["creature"],
            &["doesnt", "doesn't"],
            &["untap"],
            &["during"],
            &["its"],
            &["controllers"],
            &["next"],
            &["untap"],
            &["step"],
        ],
    ) {
        return Ok(Some(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::Untap(ObjectFilter::tagged(
                crate::tag::CompilerReferenceTag::It.key(),
            )),
            crate::effect::Until::ControllersNextUntapStep,
            None,
        )]));
    }
    if words.get(..3).is_some_and(|prefix| {
        matches!(
            prefix,
            [
                "each",
                "opponent" | "opponents" | "player" | "players",
                "who"
            ]
        )
    }) || words.get(..4).is_some_and(|prefix| {
        matches!(
            prefix,
            [
                "for",
                "each",
                "opponent" | "opponents" | "player" | "players",
                "who"
            ]
        )
    }) {
        return Ok(None);
    }

    if let Some(player) = parse_persistent_no_maximum_hand_size_player_lexed(tokens) {
        return Ok(Some(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::no_maximum_hand_size(player),
            crate::effect::Until::Forever,
            None,
        )]));
    }

    if let Some(prefix_tokens) = split_cant_sentence_next_turn_prefix_lexed(tokens) {
        let prefix_tokens = prefix_tokens.as_slice();
        if let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? {
            let next_turn_effects = match parsed.restriction {
                crate::effect::Restriction::CastSpellsMatching(player, spell_filter) => {
                    let restriction = crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::IteratedPlayer,
                        spell_filter,
                    );
                    let restriction = EffectAst::subject_verb_cant_starting(
                        restriction,
                        crate::effect::Until::EndOfTurn,
                        crate::effect::RestrictionStart::NextTurn(PlayerFilter::IteratedPlayer),
                        None,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![restriction],
                        }]),
                        PlayerFilter::IteratedPlayer => Some(vec![restriction]),
                        _ => None,
                    }
                }
                crate::effect::Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
                    let restriction = crate::effect::Restriction::CastMoreThanOneSpellEachTurn(
                        PlayerFilter::IteratedPlayer,
                        spell_filter,
                    );
                    let restriction = EffectAst::subject_verb_cant_starting(
                        restriction,
                        crate::effect::Until::EndOfTurn,
                        crate::effect::RestrictionStart::NextTurn(PlayerFilter::IteratedPlayer),
                        None,
                    );
                    match player {
                        PlayerFilter::Opponent => Some(vec![EffectAst::ForEachOpponent {
                            effects: vec![restriction],
                        }]),
                        PlayerFilter::IteratedPlayer => Some(vec![restriction]),
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(next_turn_effects) = next_turn_effects {
                return Ok(Some(next_turn_effects));
            }
        }
    }

    let source_tapped_duration = cant_sentence_has_source_remains_tapped_duration(tokens);
    if words_contain_all(&words, LOSE_MANA_STEPS_PHASES_END_WORDS) {
        return Ok(Some(vec![
            EffectAst::subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn(),
        ]));
    }
    let Some(prepared_clause) = prepare_cant_sentence_restriction_clause_lexed(tokens)? else {
        return Ok(None);
    };
    let duration = prepared_clause.duration;
    let duration_surface = prepared_clause.duration_surface;
    let clause_tokens = prepared_clause.clause_tokens;

    if let Some(fact) =
        super::activation_costs::cant_shapes::parse_per_attacker_cant_tax_tokens(&clause_tokens)
    {
        return Ok(Some(vec![
            EffectAst::subject_verb_cant_starting_with_duration_surface(
                crate::effect::Restriction::attack_you_unless_controller_pays_per_attacker(
                    fact.amount,
                    fact.covers_planeswalkers,
                ),
                duration,
                crate::effect::RestrictionStart::Immediate,
                duration_surface,
                source_tapped_duration.then_some(crate::ConditionExpr::SourceIsTapped),
            ),
        ]));
    }

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            token_word_refs(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<crate::cards::builders::TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        token_word_refs(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::subject_verb_cant_starting_with_duration_surface(
            parsed.restriction,
            duration.clone(),
            crate::effect::RestrictionStart::Immediate,
            duration_surface,
            source_tapped_duration.then_some(crate::ConditionExpr::SourceIsTapped),
        ));
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }

    Ok(Some(effects))
}

pub fn parse_cant_effect_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed(tokens)
}
#[path = "effects/cant_duration_shapes.rs"]
mod cant_duration_shapes;
pub use cant_duration_shapes::*;

#[path = "effects/effects_library.rs"]
mod effects_library_programs;
pub use effects_library_programs::parse_search_library_sentence_with_grammar_entrypoint_lexed;
