use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::clause_support::{
    parse_static_ability_ast_line_lexed, parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::super::grammar::primitives::{
    TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_list_conjunction,
};
use super::super::grammar::structure::parse_trailing_if_predicate_lexed;
use super::super::lexer::{
    OwnedLexToken, TokenKind, contains_token_kind, is_authored_proper_name_phrase,
    locate_token_kind, locate_token_word, token_slice_first_is, trim_lexed_commas,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
#[cfg(test)]
use super::super::token_primitives::str_contains as string_contains;
use super::super::token_primitives::strip_leading_if_you_do_lexed;
use super::super::util::{
    is_source_reference_words, parse_card_type, parse_mana_symbol, parse_subtype_flexible,
    parse_target_phrase, source_reference_surface_for_possessive_words,
    source_reference_surface_for_words, span_from_tokens, strip_leading_token_words_any,
    this_source_surface_for_words, trim_commas,
};
use super::clause_dispatch::parse_become_clause;
use super::dispatch_inner::trim_edge_punctuation;
use super::lex_chain_helpers::find_verb_lexed;
use super::sentence_helpers::*;
use super::subject_verb_primitives::SubjectVerbPrimitiveClause;
use super::{Verb, find_verb, parse_effect_chain, parse_effect_sentence_lexed};
use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IfResultPredicate, KeywordAction, LineAst,
    ParsedAbility, PlayerAst, PredicateAst, ReferenceImports, StaticAbilityAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, TagKey, TargetAst, TextSpan, TriggerSpec, PermissionEffectAst, PlayerPredicateAst,
};
use crate::effect::{Until, Value};
use crate::grammar::clause_support as clause_grammar;
use crate::grammar::effects::gain_ability_shapes as gain_shapes;
use crate::grammar::trigger_surface;
use crate::mana::ManaCost;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::model::compiler_semantic::CompilerAbilityCore as Ability;
use crate::model::token_definition::TokenDefinitionSpec;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter, SourceReferenceSurface};
use crate::types::CardType;
use crate::zone::Zone;
#[cfg(test)]
use ironsmith_compiler_lowering::compile_support::compile_statement_effects;
use ironsmith_compiler_semantic::keyword_abilities::assemble_parsed_triggered_ability as parsed_triggered_ability;

type GainAbilityWordView<'a> = TokenWordView<'a>;
type SharedSubjectPump = (
    Value,
    Value,
    usize,
    Until,
    Option<PredicateAst>,
    Option<(i32, i32, Value)>,
);
type SharedSubjectBasePt = (Value, Value, usize, Until);
type SharedSubjectGrant = (Vec<GrantedAbilityAst>, bool);

fn trim_edge_punctuation_and_quotes(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    fn trim_non_quote_punctuation(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
        let mut start = 0usize;
        let mut end = tokens.len();
        while start < end
            && matches!(
                tokens[start].kind,
                TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
            )
        {
            start += 1;
        }
        while end > start
            && matches!(
                tokens[end - 1].kind,
                TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
            )
        {
            end -= 1;
        }
        tokens[start..end].to_vec()
    }

    let mut tokens = trim_non_quote_punctuation(tokens);
    loop {
        let edge_kind = tokens
            .first()
            .map(|token| token.kind)
            .filter(|kind| matches!(kind, TokenKind::Quote | TokenKind::Apostrophe));
        let edge_count = edge_kind.map_or(0, |kind| {
            tokens.iter().filter(|token| token.kind == kind).count()
        });
        let has_matching_quote_pair = tokens.len() >= 2
            && edge_count.is_multiple_of(2)
            && edge_kind.is_some()
            && tokens
                .last()
                .is_some_and(|token| Some(token.kind) == edge_kind);
        if has_matching_quote_pair {
            tokens = trim_non_quote_punctuation(&tokens[1..tokens.len() - 1]);
        } else if edge_count % 2 == 1 && edge_kind.is_some() {
            // Sentence splitting keeps the opening delimiter when the closing
            // quote follows sentence-final punctuation. Remove only that
            // unmatched outer delimiter; any balanced nested quotes remain
            // available to the granted-ability parser.
            tokens = trim_non_quote_punctuation(&tokens[1..]);
        } else {
            break;
        }
    }
    tokens
}

const AND_WORD: &str = "and";
const ALSO_WORD: &str = "also";
const THE_WORD: &str = "the";

fn single_or_sequence_effect(mut effects: Vec<EffectAst>) -> Option<EffectAst> {
    if effects.len() == 1 {
        effects.pop()
    } else {
        Some(EffectAst::Sequence { effects })
    }
}

fn display_text_for_tokens(tokens: &[OwnedLexToken]) -> String {
    let mut text = String::new();
    let mut needs_space = false;
    let mut in_effect_text = tokens.first().is_some_and(|token| {
        token.kind == TokenKind::Word
            && (gain_shapes::gain_word_is_when_intro(token.parser_text())
                || gain_shapes::gain_word_is_trigger_intro(token.parser_text()))
    });

    for token in tokens {
        if let Some(word) = token.as_word() {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            let numeric_like = word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '+' | '-' | '/'));
            let rendered = match word {
                "t" => "{T}".to_string(),
                "q" => "{Q}".to_string(),
                _ if in_effect_text && numeric_like => word.to_string(),
                _ => parse_mana_symbol(word)
                    .map(|symbol| ManaCost::from_symbols(vec![symbol]).to_oracle())
                    .unwrap_or_else(|_| word.to_ascii_lowercase()),
            };
            text.push_str(&rendered);
            needs_space = true;
        } else if token.kind == TokenKind::ManaGroup {
            if needs_space && !text.is_empty() {
                text.push(' ');
            }
            text.push_str(token.slice.as_str());
            needs_space = true;
        } else if token.is_colon() {
            text.push(':');
            needs_space = true;
            in_effect_text = true;
        } else if token.is_comma() {
            text.push(',');
            needs_space = true;
        } else if token.is_period() {
            text.push('.');
            needs_space = true;
        } else if token.is_semicolon() {
            text.push(';');
            needs_space = true;
        }
    }

    text
}

fn append_shared_subject_pump_to_target(
    effects: &mut Vec<EffectAst>,
    target: &TargetAst,
    pump_effect: &Option<SharedSubjectPump>,
) {
    let Some((power, toughness, _, pump_duration, condition, for_each)) = pump_effect else {
        return;
    };
    if let Some((power_per, toughness_per, count)) = for_each {
        effects.push(EffectAst::subject_verb_pump_for_each(
            *power_per,
            *toughness_per,
            target.clone(),
            count.clone(),
            pump_duration.clone(),
        ));
    } else {
        effects.push(EffectAst::subject_verb_pump(
            power.clone(),
            toughness.clone(),
            target.clone(),
            pump_duration.clone(),
            condition.clone(),
        ));
    }
}

fn bind_shared_subject_characteristic_fallback(value: &Value) -> Value {
    let shared_target = || {
        Box::new(ChooseSpec::Tagged(
            (crate::tag::CompilerReferenceTag::It.bind()).into(),
        ))
    };
    match value {
        Value::SourcePower => Value::PowerOf(shared_target()),
        Value::SourceToughness => Value::ToughnessOf(shared_target()),
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_shared_subject_characteristic_fallback(value)),
            hints: hints.clone(),
        },
        Value::Add(left, right) => Value::Add(
            Box::new(bind_shared_subject_characteristic_fallback(left)),
            Box::new(bind_shared_subject_characteristic_fallback(right)),
        ),
        Value::Scaled(value, multiplier) => Value::Scaled(
            Box::new(bind_shared_subject_characteristic_fallback(value)),
            *multiplier,
        ),
        Value::DividedRoundedDown(value, divisor) => Value::DividedRoundedDown(
            Box::new(bind_shared_subject_characteristic_fallback(value)),
            *divisor,
        ),
        Value::HalfRoundedDown(value) => {
            Value::HalfRoundedDown(Box::new(bind_shared_subject_characteristic_fallback(value)))
        }
        Value::Min(left, right) => Value::Min(
            Box::new(bind_shared_subject_characteristic_fallback(left)),
            Box::new(bind_shared_subject_characteristic_fallback(right)),
        ),
        _ => value.clone(),
    }
}

