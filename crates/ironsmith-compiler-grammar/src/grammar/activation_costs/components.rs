use winnow::combinator::{alt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::mana::ManaSymbol;
use crate::object::CounterType;

#[cfg(any(test, feature = "test-support"))]
use super::super::super::lexer::lex_line;
use super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, render_token_slice, token_slice_at_is,
    token_slice_first_is,
};
use super::super::super::util::{is_source_reference_words, source_reference_surface_for_words};
use super::super::activated_lines::{
    ActivatedLoyaltyShorthand, parse_loyalty_shorthand_activation_tokens,
};
use super::super::keyword_action_costs::parse_payment_alternative_split_tokens;
use super::super::primitives;
use super::{
    ActivationCostCst, ActivationCostSegmentCst, ActivationCostSegmentKind,
    parse_activation_cost_segment_kind_tokens, parse_bare_symbol_segment_tokens,
    parse_behold_segment_tokens, parse_blight_segment_tokens, parse_forage_segment_tokens, parse_discard_segment_tokens,
    parse_roll_die_segment_tokens,
    parse_exert_segment_tokens, parse_exile_segment_tokens as parse_typed_exile_segment_tokens,
    parse_mill_segment_tokens, parse_move_source_to_library_bottom_cost_tokens,
    parse_move_to_library_top_cost_tokens, parse_pay_segment_tokens,
    parse_put_counter_segment_tokens,
    parse_return_segment_tokens, parse_reveal_segment_tokens,
    parse_sacrifice_segment_tokens as parse_typed_sacrifice_segment_tokens,
    parse_tap_chosen_segment_tokens, parse_untap_chosen_segment_tokens, parse_collect_evidence_segment_tokens, parse_unattach_segment_tokens,
};

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            return Some(idx);
        }
    }
    None
}

fn trim_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = first_non_comma_token_index(tokens).unwrap_or(tokens.len());
    let mut end = tokens.len();

    if token_slice_at_is(tokens, start, "and") {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    if token_slice_at_is(tokens, start, "waterbend") {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    while end > start && (tokens[end - 1].is_period() || tokens[end - 1].is_comma()) {
        end -= 1;
    }

    &tokens[start..end]
}

fn render_trimmed_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn is_exile_it_cost_segment(tokens: &[OwnedLexToken]) -> bool {
    crate::word_primitives::parse_sequence_complete(
        &primitives::TokenWordView::new(tokens).word_refs(),
        &["exile", "it"],
    )
}

fn cost_segment_preserves_source_identity(segment: &ActivationCostSegmentCst) -> bool {
    match segment {
        ActivationCostSegmentCst::PutCounters { .. }
        | ActivationCostSegmentCst::MoveSelfToLibraryBottom { .. }
        | ActivationCostSegmentCst::RemoveCounters { .. }
        | ActivationCostSegmentCst::RemoveCountersDynamic { .. } => true,
        ActivationCostSegmentCst::RemoveCountersAmong { filter, .. } => filter.source,
        _ => false,
    }
}

fn activation_cost_prefix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut quoted = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote { quoted = !quoted; }
        else if token.is_colon() && !quoted { return &tokens[..index]; }
    }
    tokens
}

fn parse_loyalty_shorthand_activation_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<ActivationCostSegmentCst>> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    match parse_loyalty_shorthand_activation_tokens(tokens)? {
        ActivatedLoyaltyShorthand::Add(0) => Some(Vec::new()),
        ActivatedLoyaltyShorthand::Add(count) => {
            Some(vec![ActivationCostSegmentCst::PutCounters {
                counter_type: CounterType::Loyalty,
                count,
            }])
        }
        ActivatedLoyaltyShorthand::Remove(count) => {
            Some(vec![ActivationCostSegmentCst::RemoveCounters {
                counter_type: CounterType::Loyalty,
                count,
            }])
        }
        ActivatedLoyaltyShorthand::RemoveX => {
            Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(CounterType::Loyalty),
                display_x: true,
                remove_all: false,
            }])
        }
    }
}

