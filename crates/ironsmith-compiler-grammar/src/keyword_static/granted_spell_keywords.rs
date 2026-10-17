//! Keywords granted to spells as they are cast (CR 601.2b, 601.2f):
//!
//! - "Each spell you cast that's exactly three colors has replicate {3}."
//!   (Threefold Signal)
//! - "Each Sliver spell you cast has replicate. The replicate cost is equal
//!   to its mana cost." (Hatchery Sliver, Djinn Illuminatus)
//! - "Creature spells you cast gain offspring {2} as you cast them." (Zinnia)
//! - "Creature spells you cast have demonstrate." (Silverquill Lecturer)
//! - "Each red or green instant or sorcery spell you cast has conspire."
//!   (Wort, the Raidmother)
//!
//! Each line becomes one typed `GrantSpellKeyword` static ability; the engine
//! discovers it while the matching spell is cast. Only spell subjects are
//! claimed: these keywords function while the spell is being cast.

use super::*;

const SPELL_MANA_COST_TAIL_PREFIX: &[&str] = &["the"];
const SPELL_MANA_COST_TAIL_SUFFIX: &[&str] = &["cost", "is", "equal", "to", "its", "mana", "cost"];

fn granted_spell_keyword_kind(
    token: &OwnedLexToken,
) -> Option<ironsmith_core::GrantedSpellKeywordKind> {
    use ironsmith_core::GrantedSpellKeywordKind as Kind;
    if token.is_word("replicate") {
        Some(Kind::Replicate)
    } else if token.is_word("offspring") {
        Some(Kind::Offspring)
    } else if token.is_word("conspire") {
        Some(Kind::Conspire)
    } else if token.is_word("demonstrate") {
        Some(Kind::Demonstrate)
    } else {
        None
    }
}

/// "<the> <keyword> cost is equal to its mana cost" (CR 702.56a's cost read
/// from the spell itself).
fn is_spell_mana_cost_tail(words: &[&str], keyword: &str) -> bool {
    let mut expected = SPELL_MANA_COST_TAIL_PREFIX.to_vec();
    expected.push(keyword);
    expected.extend_from_slice(SPELL_MANA_COST_TAIL_SUFFIX);
    crate::word_primitives::parse_sequence_complete(words, &expected)
}

/// Strip a trailing "as you cast them|it" from a "gain(s)" grant: a keyword a
/// spell gains as it is cast is the same grant as one it has (CR 601.2b).
fn strip_as_you_cast_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let as_idx = tokens.iter().rposition(|token| token.is_word("as"))?;
    let words = parser_token_word_refs(&tokens[as_idx..]);
    let complete = crate::word_primitives::parse_sequence_complete(
        &words,
        &["as", "you", "cast", "them"],
    ) || crate::word_primitives::parse_sequence_complete(&words, &["as", "you", "cast", "it"]);
    complete.then_some(&tokens[..as_idx])
}

/// Only a subject naming spells is claimed ("Creature spells you cast",
/// "Each instant and sorcery spell you cast"); the filter keeps its stack
/// zone, where the spell is while it is being cast (CR 601.2a).
fn granted_spell_subject_filter(filter: ObjectFilter) -> Option<ObjectFilter> {
    let names_spells = |filter: &ObjectFilter| {
        filter.zone == Some(Zone::Stack) || filter.stack_kind.is_some()
    };
    let is_spell_subject = names_spells(&filter)
        || (!filter.any_of.is_empty() && filter.any_of.iter().all(names_spells));
    is_spell_subject.then_some(filter)
}

/// An authored "Each ..." subject (presentation only).
fn leading_each_surface(
    subject: &[OwnedLexToken],
) -> Option<ironsmith_core::SetQuantifierSurface> {
    subject
        .first()
        .is_some_and(|token| token.is_word("each"))
        .then_some(ironsmith_core::SetQuantifierSurface::Each)
}

pub fn parse_granted_spell_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(verb_idx) = tokens
        .iter()
        .position(|token| token.is_any_word(&["have", "has", "gain", "gains"]))
    else {
        return Ok(None);
    };
    if verb_idx == 0 {
        return Ok(None);
    }
    if tokens.get(verb_idx + 1).is_some_and(|token| token.is_word("evoke")) {
        return parse_granted_hand_evoke(&tokens, verb_idx);
    }
    // "Creature spells you cast have sticker kicker {1}." (Wicker Picker)
    let sticker_kicker = tokens.get(verb_idx + 1).is_some_and(|token| token.is_word("sticker"))
        && tokens.get(verb_idx + 2).is_some_and(|token| token.is_word("kicker"));
    let (kind, keyword_len) = if sticker_kicker {
        (ironsmith_core::GrantedSpellKeywordKind::StickerKicker, 2)
    } else {
        let Some(kind) = tokens
            .get(verb_idx + 1)
            .and_then(granted_spell_keyword_kind)
        else {
            return Ok(None);
        };
        (kind, 1)
    };
    let mut tail = &tokens[verb_idx + 1 + keyword_len..];
    if tokens[verb_idx].is_any_word(&["gain", "gains"]) {
        let Some(stripped) = strip_as_you_cast_tail(tail) else {
            return Ok(None);
        };
        tail = stripped;
    }
    let tail_words = parser_token_word_refs(tail);
    let price = if kind.has_intrinsic_price() {
        if !tail_words.is_empty() {
            return Ok(None);
        }
        ironsmith_core::GrantedSpellKeywordPrice::Intrinsic
    } else if is_spell_mana_cost_tail(&tail_words, kind.keyword_text()) {
        ironsmith_core::GrantedSpellKeywordPrice::SpellManaCost
    } else {
        let Some(mana) = parse_leaf_mana_cost_prefix_tokens(tail) else {
            return Ok(None);
        };
        if !parser_token_word_refs(&tail[mana.consumed..]).is_empty() {
            return Ok(None);
        }
        ironsmith_core::GrantedSpellKeywordPrice::Fixed(ironsmith_core::TotalCost::mana(mana.cost))
    };
    let mut subject = &tokens[..verb_idx];
    let set_quantifier_surface = leading_each_surface(subject);
    if set_quantifier_surface.is_some() {
        subject = &subject[1..];
    }
    let filter = parse_object_filter_lexed(subject, false)?;
    let Some(filter) = granted_spell_subject_filter(filter) else {
        return Ok(None);
    };
    let keyword = ironsmith_core::GrantedSpellKeyword::new(kind, price);
    let display = crate::lexer::render_token_slice(&tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::grant_spell_keyword(
        filter,
        keyword,
        set_quantifier_surface,
        display,
    )))
}

