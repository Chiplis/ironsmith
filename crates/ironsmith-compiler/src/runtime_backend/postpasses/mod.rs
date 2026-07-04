use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::CardDefinition;
use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::continuous::{EffectTarget, Modification};
use crate::cost::OptionalCostKind;
use crate::effect::{
    Condition, Effect, EffectId, EffectPredicate, Until, Value, ValueComparisonOperator,
};
use crate::filter::{ObjectFilter, ObjectRef, TaggedOpbjectRelation};
use crate::resolution::{ResolutionProgram, ResolutionSegment};
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{Trigger, TriggerKind};
use crate::types::CardType;
use crate::zone::Zone;
use ironsmith_core::{EffectMetric, EffectMetricSource};

use super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::lexer::{
    LexedClause, lex_line, parser_token_word_refs, token_word_refs, word_slice_contains_any_phrase,
    word_slice_contains_phrase,
};

const BACKUP_PLACEHOLDER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("backup"),
    LexPattern::amount("amount", LexCaptureKind::WordCount(1)),
]);

fn line_starts_with_keyword(line: &str, keyword: &str) -> bool {
    lex_line(line.trim_start(), 0).ok().is_some_and(|tokens| {
        parser_token_word_refs(&tokens)
            .first()
            .is_some_and(|word| *word == keyword)
    })
}

fn overload_rewritten_text(text: &str) -> Option<String> {
    let mut rewritten_lines = Vec::new();
    let mut saw_overload = false;

    for line in text.lines() {
        if line_starts_with_keyword(line, "overload") {
            saw_overload = true;
            continue;
        }
        rewritten_lines.push(crate::cards::builders::replace_whole_word_case_insensitive(
            line, "target", "each",
        ));
    }

    saw_overload.then(|| rewritten_lines.join("\n"))
}

fn finalize_overload_definitions(
    mut definition: CardDefinition,
    original_builder: &CardDefinitionBuilder,
    original_text: &str,
) -> Result<CardDefinition, CardTextError> {
    let Some(rewritten_text) = overload_rewritten_text(original_text) else {
        return Ok(definition);
    };

    if !definition
        .alternative_casts
        .iter()
        .any(|method| matches!(method, AlternativeCastingMethod::Overload { .. }))
    {
        return Ok(definition);
    }

    let overload_builder = original_builder.clone();
    let (overloaded_definition, _) =
        super::parse_text_with_annotations(overload_builder, rewritten_text, false)?;
    let overloaded_effects = overloaded_definition.spell_effect.unwrap_or_default();

    for method in &mut definition.alternative_casts {
        if let AlternativeCastingMethod::Overload { effects, .. } = method {
            *effects = overloaded_effects.to_vec();
        }
    }

    Ok(definition)
}

fn parse_backup_placeholder_amount(ability: &Ability) -> Option<u32> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };

    let text = static_ability.display();
    let tokens = lex_line(text.trim(), 0).ok()?;
    let clause = LexedClause::new(&tokens);
    let matched = BACKUP_PLACEHOLDER_PATTERN.match_prefix(clause)?;
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    amount_clause.word_refs().first()?.parse::<u32>().ok()
}

fn backup_granted_abilities_from_slice(abilities: &[Ability]) -> Vec<Ability> {
    abilities
        .iter()
        .filter(|ability| parse_backup_placeholder_amount(ability).is_none())
        .cloned()
        .collect()
}

fn is_cipher_placeholder(ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };

    static_ability
        .display()
        .trim()
        .eq_ignore_ascii_case("Cipher")
}

pub(crate) fn finalize_backup_abilities(mut definition: CardDefinition) -> CardDefinition {
    if !definition
        .abilities
        .iter()
        .any(|ability| parse_backup_placeholder_amount(ability).is_some())
    {
        return definition;
    }

    let original_abilities = definition.abilities.clone();
    definition.abilities = original_abilities
        .iter()
        .enumerate()
        .map(|(idx, ability)| {
            let Some(amount) = parse_backup_placeholder_amount(ability) else {
                return ability.clone();
            };

            let granted_abilities =
                backup_granted_abilities_from_slice(&original_abilities[idx + 1..]);
            Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::backup(amount, granted_abilities)],
            )
        })
        .collect();
    definition
}

pub(crate) fn finalize_cipher_effects(mut definition: CardDefinition) -> CardDefinition {
    if !definition.abilities.iter().any(is_cipher_placeholder) {
        return definition;
    }

    definition
        .abilities
        .retain(|ability| !is_cipher_placeholder(ability));
    definition
        .spell_effect
        .get_or_insert_with(ResolutionProgram::default)
        .push(Effect::cipher());
    definition
}

fn finalize_squad_abilities(mut definition: CardDefinition) -> CardDefinition {
    if !definition
        .optional_costs
        .iter()
        .any(|cost| matches!(cost.kind, OptionalCostKind::Squad))
    {
        return definition;
    }

    let squad_trigger = Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![Effect::new(crate::effects::CreateTokenCopyEffect::new(
            ChooseSpec::Source,
            Value::TimesPaidLabel("Squad".into()),
            PlayerFilter::You,
        ))],
    );
    definition.abilities.push(squad_trigger);
    definition
}

