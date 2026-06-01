use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, GiftTimingAst, LineInfo, ParseAnnotations,
    PlayerAst, StaticAbilityAst, TriggerSpec,
};
use crate::runtime_backend::activation_and_restrictions::last_created_token_info;
use crate::runtime_backend::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut,
};
use crate::runtime_backend::lexer::{word_slice_contains_any_word, word_slice_contains_phrase};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

use super::super::effect_pipeline::{
    NormalizedLineChunk, NormalizedParsedAbility, NormalizedPreparedAbility,
};
use super::*;

struct LineChunkLoweringInput<'a> {
    builder: CardDefinitionBuilder,
    state: &'a mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &'a LineInfo,
    allow_unsupported: bool,
    annotations: &'a mut ParseAnnotations,
}

fn conditional_self_replacement_followup(
    effect: &crate::effect::Effect,
) -> Option<crate::effects::ConditionalEffect> {
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return Some(conditional.clone());
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| conditional_self_replacement_followup(&tagged.effect))
}

fn materialized_self_replacement_followup(
    program: &crate::resolution::ResolutionProgram,
) -> Option<crate::resolution::SelfReplacementBranch> {
    let [segment] = program.segments.as_slice() else {
        return None;
    };
    if !segment.default_effects.is_empty() || segment.self_replacements.len() != 1 {
        return None;
    }
    Some(segment.self_replacements[0].clone())
}

fn retarget_replacement_effects(
    effects: Vec<crate::effect::Effect>,
    previous_target: &ChooseSpec,
) -> Vec<crate::effect::Effect> {
    effects
        .into_iter()
        .map(|effect| {
            if let Some(replacement_damage) =
                effect.downcast_ref::<crate::effects::DealDamageEffect>()
                && replacement_damage.target == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
            {
                crate::effect::Effect::deal_damage(
                    replacement_damage.amount.clone(),
                    previous_target.clone(),
                )
            } else {
                super::rewrite_replacement_effect_target(&effect, previous_target).unwrap_or(effect)
            }
        })
        .collect()
}

fn unwrap_matching_conditional_replacement_effects(
    effects: Vec<crate::effect::Effect>,
    condition: &crate::effect::Condition,
) -> Vec<crate::effect::Effect> {
    let [effect] = effects.as_slice() else {
        return effects;
    };
    let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() else {
        return effects;
    };
    if conditional.condition != *condition || !conditional.if_false.is_empty() {
        return effects;
    }
    conditional.if_true.clone()
}

fn rewrite_prior_token_placeholder_effect(
    effect: &mut EffectAst,
    token_info: &(String, PlayerAst),
) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods { name, player, .. } =
            &mut subject_verb.action
        && matches!(name.as_str(), "of those" | "those")
    {
        *name = token_info.0.clone();
        *player = token_info.1;
        subject_verb.subject.player = token_info.1;
    }
}

fn rewrite_prior_token_placeholders(effects: &mut [EffectAst], token_info: &(String, PlayerAst)) {
    for effect in effects {
        rewrite_prior_token_placeholder_effect(effect, token_info);
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                rewrite_prior_token_placeholder_effect(nested_effect, token_info);
            }
        });
    }
}

fn rewrite_prior_token_placeholder_effect_from_template(
    effect: &mut EffectAst,
    template: &(SubjectVerbActionAst, PlayerAst),
) {
    let (template_action, template_player) = template;
    let SubjectVerbActionAst::CreateTokenWithMods {
        name: template_name,
        dynamic_power_toughness: template_dynamic_power_toughness,
        player: template_action_player,
        attached_to: template_attached_to,
        tapped: template_tapped,
        attacking: template_attacking,
        exile_at_end_of_combat: template_exile_at_end_of_combat,
        sacrifice_at_end_of_combat: template_sacrifice_at_end_of_combat,
        sacrifice_at_next_end_step: template_sacrifice_at_next_end_step,
        exile_at_next_end_step: template_exile_at_next_end_step,
        granted_abilities: template_granted_abilities,
        ..
    } = template_action
    else {
        return;
    };
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods {
            name,
            dynamic_power_toughness,
            player,
            attached_to,
            tapped,
            attacking,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            granted_abilities,
            ..
        } = &mut subject_verb.action
        && matches!(name.as_str(), "of those" | "those")
    {
        *name = template_name.clone();
        *dynamic_power_toughness = template_dynamic_power_toughness.clone();
        *player = *template_action_player;
        *attached_to = template_attached_to.clone();
        *tapped = *template_tapped;
        *attacking = *template_attacking;
        *exile_at_end_of_combat = *template_exile_at_end_of_combat;
        *sacrifice_at_end_of_combat = *template_sacrifice_at_end_of_combat;
        *sacrifice_at_next_end_step = *template_sacrifice_at_next_end_step;
        *exile_at_next_end_step = *template_exile_at_next_end_step;
        *granted_abilities = template_granted_abilities.clone();
        subject_verb.subject.player = *template_player;
    }
}

