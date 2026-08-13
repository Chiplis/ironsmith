use super::*;
use crate::cards::builders::{
    ChooseOneModeAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst,
    SubjectVerbSubjectAst,
};
use crate::grammar::effects::for_each_shapes::parse_fixed_pt_alternative_shape;
use crate::grammar::effects::misc_action_shapes::{
    self, BecomePlayerSurface, EndActionShape, FlipTargetSurface, SkipActionKind,
    SwitchTargetSurface, UntapActionShape, parse_conjoined_untap_all_tokens,
};
use crate::grammar::effects::parse_inline_creature_type_choice_tokens;
use crate::grammar::leaf::parse_leaf_mana_cost_prefix_tokens;
use crate::grammar::shared_util::value_semantics::parse_equal_to_aggregate_filter_value;
use crate::lexer::token_slice_at_is;

const ENERGY_WORD: &str = "e";
const TICKET_WORD: &str = "tk";
const ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const ENERGY_COUNTER_PAY_IGNORED_WORDS: &[&str] = &["and", "or", "energy", "counter", "counters"];
const ENERGY_TEXT_WORD: &str = "energy";

fn misc_word_is_any(word: &str, choices: &[&str]) -> bool {
    misc_action_shapes::parse_misc_word_choice(word, choices)
}

fn mana_group_token_matches_symbol(token: &OwnedLexToken, expected: &str) -> bool {
    if token.kind != TokenKind::ManaGroup {
        return false;
    }
    let Some(symbol) = token.mana_group_inner() else {
        return false;
    };
    symbol.eq_ignore_ascii_case(expected)
}

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn energy_symbol_token(token: &OwnedLexToken) -> bool {
    token_is_word(token, ENERGY_WORD) || mana_group_token_matches_symbol(token, ENERGY_WORD)
}

fn exact_pay_component(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<EffectAst> {
    let tokens = trim_commas(tokens);
    if tokens.is_empty() {
        return None;
    }

    if let Some((amount, used)) = parse_value(&tokens)
        && token_slice_at_is(&tokens, used, "life")
        && trim_commas(&tokens[used + 1..]).is_empty()
    {
        return Some(EffectAst::subject_verb_pay_life(player, amount));
    }

    if let Some((amount, used)) = parse_value(&tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.as_word().is_some_and(|word| word == ENERGY_TEXT_WORD))
        && trim_commas(&tokens[used + 1..]).is_empty()
    {
        return Some(EffectAst::subject_verb_pay_energy(player, amount));
    }

    let parsed = parse_leaf_mana_cost_prefix_tokens(&tokens)?;
    (parsed.consumed == tokens.len()).then(|| EffectAst::subject_verb_pay_mana(player, parsed.cost))
}

fn parse_compound_pay(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<EffectAst> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_some_and(|word| word == "and") {
            parts.push(trim_commas(&tokens[start..idx]));
            start = idx + 1;
        }
    }
    if parts.is_empty() {
        return None;
    }
    parts.push(trim_commas(&tokens[start..]));
    if parts.iter().any(|part| part.is_empty()) {
        return None;
    }

    let mut effects = Vec::new();
    for part in parts {
        effects.push(exact_pay_component(&part, player)?);
    }
    (effects.len() > 1).then_some(EffectAst::Sequence { effects })
}

fn ticket_symbol_token(token: &OwnedLexToken) -> bool {
    token_is_word(token, TICKET_WORD) || mana_group_token_matches_symbol(token, TICKET_WORD)
}

fn subject_verb_player_effect(
    role: SubjectVerbRoleAst,
    player: PlayerAst,
    action: SubjectVerbActionAst,
) -> EffectAst {
    EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { role, player },
        action,
    })
}

pub(crate) fn parse_become(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
        )));
    };

    if misc_action_shapes::parse_become_player_surface(tokens) == BecomePlayerSurface::Monarch {
        return Ok(EffectAst::subject_verb_become_monarch(player));
    }

    let amount = parse_value(tokens)
        .map(|(value, _)| value)
        .or_else(|| parse_half_starting_life_total_value(tokens, player))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing life total amount (clause: '{}')",
                crate::token_word_refs(tokens).join(" ")
            ))
        })?;
    Ok(EffectAst::subject_verb_set_life_total(player, amount))
}