/// Bind a bare possessive characteristic in a shared target clause to that
/// clause's declared target. Explicit source references already parse as
/// `PowerOf(Source)`/`ToughnessOf(Source)` and therefore remain unchanged.
fn bind_shared_subject_pump_characteristics(pump: &mut Option<SharedSubjectPump>) {
    let Some((power, toughness, ..)) = pump else {
        return;
    };
    *power = bind_shared_subject_characteristic_fallback(power);
    *toughness = bind_shared_subject_characteristic_fallback(toughness);
}

fn append_shared_subject_base_pt_to_target(
    effects: &mut Vec<EffectAst>,
    target: &TargetAst,
    base_pt_effect: &Option<SharedSubjectBasePt>,
) {
    let Some((power, toughness, _, duration)) = base_pt_effect else {
        return;
    };
    effects.push(EffectAst::subject_verb_set_base_power_toughness(
        power.clone(),
        toughness.clone(),
        target.clone(),
        duration.clone(),
    ));
}

fn append_shared_subject_grant_to_target(
    effects: &mut Vec<EffectAst>,
    target: &TargetAst,
    grant: &Option<SharedSubjectGrant>,
    duration: &Until,
) {
    let Some((abilities, is_choice)) = grant else {
        return;
    };
    if *is_choice {
        effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
            target.clone(),
            abilities.clone(),
            duration.clone(),
        ));
    } else {
        effects.push(EffectAst::subject_verb_grant_abilities_to_target(
            target.clone(),
            abilities.clone(),
            duration.clone(),
        ));
    }
}

fn coordinated_gain_surface(
    tokens: &[OwnedLexToken],
    mut effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    if effects.len() < 2 {
        return effects;
    }
    let words = GainAbilityWordView::new(tokens).to_word_refs();
    let Some((gain_idx, gain_verb)) = gain_shapes::find_primary_gain_ability_verb(&words) else {
        return effects;
    };
    let preceding_action = words[..gain_idx].iter().any(|word| {
        matches!(
            *word,
            "get" | "gets" | "has" | "have" | "become" | "becomes"
        )
    });
    let following_action = gain_shapes::find_shared_ability_tail(
        words.get(gain_idx + 1..).unwrap_or_default(),
        gain_shapes::SharedAbilityTail::Get,
    )
    .is_some()
        || (gain_verb == gain_shapes::GainAbilityVerb::Lose
            && gain_shapes::find_shared_ability_tail(
                words.get(gain_idx + 1..).unwrap_or_default(),
                gain_shapes::SharedAbilityTail::Gain,
            )
            .is_some())
        || (gain_verb == gain_shapes::GainAbilityVerb::Lose
            && gain_shapes::find_shared_ability_tail(
                words.get(gain_idx + 1..).unwrap_or_default(),
                gain_shapes::SharedAbilityTail::Has,
            )
            .is_some());
    if !preceding_action && !following_action {
        return effects;
    }
    let leading_duration = gain_shapes::parse_leading_gain_duration_shape(&words).is_some();
    if leading_duration {
        // The canonical control-flow node owns a leading duration. Its body
        // describes the unscoped continuous actions; lowering applies the
        // control duration to those leaves mechanically. Retaining the same
        // duration on both levels duplicates semantic ownership and loses the
        // distinction from an authored trailing duration on one child.
        for effect in &mut effects {
            apply_gain_clause_duration_to_leading_effect(effect, &Until::Forever);
        }
    }
    let coordination = crate::grammar::effects::coordination::coordination_from_effects(
        crate::model::CoordinationKindAst::SharedSubject,
        crate::model::CoordinationOperatorAst::And,
        crate::model::EffectOrderingAst::Unordered,
        effects,
    )
    .expect("coordinated gain surface contains at least two effects");
    let coordinated = EffectAst::Coordination(coordination);
    if leading_duration {
        return crate::grammar::effects::control_flow::wrap_leading_duration_program(
            tokens,
            vec![coordinated.clone()],
        )
        .map_or_else(|| vec![coordinated], |wrapped| vec![wrapped]);
    }
    vec![coordinated]
}

fn target_word_only_qualifies_a_controller(words: &[&str]) -> bool {
    let target_positions = words
        .iter()
        .enumerate()
        .filter_map(|(index, word)| (*word == "target").then_some(index))
        .collect::<Vec<_>>();
    let [target_index] = target_positions.as_slice() else {
        return false;
    };
    words
        .get(*target_index + 1)
        .is_some_and(|word| matches!(*word, "opponent" | "opponents" | "player"))
        && gain_shapes::gain_words_include_control_verb(words)
}

fn parse_shared_subject_base_pt_from_has_tail(
    tokens: &[OwnedLexToken],
    has_word_idx: usize,
    duration: &Until,
) -> Result<Option<SharedSubjectBasePt>, CardTextError> {
    let clause_words = GainAbilityWordView::new(tokens).to_word_refs();
    let Some(rest_words) = clause_words.get(has_word_idx + 1..) else {
        return Ok(None);
    };
    match gain_shapes::parse_gain_base_pt_after_has_shape(rest_words) {
        Ok(Some(shape)) => Ok(Some((
            shape.power,
            shape.toughness,
            has_word_idx,
            duration.clone(),
        ))),
        Ok(None) => Ok(None),
        Err(gain_shapes::GainBasePtShapeError::InvalidValue) => {
            Err(CardTextError::ParseError(format!(
                "invalid base power/toughness value (clause: '{}')",
                clause_words.join(" ")
            )))
        }
        Err(gain_shapes::GainBasePtShapeError::UnsupportedTail) => {
            Err(CardTextError::ParseError(format!(
                "unsupported trailing base power/toughness clause (clause: '{}')",
                clause_words.join(" ")
            )))
        }
    }
}

fn parse_leading_subject_base_pt_before_gain(
    before_gain: &[&str],
    subject_start_word_idx: usize,
    gain_idx: usize,
    duration: &Until,
) -> Result<Option<SharedSubjectBasePt>, CardTextError> {
    let shape = match gain_shapes::parse_leading_gain_base_pt_shape(before_gain) {
        Ok(Some(shape)) => shape,
        Ok(None) => return Ok(None),
        Err(gain_shapes::GainBasePtShapeError::InvalidValue) => {
            return Err(CardTextError::ParseError(format!(
                "invalid base power/toughness value (clause: '{}')",
                before_gain.join(" ")
            )));
        }
        Err(gain_shapes::GainBasePtShapeError::UnsupportedTail) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing base power/toughness clause (clause: '{}')",
                before_gain.join(" ")
            )));
        }
    };
    let has_word_idx = subject_start_word_idx + shape.has_offset;
    if has_word_idx >= gain_idx {
        return Ok(None);
    }
    Ok(Some((
        shape.power,
        shape.toughness,
        has_word_idx,
        duration.clone(),
    )))
}

fn parse_shared_subject_pump_from_get_tail(
    tokens: &[OwnedLexToken],
    get_word_idx: usize,
    duration: &Until,
    has_explicit_duration: bool,
) -> Result<Option<SharedSubjectPump>, CardTextError> {
    let word_view = GainAbilityWordView::new(tokens);
    let Some(modifier_start_token_idx) = word_view.token_index_after_words(get_word_idx + 1) else {
        return Ok(None);
    };
    let modifier_token_storage = trim_commas(&tokens[modifier_start_token_idx..]);
    let modifier_tokens = trim_commas(&modifier_token_storage);
    let Some(head) = gain_shapes::parse_gain_pump_head_shape(&modifier_tokens) else {
        return Ok(None);
    };
    let power = head.power;
    let toughness = head.toughness;
    let additional_modifier = head.modifier_token_offset > 0;
    let modifier_tokens = modifier_tokens
        .get(head.modifier_token_offset..)
        .unwrap_or_default();
    let for_each = if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) =
        (&power, &toughness)
    {
        parse_get_for_each_count_value(modifier_tokens.get(1..).unwrap_or_default())?.map(|count| {
            let count = if additional_modifier {
                count.with_surface_hint(
                    ironsmith_core::ValueSurfaceHint::AdditionalPowerToughnessModifier,
                )
            } else {
                count
            };
            (*power_per, *toughness_per, count)
        })
    } else {
        None
    };
    let has_local_duration = head.has_local_duration;
    let (power, toughness, local_duration, condition) =
        parse_get_modifier_values_with_tail(modifier_tokens, power, toughness)?;
    let pump_duration = if has_explicit_duration || !has_local_duration {
        duration.clone()
    } else {
        local_duration
    };
    Ok(Some((
        power,
        toughness,
        get_word_idx,
        pump_duration,
        condition,
        for_each,
    )))
}