fn rewrite_prior_token_placeholders_from_template(
    effects: &mut [EffectAst],
    template: &(SubjectVerbActionAst, PlayerAst),
) {
    for effect in effects {
        rewrite_prior_token_placeholder_effect_from_template(effect, template);
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                rewrite_prior_token_placeholder_effect_from_template(nested_effect, template);
            }
        });
    }
}

fn effect_references_prior_token_placeholder(effect: &EffectAst) -> bool {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::CreateTokenWithMods { name, .. } = &subject_verb.action
        && matches!(name.as_str(), "of those" | "those")
    {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_references_prior_token_placeholder);
        }
    });
    found
}

fn created_token_template_from_effect(
    effect: &EffectAst,
) -> Option<(SubjectVerbActionAst, PlayerAst)> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::CreateTokenWithMods { .. } => {
                Some((subject_verb.action.clone(), subject_verb.subject.player))
            }
            _ => {
                let mut found = None;
                for_each_nested_effects(effect, true, |nested| {
                    if found.is_none() {
                        found = token_template_before_prior_token_placeholder(nested);
                    }
                });
                found
            }
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                if found.is_none() {
                    found = token_template_before_prior_token_placeholder(nested);
                }
            });
            found
        }
    }
}

fn token_template_before_prior_token_placeholder(
    effects: &[EffectAst],
) -> Option<(SubjectVerbActionAst, PlayerAst)> {
    let mut latest_token_template = None;
    for effect in effects {
        if effect_references_prior_token_placeholder(effect) {
            return latest_token_template;
        }
        if let Some(token_template) = created_token_template_from_effect(effect) {
            latest_token_template = Some(token_template);
        }
    }
    None
}

fn compile_trailing_instead_if_condition(
    normalized_line: &str,
    line_index: usize,
    prepared: &super::super::effect_pipeline::PreparedEffectsForLowering,
) -> Result<Option<crate::effect::Condition>, CardTextError> {
    let tokens = lex_line(normalized_line, line_index).map_err(|err| {
        CardTextError::ParseError(format!(
            "failed to lex instead-if follow-up '{}': {err:?}",
            normalized_line
        ))
    })?;
    let Some(instead_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.is_word("instead") && tokens.get(idx + 1).is_some_and(|next| next.is_word("if")))
            .then_some(idx)
    }) else {
        return Ok(None);
    };
    let Some(predicate) =
        crate::runtime_backend::grammar::structure::parse_trailing_instead_if_predicate_lexed(
            &tokens[instead_idx..],
        )
    else {
        return Ok(None);
    };
    compile_condition_from_predicate_ast_with_env(
        &predicate,
        &prepared.initial_env,
        prepared.imports.last_object_tag.as_ref(),
    )
    .map(Some)
}

fn with_chosen_creature_type_filter(effect: crate::effect::Effect) -> crate::effect::Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return crate::effect::Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            with_chosen_creature_type_filter((*tagged.effect).clone()),
        ));
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        let mut continuous = continuous.clone();
        if let crate::continuous::EffectTarget::Filter(filter) = &mut continuous.target {
            filter.chosen_creature_type = true;
        }
        return crate::effect::Effect::new(continuous);
    }
    effect
}

fn creature_type_choice_program(
    normalized_line: &str,
    compiled: &crate::resolution::ResolutionProgram,
) -> Option<crate::resolution::ResolutionProgram> {
    let words = normalized_line
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '+'))
        .collect::<Vec<_>>();
    let has_choice_phrase =
        word_slice_contains_phrase(&words, &["creature", "type", "of", "your", "choice"]);
    let has_get = word_slice_contains_any_word(&words, &["get", "gets"]);
    if !has_choice_phrase || !has_get {
        return None;
    }
    let effects = compiled.to_vec();
    if effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
            .is_some()
    }) {
        return None;
    }

    let mut patched = vec![crate::effect::Effect::choose_creature_type(
        PlayerFilter::You,
        vec![],
    )];
    patched.extend(effects.into_iter().map(with_chosen_creature_type_filter));
    Some(crate::resolution::ResolutionProgram::from_effects(patched))
}

fn optional_zone_rewrite_effect(
    effect: crate::effect::Effect,
    target: ChooseSpec,
    from_zone: Zone,
    to_zone: Zone,
    replacement_zone: Zone,
    choice_description: &str,
) -> crate::effect::Effect {
    let mut replacement = crate::effects::RegisterZoneReplacementEffect::new(
        target,
        Some(from_zone),
        Some(to_zone),
        replacement_zone,
        crate::effects::ReplacementApplyMode::OneShot,
    );
    replacement.optional = true;
    replacement.choice_description = Some(choice_description.to_string());
    crate::effect::Effect::new(crate::effects::LocalRewriteEffect::new(
        effect,
        vec![replacement],
    ))
}