pub(crate) fn parse_switch(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use crate::effect::Until;

    // Split off trailing duration, if present.
    let (duration, remainder) =
        if let Some((duration, remainder)) = parse_restriction_duration(tokens)? {
            (duration, remainder)
        } else {
            (Until::EndOfTurn, trim_commas(tokens).to_vec())
        };

    let Some(shape) = misc_action_shapes::parse_switch_power_toughness_tokens(&remainder) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported switch clause (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
        )));
    };
    let target = match shape.target {
        SwitchTargetSurface::Source(tokens) => TargetAst::Source(span_from_tokens(tokens)),
        SwitchTargetSurface::Tagged(tokens) => {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens))
        }
        SwitchTargetSurface::Explicit(tokens) => parse_target_phrase(tokens)?,
    };

    Ok(EffectAst::subject_verb_switch_power_toughness(
        target, duration,
    ))
}

pub(crate) fn parse_skip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let subject_player = match subject {
        Some(SubjectAst::Player(player)) => Some(player),
        _ => None,
    };
    let shape =
        misc_action_shapes::parse_skip_action_tokens(tokens, subject_player).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported skip clause (clause: '{}')",
                crate::token_word_refs(tokens).join(" ")
            ))
        })?;
    Ok(match shape.action {
        SkipActionKind::NextCombatPhaseThisTurn => {
            EffectAst::subject_verb_skip_next_combat_phase_this_turn(shape.player)
        }
        SkipActionKind::CombatPhases => EffectAst::subject_verb_skip_combat_phases(shape.player),
        SkipActionKind::DrawStep => EffectAst::subject_verb_skip_draw_step(shape.player),
        SkipActionKind::Turn => EffectAst::subject_verb_skip_turn(shape.player),
    })
}

pub(crate) fn parse_end(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
        SubjectAst::TriggeringSourceController => {
            return Err(CardTextError::ParseError(
                "unsupported triggering-source controller subject for end action".to_string(),
            ));
        }
    };

    match misc_action_shapes::parse_end_action_tokens(tokens) {
        Some(EndActionShape::Turn) => Ok(EffectAst::subject_verb_end_turn(player)),
        Some(EndActionShape::CombatPhase) => Ok(EffectAst::subject_verb_end_combat_phase(player)),
        Some(EndActionShape::EndStepLoseGame) => {
            Ok(EffectAst::subject_verb_lose_game(PlayerAst::You))
        }
        None => Err(CardTextError::ParseError(format!(
            "unsupported end clause (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
        ))),
    }
}

pub(crate) fn parse_flip(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
        SubjectAst::TriggeringSourceController => {
            return Err(CardTextError::ParseError(
                "unsupported triggering-source controller subject for flip action".to_string(),
            ));
        }
    };
    let shape = misc_action_shapes::parse_flip_action_tokens(tokens);
    let effect = match shape.target {
        FlipTargetSurface::Source(None) => EffectAst::subject_verb_flip(TargetAst::Source(None)),
        FlipTargetSurface::Source(Some(tokens)) => {
            EffectAst::subject_verb_flip(TargetAst::Source(span_from_tokens(tokens)))
        }
        FlipTargetSurface::Coin => EffectAst::subject_verb_flip_coin(player),
        FlipTargetSurface::Explicit(tokens) => {
            EffectAst::subject_verb_flip(parse_target_phrase(tokens)?)
        }
    };
    Ok(if shape.delayed_until_next_end_step {
        EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![effect],
        }
    } else {
        effect
    })
}

pub(crate) fn parse_roll(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject.unwrap_or(SubjectAst::This) {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
        SubjectAst::TriggeringSourceController => {
            return Err(CardTextError::ParseError(
                "unsupported triggering-source controller subject for roll action".to_string(),
            ));
        }
    };
    let Some(shape) = misc_action_shapes::parse_roll_die_tokens(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported roll clause (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
        )));
    };
    Ok(EffectAst::subject_verb_roll_die_with_die_text(
        player,
        shape.sides,
        shape.die_text,
    ))
}