pub(crate) fn parse_artifact_creature_tap_price(tokens: &[OwnedLexToken]) -> Result<Option<crate::mana::ManaCost>, CardTextError> {
    let rendered = render_token_slice(tokens).to_ascii_lowercase();
    let Some(price) = rendered.strip_prefix("pay ").or_else(|| rendered.strip_prefix("pays ")).or_else(|| rendered.starts_with('{').then_some(rendered.as_str())).and_then(|rest| rest.trim_end_matches('.').strip_suffix(
        " using any combination of mana and tapping untapped artifacts and creatures you control for {1} each")) else { return Ok(None); };
    let mana = crate::grammar::values::parse_mana_cost_tokens(&crate::lexer::lex_line(price, 0)?)?;
    if mana.pips().iter().any(|pip| !matches!(pip.as_slice(), [ManaSymbol::Generic(_) | ManaSymbol::X])) {
        return Err(CardTextError::ParseError("tap-assisted payment defines only generic or X mana".into()));
    }
    Ok(Some(mana.with_artifact_creature_tap_payment()))
}

fn parse_controlled_or_revealed_payment(tokens: &[OwnedLexToken]) -> Option<ActivationCostSegmentCst> {
    let words = crate::lexer::token_word_refs(tokens);
    let (controlled, revealed) = match words.as_slice() {
        ["choose", "a" | "an", controlled, "you", "control", "or", "reveal", "a" | "an", revealed, "card", "from", "your", "hand"] |
        ["reveal", "a" | "an", revealed, "card", "from", "your", "hand", "or", "choose", "a" | "an", controlled, "you", "control"] => (*controlled, *revealed),
        _ => return None,
    };
    let subtype = crate::util::parse_subtype_word(controlled)?;
    (crate::util::parse_subtype_word(revealed) == Some(subtype))
        .then_some(ActivationCostSegmentCst::ChooseControlledOrReveal { subtype })
}

fn parse_characteristic_conversion_cost(tokens: &[OwnedLexToken]) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    if !tokens.first().is_some_and(|t| t.is_word("have")) { return None; }
    use crate::grammar::choices::{parse_typed_choice_object_clause_tokens, TypedChoiceObjectClauseKind};
    let mut quoted = false;
    let mut candidate = None;
    for (index, token) in tokens.iter().enumerate().skip(1) {
        if token.kind == TokenKind::Quote { quoted = !quoted; continue; }
        if quoted || !token.is_word("become") || index + 1 >= tokens.len() { continue; }
        let mut choice = vec![OwnedLexToken::synthetic_word("choose")];
        choice.extend_from_slice(&tokens[1..index]);
        if let Ok(Some(TypedChoiceObjectClauseKind::Object(selected))) = parse_typed_choice_object_clause_tokens(&choice) {
            if selected.count_source.is_none() && selected.filter.zone.is_none_or(|zone| zone == crate::zone::Zone::Battlefield) {
                candidate = Some((index, selected)); break;
            }
        }
    }
    let (conversion_boundary, selected) = candidate?;
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let positions = view.token_start_indices();
    let completion_at = words.windows(3).enumerate().rev().find_map(|(index, words)| {
        (words == ["and", "perform", "the"] && tokens[..positions[index]].iter()
            .filter(|token| token.kind == TokenKind::Quote).count() % 2 == 0).then_some(index)
    });
    let (descriptor_end, completion) = if let Some(index) = completion_at {
        if words.last() != Some(&"action") || index + 4 >= words.len() { return None; }
        let label = words[index + 3..words.len() - 1].join(" ");
        crate::word_claims::claim_tokens(&tokens[positions[index]..]);
        (positions[index], Some(label))
    } else { (tokens.len(), None) };
    crate::word_claims::claim_tokens(&tokens[..=conversion_boundary]);
    Some(Ok(ActivationCostSegmentCst::ConvertObjects {
        count: selected.count, filter: selected.filter,
        descriptor: tokens[conversion_boundary + 1..descriptor_end].to_vec(), completion,
    }))
}

