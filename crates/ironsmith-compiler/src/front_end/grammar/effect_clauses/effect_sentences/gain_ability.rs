use super::super::activation_and_restrictions::parse_single_word_keyword_action;
use super::super::clause_support::{
    parse_static_ability_ast_line_lexed, parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
#[cfg(test)]
use super::super::compile_support::compile_statement_effects;
use super::super::grammar::primitives::{
    TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_list_conjunction,
};
use super::super::grammar::structure::parse_trailing_if_predicate_lexed;
use super::super::lexer::{
    OwnedLexToken, TokenKind, contains_token_kind, locate_token_kind, locate_token_word,
    token_slice_first_is, trim_lexed_commas,
};
use super::super::lowering_support::{
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability as parsed_triggered_ability,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
#[cfg(test)]
use super::super::token_primitives::str_contains as string_contains;
use super::super::token_primitives::strip_leading_if_you_do_lexed;
use super::super::util::{
    is_source_reference_words, parse_card_type, parse_mana_symbol, parse_subtype_flexible,
    parse_target_phrase, preferred_source_reference_self_surface,
    source_reference_surface_for_possessive_words, source_reference_surface_for_words,
    span_from_tokens, strip_leading_token_words_any, this_source_surface_for_words, trim_commas,
};
use super::clause_dispatch::parse_become_clause;
use super::dispatch_inner::trim_edge_punctuation;
use super::lex_chain_helpers::find_verb_lexed;
use super::sentence_helpers::*;
use super::subject_verb_primitives::SubjectVerbPrimitiveClause;
use super::{Verb, find_verb, parse_effect_chain, parse_effect_sentence_lexed};
use crate::ability::Ability;
use crate::cards::builders::{
    COPIED_STACK_OBJECT_TAG, CardTextError, EffectAst, GrantedAbilityAst, IT_TAG,
    IfResultPredicate, KeywordAction, LineAst, ParsedAbility, PlayerAst, PredicateAst,
    ReferenceImports, StaticAbilityAst, SubjectVerbActionAst, SubjectVerbEffectAst, TagKey,
    TargetAst, TextSpan,
};
use crate::effect::{Until, Value};
use crate::mana::ManaCost;
use crate::grammar::clause_support as clause_grammar;
use crate::grammar::effects::gain_ability_shapes as gain_shapes;
use crate::grammar::trigger_surface;
use crate::model::token_definition::TokenDefinitionSpec;
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::target::{ObjectFilter, PlayerFilter, SourceReferenceSurface};
use crate::types::CardType;
use crate::zone::Zone;

type GainAbilityWordView<'a> = TokenWordView<'a>;
type SharedSubjectPump = (
    Value,
    Value,
    usize,
    Until,
    Option<crate::ConditionExpr>,
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
            && edge_count % 2 == 0
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

fn coordinated_gain_surface(tokens: &[OwnedLexToken], effects: Vec<EffectAst>) -> Vec<EffectAst> {
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
    vec![EffectAst::Coordinated {
        effects,
        leading_duration,
        result_conjunction: false,
    }]
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
        parse_get_modifier_values_with_tail(&modifier_tokens, power, toughness)?;
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
    ability_tokens: &[OwnedLexToken],
    abilities: Vec<crate::cards::builders::StaticAbilityAst>,
) -> Result<Vec<GrantedAbilityAst>, CardTextError> {
    let display = display_text_for_tokens(ability_tokens);
    abilities
        .into_iter()
        .map(|ability| {
            let static_ability = rewrite_lower_static_ability_ast(ability)?;
            Ok(GrantedAbilityAst::ParsedObjectAbility {
                ability: ParsedAbility {
                    ability: Ability::static_ability(static_ability).into(),
                    text: Some(display.clone()),
                    effects_ast: None,
                    reference_imports: ReferenceImports::default(),
                    trigger_spec: None,
                },
                display: display.clone(),
            })
        })
        .collect()
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
        match ability {
            GrantedAbilityAst::KeywordAction(KeywordAction::Hexproof) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player_from(
                        player_filter.clone(),
                        ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                    ),
                    duration.clone(),
                    None,
                ));
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::HexproofFrom(filter)) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player_from(
                        player_filter.clone(),
                        filter.clone().controlled_by(PlayerFilter::Opponent),
                    ),
                    duration.clone(),
                    None,
                ));
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::Shroud) => {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_targeted_player(player_filter.clone()),
                    duration.clone(),
                    None,
                ));
            }
            GrantedAbilityAst::KeywordAction(KeywordAction::ProtectionFromEverything) => {
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
    let ability_words = crate::token_word_refs(&ability_tokens);
    if ability_words == ["all", "bands", "with", "other", "abilities"]
        || ability_words == ["bands", "with", "other"]
    {
        return Ok(Some(vec![GrantedAbilityAst::StaticAbility(
            StaticAbility::bands_with_other(
                ObjectFilter::default(),
                "all \"bands with other\" abilities",
            ),
        )]));
    }
    match gain_shapes::classify_granted_ability_surface(&ability_tokens) {
        gain_shapes::GrantedAbilitySurface::CantBeBlockedExceptByHaste => {
            let restriction = crate::effect::Restriction::block_specific_attacker(
                ObjectFilter::creature().without_static_ability(StaticAbilityId::Haste),
                ObjectFilter::source(),
            );
            return Ok(Some(vec![GrantedAbilityAst::StaticAbility(
                StaticAbility::restriction(
                    restriction,
                    "can't be blocked except by creatures with haste".to_string(),
                ),
            )]));
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

    if let Some(granted) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![granted]));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        if authored_as_quoted_ability && matches!(actions.as_slice(), [KeywordAction::Unblockable])
        {
            let restriction = crate::effect::Restriction::block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            );
            return Ok(Some(vec![GrantedAbilityAst::StaticAbility(
                StaticAbility::restriction(
                    restriction,
                    "This creature can't be blocked.".to_string(),
                ),
            )]));
        }
        return Ok(Some(
            actions.into_iter().map(GrantedAbilityAst::from).collect(),
        ));
    }

    if let Some(parsed) =
        crate::families::activation_and_restrictions::parse_equip_line_lexed(
            &ability_tokens,
        )?
    {
        return Ok(Some(vec![GrantedAbilityAst::ParsedObjectAbility {
            display: parsed.text.clone().unwrap_or_else(|| {
                crate::lexer::token_word_refs(&ability_tokens).join(" ")
            }),
            ability: parsed,
        }]));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(parsed_static_granted_abilities(
            &ability_tokens,
            abilities,
        )?));
    }

    if let Some(action) = ability_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .filter(|_| ability_tokens.len() == 1)
        .and_then(parse_single_word_keyword_action)
    {
        return Ok(Some(vec![GrantedAbilityAst::from(action)]));
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

fn split_quoted_granted_ability_list<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<Vec<&'a [OwnedLexToken]>> {
    let open = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Quote)?;
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
    let conjunction = tokens
        .get(..open)?
        .iter()
        .rposition(|token| token.is_word("and"))?;
    let prefix = trim_lexed_commas(tokens.get(..conjunction)?);
    let quoted = tokens.get(open..=close)?;
    if prefix.is_empty() || quoted.is_empty() {
        return None;
    }
    Some(vec![prefix, quoted])
}

pub(crate) fn parse_granted_abilities_for_gain_clause(
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
    let words = crate::token_word_refs(ability_tokens);
    let has = |word: &str| words.iter().any(|candidate| *candidate == word);
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
pub(crate) fn parse_granted_abilities_for_token_definition(
    definition: &TokenDefinitionSpec,
    ability_tokens: &[OwnedLexToken],
) -> Result<Vec<GrantedAbilityAst>, CardTextError> {
    let (name, card_types, subtypes) = token_definition_source_identity(definition);
    // A quoted token ability may carry its normal rules-text label (for
    // example, `Landfall — Whenever ...`). The label is presentation, while
    // the trigger body is the executable ability that the nested parser must
    // see.
    let ability_tokens = crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(ability_tokens)
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
    let clause_words = crate::token_word_refs(ability_tokens);
    crate::util::with_token_source_reference_context(
        &name,
        &card_types,
        &subtypes,
        || {
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
                return static_abilities
                    .into_iter()
                    .map(|ability| {
                        rewrite_lower_static_ability_ast(ability)
                            .map(GrantedAbilityAst::StaticAbility)
                    })
                    .collect();
            }

            let (abilities, is_choice) =
                parse_granted_abilities_for_gain_clause(ability_tokens, &clause_words, false)?;
            Ok(if is_choice { Vec::new() } else { abilities })
        },
    )
}

pub(crate) fn parse_simple_ability_duration(
    words_after_verb: &[&str],
) -> Option<(usize, usize, Until)> {
    gain_shapes::parse_simple_ability_duration_shape(words_after_verb)
        .map(|shape| (shape.start, shape.len, shape.duration))
}

fn parse_ability_duration_with_condition(
    tokens_after_verb: &[OwnedLexToken],
    words_after_verb: &[&str],
) -> (Option<(usize, usize, Until)>, Option<crate::ConditionExpr>) {
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

fn player_filter_for_gain_condition(player: PlayerAst) -> Option<PlayerFilter> {
    Some(match player {
        PlayerAst::Implicit | PlayerAst::You => PlayerFilter::You,
        PlayerAst::Opponent => PlayerFilter::Opponent,
        PlayerAst::Any => PlayerFilter::Any,
        PlayerAst::That => PlayerFilter::IteratedPlayer,
        PlayerAst::Target => PlayerFilter::target_player(),
        _ => return None,
    })
}

fn condition_from_gain_trailing_predicate(predicate: PredicateAst) -> Option<crate::ConditionExpr> {
    Some(match predicate {
        PredicateAst::PlayerControls { player, filter } => crate::ConditionExpr::PlayerControls {
            player: player_filter_for_gain_condition(player)?,
            filter,
        },
        PredicateAst::PlayerHasAtLeast {
            player,
            filter,
            count,
        } => crate::ConditionExpr::PlayerHasAtLeast {
            player: player_filter_for_gain_condition(player)?,
            filter,
            count,
        },
        PredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        } => crate::ConditionExpr::PlayerControlsExactly {
            player: player_filter_for_gain_condition(player)?,
            filter,
            count,
        },
        PredicateAst::PlayerHasAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => crate::ConditionExpr::PlayerHasAtLeastWithDifferentPowers {
            player: player_filter_for_gain_condition(player)?,
            filter,
            count,
        },
        PredicateAst::And(left, right) => crate::ConditionExpr::And(
            Box::new(condition_from_gain_trailing_predicate(*left)?),
            Box::new(condition_from_gain_trailing_predicate(*right)?),
        ),
        PredicateAst::Or(left, right) => crate::ConditionExpr::Or(
            Box::new(condition_from_gain_trailing_predicate(*left)?),
            Box::new(condition_from_gain_trailing_predicate(*right)?),
        ),
        PredicateAst::Not(inner) => {
            crate::ConditionExpr::Not(Box::new(condition_from_gain_trailing_predicate(*inner)?))
        }
        _ => return None,
    })
}