pub(crate) fn parse_regenerate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::token_word_refs(tokens);
    if words
        .first()
        .copied()
        .is_some_and(|word| misc_word_is_any(word, ALL_OR_EACH_WORDS))
    {
        if tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "regenerate clause missing filter after each/all".to_string(),
            ));
        }
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::subject_verb_regenerate_all(filter));
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::subject_verb_regenerate(target))
}

pub(crate) fn parse_mill(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let subject_player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let shape =
        misc_action_shapes::parse_mill_action_tokens(tokens, subject_player)?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing or unsupported mill count (clause: '{}')",
                crate::token_word_refs(tokens).join(" ")
            ))
        })?;

    Ok(subject_verb_player_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        subject_player,
        SubjectVerbActionAst::Mill { count: shape.count },
    ))
}

fn parse_named_player_counter_count(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Value, CardTextError> {
    if let Some(for_each_idx) = tokens.windows(2).position(|window| {
        window[0].as_word() == Some("for") && window[1].as_word() == Some("each")
    }) && let Some(count) = parse_get_for_each_count_value(&tokens[for_each_idx..])?
    {
        return Ok(count);
    }
    if matches!(
        clause_words.first().copied(),
        Some("a" | "an" | "another" | "one")
    ) {
        return Ok(Value::Fixed(1));
    }
    Ok(parse_value(tokens)
        .map(|(value, _)| value)
        .unwrap_or(Value::Fixed(1)))
}

pub(crate) fn parse_get(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let tokens = if tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "get" | "gets"))
    {
        &tokens[1..]
    } else {
        tokens
    };

    fn parse_pump_for_each_tail(
        tail_tokens: &[OwnedLexToken],
        subject: Option<SubjectAst>,
        power_per: i32,
        toughness_per: i32,
        clause_words: &[&str],
    ) -> Result<Option<EffectAst>, CardTextError> {
        if grammar::match_word_prefix(tail_tokens, &["until", "end", "of", "turn", "for", "each"])
            .is_none()
        {
            return Ok(None);
        }

        let count = parse_get_for_each_count_value(&tail_tokens[4..])?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported get-for-each filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        Ok(Some(EffectAst::subject_verb_pump_for_each(
            power_per,
            toughness_per,
            target,
            count,
            Until::EndOfTurn,
        )))
    }

    let clause_words = crate::token_word_refs(tokens);
    if let Some(alternative) = parse_fixed_pt_alternative_shape(tokens) {
        let branch_tokens = |modifier: &OwnedLexToken| {
            let mut tokens = Vec::with_capacity(1 + alternative.trailing_tokens.len());
            tokens.push(modifier.clone());
            tokens.extend_from_slice(alternative.trailing_tokens);
            tokens
        };
        let first = parse_get(&branch_tokens(&alternative.first_modifier), subject)?;
        let second = parse_get(&branch_tokens(&alternative.second_modifier), subject)?;
        return Ok(EffectAst::ChooseOneOf {
            modes: vec![
                ChooseOneModeAst {
                    description: String::new(),
                    effects: vec![first],
                },
                ChooseOneModeAst {
                    description: String::new(),
                    effects: vec![second],
                },
            ],
        });
    }

    if grammar::contains_word(tokens, "poison")
        && (grammar::contains_word(tokens, "counter") || grammar::contains_word(tokens, "counters"))
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = parse_named_player_counter_count(tokens, &clause_words)?;
        return Ok(EffectAst::subject_verb_poison_counters(player, count));
    }

    if grammar::contains_word(tokens, "experience")
        && (grammar::contains_word(tokens, "counter") || grammar::contains_word(tokens, "counters"))
    {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = parse_named_player_counter_count(tokens, &clause_words)?;
        return Ok(EffectAst::subject_verb_experience_counters(player, count));
    }

    let energy_count = tokens
        .iter()
        .filter(|token| energy_symbol_token(token))
        .count();
    if energy_count > 0 {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        let count = parse_add_mana_equal_amount_value(tokens)
            .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
            .or(parse_dynamic_cost_modifier_value(tokens)?)
            .or(parse_equal_to_number_of_filter_value(tokens))
            .or_else(|| {
                let equal_idx = tokens.windows(2).position(|window| {
                    window[0].as_word() == Some("equal") && window[1].as_word() == Some("to")
                })?;
                let tail = &tokens[equal_idx + 2..];
                let (value, used) = parse_value(tail)?;
                (used == tail.len()).then_some(value)
            })
            .or_else(|| parse_value(tokens).map(|(value, _)| value))
            .unwrap_or(Value::Fixed(energy_count as i32));
        return Ok(EffectAst::subject_verb_energy_counters(player, count));
    }

    let ticket_count = tokens
        .iter()
        .filter(|token| ticket_symbol_token(token))
        .count();
    if ticket_count > 0 {
        let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
        return Ok(EffectAst::subject_verb_ticket_counters(
            player,
            Value::Fixed(ticket_count as i32),
        ));
    }

    if let Some(effect) = parse_emblem_action(tokens, subject) {
        return Ok(effect);
    }
    if let Some(effect) = parse_unquoted_emblem_action(tokens, subject) {
        return Ok(effect);
    }

    let modifier_start =
        if let Some((prefix, _)) = grammar::match_any_word_prefix(tokens, ADDITIONAL_PREFIXES) {
            prefix.len()
        } else {
            0usize
        };
    if modifier_start > 0
        && let Some(mod_token) = tokens.get(modifier_start).map(OwnedLexToken::parser_text)
        && let Ok((power_per, toughness_per)) = parse_pt_modifier(mod_token)
    {
        let tail_tokens = tokens.get(modifier_start + 1..).unwrap_or_default();
        if let Some(effect) = parse_pump_for_each_tail(
            tail_tokens,
            subject,
            power_per,
            toughness_per,
            &clause_words,
        )? {
            return Ok(effect);
        }
    }

    if let Some(mod_token) = tokens.first().map(OwnedLexToken::parser_text)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness)
            && let Some(effect) = parse_pump_for_each_tail(
                tokens.get(1..).unwrap_or_default(),
                subject,
                *power_per,
                *toughness_per,
                &clause_words,
            )?
        {
            return Ok(effect);
        }
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::subject_verb_pump(
            power, toughness, target, duration, condition,
        ));
    }

    if let Some(collapsed_tokens) = collapse_leading_signed_pt_modifier_tokens(tokens)
        && let Some(mod_token) = collapsed_tokens.first().map(OwnedLexToken::parser_text)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) = (&power, &toughness)
            && let Some(effect) = parse_pump_for_each_tail(
                collapsed_tokens.get(1..).unwrap_or_default(),
                subject,
                *power_per,
                *toughness_per,
                &clause_words,
            )?
        {
            return Ok(effect);
        }
        let (power, toughness, duration, condition) =
            parse_get_modifier_values_with_tail(&collapsed_tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::subject_verb_pump(
            power, toughness, target, duration, condition,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn constrain_untap_filter_to_battlefield(filter: &mut ObjectFilter) {
    filter.zone.get_or_insert(Zone::Battlefield);
    for branch in &mut filter.any_of {
        constrain_untap_filter_to_battlefield(branch);
    }
}

fn constrain_untap_target_to_battlefield(target: &mut TargetAst) {
    match target {
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            constrain_untap_filter_to_battlefield(filter);
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            constrain_untap_target_to_battlefield(inner);
        }
        _ => {}
    }
}

pub(crate) fn parse_untap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    if let Some(choice) = parse_inline_creature_type_choice_tokens(tokens) {
        let mut cleaned = Vec::with_capacity(
            choice
                .before_tokens
                .len()
                .saturating_add(choice.after_tokens.len()),
        );
        cleaned.extend_from_slice(&trim_commas(choice.before_tokens));
        cleaned.extend_from_slice(&trim_commas(choice.after_tokens));
        let UntapActionShape::All { filter_tokens } =
            misc_action_shapes::parse_untap_action_tokens(&cleaned)
        else {
            return Err(CardTextError::ParseError(
                "creature-type choice untap requires an all/each object set".to_string(),
            ));
        };
        let mut filter = parse_object_filter(filter_tokens, false)?;
        constrain_untap_filter_to_battlefield(&mut filter);
        filter.chosen_creature_type = true;
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
                EffectAst::subject_verb_untap_all(filter),
            ],
        });
    }
    if let Some(filter_tokens) = misc_action_shapes::parse_chosen_object_set_filter_tokens(tokens) {
        let mut filter = parse_object_filter(filter_tokens, false)?;
        constrain_untap_filter_to_battlefield(&mut filter);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::cards::builders::CHOSEN_OBJECTS_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(EffectAst::subject_verb_untap_all(filter));
    }
    if let Some(shape) = parse_conjoined_untap_all_tokens(tokens) {
        let mut left = parse_object_filter(shape.left_filter_tokens, false)?;
        let mut right = parse_object_filter(shape.right_filter_tokens, false)?;
        constrain_untap_filter_to_battlefield(&mut left);
        constrain_untap_filter_to_battlefield(&mut right);
        return Ok(EffectAst::Coordinated {
            effects: vec![
                EffectAst::subject_verb_untap_all(left),
                EffectAst::subject_verb_untap_all(right),
            ],
            leading_duration: false,
            result_conjunction: false,
        });
    }
    match misc_action_shapes::parse_untap_action_tokens(tokens) {
        UntapActionShape::All { filter_tokens } => {
            let mut filter = parse_object_filter(filter_tokens, false)?;
            constrain_untap_filter_to_battlefield(&mut filter);
            Ok(EffectAst::subject_verb_untap_all(filter))
        }
        UntapActionShape::Tagged { filter_tokens } => {
            let mut filter = filter_tokens
                .map(|tokens| parse_object_filter(tokens, false))
                .transpose()?
                .unwrap_or_default();
            constrain_untap_filter_to_battlefield(&mut filter);
            filter.set_plural_pronoun_reference_surface(filter_tokens.is_none());
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: IT_TAG.into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            Ok(EffectAst::subject_verb_untap_all(filter))
        }
        UntapActionShape::Explicit { target_tokens } => {
            let mut target = parse_target_phrase(target_tokens)?;
            constrain_untap_target_to_battlefield(&mut target);
            Ok(EffectAst::subject_verb_untap(target))
        }
    }
}