fn parsed_static_granted_abilities(
    _ability_tokens: &[OwnedLexToken],
    abilities: Vec<crate::cards::builders::StaticAbilityAst>,
) -> Result<Vec<GrantedAbilityAst>, CardTextError> {
    Ok(abilities
        .into_iter()
        .map(|ability| GrantedAbilityAst::StaticAbility(Box::new(ability)))
        .collect())
}

fn player_gain_effects_for_abilities(
    abilities: &[GrantedAbilityAst],
    duration: &Until,
    subject_tokens: &[OwnedLexToken],
    player_filter: PlayerFilter,
) -> Option<Vec<EffectAst>> {
    let player_target = TargetAst::Player(
        player_filter.clone(),
        span_from_lexed_tokens(subject_tokens),
    );
    let mut effects = Vec::new();

    for ability in abilities {
        let GrantedAbilityAst::KeywordAction(action) = ability else {
            return None;
        };
        match action.as_ref() {
            KeywordAction::Hexproof => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player_from(
                        player_filter.clone(),
                        ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                    ),
                    duration.clone(),
                    None,
                ));
            }
            KeywordAction::HexproofFrom(filter) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player_from(
                        player_filter.clone(),
                        filter.clone().controlled_by(PlayerFilter::Opponent),
                    ),
                    duration.clone(),
                    None,
                ));
            }
            KeywordAction::Shroud => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player(player_filter.clone()),
                    duration.clone(),
                    None,
                ));
            }
            KeywordAction::ProtectionFromEverything => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player(player_filter.clone()),
                    duration.clone(),
                    None,
                ));
                effects.push(EffectAst::subject_verb_prevent_all_damage_to_target(
                    player_target.clone(),
                    duration.clone(),
                ));
            }
            _ => return None,
        }
    }

    Some(effects)
}

fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    let word_view = GainAbilityWordView::new(tokens);
    word_view.to_word_refs().join(" ")
}

fn push_unique_keyword_action(actions: &mut Vec<KeywordAction>, action: KeywordAction) {
    crate::slice_primitives::push_unique(actions, action);
}

fn color_only_hexproof_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut filters = Vec::new();
    for token in tokens {
        if token
            .as_word()
            .is_some_and(|word| matches!(word, "and" | "from"))
        {
            continue;
        }
        let color = crate::color::Color::from_name(token.as_word()?)?;
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

#[inline(never)]
fn parse_direct_quoted_object_restriction(
    ability_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    if matches!(
        crate::grammar::activation_costs::cant_shapes::parse_direct_cant_fact_tokens(
            ability_tokens
        ),
        Some(
            crate::grammar::activation_costs::cant_shapes::DirectCantFact::SourceCantAttackItsOwner
        )
    ) {
        return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
            StaticAbilityAst::Static(StaticAbility::cant_attack_its_owner()),
        ))]));
    }
    let Some(parsed) = crate::activation_and_restrictions::activation_restriction_clauses::
        parse_negated_object_restriction_clause(ability_tokens)?
    else {
        return Ok(None);
    };
    let display = display_text_for_tokens(ability_tokens);
    Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
        StaticAbilityAst::Static(StaticAbility::restriction(parsed.restriction, display)),
    ))]))
}

use crate::recognition::ParseOutcome;
#[path = "gain_ability/granted_component_readings.rs"]
mod granted_component_readings;

fn parse_granted_ability_component_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let authored_as_quoted_ability = ability_tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
        && ability_tokens
            .last()
            .is_some_and(|token| token.kind == TokenKind::Quote);
    let ability_tokens = trim_edge_punctuation_and_quotes(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }
    let ability_words = crate::lexer::token_word_refs(&ability_tokens);
    let top_level_activated_ability = authored_as_quoted_ability
        && gain_shapes::parse_top_level_activated_ability_surface(&ability_tokens);
    let top_level_triggered_ability = authored_as_quoted_ability
        && ability_tokens.first().is_some_and(|token| {
            token.kind == TokenKind::Word
                && (gain_shapes::gain_word_is_when_intro(token.parser_text())
                    || (gain_shapes::gain_word_is_trigger_intro(token.parser_text())
                        && ability_tokens
                            .get(1)
                            .is_some_and(|next| next.parser_text() == THE_WORD)))
        });
    if crate::word_primitives::parse_any_sequence_complete(
        &ability_words,
        &[
            &["all", "bands", "with", "other", "abilities"],
            &["bands", "with", "other"],
        ],
    ) {
        return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
            StaticAbilityAst::Static(StaticAbility::bands_with_other(
                ObjectFilter::default(),
                "all \"bands with other\" abilities",
            )),
        ))]));
    }
    match gain_shapes::classify_granted_ability_surface(&ability_tokens) {
        gain_shapes::GrantedAbilitySurface::CantBeBlockedExceptByHaste => {
            let restriction = crate::effect::Restriction::block_specific_attacker(
                ObjectFilter::creature().without_static_ability(StaticAbilityId::Haste),
                ObjectFilter::source(),
            );
            return Ok(Some(vec![GrantedAbilityAst::StaticAbility(Box::new(
                StaticAbilityAst::Static(StaticAbility::restriction(
                    restriction,
                    "can't be blocked except by creatures with haste".to_string(),
                )),
            ))]));
        }
        gain_shapes::GrantedAbilitySurface::HexproofFrom { filter_start_token } => {
            let filter_tokens = &ability_tokens[filter_start_token..];
            if let Some(filter) = color_only_hexproof_filter(filter_tokens) {
                return Ok(Some(vec![GrantedAbilityAst::from(
                    KeywordAction::HexproofFrom(filter),
                )]));
            }
            let filter_tokens = filter_tokens.to_vec();
            if !filter_tokens.is_empty()
                && let Ok(filter) = parse_object_filter_lexed(&filter_tokens, false)
            {
                return Ok(Some(vec![GrantedAbilityAst::from(
                    KeywordAction::HexproofFrom(filter),
                )]));
            }
        }
        gain_shapes::GrantedAbilitySurface::Other => {}
    }

    // A quoted ability can itself be a filtered static grant, for example
    // `"Creatures you control have '{T}: Add {R}, {G}, or {W}.'"`. The colon
    // belongs to the nested activated ability, not to the complete quoted
    // rule. Let the typed static-line grammar preserve that outer subject
    // before the ordinary activated-ability probe sees the colon.
    // An outer quoted activation may contain its own apostrophe-quoted token
    // rule. Its colon precedes that inner quote; parse the activation before
    // the static-rule probe can claim the nested token anthem as the whole
    // granted ability. A filtered static grant has its colon inside the
    // apostrophes and keeps the existing static-first route below.
    let input = granted_component_readings::GrantedComponent {
        tokens: &ability_tokens,
        clause_words,
        authored_as_quoted_ability,
        top_level_activated_ability,
        top_level_triggered_ability,
        read_by_cache: Default::default(),
    };
    match granted_component_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }

    Ok(None)
}