fn finalize_offspring_abilities(mut definition: CardDefinition) -> CardDefinition {
    if !definition
        .optional_costs
        .iter()
        .any(|cost| matches!(cost.kind, OptionalCostKind::Offspring))
    {
        return definition;
    }

    let offspring_trigger = Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_enters_battlefield(),
            effects: ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::CreateTokenCopyEffect::new(
                    ChooseSpec::Source,
                    Value::WasPaidLabel("Offspring".into()),
                    PlayerFilter::You,
                )
                .set_base_power_toughness(1, 1),
            )]),
            choices: vec![],
            intervening_if: Some(Condition::ThisSpellPaidLabel("Offspring".into())),
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    definition.abilities.push(offspring_trigger);
    definition
}

const NEXT_UPKEEP_PHRASE: &[&str] = &["next", "upkeep"];
const NEXT_TURNS_UPKEEP_PHRASE: &[&str] = &["next", "turns", "upkeep"];
const NEXT_UPKEEP_PHRASES: &[&[&str]] = &[NEXT_UPKEEP_PHRASE, NEXT_TURNS_UPKEEP_PHRASE];
const THAT_TURNS_END_STEP_PHRASE: &[&str] = &["that", "turns", "end", "step"];
const THAT_PLAYERS_NEXT_UPKEEP_PHRASE: &[&str] = &["that", "players", "next", "upkeep"];
const THAT_PLAYERS_NEXT_END_STEP_PHRASE: &[&str] = &["that", "players", "next", "end", "step"];
const END_STEP_OF_THAT_PLAYERS_NEXT_TURN_PHRASE: &[&str] =
    &["end", "step", "of", "that", "players", "next", "turn"];
const THAT_TURN_DELAYED_STEP_PHRASES: &[&[&str]] = &[
    THAT_TURNS_END_STEP_PHRASE,
    THAT_PLAYERS_NEXT_UPKEEP_PHRASE,
    THAT_PLAYERS_NEXT_END_STEP_PHRASE,
    END_STEP_OF_THAT_PLAYERS_NEXT_TURN_PHRASE,
];
const NEXT_END_STEP_PHRASE: &[&str] = &["next", "end", "step"];
const NEXT_TURNS_END_STEP_PHRASE: &[&str] = &["next", "turns", "end", "step"];
const NEXT_END_STEP_PHRASES: &[&[&str]] = &[NEXT_END_STEP_PHRASE, NEXT_TURNS_END_STEP_PHRASE];
const YOUR_NEXT_UPKEEP_PHRASE: &[&str] = &["your", "next", "upkeep"];
const YOUR_NEXT_DRAW_STEP_PHRASE: &[&str] = &["your", "next", "draw", "step"];

fn is_upkeep_or_end_step_trigger(trigger: &Trigger) -> bool {
    matches!(
        trigger.kind,
        TriggerKind::BeginningOfUpkeep { .. } | TriggerKind::BeginningOfEndStep { .. }
    )
}

fn spell_battlefield_trigger_text_implies_delayed_schedule(
    ability_text: &str,
    trigger: &Trigger,
) -> Option<bool> {
    if !is_upkeep_or_end_step_trigger(trigger) {
        return None;
    }

    let tokens = lex_line(ability_text, 0).ok()?;
    let words = token_word_refs(&tokens);

    if word_slice_contains_any_phrase(&words, NEXT_UPKEEP_PHRASES) {
        return Some(true);
    }
    if word_slice_contains_any_phrase(&words, THAT_TURN_DELAYED_STEP_PHRASES) {
        return Some(true);
    }
    if word_slice_contains_any_phrase(&words, NEXT_END_STEP_PHRASES) {
        return Some(false);
    }

    None
}

fn convert_nonpermanent_delayed_triggered_ability_to_spell_effect(
    ability: &Ability,
    original_text: &str,
) -> Option<Effect> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }

    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if !triggered.choices.is_empty() || triggered.intervening_if.is_some() {
        return None;
    }

    let (ability_text, start_next_turn) = original_text.lines().find_map(|line| {
        let line = line.trim();
        let start_next_turn =
            spell_battlefield_trigger_text_implies_delayed_schedule(line, &triggered.trigger)?;
        Some((line, start_next_turn))
    })?;
    let trigger = delayed_trigger_spec_from_trigger(&triggered.trigger, Some(ability_text))?;

    let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
        trigger,
        triggered.effects.clone().to_vec(),
        true,
        Vec::new(),
        PlayerFilter::You,
    );
    if start_next_turn {
        delayed = delayed.starting_next_turn();
    }

    Some(Effect::new(delayed))
}