/// "Elemental permanent spells you cast from your hand gain evoke {4} as you
/// cast them." (Ashling, the Limitless): an alternative cost (CR 702.74a)
/// for matching hand cards; a permanent cast this way gets "When this
/// permanent enters, sacrifice it." (the evoke sacrifice trigger).
fn parse_granted_hand_evoke(
    tokens: &[OwnedLexToken],
    verb_idx: usize,
) -> Result<Option<StaticAbility>, CardTextError> {
    let mut tail = &tokens[verb_idx + 2..];
    if tokens[verb_idx].is_any_word(&["gain", "gains"]) {
        let Some(stripped) = strip_as_you_cast_tail(tail) else {
            return Ok(None);
        };
        tail = stripped;
    }
    let Some(mana) = parse_leaf_mana_cost_prefix_tokens(tail) else {
        return Ok(None);
    };
    if !parser_token_word_refs(&tail[mana.consumed..]).is_empty() {
        return Ok(None);
    }
    let subject = &tokens[..verb_idx];
    let view = crate::lexer::TokenWordView::new(subject);
    let words = view.word_refs();
    const SUFFIX: &[&str] = &["you", "cast", "from", "your", "hand"];
    if !words.ends_with(SUFFIX) || words.len() <= SUFFIX.len() {
        return Ok(None);
    }
    let Some(span) = view.token_span_for_words(0, words.len() - SUFFIX.len()) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(&subject[span], false)?;
    filter.zone = None;
    filter.stack_kind = None;
    filter.cast_by = None;
    filter.owner = Some(crate::target::PlayerFilter::You);
    if !filter.excluded_card_types.contains(&crate::types::CardType::Land) {
        filter.excluded_card_types.push(crate::types::CardType::Land);
    }
    let rider_tokens = crate::lexer::lex_line("When this permanent enters, sacrifice it.", 0)?;
    let rider_words = parser_token_word_refs(&rider_tokens);
    let Some(rider) = crate::effect_sentences::parse_granted_activated_or_triggered_ability_for_gain(
        &rider_tokens,
        &rider_words,
    )?
    else {
        return Ok(None);
    };
    let rider = crate::static_ability_helpers::compiler_granted_ability_ast_to_object_ability(&rider)?;
    let method = ironsmith_core::AlternativeCastingMethod::<
        crate::cards::builders::EffectAst,
        crate::model::CompilerCost,
        ironsmith_core::ThisSpellCostCondition,
    >::cast_from_zone_with_total_cost(
        "Evoke",
        Zone::Hand,
        ironsmith_core::TotalCost::mana(mana.cost),
        None,
        false,
    );
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativeCast(method),
        filter,
        Zone::Hand,
    );
    spec.permanent_this_way_grants.push(
        crate::model::CompilerStaticAbilityCore::grant_object_ability_for_filter(
            ObjectFilter::source(),
            rider,
            "When this permanent enters, sacrifice it",
        ),
    );
    let display = crate::lexer::render_token_slice(tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    spec.filtered_zone_surface = Some(display);
    Ok(Some(StaticAbility::grants(spec)))
}

/// The typed grant for a "<subject> has conspire|demonstrate" line already
/// split into an anthem subject (used by the shared keyword-grant reader so
/// both readers produce the same ability).
pub fn granted_intrinsic_spell_keyword_ability(
    filter: ObjectFilter,
    action: &KeywordAction,
    subject_tokens: &[OwnedLexToken],
    display: String,
) -> Option<StaticAbility> {
    let kind = match action {
        KeywordAction::Conspire => ironsmith_core::GrantedSpellKeywordKind::Conspire,
        KeywordAction::Demonstrate => ironsmith_core::GrantedSpellKeywordKind::Demonstrate,
        _ => return None,
    };
    let filter = granted_spell_subject_filter(filter)?;
    Some(StaticAbility::grant_spell_keyword(
        filter,
        ironsmith_core::GrantedSpellKeyword::intrinsic(kind),
        leading_each_surface(subject_tokens),
        display,
    ))
}