fn subject_verb_grant_abilities_to_target_with_optional_condition(
    target: TargetAst,
    abilities: Vec<GrantedAbilityAst>,
    duration: Until,
    condition: &Option<crate::ConditionExpr>,
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
    condition: &Option<crate::ConditionExpr>,
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

fn pronoun_set_quantifier_surface(words: &[&str]) -> Option<ironsmith_core::SetQuantifierSurface> {
    match words.first().copied() {
        Some("they" | "theyre" | "they're" | "they’re" | "them") => {
            Some(ironsmith_core::SetQuantifierSurface::They)
        }
        _ => None,
    }
}

fn subject_verb_remove_abilities_all_with_optional_condition(
    filter: ObjectFilter,
    abilities: Vec<GrantedAbilityAst>,
    duration: Until,
    condition: &Option<crate::ConditionExpr>,
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

        let branch = parse_object_filter(segment, false).ok()?;
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

pub(crate) fn parse_simple_gain_ability_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause_lexed(
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
    if matches!(
        subject_words.as_slice(),
        ["the" | "that", "copy"] | ["the" | "those", "copies"]
    ) {
        return Some(TargetAst::Tagged(
            TagKey::from(COPIED_STACK_OBJECT_TAG),
            span_from_lexed_tokens(tokens),
        ));
    }
    if subject_words == ["the", "creature", "that", "attacked"] {
        return Some(TargetAst::Tagged(
            TagKey::from("triggering"),
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
                        preferred_source_reference_self_surface().unwrap_or(surface)
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

fn parse_simple_ability_modifier_clause_lexed(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
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
    let Some(verb_token_idx) = clause_word_view.token_boundary_for_word_or_end(verb_idx) else {
        return Ok(None);
    };

    if !losing && verb == gain_shapes::GainAbilityVerb::Gain {
        if clause_words
            .get(verb_idx + 1)
            .is_some_and(|word| gain_shapes::gain_verb_is_life_or_control_head(word))
        {
            return Ok(None);
        }
    }

    let leading_duration_phrase = parse_leading_simple_ability_duration(tokens);
    let subject_start_token_idx = leading_duration_phrase
        .as_ref()
        .map(|(start_word_idx, _)| {
            clause_word_view
                .token_boundary_for_word_or_end(*start_word_idx)
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
        .token_boundary_for_word_or_end(ability_end_word_idx)
        .unwrap_or(tokens.len());
    let ability_tokens = trim_edge_punctuation(trim_lexed_commas(
        &tokens[verb_token_idx + 1..ability_end_token_idx],
    ));
    if ability_tokens.is_empty() {
        return Ok(None);
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
    let abilities = abilities;
    reject_unsupported_lost_abilities(losing, &abilities)?;

    if let Some((start, len, _)) = duration_phrase {
        let tail_word_idx = verb_idx + 1 + start + len;
        if let Some(tail_token_idx) = clause_word_view.token_boundary_for_word_or_end(tail_word_idx)
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
        let target =
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_lexed_tokens(subject_tokens));
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

    if let Some(target) = source_target_from_subject_tokens(&subject_tokens) {
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
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_lexed_tokens(subject_tokens))
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
    if crate::grammar::targets::parse_chosen_object_target(subject_tokens)
        .is_some()
    {
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
    }))
}

pub(crate) fn parse_simple_gain_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, false)
}

pub(crate) fn parse_simple_lose_ability_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause(tokens, true)
}

pub(crate) fn parse_simple_ability_modifier_clause(
    tokens: &[OwnedLexToken],
    losing: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    parse_simple_ability_modifier_clause_lexed(tokens, losing)
}

pub(crate) fn parse_gain_ability_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    {
        parse_gain_ability_sentence_inner(tokens)
    }
}

fn parse_gain_ability_sentence_inner(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(player) = super::chain_carry::parse_leading_player_may_lexed(tokens) {
        let mut stripped = super::chain_carry::remove_through_first_word(tokens);
        if let Some(rest) =
            crate::grammar::effects::chain_carry::
                strip_leading_have_tokens(&stripped)
        {
            stripped = rest.to_vec();
        }
        let Some(mut effects) = parse_gain_ability_sentence(&stripped)? else {
            return Ok(None);
        };
        for effect in &mut effects {
            super::chain_carry::bind_implicit_player_context(effect, player);
        }
        return Ok(Some(vec![EffectAst::MayByPlayer { player, effects }]));
    }

    Ok(parse_gain_ability_sentence_with_subject(tokens, None)?
        .map(|effects| coordinated_gain_surface(tokens, effects)))
}

pub(crate) fn parse_gain_ability_sentence_with_typed_subject(
    tokens: &[OwnedLexToken],
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        parse_gain_ability_sentence_with_subject(tokens, Some(subject_tokens))?
            .map(|effects| coordinated_gain_surface(tokens, effects)),
    )
}

fn parse_gain_ability_sentence_with_subject(
    tokens: &[OwnedLexToken],
    typed_subject_tokens: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let stripped_if_you_do = trim_commas(strip_leading_if_you_do_lexed(tokens));
    if stripped_if_you_do.len() < tokens.len() {
        return Ok(
            parse_gain_ability_sentence(&stripped_if_you_do)?.map(|effects| {
                vec![EffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects,
                }]
            }),
        );
    }

    let word_view = GainAbilityWordView::new(&tokens);
    let word_list = word_view.to_word_refs();
    if gain_shapes::gain_clause_is_defender_as_if_attack(&word_list) {
        return Ok(None);
    }
    let leading_duration_phrase =
        gain_shapes::parse_leading_affected_object_counter_duration_shape(tokens)
            .or_else(|| gain_shapes::parse_leading_gain_duration_shape(&word_list))
            .map(|shape| (shape.consumed_words, shape.duration));
    let subject_start_word_idx = leading_duration_phrase
        .as_ref()
        .map(|(len, _)| *len)
        .unwrap_or(0);
    let Some((relative_gain_idx, gain_verb)) =
        gain_shapes::find_primary_gain_ability_verb(&word_list[subject_start_word_idx..])
    else {
        return Ok(None);
    };
    let gain_idx = subject_start_word_idx + relative_gain_idx;
    let Some(gain_token_idx) = word_view.token_boundary_for_word_or_end(gain_idx) else {
        return Ok(None);
    };
    if let Some((Verb::Create, create_idx)) = find_verb(tokens)
        && create_idx < gain_token_idx
        && gain_shapes::gain_words_include_token_noun(&word_list)
    {
        return Ok(None);
    }
    let losing = gain_verb == gain_shapes::GainAbilityVerb::Lose;

    let after_gain = &word_list[gain_idx + 1..];
    let after_gain_tokens = tokens.get(gain_token_idx + 1..).unwrap_or_default();
    if gain_verb == gain_shapes::GainAbilityVerb::Gain {
        if after_gain
            .first()
            .is_some_and(|word| gain_shapes::gain_verb_is_life_or_control_head(word))
        {
            return Ok(None);
        }
    }

    let subject_start_token_idx = if subject_start_word_idx == 0 {
        0usize
    } else if let Some(idx) = word_view.token_boundary_for_word_or_end(subject_start_word_idx) {
        idx
    } else {
        return Ok(None);
    };
    if subject_start_token_idx < gain_token_idx
        && let Some((subject_verb, _)) = find_verb(&tokens[subject_start_token_idx..gain_token_idx])
        && subject_verb != Verb::Get
    {
        let subject_tokens = trim_commas(&tokens[subject_start_token_idx..gain_token_idx]);
        let subject_words = GainAbilityWordView::new(&subject_tokens);
        let subject_word_refs = subject_words.to_word_refs();
        let subject_shape = gain_shapes::classify_gain_subject(&subject_word_refs);
        let controller_tail_subject = subject_shape.controller_tail;
        let target_phrase_with_controller_tail = subject_shape.target && controller_tail_subject;
        let object_filter_subject = parse_object_filter(&subject_tokens, false).is_ok();
        if !target_phrase_with_controller_tail
            && !controller_tail_subject
            && !object_filter_subject
            && !subject_shape.demonstrative_object
        {
            return Ok(None);
        }
    }

    let nested_quoted_ability = if words_start_nested_triggered_ability(after_gain) {
        quoted_nested_ability_end_and_duration(tokens, gain_token_idx)
    } else {
        None
    };
    let (duration_phrase, mut duration_condition) =
        if words_start_nested_triggered_ability(after_gain) {
            (None, None)
        } else {
            parse_ability_duration_with_condition(after_gain_tokens, after_gain)
        };
    let mut duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            nested_quoted_ability
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);
    let has_explicit_duration = duration_phrase.is_some()
        || nested_quoted_ability.is_some()
        || leading_duration_phrase.as_ref().is_some();

    let shared_get_tail_word_idx = if !losing {
        gain_shapes::find_shared_ability_tail(after_gain, gain_shapes::SharedAbilityTail::Get)
    } else {
        None
    };
    let shared_gain_tail_word_idx = if losing {
        gain_shapes::find_shared_ability_tail(after_gain, gain_shapes::SharedAbilityTail::Gain)
    } else {
        None
    };
    let shared_has_tail_word_idx = if losing {
        gain_shapes::find_shared_ability_tail(after_gain, gain_shapes::SharedAbilityTail::Has)
    } else {
        None
    };
    let following_pump_effect = if let Some(shared_idx) = shared_get_tail_word_idx {
        let get_word_idx = gain_idx + 1 + shared_idx + 1;
        parse_shared_subject_pump_from_get_tail(
            tokens,
            get_word_idx,
            &duration,
            has_explicit_duration,
        )?
    } else {
        None
    };
    if shared_get_tail_word_idx.is_some() && following_pump_effect.is_none() {
        return Ok(None);
    }
    if !has_explicit_duration && let Some((_, _, _, pump_duration, _, _)) = &following_pump_effect {
        // In "gains ... and gets ... until end of turn", the trailing
        // duration scopes both predicates. The pump parser owns that tail,
        // so carry its typed duration back to the preceding ability grant.
        // An explicit duration attached to the gain always wins.
        duration = pump_duration.clone();
    }
    let following_base_pt_effect = if let Some(shared_idx) = shared_has_tail_word_idx {
        let has_word_idx = gain_idx + 1 + shared_idx + 1;
        parse_shared_subject_base_pt_from_has_tail(tokens, has_word_idx, &duration)?
    } else {
        None
    };
    if shared_has_tail_word_idx.is_some() && following_base_pt_effect.is_none() {
        return Ok(None);
    }
    // A shared subject may carry three continuous actions in one clause:
    // "<subject> loses all abilities, becomes ..., and has base P/T ...".
    // Keep the middle `becomes` arm separate from the lost-ability payload so
    // all three actions retain the original grammatical subject.
    let following_become = shared_has_tail_word_idx
        .filter(|_| losing && following_base_pt_effect.is_some())
        .and_then(|has_separator_idx| {
            let become_relative_idx =
                gain_shapes::find_become_verb(&after_gain[..has_separator_idx])?;
            let become_word_idx = gain_idx + 1 + become_relative_idx;
            let tail_start = word_view.token_boundary_for_word_or_end(become_word_idx + 1)?;
            let tail_end = word_view
                .token_boundary_for_word_or_end(gain_idx + 1 + has_separator_idx)
                .unwrap_or(tokens.len());
            let tail = trim_commas(tokens.get(tail_start..tail_end)?);
            (!tail.is_empty()).then(|| (become_word_idx, tail.to_vec()))
        });
    let following_grant = if let Some(shared_idx) = shared_gain_tail_word_idx {
        let ability_start_word_idx = gain_idx + 1 + shared_idx + 2;
        let ability_end_word_idx = duration_phrase
            .as_ref()
            .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel)
            .unwrap_or(word_list.len());
        let Some(ability_start_token_idx) =
            word_view.token_boundary_for_word_or_end(ability_start_word_idx)
        else {
            return Ok(None);
        };
        let ability_end_token_idx = word_view
            .token_boundary_for_word_or_end(ability_end_word_idx)
            .unwrap_or(tokens.len());
        let ability_tokens = trim_commas(
            tokens
                .get(ability_start_token_idx..ability_end_token_idx)
                .unwrap_or_default(),
        );
        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, &word_list, false)?;
        if abilities.is_empty() {
            return Ok(None);
        }
        Some((abilities, is_choice))
    } else {
        None
    };

    let mut trailing_tail_tokens: Vec<OwnedLexToken> = Vec::new();
    if shared_get_tail_word_idx.is_none()
        && let Some((start_rel, len_words, _)) = duration_phrase
    {
        let tail_word_idx = gain_idx + 1 + start_rel + len_words;
        if let Some(tail_token_idx) = word_view.token_boundary_for_word_or_end(tail_word_idx) {
            let trimmed_tail_tokens = trim_commas(&tokens[tail_token_idx..]);
            let tail_tokens =
                strip_leading_token_words_any(&trimmed_tail_tokens, &["and", "then"]).to_vec();
            if !tail_tokens.is_empty() {
                trailing_tail_tokens = tail_tokens;
            }
        }
    }
    if duration_condition.is_none()
        && !trailing_tail_tokens.is_empty()
        && let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing_tail_tokens)
        && let Some(condition) = condition_from_gain_trailing_predicate(predicate)
    {
        duration_condition = Some(condition);
        trailing_tail_tokens.clear();
    }
    let mut grants_must_attack = false;
    if !trailing_tail_tokens.is_empty() {
        let tail_view = GainAbilityWordView::new(&trailing_tail_tokens);
        let mut tail_words = tail_view.to_word_refs();
        if tail_words.first().is_some_and(|word| *word == AND_WORD) {
            tail_words = tail_words[1..].to_vec();
        }
        if gain_shapes::is_must_attack_this_combat_tail(&tail_words) {
            grants_must_attack = true;
            trailing_tail_tokens.clear();
        }
    }

    let ability_end_word_idx = [
        duration_phrase
            .as_ref()
            .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel),
        shared_gain_tail_word_idx.map(|idx| gain_idx + 1 + idx),
        shared_get_tail_word_idx.map(|idx| gain_idx + 1 + idx),
        shared_has_tail_word_idx.map(|idx| gain_idx + 1 + idx),
        following_become
            .as_ref()
            .map(|(become_word_idx, _)| *become_word_idx),
    ]
    .into_iter()
    .flatten()
    .min();
    let ability_end_token_idx = if let Some((close_quote_token_idx, _)) = nested_quoted_ability {
        // This index is used as the exclusive bound below, so retain the
        // closing delimiter. The granted-ability parser can then remove the
        // matching outer quote pair without mistaking it for an unmatched
        // nested-rule delimiter.
        close_quote_token_idx + 1
    } else if let Some(end_word_idx) = ability_end_word_idx {
        word_view
            .token_boundary_for_word_or_end(end_word_idx)
            .unwrap_or(tokens.len())
    } else {
        tokens.len()
    };
    let ability_start_token_idx = gain_token_idx + 1;
    if ability_start_token_idx > ability_end_token_idx || ability_start_token_idx >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_commas(&tokens[ability_start_token_idx..ability_end_token_idx]);

    let (mut abilities, grant_is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &word_list, !losing)?;
    if !trailing_tail_tokens.is_empty() {
        let tail_tokens = strip_leading_token_words_any(&trailing_tail_tokens, &["and", "then"]);
        let (trailing_abilities, trailing_is_choice) =
            parse_granted_abilities_for_gain_clause(tail_tokens, &word_list, false)?;
        if !trailing_abilities.is_empty() && !trailing_is_choice {
            abilities.extend(trailing_abilities);
            trailing_tail_tokens.clear();
        }
    }
    let removes_all_abilities = losing
        && gain_shapes::classify_ability_reference_surface(
            &GainAbilityWordView::new(&ability_tokens).to_word_refs(),
        ) == gain_shapes::AbilityReferenceSurface::AllAbilities;
    if abilities.is_empty() && !grants_must_attack && !removes_all_abilities {
        return Ok(None);
    }
    if grants_must_attack {
        abilities.push(GrantedAbilityAst::MustAttack);
    }
    reject_unsupported_lost_abilities(losing, &abilities)?;

    // Check for "gets +X/+Y and gains/has/loses ..." patterns - if there's a pump
    // modifier before the ability verb, extract it as a separate Pump/PumpAll effect.
    let before_gain = &word_list[subject_start_word_idx..gain_idx];
    let leading_become_subject_end_word_idx = gain_shapes::find_become_verb(before_gain)
        .map(|become_idx| subject_start_word_idx + become_idx);
    let leading_become_effect = if let Some(become_word_idx) = leading_become_subject_end_word_idx {
        let Some(become_token_idx) = word_view.token_boundary_for_word_or_end(become_word_idx)
        else {
            return Ok(None);
        };
        let become_subject_tokens = trim_commas(&tokens[subject_start_token_idx..become_token_idx]);
        let mut become_tail_tokens =
            trim_commas(&tokens[become_token_idx + 1..gain_token_idx]).to_vec();
        while become_tail_tokens.last().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(gain_shapes::gain_word_is_connector)
        }) {
            become_tail_tokens.pop();
        }
        let become_tail_tokens = trim_commas(&become_tail_tokens);
        if become_subject_tokens.is_empty() || become_tail_tokens.is_empty() {
            None
        } else {
            let mut become_effect =
                parse_become_clause(&become_subject_tokens, &become_tail_tokens)?;
            if has_explicit_duration {
                apply_gain_clause_duration_to_leading_effect(&mut become_effect, &duration);
            }
            Some(become_effect)
        }
    } else {
        None
    };
    let get_idx = gain_shapes::find_get_verb(before_gain);
    // Run even when `losing`: cards like Will Kenrith say "...have base power and
    // toughness 0/3 and lose all abilities", where the base P/T precedes the lose
    // clause. The parser returns None when there is no leading base-P/T clause, so
    // ordinary "lose all abilities" lines are unaffected.
    let leading_base_pt_effect = parse_leading_subject_base_pt_before_gain(
        before_gain,
        subject_start_word_idx,
        gain_idx,
        &duration,
    )?;
    let pump_effect = if let Some(gi) = get_idx {
        let modifier_start_word_idx = subject_start_word_idx + gi + 1;
        let Some(modifier_start_token_idx) =
            word_view.token_boundary_for_word_or_end(modifier_start_word_idx)
        else {
            return Ok(None);
        };
        let mut modifier_tokens =
            trim_commas(&tokens[modifier_start_token_idx..gain_token_idx]).to_vec();
        while modifier_tokens.last().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(gain_shapes::gain_word_is_connector)
        }) {
            modifier_tokens.pop();
        }
        let modifier_tokens = trim_commas(&modifier_tokens);
        if let Some(head) = gain_shapes::parse_gain_pump_head_shape(&modifier_tokens) {
            let power = head.power;
            let toughness = head.toughness;
            let additional_modifier = head.modifier_token_offset > 0;
            let modifier_tokens = modifier_tokens
                .get(head.modifier_token_offset..)
                .unwrap_or_default();
            let for_each = if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) =
                (&power, &toughness)
            {
                parse_get_for_each_count_value(modifier_tokens.get(1..).unwrap_or_default())?.map(
                    |count| {
                        let count = if additional_modifier {
                            count.with_surface_hint(
                                ironsmith_core::ValueSurfaceHint::AdditionalPowerToughnessModifier,
                            )
                        } else {
                            count
                        };
                        (*power_per, *toughness_per, count)
                    },
                )
            } else {
                None
            };
            let has_local_duration = head.has_local_duration;
            let (power, toughness, local_duration, condition) =
                parse_get_modifier_values_with_tail(&modifier_tokens, power, toughness)?;
            let pump_duration = if has_explicit_duration || !has_local_duration {
                duration.clone()
            } else {
                local_duration
            };
            let condition = if has_local_duration {
                condition
            } else {
                condition.or_else(|| duration_condition.clone())
            };
            Some((
                power,
                toughness,
                subject_start_word_idx + gi,
                pump_duration,
                condition,
                for_each,
            ))
        } else {
            None
        }
    } else {
        None
    };
    if !losing
        && let Some((power, toughness, _gi, pump_duration, condition, for_each)) = &pump_effect
        && let Some(local_get_idx) = get_idx
        && let Some(and_idx) = gain_shapes::find_gain_and_separator(before_gain, local_get_idx + 1)
        && and_idx + 1 < before_gain.len()
    {
        let source_subject_words = &before_gain[..local_get_idx];
        if gain_shapes::classify_gain_subject(source_subject_words).source_subject {
            let filter_word_start = subject_start_word_idx + and_idx + 1;
            let filter_tokens = word_view
                .token_boundary_for_word_or_end(filter_word_start)
                .map(|filter_token_start| trim_commas(&tokens[filter_token_start..gain_token_idx]));
            if let Some(filter_tokens) = filter_tokens
                && let Ok(filter) = parse_object_filter(&filter_tokens, false)
            {
                let mut effects = Vec::new();
                let source_target = TargetAst::Source(None);
                if let Some((power_per, toughness_per, count)) = for_each {
                    effects.push(EffectAst::subject_verb_pump_for_each(
                        *power_per,
                        *toughness_per,
                        source_target,
                        count.clone(),
                        pump_duration.clone(),
                    ));
                } else {
                    effects.push(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        source_target,
                        pump_duration.clone(),
                        condition.clone(),
                    ));
                }
                if grant_is_choice {
                    effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
                        filter, abilities, duration,
                    ));
                } else {
                    effects.push(EffectAst::subject_verb_grant_abilities_all(
                        filter, abilities, duration,
                    ));
                }
                effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
                return Ok(Some(effects));
            }
        }
    }
    let has_have_verb = gain_verb == gain_shapes::GainAbilityVerb::Has;
    let has_nested_granted_ability = abilities
        .iter()
        .any(|ability| matches!(ability, GrantedAbilityAst::ParsedObjectAbility { .. }));
    if has_have_verb
        && pump_effect.is_none()
        && !has_explicit_duration
        && !has_nested_granted_ability
    {
        return Ok(None);
    }

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end_word_idx = pump_effect
        .as_ref()
        .map(|(_, _, gi, _, _, _)| *gi)
        .or(leading_base_pt_effect
            .as_ref()
            .map(|(_, _, has_idx, _)| *has_idx))
        .or(leading_become_subject_end_word_idx)
        .unwrap_or(gain_idx);
    let real_subject_start_word_idx = if let Some(gi) = get_idx {
        subject_start_word_idx + gain_shapes::find_gain_real_subject_start(before_gain, gi)
    } else {
        subject_start_word_idx
    };
    let real_subject_start_token_idx = word_view
        .token_boundary_for_word_or_end(real_subject_start_word_idx)
        .unwrap_or(subject_start_token_idx);
    let real_subject_end_token_idx = word_view
        .token_boundary_for_word_or_end(real_subject_end_word_idx)
        .unwrap_or(gain_token_idx);
    if typed_subject_tokens.is_none() && real_subject_start_token_idx >= real_subject_end_token_idx
    {
        return Ok(None);
    }
    let inferred_subject_tokens = tokens
        .get(real_subject_start_token_idx..real_subject_end_token_idx)
        .unwrap_or_default();
    let real_subject_token_storage =
        trim_commas(typed_subject_tokens.unwrap_or(inferred_subject_tokens));
    let real_subject_tokens = trim_trailing_also(&real_subject_token_storage);
    let following_become_effect = if let Some((_, become_tail_tokens)) = &following_become {
        let mut effect = parse_become_clause(&real_subject_tokens, become_tail_tokens)?;
        if has_explicit_duration {
            apply_gain_clause_duration_to_leading_effect(&mut effect, &duration);
        }
        Some(effect)
    } else {
        None
    };

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_word_view = GainAbilityWordView::new(&real_subject_tokens);
    let real_subject_words = real_subject_word_view.to_word_refs();
    let real_subject_shape = gain_shapes::classify_gain_subject(&real_subject_words);
    let pronoun_set_quantifier_surface = pronoun_set_quantifier_surface(&real_subject_words);
    let target_word_qualifies_controller =
        target_word_only_qualifies_a_controller(&real_subject_words);

    // The typed get-then-gain shape owns the complete subject capture. Resolve an
    // explicit target before considering references embedded inside that target
    // (for example, "other than this creature" or "with a sticker on it").
    if real_subject_shape.target && !target_word_qualifies_controller {
        let has_preceding_target_effect = pump_effect.is_some() || leading_become_effect.is_some();
        let declares_shared_target =
            !has_preceding_target_effect && following_pump_effect.is_some();
        let target = parse_target_phrase(&real_subject_tokens)?;
        if declares_shared_target {
            // A gain-then-get clause has one authored target shared by both
            // continuous actions. Declare that target once, then compile both
            // consumers through the target prelude's durable `it` alias.
            // Repeating the explicit TargetAst on each child creates two
            // independently assignable target slots at cast time.
            effects.push(EffectAst::subject_verb_target_only(target.clone()));
        }
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        let grant_target = if has_preceding_target_effect || declares_shared_target {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens))
        } else {
            target.clone()
        };
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                grant_target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                grant_target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    grant_target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(
            &mut effects,
            &grant_target,
            &following_grant,
            &duration,
        );
        let following_pump_target = if has_preceding_target_effect || declares_shared_target {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens))
        } else {
            // A single-action target grant keeps its ordinary direct target.
            target
        };
        append_shared_subject_pump_to_target(
            &mut effects,
            &following_pump_target,
            &following_pump_effect,
        );
        append_shared_subject_base_pt_to_target(
            &mut effects,
            &following_pump_target,
            &following_base_pt_effect,
        );
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_pronoun_subject = real_subject_shape.pronoun;
    if is_pronoun_subject {
        let span = span_from_tokens(&real_subject_tokens);
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span);
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                )
                .with_set_quantifier_surface(pronoun_set_quantifier_surface),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(&mut effects, &target, &following_grant, &duration);
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if let Some(target) = source_target_from_subject_tokens(&real_subject_tokens) {
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(&mut effects, &target, &following_grant, &duration);
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_demonstrative_subject = real_subject_shape.demonstrative_object;
    if is_demonstrative_subject {
        let target =
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&real_subject_tokens));
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(&mut effects, &target, &following_grant, &duration);
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_shape.player_you {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            &real_subject_tokens,
            PlayerFilter::You,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_shape.you_and_permanents {
        let permanent_filter = crate::target::ObjectFilter::permanent().you_control();
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            &real_subject_tokens,
            PlayerFilter::You,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed player/permanent gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects.push(EffectAst::subject_verb_grant_abilities_all(
            permanent_filter,
            abilities,
            duration,
        ));
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_shape.player_any {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            &real_subject_tokens,
            PlayerFilter::Any,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    // "The chosen creature gains ..." names the accumulated chosen set, not
    // a filtered grant over every creature.
    if leading_become_effect.is_none()
        && leading_base_pt_effect.is_none()
        && pump_effect.is_none()
        && following_grant.is_none()
        && crate::grammar::targets::parse_chosen_object_target(
            &real_subject_tokens,
        )
        .is_some()
    {
        let target = parse_target_phrase(&real_subject_tokens)?;
        let mut effects = effects;
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target,
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let filter =
        if let Some(filter) = parse_bare_card_type_subtype_union_filter(&real_subject_tokens) {
            filter
        } else {
            parse_object_filter(&real_subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported subject in {}-ability clause (clause: '{}')",
                    if losing { "lose" } else { "gain" },
                    word_list.join(" ")
                ))
            })?
        };

    if let Some(become_effect) = &leading_become_effect {
        effects.push(become_effect.clone());
    }
    if let Some((power, toughness, _has_idx, base_pt_duration)) = &leading_base_pt_effect {
        effects.push(EffectAst::subject_verb_set_base_power_toughness(
            power.clone(),
            toughness.clone(),
            TargetAst::Object(filter.clone(), None, None),
            base_pt_duration.clone(),
        ));
    }
    if let Some((power, toughness, _, pump_duration, _condition, _for_each)) = pump_effect {
        effects.push(EffectAst::subject_verb_pump_all(
            filter.clone(),
            power,
            toughness,
            pump_duration,
        ));
    }
    if losing {
        let mut remove = subject_verb_remove_abilities_all_with_optional_condition(
            filter.clone(),
            abilities,
            duration.clone(),
            &duration_condition,
        );
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RemoveAbilitiesAll {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = &mut remove
        {
            *set_quantifier_surface = match real_subject_tokens.first() {
                Some(token) if token.is_word("all") => {
                    Some(ironsmith_core::SetQuantifierSurface::All)
                }
                Some(token) if token.is_word("each") => {
                    Some(ironsmith_core::SetQuantifierSurface::Each)
                }
                Some(token) if token.is_word("those") => {
                    Some(ironsmith_core::SetQuantifierSurface::Those)
                }
                _ => None,
            };
        }
        effects.push(remove);
    } else if grant_is_choice {
        effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
            filter.clone(),
            abilities,
            duration.clone(),
        ));
    } else {
        effects.push(subject_verb_grant_abilities_all_with_optional_condition(
            filter.clone(),
            abilities,
            duration.clone(),
            &duration_condition,
        ));
    }
    if let Some(become_effect) = &following_become_effect {
        effects.push(become_effect.clone());
    }
    if let Some((abilities, is_choice)) = &following_grant {
        if *is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
                filter.clone(),
                abilities.clone(),
                duration.clone(),
            ));
        } else {
            effects.push(EffectAst::subject_verb_grant_abilities_all(
                filter.clone(),
                abilities.clone(),
                duration.clone(),
            ));
        }
    }
    if let Some((power, toughness, _, pump_duration, _condition, _for_each)) = following_pump_effect
    {
        effects.push(EffectAst::subject_verb_pump_all(
            filter.clone(),
            power,
            toughness,
            pump_duration,
        ));
    }
    if let Some((power, toughness, _, base_pt_duration)) = following_base_pt_effect {
        effects.push(EffectAst::subject_verb_set_base_power_toughness(
            power,
            toughness,
            TargetAst::Object(filter.clone(), None, None),
            base_pt_duration,
        ));
    }
    effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;

    Ok(Some(effects))
}

