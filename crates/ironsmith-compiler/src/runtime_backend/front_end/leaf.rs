#![allow(dead_code)]

use crate::cards::builders::{CardTextError, ChoiceCount};
use crate::color::Color;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::effect_sentences::parse_subtype_word;
use super::grammar::primitives::TokenWordView;
use super::grammar::values::{parse_count_word_tokens, parse_mana_cost_tokens};
use super::lexer::{
    OwnedLexToken, TokenKind, lex_line, render_token_slice, token_slice_at_is,
    token_slice_first_is, word_slice_eq, word_slice_eq_any, word_slice_find_window_by,
    word_slice_find_word, word_slice_first_is, word_slice_last_is_any, words_end_with,
    words_start_with,
};
use super::token_primitives::locate_index as locate_token_index;
use super::util::{is_source_reference_words, parse_number};

pub(crate) use super::grammar::activation_costs::{ActivationCostCst, ActivationCostSegmentCst};
use super::grammar::activation_costs::{
    ActivationCostSegmentKind, is_tap_activation_symbol_token,
    parse_activation_cost_segment_kind_tokens, parse_activation_cost_word_suffix,
    parse_bare_symbol_segment_tokens, parse_behold_segment_tokens, parse_blight_segment_tokens,
    parse_discard_segment_tokens, parse_exile_segment_tokens as parse_typed_exile_segment_tokens,
    parse_mill_segment_tokens, parse_optional_activation_counter_type_tokens,
    parse_pay_segment_tokens, parse_put_counter_segment_tokens,
    parse_remove_counter_segment_tokens, parse_return_segment_tokens, parse_reveal_segment_tokens,
    parse_sacrifice_segment_tokens as parse_typed_sacrifice_segment_tokens,
    parse_tap_chosen_segment_tokens, parse_unattach_segment_tokens,
};
use super::grammar::keyword_action_costs::parse_payment_alternative_split_tokens;

type LeafCompatWords<'a> = TokenWordView<'a>;

const LEAF_X_WORD: &str = "x";
const LEAF_ZERO_WORD: &str = "0";
const LEAF_EXERT_WORD: &str = "exert";

fn apply_activation_cost_default_battlefield_scope(filter: &mut ObjectFilter) {
    if !filter.any_of.is_empty() {
        for arm in &mut filter.any_of {
            apply_activation_cost_default_battlefield_scope(arm);
        }
        return;
    }
    if filter.controller.is_none() && filter.owner.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
}

fn first_non_comma_token(tokens: &[OwnedLexToken]) -> Option<&OwnedLexToken> {
    for token in tokens {
        if !token.is_comma() {
            return Some(token);
        }
    }
    None
}

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

fn render_lower_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_trimmed_lexed_tokens(tokens).to_ascii_lowercase()
}

fn activation_cost_prefix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if let Some(colon_idx) = locate_token_index(tokens, OwnedLexToken::is_colon) {
        &tokens[..colon_idx]
    } else {
        tokens
    }
}

fn parse_loyalty_shorthand_activation_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<ActivationCostSegmentCst>> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    let tokens = trim_bracketed_loyalty_cost_tokens(tokens);
    let parse_single = |text: &str| {
        let bytes = text.as_bytes();
        if let Some((&sign, rest)) = bytes.split_first() {
            let rest = std::str::from_utf8(rest).ok()?;
            if sign == b'+'
                && let Ok(amount) = rest.parse::<u32>()
            {
                return Some(if amount == 0 {
                    Vec::new()
                } else {
                    vec![ActivationCostSegmentCst::PutCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]
                });
            }

            if sign == b'-' {
                if rest == LEAF_X_WORD {
                    return Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                        counter_type: Some(CounterType::Loyalty),
                        display_x: true,
                        remove_all: false,
                    }]);
                }
                if let Ok(amount) = rest.parse::<u32>() {
                    return Some(vec![ActivationCostSegmentCst::RemoveCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]);
                }
            }
        }

        (text == LEAF_ZERO_WORD).then(Vec::new)
    };

    match tokens {
        [token] => parse_single(token.parser_text()),
        [sign, value] if sign.kind == TokenKind::Plus => {
            value.parser_text().parse::<u32>().ok().map(|amount| {
                if amount == 0 {
                    Vec::new()
                } else {
                    vec![ActivationCostSegmentCst::PutCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]
                }
            })
        }
        [sign, value] if sign.kind == TokenKind::Dash => {
            let value = value.parser_text();
            if value == LEAF_X_WORD {
                Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                    counter_type: Some(CounterType::Loyalty),
                    display_x: true,
                    remove_all: false,
                }])
            } else {
                value.parse::<u32>().ok().map(|amount| {
                    vec![ActivationCostSegmentCst::RemoveCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]
                })
            }
        }
        _ => None,
    }
}