/// A quoted program is one bounded cost, regardless of its commas or clauses.
pub fn parse_quoted_cost_program_tokens(tokens: &[OwnedLexToken]) -> Option<Result<Vec<crate::cards::builders::EffectAst>, CardTextError>> {
    let mut tokens = tokens;
    while tokens.first().is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon)) { tokens = &tokens[1..]; }
    while tokens.last().is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon)) { tokens = &tokens[..tokens.len()-1]; }
    if tokens.len() < 6 || !tokens[..3].iter().zip(["pay", "the", "cost"]).all(|(token, word)| token.is_word(word))
        || tokens[3].kind != TokenKind::Quote || tokens.last()?.kind != TokenKind::Quote { return None; }
    let inner = &tokens[4..tokens.len()-1];
    if inner.iter().any(|token| token.kind == TokenKind::Quote) { return Some(Err(CardTextError::ParseError("a quoted cost program cannot nest quotation marks".into()))); }
    // A disjunction of complete actions is one atomic alternative price.
    // Give its existing choice owner the whole inner program before a broad
    // discard reader can retain only its first action. Child readers still
    // own every operand/word; this does not blanket-claim an unknown tail.
    // Punctuated sequences retain the ordinary bounded program owner; an
    // operand-list comma must not become a new alternative-action boundary.
    let single_instruction = !inner.iter().any(|token| matches!(token.kind,
        TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Colon
        | TokenKind::LBracket | TokenKind::RBracket));
    let choice = if single_instruction {crate::effect_sentences::parse_or_action_clause_lexed(inner)} else {Ok(None)};
    let program = match choice {
        Ok(Some(choice)) => Ok(vec![choice]),
        Ok(None) => crate::effect_sentences::parse_bounded_program(inner),
        Err(error) => Err(error),
    };
    Some(program.and_then(|effects| {
        if effects.is_empty() { return Err(CardTextError::ParseError("a quoted cost program must contain an action".into())); }
        crate::word_claims::claim_tokens(&tokens[..4]);
        crate::word_claims::claim_tokens(&tokens[tokens.len()-1..]);
        Ok(effects)
    }))
}