fn delayed_trigger_spec_from_trigger(
    trigger: &Trigger,
    ability_text: Option<&str>,
) -> Option<ironsmith_core::DelayedTriggerSpec> {
    let ability_tokens = ability_text
        .and_then(|text| lex_line(text, 0).ok())
        .unwrap_or_default();
    let ability_words = token_word_refs(&ability_tokens);

    match trigger.kind {
        TriggerKind::BeginningOfUpkeep { .. } => {
            let player = if word_slice_contains_phrase(&ability_words, YOUR_NEXT_UPKEEP_PHRASE) {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            Some(ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(
                player,
            ))
        }
        TriggerKind::BeginningOfDrawStep { .. } => {
            let player = if word_slice_contains_phrase(&ability_words, YOUR_NEXT_DRAW_STEP_PHRASE) {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            Some(ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(
                player,
            ))
        }
        TriggerKind::BeginningOfEndStep { .. } => Some(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
        ),
        TriggerKind::EndOfCombat => Some(ironsmith_core::DelayedTriggerSpec::EndOfCombat),
        TriggerKind::ThisDies => Some(ironsmith_core::DelayedTriggerSpec::ThisDies),
        _ => None,
    }
}

fn finalize_nonpermanent_delayed_triggered_abilities(
    mut definition: CardDefinition,
    original_text: &str,
) -> CardDefinition {
    if !definition.card.is_instant() && !definition.card.is_sorcery() {
        return definition;
    }

    let mut rewritten_effects = Vec::new();
    let mut remaining_abilities = Vec::with_capacity(definition.abilities.len());
    for ability in std::mem::take(&mut definition.abilities) {
        if let Some(effect) =
            convert_nonpermanent_delayed_triggered_ability_to_spell_effect(&ability, original_text)
        {
            rewritten_effects.push(effect);
        } else {
            remaining_abilities.push(ability);
        }
    }

    definition.abilities = remaining_abilities;
    if !rewritten_effects.is_empty() {
        definition
            .spell_effect
            .get_or_insert_with(ResolutionProgram::default)
            .extend(ResolutionProgram::from_effects(rewritten_effects));
    }
    definition
}

fn semantic_text(original_text: &str) -> String {
    original_text
        .to_ascii_lowercase()
        .replace('’', "'")
        .replace('\n', " ")
}

fn has_all(text: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| text.contains(needle))
}

fn is_source_enters_battlefield_trigger(trigger: &Trigger) -> bool {
    match &trigger.kind {
        TriggerKind::ThisEntersBattlefield => true,
        TriggerKind::ZoneChange(zone_change) => {
            zone_change.this && zone_change.to == Some(Zone::Battlefield)
        }
        TriggerKind::EntersBattlefield { filter, .. } => filter.source,
        _ => false,
    }
}

fn target_player(filter: PlayerFilter) -> ChooseSpec {
    ChooseSpec::target(ChooseSpec::Player(filter))
}

fn target_creature() -> ChooseSpec {
    ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
}

fn target_creature_you_control_other() -> ChooseSpec {
    ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature().you_control().other(),
    ))
}

fn tagged_filter(tag: &str) -> ObjectFilter {
    ObjectFilter::tagged(TagKey::from(tag))
}

fn combined_filter(filters: Vec<ObjectFilter>) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = filters;
    filter
}

fn target_only(tag: &str, spec: ChooseSpec) -> Effect {
    Effect::new(crate::effects::TaggedEffect::new(
        TagKey::from(tag),
        Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
    ))
}

fn tagged(tag: &str, effect: Effect) -> Effect {
    Effect::new(crate::effects::TaggedEffect::new(TagKey::from(tag), effect))
}

fn with_id(id: u32, effect: Effect) -> Effect {
    Effect::new(crate::effects::WithIdEffect::new(EffectId(id), effect))
}

fn choose_objects(filter: ObjectFilter, count: usize, chooser: PlayerFilter, tag: &str) -> Effect {
    Effect::new(
        crate::effects::ChooseObjectsEffect::new(filter, count, chooser, TagKey::from(tag))
            .in_zone(Zone::Battlefield),
    )
}

fn continuous_effect(
    target_filter: ObjectFilter,
    target_spec: Option<ChooseSpec>,
    modification: Option<Modification>,
    runtime_modifications: Vec<crate::effects::continuous::RuntimeModification>,
    until: Until,
    lock_filter_at_resolution: bool,
) -> Effect {
    Effect::new(crate::effects::ApplyContinuousEffect {
        target: EffectTarget::Filter(target_filter),
        target_spec,
        modification,
        additional_modifications: Vec::new(),
        runtime_modifications,
        until,
        condition: None,
        source_type: None,
        source_reference_surface: None,
        lock_filter_at_resolution,
        resolve_set_pt_values_at_resolution: false,
        require_creature_target: false,
    })
}

fn target_continuous_effect(
    target_spec: ChooseSpec,
    modification: Option<Modification>,
    runtime_modifications: Vec<crate::effects::continuous::RuntimeModification>,
    until: Until,
    require_creature_target: bool,
) -> Effect {
    Effect::new(crate::effects::ApplyContinuousEffect {
        target: target_spec.clone().into(),
        target_spec: Some(target_spec),
        modification,
        additional_modifications: Vec::new(),
        runtime_modifications,
        until,
        condition: None,
        source_type: None,
        source_reference_surface: None,
        lock_filter_at_resolution: false,
        resolve_set_pt_values_at_resolution: false,
        require_creature_target,
    })
}