fn trim_bracketed_loyalty_cost_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    if start < end && tokens[start].kind == TokenKind::LBracket {
        start += 1;
    }
    if end > start && tokens[end - 1].kind == TokenKind::RBracket {
        end -= 1;
    }
    &tokens[start..end]
}

fn parse_activation_cost_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    match parse_activation_cost_segment_kind_tokens(tokens) {
        ActivationCostSegmentKind::Pay => Some(parse_pay_segment_tokens(tokens)),
        ActivationCostSegmentKind::Discard => Some(parse_discard_segment_tokens(tokens)),
        ActivationCostSegmentKind::Mill => Some(parse_mill_segment_tokens(tokens)),
        ActivationCostSegmentKind::Sacrifice => Some(parse_typed_sacrifice_segment_tokens(
            tokens,
            is_source_reference_words,
        )),
        ActivationCostSegmentKind::Unattach => Some(parse_unattach_segment_tokens(
            tokens,
            is_source_reference_words,
        )),
        ActivationCostSegmentKind::TapChosen => Some(parse_tap_chosen_segment_tokens(tokens)),
        ActivationCostSegmentKind::Behold => Some(parse_behold_segment_tokens(tokens)),
        ActivationCostSegmentKind::Blight => Some(parse_blight_segment_tokens(tokens)),
        ActivationCostSegmentKind::Exile => Some(parse_typed_exile_segment_tokens(
            tokens,
            is_source_reference_words,
        )),
        ActivationCostSegmentKind::Reveal => Some(parse_reveal_segment_tokens(tokens)),
        ActivationCostSegmentKind::Return => Some(parse_return_segment_tokens(tokens)),
        ActivationCostSegmentKind::Exert => Some(parse_exert_segment_tokens(tokens)),
        ActivationCostSegmentKind::PutCounter => Some(parse_put_counter_segment_tokens(tokens)),
        ActivationCostSegmentKind::RemoveCounter => {
            Some(parse_remove_counter_segment_tokens(tokens))
        }
        ActivationCostSegmentKind::BareSymbol => parse_bare_symbol_segment_tokens(tokens).map(Ok),
    }
}

