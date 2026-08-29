use crate::cards::builders::{
    ActivationTiming, CardTextError, ConditionalModeSelection, EffectAst, EffectPredicate,
    IfResultPredicate, LineInfo, ParsedConditionalModeChange, ParsedModalActivatedHeader,
    ParsedModalGate, ParsedModalHeader, SubjectVerbActionAst,
};
use crate::effect::Value;
use crate::target::PlayerFilter;
use ironsmith_core::ValueSurfaceHint;

use super::activation_and_restrictions::activated_line_core::{
    infer_activated_functional_zones_lexed, parse_activate_only_timing_lexed,
};
use super::clause_support::{parse_effect_sentences_lexed, parse_trigger_clause_lexed};
use super::effect_ast_traversal::try_for_each_nested_effects_mut;
use super::grammar::abilities::parse_activation_condition_lexed;
use super::grammar::activation_costs::parse_activation_cost_tokens;
use super::grammar::primitives as grammar;
use super::grammar::structure::{
    ModalHeaderChooseSpec, parse_modal_header_choose_spec, scan_modal_header_flags,
    split_lexed_sentences, split_trailing_modal_gate_clause,
};
use super::keyword_static::parse_value_binding_clause_lexed;
use super::lexer::{
    OwnedLexToken, TokenKind, contains_token_word, locate_token_word_choice, render_token_slice,
    trim_lexed_commas,
};
use super::modal_helpers::{replace_unbound_x_with_value, value_contains_unbound_x};
use super::semantic_assembly::assemble_activation_cost;

type ModalHeader = ParsedModalHeader;
type ModalActivatedHeader = ParsedModalActivatedHeader;
type ModalGate = ParsedModalGate;

fn locate_token_index(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if predicate(&tokens[idx]) {
            return Some(idx);
        }
        idx += 1;
    }

    None
}

fn strip_leading_sign(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes
        .first()
        .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
    {
        return text.get(1..);
    }

    None
}

/// Parse a return action authored once between a modal choice instruction and
/// its bullet list. Restrict this to demonstrative object follow-ups so
/// ordinary modal metadata sentences (for example, repeated-mode permission)
/// and not-yet-specialized common actions remain header text.
fn parse_modal_common_suffix_effects(
    tokens: &[OwnedLexToken],
    choose_idx: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let Some(sentence_end) = tokens
        .iter()
        .enumerate()
        .skip(choose_idx)
        .find_map(|(index, token)| token.is_period().then_some(index))
    else {
        return Ok(Vec::new());
    };

    let mut effects = Vec::new();
    for sentence in split_lexed_sentences(&tokens[sentence_end + 1..]) {
        let return_action = sentence
            .first()
            .is_some_and(|token| token.is_word("return"));
        let demonstrative_object = sentence.iter().any(|token| {
            ["it", "them", "that", "those"]
                .iter()
                .any(|word| token.is_word(word))
        });
        if return_action && demonstrative_object {
            effects.extend(parse_effect_sentences_lexed(sentence)?);
        }
    }
    Ok(effects)
}