fn reject_unsupported_lost_abilities(
    losing: bool,
    abilities: &[GrantedAbilityAst],
) -> Result<(), CardTextError> {
    if !losing {
        return Ok(());
    }
    if abilities.iter().any(|ability| {
        matches!(
            ability,
            GrantedAbilityAst::KeywordAction(KeywordAction::Soulbond)
        )
    }) {
        return Err(CardTextError::ParseError(
            "removing soulbond requires non-marker semantics".to_string(),
        ));
    }
    Ok(())
}

fn apply_gain_clause_duration_to_leading_effect(effect: &mut EffectAst, duration: &Until) {
    match effect {
        EffectAst::Sequence { effects } => {
            for child in effects {
                apply_gain_clause_duration_to_leading_effect(child, duration);
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveSubtypes {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::SetColors {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                }
                | SubjectVerbActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                },
            ..
        }) => {
            *effect_duration = duration.clone();
        }
        _ => {}
    }
}

fn parse_granted_trigger_with_nested_token_rule(
    ability_tokens: &[OwnedLexToken],
    display: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_intro = clause_grammar::parse_trigger_intro_tokens(ability_tokens);
    let start_idx = trigger_intro.body_first;
    let Some(split_idx) =
        clause_grammar::parse_trigger_delimiters_tokens(ability_tokens).first_comma
    else {
        return Ok(None);
    };
    if split_idx <= start_idx || split_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }

    let trigger_tokens = &ability_tokens[start_idx..split_idx];
    let effect_tokens = trim_lexed_commas(&ability_tokens[split_idx + 1..]);
    let stripped_effect_tokens = strip_embedded_token_rules_text(&effect_tokens);
    if stripped_effect_tokens.as_slice() == effect_tokens {
        return Ok(None);
    }

    // Only claim this boundary when both ordinary typed parsers succeed.
    // Otherwise the complete triggered-line grammar retains first refusal for
    // complex trigger clauses.
    let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) else {
        return Ok(None);
    };
    let Ok(mut effects) = super::parse_effect_sentences_lexed(&stripped_effect_tokens) else {
        return Ok(None);
    };
    if !super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
        &mut effects,
        &effect_tokens,
    ) {
        return Ok(None);
    }

    Ok(Some(parsed_triggered_ability(
        trigger,
        effects,
        vec![Zone::Battlefield],
        Some(display.to_string()),
        trigger_surface::parse_trigger_frequency_condition_tokens(ability_tokens, None),
        None,
        ReferenceImports::default(),
    )))
}