fn map_effect_tree(effect: &Effect, leaf: &impl Fn(&Effect) -> Option<Effect>) -> Effect {
    if let Some(rewritten) = leaf(effect) {
        return rewritten;
    }

    if let Some(tagged_effect) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(crate::effects::TaggedEffect::new(
            tagged_effect.tag.clone(),
            map_effect_tree(&tagged_effect.effect, leaf),
        ));
    }
    if let Some(with_id_effect) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Effect::new(crate::effects::WithIdEffect::new(
            with_id_effect.id,
            map_effect_tree(&with_id_effect.effect, leaf),
        ));
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<Effect>>() {
        return Effect::new(crate::effects::MayEffect {
            decider: may.decider.clone(),
            effects: may
                .effects
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
        });
    }
    if let Some(unless) = effect.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>() {
        return Effect::new(crate::effects::UnlessPaysEffect {
            player: unless.player.clone(),
            effects: unless
                .effects
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
            cost: unless.cost.clone(),
        });
    }
    if let Some(delayed) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        let mut delayed = delayed.clone();
        delayed.effects = delayed
            .effects
            .iter()
            .map(|effect| map_effect_tree(effect, leaf))
            .collect();
        return Effect::new(delayed);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return Effect::new(crate::effects::ConditionalEffect::new(
            conditional.condition.clone(),
            conditional
                .if_true
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
            conditional
                .if_false
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
        ));
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return Effect::new(crate::effects::IfEffect::new(
            if_effect.condition,
            if_effect.predicate.clone(),
            if_effect
                .then
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
            if_effect
                .else_
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
        ));
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect<Effect>>() {
        return Effect::new(crate::effects::ForPlayersEffect {
            filter: for_players.filter.clone(),
            effects: for_players
                .effects
                .iter()
                .map(|effect| map_effect_tree(effect, leaf))
                .collect(),
            starting_with_controller: for_players.starting_with_controller,
            stop_after_first_happened: for_players.stop_after_first_happened,
        });
    }

    effect.clone()
}

fn rewrite_program(program: &mut ResolutionProgram, leaf: impl Fn(&Effect) -> Option<Effect>) {
    let rewritten = program
        .clone()
        .try_map_effects(|effect| Ok::<_, ()>(map_effect_tree(&effect, &leaf)))
        .expect("infallible effect rewrite");
    *program = rewritten;
}

fn effect_is_damage_to_player_or_planeswalker(effect: &Effect, amount: Value) -> bool {
    effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .is_some_and(|damage| {
            damage.amount == amount && matches!(damage.target, ChooseSpec::PlayerOrPlaneswalker(_))
        })
}

fn fix_quenchable_fire(definition: &mut CardDefinition) {
    if let Some(program) = &mut definition.spell_effect {
        let has_initial_player_or_planeswalker_damage = program.segments.iter().any(|segment| {
            segment
                .default_effects
                .iter()
                .any(|effect| effect_is_damage_to_player_or_planeswalker(effect, Value::Fixed(3)))
        });
        if !has_initial_player_or_planeswalker_damage {
            return;
        }
        rewrite_program(program, |effect| {
            let damage = effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
            if damage.amount != Value::Fixed(3) {
                return None;
            }
            let ChooseSpec::Object(filter) = &damage.target else {
                return None;
            };
            if filter.card_types.as_slice() != [CardType::Planeswalker] {
                return None;
            }
            let mut repaired = damage.clone();
            repaired.target =
                ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::TargetPlayerOrControllerOfTarget);
            Some(Effect::new(repaired))
        });
    }
}

fn choose_spec_is_tagged(spec: &ChooseSpec, tag: &TagKey) -> bool {
    matches!(spec, ChooseSpec::Tagged(found) if found == tag)
}

fn is_target_opponent_nonland_hand_choice(choose: &crate::effects::ChooseObjectsEffect) -> bool {
    choose.zone == Some(Zone::Hand)
        && choose.filter.zone == Some(Zone::Hand)
        && choose.filter.owner == Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        && choose.filter.excluded_card_types.contains(&CardType::Land)
        && choose.count.is_single()
        && !choose.is_search
}

fn is_matching_exile_move(effect: &Effect, tag: &TagKey) -> bool {
    effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some_and(|move_to_zone| {
            move_to_zone.zone == Zone::Exile && choose_spec_is_tagged(&move_to_zone.target, tag)
        })
}

fn effect_already_tagged(effect: &Effect, tag: &str) -> bool {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .is_some_and(|tagged| tagged.tag.as_str() == tag)
}