/// Parse effects authored once after the modal instruction itself, as in
/// "choose one and this creature gets +1/+1 until end of turn." These effects
/// are neither pre-choice setup nor part of any individual bullet.
fn parse_modal_common_prefix_effects(
    tokens: &[OwnedLexToken],
    choose_idx: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let Some(sentence_end) =
        tokens
            .iter()
            .enumerate()
            .skip(choose_idx)
            .find_map(|(index, token)| {
                (token.is_period() || matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
                    .then_some(index)
            })
    else {
        return Ok(Vec::new());
    };
    let Some(and_idx) = crate::slice_primitives::select_last_position(
        &tokens[choose_idx + 1..sentence_end],
        |token| token.is_word("and"),
    )
    .map(|offset| choose_idx + 1 + offset) else {
        return Ok(Vec::new());
    };
    let effect_tokens = trim_lexed_commas(&tokens[and_idx + 1..sentence_end]);
    if effect_tokens.is_empty()
        || effect_tokens
            .first()
            .is_some_and(|token| token.is_word("or"))
    {
        return Ok(Vec::new());
    }
    parse_effect_sentences_lexed(effect_tokens)
}

pub fn parse_modal_header(
    info: &LineInfo,
    tokens: &[OwnedLexToken],
) -> Result<Option<ModalHeader>, CardTextError> {
    let spree = tokens.first().is_some_and(|token| token.is_word("spree"));
    let tiered = tokens.first().is_some_and(|token| token.is_word("tiered"));
    let choose_spec = if spree || tiered {
        ModalHeaderChooseSpec {
            choose_idx: 0,
            min: Value::Fixed(1),
            max: tiered.then_some(Value::Fixed(1)),
            random: false,
            x_clause_start: None,
        }
    } else {
        let Some(choose_spec) = grammar::parse_all_with_display_line(
            tokens,
            parse_modal_header_choose_spec,
            "modal-header",
            info.display_line_index,
        )?
        else {
            return Ok(None);
        };
        choose_spec
    };
    let modal_flags = scan_modal_header_flags(tokens);
    let conditional_mode_change = parse_conditional_mode_change(tokens, choose_spec.choose_idx)?;
    let presentation_label = parse_modal_presentation_label(&info.source_tokens);
    let presentation_prefix_end = presentation_label.as_ref().and_then(|_| {
        tokens
            .iter()
            .enumerate()
            .take(choose_spec.choose_idx)
            .find_map(|(idx, token)| {
                matches!(token.kind, TokenKind::Dash | TokenKind::EmDash).then_some(idx + 1)
            })
    });
    let choose_idx = choose_spec.choose_idx;
    let min = choose_spec.min;
    let max = choose_spec.max;
    let random = choose_spec.random;

    let mut trigger = None;
    let mut activated = None;
    let x_replacement = choose_spec.x_clause_start.and_then(|x_clause_start| {
        parse_x_is_value_clause(trim_lexed_commas(&tokens[x_clause_start..]))
    });
    let mut effect_start_idx = presentation_prefix_end.unwrap_or(0);
    if let Some(colon_idx) = locate_token_index(tokens, |token| token.kind == TokenKind::Colon)
        .filter(|idx| *idx < choose_idx)
    {
        let cost_tokens = &tokens[..colon_idx];
        let cost_raw = render_token_slice(cost_tokens);
        let cost_raw = cost_raw.trim();
        if !cost_raw.is_empty() {
            let cost_cst = parse_activation_cost_tokens(cost_tokens)?;
            let mana_cost = assemble_activation_cost(&cost_cst)?.to_core_total_cost();
            let prechoose_tokens = trim_lexed_commas(&tokens[colon_idx + 1..choose_idx]);
            let effect_sentences = if prechoose_tokens.is_empty() {
                Vec::new()
            } else {
                split_lexed_sentences(prechoose_tokens)
            };
            let loyalty_shorthand = is_loyalty_shorthand_cost_text(cost_raw);
            let functional_zones =
                infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);

            activated = Some(ModalActivatedHeader {
                mana_cost,
                functional_zones,
                timing: if loyalty_shorthand {
                    ActivationTiming::SorcerySpeed
                } else {
                    ActivationTiming::AnyTime
                },
                is_loyalty_ability: loyalty_shorthand,
                once_per_turn: loyalty_shorthand,
                activation_restrictions: Vec::new(),
            });
            effect_start_idx = colon_idx + 1;
        }
    }

    if let Some(activated) = activated.as_mut() {
        for sentence in split_lexed_sentences(&tokens[choose_idx + 1..]) {
            if let Some(timing) = parse_activate_only_timing_lexed(sentence) {
                activated.timing = timing;
            } else if let Some(condition) = parse_activation_condition_lexed(sentence) {
                activated.activation_restrictions.push(condition);
            }
        }
    }

    if activated.is_none()
        && let Some(comma_idx) = locate_token_index(tokens, |token| token.kind == TokenKind::Comma)
        && choose_idx > comma_idx
    {
        let start_idx = if effect_start_idx > 0 {
            effect_start_idx
        } else if tokens.first().is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        }) {
            1
        } else {
            0
        };
        if comma_idx > start_idx {
            let trigger_tokens = &tokens[start_idx..comma_idx];
            if !trigger_tokens.is_empty() {
                trigger = Some(parse_trigger_clause_lexed(trigger_tokens)?);
            }
        }
        effect_start_idx = comma_idx + 1;
    }

    let prechoose_tokens = if spree || tiered {
        &[]
    } else {
        trim_lexed_commas(&tokens[effect_start_idx..choose_idx])
    };
    let (prefix_effects_ast, modal_gate) = parse_modal_header_prefix_effects(prechoose_tokens)?;
    let common_prefix_effects_ast = parse_modal_common_prefix_effects(tokens, choose_idx)?;
    let common_suffix_effects_ast = parse_modal_common_suffix_effects(tokens, choose_idx)?;

    Ok(Some(ModalHeader {
        info: info.semantic_info(),
        min,
        max,
        spree,
        tiered,
        weighted_mode_points: super::grammar::modal::parse_modal_point_header_tokens(tokens)
            .is_some(),
        random,
        same_mode_more_than_once: modal_flags.same_mode_more_than_once,
        mode_must_be_unchosen: modal_flags.mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn: modal_flags.mode_must_be_unchosen_this_turn,
        distinct_player_targets_per_mode: modal_flags.distinct_player_targets_per_mode,
        if_kicked_choose_any_number: modal_flags.if_kicked_choose_any_number,
        conditional_mode_change,
        presentation_label,
        commander_allows_both: modal_flags.commander_allows_both,
        choose_both_control_card_types: modal_flags.choose_both_control_card_types,
        choose_both_exact_life_total: modal_flags.choose_both_exact_life_total,
        trigger,
        activated,
        x_replacement,
        prefix_effects_ast,
        common_prefix_effects_ast,
        common_suffix_effects_ast,
        modal_gate,
    }))
}