fn optional_returned_creature_to_battlefield_branch(
    return_effect: crate::effect::Effect,
    condition: crate::effect::Condition,
) -> crate::resolution::SelfReplacementBranch {
    let target = ChooseSpec::Object(
        crate::target::ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .with_mana_value(crate::target::Comparison::LessThanOrEqual(4)),
    );
    let replacement_effect = optional_zone_rewrite_effect(
        return_effect,
        target,
        Zone::Graveyard,
        Zone::Hand,
        Zone::Battlefield,
        "Put one returned card with mana value 4 or less onto the battlefield",
    );
    crate::resolution::SelfReplacementBranch::new(condition, vec![replacement_effect])
}

fn search_to_hand_replacement_target(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
    if let Some(search) = effect.as_search()
        && search.destination == Zone::Hand
    {
        return Some(ChooseSpec::Object(search.filter.clone()));
    }
    if let Some(search_slots) = effect.as_search_slots()
        && search_slots.destination == Zone::Hand
        && search_slots.slots.len() == 1
    {
        return Some(ChooseSpec::Object(search_slots.slots[0].filter.clone()));
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.is_search
        && choose.zone == Some(Zone::Library)
    {
        return Some(ChooseSpec::Object(choose.filter.clone()));
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .find_map(search_to_hand_replacement_target);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return search_to_hand_replacement_target(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return search_to_hand_replacement_target(&with_id.effect);
    }
    None
}

fn optional_search_to_battlefield_rewrite(
    effect: &crate::effect::Effect,
) -> Option<crate::effect::Effect> {
    let target = search_to_hand_replacement_target(effect)?;
    Some(optional_zone_rewrite_effect(
        effect.clone(),
        target,
        Zone::Library,
        Zone::Hand,
        Zone::Battlefield,
        "Put that card onto the battlefield instead of into your hand",
    ))
}

fn attach_morbid_search_to_battlefield_self_replacement(
    builder: &mut CardDefinitionBuilder,
    normalized_line: &str,
) -> bool {
    if !text_mentions_morbid_search_to_battlefield_replacement(normalized_line) {
        return false;
    }
    let Some(existing) = builder.spell_effect.as_mut() else {
        return false;
    };
    let Some(segment) = existing.last_segment_mut() else {
        return false;
    };
    let replacement_effects = segment
        .default_effects
        .iter()
        .map(optional_search_to_battlefield_rewrite)
        .collect::<Option<Vec<_>>>();
    let Some(replacement_effects) = replacement_effects else {
        return false;
    };
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            crate::effect::Condition::CreatureDiedThisTurn,
            replacement_effects,
        ));
    true
}

fn back_for_seconds_style_replacement_program(
    compiled: &[crate::effect::Effect],
    normalized_line: &str,
) -> Option<crate::resolution::ResolutionProgram> {
    if !text_mentions_bargained_return_to_battlefield_replacement(normalized_line) {
        return None;
    }
    let (default_effects, return_effect, condition) = match compiled {
        [return_effect, followup] => {
            let conditional = conditional_self_replacement_followup(followup)?;
            if !conditional.if_false.is_empty() {
                return None;
            }
            (
                vec![return_effect.clone()],
                return_effect.clone(),
                conditional.condition,
            )
        }
        [target_only, return_effect, followup]
            if target_only
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some() =>
        {
            let conditional = conditional_self_replacement_followup(followup)?;
            if !conditional.if_false.is_empty() {
                return None;
            }
            (
                vec![target_only.clone(), return_effect.clone()],
                return_effect.clone(),
                conditional.condition,
            )
        }
        _ => return None,
    };
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    let segment = program.last_segment_mut()?;
    segment
        .self_replacements
        .push(optional_returned_creature_to_battlefield_branch(
            return_effect,
            condition,
        ));
    Some(program)
}

fn attach_back_for_seconds_style_replacement(
    builder: &mut CardDefinitionBuilder,
    compiled: &[crate::effect::Effect],
    normalized_line: &str,
) -> bool {
    if !text_mentions_bargained_return_to_battlefield_replacement(normalized_line) {
        return false;
    }
    let [followup] = compiled else {
        return false;
    };
    let Some(conditional) = conditional_self_replacement_followup(followup) else {
        return false;
    };
    if !conditional.if_false.is_empty() {
        return false;
    }
    let Some(existing) = builder.spell_effect.as_mut() else {
        return false;
    };
    let Some(segment) = existing.last_segment_mut() else {
        return false;
    };
    let [return_effect] = segment.default_effects.as_slice() else {
        return false;
    };
    segment
        .self_replacements
        .push(optional_returned_creature_to_battlefield_branch(
            return_effect.clone(),
            conditional.condition,
        ));
    true
}

fn kicked_count_override_self_replacement_program(
    compiled: &[crate::effect::Effect],
    normalized_line: &str,
) -> Option<crate::resolution::ResolutionProgram> {
    if !text_mentions_kicked_count_override_replacement(normalized_line) {
        return None;
    }
    let [look_effect, conditional_effect] = compiled else {
        return None;
    };
    if look_effect
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .is_none()
    {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != crate::effect::Condition::ThisSpellWasKicked
        || conditional.if_true.is_empty()
        || conditional.if_false.is_empty()
    {
        return None;
    }
    let mut default_effects = vec![look_effect.clone()];
    default_effects.extend(conditional.if_false.clone());
    let mut replacement_effects = vec![look_effect.clone()];
    replacement_effects.extend(conditional.if_true.clone());
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    let segment = program.last_segment_mut()?;
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            replacement_effects,
        ));
    Some(program)
}