fn parse_exert_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_trimmed_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if !word_slice_first_is(&lowered, LEAF_EXERT_WORD) {
        return Err(CardTextError::ParseError(
            "rewrite exert-cost parser expected leading 'exert'".to_string(),
        ));
    }
    let missing_object = match parse_activation_cost_word_suffix(tokens, 1) {
        None => true,
        Some(rest) => LeafCompatWords::new(rest.tokens).is_empty(),
    };
    if missing_object {
        return Err(CardTextError::ParseError(format!(
            "rewrite exert-cost parser missing exerted object in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ExertSelf { display_text: raw })
}

fn parse_shard_style_branch_tokens(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
    let tokens = trim_activation_cost_segment_tokens(tokens);
    let comma_idx = locate_token_index(tokens, OwnedLexToken::is_comma)?;
    let mana_tokens = trim_activation_cost_segment_tokens(&tokens[..comma_idx]);
    let tap_tokens = trim_activation_cost_segment_tokens(&tokens[comma_idx + 1..]);
    if tap_tokens.len() != 1 || tap_tokens[0].kind != TokenKind::ManaGroup {
        return None;
    }
    if !is_tap_activation_symbol_token(&tap_tokens[0]) {
        return None;
    }

    let mana_cost = parse_mana_cost_tokens(mana_tokens).ok()?;
    let [pip] = mana_cost.pips() else {
        return None;
    };
    let [symbol] = pip.as_slice() else {
        return None;
    };
    Some(*symbol)
}

fn parse_shard_style_mana_or_tap_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ManaSymbol, ManaSymbol)> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    let or_idx = locate_token_index(tokens, |token| token.is_word("or"))?;
    let left = parse_shard_style_branch_tokens(&tokens[..or_idx])?;
    let right = parse_shard_style_branch_tokens(&tokens[or_idx + 1..])?;
    Some((left, right))
}

fn starts_new_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(first) = first_non_comma_token(tokens) else {
        return false;
    };

    match first.kind {
        TokenKind::ManaGroup | TokenKind::Number | TokenKind::Plus | TokenKind::Dash => true,
        TokenKind::Word => matches!(
            first.slice.to_ascii_lowercase().as_str(),
            "tap"
                | "t"
                | "untap"
                | "q"
                | "pay"
                | "discard"
                | "mill"
                | "sacrifice"
                | "unattach"
                | "exile"
                | "return"
                | "put"
                | "remove"
                | "behold"
                | "exert"
                | "reveal"
                | "waterbend"
                | "e"
                | "and"
                | "0"
        ),
        _ => false,
    }
}

fn split_activation_cost_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_named_card = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
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

fn parse_activation_cost_cst_tokens(
    tokens: &[OwnedLexToken],
    raw: &str,
) -> Result<ActivationCostCst, CardTextError> {
    let trimmed_raw = raw.trim();
    if let Some(segments) = parse_loyalty_shorthand_activation_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments,
            alternative_branches: Vec::new(),
            is_loyalty_shorthand: true,
        });
    }

    if let Some((left, right)) = parse_shard_style_mana_or_tap_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments: vec![
                ActivationCostSegmentCst::Mana(ManaCost::from_pips(vec![vec![left, right]])),
                ActivationCostSegmentCst::Tap,
            ],
            alternative_branches: Vec::new(),
            is_loyalty_shorthand: false,
        });
    }

    if let Some(split) = parse_payment_alternative_split_tokens(tokens) {
        let left_tokens = trim_activation_cost_segment_tokens(&tokens[..split.delimiter]);
        let right_tokens = trim_activation_cost_segment_tokens(&tokens[split.delimiter + 1..]);
        if !left_tokens.is_empty() && !right_tokens.is_empty() {
            let left_raw = render_trimmed_lexed_tokens(left_tokens);
            let right_raw = render_trimmed_lexed_tokens(right_tokens);
            if let (Ok(left), Ok(right)) = (
                parse_activation_cost_cst_tokens(left_tokens, &left_raw),
                parse_activation_cost_cst_tokens(right_tokens, &right_raw),
            ) {
                return Ok(ActivationCostCst {
                    raw: trimmed_raw.to_string(),
                    segments: Vec::new(),
                    alternative_branches: vec![left, right],
                    is_loyalty_shorthand: false,
                });
            }
        }
    }

    let mut segments = Vec::new();
    for segment_tokens in split_activation_cost_segments_tokens(tokens) {
        let segment_tokens = trim_activation_cost_segment_tokens(&segment_tokens);
        if segment_tokens.is_empty() {
            continue;
        }

        let segment = render_trimmed_lexed_tokens(segment_tokens);
        let parsed = parse_activation_cost_segment_tokens(segment_tokens)
            .unwrap_or_else(|| {
                Err(CardTextError::ParseError(format!(
                    "rewrite activation-cost segment parser does not yet support '{segment}'",
                )))
            })
            .map_err(|err| {
                CardTextError::ParseError(format!(
                    "unsupported activation cost segment (clause: '{}'): {err}",
                    segment,
                ))
            })?;
        segments.push(parsed);
    }

    if segments.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activation-cost parser found no segments".to_string(),
        ));
    }

    Ok(ActivationCostCst {
        raw: trimmed_raw.to_string(),
        segments,
        alternative_branches: Vec::new(),
        is_loyalty_shorthand: false,
    })
}