fn parse_modal_presentation_label(
    source_tokens: &[OwnedLexToken],
) -> Option<crate::ability::PresentationLabel> {
    let choose_idx =
        crate::slice_primitives::select_position(source_tokens, |token| token.is_word("choose"))?;
    let dash_idx = crate::slice_primitives::select_position(source_tokens, |token| {
        matches!(token.kind, TokenKind::Dash | TokenKind::EmDash)
    })?;
    if dash_idx >= choose_idx {
        return None;
    }
    let label_tokens = trim_lexed_commas(&source_tokens[..dash_idx]);
    let word_count = label_tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Word || token.kind == TokenKind::Number)
        .count();
    if word_count == 0 || word_count > 4 || label_tokens.iter().any(|token| token.is_period()) {
        return None;
    }
    let label = render_token_slice(label_tokens).trim().to_string();
    (!label.is_empty()).then(|| crate::ability::PresentationLabel::from_ability_word(label))
}

/// Parse the typed condition and alternate selection range from the common
/// modal sentence "Choose one. If ..., choose ... instead." Ordinary prose
/// containing either word is rejected unless the complete grammar is present.
fn parse_conditional_mode_change(
    tokens: &[OwnedLexToken],
    base_choose_idx: usize,
) -> Result<Option<ParsedConditionalModeChange>, CardTextError> {
    let Some(if_idx) = tokens
        .iter()
        .enumerate()
        .skip(base_choose_idx + 1)
        .find_map(|(idx, token)| token.is_word("if").then_some(idx))
    else {
        return Ok(None);
    };
    let Some(comma_idx) = tokens
        .iter()
        .enumerate()
        .skip(if_idx + 1)
        .find_map(|(idx, token)| (token.kind == TokenKind::Comma).then_some(idx))
    else {
        return Ok(None);
    };
    let Some(change_choose_idx) = tokens
        .iter()
        .enumerate()
        .skip(comma_idx + 1)
        .find_map(|(idx, token)| token.is_word("choose").then_some(idx))
    else {
        return Ok(None);
    };
    if !tokens[change_choose_idx + 1..]
        .iter()
        .any(|token| token.is_word("instead"))
    {
        return Ok(None);
    }

    let selection = if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("both") || token.is_word("two"))
    {
        ConditionalModeSelection::BothOrTwo
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("any"))
        && tokens
            .get(change_choose_idx + 2)
            .is_some_and(|token| token.is_word("number"))
    {
        ConditionalModeSelection::AnyNumber
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("one"))
        && tokens
            .get(change_choose_idx + 2)
            .is_some_and(|token| token.is_word("or"))
        && tokens
            .get(change_choose_idx + 3)
            .is_some_and(|token| token.is_word("more"))
    {
        ConditionalModeSelection::OneOrMore
    } else if tokens
        .get(change_choose_idx + 1)
        .is_some_and(|token| token.is_word("one"))
    {
        ConditionalModeSelection::One
    } else {
        return Ok(None);
    };

    let mut condition_end = comma_idx;
    if condition_end >= if_idx + 5
        && tokens[condition_end - 5..condition_end]
            .iter()
            .zip(["as", "you", "cast", "this", "spell"])
            .all(|(token, word)| token.is_word(word))
    {
        condition_end -= 5;
    }
    let condition_tokens = trim_lexed_commas(&tokens[if_idx + 1..condition_end]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = if condition_tokens.len() == 3
        && condition_tokens[0].is_word("it")
        && condition_tokens[1].is_word("was")
        && condition_tokens[2].is_word("kicked")
    {
        crate::cards::builders::PredicateAst::ThisSpellWasKicked
    } else {
        super::grammar::filters::parse_condition_predicate_lexed(condition_tokens)?
    };
    Ok(Some(ParsedConditionalModeChange {
        condition,
        selection,
    }))
}