fn parse_activation_cost_segment_tokens(
    tokens: &[OwnedLexToken],
    named_source: &impl Fn(&[&str]) -> Option<crate::target::SourceReferenceSurface>,
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    if let Some(conversion) = parse_characteristic_conversion_cost(tokens) { return Some(conversion); }
    if let Some(result) = parse_quoted_cost_program_tokens(tokens) { return Some(result.map(ActivationCostSegmentCst::EffectProgram)); }
    if matches!(render_token_slice(tokens).trim().to_ascii_lowercase().as_str(),
        "return an unblocked attacker you control to hand" | "return an unblocked attacker you control to its owner's hand") {
        crate::word_claims::claim_tokens(tokens);
        return Some(Ok(ActivationCostSegmentCst::ReturnUnblockedAttacker));
    }
    match parse_artifact_creature_tap_price(tokens) {
        Ok(Some(mana)) => return Some(Ok(ActivationCostSegmentCst::Mana(mana))),
        Err(error) => return Some(Err(error)),
        Ok(None) => {},
    }
    if render_token_slice(tokens).trim().eq_ignore_ascii_case("Choose to have this creature not untap during your next untap step") {
        return Some(Ok(ActivationCostSegmentCst::ExertSelf { display_text: render_token_slice(tokens).trim().to_owned(), descriptive: true }));
    }
    if let Some(payment) = parse_controlled_or_revealed_payment(tokens) { return Some(Ok(payment)); }
    match parse_activation_cost_segment_kind_tokens(tokens) {
        ActivationCostSegmentKind::Pay => Some(parse_pay_segment_tokens(tokens)),
        ActivationCostSegmentKind::Discard => Some(parse_discard_segment_tokens(tokens)),
        ActivationCostSegmentKind::Mill => Some(parse_mill_segment_tokens(tokens)),
        ActivationCostSegmentKind::Sacrifice => {
            Some(parse_typed_sacrifice_segment_tokens(tokens, named_source))
        }
        ActivationCostSegmentKind::Unattach => {
            Some(parse_unattach_segment_tokens(tokens, |words| {
                is_source_reference_words(words) || named_source(words).is_some()
            }))
        }
        ActivationCostSegmentKind::TapChosen => Some(parse_tap_chosen_segment_tokens(tokens)),
        ActivationCostSegmentKind::UntapChosen => Some(parse_untap_chosen_segment_tokens(tokens)),
        ActivationCostSegmentKind::Behold => Some(parse_behold_segment_tokens(tokens)),
        ActivationCostSegmentKind::Blight => Some(parse_blight_segment_tokens(tokens)),
        ActivationCostSegmentKind::Forage => Some(parse_forage_segment_tokens(tokens)),
        ActivationCostSegmentKind::RollDie => Some(parse_roll_die_segment_tokens(tokens)),
        ActivationCostSegmentKind::CollectEvidence => Some(parse_collect_evidence_segment_tokens(tokens)),
        ActivationCostSegmentKind::Exile => {
            Some(parse_typed_exile_segment_tokens(tokens, |words| {
                is_source_reference_words(words) || named_source(words).is_some()
            }))
        }
        ActivationCostSegmentKind::Reveal => Some(parse_reveal_segment_tokens(tokens)),
        ActivationCostSegmentKind::Return => Some(parse_return_segment_tokens(tokens)),
        ActivationCostSegmentKind::Exert => Some(parse_exert_segment_tokens(tokens)),
        ActivationCostSegmentKind::PutCounter => {
            super::zone_segments::parse_move_chosen_to_graveyard_cost_tokens(tokens)
                .or_else(|| parse_move_source_to_library_bottom_cost_tokens(tokens))
                .or_else(|| parse_move_to_library_top_cost_tokens(tokens))
                .or_else(|| {
                    Some(parse_put_counter_segment_tokens(tokens, &|words| {
                        is_source_reference_words(words) || named_source(words).is_some()
                    }))
                })
        }
        ActivationCostSegmentKind::RemoveCounter => {
            Some(super::counter_segments::parse_remove_counter_segment_tokens_with_source(tokens, &|words| {
                is_source_reference_words(words) || named_source(words).is_some()
            }))
        }
        ActivationCostSegmentKind::BareSymbol => parse_bare_symbol_segment_tokens(tokens).map(Ok),
    }
}

fn parse_source_and_chosen_sacrifice_segment_tokens(
    tokens: &[OwnedLexToken],
    named_source: &impl Fn(&[&str]) -> Option<crate::target::SourceReferenceSurface>,
) -> Option<Vec<ActivationCostSegmentCst>> {
    if !token_slice_first_is(tokens, "sacrifice") {
        return None;
    }

    for conjunction in 1..tokens.len().saturating_sub(1) {
        if !tokens[conjunction].is_word("and") {
            continue;
        }
        let Ok(left) = parse_typed_sacrifice_segment_tokens(&tokens[..conjunction], named_source)
        else {
            continue;
        };

        // The second operand inherits the authored sacrifice verb. Exactly
        // one operand must be the source and the other a chosen set, in either
        // authored order: both "Sacrifice this artifact and two lands" and
        // "Sacrifice two lands and this artifact" are two executable costs,
        // not one union filter.
        let mut inherited = Vec::with_capacity(tokens.len() - conjunction);
        inherited.push(tokens[0].clone());
        inherited.extend_from_slice(&tokens[conjunction + 1..]);
        let Ok(right) = parse_typed_sacrifice_segment_tokens(&inherited, named_source) else {
            continue;
        };

        let is_source = |segment: &ActivationCostSegmentCst| {
            matches!(segment, ActivationCostSegmentCst::SacrificeSelf { .. })
        };
        let is_chosen = |segment: &ActivationCostSegmentCst| {
            matches!(
                segment,
                ActivationCostSegmentCst::SacrificeChosen { .. }
                    | ActivationCostSegmentCst::SacrificeCreature
            )
        };
        if !((is_source(&left) && is_chosen(&right)) || (is_chosen(&left) && is_source(&right))) {
            continue;
        }
        return Some(vec![left, right]);
    }
    None
}