fn fix_pick_the_brain(definition: &mut CardDefinition) {
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    for segment in &mut program.segments {
        let mut selected_tag = None;
        for effect in &segment.default_effects {
            if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
                && is_target_opponent_nonland_hand_choice(choose)
            {
                selected_tag = Some(choose.tag.clone());
                break;
            }
        }
        let Some(selected_tag) = selected_tag else {
            continue;
        };
        for effect in &mut segment.default_effects {
            if is_matching_exile_move(effect, &selected_tag)
                && !effect_already_tagged(effect, "__source_exiled__")
            {
                *effect = tagged("__source_exiled__", effect.clone());
                return;
            }
        }
    }
}

fn fix_shadow_of_the_grave(definition: &mut CardDefinition, text: &str) {
    if !text.contains("all cards in your graveyard that you cycled or discarded this turn") {
        return;
    }
    if let Some(program) = &mut definition.spell_effect {
        rewrite_program(program, |effect| {
            let return_to_hand = effect.downcast_ref::<crate::effects::ReturnToHandEffect>()?;
            let ChooseSpec::All(filter) = &return_to_hand.spec else {
                return None;
            };
            if filter.zone != Some(Zone::Graveyard) || filter.owner != Some(PlayerFilter::You) {
                return None;
            }
            let mut repaired = return_to_hand.clone();
            let mut repaired_filter = filter.clone();
            repaired_filter.discarded_or_cycled_this_turn_by = Some(PlayerFilter::You);
            repaired.spec = ChooseSpec::All(repaired_filter);
            Some(Effect::new(repaired))
        });
    }
}

fn is_creature_first_match_consult(consult: &crate::effects::ConsultTopOfLibraryEffect) -> bool {
    consult.mode == crate::effects::LibraryConsultMode::Reveal
        && matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        && consult.filter.card_types.as_slice() == [CardType::Creature]
}

fn fix_telemin_performance(definition: &mut CardDefinition) {
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    for segment in &mut program.segments {
        let match_tag = segment.default_effects.iter().find_map(|effect| {
            let consult = effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
            if !is_creature_first_match_consult(consult) {
                return None;
            }
            Some(consult.match_tag.clone())
        });
        let Some(match_tag) = match_tag else {
            continue;
        };
        let already_puts = segment.default_effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
                .is_some()
        });
        if !already_puts {
            segment.default_effects.push(Effect::new(
                crate::effects::PutOntoBattlefieldEffect::you_control(
                    ChooseSpec::Tagged(match_tag),
                    false,
                ),
            ));
        }
    }
    *program = ResolutionProgram::new(program.segments.clone());
}

fn is_any_player_choose_then_self_sacrifice_model(triggered: &TriggeredAbility) -> bool {
    let has_any_player_may_choose_creatures = triggered.effects.segments.iter().any(|segment| {
        segment.default_effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::WithIdEffect>()
                .and_then(|with_id| {
                    with_id
                        .effect
                        .downcast_ref::<crate::effects::MayEffect<Effect>>()
                })
                .is_some_and(|may| {
                    may.decider == Some(PlayerFilter::Any)
                        && may.effects.iter().any(|inner| {
                            inner
                                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                                .is_some_and(|choose| {
                                    choose.filter.card_types.as_slice() == [CardType::Creature]
                                        && choose.filter.controller == Some(PlayerFilter::Any)
                                        && choose.count.min == 2
                                        && choose.count.max == Some(2)
                                })
                        })
                })
        })
    });
    let sacrifices_source_if_choice_happened = triggered.effects.segments.iter().any(|segment| {
        segment.default_effects.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::IfEffect>()
                .is_some_and(|if_effect| {
                    if_effect.predicate == EffectPredicate::Happened
                        && if_effect.then.iter().any(|then_effect| {
                            then_effect
                                .downcast_ref::<crate::effects::SacrificeTargetEffect>()
                                .is_some_and(|sacrifice| sacrifice.target == ChooseSpec::Source)
                        })
                })
        })
    });
    has_any_player_may_choose_creatures && sacrifices_source_if_choice_happened
}

fn fix_prowling_pangolin(definition: &mut CardDefinition) {
    let sacrifice_tag = "__semantic_sacrificed_creatures__";
    for ability in &mut definition.abilities {
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            continue;
        };
        if !is_source_enters_battlefield_trigger(&triggered.trigger) {
            continue;
        }
        if !is_any_player_choose_then_self_sacrifice_model(triggered) {
            continue;
        }
        let choice_filter = ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer);
        let sacrifice_filter = tagged_filter(sacrifice_tag);
        let offer = Effect::new(crate::effects::MayEffect {
            decider: Some(PlayerFilter::IteratedPlayer),
            effects: vec![
                choose_objects(
                    choice_filter,
                    2,
                    PlayerFilter::IteratedPlayer,
                    sacrifice_tag,
                ),
                Effect::new(crate::effects::SacrificePlayerEffect::new(
                    sacrifice_filter,
                    Value::Fixed(2),
                    PlayerFilter::IteratedPlayer,
                )),
            ],
        });
        triggered.effects = ResolutionProgram::from_effects(vec![
            with_id(
                0,
                Effect::new(crate::effects::ForPlayersEffect {
                    filter: PlayerFilter::Any,
                    effects: vec![offer],
                    starting_with_controller: true,
                    stop_after_first_happened: true,
                }),
            ),
            Effect::new(crate::effects::IfEffect::if_then(
                EffectId(0),
                EffectPredicate::Happened,
                vec![Effect::new(crate::effects::SacrificeTargetEffect::source())],
            )),
        ]);
    }
}