pub(crate) fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<GrantedAbilityAst>, CardTextError> {
    let ability_tokens = trim_edge_punctuation_and_quotes(ability_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let has_colon = contains_token_kind(&ability_tokens, TokenKind::Colon);
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.kind == TokenKind::Word
            && (gain_shapes::gain_word_is_when_intro(token.parser_text())
                || (gain_shapes::gain_word_is_trigger_intro(token.parser_text())
                    && ability_tokens
                        .get(1)
                        .is_some_and(|next| next.parser_text() == THE_WORD)))
    });
    if !has_colon && !looks_like_trigger {
        return Ok(None);
    }

    let display = display_text_for_tokens(&ability_tokens);
    // Nested quoted rules use apostrophes when their enclosing granted
    // ability is already double-quoted. Normalize those standalone delimiter
    // tokens for semantic parsing so sentence splitting treats punctuation
    // inside the nested activation as part of that rule. Possessives remain
    // ordinary word tokens and are unaffected.
    let semantic_tokens = ability_tokens
        .iter()
        .map(|token| {
            if token.kind == TokenKind::Apostrophe {
                OwnedLexToken::new(TokenKind::Quote, "\"", token.span())
            } else {
                token.clone()
            }
        })
        .collect::<Vec<_>>();
    // An activated ability nested inside a triggered ability can contribute a
    // colon to the full token stream. The leading grammatical shape owns the
    // outer ability kind; only use a colon to select activation when the
    // ability itself does not begin with a trigger.
    let mut parsed_ability = if looks_like_trigger {
        if let Some(parsed) =
            parse_granted_trigger_with_nested_token_rule(&semantic_tokens, &display)?
        {
            parsed
        } else if let Some(parsed) =
            parse_granted_triggered_otherwise_ability(&semantic_tokens, &display)?
        {
            parsed
        } else {
            match parse_triggered_line_lexed(&semantic_tokens)? {
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                } => parsed_triggered_ability(
                    trigger,
                    effects,
                    vec![Zone::Battlefield],
                    Some(display.clone()),
                    trigger_surface::parse_trigger_frequency_condition_tokens(
                        &semantic_tokens,
                        max_triggers_per_turn,
                    ),
                    None,
                    ReferenceImports::default(),
                ),
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported granted activated/triggered ability clause (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
            }
        }
    } else {
        let Some(parsed) = parse_activated_line(&semantic_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        parsed
    };

    // A generic quoted token ability can use the token's authored name as its
    // trigger subject (`When Ember dies, ...`). That route parses a complete
    // typed zone-change trigger, but unlike the ordinary triggered-line CST
    // handoff it can arrive without the leading trigger presentation. Carry
    // only the explicit first-word intro onto that already-typed trigger;
    // this keeps `When` distinct from `Whenever` without inferring frequency
    // from the matched event.
    if let crate::ability::AbilityKind::Triggered(triggered) = parsed_ability.kind_mut()
        && triggered.trigger.intro_surface.is_none()
        && let Some(intro_surface) =
            ability_tokens
                .first()
                .and_then(|token| match token.parser_text() {
                    "when" => Some(crate::triggers::TriggerIntroSurface::When),
                    "whenever" => Some(crate::triggers::TriggerIntroSurface::Whenever),
                    "at" => Some(crate::triggers::TriggerIntroSurface::At),
                    _ => None,
                })
    {
        triggered.trigger.intro_surface = Some(intro_surface);
    }

    Ok(Some(GrantedAbilityAst::ParsedObjectAbility {
        ability: parsed_ability,
        display,
    }))
}