fn parse_x_is_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    if tokens.len() < 2 || !tokens[0].is_word("x") || !tokens[1].is_word("is") {
        return None;
    }

    if locate_token_word_choice(tokens, &["spell", "spells"]).is_some()
        && locate_token_word_choice(tokens, &["cast", "casts"]).is_some()
        && contains_token_word(tokens, "turn")
    {
        let player =
            if locate_token_word_choice(tokens, &["you", "your", "youve", "you've"]).is_some() {
                PlayerFilter::You
            } else if locate_token_word_choice(tokens, &["opponent", "opponents"]).is_some() {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
        return Some(Value::SpellsCastThisTurn(player));
    }

    let mut where_prefixed = Vec::with_capacity(tokens.len() + 3);
    where_prefixed.push(OwnedLexToken::word(
        "where",
        tokens
            .first()
            .map(|token| token.span)
            .unwrap_or_else(crate::cards::builders::TextSpan::synthetic),
    ));
    where_prefixed.extend_from_slice(tokens);
    parse_value_binding_clause_lexed(&where_prefixed)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
}

pub fn replace_modal_header_x_in_effects_ast(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_modal_header_x_in_effect_ast(effect, replacement, clause)?;
    }
    Ok(())
}

fn replace_modal_header_x_in_value(
    value: &mut Value,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    if !value_contains_unbound_x(value) {
        return Ok(());
    }
    *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
    Ok(())
}