/// "Sacrifice a creature and a Swamp", "Sacrifice a blue creature, a black
/// creature, and a red creature": each indefinite member is its own
/// sacrifice cost (CR 118.3: every listed object must be sacrificed), not one
/// sacrifice of an object matching any member.
fn parse_sacrifice_member_list_segment_tokens(
    tokens: &[OwnedLexToken],
    named_source: &impl Fn(&[&str]) -> Option<crate::target::SourceReferenceSurface>,
) -> Option<Vec<ActivationCostSegmentCst>> {
    if !token_slice_first_is(tokens, "sacrifice") {
        return None;
    }
    let mut members: Vec<&[OwnedLexToken]> = Vec::new();
    let mut start = 1usize;
    let mut idx = 1usize;
    while idx < tokens.len() {
        let token = &tokens[idx];
        if token.is_comma() || token.is_word("and") {
            if idx > start {
                members.push(&tokens[start..idx]);
            }
            start = idx + 1;
        }
        idx += 1;
    }
    if start < tokens.len() {
        members.push(&tokens[start..]);
    }
    // "Sacrifice a creature named Feral Shadow, a creature named
    // Breathstealer, and this creature" (Urborg Panther): the source may be
    // one member beside the indefinite ones.
    let is_indefinite = |member: &&[OwnedLexToken]| {
        member
            .first()
            .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            && member.len() > 1
    };
    let is_source_member = |member: &&[OwnedLexToken]| {
        crate::util::is_source_reference_words(&crate::lexer::token_word_refs(member))
    };
    if members.len() < 2
        || !members
            .iter()
            .all(|member| is_indefinite(member) || is_source_member(member))
        || members.iter().filter(|member| is_source_member(member)).count() > 1
        || !members.iter().any(is_indefinite)
    {
        return None;
    }
    let mut segments = Vec::with_capacity(members.len());
    for member in members {
        let mut inherited = Vec::with_capacity(member.len() + 1);
        inherited.push(tokens[0].clone());
        inherited.extend_from_slice(member);
        let segment = parse_typed_sacrifice_segment_tokens(&inherited, named_source).ok()?;
        match &segment {
            ActivationCostSegmentCst::SacrificeCreature => {}
            ActivationCostSegmentCst::SacrificeChosen { count, .. }
                if *count == crate::effect::ChoiceCount::exactly(1) => {}
            ActivationCostSegmentCst::SacrificeSelf { .. } => {}
            _ => return None,
        }
        segments.push(segment);
    }
    Some(segments)
}

fn named_source_reference_surface_for_words(
    words: &[&str],
) -> Option<crate::target::SourceReferenceSurface> {
    match source_reference_surface_for_words(words)? {
        surface @ (crate::target::SourceReferenceSurface::FullName(_)
        | crate::target::SourceReferenceSurface::ShortName(_)) => Some(surface),
        crate::target::SourceReferenceSurface::ThisPermanentType(_) => None,
    }
}

fn starts_new_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    parse_activation_cost_segment_head_lexed
        .parse_next(&mut input)
        .is_ok()
}