fn parse_granted_triggered_otherwise_ability(
    ability_tokens: &[OwnedLexToken],
    display: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let start_idx = if ability_tokens
        .first()
        .is_some_and(|token| gain_shapes::gain_word_is_trigger_intro(token.parser_text()))
    {
        1
    } else {
        0
    };
    let Some(comma_idx) = locate_token_kind(ability_tokens, TokenKind::Comma) else {
        return Ok(None);
    };
    let Some(otherwise_idx) = locate_token_word(ability_tokens, "otherwise") else {
        return Ok(None);
    };
    if otherwise_idx <= comma_idx + 1 || comma_idx <= start_idx {
        return Ok(None);
    }

    let trigger = parse_trigger_clause_lexed(&ability_tokens[start_idx..comma_idx])?;
    let true_tokens = trim_edge_punctuation(trim_lexed_commas(
        &ability_tokens[comma_idx + 1..otherwise_idx],
    ));
    let false_tokens =
        trim_edge_punctuation(trim_lexed_commas(&ability_tokens[otherwise_idx + 1..]));
    if true_tokens.is_empty() || false_tokens.is_empty() {
        return Ok(None);
    }

    let true_effect = parse_single_effect_sentence_for_granted_otherwise(&true_tokens)?;
    let mut conditional = match true_effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } if if_false.is_empty() => EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
        EffectAst::TrailingIf { predicate, effects } => EffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        },
        _ => return Ok(None),
    };
    if let EffectAst::Conditional { if_false, .. } = &mut conditional {
        *if_false = vec![parse_single_effect_sentence_for_granted_otherwise(
            &false_tokens,
        )?];
    }

    Ok(Some(parsed_triggered_ability(
        trigger,
        vec![conditional],
        vec![Zone::Battlefield],
        Some(display.to_string()),
        None,
        None,
        ReferenceImports::default(),
    )))
}

fn parse_single_effect_sentence_for_granted_otherwise(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let mut effects = parse_effect_sentence_lexed(tokens)?;
    match effects.len() {
        0 => Err(CardTextError::ParseError(
            "empty otherwise branch in granted triggered ability".to_string(),
        )),
        1 => Ok(effects.remove(0)),
        _ => Ok(EffectAst::Sequence { effects }),
    }
}

pub(crate) fn append_gain_ability_trailing_effects(
    mut effects: Vec<EffectAst>,
    trailing_tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if trailing_tokens.is_empty() {
        return Ok(effects);
    }

    let trimmed = trim_commas(trailing_tokens);
    if let Some(predicate) = parse_trailing_if_predicate_lexed(&trimmed) {
        return Ok(vec![EffectAst::Conditional {
            predicate,
            if_true: effects,
            if_false: Vec::new(),
        }]);
    }

    if token_slice_first_is(&trimmed, "unless") {
        if let Some(unless_effect) =
            try_build_unless(effects, SubjectVerbPrimitiveClause::new(&trimmed), 0)?
        {
            return Ok(vec![unless_effect]);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing unless gain-ability clause (clause: '{}')",
            render_lower_words(&trimmed)
        )));
    }

    if let Ok(parsed_tail) = parse_effect_chain(&trimmed)
        && !parsed_tail.is_empty()
    {
        effects.extend(parsed_tail);
    }
    Ok(effects)
}

pub(crate) fn parse_choice_of_abilities(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let shape = gain_shapes::parse_ability_choice_shape(tokens)?;
    let mut actions = Vec::new();
    for segment in shape.options {
        let action = parse_ability_phrase(segment)?;
        push_unique_keyword_action(&mut actions, action);
    }

    if actions.len() < 2 {
        return None;
    }
    Some(actions)
}

pub(crate) fn parse_gain_ability_to_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = gain_shapes::parse_source_gain_ability_shape(tokens) else {
        return Ok(None);
    };
    let ability_tokens = trim_edge_punctuation(shape.ability_tokens);
    if let Some(parsed) = parse_activated_line(&ability_tokens)? {
        return Ok(Some(EffectAst::subject_verb_grant_ability_to_source(
            parsed,
            shape.duration,
        )));
    }

    Ok(None)
}

#[cfg(test)]
#[path = "gain_ability/source_tapped_tests.rs"]
mod source_tapped_tests;