fn replace_modal_header_x_in_effect_ast(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Draw { count: amount }
            | SubjectVerbActionAst::ExileTopOfLibrary { count: amount, .. }
            | SubjectVerbActionAst::LoseLife { amount }
            | SubjectVerbActionAst::PayLife { amount }
            | SubjectVerbActionAst::GainLife { amount }
            | SubjectVerbActionAst::Mill { count: amount }
            | SubjectVerbActionAst::Scry { count: amount }
            | SubjectVerbActionAst::Surveil { count: amount }
            | SubjectVerbActionAst::Proliferate { count: amount }
            | SubjectVerbActionAst::Investigate { count: amount }
            | SubjectVerbActionAst::Monstrosity { amount }
            | SubjectVerbActionAst::Discover { count: amount }
            | SubjectVerbActionAst::Fateseal { count: amount }
            | SubjectVerbActionAst::Populate { count: amount, .. }
            | SubjectVerbActionAst::Connive { count: amount, .. }
            | SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. }
            | SubjectVerbActionAst::PreventDamage { amount, .. }
            | SubjectVerbActionAst::PreventDamageEach { amount, .. }
            | SubjectVerbActionAst::CopySpell { count: amount, .. }
            | SubjectVerbActionAst::PutCounters { count: amount, .. }
            | SubjectVerbActionAst::PutCounterChoice { count: amount, .. }
            | SubjectVerbActionAst::PutCountersAll { count: amount, .. }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { amount, .. }
            | SubjectVerbActionAst::RemoveCountersAll { amount, .. }
            | SubjectVerbActionAst::Discard { count: amount, .. }
            | SubjectVerbActionAst::PoisonCounters { count: amount }
            | SubjectVerbActionAst::EnergyCounters { count: amount }
            | SubjectVerbActionAst::ExperienceCounters { count: amount }
            | SubjectVerbActionAst::TicketCounters { count: amount }
            | SubjectVerbActionAst::PayEnergy { amount }
            | SubjectVerbActionAst::SetLifeTotal { amount }
            | SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount }
            | SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, .. }
            | SubjectVerbActionAst::LookAtTopCards { count: amount, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop {
                position: amount, ..
            }
            | SubjectVerbActionAst::AdditionalLandPlays { count: amount, .. }
            | SubjectVerbActionAst::HealDamage {
                amount: Some(amount),
                ..
            } => {
                replace_modal_header_x_in_value(amount, replacement, clause)?
            }
            SubjectVerbActionAst::Incubate { amount, count } => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
                replace_modal_header_x_in_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            } => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            } => {
                replace_modal_header_x_in_value(put_count, replacement, clause)?;
                replace_modal_header_x_in_value(remove_count, replacement, clause)?;
            }
            SubjectVerbActionAst::Pump {
                power, toughness, ..
            }
            | SubjectVerbActionAst::SetBasePowerToughness {
                power, toughness, ..
            }
            | SubjectVerbActionAst::PumpAll {
                power, toughness, ..
            } => {
                replace_modal_header_x_in_value(power, replacement, clause)?;
                replace_modal_header_x_in_value(toughness, replacement, clause)?;
            }
            SubjectVerbActionAst::SetBasePower { power, .. } => {
                replace_modal_header_x_in_value(power, replacement, clause)?;
            }
            SubjectVerbActionAst::PumpForEach { count, .. } => {
                replace_modal_header_x_in_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::DealDamageEqualToPower { .. }
            | SubjectVerbActionAst::DrawForEachTaggedMatching { .. }
            | SubjectVerbActionAst::RevealHand
            | SubjectVerbActionAst::RevealTop
            | SubjectVerbActionAst::RevealTagged { .. }
            | SubjectVerbActionAst::RevealCardsFromHand { .. }
            | SubjectVerbActionAst::LookAtObjects { .. }
            | SubjectVerbActionAst::LookAtTarget { .. }
            | SubjectVerbActionAst::EmitKeywordAction { .. }
            | SubjectVerbActionAst::Amass { .. }
            | SubjectVerbActionAst::Bolster { .. }
            | SubjectVerbActionAst::Support { .. }
            | SubjectVerbActionAst::Adapt { .. }
            | SubjectVerbActionAst::Explore { .. }
            | SubjectVerbActionAst::Endure { .. }
            | SubjectVerbActionAst::Exploit
            | SubjectVerbActionAst::ConniveIterated
            | SubjectVerbActionAst::OpenAttraction { .. }
            | SubjectVerbActionAst::ManifestTopCardOfLibrary
            | SubjectVerbActionAst::CloakTopCardOfLibrary
            | SubjectVerbActionAst::ManifestCardFromHand
            | SubjectVerbActionAst::ManifestDread
            | SubjectVerbActionAst::HealDamage { amount: None, .. }
            | SubjectVerbActionAst::Earthbend { .. }
            | SubjectVerbActionAst::Behold { .. }
            | SubjectVerbActionAst::Fight { .. }
            | SubjectVerbActionAst::FightIterated { .. }
            | SubjectVerbActionAst::Clash { .. }
            | SubjectVerbActionAst::FlipCoin
            | SubjectVerbActionAst::FlipCoinFaceOnly
            | SubjectVerbActionAst::RollDie { .. }
            | SubjectVerbActionAst::RollDiceChooseResult { .. }
            | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
            | SubjectVerbActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary
            | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary { .. }
            | SubjectVerbActionAst::ReorderGraveyard
            | SubjectVerbActionAst::ChooseColor
            | SubjectVerbActionAst::ChooseCardType { .. }
            | SubjectVerbActionAst::ChooseNamedOption { .. }
            | SubjectVerbActionAst::ChooseCreatureType { .. }
            | SubjectVerbActionAst::ChooseLandType { .. }
            | SubjectVerbActionAst::ChooseCardName { .. }
            | SubjectVerbActionAst::ChoosePlayer { .. }
            | SubjectVerbActionAst::NoteLifeTotal
            | SubjectVerbActionAst::AddMana { .. }
            | SubjectVerbActionAst::ExchangeLifeTotals { .. }
            | SubjectVerbActionAst::ExchangeTextBoxes { .. }
            | SubjectVerbActionAst::ExchangeZones { .. }
            | SubjectVerbActionAst::PutRestOnBottomOfLibrary
            | SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn
            | SubjectVerbActionAst::ExchangeValues { .. }
            | SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn
            | SubjectVerbActionAst::ControlCombatChoicesThisTurn { .. }
            | SubjectVerbActionAst::GainControl { .. }
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::SwitchPowerToughness { .. }
            | SubjectVerbActionAst::AddManaColorsAmong { .. }
            | SubjectVerbActionAst::AddOneManaAnyColorAmong { .. }
            | SubjectVerbActionAst::AddManaImprintedColors
            | SubjectVerbActionAst::DoubleManaPool
            | SubjectVerbActionAst::EmptyManaPool
            | SubjectVerbActionAst::EndTurn
            | SubjectVerbActionAst::EndCombatPhase
            | SubjectVerbActionAst::SkipTurn
            | SubjectVerbActionAst::SkipCombatPhases
            | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
            | SubjectVerbActionAst::SkipMainPhasesThisTurn
            | SubjectVerbActionAst::SkipCombatPhasesThisTurn
            | SubjectVerbActionAst::SkipDrawStep
            | SubjectVerbActionAst::PlayFromGraveyardUntilEot
            | SubjectVerbActionAst::ControlPlayer { .. }
            | SubjectVerbActionAst::ReduceNextSpellCostThisTurn { .. }
            | SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn { .. }
            | SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { .. }
            | SubjectVerbActionAst::RingTemptsYou
            | SubjectVerbActionAst::VentureIntoDungeon { .. }
            | SubjectVerbActionAst::BecomeMonarch
            | SubjectVerbActionAst::TakeInitiative
            | SubjectVerbActionAst::CreateEmblem { .. }
            | SubjectVerbActionAst::LoseGame
            | SubjectVerbActionAst::WinGame
            | SubjectVerbActionAst::ReorderTopPlanarDeck { .. }
            | SubjectVerbActionAst::ReturnSourceTransformedFromExile
            | SubjectVerbActionAst::Reconfigure { .. }
            | SubjectVerbActionAst::CumulativeUpkeep { .. }
            | SubjectVerbActionAst::Casualty { .. }
            | SubjectVerbActionAst::PayAnyEnergy { .. }
            | SubjectVerbActionAst::PayAnyLife { .. }
            | SubjectVerbActionAst::PayMana { .. }
            | SubjectVerbActionAst::DiscardHand
            | SubjectVerbActionAst::Detain { .. }
            | SubjectVerbActionAst::Goad { .. }
            | SubjectVerbActionAst::Suspect { .. }
            | SubjectVerbActionAst::ClearSuspected { .. }
            | SubjectVerbActionAst::RemoveFromCombat { .. }
            | SubjectVerbActionAst::Flip { .. }
            | SubjectVerbActionAst::Regenerate { .. }
            | SubjectVerbActionAst::RegenerateAll { .. }
            | SubjectVerbActionAst::TapAll { .. }
            | SubjectVerbActionAst::UntapAll { .. }
            | SubjectVerbActionAst::TapOrUntap { .. }
            | SubjectVerbActionAst::TapOrUntapAll { .. }
            | SubjectVerbActionAst::PhaseOut { .. }
            | SubjectVerbActionAst::PhaseOutAll { .. }
            | SubjectVerbActionAst::PhaseIn { .. }
            | SubjectVerbActionAst::PhaseInAll { .. }
            | SubjectVerbActionAst::Transform { .. }
            | SubjectVerbActionAst::Convert { .. }
            | SubjectVerbActionAst::Tap { .. }
            | SubjectVerbActionAst::Untap { .. }
            | SubjectVerbActionAst::Destroy { .. }
            | SubjectVerbActionAst::DestroyAll { .. }
            | SubjectVerbActionAst::DestroyAllOfChosenColor { .. }
            | SubjectVerbActionAst::Exile { .. }
            | SubjectVerbActionAst::ExileAll { .. }
            | SubjectVerbActionAst::LookAtHand { .. }
            | SubjectVerbActionAst::Counter { .. }
            | SubjectVerbActionAst::CounterUnlessPays { .. }
            | SubjectVerbActionAst::ReturnToHand { .. }
            | SubjectVerbActionAst::ReturnAllToHand { .. }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { .. }
            | SubjectVerbActionAst::DoubleCountersOnEach { .. }
            | SubjectVerbActionAst::DoubleCountersOnTarget { .. }
            | SubjectVerbActionAst::MoveAllCounters { .. }
            | SubjectVerbActionAst::MoveOneCounter { .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { .. }
            | SubjectVerbActionAst::PutCounterOfChosenKind { .. }
            | SubjectVerbActionAst::Sacrifice { .. }
            | SubjectVerbActionAst::SacrificeAll { .. }
            | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
            | SubjectVerbActionAst::ReorderTopOfLibrary { .. }
            | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
            | SubjectVerbActionAst::ScalePowerToughnessAll { .. }
            | SubjectVerbActionAst::ScaleXValue { .. }
            | SubjectVerbActionAst::GrantProtectionChoice { .. }
            | SubjectVerbActionAst::PreventAllCombatDamage { .. }
            | SubjectVerbActionAst::AssignNoCombatDamage { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToPlayers { .. }
            | SubjectVerbActionAst::PreventAllCombatDamageToYou { .. }
            | SubjectVerbActionAst::PreventNextTimeDamage { .. }
            | SubjectVerbActionAst::ReplaceNextDamageToTarget { .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget { .. }
            | SubjectVerbActionAst::PreventAllDamageToTarget { .. }
            | SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventAllDamageFromSourceFilter { .. }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters { amount: None, .. }
            | SubjectVerbActionAst::Meld { .. }
            | SubjectVerbActionAst::CreateTokenChoice { .. }
            | SubjectVerbActionAst::SearchLibrarySlotsToHand { .. }
            | SubjectVerbActionAst::RetargetStackObject { .. }
            | SubjectVerbActionAst::GrantAbilityToSource { .. }
            | SubjectVerbActionAst::ExchangeControl { .. }
            | SubjectVerbActionAst::ExchangeControlHeterogeneous { .. }
            | SubjectVerbActionAst::DestroyAllAttachedTo { .. }
            | SubjectVerbActionAst::ExileAllAttachedTo { .. }
            | SubjectVerbActionAst::Attach { .. }
            | SubjectVerbActionAst::Unattach { .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { .. }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { .. }
            | SubjectVerbActionAst::MayMoveToZone { .. }
            | SubjectVerbActionAst::RegisterZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterFutureZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterDrawReplacement { .. }
            | SubjectVerbActionAst::RegisterManaReplacement { .. }
            | SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement { .. }
            | SubjectVerbActionAst::RegisterEnterUnderControlReplacement { .. }
            | SubjectVerbActionAst::RegisterEnterTappedReplacement { .. }
            | SubjectVerbActionAst::RegisterNextBatchEnterWithCounters { .. }
            | SubjectVerbActionAst::Enchant { .. }
            | SubjectVerbActionAst::ChooseSpellCastHistory { .. }
            | SubjectVerbActionAst::CopySpellForEachTarget { .. }
            | SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }
            | SubjectVerbActionAst::PutTaggedRemainderInZone { .. }
            | SubjectVerbActionAst::CastTagged { .. }
            | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. }
            | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. }
            | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. }
            | SubjectVerbActionAst::ReturnToBattlefield { .. }
            | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { .. }
            | SubjectVerbActionAst::MoveToZone { .. }
            | SubjectVerbActionAst::PutOntoBattlefield { .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { .. }
            | SubjectVerbActionAst::TargetOnly { .. }
            | SubjectVerbActionAst::TagMatchingObjects { .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { .. }
            | SubjectVerbActionAst::PumpByLastEffect { .. }
            | SubjectVerbActionAst::AddCardTypes { .. }
            | SubjectVerbActionAst::SetCardTypes { .. }
            | SubjectVerbActionAst::RemoveCardTypes { .. }
            | SubjectVerbActionAst::AddSubtypes { .. }
            | SubjectVerbActionAst::RemoveSubtypes { .. }
            | SubjectVerbActionAst::SetCreatureSubtypes { .. }
            | SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { .. }
            | SubjectVerbActionAst::AddColors { .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { .. }
            | SubjectVerbActionAst::BecomeAuraEnchantment { .. }
            | SubjectVerbActionAst::BecomeBasicLandType { .. }
            | SubjectVerbActionAst::SetColors { .. }
            | SubjectVerbActionAst::MakeColorless { .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { .. }
            | SubjectVerbActionAst::BecomeColorChoice { .. }
            | SubjectVerbActionAst::BecomeCopy { .. }
            | SubjectVerbActionAst::GrantAbilitiesAll { .. }
            | SubjectVerbActionAst::RemoveAbilitiesAll { .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceAll { .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { .. }
            | SubjectVerbActionAst::GrantToTarget { .. }
            | SubjectVerbActionAst::GrantBySpec { .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { .. }
            | SubjectVerbActionAst::ConsultTopOfLibrary { .. }
            | SubjectVerbActionAst::SearchLibrary { .. }
            | SubjectVerbActionAst::Cant { .. }
            | SubjectVerbActionAst::AdditionalPhases { .. }
            | SubjectVerbActionAst::ReverseTurnOrder
            | SubjectVerbActionAst::TurnFaceUp { .. }
            | SubjectVerbActionAst::ShuffleLibrary => {}
            SubjectVerbActionAst::CreateTokenCopy { count: amount, .. }
            | SubjectVerbActionAst::CreateTokenCopyFromSource { count: amount, .. } => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                count: amount,
                dynamic_power_toughness,
                ..
            } => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
                if let Some((power, toughness)) = dynamic_power_toughness {
                    replace_modal_header_x_in_value(power, replacement, clause)?;
                    replace_modal_header_x_in_value(toughness, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::Learn | SubjectVerbActionAst::UnlockRoomDoor => {}
        },
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_modal_header_x_in_effects_ast(nested, replacement, clause)
            })?;
        }
    }

    Ok(())
}

fn parse_modal_header_prefix_effects(
    tokens: &[OwnedLexToken],
) -> Result<(Vec<EffectAst>, Option<ModalGate>), CardTextError> {
    if tokens.is_empty() {
        return Ok((Vec::new(), None));
    }

    let (prefix_tokens, modal_gate) =
        if let Some(gate_spec) = split_trailing_modal_gate_clause(tokens) {
            let effect_predicate = match gate_spec.predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::WonClash => {
                    EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
                }
                IfResultPredicate::AcceptedChoice => EffectPredicate::Chosen,
                IfResultPredicate::DidNot
                | IfResultPredicate::ExplicitDidNot
                | IfResultPredicate::Otherwise => EffectPredicate::DidNotHappen,
                IfResultPredicate::SearchedLibrary => EffectPredicate::SearchedLibrary,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
                IfResultPredicate::ExcessDamageDealt => EffectPredicate::ExcessDamageDealt,
                IfResultPredicate::DealtDamageToPlayer => EffectPredicate::DealtDamageToPlayer,
                IfResultPredicate::AffectedObjectMatchesCardType { card_type, negated } => {
                    EffectPredicate::AffectedObjectMatchesCardType { card_type, negated }
                }
                IfResultPredicate::PriorEffectResult(surface) => {
                    EffectPredicate::PriorEffectResult(surface)
                }
                IfResultPredicate::WasDeclined => EffectPredicate::WasDeclined,
                IfResultPredicate::Value(cmp) => EffectPredicate::Value(cmp),
            };
            (
                gate_spec.prefix_tokens,
                Some(ModalGate {
                    predicate: effect_predicate,
                    remove_mode_only: gate_spec.remove_mode_only,
                    reflexive: gate_spec.reflexive,
                }),
            )
        } else {
            (tokens, None)
        };
    if prefix_tokens.is_empty() {
        return Ok((Vec::new(), modal_gate));
    }

    let effects = parse_effect_sentences_lexed(prefix_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "modal header prefix produced no effects".to_string(),
        ));
    }

    Ok((effects, modal_gate))
}

fn is_loyalty_shorthand_cost_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "0"
        || strip_leading_sign(trimmed)
            .is_some_and(|tail| tail.eq_ignore_ascii_case("x") || tail.parse::<u32>().is_ok())
}