fn parse_granted_ability_conjunction_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<Vec<GrantedAbilityAst>>, CardTextError> {
    let segments = split_lexed_slices_on_and(ability_tokens);
    if segments.len() <= 1 {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    for segment in segments {
        let Some(parsed) = parse_granted_ability_component_for_gain(segment, clause_words)? else {
            return Ok(None);
        };
        abilities.extend(parsed);
    }

    Ok((!abilities.is_empty()).then_some(abilities))
}

fn granted_ability_conjunction_is_keyword_list(abilities: &[GrantedAbilityAst]) -> bool {
    !abilities.is_empty()
        && abilities
            .iter()
            .all(|ability| matches!(ability, GrantedAbilityAst::KeywordAction(_)))
}

fn split_quoted_granted_ability_list(tokens: &[OwnedLexToken]) -> Option<Vec<&[OwnedLexToken]>> {
    let open =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Quote)?;
    let close = tokens
        .iter()
        .enumerate()
        .skip(open + 1)
        .find_map(|(index, token)| (token.kind == TokenKind::Quote).then_some(index))?;
    let tail_start = close + 1;
    if tokens.get(tail_start).and_then(OwnedLexToken::as_word) == Some("and") {
        let prefix = trim_lexed_commas(tokens.get(..open)?);
        let quoted = tokens.get(open..=close)?;
        let tail = trim_lexed_commas(tokens.get(tail_start + 1..)?);
        if prefix.is_empty() || quoted.is_empty() || tail.is_empty() {
            return None;
        }
        return Some(vec![prefix, quoted, tail]);
    }

    // The quoted ability may be the final item: `gains haste and "When ..."`
    // or `gains vigilance, indestructible, and "This ..."`. Split at the
    // final top-level conjunction so the ordinary keyword prefix cannot be
    // swallowed by the more permissive quoted-ability parser.
    if !trim_edge_punctuation(tokens.get(tail_start..)?).is_empty() {
        return None;
    }
    let conjunction =
        crate::slice_primitives::select_last_position(tokens.get(..open)?, |token| {
            token.is_word("and")
        })?;
    let prefix = trim_lexed_commas(tokens.get(..conjunction)?);
    let quoted = tokens.get(open..=close)?;
    if prefix.is_empty() || quoted.is_empty() {
        return None;
    }
    Some(vec![prefix, quoted])
}

pub fn parse_granted_abilities_for_gain_clause(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
    allow_choice: bool,
) -> Result<(Vec<GrantedAbilityAst>, bool), CardTextError> {
    if let Some(segments) = split_quoted_granted_ability_list(ability_tokens) {
        let mut abilities = Vec::new();
        for segment in segments {
            let parsed = parse_granted_ability_component_for_gain(segment, clause_words)?;
            let Some(parsed) = parsed else {
                abilities.clear();
                break;
            };
            abilities.extend(parsed);
        }
        if !abilities.is_empty() {
            return Ok((abilities, false));
        }
    }

    if allow_choice && let Some(actions) = parse_choice_of_abilities(ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        return Ok((
            actions.into_iter().map(GrantedAbilityAst::from).collect(),
            true,
        ));
    }

    let comma_segments = split_lexed_slices_on_comma(ability_tokens);
    if comma_segments.len() > 1 {
        // Parse every comma-delimited item independently before trying the
        // whole surface. This keeps an executable keyword in the middle of a
        // mixed list from greedily consuming the list and dropping a later
        // static keyword.
        let mut abilities = Vec::new();
        for comma_segment in &comma_segments {
            // In an Oxford-comma list the final comma-delimited item starts
            // with the coordinating conjunction (`..., annihilator 2, and
            // haste`).  That conjunction is list syntax, not part of the
            // granted ability.  Remove it before dispatching the item so a
            // later keyword cannot be lost when the whole list falls back to
            // a greedier executable-keyword parser.
            let comma_segment = strip_leading_token_words_any(comma_segment, &["and", "and/or"]);

            // Effect-chain normalization can remove the Oxford comma while
            // retaining the final conjunction, producing an item such as
            // `annihilator 2 and haste`. Prefer a fully successful
            // decomposition before the whole-item parser: count keywords are
            // prefix-tolerant and would otherwise accept only `annihilator 2`
            // and silently discard the final keyword. If either arm is not an
            // independent ability (for example, `hexproof from red and
            // green`), fall back to parsing the compound phrase as a whole.
            let conjunction =
                parse_granted_ability_conjunction_for_gain(comma_segment, clause_words)?;
            if let Some(parsed) = conjunction
                .as_ref()
                .filter(|parsed| granted_ability_conjunction_is_keyword_list(parsed))
            {
                abilities.extend(parsed.iter().cloned());
                continue;
            }

            if let Some(parsed) =
                parse_granted_ability_component_for_gain(comma_segment, clause_words)?
            {
                abilities.extend(parsed);
                continue;
            }

            if let Some(parsed) = conjunction {
                abilities.extend(parsed);
                continue;
            }

            abilities.clear();
            break;
        }
        if !abilities.is_empty() {
            return Ok((abilities, false));
        }
    }

    let conjunction = parse_granted_ability_conjunction_for_gain(ability_tokens, clause_words)?;
    if let Some(abilities) = conjunction
        .as_ref()
        .filter(|abilities| granted_ability_conjunction_is_keyword_list(abilities))
    {
        return Ok((abilities.clone(), false));
    }

    if let Some(abilities) = parse_granted_ability_component_for_gain(ability_tokens, clause_words)?
    {
        return Ok((abilities, false));
    }

    if let Some(abilities) = conjunction {
        return Ok((abilities, false));
    }

    Ok((Vec::new(), false))
}

fn token_definition_source_identity(
    definition: &TokenDefinitionSpec,
) -> (String, Vec<CardType>, Vec<crate::types::Subtype>) {
    match definition {
        TokenDefinitionSpec::Creature(creature) => (
            creature.name.clone(),
            creature.card_types.clone(),
            creature.subtypes.clone(),
        ),
        TokenDefinitionSpec::Artifact(artifact) => (
            artifact.name.clone(),
            vec![CardType::Artifact],
            artifact.subtypes.clone(),
        ),
        TokenDefinitionSpec::Vehicle(vehicle) => {
            (vehicle.name.clone(), vec![CardType::Artifact], Vec::new())
        }
        TokenDefinitionSpec::Construct(_) => (
            "Construct".to_string(),
            vec![CardType::Artifact, CardType::Creature],
            Vec::new(),
        ),
        _ => ("Token".to_string(), vec![CardType::Creature], Vec::new()),
    }
}

fn token_rule_is_already_lowered_by_specialized_shape(
    definition: &TokenDefinitionSpec,
    ability_tokens: &[OwnedLexToken],
    token_name: &str,
) -> bool {
    let words = crate::lexer::parser_token_word_refs(ability_tokens);
    let has = |word: &str| crate::word_primitives::sequence_occurs(&words, &[word]);
    let all = |expected: &[&str]| expected.iter().all(|word| has(word));

    if super::super::grammar::token_definitions::parse_embedded_token_rule_tokens(
        ability_tokens,
        Some(token_name),
    )
    .is_some()
    {
        return true;
    }

    if matches!(
        definition,
        TokenDefinitionSpec::Construct(construct) if construct.artifact_scaling.is_some()
    ) && all(&["artifact", "control"])
        && (all(&["gets", "+1/+1", "each"]) || all(&["power", "toughness", "equal", "number"]))
    {
        // The typed Construct blueprint already carries the source rule. A
        // second generic ability would double its power and toughness.
        return true;
    }

    let TokenDefinitionSpec::Creature(creature) = definition else {
        return false;
    };
    let rules = &creature.rules;
    if rules.cumulative_upkeep_mana_symbols.is_some() && all(&["cumulative", "upkeep"])
        || rules.tap_mana_ability.is_some() && all(&["t", "add"])
        || rules.saddle_crew_power_bonus.is_some() && all(&["saddles", "crews"])
        || rules.sacrifice_return.is_some() && all(&["sacrifice", "return", "graveyard"])
        || rules.upkeep_return_name.is_some() && all(&["upkeep", "sacrifice", "return"])
        || rules.dies_create_firebreathing_dragon && all(&["dies", "create", "dragon"])
        || rules.dies_damage_any_target.is_some() && all(&["dies", "damage", "target"])
        || rules.dies_minus_one_target_creature && all(&["dies", "target", "-1/-1"])
        || rules.leaves_damage_you_and_creatures.is_some()
            && all(&["leaves", "damage", "each", "creature"])
        || rules.red_pump && all(&["r", "+1/+0"])
        || rules.white_tap_target_creature && all(&["w", "tap", "target", "creature"])
        || rules.combat_damage_poison && all(&["combat", "damage", "poison"])
        || rules.noncreature_spell_each_opponent_damage.is_some()
            && all(&["noncreature", "spell", "each", "opponent", "damage"])
        || rules.becomes_tapped_damage_player.is_some()
            && all(&["becomes", "tapped", "damage", "player"])
        || rules.combat_damage_gain_artifact && all(&["combat", "damage", "gain", "artifact"])
        || rules.leaves_return_named_to_hand.is_some()
            && all(&["leaves", "return", "named", "hand"])
        || rules.pest_dies_gain_life && all(&["dies", "gain", "life"])
        || rules.can_block_only_flying && all(&["block", "only", "flying"])
        || rules.counter_noncreature_unless_pays
            && all(&["counter", "noncreature", "unless", "pays"])
        || rules.graveyard_anthem_card_name.is_some()
            && all(&["gets", "+1/+1", "graveyard", "named"])
        || rules.landfall_pump && all(&["land", "enters", "+1/+0", "turn"])
    {
        return true;
    }

    if rules.combat_restriction.is_some()
        && (has("attack") || has("attacks") || has("block") || has("blocked"))
    {
        let qualified_blocking_rule = has("by") || all(&["more", "than"]);
        if !qualified_blocking_rule {
            return true;
        }
    }

    false
}