fn fix_tunnel_ignus(definition: &mut CardDefinition, text: &str) {
    if !text.contains(
        "if that player had another land enter the battlefield under their control this turn",
    ) {
        return;
    }
    for ability in &mut definition.abilities {
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            continue;
        };
        if !matches!(
            triggered.trigger.kind,
            TriggerKind::EntersBattlefield { .. }
        ) {
            continue;
        }
        triggered.intervening_if = Some(Condition::ValueComparison {
            left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::ControllerOf(
                ObjectRef::tagged("triggering"),
            )),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        });
    }
}

fn fix_kusari_gama(definition: &mut CardDefinition, text: &str) {
    if !text.contains("equipped creature deals damage to a blocking creature") {
        return;
    }
    for ability in &mut definition.abilities {
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            continue;
        };
        let TriggerKind::DealsDamage { filter } = &triggered.trigger.kind else {
            continue;
        };
        let mut blocking_creature = ObjectFilter::creature();
        blocking_creature.blocking = true;
        triggered.trigger = Trigger::deals_damage_to(filter.clone(), blocking_creature);
    }
}

fn fix_multanis_presence(definition: &mut CardDefinition, text: &str) {
    if !text.contains("whenever a spell you've cast is countered") {
        return;
    }
    for ability in &mut definition.abilities {
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            continue;
        };
        let filter = match &triggered.trigger.kind {
            TriggerKind::SpellCast { filter, .. }
            | TriggerKind::SpellCastQualified { filter, .. } => filter.clone(),
            _ => continue,
        };
        triggered.trigger = Trigger::spell_countered(filter, PlayerFilter::You);
    }
}

fn fix_emet_selch(definition: &mut CardDefinition, text: &str) {
    if !text.contains("one or more opponents lose life") {
        return;
    }
    for ability in &mut definition.abilities {
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            continue;
        };
        if matches!(
            triggered.trigger.kind,
            TriggerKind::PlayerLosesLife {
                player: PlayerFilter::Opponent
            }
        ) {
            triggered.trigger = Trigger::players_lose_life_one_or_more(PlayerFilter::Opponent);
        }
    }
}

fn fix_serpentine_spike(definition: &mut CardDefinition, text: &str) {
    if !text.contains("deals 2 damage to target creature, 3 damage to another target creature, and 4 damage to a third target creature") {
        return;
    }
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    for segment in &mut program.segments {
        let mut rest = Vec::new();
        for effect in std::mem::take(&mut segment.default_effects) {
            if effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .is_some()
                || effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| {
                        tagged
                            .effect
                            .downcast_ref::<crate::effects::DealDamageEffect>()
                    })
                    .is_some()
            {
                continue;
            }
            rest.push(effect);
        }
        let mut second = ObjectFilter::creature();
        second = second.not_tagged("damaged_0");
        let mut third = ObjectFilter::creature();
        third = third.not_tagged("damaged_0");
        segment.default_effects = vec![
            tagged(
                "damaged_0",
                Effect::new(crate::effects::DealDamageEffect::new(2, target_creature())),
            ),
            tagged(
                "damaged_0",
                Effect::new(crate::effects::DealDamageEffect::new(
                    3,
                    ChooseSpec::target(ChooseSpec::Object(second)),
                )),
            ),
            tagged(
                "damaged_0",
                Effect::new(crate::effects::DealDamageEffect::new(
                    4,
                    ChooseSpec::target(ChooseSpec::Object(third)),
                )),
            ),
        ];
        segment.default_effects.extend(rest);
    }
    *program = ResolutionProgram::new(program.segments.clone());
}

fn fix_fistful_of_force(definition: &mut CardDefinition, text: &str) {
    if !text.contains("if you win, that creature gets an additional +2/+2 and gains trample") {
        return;
    }
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    for segment in &mut program.segments {
        let Some(clash_idx) = segment.default_effects.iter().position(|effect| {
            effect
                .downcast_ref::<crate::effects::ClashEffect>()
                .is_some()
        }) else {
            continue;
        };
        let target = ChooseSpec::Tagged(TagKey::from("pumped_0"));
        let extra_pump = target_continuous_effect(
            target.clone(),
            None,
            vec![
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power: Value::Fixed(2),
                    toughness: Value::Fixed(2),
                },
            ],
            Until::EndOfTurn,
            true,
        );
        let trample = target_continuous_effect(
            target,
            Some(Modification::AddAbility(StaticAbility::trample())),
            Vec::new(),
            Until::EndOfTurn,
            false,
        );
        segment.default_effects[clash_idx] = with_id(0, segment.default_effects[clash_idx].clone());
        segment.default_effects.retain(|effect| {
            !effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
                .and_then(|apply| apply.modification.as_ref())
                .is_some_and(|modification| {
                    matches!(modification, Modification::AddAbility(ability) if ability.id() == crate::static_abilities::StaticAbilityId::Trample)
                })
        });
        segment
            .default_effects
            .push(Effect::new(crate::effects::IfEffect::if_then(
                EffectId(0),
                EffectPredicate::Happened,
                vec![extra_pump, trample],
            )));
    }
    *program = ResolutionProgram::new(program.segments.clone());
}