fn kicked_multi_zone_search_to_battlefield_program(
    compiled: &[crate::effect::Effect],
    normalized_line: &str,
) -> Option<crate::resolution::ResolutionProgram> {
    if !text_mentions_kicked_multi_zone_search_to_battlefield_replacement(normalized_line) {
        return None;
    }
    let [choose, reveal, move_to_hand, shuffle, conditional] = compiled else {
        return None;
    };
    let conditional = conditional.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != crate::effect::Condition::ThisSpellWasKicked {
        return None;
    }

    let searched_tag = crate::TagKey::from("searched_multi_zone");
    let default_effects = vec![
        choose.clone(),
        reveal.clone(),
        move_to_hand.clone(),
        shuffle.clone(),
    ];
    let replacement_effects = vec![
        choose.clone(),
        reveal.clone(),
        crate::effect::Effect::move_to_zone(
            ChooseSpec::tagged(searched_tag),
            Zone::Battlefield,
            false,
        ),
        shuffle.clone(),
    ];
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    let segment = program.last_segment_mut()?;
    segment
        .self_replacements
        .push(crate::resolution::SelfReplacementBranch::new(
            conditional.condition.clone(),
            replacement_effects,
        ));
    Some(program)
}

fn clash_win_optional_top_replacement_program(
    compiled: &[crate::effect::Effect],
    normalized_line: &str,
) -> Option<crate::resolution::ResolutionProgram> {
    if !text_mentions_clash_win_top_replacement(normalized_line) {
        return None;
    }
    let [clash_effect, return_with_id_effect, followup] = compiled else {
        return None;
    };
    if clash_effect
        .downcast_ref::<crate::effects::ClashEffect>()
        .is_none()
    {
        return None;
    }
    let return_with_id = return_with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let followup = followup.downcast_ref::<crate::effects::IfEffect>()?;
    if followup.condition != return_with_id.id
        || !matches!(
            followup.predicate,
            crate::effect::EffectPredicate::Happened
                | crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
        )
    {
        return None;
    }
    let target = ChooseSpec::Tagged(crate::tag::TagKey::from("returned_0"));
    let return_effect = (*return_with_id.effect).clone();
    let replacement_return = optional_zone_rewrite_effect(
        return_effect.clone(),
        target,
        Zone::Battlefield,
        Zone::Hand,
        Zone::Library,
        "Put that creature on top of its owner's library instead of into its owner's hand",
    );
    let clash_id = return_with_id.id;
    Some(crate::resolution::ResolutionProgram::from_effects(vec![
        crate::effect::Effect::with_id(clash_id.0, clash_effect.clone()),
        crate::effect::Effect::new(crate::effects::IfEffect::new(
            clash_id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0)),
            vec![replacement_return],
            vec![return_effect],
        )),
    ]))
}

