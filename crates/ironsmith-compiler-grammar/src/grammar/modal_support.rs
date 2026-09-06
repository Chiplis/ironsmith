use crate::cards::builders::DamagePreventionActionAst;
use crate::cards::builders::{
    ActivationTiming, CardTextError, ConditionalModeSelection, EffectAst, EffectPredicate,
    IfResultPredicate, LineInfo, ParsedConditionalModeChange, ParsedModalActivatedHeader,
    ParsedModalGate, ParsedModalHeader, SubjectVerbActionAst, CounterActionAst, GrantActionAst, ManaActionAst, LibraryActionAst, ReplacementActionAst, ExchangeActionAst, TurnStructureActionAst, CharacteristicActionAst, KeywordActionAst, ZoneMoveActionAst, PermanentStateActionAst, RevealLookActionAst, RandomActionAst, LifeResourceActionAst, ChoiceActionAst, DamageActionAst, StatChangeActionAst, StackActionAst, TokenActionAst, ControlActionAst, GameActionAst,
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
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: amount })
            | SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary { count: amount, .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
            | SubjectVerbActionAst::Library(LibraryActionAst::Mill { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Surveil { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Proliferate { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Monstrosity { amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Discover { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fateseal { count: amount })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Populate { count: amount, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive { count: amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage { amount, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageEach { amount, .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpell { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters { amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll { amount, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { count: amount, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PoisonCounters { count: amount })
            | SubjectVerbActionAst::Counters(CounterActionAst::EnergyCounters { count: amount })
            | SubjectVerbActionAst::Counters(CounterActionAst::ExperienceCounters { count: amount })
            | SubjectVerbActionAst::Counters(CounterActionAst::TicketCounters { count: amount })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { amount })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetLifeTotal { amount })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaScaled { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyColor { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyOneColor { amount })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaChosenColor { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaFromLandCouldProduce { amount, .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaCommanderIdentity { amount })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget { amount, .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count: amount, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop {
                position: amount, ..
            })
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalLandPlays { count: amount, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage {
                amount: Some(amount),
                ..
            }) => {
                replace_modal_header_x_in_value(amount, replacement, clause)?
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Incubate { amount, count }) => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
                replace_modal_header_x_in_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            }) => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            }) => {
                replace_modal_header_x_in_value(put_count, replacement, clause)?;
                replace_modal_header_x_in_value(remove_count, replacement, clause)?;
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                power, toughness, ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power, toughness, ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                power, toughness, ..
            }) => {
                replace_modal_header_x_in_value(power, replacement, clause)?;
                replace_modal_header_x_in_value(toughness, replacement, clause)?;
            }
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { power, .. }) => {
                replace_modal_header_x_in_value(power, replacement, clause)?;
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { count, .. }) => {
                replace_modal_header_x_in_value(count, replacement, clause)?;
            }
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealHand)
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTop)
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealCardsFromHand { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtObjects { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTarget { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmitKeywordAction { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Support { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Adapt { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Exploit)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ConniveIterated)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::OpenAttraction { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ManifestTopCardOfLibrary)
            | SubjectVerbActionAst::Library(LibraryActionAst::CloakTopCardOfLibrary)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestCardFromHand)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestDread)
            | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { amount: None, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Earthbend { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Behold { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::FightIterated { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Clash { .. })
            | SubjectVerbActionAst::Random(RandomActionAst::FlipCoin)
            | SubjectVerbActionAst::Random(RandomActionAst::FlipCoinFaceOnly)
            | SubjectVerbActionAst::Random(RandomActionAst::RollDie { .. })
            | SubjectVerbActionAst::Random(RandomActionAst::RollDiceChooseResult { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary)
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary)
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleGraveyardIntoLibrary { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ReorderGraveyard)
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor)
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardType { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNamedOption { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseLandType { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardName { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::NoteLifeTotal)
            | SubjectVerbActionAst::Mana(ManaActionAst::AddMana { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeLifeTotals { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeZones { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::PutRestOnBottomOfLibrary)
            | SubjectVerbActionAst::Mana(ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn)
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeValues { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn)
            | SubjectVerbActionAst::Control(ControlActionAst::ControlCombatChoicesThisTurn { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::GainControl { .. })
            | SubjectVerbActionAst::PutSticker { .. }
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaColorsAmong { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::AddManaImprintedColors)
            | SubjectVerbActionAst::Mana(ManaActionAst::DoubleManaPool)
            | SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool)
            | SubjectVerbActionAst::Game(GameActionAst::EndTurn)
            | SubjectVerbActionAst::Game(GameActionAst::EndCombatPhase)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhases)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn)
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipDrawStep)
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot)
            | SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RingTemptsYou)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::VentureIntoDungeon { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeMonarch)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::TakeInitiative)
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { .. })
            | SubjectVerbActionAst::Game(GameActionAst::LoseGame)
            | SubjectVerbActionAst::Game(GameActionAst::WinGame)
            | SubjectVerbActionAst::ReorderTopPlanarDeck { .. }
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnSourceTransformedFromExile)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Reconfigure { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::CumulativeUpkeep { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Casualty { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyEnergy { .. })
            | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { .. })
            | SubjectVerbActionAst::Mana(ManaActionAst::PayMana { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand)
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate { .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::Counter { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove { .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { .. })
            | SubjectVerbActionAst::Game(GameActionAst::ExtraTurnAfterTurn { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ReorderTopOfLibrary { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary { .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ScaleXValue { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantProtectionChoice { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamage { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToPlayers { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToYou { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventNextTimeDamage { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::ReplaceNextDamageToTarget { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter { .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters { amount: None, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Meld { .. })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenChoice { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrarySlotsToHand { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilityToSource { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControl { .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Attach { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Unattach { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDrawReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterManaReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamagedBySourceZoneReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterUnderControlReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterTappedReplacement { .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters { .. })
            | SubjectVerbActionAst::Control(ControlActionAst::Enchant { .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseSpellCastHistory { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CastTagged { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { .. })
            | SubjectVerbActionAst::TargetOnly { .. }
            | SubjectVerbActionAst::TagMatchingObjects { .. }
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice { .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec { .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget { .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { .. })
            | SubjectVerbActionAst::Cant { .. }
            | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases { .. })
            | SubjectVerbActionAst::Game(GameActionAst::ReverseTurnOrder)
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary) => {}
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { count: amount, .. })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { count: amount, .. }) => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                count: amount,
                dynamic_power_toughness,
                ..
            }) => {
                replace_modal_header_x_in_value(amount, replacement, clause)?;
                if let Some((power, toughness)) = dynamic_power_toughness {
                    replace_modal_header_x_in_value(power, replacement, clause)?;
                    replace_modal_header_x_in_value(toughness, replacement, clause)?;
                }
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Learn) | SubjectVerbActionAst::KeywordActions(KeywordActionAst::UnlockRoomDoor) => {}
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