/// Parses rules text that belongs to a token under the token's own source
/// identity. The outer card's source-reference context must not leak into a
/// nested token ability (for example, `this creature` must mean the token).
pub fn parse_granted_abilities_for_token_definition(
    definition: &TokenDefinitionSpec,
    ability_tokens: &[OwnedLexToken],
) -> Result<Vec<GrantedAbilityAst>, CardTextError> {
    let (name, _, _) = token_definition_source_identity(definition);
    // A quoted token ability may carry its normal rules-text label (for
    // example, `Landfall — Whenever ...`). The label is presentation, while
    // the trigger body is the executable ability that the nested parser must
    // see.
    let ability_tokens =
        crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(
            ability_tokens,
        )
        .map_or(ability_tokens, |shape| shape.body_tokens);
    // A mixed `It has <keyword>, "<rule>," and <activation>` sentence is a
    // list of independent abilities.  A compact token-rule probe can match
    // one member (most commonly the trailing equip ability), but treating
    // that partial match as the whole sentence discards the siblings.  Let
    // the complete granted-ability list parser own exactly this authored
    // mixed-pronoun shape; individual specialized token rules retain their
    // normal short-circuit below.
    // Callers pass the ability-list tail after stripping `It has`/`They
    // have`, so recognize the mixed list from its top-level quoted split at
    // this boundary rather than looking for the already-consumed pronoun.
    let mixed_pronoun_list = split_quoted_granted_ability_list(ability_tokens).is_some();
    let specialized = !mixed_pronoun_list
        && token_rule_is_already_lowered_by_specialized_shape(definition, ability_tokens, &name);
    if specialized {
        return Ok(Vec::new());
    }
    let clause_words = crate::lexer::parser_token_word_refs(ability_tokens);
    (|| {
        // A quoted token rule is one ability even when its trigger or
        // activation uses a comma. The general grant-list parser tries
        // comma-delimited items first, which would otherwise feed an
        // incomplete trigger (for example, `Whenever this token blocks a
        // creature`) to triggered-line parsing and discard the rule.
        let starts_triggered_rule = ability_tokens.first().is_some_and(|token| {
            token.parser_word_pieces().first().is_some_and(|word| {
                gain_shapes::gain_word_is_when_intro(&word.text)
                    || (gain_shapes::gain_word_is_trigger_intro(&word.text)
                        && ability_tokens
                            .get(1)
                            .is_some_and(|next| next.parser_text() == THE_WORD))
            })
        });
        if (starts_triggered_rule || contains_token_kind(ability_tokens, TokenKind::Colon))
            && let Some(ability) = parse_granted_activated_or_triggered_ability_for_gain(
                ability_tokens,
                &clause_words,
            )?
        {
            return Ok(vec![ability]);
        }

        // A complete quoted static rule with an explicit filtered subject
        // (for example, `Creatures you control attack each combat if
        // able`) is an ability of the token, not a list of abilities the
        // subject itself "has". Preserve the ordinary typed static-line
        // parse as one granted carrier before the gain-list grammar can
        // reduce the trailing restriction to an intrinsic token keyword.
        if !mixed_pronoun_list
            && let Some(static_abilities) = parse_static_ability_ast_line_lexed(ability_tokens)?
            && !static_abilities.is_empty()
            && static_abilities
                .iter()
                .all(|ability| matches!(ability, StaticAbilityAst::GrantStaticAbility { .. }))
        {
            return Ok(static_abilities
                .into_iter()
                .map(|ability| GrantedAbilityAst::StaticAbility(Box::new(ability)))
                .collect());
        }

        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(ability_tokens, &clause_words, false)?;
        Ok(if is_choice { Vec::new() } else { abilities })
    })()
}

pub fn parse_simple_ability_duration(words_after_verb: &[&str]) -> Option<(usize, usize, Until)> {
    gain_shapes::parse_simple_ability_duration_shape(words_after_verb)
        .map(|shape| (shape.start, shape.len, shape.duration))
}

fn parse_ability_duration_with_condition(
    tokens_after_verb: &[OwnedLexToken],
    words_after_verb: &[&str],
) -> (Option<(usize, usize, Until)>, Option<PredicateAst>) {
    let Some(shape) = gain_shapes::parse_source_tapped_gain_duration_shape(tokens_after_verb)
        .or_else(|| gain_shapes::parse_gain_ability_duration_shape(words_after_verb))
    else {
        return (None, None);
    };
    (
        Some((shape.start, shape.len, shape.duration)),
        shape.condition,
    )
}

fn parse_temporary_escape_grant(
    subject_tokens: &[OwnedLexToken],
    ability_tokens: &[OwnedLexToken],
    duration: &Until,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(method) = crate::util::parse_escape_line_lexed(ability_tokens)? else {
        return Ok(None);
    };
    let grant_duration = match duration {
        Until::Forever => crate::grant::GrantDuration::Forever,
        Until::EndOfTurn => crate::grant::GrantDuration::UntilEndOfTurn,
        Until::YourNextTurn | Until::YourNextTurnEnd => {
            crate::grant::GrantDuration::UntilYourNextTurnEnd
        }
        _ => return Ok(None),
    };
    let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in escape-grant clause (clause: '{}')",
            crate::lexer::token_word_refs(subject_tokens).join(" ")
        ))
    })?;
    let zone = filter.zone.take().unwrap_or(Zone::Graveyard);
    let spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativeCast(method),
        filter,
        zone,
    );
    Ok(Some(EffectAst::subject_verb_grant_by_spec(
        spec,
        PlayerAst::You,
        grant_duration,
    )))
}

/// The players a trailing gain condition can be about.
///
/// The clause only accepts the handful of subjects it can bind; anything else
/// declines. The recognized player passes through unchanged — turning it into a
/// filter is the resolver's job.
fn player_filter_for_gain_condition(player: PlayerAst) -> Option<PlayerAst> {
    match player {
        PlayerAst::Implicit => Some(PlayerAst::You),
        PlayerAst::You
        | PlayerAst::Opponent
        | PlayerAst::Any
        | PlayerAst::That
        | PlayerAst::Target => Some(player),
        _ => None,
    }
}