pub(super) fn rewrite_apply_line_ast(
    builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match parsed {
        parsed @ NormalizedLineChunk::Abilities(_) => {
            lower_abilities_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::StaticAbility(_) => {
            lower_static_ability_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::StaticAbilities(_) => {
            lower_static_abilities_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Ability(_) => {
            lower_parsed_ability_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Statement { .. } => {
            lower_statement_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AdditionalCost { .. } => {
            lower_additional_cost_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::OptionalCost(_) => {
            lower_optional_cost_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::GiftKeyword { .. } => {
            lower_gift_keyword_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::OptionalCostWithCastTrigger { .. } => {
            lower_optional_cost_with_cast_trigger_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AdditionalCostChoice { .. } => {
            lower_additional_cost_choice_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::AlternativeCastingMethod(_) => {
            lower_alternative_casting_method_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
        parsed @ NormalizedLineChunk::Triggered { .. } => {
            lower_triggered_chunk(LineChunkLoweringInput {
                builder,
                state,
                parsed,
                info,
                allow_unsupported,
                annotations,
            })
        }
    }
}

fn lower_abilities_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        ..
    } = input;
    let NormalizedLineChunk::Abilities(actions) = parsed else {
        unreachable!("abilities lowerer received mismatched chunk");
    };

    for action in actions {
        builder = builder.apply_keyword_action(action);
    }
    Ok(builder)
}

fn compile_static_ability_with_zones(
    ability: crate::static_abilities::StaticAbility,
    info: &LineInfo,
) -> Ability {
    let ability = rewrite_self_spell_cost_modifier(ability, info.raw_line.as_str());
    let mut compiled = Ability::static_ability(ability);
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_spell_only_functional_zones(static_ability)
    {
        compiled = compiled.in_zones(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_all_zone_functional_zones(static_ability)
    {
        compiled = compiled.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let AbilityKind::Static(static_ability) = &compiled.kind
        && super::uses_referenced_ability_functional_zones(
            static_ability,
            info.normalized.normalized.as_str(),
        )
    {
        compiled = compiled.in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if let Some(zones) =
        super::infer_static_ability_functional_zones(info.normalized.normalized.as_str())
    {
        compiled = compiled.in_zones(zones);
    }
    compiled
}

fn rewrite_self_spell_cost_modifier(
    ability: crate::static_abilities::StaticAbility,
    raw_line: &str,
) -> crate::static_abilities::StaticAbility {
    if !text_starts_with_this_spell_cost(raw_line) {
        return ability;
    }

    match &ability.payload {
        ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => {
            let mut amount = reduction.amount.clone();
            if let Some(cap) = extract_cost_reduction_cap_from_text(raw_line) {
                amount = crate::effect::Value::Min(
                    Box::new(amount),
                    Box::new(crate::effect::Value::Fixed(cap)),
                );
            }
            crate::static_abilities::StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount,
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )
        }
        ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => {
            crate::static_abilities::StaticAbility::new(
                crate::static_abilities::ThisSpellCostReductionManaCost::new(
                    reduction.cost.clone(),
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )
        }
        _ => ability,
    }
}

fn extract_cost_reduction_cap_from_text(raw_line: &str) -> Option<i32> {
    let lower = raw_line.to_ascii_lowercase();
    let cap_start = lower.find("by more than {")?;
    let digits_start = cap_start + "by more than {".len();
    let digits_end = lower[digits_start..].find('}')? + digits_start;
    lower[digits_start..digits_end].parse::<i32>().ok()
}

fn lower_static_ability_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::StaticAbility(ability) = parsed else {
        unreachable!("static-ability lowerer received mismatched chunk");
    };

    if let StaticAbilityAst::AttachmentRestriction { filter, display } = ability {
        builder.aura_attach_filter = Some(filter);
        let _ = display;
        return Ok(builder);
    }

    let ability = match super::rewrite_lower_static_ability_ast(ability) {
        Ok(ability) => ability,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    Ok(builder.with_ability(compile_static_ability_with_zones(ability, info)))
}

fn lower_static_abilities_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::StaticAbilities(abilities) = parsed else {
        unreachable!("static-abilities lowerer received mismatched chunk");
    };

    let mut lowered_abilities = Vec::new();
    let mut regular_abilities = Vec::new();
    for ability in abilities {
        match ability {
            StaticAbilityAst::AttachmentRestriction { filter, display } => {
                builder.aura_attach_filter = Some(filter);
                let _ = display;
            }
            other => regular_abilities.push(other),
        }
    }

    let abilities = match super::rewrite_lower_static_abilities_ast(regular_abilities) {
        Ok(abilities) => abilities,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    lowered_abilities.extend(abilities);
    for ability in lowered_abilities {
        builder = builder.with_ability(compile_static_ability_with_zones(ability, info));
    }
    Ok(builder)
}

fn lower_parsed_ability_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        annotations,
        ..
    } = input;
    let NormalizedLineChunk::Ability(parsed_ability) = parsed else {
        unreachable!("ability lowerer received mismatched chunk");
    };

    let parsed_ability = super::rewrite_lower_prepared_ability(parsed_ability)?;
    if let Some(effects_ast) = parsed_ability.effects_ast.as_ref().map(Vec::as_slice) {
        super::collect_tag_spans_from_effects_with_context(
            effects_ast,
            annotations,
            &info.normalized,
        );
    }
    let ability = parsed_ability.into_runtime();
    builder = builder.with_ability(ability);
    Ok(builder)
}

fn lower_statement_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::Statement {
        effects_ast,
        mut prepared,
    } = parsed
    else {
        unreachable!("statement lowerer received mismatched chunk");
    };

    if effects_ast.is_empty() {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "empty effect statement".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to empty effect statement: '{}'",
            info.raw_line
        )));
    }
    if let Some(enchant_filter) = effects_ast.iter().find_map(|effect| {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return None;
        };
        match &subject_verb.action {
            SubjectVerbActionAst::Enchant { filter } => Some(filter.clone()),
            _ => None,
        }
    }) {
        builder.aura_attach_filter = Some(enchant_filter);
    }
    if let Some(token_template) = token_template_before_prior_token_placeholder(&prepared.effects) {
        rewrite_prior_token_placeholders_from_template(&mut prepared.effects, &token_template);
        prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.effects,
            prepared.imports.clone(),
        )?;
    } else if let Some(token_info) = state.latest_created_token.clone() {
        rewrite_prior_token_placeholders(&mut prepared.effects, &token_info);
        prepared = super::rewrite_prepare_effects_for_lowering(
            &prepared.effects,
            prepared.imports.clone(),
        )?;
    }

    let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
        Ok(lowered) => lowered,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    super::rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "spell text effects",
    )?;
    if let Some(token_info) =
        last_created_token_info(&effects_ast).or_else(|| last_created_token_info(&prepared.effects))
    {
        state.latest_created_token = Some(token_info);
    }
    let compiled = lowered.effects;
    state.latest_spell_exports = lowered.exports;

    let normalized_line = info.normalized.normalized.as_str().to_ascii_lowercase();
    let instead_semantics = super::classify_instead_followup_text(&normalized_line);
    let trailing_instead_if_condition = if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) {
        compile_trailing_instead_if_condition(&normalized_line, info.line_index, &prepared)?
    } else {
        None
    };
    if let Some(program) = back_for_seconds_style_replacement_program(&compiled, &normalized_line) {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if attach_back_for_seconds_style_replacement(&mut builder, &compiled, &normalized_line) {
        return Ok(builder);
    }
    if let Some(program) =
        kicked_count_override_self_replacement_program(&compiled, &normalized_line)
    {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if let Some(program) =
        kicked_multi_zone_search_to_battlefield_program(&compiled, &normalized_line)
    {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if let Some(program) = clash_win_optional_top_replacement_program(&compiled, &normalized_line) {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if attach_morbid_search_to_battlefield_self_replacement(&mut builder, &normalized_line) {
        return Ok(builder);
    }
    if let Some(program) = creature_type_choice_program(&normalized_line, &compiled) {
        builder.spell_effect = Some(program);
        return Ok(builder);
    }
    if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && builder.spell_effect.is_none()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[1])
        && replacement.if_false.is_empty()
    {
        let previous = compiled[0].clone();
        let mut replacement = replacement;
        if let Some(previous_target) = super::extract_previous_replacement_target(&previous) {
            replacement.if_true =
                retarget_replacement_effects(replacement.if_true, &previous_target);
        }
        let mut spell_effect = crate::resolution::ResolutionProgram::from_effects(vec![previous]);
        let Some(segment) = spell_effect.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for inline self-replacement"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
        builder.spell_effect = Some(spell_effect);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[1])
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement;
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
            .or_else(|| super::extract_previous_replacement_target(&compiled[0]))
        {
            replacement.if_true =
                retarget_replacement_effects(replacement.if_true, &previous_target);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for repeated self-replacement"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && builder.spell_effect.is_none()
        && let Some(condition) = trailing_instead_if_condition
    {
        let previous = compiled[0].clone();
        let mut replacement_effects = vec![compiled[1].clone()];
        if let Some(previous_target) = super::extract_previous_replacement_target(&previous) {
            replacement_effects =
                retarget_replacement_effects(replacement_effects, &previous_target);
        }
        replacement_effects =
            unwrap_matching_conditional_replacement_effects(replacement_effects, &condition);
        let mut spell_effect = crate::resolution::ResolutionProgram::from_effects(vec![previous]);
        let Some(segment) = spell_effect.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for inline plain instead-if follow-up"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                condition,
                replacement_effects,
            ));
        builder.spell_effect = Some(spell_effect);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 2
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(condition) = trailing_instead_if_condition
    {
        let mut replacement_effects = vec![compiled[1].clone()];
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
            .or_else(|| super::extract_previous_replacement_target(&compiled[0]))
        {
            replacement_effects =
                retarget_replacement_effects(replacement_effects, &previous_target);
        }
        replacement_effects =
            unwrap_matching_conditional_replacement_effects(replacement_effects, &condition);
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for plain instead-if follow-up"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                condition,
                replacement_effects,
            ));
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(condition) = trailing_instead_if_condition
    {
        let mut replacement_effects = vec![compiled[0].clone()];
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement_effects =
                retarget_replacement_effects(replacement_effects, &previous_target);
        }
        replacement_effects =
            unwrap_matching_conditional_replacement_effects(replacement_effects, &condition);
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for single-effect instead-if follow-up"
                    .to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                condition,
                replacement_effects,
            ));
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && let Some(mut replacement) = materialized_self_replacement_followup(&compiled)
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
    {
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement.replacement_effects =
                retarget_replacement_effects(replacement.replacement_effects, &previous_target);
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for materialized self-replacement"
                    .to_string(),
            ));
        };
        segment.self_replacements.push(replacement);
        return Ok(builder);
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && builder.spell_effect.is_none()
        && (text_starts_with_if(normalized_line.as_str())
            || conditional_self_replacement_followup(&compiled[0])
                .is_some_and(|replacement| replacement.if_false.is_empty()))
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    } else if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && builder.spell_effect.is_none()
        && materialized_self_replacement_followup(&compiled).is_some()
    {
        return Err(CardTextError::UnsupportedLine(
            "unsupported self-replacement follow-up without a prior spell segment".to_string(),
        ));
    }
    if matches!(
        instead_semantics,
        crate::cards::builders::InsteadSemantics::SelfReplacement
    ) && compiled.len() == 1
        && let Some(ref mut existing) = builder.spell_effect
        && !existing.is_empty()
        && let Some(replacement) = conditional_self_replacement_followup(&compiled[0])
        && replacement.if_false.is_empty()
    {
        let mut replacement = replacement;
        if let Some(previous_target) = existing
            .last()
            .and_then(super::extract_previous_replacement_target)
        {
            replacement.if_true = replacement
                .if_true
                .into_iter()
                .map(|effect| {
                    if let Some(replacement_damage) =
                        effect.downcast_ref::<crate::effects::DealDamageEffect>()
                        && replacement_damage.target
                            == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                    {
                        crate::effect::Effect::deal_damage(
                            replacement_damage.amount.clone(),
                            previous_target.clone(),
                        )
                    } else {
                        super::rewrite_replacement_effect_target(&effect, &previous_target)
                            .unwrap_or(effect)
                    }
                })
                .collect();
        }
        let Some(segment) = existing.last_segment_mut() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous spell resolution segment for self-replacement".to_string(),
            ));
        };
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                replacement.condition,
                replacement.if_true,
            ));
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(compiled);
    } else {
        builder.spell_effect = Some(compiled);
    }
    Ok(builder)
}