pub(crate) fn parse_activation_cost_tokens_rewrite(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_cst_tokens(tokens, &render_token_slice(tokens))
}

pub(crate) fn parse_activation_cost_rewrite(raw: &str) -> Result<ActivationCostCst, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_activation_cost_cst_tokens(&tokens, raw)
}

pub(crate) fn lower_activation_cost_cst(
    cst: &ActivationCostCst,
) -> Result<TotalCost, CardTextError> {
    if !cst.alternative_branches.is_empty() {
        let mut modes = Vec::with_capacity(cst.alternative_branches.len());
        for branch in &cst.alternative_branches {
            let total = lower_activation_cost_cst(branch)?;
            modes.push(crate::effect::EffectMode::new(
                branch.raw.clone(),
                crate::costs::total_cost_to_payment_effects(&total),
            ));
        }
        return Ok(TotalCost::from_cost(Cost::validated_effect(
            Effect::choose_one(modes),
        )));
    }

    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut Vec<Vec<ManaSymbol>>) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(ManaCost::from_pips(std::mem::take(pending))));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = Vec::new();
    let mut tap_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    for segment in &cst.segments {
        match segment {
            ActivationCostSegmentCst::Mana(cost) => {
                pending_mana_pips.extend(cost.pips().to_vec());
            }
            ActivationCostSegmentCst::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            ActivationCostSegmentCst::TapChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                filter.untapped = true;
                let tag = format!("tap_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::Untap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::untap());
            }
            ActivationCostSegmentCst::Life(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if matches!(amount, crate::effect::Value::Fixed(_)) {
                    costs.push(Cost::life(amount.clone()));
                } else {
                    costs.push(Cost::validated_effect(Effect::lose_life_player(
                        amount.clone(),
                        PlayerFilter::You,
                    )));
                }
            }
            ActivationCostSegmentCst::Energy(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::energy(*amount));
            }
            ActivationCostSegmentCst::DiscardSource => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_source());
            }
            ActivationCostSegmentCst::DiscardHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_hand());
            }
            ActivationCostSegmentCst::DiscardCard(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard(*count, None));
            }
            ActivationCostSegmentCst::DiscardFiltered {
                count,
                card_types,
                supertypes,
                filter,
                random,
                name,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *random || name.is_some() || *other || filter.is_some() || !supertypes.is_empty()
                {
                    let card_filter = if let Some(filter) = filter {
                        Some(filter.clone())
                    } else if card_types.is_empty()
                        && supertypes.is_empty()
                        && name.is_none()
                        && !*other
                    {
                        None
                    } else {
                        let mut filter = ObjectFilter {
                            zone: Some(crate::zone::Zone::Hand),
                            card_types: card_types.clone(),
                            supertypes: supertypes.clone(),
                            ..Default::default()
                        };
                        if let Some(name) = name {
                            filter = filter.named(name.clone());
                        }
                        if *other {
                            filter.other = true;
                        }
                        Some(filter)
                    };
                    costs.push(Cost::validated_effect(Effect::discard_player_filtered(
                        *count as i32,
                        PlayerFilter::You,
                        *random,
                        card_filter,
                    )));
                } else if card_types.len() > 1 {
                    costs.push(Cost::discard_types(*count, card_types.clone()));
                } else if let Some(card_type) = card_types.first().copied() {
                    costs.push(Cost::discard(*count, Some(card_type)));
                } else {
                    costs.push(Cost::discard(*count, None));
                }
            }
            ActivationCostSegmentCst::Mill(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::mill(*count));
            }
            ActivationCostSegmentCst::Behold { subtype, count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::behold(*subtype, *count)));
            }
            ActivationCostSegmentCst::Blight { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("blight_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    CounterType::MinusOneMinusOne,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::SacrificeSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::sacrifice_self());
            }
            ActivationCostSegmentCst::SacrificeCreature => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    1,
                )));
            }
            ActivationCostSegmentCst::SacrificeChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    count.clone(),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                let exact_count =
                    (!count.dynamic_x && count.max == Some(count.min)).then_some(count.min as u32);
                let sacrifice = if let Some(exact_count) = exact_count {
                    Effect::sacrifice(ObjectFilter::tagged(tag), exact_count)
                } else {
                    Effect::sacrifice_player(
                        ObjectFilter::tagged(tag.clone()),
                        crate::effect::Value::Count(ObjectFilter::tagged(tag)),
                        PlayerFilter::You,
                    )
                };
                costs.push(Cost::validated_effect(sacrifice));
            }
            ActivationCostSegmentCst::UnattachChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("unattach_cost_{return_tag_id}");
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::unattach_objects(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileSelfFromGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileFromHand {
                count,
                color_filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_from_hand(*count, *color_filter));
            }
            ActivationCostSegmentCst::ExileFromGraveyard { count, card_type } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = ObjectFilter::default()
                    .owned_by(PlayerFilter::You)
                    .in_zone(crate::zone::Zone::Graveyard);
                if let Some(card_type) = card_type {
                    filter = filter.with_type(*card_type);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.zone == Some(crate::zone::Zone::Battlefield)
                    && filter.controller.is_none()
                {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileSelfAndNamedArtifacts { names } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
                for name in names {
                    let tag = format!("exile_cost_{exile_tag_id}");
                    exile_tag_id += 1;
                    let mut filter = ObjectFilter {
                        zone: Some(crate::zone::Zone::Battlefield),
                        controller: Some(PlayerFilter::You),
                        card_types: vec![CardType::Artifact],
                        ..Default::default()
                    };
                    filter.name = Some(name.clone());
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        ChoiceCount::exactly(1),
                        PlayerFilter::You,
                        tag.clone(),
                    )));
                    costs.push(Cost::validated_effect(Effect::exile(
                        crate::target::ChooseSpec::tagged(tag),
                    )));
                }
            }
            ActivationCostSegmentCst::ExileTopLibrary { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                #[cfg(not(feature = "serialization"))]
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                    crate::tag::TagKey::from("__cost_exiled_top__"),
                    None,
                )));
                #[cfg(feature = "serialization")]
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                )));
            }
            ActivationCostSegmentCst::RevealSourceFromHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_source_from_hand()));
            }
            ActivationCostSegmentCst::RevealFromHand {
                count,
                color_filter,
                card_type,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_from_hand(
                    count.clone(),
                    *card_type,
                    *color_filter,
                )));
            }
            ActivationCostSegmentCst::ReturnSelfToHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::return_self_to_hand());
            }
            ActivationCostSegmentCst::ReturnChosenToHand { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("return_cost_{return_tag_id}");
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::return_to_hand(
                    ObjectFilter::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("graveyard_cost_{return_tag_id}");
                return_tag_id += 1;
                let filter = ObjectFilter {
                    zone: Some(crate::zone::Zone::Exile),
                    owner: Some(PlayerFilter::Opponent),
                    ..Default::default()
                };
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Graveyard,
                    false,
                )));
            }
            ActivationCostSegmentCst::ExertSelf { display_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(crate::effects::ExertCostEffect::new(
                    display_text.clone(),
                )));
            }
            ActivationCostSegmentCst::PutCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::add_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::PutCountersChosen {
                counter_type,
                count,
                filter,
                source_equivalent,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *source_equivalent {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                if filter.source {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let tag = format!("put_counter_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    *counter_type,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::RemoveCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count,
                filter,
                display_x,
                dynamic,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                let effect = if *dynamic {
                    Effect::remove_dynamic_counters_among(
                        *count,
                        u32::MAX / 4,
                        filter,
                        *counter_type,
                        *display_x,
                    )
                } else {
                    Effect::remove_any_counters_among(*count, filter, *counter_type)
                };
                costs.push(Cost::validated_effect(effect));
            }
            ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x,
                remove_all,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let cost = if *remove_all {
                    Cost::remove_all_counters_from_source(*counter_type)
                } else {
                    Cost::remove_any_counters_from_source(*counter_type, *display_x)
                };
                costs.push(cost);
            }
        }
    }
    flush_pending_mana(&mut costs, &mut pending_mana_pips);
    Ok(TotalCost::from_costs(costs))
}