pub(crate) fn parse_scry(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(subject_verb_player_effect(
        SubjectVerbRoleAst::Chooser,
        player,
        SubjectVerbActionAst::Scry { count },
    ))
}

pub(crate) fn parse_surveil(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            crate::token_word_refs(tokens).join(" ")
        ))
    })?;

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    Ok(subject_verb_player_effect(
        SubjectVerbRoleAst::Chooser,
        player,
        SubjectVerbActionAst::Surveil { count },
    ))
}

pub(crate) fn parse_pay(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let energy_symbol_count = tokens
        .iter()
        .filter(|token| energy_symbol_token(token))
        .count();

    let clause_words = crate::token_word_refs(tokens);
    if grammar::match_any_word_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::subject_verb_pay_any_energy(player, 0));
    }
    if grammar::match_any_word_prefix(tokens, ANY_AMOUNT_OF_PREFIXES).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(EffectAst::subject_verb_pay_any_life(player, 0));
    }
    if grammar::match_any_word_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && (grammar::contains_word(tokens, "e") || energy_symbol_count > 0)
    {
        return Ok(EffectAst::subject_verb_pay_any_energy(player, 1));
    }
    if grammar::match_any_word_prefix(tokens, &[&["one", "or", "more"]]).is_some()
        && grammar::contains_word(tokens, "life")
    {
        return Ok(EffectAst::subject_verb_pay_any_life(player, 1));
    }
    if let Some(compound) = parse_compound_pay(tokens, player) {
        return Ok(compound);
    }
    if let Some(repeated) = misc_action_shapes::parse_repeated_tagged_mana_payment_tokens(tokens) {
        // In a clause such as "that player may choose ... and pay {2} for
        // each creature chosen this way", the omitted subject of the payment
        // is the iterated player, not the resolving ability's controller.
        let payer = if player == PlayerAst::Implicit {
            PlayerAst::That
        } else {
            player
        };
        return Ok(EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![EffectAst::subject_verb_pay_mana(
                payer,
                ManaCost::from_pips(repeated.pip_groups),
            )],
        });
    }

    if let Some((for_each_idx, (), _)) =
        grammar::find_prefix(tokens, || grammar::phrase(&["for", "each"]))
        && let Some(parsed_cost) = parse_leaf_mana_cost_prefix_tokens(&tokens[..for_each_idx])
        && parsed_cost.consumed == for_each_idx
        && let [pip] = parsed_cost.cost.pips()
        && let [crate::mana::ManaSymbol::Generic(multiplier)] = pip.as_slice()
    {
        let count_words = crate::token_word_refs(&tokens[for_each_idx..]);
        if let Some((count, used)) =
            crate::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
        {
            let count = match *multiplier {
                1 => count,
                multiplier => Value::Scaled(Box::new(count), i32::from(multiplier)),
            }
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
            return Ok(subject_verb_player_effect(
                SubjectVerbRoleAst::AffectedPlayer,
                player,
                SubjectVerbActionAst::PayMana {
                    cost: ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X]),
                    x_value: Some(count),
                    x_maximum: None,
                },
            ));
        }
    }

    if clause_words.len() >= 4
        && grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && let Ok(symbols) = parse_mana_symbol_group(clause_words[0])
    {
        return Ok(EffectAst::subject_verb_pay_mana(
            player,
            ManaCost::from_pips(vec![symbols]),
        ));
    }

    if let Some((amount, used)) = parse_value(tokens)
        && token_slice_at_is(tokens, used, "life")
    {
        return Ok(EffectAst::subject_verb_pay_life(player, amount));
    }
    if let Some((amount, used)) = parse_value(tokens)
        && tokens
            .get(used)
            .is_some_and(|token| token.as_word().is_some_and(|word| word == ENERGY_TEXT_WORD))
    {
        return Ok(EffectAst::subject_verb_pay_energy(player, amount));
    }
    if energy_symbol_count > 0 {
        if let Some(equal_idx) = tokens.windows(2).position(|window| {
            window[0].as_word() == Some("equal") && window[1].as_word() == Some("to")
        }) {
            let amount_tokens = &tokens[equal_idx + 2..];
            if let Some((amount, used)) = parse_value(amount_tokens)
                && used == amount_tokens.len()
            {
                return Ok(EffectAst::subject_verb_pay_energy(player, amount));
            }
            if let Some(amount) = parse_dynamic_cost_modifier_value(amount_tokens)? {
                return Ok(EffectAst::subject_verb_pay_energy(player, amount));
            }
        }
        let mut energy_count = 0u32;
        for token in tokens {
            if energy_symbol_token(token) {
                energy_count += 1;
                continue;
            }
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word) || misc_word_is_any(word, ENERGY_COUNTER_PAY_IGNORED_WORDS) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                crate::token_word_refs(tokens).join(" ")
            )));
        }
        if energy_count > 0 {
            return Ok(EffectAst::subject_verb_pay_energy(
                player,
                Value::Fixed(energy_count as i32),
            ));
        }
    }

    let pips = {
        use winnow::prelude::*;
        let mut stream = LexStream::new(tokens);
        grammar::collect_mana_pip_groups
            .parse_next(&mut stream)
            .map_err(|_| {
                CardTextError::ParseError(format!(
                    "missing payment cost (clause: '{}')",
                    crate::token_word_refs(tokens).join(" ")
                ))
            })?
    };

    Ok(EffectAst::subject_verb_pay_mana(
        player,
        ManaCost::from_pips(pips),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn energy_for_each_keeps_for_each_value_surface() {
        let tokens = lex_line("get {E} for each creature attacking you.", 0)
            .expect("energy clause should lex");
        let effect = parse_get(&tokens, None).expect("energy clause should parse");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("EnergyCounters"), "{debug}");
        assert!(debug.contains("ForEach"), "{debug}");
        assert!(debug.contains("attacking_player_only: true"), "{debug}");
    }
}