fn lower_additional_cost_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::AdditionalCost {
        effects_ast,
        prepared,
    } = parsed
    else {
        unreachable!("additional-cost lowerer received mismatched chunk");
    };

    if effects_ast.is_empty() {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "empty additional cost statement".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to empty additional-cost statement: '{}'",
            info.raw_line
        )));
    }
    let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
        Ok(lowered) => lowered,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    let compiled = super::runtime_effects_to_costs(lowered.effects.to_vec())?;
    state.latest_additional_cost_exports = lowered.exports;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.extend(compiled);
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    Ok(builder)
}

fn lower_optional_cost_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder, parsed, ..
    } = input;
    let NormalizedLineChunk::OptionalCost(cost) = parsed else {
        unreachable!("optional-cost lowerer received mismatched chunk");
    };
    Ok(builder.optional_cost(cost))
}

fn lower_gift_keyword_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::GiftKeyword {
        cost,
        prepared,
        followup_text,
        timing,
    } = parsed
    else {
        unreachable!("gift-keyword lowerer received mismatched chunk");
    };

    builder = builder.optional_cost(cost);
    match timing {
        GiftTimingAst::SpellResolution => {
            let lowered = match super::rewrite_lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(super::push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut gift_effects = lowered.effects.to_vec();
            gift_effects.push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            let gift_effect = crate::effect::Effect::conditional(
                crate::ConditionExpr::ThisSpellPaidLabel("Gift".to_string()),
                gift_effects,
                Vec::new(),
            );
            if let Some(ref mut existing) = builder.spell_effect {
                existing.push(gift_effect);
            } else {
                builder.spell_effect =
                    Some(crate::resolution::ResolutionProgram::from_effects(vec![
                        gift_effect,
                    ]));
            }
        }
        GiftTimingAst::PermanentEtb => {
            let parsed = super::rewrite_parsed_triggered_ability(
                TriggerSpec::ThisEntersBattlefield,
                prepared.effects.clone(),
                vec![Zone::Battlefield],
                Some(format!(
                    "When this permanent enters, if the gift was promised, {followup_text}"
                )),
                Some(crate::ConditionExpr::ThisSpellPaidLabel("Gift".to_string())),
                None,
                prepared.imports.clone(),
            );
            let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered {
                    trigger: TriggerSpec::ThisEntersBattlefield,
                    prepared: super::super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                        prepared,
                        intervening_if: None,
                    },
                }),
            }) {
                Ok(parsed) => parsed,
                Err(err) if allow_unsupported => {
                    return Ok(super::push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut parsed = parsed;
            if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                triggered
                    .effects
                    .push(crate::Effect::emit_gift_given(PlayerFilter::ChosenPlayer));
            }
            builder = builder.with_ability(parsed.into_runtime());
        }
    }
    Ok(builder)
}