fn parse_activation_cost_segment_head_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)?;
    alt((
        alt((
            primitives::token_kind(TokenKind::ManaGroup),
            primitives::token_kind(TokenKind::Number),
            primitives::token_kind(TokenKind::Plus),
            primitives::token_kind(TokenKind::Dash),
        ))
        .void(),
        alt((
            alt((
                alt((primitives::kw("collect"), primitives::kw("tap"))),
                alt((primitives::kw("t"), primitives::kw("have"))),
                primitives::kw("untap"),
                primitives::kw("q"),
                primitives::kw("pay"),
                primitives::kw("discard"),
                primitives::kw("mill"),
                primitives::kw("sacrifice"),
                primitives::kw("unattach"),
            ))
            .void(),
            alt((
                alt((
                    primitives::kw("exile"),
                    primitives::kw("return"),
                    primitives::kw("put"),
                    primitives::kw("remove"),
                    primitives::kw("behold"),
                    primitives::kw("blight"),
                    primitives::kw("forage"),
                    // "{6}, Roll a d8: ..." (Clay Golem).
                    primitives::kw("roll"),
                ))
                .void(),
                alt((
                    primitives::kw("exert"),
                    primitives::kw("choose"),
                    primitives::kw("reveal"),
                    primitives::kw("waterbend"),
                    primitives::kw("e"),
                    primitives::kw("and"),
                ))
                .void(),
            ))
            .void(),
        ))
        .void(),
    ))
    .parse_next(input)
}

fn split_activation_cost_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_named_card = false;
    let mut quoted = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens[idx].kind == TokenKind::Quote { quoted = !quoted; }
        if quoted { idx += 1; continue; }
        if !inside_named_card
            && tokens[idx].is_word("card")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("named"))
        {
            inside_named_card = true;
        }

        let split_here = if tokens[idx].is_comma() {
            let remainder = &tokens[idx + 1..];
            let remainder = if token_slice_first_is(remainder, "and") {
                &remainder[1..]
            } else {
                remainder
            };
            starts_new_activation_cost_segment_tokens(remainder)
        } else if tokens[idx].is_word("and") && idx > start {
            let remainder = &tokens[idx + 1..];
            !inside_named_card && starts_new_activation_cost_segment_tokens(remainder)
        } else {
            false
        };

        if split_here {
            let segment = tokens[start..idx].to_vec();
            if !segment.is_empty() {
                segments.push(segment);
            }
            start = idx + 1;
            inside_named_card = false;
        }

        idx += 1;
    }

    let tail = tokens[start..].to_vec();
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

/// Rebuild the two sides of an "X or Y" activation cost so each branch is a
/// complete payment.
///
/// - "{1}{R}, Remove a +1/+1 counter or a charge counter from a permanent you
///   control" (Ion Storm) elides the shared verb and source of the counter
///   removal; each branch names one counter type removed from that source.
/// - A comma list binds tighter than the final "or": in "{1}, Sacrifice a
///   creature or discard a card" the "{1}" is paid with either alternative,
///   so the leading segments are shared by the right branch. A right branch
///   that opens with its own mana or tap symbol ("{3}, {T} or {U}, {T}") is a
///   complete alternative and shares nothing.
fn distribute_activation_cost_alternative(
    left: &[OwnedLexToken],
    right: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, Vec<OwnedLexToken>) {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    let last_comma = left.iter().rposition(OwnedLexToken::is_comma);
    let segment_start = last_comma.map_or(0, |index| index + 1);
    let left_segment = &left[segment_start..];
    let counter_noun = |token: &OwnedLexToken| token.is_any_word(&["counter", "counters"]);
    if left_segment.first().is_some_and(|token| token.is_word("remove"))
        && left_segment.last().is_some_and(counter_noun)
        && !left_segment.iter().any(|token| token.is_word("from"))
        && right.first().is_some_and(|token| token.is_any_word(&["a", "an"]))
        && let Some(from_index) = right.iter().position(|token| token.is_word("from"))
        && from_index >= 2
        && counter_noun(&right[from_index - 1])
    {
        let remove = left_segment[0].clone();
        let source_tail = right[from_index..].to_vec();
        left.extend(source_tail);
        right.insert(0, remove);
    }
    if let Some(comma) = last_comma
        && right
            .first()
            .is_some_and(|token| token.kind != TokenKind::ManaGroup)
    {
        let mut shared = left[..=comma].to_vec();
        shared.extend(right);
        right = shared;
    }
    (left, right)
}