fn condition_from_gain_trailing_predicate(predicate: PredicateAst) -> Option<PredicateAst> {
    Some(match predicate {
        PredicateAst::Player(PlayerPredicateAst::PlayerControls { player, filter }) => PredicateAst::Player(PlayerPredicateAst::PlayerControls {
            player: player_filter_for_gain_condition(player)?,
            filter,
        }),
        PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast {
            player,
            filter,
            count,
        }) => PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast {
            player: player_filter_for_gain_condition(player)?,
            filter,
            count,
        }),
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        }) => PredicateAst::Player(PlayerPredicateAst::PlayerControlsExactly {
            player: player_filter_for_gain_condition(player)?,
            filter,
            count,
        }),
        PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        }) => PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeastWithDifferentPowers {
            player: player_filter_for_gain_condition(player)?,
            filter,
            count,
        }),
        PredicateAst::And(left, right) => PredicateAst::And(
            Box::new(condition_from_gain_trailing_predicate(*left)?),
            Box::new(condition_from_gain_trailing_predicate(*right)?),
        ),
        PredicateAst::Or(left, right) => PredicateAst::Or(
            Box::new(condition_from_gain_trailing_predicate(*left)?),
            Box::new(condition_from_gain_trailing_predicate(*right)?),
        ),
        PredicateAst::Not(inner) => {
            PredicateAst::Not(Box::new(condition_from_gain_trailing_predicate(*inner)?))
        }
        _ => return None,
    })
}

fn subject_verb_grant_abilities_to_target_with_optional_condition(
    target: TargetAst,
    abilities: Vec<GrantedAbilityAst>,
    duration: Until,
    condition: &Option<PredicateAst>,
) -> EffectAst {
    if let Some(condition) = condition {
        EffectAst::subject_verb_grant_abilities_to_target_with_condition(
            target,
            abilities,
            duration,
            condition.clone(),
        )
    } else {
        EffectAst::subject_verb_grant_abilities_to_target(target, abilities, duration)
    }
}

fn subject_verb_grant_abilities_all_with_optional_condition(
    filter: ObjectFilter,
    abilities: Vec<GrantedAbilityAst>,
    duration: Until,
    condition: &Option<PredicateAst>,
) -> EffectAst {
    if let Some(condition) = condition {
        EffectAst::subject_verb_grant_abilities_all_with_condition(
            filter,
            abilities,
            duration,
            condition.clone(),
        )
    } else {
        EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration)
    }
}

fn tagged_subject_target(tokens: &[OwnedLexToken]) -> TargetAst {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.first() == Some(&"those") {
        let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
        filter.source_surface = Some(SourceReferenceSurface::ThisPermanentType(words.join(" ")));
        TargetAst::Object(filter, None, span_from_tokens(tokens))
    } else {
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span_from_tokens(tokens))
    }
}

fn pronoun_set_quantifier_surface(words: &[&str]) -> Option<ironsmith_core::SetQuantifierSurface> {
    if words.last() == Some(&"each") { return Some(ironsmith_core::SetQuantifierSurface::Each); }
    match words.first().copied() {
        Some("they" | "theyre" | "they're" | "they’re" | "them") => {
            Some(ironsmith_core::SetQuantifierSurface::They)
        }
        Some("all") => Some(ironsmith_core::SetQuantifierSurface::All),
        Some("each") => Some(ironsmith_core::SetQuantifierSurface::Each),
        Some("those") => Some(ironsmith_core::SetQuantifierSurface::Those),
        _ => None,
    }
}

fn subject_verb_remove_abilities_all_with_optional_condition(
    filter: ObjectFilter,
    abilities: Vec<GrantedAbilityAst>,
    duration: Until,
    condition: &Option<PredicateAst>,
) -> EffectAst {
    if let Some(condition) = condition {
        EffectAst::subject_verb_remove_abilities_all_with_condition(
            filter,
            abilities,
            duration,
            condition.clone(),
        )
    } else {
        EffectAst::subject_verb_remove_abilities_all(filter, abilities, duration)
    }
}

/// A conjunction of bare object classes denotes a set union even when one
/// class is a card type and another is a subtype (`creatures and Vehicles`).
/// The ordinary flattened filter representation would make that pair an
/// intersection, so retain independently parsed branches in `any_of`.
fn parse_bare_card_type_subtype_union_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let segments = split_lexed_slices_on_list_conjunction(tokens);
    if segments.len() < 2 {
        return None;
    }

    let mut branches = Vec::with_capacity(segments.len());
    let mut saw_card_type = false;
    let mut saw_subtype = false;
    for segment in segments {
        let words = GainAbilityWordView::new(segment).to_word_refs();
        let bare_words = words
            .iter()
            .copied()
            .skip_while(|word| matches!(*word, "a" | "an" | "all" | "each" | "the"))
            .collect::<Vec<_>>();
        let [kind_word] = bare_words.as_slice() else {
            return None;
        };
        if parse_card_type(kind_word).is_some() {
            saw_card_type = true;
        } else if parse_subtype_flexible(kind_word).is_some() {
            saw_subtype = true;
        } else {
            return None;
        }

        let branch = crate::grammar::primitives::probe_shape(parse_object_filter(segment, false))?;
        if !branch.any_of.is_empty() {
            return None;
        }
        branches.push(branch);
    }

    (saw_card_type && saw_subtype).then_some(ObjectFilter {
        any_of: branches,
        ..ObjectFilter::default()
    })
}

fn words_start_nested_triggered_ability(words_after_verb: &[&str]) -> bool {
    gain_shapes::starts_nested_triggered_ability(words_after_verb)
}

fn quoted_nested_ability_end_and_duration(
    tokens: &[OwnedLexToken],
    gain_token_idx: usize,
) -> Option<(usize, Until)> {
    gain_shapes::parse_quoted_gain_duration_shape(tokens, gain_token_idx)
        .map(|shape| (shape.close_quote_token, shape.duration))
}

fn parse_leading_simple_ability_duration(tokens: &[OwnedLexToken]) -> Option<(usize, Until)> {
    let words = GainAbilityWordView::new(tokens).to_word_refs();
    gain_shapes::parse_leading_gain_duration_shape(&words)
        .map(|shape| (shape.consumed_words, shape.duration))
}

pub fn parse_simple_gain_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, false)
}

pub fn parse_simple_lose_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, true)
}

fn span_from_lexed_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => Some(TextSpan {
            line: first.span.line,
            start: first.span.start,
            end: last.span.end,
        }),
        _ => None,
    }
}

fn trim_trailing_also(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut end = tokens.len();
    while end > 0 && tokens[end - 1].as_word() == Some(ALSO_WORD) {
        end -= 1;
    }
    &tokens[..end]
}

fn source_target_from_subject_tokens(tokens: &[OwnedLexToken]) -> Option<TargetAst> {
    let subject_words = GainAbilityWordView::new(tokens).to_word_refs();
    // A source name can itself be a typed subtype phrase (for example,
    // "Time Lord"). Once the authored subject has explicit target grammar,
    // let the ordinary target parser own it rather than treating that subtype
    // surface as a reference to this source.
    if subject_words.first() == Some(&"target") && parse_target_phrase(tokens).is_ok() {
        return None;
    }
    if crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[&["the", "that"], &["copy"]],
    ) || crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[&["the", "those"], &["copies"]],
    ) {
        return Some(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::CopiedStackObject.bind(),
            span_from_lexed_tokens(tokens),
        ));
    }
    if crate::word_primitives::parse_sequence_complete(
        &subject_words,
        &["the", "creature", "that", "attacked"],
    ) {
        return Some(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::Triggering.bind(),
            span_from_lexed_tokens(tokens),
        ));
    }
    if let Some(surface) = source_reference_surface_for_possessive_words(&subject_words) {
        return Some(TargetAst::Object(
            ObjectFilter::source().with_source_surface(surface),
            None,
            None,
        ));
    }
    for prefix_len in (1..=subject_words.len()).rev() {
        if !is_source_reference_words(&subject_words[..prefix_len]) {
            continue;
        }

        if prefix_len == subject_words.len()
            || find_verb_lexed(&tokens[prefix_len..]).is_some_and(|(_, verb_idx)| verb_idx == 0)
        {
            let surface = source_reference_surface_for_words(&subject_words[..prefix_len])
                .map(|surface| match surface {
                    SourceReferenceSurface::FullName(_) | SourceReferenceSurface::ShortName(_) => {
                        surface
                    }
                    SourceReferenceSurface::ThisPermanentType(_) => surface,
                })
                .or_else(|| this_source_surface_for_words(&subject_words[..prefix_len]));
            if let Some(surface) = surface {
                return Some(TargetAst::Object(
                    ObjectFilter::source().with_source_surface(surface),
                    None,
                    None,
                ));
            }
            return Some(TargetAst::Source(span_from_lexed_tokens(
                &tokens[..prefix_len],
            )));
        }
    }

    None
}