fn lower_optional_cost_with_cast_trigger_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::OptionalCostWithCastTrigger {
        cost,
        prepared,
        followup_text,
    } = parsed
    else {
        unreachable!("optional-cost-cast-trigger lowerer received mismatched chunk");
    };

    let cost_label = cost.label.clone();
    builder = builder.optional_cost(cost);
    let parsed = super::rewrite_parsed_triggered_ability(
        TriggerSpec::YouCastThisSpell,
        prepared.effects.clone(),
        vec![Zone::Stack],
        Some(followup_text),
        Some(crate::ConditionExpr::ThisSpellPaidLabel(cost_label)),
        None,
        prepared.imports.clone(),
    );
    let parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered {
            trigger: TriggerSpec::YouCastThisSpell,
            prepared: super::super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                prepared,
                intervening_if: None,
            },
        }),
    }) {
        Ok(parsed) => parsed,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    Ok(builder.with_ability(parsed.into_runtime()))
}

fn lower_additional_cost_choice_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::AdditionalCostChoice { options } = parsed else {
        unreachable!("additional-cost-choice lowerer received mismatched chunk");
    };

    if options.len() < 2 {
        if allow_unsupported {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                "additional cost choice requires at least two options".to_string(),
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "line parsed to invalid additional-cost choice (line: '{}')",
            info.raw_line
        )));
    }
    for option in &options {
        if option.effects_ast.is_empty() {
            if allow_unsupported {
                return Ok(super::push_unsupported_marker(
                    builder,
                    info.raw_line.as_str(),
                    "additional cost choice option produced no effects".to_string(),
                ));
            }
            return Err(CardTextError::ParseError(format!(
                "line parsed to empty additional-cost option (line: '{}')",
                info.raw_line
            )));
        }
    }
    let (modes, exports) =
        match super::rewrite_lower_prepared_additional_cost_choice_modes_with_exports(&options) {
            Ok(outputs) => outputs,
            Err(err) if allow_unsupported => {
                return Ok(super::push_unsupported_marker(
                    builder,
                    info.raw_line.as_str(),
                    format!("{err:?}"),
                ));
            }
            Err(err) => return Err(err),
        };
    state.latest_additional_cost_exports = exports;
    let mut costs = builder.additional_cost.costs().to_vec();
    costs.push(
        crate::costs::payment_effect_to_cost(crate::effect::Effect::choose_one(modes))
            .map_err(CardTextError::ParseError)?,
    );
    builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
    Ok(builder)
}