#[cfg(test)]
#[path = "gain_ability/typed_grant_tests.rs"]
mod typed_grant_tests;

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::super::super::util::tokenize_line;
    use super::*;
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::{CardId, ChoiceCount};

    #[test]
    fn quoted_filtered_static_rule_remains_an_ability_of_the_token() {
        let definition = crate::runtime_backend::front_end::grammar::token_definitions::
            parse_token_definition_shape_text("1/1 red Pirate creature token")
            .expect("Pirate token definition");
        let tokens = lex_line("Creatures you control attack each combat if able.", 0)
            .expect("filtered quoted rule should lex");
        let parsed = parse_granted_abilities_for_token_definition(&definition, &tokens)
            .expect("filtered quoted rule should parse under the token identity");
        let [GrantedAbilityAst::StaticAbility(ability)] = parsed.as_slice() else {
            panic!("expected one filtered static carrier: {parsed:#?}");
        };
        let crate::static_abilities::StaticAbilityPayload::GrantAbility(grant) = &ability.payload
        else {
            panic!("expected a filtered object-ability grant: {ability:#?}");
        };
        assert_eq!(grant.filter.card_types, [CardType::Creature]);
        assert_eq!(grant.filter.controller, Some(PlayerFilter::You));
        assert!(format!("{:#?}", grant.ability).contains("MustAttack"));
    }

    #[test]
    fn triggered_grant_display_keeps_fixed_numbers_out_of_mana_braces() {
        let tokens = lex_line(
            "whenever this creature dies, each opponent loses 1 life and you gain 2 life",
            0,
        )
        .expect("granted trigger should lex");

        assert_eq!(
            display_text_for_tokens(&tokens),
            "whenever this creature dies, each opponent loses 1 life and you gain 2 life"
        );
    }

    #[test]
    fn leading_trigger_wins_over_colon_inside_nested_token_ability() {
        let tokens = lex_line(
            "When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'",
            0,
        )
        .expect("nested token trigger should lex");
        let words = crate::runtime_backend::token_word_refs(&tokens);
        let parsed = crate::runtime_backend::util::with_token_source_reference_context(
            "Dragon Egg",
            &[crate::types::CardType::Creature],
            &[crate::types::Subtype::Dragon],
            || parse_granted_activated_or_triggered_ability_for_gain(&tokens, &words),
        )
        .expect("nested token trigger should parse")
        .expect("nested token trigger should produce an ability");
        let GrantedAbilityAst::ParsedObjectAbility { ability, .. } = parsed else {
            panic!("expected a parsed object ability");
        };
        assert!(
            matches!(ability.kind(), AbilityKind::Triggered(_)),
            "the nested activation must not become the outer ability: {ability:#?}"
        );
    }

    #[test]
    fn named_quoted_token_death_trigger_keeps_authored_when_surface() {
        for (intro, expected) in [
            ("When", crate::triggers::TriggerIntroSurface::When),
            ("Whenever", crate::triggers::TriggerIntroSurface::Whenever),
        ] {
            let tokens = lex_line(
                &format!("{intro} Ember dies, create fourteen Treasure tokens."),
                0,
            )
            .expect("named token death trigger should lex");
            let words = crate::runtime_backend::token_word_refs(&tokens);
            let parsed = crate::runtime_backend::util::with_token_source_reference_context(
                "Ember",
                &[crate::types::CardType::Creature],
                &[crate::types::Subtype::Dragon],
                || parse_granted_activated_or_triggered_ability_for_gain(&tokens, &words),
            )
            .expect("named token death trigger should parse")
            .expect("named token death trigger should produce an ability");
            let GrantedAbilityAst::ParsedObjectAbility { ability, .. } = parsed else {
                panic!("expected a parsed object ability");
            };
            let AbilityKind::Triggered(triggered) = ability.kind() else {
                panic!("expected a triggered token ability: {ability:#?}");
            };
            assert_eq!(triggered.trigger.intro_surface, Some(expected));
        }
    }

    #[test]
    fn edge_trimming_preserves_nested_rules_closing_quote() {
        for text in [
            "When this token dies, create a token with '{R}: This token gets +1/+0 until end of turn.'",
            "When this token dies, create a token with \"{R}: This token gets +1/+0 until end of turn.\"",
        ] {
            let tokens = lex_line(text, 0).expect("nested token rule should lex");
            let trimmed = trim_edge_punctuation_and_quotes(&tokens);
            let quote_count = trimmed
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Apostrophe))
                .count();

            assert_eq!(quote_count, 2, "{trimmed:#?}");
            assert!(
                trimmed.last().is_some_and(|token| matches!(
                    token.kind,
                    TokenKind::Quote | TokenKind::Apostrophe
                )),
                "{trimmed:#?}"
            );
        }
    }

    #[test]
    fn quoted_mixed_ability_list_splits_only_at_top_level_separators() {
        let ability_tokens = lex_line(
            "indestructible, \"Equipped creature gets +5/+5 and has double strike,\" and equip {0}.",
            0,
        )
        .expect("mixed granted-ability list should lex");
        let clause_words = crate::runtime_backend::token_word_refs(&ability_tokens);
        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)
                .expect("mixed granted-ability list should parse");
        assert!(!is_choice);
        let debug = format!("{abilities:#?}");
        assert!(debug.contains("Indestructible"), "{debug}");
        assert!(
            debug
                .to_ascii_lowercase()
                .contains("equipped creature gets +5/+5 and has double strike"),
            "{debug}"
        );
        assert!(debug.contains("Equip {0}"), "{debug}");
    }

    #[test]
    fn oxford_list_with_final_quoted_ability_is_not_a_choice() {
        let ability_tokens = lex_line(
            "vigilance, indestructible, and \"This creature can't be blocked.\"",
            0,
        )
        .expect("mixed granted-ability list should lex");
        let clause_words = crate::runtime_backend::token_word_refs(&ability_tokens);
        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, true)
                .expect("mixed granted-ability list should parse");

        assert!(!is_choice, "{abilities:#?}");
        assert_eq!(abilities.len(), 3, "{abilities:#?}");
        assert!(matches!(
            &abilities[2],
            GrantedAbilityAst::StaticAbility(ability)
                if ability.id() == StaticAbilityId::RuleRestriction
        ));
    }

    #[test]
    fn keyword_before_final_quoted_ability_is_preserved() {
        let ability_tokens = lex_line("trample and \"{G}: Regenerate this creature.\"", 0)
            .expect("mixed granted-ability list should lex");
        let clause_words = crate::runtime_backend::token_word_refs(&ability_tokens);
        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, true)
                .expect("mixed granted-ability list should parse");
        let debug = format!("{abilities:#?}");

        assert!(!is_choice, "{debug}");
        assert_eq!(abilities.len(), 2, "{debug}");
        assert!(debug.contains("Trample"), "{debug}");
        assert!(debug.contains("Regenerate"), "{debug}");
    }

    #[test]
    fn become_then_oxford_grant_list_keeps_all_grants_nonmodal() {
        let tokens = tokenize_line(
            "Target artifact you control becomes a 9/9 Construct artifact creature and gains vigilance, indestructible, and \"This creature can't be blocked.\"",
            0,
        );
        let effects = super::super::parse_effect_sentences_lexed(&tokens)
            .expect("become-and-grant sentence should parse through the full effect pipeline");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("BecomeBasePtCreature"), "{debug}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(!debug.contains("GrantAbilitiesChoiceToTarget"), "{debug}");
        assert!(
            debug.contains("RuleRestriction"),
            "quoted can't-be-blocked clause must remain a typed quoted rule: {debug}"
        );
    }

    #[test]
    fn explicit_copy_subject_uses_the_copy_result_tag() {
        let tokens = tokenize_line("The copy gains haste.", 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("copy-result grant should parse")
            .expect("copy-result grant should produce effects");
        let debug = format!("{effects:#?}");

        assert!(debug.contains(COPIED_STACK_OBJECT_TAG), "{debug}");
        assert!(
            !debug.contains(&format!("TagKey(\n                    \"{IT_TAG}\"")),
            "{debug}"
        );
    }

    #[test]
    fn mixed_keyword_list_keeps_static_keyword_after_executable_keyword() {
        let ability_tokens = lex_line("trample, annihilator 2, and haste", 0)
            .expect("mixed keyword grant should lex");
        let clause_words = crate::runtime_backend::token_word_refs(&ability_tokens);
        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)
                .expect("mixed keyword grant should parse");

        assert!(!is_choice);
        assert_eq!(abilities.len(), 3, "{abilities:#?}");
        assert!(
            matches!(
                abilities[0],
                GrantedAbilityAst::KeywordAction(KeywordAction::Trample)
            ),
            "{abilities:#?}"
        );
        assert!(
            matches!(
                abilities[1],
                GrantedAbilityAst::KeywordAction(KeywordAction::Annihilator(2))
            ),
            "{abilities:#?}"
        );
        assert!(
            matches!(
                abilities[2],
                GrantedAbilityAst::KeywordAction(KeywordAction::Haste)
            ),
            "{abilities:#?}"
        );
    }

    #[test]
    fn effect_chain_keeps_keyword_after_oxford_comma_normalization() {
        let tokens = lex_line(
            "Until end of turn, it has base power and toughness 10/10 and gains trample, annihilator 2, and haste.",
            0,
        )
        .expect("leading-duration mixed grant should lex");
        let effects = parse_effect_chain(&tokens)
            .expect("leading-duration mixed grant should parse through the effect chain");

        let ast_debug = format!("{effects:#?}");
        assert!(ast_debug.contains("SetBasePowerToughness"), "{ast_debug}");
        assert!(ast_debug.contains("Trample"), "{ast_debug}");
        assert!(ast_debug.contains("Annihilator"), "{ast_debug}");
        assert!(ast_debug.contains("Haste"), "{ast_debug}");

        let compiled = compile_statement_effects(&effects)
            .expect("leading-duration mixed grant should lower to runtime effects");
        let compiled_debug = format!("{compiled:#?}");
        assert!(compiled_debug.contains("Trample"), "{compiled_debug}");
        // Annihilator is a keyword action in the AST, but lowers to its
        // trigger-and-sacrifice runtime representation rather than retaining
        // the keyword name in the compiled debug form.
        assert!(
            compiled_debug.contains("SacrificePlayerEffect"),
            "{compiled_debug}"
        );
        let compact_debug: String = compiled_debug.split_whitespace().collect();
        assert!(compact_debug.contains("count:Fixed(2"), "{compiled_debug}");
        assert!(compiled_debug.contains("Haste"), "{compiled_debug}");
    }

    #[test]
    fn gain_ability_to_source_keeps_parsed_ability_until_lowering() {
        let tokens = tokenize_line("This creature gains {T}: Draw a card.", 0);
        let effect = parse_gain_ability_to_source_sentence(&tokens)
            .expect("gain-to-source sentence should parse")
            .expect("gain-to-source sentence should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilityToSource"),
            "expected source grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "duration: Forever"),
            "source ability grants without an explicit duration should be indefinite, got {debug}"
        );
        assert!(
            string_contains(&debug, "effects_ast: Some"),
            "expected parsed ability to remain unlowered in the AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("grant-to-source effect should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            (string_contains(&compiled_debug, "ApplyContinuousEffect")
                && string_contains(&compiled_debug, "AddAbilityGeneric")
                && string_contains(&compiled_debug, "target_spec: Some(Source)")
                && string_contains(&compiled_debug, "until: Forever"))
                || (string_contains(&compiled_debug, "GrantObjectAbilityEffect")
                    && string_contains(&compiled_debug, "target: Source")),
            "expected source grant effect after lowering, got {compiled_debug}"
        );
    }

    #[test]
    fn gain_ability_to_source_respects_explicit_until_end_of_turn_duration() {
        let tokens = tokenize_line("This creature gains {T}: Draw a card until end of turn.", 0);
        let effect = parse_gain_ability_to_source_sentence(&tokens)
            .expect("gain-to-source sentence should parse")
            .expect("gain-to-source sentence should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilityToSource"),
            "expected source grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "duration: EndOfTurn"),
            "explicit source ability grant duration should be preserved, got {debug}"
        );
    }

    #[test]
    fn quoted_nested_trigger_grant_keeps_outer_until_end_of_turn_duration() {
        let tokens = tokenize_line(
            "It gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.",
            0,
        );
        let effect = parse_gain_ability_sentence(&tokens)
            .expect("quoted nested trigger grant should parse")
            .expect("quoted nested trigger grant should produce effects")
            .into_iter()
            .next()
            .expect("quoted nested trigger grant should produce one effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilities")
                && string_contains(&debug, "ParsedObjectAbility")
                && string_contains(&debug, "duration: EndOfTurn")
                && string_contains(&debug, "Draw")
                && string_contains(&debug, "Fixed(2)"),
            "expected quoted combat-damage draw trigger to be granted until end of turn, got {debug}"
        );
    }

    #[test]
    fn keyword_and_quoted_trigger_share_target_and_duration() {
        let tokens = tokenize_line(
            "Until end of turn, target creature you control with power 4 or greater gains trample and \"Whenever this creature deals combat damage to a player, draw a card.\"",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("mixed keyword and quoted trigger grant should parse")
            .expect("mixed grant should produce effects");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("Trample"), "{debug}");
        assert!(debug.contains("ThisDealsCombatDamageToPlayer"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
        assert!(debug.contains("duration: EndOfTurn"), "{debug}");
        assert!(debug.contains("power: Some"), "{debug}");
    }

    #[test]
    fn activated_line_keeps_mixed_keyword_and_quoted_trigger_together() {
        let (parsed, trace) = crate::parse_trace::capture(|| {
            CardDefinitionBuilder::new(CardId::from_raw(1), "Test Heirloom").parse_text(
                "{T}: Until end of turn, target creature you control with power 4 or greater gains trample and \"Whenever this creature deals combat damage to a player, draw a card.\"",
            )
        });
        let def = parsed.unwrap_or_else(|error| {
            panic!(
                "mixed grant inside an activated line should parse: {error:?}\n{}",
                trace.render()
            )
        });
        let debug = format!("{def:#?}");

        assert!(debug.contains("Trample"), "{debug}");
        assert!(debug.contains("ThisDealsCombatDamageToPlayer"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
        assert!(debug.contains("EndOfTurn"), "{debug}");
    }

    #[test]
    fn target_gain_activated_ability_stays_unlowered_until_compile() {
        let tokens = tokenize_line(
            "Target creature gains {T}: Draw a card until end of turn.",
            0,
        );
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("target gain clause should parse")
            .expect("target gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "ParsedObjectAbility"),
            "expected parsed granted ability in AST, got {debug}"
        );
        assert!(
            string_contains(&debug, "effects_ast: Some"),
            "expected granted ability to remain unlowered in AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("target gain clause should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            string_contains(&compiled_debug, "ApplyContinuousEffect")
                && (string_contains(&compiled_debug, "AddAbilityGeneric")
                    || string_contains(&compiled_debug, "GrantObjectAbilityForFilter")),
            "expected lowered granted ability effect, got {compiled_debug}"
        );
    }

    #[test]
    fn target_lose_activated_ability_stays_unlowered_until_compile() {
        let tokens = tokenize_line(
            "Target creature loses {T}: Draw a card until end of turn.",
            0,
        );
        let effect = parse_simple_lose_ability_clause(&tokens)
            .expect("target lose clause should parse")
            .expect("target lose clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "ParsedObjectAbility"),
            "expected parsed removed ability in AST, got {debug}"
        );
        assert!(
            string_contains(&debug, "effects_ast: Some"),
            "expected removed ability to remain unlowered in AST, got {debug}"
        );

        let compiled =
            compile_statement_effects(&[effect]).expect("target lose clause should lower");
        let compiled_debug = format!("{compiled:?}");
        assert!(
            string_contains(&compiled_debug, "RemoveAbility"),
            "expected lowered remove-ability effect, got {compiled_debug}"
        );
        assert!(
            string_contains(&compiled_debug, "ApplyContinuousEffect"),
            "expected removed ability to lower through a continuous effect, got {compiled_debug}"
        );
    }

    #[test]
    fn pump_and_lose_ability_sentence_keeps_shared_until_your_next_turn() {
        let tokens = tokenize_line(
            "Target creature gets -2/-0 and loses flying until your next turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("pump-and-lose sentence should parse")
            .expect("pump-and-lose sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "Pump") && string_contains(&debug, "RemoveAbilitiesFromTarget"),
            "expected pump plus remove-ability effects, got {debug}"
        );
        assert!(
            debug.matches("YourNextTurn").count() >= 2,
            "expected shared duration to apply to both effects, got {debug}"
        );
    }

    #[test]
    fn leading_duration_pump_and_keyword_chain_preserves_optional_target_count() {
        let tokens = tokenize_line(
            "Until end of turn, up to one target creature gets +2/+2 and gains vigilance and haste.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("optional-target pump-and-grant sentence should parse")
            .expect("optional-target pump-and-grant sentence should produce effects");

        let [
            EffectAst::Coordinated {
                effects: coordinated,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one coordinated pump-and-grant clause: {effects:#?}");
        };
        let parsed_count = coordinated.iter().find_map(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Pump {
                        target: TargetAst::WithCount(_, count),
                        ..
                    },
                ..
            }) => Some(*count),
            _ => None,
        });
        assert_eq!(parsed_count, Some(ChoiceCount::up_to(1)), "{effects:#?}");

        let compiled = compile_statement_effects(&effects)
            .expect("optional-target pump-and-grant sentence should lower");

        fn contains_optional_target(effect: &crate::effect::Effect) -> bool {
            if effect
                .target_spec()
                .is_some_and(|target| target.count() == ChoiceCount::up_to(1))
            {
                return true;
            }
            let mut found = false;
            effect.visit_child_effects(&mut |child| {
                found |= contains_optional_target(child);
            });
            found
        }
        assert!(
            compiled.iter().any(contains_optional_target),
            "the authored optional target must survive lowering: {compiled:#?}"
        );
    }

    #[test]
    fn pump_then_gain_is_preserved_as_one_coordinated_typed_clause() {
        let tokens = tokenize_line(
            "This creature gets +2/+2 and gains trample until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("pump-and-grant sentence should parse")
            .expect("pump-and-grant sentence should produce effects");

        let [
            EffectAst::Coordinated {
                effects: coordinated,
                leading_duration: false,
                result_conjunction: false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected coordinated pump-and-grant clause, got {effects:#?}");
        };
        let debug = format!("{coordinated:#?}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("Trample"), "{debug}");
    }

    #[test]
    fn leading_become_lose_then_gain_keeps_the_trailing_keyword() {
        let tokens = tokenize_line(
            "Until end of turn, target creature you control becomes a blue Dragon Illusion with base power and toughness 4/4, loses all abilities, and gains flying.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("become-lose-gain sentence should parse")
            .expect("become-lose-gain sentence should produce effects");

        let [
            EffectAst::Coordinated {
                effects: coordinated,
                leading_duration: true,
                result_conjunction: false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected leading-duration coordinated clause, got {effects:#?}");
        };
        let debug = format!("{coordinated:#?}");
        assert!(debug.contains("BecomeBasePtCreature"), "{debug}");
        assert!(debug.contains("RemoveAbilitiesFromTarget"), "{debug}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("Flying"), "{debug}");
    }

    #[test]
    fn base_pt_then_gains_keyword_in_single_clause_parses() {
        let tokens = tokenize_line(
            "This creature has base power and toughness 4/5 until end of turn and gains wither until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("base-pt then gains clause should parse")
            .expect("base-pt then gains clause should produce effects");

        let debug = format!("{effects:?}").to_ascii_lowercase();
        assert!(
            string_contains(&debug, "setbasepowertoughness")
                && string_contains(&debug, "grantabilitiestotarget")
                && string_contains(&debug, "wither")
                && debug.matches("endofturn").count() >= 2,
            "expected shared self-targeted base P/T plus wither grant until EOT, got {debug}"
        );
    }

    #[test]
    fn leading_duration_demonstrative_base_pt_then_gains_keyword_parses() {
        let tokens = tokenize_line(
            "Until end of turn, that creature has base power and toughness 4/4 and gains indestructible.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("leading-duration base-pt then gains clause should parse")
            .expect("leading-duration base-pt then gains clause should produce effects");

        let debug = format!("{effects:?}").to_ascii_lowercase();
        assert!(
            string_contains(&debug, "setbasepowertoughness")
                && string_contains(&debug, "grantabilitiestotarget")
                && string_contains(&debug, "indestructible")
                && debug.matches("endofturn").count() >= 2,
            "expected demonstrative base P/T plus keyword grant until EOT, got {debug}"
        );
    }

    #[test]
    fn gain_landwalk_until_next_upkeep_sentence_parses() {
        let tokens = tokenize_line(
            "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("gain-until-next-upkeep sentence should parse")
            .expect("gain-until-next-upkeep sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesToTarget"),
            "expected target ability grant, got {debug}"
        );
        assert!(
            string_contains(&debug, "Landwalk(Subtype { subtype: Forest, snow: false })")
                && string_contains(&debug, "YourNextUpkeep"),
            "expected forestwalk grant to keep next-upkeep duration, got {debug}"
        );
    }

    #[test]
    fn lexed_gain_landwalk_until_next_upkeep_sentence_parses() {
        let mut tokens = lex_line(
            "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
            0,
        )
        .expect("rewrite lexer should classify landwalk gain clause");
        for token in &mut tokens {
            token.lowercase_word();
        }
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("lexed gain-until-next-upkeep sentence should parse")
            .expect("lexed gain-until-next-upkeep sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesToTarget"),
            "expected target ability grant, got {debug}"
        );
        assert!(
            string_contains(&debug, "Landwalk(Subtype { subtype: Forest, snow: false })")
                && string_contains(&debug, "YourNextUpkeep"),
            "expected forestwalk grant to keep next-upkeep duration, got {debug}"
        );
    }

    #[test]
    fn gain_haste_and_except_by_haste_with_trailing_where_clause_keeps_unblockable_grant() {
        let tokens = tokenize_line(
            "Up to X target creatures you control each gain haste until end of turn and can't be blocked this turn except by creatures with haste, where X is the number of Bobbleheads you control as you activate this ability.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("agility bobblehead-style grant clause should parse")
            .expect("agility bobblehead-style grant clause should produce effects");

        let debug = format!("{effects:?}").to_ascii_lowercase();
        assert!(
            string_contains(&debug, "haste")
                && string_contains(&debug, "can't be blocked except by creatures with haste"),
            "expected haste plus except-by-haste unblockable grant, got {debug}"
        );
    }

    #[test]
    fn you_and_permanents_gain_hexproof_splits_player_and_permanent_grants() {
        let tokens = tokenize_line(
            "You and permanents you control gain hexproof until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("mixed player/permanent grant should parse")
            .expect("mixed player/permanent grant should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "Cant")
                && string_contains(&debug, "BeTargetedPlayerFrom")
                && string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "Hexproof"),
            "expected player hexproof restriction plus permanent hexproof grant, got {debug}"
        );
    }

    #[test]
    fn you_gain_shroud_lowers_to_unscoped_player_target_restriction() {
        let tokens = tokenize_line("You gain shroud until end of turn.", 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("player shroud grant should parse")
            .expect("player shroud grant should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "Cant")
                && string_contains(&debug, "BeTargetedPlayer")
                && !string_contains(&debug, "BeTargetedPlayerFrom"),
            "expected shroud to prevent all targeting of the player, got {debug}"
        );
    }

    #[test]
    fn you_and_permanents_gain_hexproof_from_keeps_player_grant_opponent_scoped() {
        let tokens = tokenize_line(
            "You and permanents you control gain hexproof from blue and from black until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("mixed player/permanent hexproof-from grant should parse")
            .expect("mixed player/permanent hexproof-from grant should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "BeTargetedPlayerFrom")
                && string_contains(&debug, "Opponent")
                && string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "HexproofFrom"),
            "expected player hexproof-from restriction to apply only to opponents' sources plus permanent hexproof-from grant, got {debug}"
        );
    }

    #[test]
    fn gain_ability_subject_ignores_also_before_gain() {
        let tokens = tokenize_line(
            "Permanents you control also gain indestructible until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("also-gain sentence should parse")
            .expect("also-gain sentence should produce effects");

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesAll")
                && string_contains(&debug, "Indestructible"),
            "expected also to be ignored in the subject filter, got {debug}"
        );
    }

    #[test]
    fn mass_ability_loss_keeps_spent_mana_condition_through_lowering() {
        let tokens = tokenize_line(
            "Creatures your opponents control lose flying until end of turn if {G} was spent to cast this spell.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("conditional mass ability loss should parse")
            .expect("conditional mass ability loss should produce effects");

        let [
            EffectAst::Conditional {
                predicate,
                if_true,
                if_false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected conditional mass ability removal, got {effects:#?}");
        };
        assert!(matches!(
            predicate,
            PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 1,
                symbol: Some(crate::mana::ManaSymbol::Green),
            }
        ));
        assert!(matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RemoveAbilitiesAll { .. },
                ..
            })]
        ));
        assert!(if_false.is_empty());

        let compiled = compile_statement_effects(&effects)
            .expect("conditional mass ability loss should lower");
        let debug = format!("{compiled:#?}");
        assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
        assert!(debug.contains("Green"), "{debug}");
    }

    #[test]
    fn bare_card_type_and_subtype_mass_loss_uses_union_filter() {
        let tokens = tokenize_line(
            "All creatures and Vehicles lose indestructible until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("cross-kind mass ability loss should parse")
            .expect("cross-kind mass ability loss should produce effects");

        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one mass ability-removal AST, got {effects:#?}");
        };
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.card_types == [CardType::Creature]),
            "{filter:#?}"
        );
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.subtypes == [crate::types::Subtype::Vehicle]),
            "{filter:#?}"
        );
    }

    #[test]
    fn dawns_truce_gift_line_compiles_promised_and_not_promised_branches() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dawn's Truce")
            .parse_text(
                "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)\nYou and permanents you control gain hexproof until end of turn. If the gift was promised, permanents you control also gain indestructible until end of turn.",
            )
            .expect("Dawn's Truce gift text should parse");

        let debug = format!("{def:#?}");
        assert!(
            string_contains(&debug, "ThisSpellPaidLabel")
                && string_contains(&debug, "kind: Gift")
                && string_contains(&debug, "EmitGiftGiven")
                && string_contains(&debug, "Hexproof")
                && string_contains(&debug, "Indestructible"),
            "expected Gift condition, gift event, hexproof, and indestructible effects, got {debug}"
        );
    }

    #[test]
    fn source_reference_simple_gain_clause_keeps_leading_duration_and_source_target() {
        let tokens = tokenize_line("Until end of turn, this creature gains flying.", 0);
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("source-referenced simple gain clause should parse")
            .expect("source-referenced simple gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesToTarget"),
            "expected a self-targeted temporary grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "source: true"),
            "expected the simple gain clause to stay targeted on the source, got {debug}"
        );
        assert!(
            string_contains(&debug, "ThisPermanentType(\"this creature\")"),
            "expected the simple gain clause to preserve the source surface, got {debug}"
        );
        assert!(
            string_contains(&debug, "EndOfTurn"),
            "expected the leading duration to survive lowering, got {debug}"
        );
        assert!(
            !string_contains(&debug, "GrantAbilitiesAll"),
            "expected no broad battlefield-wide grant effect, got {debug}"
        );
    }

    #[test]
    fn source_reference_simple_lose_clause_keeps_leading_duration_and_source_target() {
        let tokens = tokenize_line("Until end of turn, this creature loses defender.", 0);
        let effect = parse_simple_lose_ability_clause(&tokens)
            .expect("source-referenced simple lose clause should parse")
            .expect("source-referenced simple lose clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "RemoveAbilitiesFromTarget"),
            "expected a self-targeted temporary removal effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "Source("),
            "expected the simple lose clause to stay targeted on the source, got {debug}"
        );
        assert!(
            string_contains(&debug, "EndOfTurn"),
            "expected the leading duration to survive lowering, got {debug}"
        );
        assert!(
            !string_contains(&debug, "RemoveAbilitiesAll"),
            "expected no broad battlefield-wide removal effect, got {debug}"
        );
    }

    #[test]
    fn quoted_granted_trigger_keeps_all_sentences_inside_the_grant() {
        let tokens = tokenize_line(
            "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("quoted granted trigger should parse")
            .expect("quoted granted trigger should produce effects");

        assert_eq!(
            effects.len(),
            1,
            "quoted granted trigger should stay inside a single grant effect: {effects:?}"
        );

        let debug = format!("{effects:?}");
        assert!(
            string_contains(&debug, "GrantAbilitiesAll"),
            "expected a global grant effect, got {debug}"
        );
        assert!(
            string_contains(&debug, "ParsedObjectAbility"),
            "expected parsed granted ability payload, got {debug}"
        );
        assert!(
            string_contains(&debug, "LoseLife"),
            "expected lose-life text to remain inside the granted ability payload, got {debug}"
        );
    }

    #[test]
    fn quoted_granted_trigger_keeps_trailing_if_otherwise_branch() {
        let tokens = tokenize_line(
            "Sliver creatures you control have \"When this creature enters, Slivers you control get +1/+1 until end of turn if you're the monarch. Otherwise, you become the monarch.\"",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("quoted monarch trigger should parse")
            .expect("quoted monarch trigger should produce effects");

        let granted_abilities = effects
            .iter()
            .find_map(|effect| match effect {
                EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
                    SubjectVerbActionAst::GrantAbilitiesAll { abilities, .. } => Some(abilities),
                    _ => None,
                },
                _ => None,
            })
            .expect("expected global grant effect");
        let granted_trigger = granted_abilities
            .iter()
            .find_map(|ability| match ability {
                GrantedAbilityAst::ParsedObjectAbility { ability, .. } => Some(ability),
                _ => None,
            })
            .expect("expected parsed granted trigger");
        let trigger_effects = granted_trigger
            .effects_ast
            .as_ref()
            .expect("expected granted trigger effects");
        let false_branch = trigger_effects
            .iter()
            .find_map(|effect| match effect {
                EffectAst::Conditional {
                    predicate,
                    if_false,
                    ..
                } if matches!(predicate, PredicateAst::PlayerIsMonarch { .. }) => Some(if_false),
                _ => None,
            })
            .expect("expected monarch conditional inside granted trigger");
        assert!(
            false_branch.iter().any(|effect| matches!(
                effect,
                EffectAst::SubjectVerb(subject_verb)
                    if matches!(subject_verb.action, SubjectVerbActionAst::BecomeMonarch)
            )),
            "expected otherwise branch to become the monarch"
        );
    }

    #[test]
    fn hellish_rebuke_lowering_keeps_lose_life_inside_granted_trigger() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellish Rebuke")
            .parse_text(
                "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            )
            .expect("hellish rebuke grant line should parse");

        let spell_effects = def
            .spell_effect
            .as_ref()
            .expect("hellish rebuke should compile to spell effects");
        assert_eq!(
            spell_effects.len(),
            1,
            "lose life should not be hoisted to a top-level spell effect: {spell_effects:?}"
        );

        let debug = format!("{spell_effects:?}");
        assert!(
            string_contains(&debug, "AddAbilityGeneric")
                && string_contains(&debug, "TriggeredAbility")
                && string_contains(&debug, "LoseLifeEffect")
                && (string_contains(&debug, "sacrifice_source")
                    || (string_contains(&debug, "SacrificeTargetEffect")
                        && string_contains(&debug, "Source"))),
            "granted trigger should keep its inline trigger effects together, got {debug}"
        );
        assert!(
            string_contains(&debug, "this_deals_damage_to_player")
                || string_contains(&debug, "ThisDealsDamageTrigger"),
            "granted trigger should constrain damage-to-player semantics: {debug}"
        );
    }

    #[test]
    fn counter_linked_leading_duration_keeps_quoted_trigger_as_a_grant() {
        for (text, counter_name) in [
            (
                "For as long as that land has a blaze counter on it, it has \"At the beginning of your upkeep, this land deals 1 damage to you.\"",
                "blaze",
            ),
            (
                "For as long as that creature has a bounty counter on it, it has \"When this creature dies, each opponent draws a card and gains 2 life.\"",
                "bounty",
            ),
        ] {
            let tokens = tokenize_line(text, 0);
            let effects = parse_gain_ability_sentence(&tokens)
                .expect("counter-linked quoted grant should parse")
                .expect("counter-linked quoted grant should produce an effect");
            let debug = format!("{effects:#?}");
            let normalized_debug = debug.to_ascii_lowercase();

            assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
            assert!(debug.contains("ParsedObjectAbility"), "{debug}");
            assert!(
                debug.contains("ForAsLongAs") && normalized_debug.contains(counter_name),
                "{debug}"
            );
        }
    }

    #[test]
    fn mixed_keyword_and_quoted_trigger_grant_stays_targeted() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strength of Will")
            .parse_text(
                "Until end of turn, target creature you control gains indestructible and \"Whenever this creature is dealt damage, put that many +1/+1 counters on it.\"",
            )
            .expect("strength of will grant line should parse");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            string_contains(&debug, "TriggeredAbility"),
            "grant should keep the quoted triggered ability payload: {debug}"
        );

        let rendered = format!("{def:#?}").to_ascii_lowercase();
        let compact_rendered = rendered.split_whitespace().collect::<String>();
        assert!(
            (string_contains(&compact_rendered, "targetonlyeffect")
                || string_contains(&compact_rendered, "target_spec:some(target(object("))
                && string_contains(&compact_rendered, "controller:some(you")
                && string_contains(&compact_rendered, "addability")
                && string_contains(&compact_rendered, "indestructible")
                && string_contains(&compact_rendered, "addabilitygeneric")
                && string_contains(&compact_rendered, "isdealtdamage")
                && string_contains(&compact_rendered, "putcounterseffect"),
            "grant should stay targeted in the lowered structure: {rendered}"
        );
    }

    #[test]
    fn players_gain_hexproof_clause_parses_as_player_wide_targeting_restriction() {
        let tokens = tokenize_line("Players gain hexproof until end of turn.", 0);
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("players gain clause should parse")
            .expect("players gain clause should produce an effect");

        let debug = format!("{effect:?}");
        assert!(
            string_contains(&debug, "Cant")
                && string_contains(&debug, "BeTargetedPlayerFrom(Any")
                && string_contains(&debug, "EndOfTurn"),
            "expected a player-wide temporary targeting restriction, got {debug}"
        );
    }

    #[test]
    fn lose_become_and_base_pt_chain_keeps_one_unmodified_subject() {
        let tokens = tokenize_line(
            "Each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("shared-subject continuous chain should parse")
            .expect("shared-subject continuous chain should produce effects");
        let [
            EffectAst::Coordinated {
                effects: coordinated,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one coordinated continuous chain, got {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::RemoveAbilitiesAll {
                        filter: remove,
                        set_quantifier_surface: remove_quantifier,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::AddSubtypes {
                        target: TargetAst::Object(add, ..),
                        subtypes,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SetBasePowerToughness {
                        target: TargetAst::Object(set_pt, ..),
                        power: Value::Fixed(1),
                        toughness: Value::Fixed(1),
                        ..
                    },
                ..
            }),
        ] = coordinated.as_slice()
        else {
            panic!("expected remove/add-subtype/set-P/T actions, got {coordinated:#?}");
        };
        assert_eq!(
            *remove_quantifier,
            Some(ironsmith_core::SetQuantifierSurface::Each)
        );

        for filter in [remove, add, set_pt] {
            assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
            assert_eq!(
                filter.controller,
                Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
                "{filter:#?}"
            );
            assert!(filter.subtypes.is_empty(), "{filter:#?}");
            assert!(!filter.other, "{filter:#?}");
        }
        assert_eq!(subtypes, &[crate::types::Subtype::Coward]);
    }

    #[test]
    fn sentence_dispatch_preserves_loss_become_and_base_pt_coordination() {
        let tokens = tokenize_line(
            "Each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1.",
            0,
        );
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("full sentence dispatch should preserve the coordinated chain");
        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::Coordinated {
                    effects: coordinated,
                    ..
                }] if matches!(
                    coordinated.as_slice(),
                    [
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::RemoveAbilitiesAll { .. },
                            ..
                        }),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::AddSubtypes { .. },
                            ..
                        }),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::SetBasePowerToughness { .. },
                            ..
                        }),
                    ]
                )
            ),
            "full sentence route must not contaminate the subject filter: {effects:#?}"
        );
    }

    #[test]
    fn target_controller_qualifier_does_not_hide_an_explicit_object_target() {
        let tokens = tokenize_line(
            "Target creature an opponent controls loses all abilities and has base power and toughness 1/1 until end of turn.",
            0,
        );
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("targeted continuous chain should parse")
            .expect("targeted continuous chain should produce effects");
        let [
            EffectAst::Coordinated {
                effects: coordinated,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one coordinated targeted chain, got {effects:#?}");
        };
        assert!(
            matches!(
                coordinated.as_slice(),
                [
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::RemoveAbilitiesFromTarget { .. },
                        ..
                    }),
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::SetBasePowerToughness { .. },
                        ..
                    })
                ]
            ),
            "explicit object target must remain targeted: {coordinated:#?}"
        );
    }

    #[test]
    fn plural_pronoun_grant_is_typed_without_pluralizing_singular_it() {
        let surface = |text: &str| {
            let tokens = tokenize_line(text, 0);
            let effects = parse_gain_ability_sentence(&tokens)
                .expect("pronoun grant should parse")
                .expect("pronoun grant should produce an effect");
            let [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::GrantAbilitiesToTarget {
                            set_quantifier_surface,
                            ..
                        },
                    ..
                }),
            ] = effects.as_slice()
            else {
                panic!("expected one typed target grant, got {effects:#?}");
            };
            *set_quantifier_surface
        };
        let simple_surface = |text: &str| {
            let tokens = tokenize_line(text, 0);
            let effect = parse_simple_gain_ability_clause(&tokens)
                .expect("simple pronoun grant should parse")
                .expect("simple pronoun grant should produce an effect");
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        set_quantifier_surface,
                        ..
                    },
                ..
            }) = effect
            else {
                panic!("expected one simple typed target grant, got {effect:#?}");
            };
            set_quantifier_surface
        };

        assert_eq!(
            surface("They gain haste until end of turn."),
            Some(ironsmith_core::SetQuantifierSurface::They)
        );
        assert_eq!(surface("It gains haste until end of turn."), None);
        assert_eq!(
            simple_surface("They gain haste until end of turn."),
            Some(ironsmith_core::SetQuantifierSurface::They)
        );
        assert_eq!(simple_surface("It gains haste until end of turn."), None);
    }

    #[test]
    fn this_creature_keyword_grant_targets_only_the_ability_source() {
        let tokens = tokenize_line("This creature gains indestructible until end of turn.", 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("source keyword grant should parse")
            .expect("source keyword grant should produce an effect");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Object(source_filter, None, None),
                        abilities,
                        duration: Until::EndOfTurn,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("source grant must not widen to an unscoped object filter: {effects:#?}");
        };
        assert!(source_filter.source, "{source_filter:#?}");
        assert_eq!(
            source_filter.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this creature".to_string()
            ))
        );
        assert_eq!(
            abilities,
            &[GrantedAbilityAst::KeywordAction(
                KeywordAction::Indestructible
            )]
        );
    }
}