fn named_source_target_from_granted_ability_surface(
    subject_tokens: &[OwnedLexToken],
    ability_tokens: &[OwnedLexToken],
) -> Option<TargetAst> {
    if !is_authored_proper_name_phrase(subject_tokens) {
        return None;
    }
    let ability_words = GainAbilityWordView::new(ability_tokens).to_word_refs();
    let mut permanent_type = None;
    for candidate in [
        "creature",
        "artifact",
        "enchantment",
        "land",
        "planeswalker",
        "battle",
    ] {
        if crate::word_primitives::sequence_occurs(&ability_words, &["this", candidate]) {
            permanent_type = Some(candidate);
            break;
        }
    }
    let permanent_type = permanent_type?;
    Some(TargetAst::Object(
        ObjectFilter::source().with_source_surface(SourceReferenceSurface::ThisPermanentType(
            format!("this {permanent_type}"),
        )),
        None,
        span_from_lexed_tokens(subject_tokens),
    ))
}

fn parse_simple_ability_modifier_clause_lexed(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.first().is_some_and(|token| token.is_any_word(&["if", "unless", "instead"])) {
        return Ok(None);
    }
    if losing
        && let Some(effects) =
            super::chain_carry::parse_return_it_then_loses_all_abilities_lexed(tokens)?
    {
        return Ok(Some(EffectAst::Sequence { effects }));
    }

    let clause_word_view = GainAbilityWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let Some((verb_idx, verb)) = gain_shapes::find_gain_or_lose_verb(&clause_words, losing) else {
        return Ok(None);
    };
    let implied_it_subject = verb_idx == 0;
    let Some(verb_token_idx) = clause_word_view.map_word_or_end_to_token_boundary(verb_idx) else {
        return Ok(None);
    };

    if !losing
        && verb == gain_shapes::GainAbilityVerb::Gain
        && clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| gain_shapes::gain_verb_is_life_or_control_head(word))
    {
        return Ok(None);
    }

    let leading_duration_phrase = parse_leading_simple_ability_duration(tokens);
    let subject_start_token_idx = leading_duration_phrase
        .as_ref()
        .map(|(start_word_idx, _)| {
            clause_word_view
                .map_word_or_end_to_token_boundary(*start_word_idx)
                .unwrap_or(tokens.len())
        })
        .unwrap_or(0);
    if subject_start_token_idx > verb_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_trailing_also(trim_lexed_commas(
        &tokens[subject_start_token_idx..verb_token_idx],
    ));
    if subject_tokens.is_empty() && !implied_it_subject {
        return Ok(None);
    }
    let subject_words = GainAbilityWordView::new(subject_tokens);
    let subject_word_refs = subject_words.to_word_refs();
    if gain_shapes::subject_contains_gain_base_pt(&subject_word_refs) {
        return Ok(parse_gain_ability_sentence(tokens)?.and_then(single_or_sequence_effect));
    }

    if !losing
        && !subject_tokens.is_empty()
        && let Some((subject_verb, _)) = find_verb_lexed(subject_tokens)
        && subject_verb != Verb::Get
    {
        let subject_shape = gain_shapes::classify_gain_subject(&subject_word_refs);
        let target_phrase_with_controller_tail =
            subject_shape.target && subject_shape.controller_tail;
        if !target_phrase_with_controller_tail {
            return Ok(None);
        }
    }

    let words_after_verb = &clause_words[verb_idx + 1..];
    if words_after_verb.is_empty() {
        return Ok(None);
    }

    let duration_phrase = if words_start_nested_triggered_ability(words_after_verb) {
        None
    } else {
        parse_simple_ability_duration(words_after_verb)
    };
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);

    let shared_gain_tail_word_idx = if losing {
        gain_shapes::find_shared_ability_tail(
            words_after_verb,
            gain_shapes::SharedAbilityTail::Gain,
        )
    } else {
        None
    };
    let shared_get_tail_word_idx = if !losing {
        gain_shapes::find_shared_ability_tail(words_after_verb, gain_shapes::SharedAbilityTail::Get)
    } else {
        None
    };
    let shared_has_tail_word_idx = if losing {
        gain_shapes::find_shared_ability_tail(words_after_verb, gain_shapes::SharedAbilityTail::Has)
    } else {
        None
    };
    let ability_end_word_idx = shared_gain_tail_word_idx
        .or(shared_get_tail_word_idx)
        .or(shared_has_tail_word_idx)
        .map(|idx| verb_idx + 1 + idx)
        .unwrap_or_else(|| {
            duration_phrase
                .as_ref()
                .map(|(start, _, _)| verb_idx + 1 + *start)
                .unwrap_or(clause_words.len())
        });
    let ability_end_token_idx = clause_word_view
        .map_word_or_end_to_token_boundary(ability_end_word_idx)
        .unwrap_or(tokens.len());
    let ability_tokens = trim_edge_punctuation(trim_lexed_commas(
        &tokens[verb_token_idx + 1..ability_end_token_idx],
    ));
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if !losing
        && let Some(grant) =
            parse_temporary_escape_grant(subject_tokens, &ability_tokens, &duration)?
    {
        return Ok(Some(grant));
    }

    let ability_word_refs = GainAbilityWordView::new(&ability_tokens).to_word_refs();
    let ability_surface = gain_shapes::classify_ability_reference_surface(&ability_word_refs);
    let (abilities, is_choice) =
        if losing && ability_surface == gain_shapes::AbilityReferenceSurface::ThisAbility {
            (vec![GrantedAbilityAst::ThisAbility], false)
        } else {
            parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, !losing)?
        };
    let removes_all_abilities =
        losing && ability_surface == gain_shapes::AbilityReferenceSurface::AllAbilities;
    if abilities.is_empty() && !removes_all_abilities {
        return Ok(None);
    }
    reject_unsupported_lost_abilities(losing, &abilities)?;

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) =
            clause_word_view.map_word_or_end_to_token_boundary(tail_word_idx)
        {
            let trailing = trim_lexed_commas(&tokens[tail_token_idx..]);
            if !trailing.is_empty() {
                return Ok(None);
            }
        }
    } else if shared_gain_tail_word_idx.is_some() {
        // Shared-subject chains such as "loses all abilities and gains flying"
        // are accepted here for the remove-abilities half. The gain half is
        // handled by higher-level chain parsing when it can be split safely.
    } else if shared_get_tail_word_idx.is_some() {
        // Shared-subject chains such as "gains menace and gets +X/+0"
        // are accepted here for the grant-ability half.
    } else if shared_has_tail_word_idx.is_some() {
        // Shared-subject chains such as "loses all abilities and has base
        // power and toughness 1/1" are accepted here for the remove-abilities
        // half. The characteristic-setting half is handled by chain parsing.
    }

    let subject_shape = gain_shapes::classify_gain_subject(&subject_word_refs);
    let is_pronoun_subject = implied_it_subject || subject_shape.tagged_pronoun;
    if is_pronoun_subject {
        let set_quantifier_surface = pronoun_set_quantifier_surface(&subject_word_refs);
        let target = tagged_subject_target(subject_tokens);
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(if is_choice {
            EffectAst::subject_verb_grant_abilities_choice_to_target(target, abilities, duration)
        } else {
            EffectAst::subject_verb_grant_abilities_to_target(target, abilities, duration)
                .with_set_quantifier_surface(set_quantifier_surface)
        }));
    }

    if let Some(target) = source_target_from_subject_tokens(subject_tokens).or_else(|| {
        named_source_target_from_granted_ability_surface(subject_tokens, &ability_tokens)
    }) {
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                TargetAst::Source(span_from_lexed_tokens(subject_tokens)),
                abilities,
                duration,
            )));
        }
        return Ok(Some(if is_choice {
            EffectAst::subject_verb_grant_abilities_choice_to_target(target, abilities, duration)
        } else {
            EffectAst::subject_verb_grant_abilities_to_target(target, abilities, duration)
        }));
    }

    let is_demonstrative_subject = subject_shape.demonstrative_object;
    if is_demonstrative_subject || subject_shape.target {
        let target = if is_demonstrative_subject {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                span_from_lexed_tokens(subject_tokens),
            )
        } else {
            parse_target_phrase(subject_tokens)?
        };
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(if is_choice {
            EffectAst::subject_verb_grant_abilities_choice_to_target(target, abilities, duration)
        } else {
            EffectAst::subject_verb_grant_abilities_to_target(target, abilities, duration)
                .with_set_quantifier_surface(pronoun_set_quantifier_surface(&subject_word_refs))
        }));
    }

    if !losing && subject_shape.player_any {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            subject_tokens,
            PlayerFilter::Any,
        ) else {
            return Ok(None);
        };
        if player_effects.len() == 1 {
            return Ok(player_effects.pop());
        }
        return Ok(None);
    }

    // "The chosen creature gains ..." names the accumulated chosen set, not
    // a filtered grant over every creature.
    if crate::grammar::targets::parse_chosen_object_target(subject_tokens).is_some() {
        let target = parse_target_phrase(subject_tokens)?;
        if losing {
            return Ok(Some(EffectAst::subject_verb_remove_abilities_from_target(
                target, abilities, duration,
            )));
        }
        return Ok(Some(if is_choice {
            EffectAst::subject_verb_grant_abilities_choice_to_target(target, abilities, duration)
        } else {
            EffectAst::subject_verb_grant_abilities_to_target(target, abilities, duration)
        }));
    }

    let filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in {}-ability clause (clause: '{}')",
            if losing { "lose" } else { "gain" },
            clause_words.join(" ")
        ))
    })?;
    if losing {
        return Ok(Some(EffectAst::subject_verb_remove_abilities_all(
            filter, abilities, duration,
        )));
    }
    Ok(Some(if is_choice {
        EffectAst::subject_verb_grant_abilities_choice_all(filter, abilities, duration)
    } else {
        EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration)
            .with_set_quantifier_surface(pronoun_set_quantifier_surface(&subject_word_refs))
    }))
}