fn parse_activation_cost_cst_tokens(
    tokens: &[OwnedLexToken],
    raw: &str,
    named_source: &impl Fn(&[&str]) -> Option<crate::target::SourceReferenceSurface>,
) -> Result<ActivationCostCst, CardTextError> {
    let trimmed_raw = raw.trim();
    if let Some(program)=parse_quoted_cost_program_tokens(tokens){
        return Ok(ActivationCostCst{raw:trimmed_raw.to_owned(),segments:vec![ActivationCostSegmentCst::EffectProgram(program?)],alternative_branches:Vec::new(),is_loyalty_shorthand:false,waterbend_cost:None});
    }

    if let Some(mana) = parse_artifact_creature_tap_price(tokens)? {
        return Ok(ActivationCostCst { raw: trimmed_raw.to_owned(), segments: vec![ActivationCostSegmentCst::Mana(mana)],
            alternative_branches: Vec::new(), is_loyalty_shorthand: false, waterbend_cost: None });
    }
    if let Some(segments) = parse_loyalty_shorthand_activation_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments,
            alternative_branches: Vec::new(),
            is_loyalty_shorthand: true,
            waterbend_cost: None,
        });
    }

    // The "or" in this instruction selects an object/visibility route; it is
    // one payment with one captured result, not two independently paid costs.
    if let Some(payment) = parse_controlled_or_revealed_payment(tokens) {
        return Ok(ActivationCostCst { raw: trimmed_raw.to_owned(), segments: vec![payment],
            alternative_branches: Vec::new(), is_loyalty_shorthand: false, waterbend_cost: None });
    }
    if let Some(split) = parse_payment_alternative_split_tokens(tokens) {
        let (left_owned, right_owned) = distribute_activation_cost_alternative(
            trim_activation_cost_segment_tokens(&tokens[..split.delimiter]),
            trim_activation_cost_segment_tokens(&tokens[split.delimiter + 1..]),
        );
        let left_tokens = left_owned.as_slice();
        let right_tokens = right_owned.as_slice();
        if !left_tokens.is_empty() && !right_tokens.is_empty() {
            let left_raw = render_trimmed_lexed_tokens(left_tokens);
            let right_raw = render_trimmed_lexed_tokens(right_tokens);
            if let (Ok(left), Ok(right)) = (
                parse_activation_cost_cst_tokens(left_tokens, &left_raw, named_source),
                parse_activation_cost_cst_tokens(right_tokens, &right_raw, named_source),
            ) {
                return Ok(ActivationCostCst {
                    raw: trimmed_raw.to_string(),
                    segments: Vec::new(),
                    alternative_branches: vec![left, right],
                    is_loyalty_shorthand: false,
                    waterbend_cost: None,
                });
            }
        }
    }

    let mut segments = Vec::new();
    for segment_tokens in split_activation_cost_segments_tokens(tokens) {
        let waterbend = first_non_comma_token_index(&segment_tokens)
            .is_some_and(|start| token_slice_at_is(&segment_tokens, start, "waterbend"));
        let segment_tokens = trim_activation_cost_segment_tokens(&segment_tokens);
        if segment_tokens.is_empty() {
            continue;
        }

        // The member list reads every comma member first, so a later "and
        // this creature" never folds earlier members into one name.
        if let Some(compound) =
            parse_sacrifice_member_list_segment_tokens(segment_tokens, named_source)
                .or_else(|| parse_source_and_chosen_sacrifice_segment_tokens(segment_tokens, named_source))
        {
            segments.extend(compound);
            continue;
        }

        let segment = render_trimmed_lexed_tokens(segment_tokens);
        let mut parsed = if is_exile_it_cost_segment(segment_tokens)
            && segments
                .last()
                .is_some_and(cost_segment_preserves_source_identity)
        {
            Ok(ActivationCostSegmentCst::ExileSelf)
        } else {
            parse_activation_cost_segment_tokens(segment_tokens, named_source).unwrap_or_else(
                || {
                    Err(CardTextError::ParseError(format!(
                        "rewrite activation-cost segment parser does not yet support '{segment}'",
                    )))
                },
            )
        }
        .map_err(|err| {
            CardTextError::ParseError(format!(
                "unsupported activation cost segment (clause: '{}'): {err}",
                segment,
            ))
        })?;
        if waterbend {
            let ActivationCostSegmentCst::Mana(cost) = &mut parsed else {
                return Err(CardTextError::ParseError("waterbend requires a generic or X cost".into()));
            };
            if cost.pips().iter().any(|pip| !matches!(pip.as_slice(), [ManaSymbol::Generic(_) | ManaSymbol::X])) {
                return Err(CardTextError::ParseError("waterbend requires a generic or X cost".into()));
            }
            *cost = cost.clone().with_waterbend();
        }
        segments.push(parsed);
    }

    if segments.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activation-cost parser found no segments".to_string(),
        ));
    }

    let waterbend_cost = segments.iter().find_map(|segment| match segment {
        ActivationCostSegmentCst::Mana(cost) if cost.has_waterbend_obligation() => Some(cost.clone()),
        _ => None,
    });

    Ok(ActivationCostCst {
        raw: trimmed_raw.to_string(),
        segments,
        alternative_branches: Vec::new(),
        is_loyalty_shorthand: false,
        waterbend_cost,
    })
}