fn fix_hisokas_guard(definition: &mut CardDefinition, text: &str) {
    if !text.contains("has shroud for as long as this creature remains tapped") {
        return;
    }
    for ability in &mut definition.abilities {
        let AbilityKind::Activated(activated) = &mut ability.kind else {
            continue;
        };
        let target = target_creature_you_control_other();
        activated.choices = vec![target.clone()];
        activated.effects = ResolutionProgram::from_effects(vec![target_continuous_effect(
            target,
            Some(Modification::AddAbility(StaticAbility::shroud())),
            Vec::new(),
            Until::SourceUntaps,
            false,
        )]);
    }
}

fn fix_tempt_with_mayhem(definition: &mut CardDefinition, text: &str) {
    if !has_all(
        text,
        &[
            "each opponent may copy that spell",
            "once plus an additional time for each opponent who copied the spell this way",
        ],
    ) {
        return;
    }
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    let spell_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        has_mana_cost: true,
        ..Default::default()
    };
    let opponent_copy = with_id(
        1,
        Effect::new(crate::effects::CopySpellEffect::new_for_player(
            ChooseSpec::Tagged(TagKey::from("targeted_0")),
            1,
            PlayerFilter::IteratedPlayer,
        )),
    );
    let opponent_offer = Effect::new(crate::effects::MayEffect {
        decider: Some(PlayerFilter::IteratedPlayer),
        effects: vec![
            opponent_copy,
            Effect::new(crate::effects::ChooseNewTargetsEffect::may_for_player(
                EffectId(1),
                PlayerFilter::IteratedPlayer,
            )),
        ],
    });
    let you_copy_count = Value::EffectMetricOffset {
        effect_id: EffectId(0),
        source: EffectMetricSource::Outcome,
        metric: EffectMetric::PlayersWithPositiveCount,
        offset: 1,
    };
    let you_copy = with_id(
        2,
        Effect::new(crate::effects::CopySpellEffect::new(
            ChooseSpec::Tagged(TagKey::from("targeted_0")),
            you_copy_count,
        )),
    );
    for segment in &mut program.segments {
        segment.default_effects = vec![
            target_only(
                "targeted_0",
                ChooseSpec::target(ChooseSpec::Object(spell_filter.clone())),
            ),
            with_id(
                0,
                Effect::new(crate::effects::ForPlayersEffect {
                    filter: PlayerFilter::Opponent,
                    effects: vec![opponent_offer.clone()],
                    starting_with_controller: true,
                    stop_after_first_happened: false,
                }),
            ),
            you_copy.clone(),
            Effect::new(crate::effects::ChooseNewTargetsEffect::may(EffectId(2))),
        ];
    }
    *program = ResolutionProgram::new(program.segments.clone());
}

fn fix_twist_allegiance(definition: &mut CardDefinition, text: &str) {
    if !text.contains("you and target opponent each gain control of all creatures the other controls until end of turn") {
        return;
    }
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    let target_opponent = PlayerFilter::Target(Box::new(PlayerFilter::Opponent));
    let your_creatures = ObjectFilter::creature().you_control();
    let opponent_creatures = ObjectFilter::creature().controlled_by(target_opponent.clone());
    let your_tagged = tagged_filter("__twist_your_creatures__");
    let opponent_tagged = tagged_filter("__twist_opponent_creatures__");
    let both = combined_filter(vec![your_tagged.clone(), opponent_tagged.clone()]);
    for segment in &mut program.segments {
        segment.default_effects = vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(target_player(
                PlayerFilter::Opponent,
            ))),
            Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                your_creatures.clone(),
                "__twist_your_creatures__",
            )),
            Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                opponent_creatures.clone(),
                "__twist_opponent_creatures__",
            )),
            continuous_effect(
                opponent_tagged.clone(),
                Some(ChooseSpec::All(opponent_tagged.clone())),
                None,
                vec![crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController],
                Until::EndOfTurn,
                true,
            ),
            continuous_effect(
                your_tagged.clone(),
                Some(ChooseSpec::All(your_tagged.clone())),
                None,
                vec![crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    target_opponent.clone(),
                )],
                Until::EndOfTurn,
                true,
            ),
            Effect::new(crate::effects::UntapEffect::all(both.clone())),
            continuous_effect(
                both.clone(),
                Some(ChooseSpec::All(both.clone())),
                Some(Modification::AddAbility(StaticAbility::haste())),
                Vec::new(),
                Until::EndOfTurn,
                true,
            ),
        ];
    }
    *program = ResolutionProgram::new(program.segments.clone());
}