pub fn parse_simple_gain_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, false)
}

pub fn parse_simple_lose_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, true)
}

pub fn parse_simple_ability_modifier_clause(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, losing)
}

pub fn parse_gain_ability_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_ability_sentence_inner(tokens)
}

fn parse_complete_simple_source_gain_ability_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.first().is_some_and(|token| token.is_any_word(&["if", "unless", "instead"])) {
        return Ok(None);
    }
    if gain_shapes::parse_gain_then_get_shape(tokens).is_some()
        || gain_shapes::parse_get_then_ability_shape(tokens).is_some()
    {
        return Ok(None);
    }
    let Some(shape) = gain_shapes::parse_simple_gain_ability_shape(tokens) else {
        return Ok(None);
    };
    if !shape.complete {
        return Ok(None);
    }
    let mut ability_start = 0usize;
    let mut ability_end = shape.ability_tokens.len();
    while ability_start < ability_end
        && matches!(
            shape.ability_tokens[ability_start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        )
    {
        ability_start += 1;
    }
    while ability_end > ability_start
        && matches!(
            shape.ability_tokens[ability_end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        )
    {
        ability_end -= 1;
    }
    let ability_tokens = &shape.ability_tokens[ability_start..ability_end];
    let authored_as_quoted_ability = ability_tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
        && ability_tokens
            .last()
            .is_some_and(|token| token.kind == TokenKind::Quote);
    let subject_tokens = trim_commas(shape.subject_tokens);
    let subject_words = GainAbilityWordView::new(&subject_tokens).to_word_refs();
    let target = if let Some(target) = source_target_from_subject_tokens(&subject_tokens) {
        target
    } else {
        let subject_shape = gain_shapes::classify_gain_subject(&subject_words);
        if !subject_shape.pronoun && !subject_shape.demonstrative_object {
            return Ok(None);
        }
        tagged_subject_target(&subject_tokens)
    };
    let source_target = matches!(&target, TargetAst::Source(_))
        || matches!(&target, TargetAst::Object(filter, None, _) if filter.source);
    if authored_as_quoted_ability && source_target {
        let restriction_tokens = trim_edge_punctuation_and_quotes(ability_tokens);
        if let Some(parsed) = crate::activation_and_restrictions::activation_restriction_clauses::
            parse_negated_object_restriction_clause(&restriction_tokens)?
        {
            return Ok(Some(vec![EffectAst::subject_verb_cant(
                parsed.restriction,
                shape.duration,
                None,
            )]));
        }
    }
    let clause_words = crate::lexer::token_word_refs(tokens);
    let (abilities, is_choice) =
        parse_granted_abilities_for_gain_clause(ability_tokens, &clause_words, true)?;
    if abilities.is_empty() {
        return Ok(None);
    }
    let effect = if is_choice {
        EffectAst::subject_verb_grant_abilities_choice_to_target(target, abilities, shape.duration)
    } else {
        EffectAst::subject_verb_grant_abilities_to_target(target, abilities, shape.duration)
            .with_set_quantifier_surface(pronoun_set_quantifier_surface(&subject_words))
    };
    Ok(Some(vec![effect]))
}

fn parse_gain_ability_sentence_inner(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // `can be the target ... as though` is a targeting permission relation,
    // not a grant of the ability named in the comparison. Keep that complete
    // typed domain outside every gain-ability route.
    if super::clause_dispatch::parse_targeting_as_though_no_ability_spec(tokens)?.is_some() {
        return Ok(None);
    }
    if let Some(effects) = parse_complete_simple_source_gain_ability_sentence(tokens)? {
        return Ok(Some(effects));
    }
    if let Some(player) = super::chain_carry::parse_leading_player_may_lexed(tokens) {
        let mut stripped = super::chain_carry::remove_through_first_word(tokens);
        if let Some(rest) =
            crate::grammar::effects::chain_carry::strip_leading_have_tokens(&stripped)
        {
            stripped = rest.to_vec();
        }
        let Some(mut effects) = parse_gain_ability_sentence(&stripped)? else {
            return Ok(None);
        };
        for effect in &mut effects {
            super::chain_carry::bind_implicit_player_context(effect, player);
        }
        return Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects })]));
    }

    Ok(parse_gain_ability_sentence_with_subject(tokens, None)?
        .map(|effects| coordinated_gain_surface(tokens, effects)))
}

pub fn parse_gain_ability_sentence_with_typed_subject(
    tokens: &[OwnedLexToken],
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        parse_gain_ability_sentence_with_subject(tokens, Some(subject_tokens))?
            .map(|effects| coordinated_gain_surface(tokens, effects)),
    )
}

#[cfg(test)]
#[path = "gain_ability/source_tapped_tests.rs"]
mod source_tapped_tests;

#[cfg(test)]
#[path = "gain_ability/typed_grant_tests.rs"]
mod typed_grant_tests;

#[cfg(test)]
#[path = "gain_ability_inline_tests.rs"]
mod tests;

#[path = "gain_ability/subject_resolution.rs"]
mod subject_resolution;
use subject_resolution::parse_gain_ability_sentence_with_subject;
pub use subject_resolution::parse_gain_ability_to_source_sentence;
#[path = "gain_ability/ability_choices.rs"]
mod ability_choices;
pub use ability_choices::parse_choice_of_abilities;
#[path = "gain_ability/grant_followups.rs"]
mod grant_followups;
pub use grant_followups::append_gain_ability_trailing_effects;
use grant_followups::{
    apply_gain_clause_duration_to_leading_effect,
    parse_single_effect_sentence_for_granted_otherwise,
};
#[path = "gain_ability/triggered_abilities.rs"]
mod triggered_abilities;
pub use triggered_abilities::parse_granted_activated_or_triggered_ability_for_gain;
use triggered_abilities::{
    normalize_named_granted_trigger_subject, parse_granted_trigger_with_nested_token_rule,
    parse_granted_triggered_otherwise_ability,
};
#[path = "gain_ability/ability_validation.rs"]
mod ability_validation;
use ability_validation::reject_unsupported_lost_abilities;