pub fn parse_activation_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_cst_tokens(
        tokens,
        &render_token_slice(tokens),
        &named_source_reference_surface_for_words,
    )
}

pub fn parse_activation_cost_tokens_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    let named_source =
        |words: &[&str]| match crate::util::source_reference_surface_for_words_with_context(
            context, words,
        )? {
            surface @ (crate::target::SourceReferenceSurface::FullName(_)
            | crate::target::SourceReferenceSurface::ShortName(_)) => Some(surface),
            crate::target::SourceReferenceSurface::ThisPermanentType(_) => None,
        };
    parse_activation_cost_cst_tokens(tokens, &render_token_slice(tokens), &named_source)
}

#[cfg(test)]
pub fn parse_activation_cost_tokens_rewrite(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_tokens(tokens)
}

#[cfg(test)]
pub fn parse_activation_cost_rewrite(raw: &str) -> Result<ActivationCostCst, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_activation_cost_cst_tokens(&tokens, raw, &named_source_reference_surface_for_words)
}

#[cfg(test)]
#[path = "program_inline_tests.rs"]
mod tests;

#[cfg(test)]
mod quoted_alternative_cost_program_tests {
    use super::*;
    #[test]
    fn explicit_quoted_price_keeps_both_actions_and_excluded_antecedent() {
        for reference in ["that creature", "this creature", "it"] {
            let text=format!("Pay the cost \"Discard a nonland card or sacrifice a creature other than {reference}\".");
            let tokens=crate::lexer::lex_line(&text,0).unwrap();
            let effects=parse_quoted_cost_program_tokens(&tokens).unwrap().unwrap();
            let [crate::cards::builders::EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf{modes,..})]=effects.as_slice() else{panic!("whole alternative cost must remain a choice: {effects:?}")};
            assert_eq!(modes.len(),2);assert!(!modes[0].effects.is_empty());assert!(!modes[1].effects.is_empty());
            let second=format!("{:?}",modes[1].effects);assert!(second.contains("Sacrifice"),"{second}");
        }
    }
}