fn lower_alternative_casting_method_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        mut builder,
        parsed,
        ..
    } = input;
    let NormalizedLineChunk::AlternativeCastingMethod(mut method) = parsed else {
        unreachable!("alternative-casting-method lowerer received mismatched chunk");
    };
    if let crate::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost {
        additional_cost,
        ..
    } = &method
    {
        let printed_cost = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut pips = printed_cost.pips().to_vec();
        pips.extend(additional_cost.pips().iter().cloned());
        let total_mana_cost = crate::mana::ManaCost::from_pips(pips);
        method = crate::alternative_cast::AlternativeCastingMethod::flash_with_additional_cost(
            additional_cost.clone(),
            crate::cost::TotalCost::mana(total_mana_cost),
        );
    }
    if let crate::alternative_cast::AlternativeCastingMethod::Retrace { total_cost } = &method {
        let printed_cost = builder
            .card_builder
            .mana_cost_ref()
            .cloned()
            .unwrap_or_default();
        let mut costs = vec![crate::costs::Cost::mana(printed_cost)];
        costs.extend(total_cost.costs().iter().cloned());
        method = crate::alternative_cast::AlternativeCastingMethod::Retrace {
            total_cost: crate::cost::TotalCost::from_costs(costs),
        };
    }
    builder.alternative_casts.push(method);
    Ok(builder)
}

fn lower_triggered_chunk(
    input: LineChunkLoweringInput<'_>,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let LineChunkLoweringInput {
        builder,
        state,
        parsed,
        info,
        allow_unsupported,
        ..
    } = input;
    let NormalizedLineChunk::Triggered {
        trigger,
        prepared,
        max_triggers_per_turn,
    } = parsed
    else {
        unreachable!("triggered lowerer received mismatched chunk");
    };

    let contains_haunted_creature_dies = matches!(
        &trigger,
        TriggerSpec::Either(_, right) if matches!(**right, TriggerSpec::HauntedCreatureDies)
    ) || matches!(&trigger, TriggerSpec::HauntedCreatureDies);
    let functional_zones = super::infer_triggered_ability_functional_zones(
        &trigger,
        info.normalized.normalized.as_str(),
    );
    let mut intervening_if = crate::runtime_backend::trigger_frequency_condition(
        Some(info.raw_line.as_str()),
        max_triggers_per_turn,
    );
    let trigger_surface_source = format!("{} {}", info.raw_line, info.normalized.original);
    if text_mentions_becomes_tapped_during_your_turn(&trigger_surface_source) {
        let condition = crate::ConditionExpr::YourTurn;
        intervening_if = Some(match intervening_if {
            Some(existing) => crate::ConditionExpr::And(Box::new(condition), Box::new(existing)),
            None => condition,
        });
    }

    let parsed = super::rewrite_parsed_triggered_ability(
        trigger.clone(),
        prepared.prepared.effects.clone(),
        functional_zones,
        Some(info.raw_line.clone()),
        intervening_if,
        None,
        prepared.prepared.imports.clone(),
    );
    let mut parsed = match super::rewrite_lower_prepared_ability(NormalizedParsedAbility {
        parsed,
        prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
    }) {
        Ok(parsed) => parsed,
        Err(err) if allow_unsupported => {
            return Ok(super::push_unsupported_marker(
                builder,
                info.raw_line.as_str(),
                format!("{err:?}"),
            ));
        }
        Err(err) => return Err(err),
    };
    if let Some(surface_count) = do_this_frequency_surface_from_text(&trigger_surface_source)
        && let AbilityKind::Triggered(triggered) = parsed.kind_mut()
        && triggered.intervening_if == Some(crate::ConditionExpr::MaxTimesEachTurn(surface_count))
    {
        triggered.intervening_if =
            Some(crate::ConditionExpr::DoThisMaxTimesEachTurn(surface_count));
    }
    if contains_haunted_creature_dies && let AbilityKind::Triggered(triggered) = parsed.kind() {
        state.haunt_linkage = Some((triggered.effects.to_vec(), triggered.choices.clone()));
    }
    Ok(builder.with_ability(parsed.into_runtime()))
}
