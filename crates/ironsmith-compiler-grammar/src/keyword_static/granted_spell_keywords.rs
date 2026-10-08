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
    let Some(kind) = tokens
        .get(verb_idx + 1)
        .and_then(granted_spell_keyword_kind)
    else {
        return Ok(None);
    };
    let mut tail = &tokens[verb_idx + 2..];
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