fn fix_forced_block_spells(definition: &mut CardDefinition, text: &str) {
    if text.contains("target creature blocks target creature this turn if able") {
        let Some(program) = &mut definition.spell_effect else {
            return;
        };
        for segment in &mut program.segments {
            segment.default_effects = vec![
                target_only("targeted_blocker", target_creature()),
                target_only("targeted_attacker", target_creature()),
                Effect::new(crate::effects::CantEffect::until_end_of_turn(
                    crate::effect::Restriction::must_block_specific_attacker(
                        tagged_filter("targeted_blocker"),
                        tagged_filter("targeted_attacker"),
                    ),
                )),
            ];
        }
        *program = ResolutionProgram::new(program.segments.clone());
    }

    if text.contains("target creature defending player controls blocks it this combat if able") {
        for ability in &mut definition.abilities {
            let AbilityKind::Triggered(triggered) = &mut ability.kind else {
                continue;
            };
            if !matches!(triggered.trigger.kind, TriggerKind::ThisAttacks) {
                continue;
            }
            let blocker_filter = ObjectFilter::creature().controlled_by(PlayerFilter::Defending);
            triggered.effects = ResolutionProgram::from_effects(vec![
                Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
                target_only(
                    "targeted_blocker",
                    ChooseSpec::target(ChooseSpec::Object(blocker_filter)),
                ),
                Effect::new(crate::effects::CantEffect::new(
                    crate::effect::Restriction::must_block_specific_attacker(
                        tagged_filter("targeted_blocker"),
                        tagged_filter("triggering"),
                    ),
                    Until::EndOfCombat,
                )),
            ]);
        }
    }
}

fn fix_march_from_velis_vel(definition: &mut CardDefinition, text: &str) {
    if !text.contains("choose a nonbasic land type")
        || !text.contains(
            "each land you control of that type becomes a copy of target creature you control",
        )
    {
        return;
    }
    let Some(program) = &mut definition.spell_effect else {
        return;
    };
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let land_filter = ObjectFilter::land()
        .you_control()
        .nonbasic()
        .of_chosen_land_type();
    for segment in &mut program.segments {
        segment.default_effects = vec![
            target_only("targeted_0", target.clone()),
            Effect::new(crate::effects::ChooseLandTypeEffect::new(
                PlayerFilter::You,
                true,
            )),
            continuous_effect(
                land_filter.clone(),
                Some(ChooseSpec::All(land_filter.clone())),
                None,
                vec![crate::effects::continuous::RuntimeModification::CopyOf {
                    source: ChooseSpec::Tagged(TagKey::from("targeted_0")),
                    preserve_source_abilities: false,
                    name_override: None,
                    name_override_surface: None,
                    add_supertypes: Vec::new(),
                }],
                Until::EndOfTurn,
                true,
            ),
            continuous_effect(
                land_filter.clone(),
                Some(ChooseSpec::All(land_filter.clone())),
                Some(Modification::AddAbility(StaticAbility::haste())),
                Vec::new(),
                Until::EndOfTurn,
                true,
            ),
        ];
    }
    *program = ResolutionProgram::new(program.segments.clone());
}

fn finalize_shape_driven_semantic_repairs(mut definition: CardDefinition) -> CardDefinition {
    fix_quenchable_fire(&mut definition);
    fix_pick_the_brain(&mut definition);
    fix_prowling_pangolin(&mut definition);
    fix_telemin_performance(&mut definition);
    definition
}

fn finalize_source_text_semantic_repairs(
    mut definition: CardDefinition,
    original_text: &str,
) -> CardDefinition {
    let text = semantic_text(original_text);
    fix_tunnel_ignus(&mut definition, &text);
    fix_kusari_gama(&mut definition, &text);
    fix_multanis_presence(&mut definition, &text);
    fix_shadow_of_the_grave(&mut definition, &text);
    fix_tempt_with_mayhem(&mut definition, &text);
    fix_twist_allegiance(&mut definition, &text);
    fix_fistful_of_force(&mut definition, &text);
    fix_hisokas_guard(&mut definition, &text);
    fix_emet_selch(&mut definition, &text);
    fix_serpentine_spike(&mut definition, &text);
    fix_forced_block_spells(&mut definition, &text);
    fix_march_from_velis_vel(&mut definition, &text);
    definition
}

pub(crate) fn apply(
    definition: CardDefinition,
    original_builder: &CardDefinitionBuilder,
    original_text: &str,
) -> Result<CardDefinition, CardTextError> {
    let definition = finalize_overload_definitions(definition, original_builder, original_text)?;
    let definition = finalize_backup_abilities(definition);
    let definition = finalize_cipher_effects(definition);
    let definition = finalize_squad_abilities(definition);
    let definition = finalize_offspring_abilities(definition);
    let definition = finalize_shape_driven_semantic_repairs(definition);
    let definition = finalize_source_text_semantic_repairs(definition, original_text);
    Ok(finalize_nonpermanent_delayed_triggered_abilities(
        definition,
        original_text,
    ))
}
