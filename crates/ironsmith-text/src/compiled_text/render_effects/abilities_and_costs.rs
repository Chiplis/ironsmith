use super::*;
use crate::target::SourceReferenceSurface;

fn triggered_named_source_resolution_surface(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<&str> {
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [prefix @ .., execute_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    if prefix.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_none()
            && effect
                .downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()
                .is_none()
            && effect
                .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                .is_none()
    }) {
        return None;
    }
    let execute = execute_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    match execute.source.source_reference_surface()? {
        SourceReferenceSurface::FullName(text) | SourceReferenceSurface::ShortName(text) => {
            Some(text)
        }
        SourceReferenceSurface::ThisPermanentType(_) => None,
    }
}

fn triggered_resolution_clause_case(
    triggered: &crate::ability::TriggeredAbility,
    clause: &str,
) -> String {
    if triggered_named_source_resolution_surface(triggered).is_some_and(|source| {
        clause
            .strip_prefix(source)
            .is_some_and(|tail| tail.chars().next().is_some_and(char::is_whitespace))
    }) {
        clause.to_string()
    } else {
        lowercase_first(clause)
    }
}

#[cfg(test)]
mod named_source_resolution_case_tests {
    use super::*;

    fn fixture(surface: SourceReferenceSurface) -> crate::ability::TriggeredAbility {
        let source = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(surface),
        );
        crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::this_enters_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
                Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                    source,
                    Effect::deal_damage(
                        Value::Fixed(2),
                        ChooseSpec::Player(PlayerFilter::Opponent),
                    ),
                )),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }
    }

    #[test]
    fn explicit_named_source_keeps_case_but_typed_and_unrelated_heads_do_not() {
        let named = fixture(SourceReferenceSurface::ShortName(
            "Ghyrson Starn".to_string(),
        ));
        assert_eq!(
            triggered_resolution_clause_case(&named, "Ghyrson Starn deals 2 damage to that player"),
            "Ghyrson Starn deals 2 damage to that player"
        );
        assert_eq!(
            triggered_resolution_clause_case(&named, "Draw a card"),
            "draw a card"
        );

        let typed = fixture(SourceReferenceSurface::ThisPermanentType(
            "This creature".to_string(),
        ));
        assert_eq!(
            triggered_resolution_clause_case(&typed, "This creature deals 2 damage"),
            "this creature deals 2 damage"
        );
    }
}

fn rewrite_removed_counter_type_surface(
    mut effects: String,
    costs: &[crate::costs::Cost],
) -> String {
    let Some(count_phrase) = removed_counters_this_way_x_phrase(costs) else {
        return effects;
    };
    let Some(counter_phrase) = count_phrase
        .strip_prefix("the number of ")
        .and_then(|phrase| phrase.strip_suffix(" removed this way"))
    else {
        return effects;
    };
    let singular = counter_phrase
        .strip_suffix(" counters")
        .map(|prefix| format!("{prefix} counter"))
        .unwrap_or_else(|| counter_phrase.to_string());
    effects = effects.replace(
        "for each counter removed this way",
        &format!("for each {singular} removed this way"),
    );
    effects
}

pub(super) fn describe_zone_change_triggering_card_to_your_library(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
        return None;
    }

    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()?;
    if trigger.from != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || trigger.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        || trigger.player != crate::triggers::zone_changes::PlayerRelation::Any
        || trigger.count_mode != crate::triggers::zone_changes::CountMode::Each
        || trigger.object_filter.owner.as_ref() != Some(&PlayerFilter::You)
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, move_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let tag_triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || !move_to_zone.to_top
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_zone.enters_tapped
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &tag_triggering.tag)
    {
        return None;
    }

    let mut subject_filter = trigger.object_filter.clone();
    subject_filter.owner = None;
    let mut subject = subject_filter.description();
    if subject == "object" {
        subject = "card".to_string();
    }
    subject = ensure_indefinite_article(&subject);

    Some(format!(
        "Whenever {subject} is put into your graveyard from the battlefield, put that card on top of your library"
    ))
}

pub(in crate::compiled_text) fn describe_annihilator_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
        return None;
    }
    if triggered
        .trigger
        .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
        .is_none()
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let (filter, player, count) = if let Some(sacrifice) =
        effect.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()
    {
        (&sacrifice.filter, &sacrifice.player, &sacrifice.count)
    } else if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        (&sacrifice.filter, &sacrifice.player, &sacrifice.count)
    } else {
        return None;
    };
    if filter != &ObjectFilter::permanent() || player != &PlayerFilter::Defending {
        return None;
    }
    let Value::Fixed(amount) = count else {
        return None;
    };
    Some(format!("Annihilator {amount}"))
}

/// Recognize the canonical runtime expansion of myriad. Keeping this typed
/// means copied or granted abilities can render the keyword without relying on
/// the incidental wording produced by the nested-effect renderer.
pub(super) fn describe_myriad_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
            .is_none()
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [for_players_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter
        != PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending)
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }

    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.is_some() || !matches!(&may.fallback, crate::decision::FallbackStrategy::Decline)
    {
        return None;
    }
    let [create_effect] = may.effects.as_slice() else {
        return None;
    };
    let create = create_effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if !matches!(create.target, ChooseSpec::Source)
        || !matches!(create.count.unhinted(), Value::Fixed(1))
        || create.controller != PlayerFilter::You
        || !create.enters_tapped
        || create.has_haste
        || create.loses_soulbond
        || !create.enters_attacking
        || !matches!(
            &create.attack_target_mode,
            Some(
                crate::effects::CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                    PlayerFilter::IteratedPlayer
                )
            )
        )
        || !create.exile_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.sacrifice_at_next_end_step_ability_text.is_some()
        || create.exile_at_next_end_step
        || create.next_end_step_player != PlayerFilter::Any
        || create.pt_adjustment.is_some()
        || create.clear_mana_cost
        || !create.added_card_types.is_empty()
        || !create.added_subtypes.is_empty()
        || !create.removed_supertypes.is_empty()
        || create.set_base_power_toughness.is_some()
        || create.set_base_power_toughness_value.is_some()
        || create.set_colors.is_some()
        || create.set_card_types.is_some()
        || create.set_subtypes.is_some()
        || !create.granted_static_abilities.is_empty()
    {
        return None;
    }

    Some("Myriad".to_string())
}

pub(super) fn describe_champion_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }

    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !trigger.this_object
        || trigger.from != crate::triggers::ZonePattern::Any
        || trigger.to != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || trigger.object_filter != ObjectFilter::default()
        || trigger.player != crate::triggers::PlayerRelation::Any
        || trigger.cause_filter.is_some()
        || trigger.count_mode != crate::triggers::CountMode::Each
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, unless_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let tag_triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let unless = unless_effect.downcast_ref::<crate::effects::UnlessActionEffect>()?;
    if unless.player != PlayerFilter::You {
        return None;
    }

    let [sacrifice_effect] = unless.effects.as_slice() else {
        return None;
    };
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(&sacrifice.target, ChooseSpec::Tagged(tag) if tag == &tag_triggering.tag) {
        return None;
    }

    let [alternative_effect] = unless.alternative.as_slice() else {
        return None;
    };
    let exile_until = alternative_effect.downcast_ref::<crate::effects::ExileUntilEffect>()?;
    if exile_until.duration != crate::effects::ExileUntilDuration::SourceLeavesBattlefield
        || exile_until.leave_watcher.is_some()
        || exile_until.return_zone != Zone::Battlefield
        || exile_until.face_down
    {
        return None;
    }
    let ChooseSpec::Object(filter) = &exile_until.spec else {
        return None;
    };
    if filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::You)
        || !filter.other
    {
        return None;
    }

    let target = describe_choose_spec(&exile_until.spec);
    let championed = target
        .strip_prefix("another ")
        .and_then(|text| text.strip_suffix(" you control"))?
        .trim();
    if championed.is_empty() {
        return None;
    }

    Some(format!("Champion {}", with_indefinite_article(championed)))
}

pub(super) fn describe_recover_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let cost = triggered.presentation_label.as_ref()?.recover_cost()?;
    Some(format!("Recover {cost}"))
}

pub(super) fn describe_backup_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }

    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !trigger.this_object
        || trigger.from != crate::triggers::ZonePattern::Any
        || trigger.to != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || trigger.object_filter != ObjectFilter::default()
        || trigger.player != crate::triggers::PlayerRelation::Any
        || trigger.cause_filter.is_some()
        || trigger.count_mode != crate::triggers::CountMode::Each
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let backup = effect.downcast_ref::<crate::effects::BackupEffect>()?;
    Some(format!("Backup {}", backup.amount))
}

pub(super) fn describe_class_level_activation(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    let level = activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix("__ironsmith_class_level:"))?;
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != crate::CounterType::Level || !matches!(put.target, ChooseSpec::Source) {
        return None;
    }
    Some(format!(
        "{}: Level {level}",
        describe_total_cost(&activated.mana_cost)
    ))
}

pub(super) fn describe_level_up_activation(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.choices.is_empty()
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
    {
        return None;
    }
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != crate::CounterType::Level || !matches!(put.target, ChooseSpec::Source) {
        return None;
    }
    Some(format!(
        "Level up {}",
        describe_total_cost(&activated.mana_cost)
    ))
}

fn describe_attacking_hand_entry_token_copy(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    let mut expected_trigger_filter = ObjectFilter::default();
    expected_trigger_filter.owner = Some(PlayerFilter::You);
    if !trigger.this_object
        || trigger.from != crate::triggers::ZonePattern::Specific(Zone::Hand)
        || trigger.to != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || trigger.object_filter != expected_trigger_filter
        || trigger.player != crate::triggers::PlayerRelation::Any
        || trigger.cause_filter.is_some()
        || trigger.during_turn.is_some()
        || trigger.origin_condition.is_some()
        || trigger.count_mode != crate::triggers::CountMode::Each
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let Condition::TaggedObjectMatches(condition_tag, condition_filter) =
        triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    let mut expected_condition = crate::target::ObjectFilter::default();
    expected_condition.attacking = true;
    if condition_tag.as_str() != "triggering" || condition_filter != &expected_condition {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if segment.starts_new_source_line || !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let create = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    let [choice] = triggered.choices.as_slice() else {
        return None;
    };
    if choice != &create.target
        || create.count.unhinted() != &Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || !create.enters_tapped
        || create.has_haste
        || create.haste_followup_reference_surface.is_some()
        || !create.enters_attacking
        || create.attack_target_mode.is_some()
        || create.exile_at_end_of_combat
        || create.exile_at_end_of_combat_reference_surface.is_some()
        || create.loses_soulbond
        || create.sacrifice_at_next_end_step
        || create
            .sacrifice_at_next_end_step_reference_surface
            .is_some()
        || create.sacrifice_at_next_end_step_ability_text.is_some()
        || create.exile_at_next_end_step
        || create.exile_at_next_end_step_reference_surface.is_some()
        || create.next_end_step_player != PlayerFilter::Any
        || create.pt_adjustment.is_some()
        || create.clear_mana_cost
        || !create.added_card_types.is_empty()
        || !create.added_subtypes.is_empty()
        || !create.removed_supertypes.is_empty()
        || create.set_base_power_toughness.is_some()
        || create.set_base_power_toughness_value.is_some()
        || create.set_colors.is_some()
        || create.set_card_types.is_some()
        || create.set_subtypes.is_some()
        || !create.granted_static_abilities.is_empty()
    {
        return None;
    }
    let target = describe_choose_spec(&create.target);
    if target != "another target attacking creature" {
        return None;
    }
    Some(format!(
        "When this creature enters from your hand, if it's attacking, create a token that's a copy of {target}. The token enters tapped and attacking"
    ))
}

fn describe_source_etb_damage_to_opponents_and_their_creatures(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !trigger.this_object
        || trigger.to != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || trigger.player != crate::triggers::PlayerRelation::Any
        || trigger.cause_filter.is_some()
        || trigger.count_mode != crate::triggers::CountMode::Each
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if segment.starts_new_source_line || !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, sequence_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let source_tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
        return None;
    }
    let [player_effect, creature_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let for_players = player_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [player_damage_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let player_damage = player_damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if for_players.filter != PlayerFilter::Opponent
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
        || !matches!(
            player_damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        || player_damage.source_is_combat
        || player_damage.unpreventable
    {
        return None;
    }

    let for_each = creature_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let mut expected_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
    expected_filter.controller = Some(PlayerFilter::Opponent);
    let mut semantic_filter = for_each.filter.clone();
    semantic_filter.union_surface = Default::default();
    if semantic_filter != expected_filter {
        return None;
    }
    let [iterated_damage_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let iterated_damage_effect = unwrap_basic_tag_wrappers(iterated_damage_effect);
    let sourced =
        iterated_damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(&sourced.source, ChooseSpec::Tagged(tag) if tag == &source_tag.tag) {
        return None;
    }
    let creature_damage = sourced
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if creature_damage.amount != player_damage.amount
        || !matches!(creature_damage.target, ChooseSpec::Iterated)
        || creature_damage.source_is_combat
        || creature_damage.unpreventable
    {
        return None;
    }
    let trigger_surface = triggered.trigger.display();
    if !(trigger_surface.starts_with("When ") || trigger_surface.starts_with("Whenever ")) {
        return None;
    }
    let amount = describe_value(&player_damage.amount);
    Some(format!(
        "{trigger_surface}, it deals {amount} damage to each opponent and {amount} damage to each creature your opponents control"
    ))
}

fn describe_each_combat_unless_pay_delayed_block_grant(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let combat = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfCombatTrigger>()?;
    if combat.player != PlayerFilter::Any
        || triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if segment.starts_new_source_line || !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let unless = effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless.player != PlayerFilter::You
        || unless.leading_surface
        || unless.before_delayed_step
        || describe_total_cost(&unless.cost) != "{R}"
    {
        return None;
    }
    let [schedule_effect] = unless.effects.as_slice() else {
        return None;
    };
    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if schedule.one_shot
        || schedule.start_next_turn
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::EndOfCombat
        || !schedule.until_end_of_combat
        || schedule.until_end_of_turn
        || !schedule.leading_duration_surface
        || schedule.watch_ability_source
        || schedule.watch_all_object_targets
        || schedule.either_of_watched_objects
        || schedule.while_any_tagged_object_in_zone.is_some()
        || !schedule.target_objects.is_empty()
        || schedule.target_tag.is_some()
        || schedule.target_filter.is_some()
        || schedule.controller != PlayerFilter::You
        || schedule.prepayment.is_some()
        || schedule.event_value_from_prior_prevention
    {
        return None;
    }
    if describe_this_blocks_or_becomes_blocked_by_trigger(&schedule.trigger).as_deref()
        != Some("Whenever this creature blocks or becomes blocked by a creature")
    {
        return None;
    }
    let [delayed_segment] = schedule.effects.segments.as_slice() else {
        return None;
    };
    if delayed_segment.starts_new_source_line || !delayed_segment.self_replacements.is_empty() {
        return None;
    }
    let [participant_effect, grant_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let participant =
        participant_effect.downcast_ref::<crate::effects::TagOtherBlockParticipantEffect>()?;
    if participant.filter.as_ref() != Some(&ObjectFilter::creature())
        || !super::structural_bundles::describes_first_strike_grant_to_tag(
            grant_effect,
            &participant.tag,
        )
    {
        return None;
    }
    Some(
        "At the beginning of each combat, unless you pay {R}, whenever this creature blocks or becomes blocked by a creature this combat, that creature gains first strike until end of turn"
            .to_string(),
    )
}

fn describe_spell_cast_copy_while_source_has_quest_counters(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !triggered.choices.is_empty() || triggered.presentation_label.is_some() {
        return None;
    }
    let spell_cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    if *spell_cast
        != crate::triggers::SpellCastTrigger::new(
            Some(ObjectFilter::instant_or_sorcery()),
            PlayerFilter::You,
        )
        || !matches!(
            triggered.intervening_if,
            Some(Condition::SourceHasCounterAtLeast {
                counter_type: crate::object::CounterType::Quest,
                count: 2,
                ..
            })
        )
    {
        return None;
    }
    let resolution = describe_triggered_resolution_text(triggered, "this enchantment", false)?;
    if resolution != "You may copy that spell. You may choose new targets for the copy" {
        return None;
    }
    Some(format!(
        "Whenever you cast an instant or sorcery spell while this enchantment has two or more quest counters on it, {resolution}"
    ))
}

/// Preserve an authored third-person self reference carried by the typed
/// damage source. The general permanent-ability pass normally expands
/// `It deals` to `This creature deals`, but that would erase an explicit
/// `it` surface retained by `ExecuteWithSourceEffect`.
fn activated_resolution_has_explicit_it_damage_source(
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    fn effect_has_explicit_it_damage_source(effect: &Effect) -> bool {
        if let Some(execute) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && matches!(
                execute.source.source_reference_surface(),
                Some(SourceReferenceSurface::ThisPermanentType(surface))
                    if surface.trim().eq_ignore_ascii_case("it")
            )
            && execute
                .effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .is_some()
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            if !found {
                found = effect_has_explicit_it_damage_source(child);
            }
        });
        found
    }

    if super::ast_render::resolution_uses_named_artifacts_damage_replacement(&activated.effects) {
        return true;
    }
    activated
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .any(effect_has_explicit_it_damage_source)
}

fn graveyard_self_exile_damage_uses_it_subject(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> bool {
    if ability.functional_zones.as_slice() != [Zone::Graveyard] {
        return false;
    }
    let cost = normalize_zone_bound_self_exile_cost(
        Some(describe_total_cost(&activated.mana_cost)),
        ability,
    );
    if !cost
        .as_deref()
        .is_some_and(|cost| cost.contains("Exile this card from your graveyard"))
    {
        return false;
    }
    if activated
        .effects
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return false;
    }
    let flattened = activated.effects.flattened_default_effects();
    let pair = match flattened {
        [first, second] => [first, second],
        [sequence_root] => {
            let Some(sequence) = sequence_root.downcast_ref::<crate::effects::SequenceEffect>()
            else {
                return false;
            };
            let [first, second] = sequence.effects.as_slice() else {
                return false;
            };
            [first, second]
        }
        _ => return false,
    };
    describe_player_or_planeswalker_damage_then_controlled_creature_damage(pair[0], pair[1])
        .is_some()
}

fn sacrifice_cost_copy_reference_noun(
    activated: &crate::ability::ActivatedAbility,
    target: &ChooseSpec,
) -> Option<&'static str> {
    let costs = activated.mana_cost.as_all()?;
    costs.iter().find_map(|cost| {
        let cost_effect = cost.downcast_ref::<crate::costs::CostEffect>()?;
        let tagged = cost_effect
            .effect
            .downcast_ref::<crate::effects::TaggedEffect>()?;
        if !choose_spec_references_exact_tag(target, &tagged.tag) {
            return None;
        }
        let sacrifice = tagged
            .effect
            .downcast_ref::<crate::effects::SacrificeEffect>()?;
        if sacrifice.player != PlayerFilter::You || sacrifice.count != Value::Fixed(1) {
            return None;
        }
        if sacrifice.filter.card_types.contains(&CardType::Creature)
            || sacrifice
                .filter
                .subtypes
                .iter()
                .any(crate::types::Subtype::is_creature_type)
        {
            Some("creature")
        } else if sacrifice.filter.card_types == [CardType::Artifact] {
            Some("artifact")
        } else if sacrifice.filter.card_types == [CardType::Land] {
            Some("land")
        } else {
            None
        }
    })
}

fn rewrite_sacrifice_cost_token_copy_reference(
    mut rendered: String,
    activated: &crate::ability::ActivatedAbility,
) -> String {
    let [effect] = activated.effects.flattened_default_effects() else {
        return rendered;
    };
    let Some(copy) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
    else {
        return rendered;
    };
    let Some(noun) = sacrifice_cost_copy_reference_noun(activated, &copy.target) else {
        return rendered;
    };
    rendered = rendered.replace(
        "tokens that are copies of it",
        &format!("tokens that are copies of the sacrificed {noun}"),
    );
    rendered.replace(
        "token that's a copy of it",
        &format!("token that's a copy of the sacrificed {noun}"),
    )
}

fn describe_exiled_last_time_counter_creatures_unblockable(ability: &Ability) -> Option<String> {
    if ability.functional_zones.as_slice() != [Zone::Exile] {
        return None;
    }
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || triggered.presentation_label.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let counter = triggered
        .trigger
        .downcast_ref::<crate::triggers::CounterRemovedFromTrigger>()?;
    if !counter.last || counter.counter_type != Some(CounterType::Time) || !counter.filter.source {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let cant = effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeBlocked(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn || filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }
    let mut plain = filter.clone();
    plain.zone = None;
    plain.card_types.clear();
    plain.set_explicit_card_type_noun(None);
    plain.set_plural_object_noun_surface(false);
    if plain != ObjectFilter::default() {
        return None;
    }
    let source = counter
        .filter
        .source_surface
        .as_ref()
        .map(crate::target::SourceReferenceSurface::display_text)
        .unwrap_or_else(|| "this card".to_string());
    Some(format!(
        "When the last time counter is removed from {source} while it's exiled, creatures can't be blocked this turn"
    ))
}

pub(crate) fn describe_ability(
    index: usize,
    ability: &Ability,
    subject: &str,
    rewrite_it_deals: bool,
) -> Vec<String> {
    if let Some(rendered) = describe_exiled_last_time_counter_creatures_unblockable(ability) {
        return vec![format!("Triggered ability {index}: {rendered}")];
    }
    if let Some(rendered) = describe_conditional_spell_uncounterability(ability) {
        return vec![format!("Static ability {index}: {rendered}")];
    }
    if let AbilityKind::Static(static_ability) = &ability.kind
        && matches!(
            static_ability.compiled_model().map(|model| &model.payload),
            Some(ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { .. })
        )
    {
        // The runtime leaf intentionally uses a compact value debug surface
        // for dynamic characteristics. Render the retained typed payload
        // before any keyword or legacy-display fallback can expose `Count`
        // or `SurfaceHinted` in compiled Oracle text.
        return vec![format!(
            "Static ability {index}: {}",
            describe_static_ability_with_subject(static_ability, subject)
        )];
    }
    if let AbilityKind::Static(static_ability) = &ability.kind
        && let Some(model) = static_ability.compiled_model()
        && let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
            &model.payload
        && grant.filter.source
        && grant.condition == Some(crate::ConditionExpr::YourTurn)
        && matches!(
            &grant.ability.kind,
            ironsmith_core::AbilityKind::Static(granted)
                if granted.id == Some(crate::static_abilities::StaticAbilityId::FirstStrike)
        )
    {
        return vec![format!(
            "Static ability {index}: As long as it's your turn, {subject} has first strike"
        )];
    }
    if let AbilityKind::Static(static_ability) = &ability.kind
        && static_ability.compiled_model().is_some_and(|model| {
            model.label.starts_with(
                ironsmith_core::static_ability_model::AS_LONG_AS_ITS_YOUR_TURN_STATIC_LABEL_PREFIX,
            )
        })
        && let Some(rest) = static_ability.display().strip_prefix("During your turn, ")
    {
        return vec![format!(
            "Static ability {index}: As long as it's your turn, {rest}"
        )];
    }
    // Basic-subtype landwalk is both a typed keyword and commonly carries
    // its standard reminder in Oracle. Its authored-line surface is just the
    // keyword token, so let the typed keyword renderer restore the reminder
    // before the general authored-static fallback returns that bare token.
    if matches!(
        &ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id() == crate::static_abilities::StaticAbilityId::Landwalk
                && matches!(
                    static_ability.landwalk_kind(),
                    Some(crate::static_abilities::LandwalkKind::Subtype { subtype, .. })
                        if subtype.is_basic_land_type()
                )
    ) && let Some(keyword) = describe_keyword_ability(ability)
    {
        return vec![format!("Keyword ability {index}: {keyword}")];
    }
    if let AbilityKind::Static(static_ability) = &ability.kind
        && let Some(surface) = static_ability.authored_line_surface()
    {
        let surface = restore_modeled_value_surface(static_ability, surface);
        return vec![format!("Static ability {index}: {surface}")];
    }
    if let Some(keyword) = describe_keyword_ability(ability) {
        return vec![format!("Keyword ability {index}: {keyword}")];
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            let static_display = static_ability
                .display()
                .trim()
                .trim_end_matches('.')
                .to_ascii_lowercase();
            if static_ability.id() == crate::static_abilities::StaticAbilityId::GrantAbility
                && matches!(
                    static_display.as_str(),
                    "creatures have blocks each combat if able"
                        | "all creatures have blocks each combat if able"
                )
            {
                return vec![format!(
                    "Static ability {index}: All creatures able to block {subject} do so"
                )];
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::GrantAbility
                && let Some(granted) = static_ability.granted_inline_ability()
                && matches!(
                    &granted.kind,
                    AbilityKind::Static(granted_static)
                        if granted_static.id() == crate::static_abilities::StaticAbilityId::MustBlock
                )
            {
                return vec![format!(
                    "Static ability {index}: All creatures able to block {subject} do so"
                )];
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::SoulbondSharedBonus
                && let Some(granted) = static_ability.granted_inline_ability()
            {
                let granted_surface = describe_soulbond_shared_delayed_return(granted)
                    .unwrap_or_else(|| {
                        normalize_granted_triggered_ability_surface(describe_inline_ability(
                            granted,
                        ))
                    });
                return vec![format!(
                    "Static ability {index}: As long as this creature is paired with another creature, each of those creatures has \"{}\"",
                    granted_surface
                )];
            }
            if let Some(levels) = static_ability.level_abilities()
                && !levels.is_empty()
            {
                let mut lines = Vec::new();
                for level in levels {
                    let range = match level.max_level {
                        Some(max) if max == level.min_level => format!("Level {}", level.min_level),
                        Some(max) => format!("Level {}-{}", level.min_level, max),
                        None => format!("Level {}+", level.min_level),
                    };
                    lines.push(format!("Static ability {index}: {range}"));
                    if let Some((power, toughness)) = level.power_toughness {
                        lines.push(format!("Static ability {index}: {power}/{toughness}"));
                    }
                    for granted in &level.abilities {
                        lines.push(format!("Static ability {index}: {}", granted.display()));
                    }
                }
                return lines;
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::CanBeCommander {
                return vec![format!(
                    "Static ability {index}: {subject} can be your commander"
                )];
            }
            let restored_display =
                restore_modeled_value_surface(static_ability, static_ability.display());
            let normalized = normalize_sentence_surface_style(restored_display.trim());
            let lower = normalized.to_ascii_lowercase();
            let prefer_safe_label_text = matches!(
                static_ability.id(),
                crate::static_abilities::StaticAbilityId::KeywordMarker
                    | crate::static_abilities::StaticAbilityId::DraftRuleText
                    | crate::static_abilities::StaticAbilityId::KeywordFallbackText
                    | crate::static_abilities::StaticAbilityId::RuleFallbackText
                    | crate::static_abilities::StaticAbilityId::UnsupportedParserLine
            ) || normalized.contains('"')
                || lower.contains("cycling ");
            if prefer_safe_label_text && !normalized.is_empty() {
                let granted_cycling_surface = if (lower.contains(" has ")
                    || lower.contains(" have "))
                    && lower.contains("cycling ")
                {
                    normalize_granted_cycling_surface_text(&normalized)
                } else {
                    normalized
                };
                return vec![format!("Static ability {index}: {granted_cycling_surface}")];
            }
            let mut rendered = describe_static_ability_with_subject(static_ability, subject);
            if let Some(label) = static_ability.compiled_model().and_then(|model| {
                model.label.strip_prefix(
                    ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX,
                )
            }) && !label.is_empty()
                && !rendered.starts_with(&format!("{label} — "))
            {
                rendered = format!("{label} — {}", capitalize_first(&rendered));
            }
            vec![format!("Static ability {index}: {rendered}")]
        }
        AbilityKind::Triggered(triggered) => {
            if let Some(rendered) =
                describe_spell_cast_copy_while_source_has_quest_counters(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_each_combat_unless_pay_delayed_block_grant(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_source_etb_damage_to_opponents_and_their_creatures(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_attacking_hand_entry_token_copy(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_case_to_solve_triggered_ability(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_recover_keyword(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_kamiz_relational_attacker_sequence(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_backup_keyword(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_champion_keyword(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_annihilator_keyword(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_unique_creature_control_leader_upkeep_control_change(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_targeted_player_or_permanent_counter_unless_life(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_target_opponent_may_copy_triggering_spell(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_oath_of_ghouls_triggered_ability(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_flurry_copy_exile_suspend_triggered_ability(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_convoke_cast_damage_opponents_and_protected_battles(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_optional_copy_plural_keyword_grant(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_tap_lands_sharing_mana_types_with_triggering_land(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_tap_for_mana_additional_mana_trigger(triggered) {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_each_player_first_main_counter_then_scaled_mana(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_each_player_first_main_scaled_artifact_mana(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) =
                describe_active_player_postcombat_opponents_lost_life_mana_trigger(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            if let Some(rendered) = describe_zone_change_triggering_card_to_your_library(triggered)
            {
                return vec![format!("Triggered ability {index}: {rendered}")];
            }
            let (intervening_condition, trigger_frequency) = triggered
                .intervening_if
                .as_ref()
                .map(split_trigger_intervening_if)
                .unwrap_or((None, None));
            let mut intervening_condition =
                retain_state_trigger_residual_condition(&triggered.trigger, intervening_condition);
            intervening_condition = intervening_condition.and_then(|condition| {
                remove_presentation_label_chosen_option(&condition, triggered)
            });
            let mut trigger_surface = apply_triggered_presentation_label(
                triggered,
                describe_trigger_surface_with_frequency(triggered, trigger_frequency, subject),
            );
            if triggered_deals_same_damage_to_each_other_opponent(triggered) {
                trigger_surface = trigger_surface
                    .replace("combat damage to a player", "combat damage to an opponent");
            }
            if triggered.presentation_label.is_none()
                && trigger_surface
                    .starts_with("Whenever you cast a spell that targets this creature")
            {
                trigger_surface = format!("Heroic — {trigger_surface}");
            }
            apply_attacks_while_most_life_surface(
                triggered,
                &mut trigger_surface,
                &mut intervening_condition,
            );
            if trigger_surface
                .to_ascii_lowercase()
                .ends_with("becomes tapped")
            {
                let mut conjuncts = Vec::new();
                if let Some(condition) = intervening_condition.take() {
                    flatten_condition_and_expr(&condition, &mut conjuncts);
                }
                let had_your_turn = conjuncts
                    .iter()
                    .any(|condition| matches!(condition, Condition::YourTurn));
                conjuncts.retain(|condition| !matches!(condition, Condition::YourTurn));
                intervening_condition = fold_condition_exprs(conjuncts);
                if had_your_turn {
                    trigger_surface.push_str(" during your turn");
                }
            }
            let saga_intervening_condition = if triggered.trigger.saga_chapters().is_some() {
                intervening_condition.take()
            } else {
                None
            };
            let mut line = format!("Triggered ability {index}: {trigger_surface}");
            if let Some(condition) = intervening_condition {
                line.push_str(", if ");
                line.push_str(&describe_trigger_intervening_condition(
                    &condition, triggered, None,
                ));
            }
            // A parameterized keyword label ("Firebending 2") IS the whole
            // printed line; its resolution effects are the keyword's rules
            // meaning and never appear alongside it in oracle text.
            let keyword_label_replaces_resolution = matches!(
                triggered.presentation_label,
                Some(PresentationLabel::Keyword(
                    PresentationKeyword::Firebending(_)
                ))
            );
            let mut clauses = Vec::new();
            if !triggered.choices.is_empty()
                && !(!triggered.effects.is_empty()
                    && choices_are_simple_targets(&triggered.choices))
            {
                let choices = triggered
                    .choices
                    .iter()
                    .map(describe_choose_spec)
                    .collect::<Vec<_>>()
                    .join(", ");
                clauses.push(format!("choose {choices}"));
            }
            if !keyword_label_replaces_resolution
                && let Some(effects) =
                    describe_triggered_resolution_text(triggered, subject, rewrite_it_deals)
            {
                clauses.push(effects);
            }
            if !clauses.is_empty() {
                // Oracle-style: "Whenever ..., if ..., ..." rather than "Whenever ...: If ..."
                if let Some(condition) = saga_intervening_condition.as_ref() {
                    line.push_str(" — If ");
                    line.push_str(&describe_trigger_intervening_condition(
                        condition, triggered, None,
                    ));
                    line.push_str(", ");
                    line.push_str(&lowercase_first(&clauses.join(": ")));
                } else if clauses.len() == 1 {
                    let only = clauses[0].trim_start();
                    if let Some(rest) = only.strip_prefix("If ") {
                        line.push_str(", if ");
                        line.push_str(rest.trim_start());
                    } else if let Some(rest) = only.strip_prefix("if ") {
                        line.push_str(", if ");
                        line.push_str(rest.trim_start());
                    } else if triggered.trigger.saga_chapters().is_some() {
                        line.push_str(" — ");
                        line.push_str(&capitalize_first(only));
                    } else if triggered.presentation_label.is_some() {
                        line.push_str(", ");
                        line.push_str(&triggered_resolution_clause_case(triggered, only));
                    } else {
                        line.push_str(", ");
                        line.push_str(&triggered_resolution_clause_case(triggered, only));
                    }
                } else {
                    line.push_str(": ");
                    line.push_str(&clauses.join(": "));
                }
            }
            if triggered_has_you_difference_draw(triggered) {
                line = line.replace(
                    "you draw cards equal to the difference",
                    "draw cards equal to the difference",
                );
            }
            let line_lower = line.to_ascii_lowercase();
            if line_lower.contains("whenever an opponent loses life")
                && line_lower
                    .contains("you may cast target instant or sorcery card from your graveyard")
            {
                line = line.replacen(
                    "Whenever an opponent loses life",
                    "Whenever one or more opponents lose life",
                    1,
                );
            }
            match trigger_frequency {
                Some(TriggerFrequencySurface::AbilityMaxTimesEachTurn(max)) => {
                    if max == 1
                        && line.to_ascii_lowercase().contains(
                            "you may cast target instant or sorcery card from your graveyard",
                        )
                    {
                        line.push_str(". Do this only once each turn");
                    } else if max == 1 {
                        line.push_str(". This ability triggers only once each turn");
                    } else if max == 2 {
                        line.push_str(". This ability triggers only twice each turn");
                    } else {
                        line.push_str(". This ability triggers only ");
                        line.push_str(&max.to_string());
                        line.push_str(" times");
                        line.push_str(" each turn");
                    }
                }
                Some(TriggerFrequencySurface::DoThisMaxTimesEachTurn(max)) => {
                    if max == 1 {
                        line.push_str(". Do this only once each turn");
                    } else if max == 2 {
                        line.push_str(". Do this only twice each turn");
                    } else {
                        line.push_str(". Do this only ");
                        line.push_str(&max.to_string());
                        line.push_str(" times");
                        line.push_str(" each turn");
                    }
                }
                _ => {}
            }
            line = normalize_spellcast_trigger_mana_value_surface(triggered, line);
            line = normalize_redundant_short_name_etb_surface(line, triggered, subject);
            line = normalize_modal_named_source_etb_surface(line, triggered, subject);
            line = normalize_ability_self_reference_surface(&line, subject);
            line = normalize_graveyard_source_return_surface(&line, ability);
            line = normalize_ability_self_reference_surface(&line, subject);
            line = deduplicate_triggered_presentation_label(triggered, line);
            vec![line]
        }
        AbilityKind::Activated(activated) if activated.is_mana_ability() => {
            let mut line = if activated_presentation_label(activated).is_some() {
                String::new()
            } else {
                format!("Mana ability {index}")
            };
            let rendered_cost = describe_total_cost(&activated.mana_cost);
            let mut cost_text = if !rendered_cost.is_empty() {
                Some(rendered_cost)
            } else {
                None
            };
            cost_text = normalize_zone_bound_self_exile_cost(cost_text, ability);
            if ability.functional_zones.contains(&Zone::Hand)
                && cost_text.as_deref().is_some_and(|cost| {
                    matches!(
                        cost,
                        "Exile this creature"
                            | "Exile this permanent"
                            | "Exile this source"
                            | "Exile this card"
                    )
                })
            {
                cost_text = Some("Exile this card from your hand".to_string());
            }
            let loyalty_prefix = describe_loyalty_activation_prefix_for_activated(activated);
            let mana_symbols = activated.mana_symbols();
            let add_text = if !mana_symbols.is_empty() {
                Some(format!(
                    "Add {}",
                    mana_symbols
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join("")
                ))
            } else {
                None
            };
            let is_loyalty_ability = loyalty_prefix.is_some();
            if let Some(prefix) = loyalty_prefix {
                line = format!("{prefix}:");
                if let Some(add) = &add_text {
                    line.push(' ');
                    line.push_str(add);
                }
            } else if let (Some(cost), Some(add)) = (&cost_text, &add_text) {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(cost);
                line.push_str(": ");
                line.push_str(add);
            } else if let Some(cost) = &cost_text {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(cost);
            } else if let Some(add) = &add_text {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(add);
            }
            if !activated.effects.is_empty() {
                if is_loyalty_ability && add_text.is_none() {
                    line.push(' ');
                } else {
                    line.push_str(": ");
                }
                let effects =
                    super::ast_render::describe_mana_ability_resolution_program(&activated.effects)
                        .unwrap_or_else(|| {
                            super::ast_render::describe_resolution_program(&activated.effects)
                        });
                let mut effects = rewrite_damage_phrases_for_permanent_abilities(
                    &effects,
                    subject,
                    rewrite_it_deals,
                );
                let flat_costs = activated.mana_cost.as_all().unwrap_or(&[]);
                effects = rewrite_cost_bound_x_phrases(effects, flat_costs);
                effects = rewrite_removed_counter_type_surface(effects, flat_costs);
                effects = rewrite_sacrifice_cost_token_copy_reference(effects, activated);
                if subject != "This spell" {
                    effects = replace_this_spell_self_reference(effects, subject);
                }
                line.push_str(&effects);
            }
            if let Some(prefix) = station_threshold_prefix(activated) {
                line = prefix_rendered_ability_body(line, &format!("{prefix} | "));
            } else if let Some(condition) =
                activation_condition_without_presentation_label(activated)
            {
                let clause = describe_mana_activation_condition(&condition);
                if !clause.is_empty() {
                    line.push_str(". ");
                    line.push_str(&clause);
                }
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
                &activated.activation_restrictions,
            );
            if !restriction_clauses.is_empty() {
                append_activation_clause(
                    &mut line,
                    &join_activation_restriction_clauses(&restriction_clauses),
                );
            }
            for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                line.push_str(". ");
                line.push_str(&clause);
            }
            if let Some(label) = activated_presentation_label(activated)
                && !line.starts_with(label)
            {
                line = format!("{label} — {line}");
            }
            line = normalize_ability_self_reference_surface(&line, subject);
            line = normalize_graveyard_source_return_surface(&line, ability);
            vec![line]
        }
        AbilityKind::Activated(activated) => {
            if let Some(ninjutsu) = describe_ninjutsu_activation(ability, activated) {
                return vec![ninjutsu];
            }
            if let Some(level_up) = describe_level_up_activation(activated) {
                return vec![level_up];
            }
            if let Some(level_text) = describe_class_level_activation(activated) {
                return vec![level_text];
            }
            if activated.choices.is_empty()
                && matches!(activated.timing, ActivationTiming::SorcerySpeed)
                && activated.effects.segments.len() == 1
                && activated.effects.segments[0].self_replacements.is_empty()
                && activated.effects.segments[0].default_effects.len() == 1
                && activated.effects.segments[0].default_effects[0]
                    .downcast_ref::<crate::effects::UnearthEffect>()
                    .is_some()
            {
                return vec![format!(
                    "Unearth {}",
                    describe_total_cost(&activated.mana_cost)
                )];
            }
            if let Some(loyalty_prefix) =
                describe_loyalty_activation_prefix_for_activated(activated)
            {
                let mut line = format!("{loyalty_prefix}:");
                let mana_symbols = activated.mana_symbols();
                if !activated.choices.is_empty()
                    && !(!activated.effects.is_empty()
                        && choices_are_simple_targets(&activated.choices))
                {
                    line.push_str(" choose ");
                    line.push_str(
                        &activated
                            .choices
                            .iter()
                            .map(describe_choose_spec)
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    line.push(':');
                }
                if !mana_symbols.is_empty() {
                    line.push(' ');
                    line.push_str("Add ");
                    line.push_str(
                        &mana_symbols
                            .iter()
                            .copied()
                            .map(describe_mana_symbol)
                            .collect::<Vec<_>>()
                            .join(""),
                    );
                } else if !activated.effects.is_empty() {
                    line.push(' ');
                    let effects =
                        super::ast_render::describe_resolution_program(&activated.effects);
                    let mut effects = rewrite_damage_phrases_for_permanent_abilities(
                        &effects,
                        subject,
                        rewrite_it_deals
                            && !activated_resolution_has_explicit_it_damage_source(activated),
                    );
                    let flat_costs = activated.mana_cost.as_all().unwrap_or(&[]);
                    effects = rewrite_cost_bound_x_phrases(effects, flat_costs);
                    effects = rewrite_removed_counter_type_surface(effects, flat_costs);
                    line.push_str(&effects);
                }
                if let Some(condition) = activation_condition_without_presentation_label(activated)
                {
                    let clause = describe_mana_activation_condition(&condition);
                    if !clause.is_empty() {
                        line.push_str(". ");
                        line.push_str(&clause);
                    }
                }
                for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                    line.push_str(". ");
                    line.push_str(&clause);
                }
                return vec![line];
            }
            let is_grandeur = is_grandeur_activation_cost(activated);
            let is_exhaust = activated.is_exhaust_ability();
            let has_presentation_label = activated_presentation_label(activated).is_some();
            let has_level_range = level_range_activation_prefix(activated).is_some();
            let omit_debug_prefix =
                is_grandeur || is_exhaust || has_presentation_label || has_level_range;
            let mut line = if omit_debug_prefix {
                String::new()
            } else {
                format!("Activated ability {index}")
            };
            let mut pre = Vec::new();
            let mut trailing_x_definition = None;
            let waterbend_label = activated_presentation_label(activated)
                .filter(|label| label.starts_with("Waterbend {") && label.ends_with('}'));
            if let Some(label) = waterbend_label {
                // Waterbend's expanded `OneOf` cost is the executable payment
                // model. Its authored keyword and mana value are the complete
                // public cost surface, so do not print every equivalent tap
                // branch after the presentation label.
                pre.push(label.to_string());
            } else {
                let rendered_cost = describe_total_cost(&activated.mana_cost);
                if !rendered_cost.is_empty() {
                    let (cost_text, x_definition) =
                        describe_total_cost_with_trailing_x_definition(&activated.mana_cost);
                    trailing_x_definition = x_definition;
                    if let Some(cost_text) =
                        normalize_zone_bound_self_exile_cost(Some(cost_text), ability)
                    {
                        pre.push(cost_text);
                    }
                }
            }
            if !activated.choices.is_empty()
                && !(!activated.effects.is_empty()
                    && choices_are_simple_targets(&activated.choices))
            {
                pre.push(format!(
                    "choose {}",
                    activated
                        .choices
                        .iter()
                        .map(describe_choose_spec)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !pre.is_empty() {
                if line.is_empty() {
                    line.push_str(&pre.join(", "));
                } else {
                    line.push_str(": ");
                    line.push_str(&pre.join(", "));
                }
            }
            if !activated.effects.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                let effects = super::ast_render::describe_resolution_program(&activated.effects);
                let mut effects = if graveyard_self_exile_damage_uses_it_subject(ability, activated)
                    && let Some(rest) = effects.strip_prefix("Deal ")
                {
                    format!("It deals {rest}")
                } else {
                    rewrite_damage_phrases_for_permanent_abilities(
                        &effects,
                        subject,
                        rewrite_it_deals
                            && !activated_resolution_has_explicit_it_damage_source(activated),
                    )
                };
                let flat_costs = activated.mana_cost.as_all().unwrap_or(&[]);
                effects = rewrite_cost_bound_x_phrases(effects, flat_costs);
                effects = rewrite_removed_counter_type_surface(effects, flat_costs);
                effects = rewrite_sacrifice_cost_token_copy_reference(effects, activated);
                if subject != "This spell" {
                    effects = replace_this_spell_self_reference(effects, subject);
                }
                line.push_str(&effects);
            }
            if let Some(x_definition) = trailing_x_definition {
                append_sentence_clause(&mut line, &x_definition);
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
                &activated.activation_restrictions,
            );
            if !restriction_clauses.is_empty() {
                append_activation_clause(
                    &mut line,
                    &join_activation_restriction_clauses(&restriction_clauses),
                );
            }
            for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                append_activation_clause(&mut line, &clause);
            }
            if is_grandeur_activation_cost(activated) {
                line = format!("Grandeur — {line}");
            }
            if let Some(prefix) = level_range_activation_prefix(activated) {
                line = format!("{prefix}. {line}");
            }
            if is_exhaust {
                line = format!("Exhaust — {line}");
            } else if let Some(label) = activated_presentation_label(activated)
                && !line.starts_with(label)
            {
                line = format!("{label} — {line}");
            }
            line = normalize_ability_self_reference_surface(&line, subject);
            line = normalize_graveyard_source_return_surface(&line, ability);
            vec![line]
        }
    }
}

pub(in crate::compiled_text) fn describe_conditional_spell_uncounterability(
    ability: &Ability,
) -> Option<String> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::Conditional {
        ability: inner,
        condition,
    } = &model.payload
    else {
        return None;
    };
    if model.id != Some(crate::static_abilities::StaticAbilityId::CantBeCountered)
        || inner.id != Some(crate::static_abilities::StaticAbilityId::CantBeCountered)
        || !matches!(inner.payload, ironsmith_core::StaticAbilityPayload::None)
    {
        return None;
    }

    Some(format!(
        "If {}, this spell can't be countered",
        describe_condition(condition)
    ))
}

/// Preserve the verb-first surface of the exact soulbond-shared delayed
/// return program. The triggering-object tag is the executable antecedent;
/// the one-shot next-upkeep schedule owns the optional return. Generic delayed
/// rendering is timing-first, but the authored granted ability reads
/// "you may return it ... at the beginning of your next upkeep."
fn describe_soulbond_shared_delayed_return(granted: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &granted.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }

    let dies = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !dies.this_object
        || dies.from != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || dies.to != crate::triggers::ZonePattern::Specific(Zone::Graveyard)
        || dies.player != crate::triggers::PlayerRelation::Any
        || dies.cause_filter.is_some()
        || dies.count_mode != crate::triggers::CountMode::Each
    {
        return None;
    }
    if dies.object_filter != ObjectFilter::creature() {
        return None;
    }

    let [tag_effect, schedule_effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tag = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if !schedule.one_shot
        || !schedule.start_next_turn
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::Forever
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
        || schedule.leading_duration_surface
        || schedule.watch_ability_source
        || schedule.watch_all_object_targets
        || schedule.either_of_watched_objects
        || schedule.while_any_tagged_object_in_zone.is_some()
        || !schedule.target_objects.is_empty()
        || schedule.target_tag.is_some()
        || schedule.target_filter.is_some()
        || schedule.controller != PlayerFilter::You
        || schedule.prepayment.is_some()
        || schedule.event_value_from_prior_prevention
        || !schedule
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_some_and(|upkeep| upkeep.player == PlayerFilter::You)
    {
        return None;
    }

    let [may_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|player| *player != PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let expected = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(tag.tag.clone()),
        Zone::Battlefield,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
    .under_owner_control();
    if move_to_zone != &expected {
        return None;
    }

    Some(
        "When this creature dies, you may return it to the battlefield under its owner's control at the beginning of your next upkeep."
            .to_string(),
    )
}

#[cfg(test)]
mod soulbond_shared_delayed_return_tests {
    use super::*;

    fn fixture(start_next_turn: bool) -> Ability {
        let tag = crate::tag::TagKey::from("triggering");
        let return_effect = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(tag.clone()),
                Zone::Battlefield,
                false,
            )
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
            .under_owner_control(),
        );
        let may = Effect::new(crate::effects::MayEffect::new_for_player(
            vec![return_effect],
            PlayerFilter::You,
        ));
        let mut schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
            crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),
            vec![may],
            true,
            Vec::new(),
            PlayerFilter::You,
        );
        schedule.start_next_turn = start_next_turn;

        Ability::triggered(
            crate::triggers::Trigger::this_dies(),
            vec![
                Effect::new(crate::effects::TagTriggeringObjectEffect::new(tag)),
                Effect::new(schedule),
            ],
        )
    }

    #[test]
    fn exact_tagged_next_upkeep_return_uses_verb_first_surface() {
        let fixture = fixture(true);
        assert_eq!(
            describe_soulbond_shared_delayed_return(&fixture).as_deref(),
            Some(
                "When this creature dies, you may return it to the battlefield under its owner's control at the beginning of your next upkeep."
            )
        );
    }

    #[test]
    fn non_next_turn_schedule_is_not_rewritten() {
        assert!(describe_soulbond_shared_delayed_return(&fixture(false)).is_none());
    }
}

pub(super) fn normalize_granted_triggered_ability_surface(surface: String) -> String {
    let Some((head, tail)) = surface
        .split_once(": ")
        .or_else(|| surface.split_once(", "))
    else {
        return surface;
    };
    let lower_head = head.to_ascii_lowercase();
    if !(lower_head.starts_with("when ")
        || lower_head.starts_with("whenever ")
        || lower_head.starts_with("at the beginning "))
    {
        return surface;
    }

    // Oracle keeps the explicit subject in optional instructions ("When this
    // creature dies, you may return it ..."); only a mandatory "You <verb>"
    // drops to the bare imperative.
    let keeps_you_subject = tail.to_ascii_lowercase().starts_with("you may ");
    let tail = if keeps_you_subject {
        tail
    } else {
        tail.strip_prefix("You ")
            .or_else(|| tail.strip_prefix("you "))
            .unwrap_or(tail)
            .trim_start()
    };
    if tail.is_empty() {
        return surface;
    }

    let mut normalized_tail = lowercase_first(tail);
    if !normalized_tail.ends_with('.')
        && !normalized_tail.ends_with('!')
        && !normalized_tail.ends_with('?')
    {
        normalized_tail.push('.');
    }

    format!("{head}, {normalized_tail}")
}

pub(super) fn normalize_zone_bound_self_exile_cost(
    cost_text: Option<String>,
    ability: &Ability,
) -> Option<String> {
    let cost = cost_text?;
    if ability.functional_zones.contains(&Zone::Graveyard) {
        let mut rewritten = cost;
        if rewritten.contains("Exile this card from your graveyard") {
            return Some(rewritten.replace(
                "from your graveyard from your graveyard",
                "from your graveyard",
            ));
        }
        for subject in ["creature", "permanent", "source", "spell", "card"] {
            rewritten = rewritten.replace(
                &format!("Exile this {subject}"),
                "Exile this card from your graveyard",
            );
        }
        return Some(rewritten.replace(
            "from your graveyard from your graveyard",
            "from your graveyard",
        ));
    }
    Some(cost)
}

pub(crate) fn normalize_ability_self_reference_surface(line: &str, subject: &str) -> String {
    if subject.eq_ignore_ascii_case("this source") {
        return line.to_string();
    }
    let capitalized = capitalize_first(subject);
    let normalized = replace_ability_source_reference_outside_quotes(line, "this source", subject);
    let normalized =
        replace_ability_source_reference_outside_quotes(&normalized, "This source", &capitalized);
    let normalized =
        replace_ability_source_reference_outside_quotes(&normalized, "this permanent", subject);
    let normalized = replace_ability_source_reference_outside_quotes(
        &normalized,
        "This permanent",
        &capitalized,
    );
    let normalized = if let Some(rest) = normalized.strip_prefix("Exile this:") {
        format!("Exile {subject}:{rest}")
    } else {
        normalized
    };
    let normalized = normalize_source_owned_quoted_ability_references(&normalized, subject);
    let normalized = if let Some(source_type) = subject.strip_prefix("this ") {
        let typed_object = with_indefinite_article(source_type);
        normalized
            .replace(
                &format!("if {subject} is {typed_object}"),
                &format!("if this permanent is {typed_object}"),
            )
            .replace(
                &format!("If {subject} is {typed_object}"),
                &format!("If this permanent is {typed_object}"),
            )
    } else {
        normalized
    };
    let normalized = rewrite_source_no_counter_resolution_surface(normalized, subject);
    let named_self_exile =
        format!("Exile {subject} and target creature without flying that's attacking you");
    let normalized = normalized.replace(
        &named_self_exile,
        "Exile this creature and target creature without flying that's attacking you",
    );

    // A proven inline no-regeneration rider inside the failed-payment branch
    // is authored as one coordinated instruction. The executable
    // DestroyNoRegeneration effect deliberately renders the reusable default
    // as two sentences, so rejoin only this exact conditional surface.
    if normalized.contains("If you don't, destroy ")
        && normalized.ends_with(". It can't be regenerated")
    {
        return normalized.replacen(
            ". It can't be regenerated",
            " and it can't be regenerated",
            1,
        );
    }

    if line.contains("Sacrifice this permanent when that token leaves the battlefield")
        && normalized.contains("Sacrifice it when that token leaves the battlefield")
    {
        normalized.replace(
            "Sacrifice it when that token leaves the battlefield",
            "Sacrifice this permanent when that token leaves the battlefield",
        )
    } else {
        normalized
    }
}

/// Source-reference normalization belongs to the ability that owns `line`.
/// Do not blindly rewrite inside quotes: a later pass retypes a quoted
/// ability only when its receiving object is proven to be this source.
fn replace_ability_source_reference_outside_quotes(input: &str, from: &str, to: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_quote = false;
    let mut index = 0;
    while index < input.len() {
        let ch = input[index..]
            .chars()
            .next()
            .expect("index should be on a char boundary");
        if ch == '"' {
            in_quote = !in_quote;
            output.push(ch);
            index += ch.len_utf8();
        } else if !in_quote && input[index..].starts_with(from) {
            output.push_str(to);
            index += from.len();
        } else {
            output.push(ch);
            index += ch.len_utf8();
        }
    }
    output
}

fn quote_is_granted_to_subject(prefix: &str, subject: &str) -> bool {
    let prefix = prefix.to_ascii_lowercase();
    let subject = subject.to_ascii_lowercase();
    [" gains ", " gain ", " has ", " have "]
        .iter()
        .filter_map(|verb| prefix.rfind(verb))
        .max()
        .is_some_and(|index| prefix[..index].trim_end().ends_with(&subject))
}

fn normalize_source_owned_quoted_ability_references(input: &str, subject: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut remainder = input;
    let capitalized = capitalize_first(subject);

    while let Some(open) = remainder.find('"') {
        let before_quote = &remainder[..open];
        output.push_str(before_quote);
        let quote_is_source_owned = quote_is_granted_to_subject(&output, subject);
        output.push('"');
        let after_open = &remainder[open + 1..];
        let Some(close) = after_open.find('"') else {
            output.push_str(after_open);
            return output;
        };
        let quoted = &after_open[..close];
        if quote_is_source_owned {
            output.push_str(
                &quoted
                    .replace("this source", subject)
                    .replace("This source", &capitalized)
                    .replace("this permanent", subject)
                    .replace("This permanent", &capitalized),
            );
        } else {
            output.push_str(quoted);
        }
        output.push('"');
        remainder = &after_open[close + 1..];
    }
    output.push_str(remainder);
    output
}

#[cfg(test)]
#[test]
fn outer_source_normalization_preserves_nested_granted_ability_identity() {
    assert_eq!(
        normalize_ability_self_reference_surface(
            "This permanent copies a spell and gains \"At the beginning of the end step, sacrifice this permanent.\"",
            "this creature",
        ),
        "This creature copies a spell and gains \"At the beginning of the end step, sacrifice this permanent.\"",
    );
}

#[cfg(test)]
#[test]
fn source_owned_granted_trigger_uses_the_vehicle_subject() {
    assert_eq!(
        normalize_ability_self_reference_surface(
            "Whenever this permanent becomes crewed for the first time each turn, until end of turn, this permanent gains \"Whenever this permanent deals combat damage to a player, draw two cards.\"",
            "this Vehicle",
        ),
        "Whenever this Vehicle becomes crewed for the first time each turn, until end of turn, this Vehicle gains \"Whenever this Vehicle deals combat damage to a player, draw two cards.\"",
    );
}

pub(super) fn normalize_graveyard_source_return_surface(line: &str, ability: &Ability) -> String {
    if !ability.functional_zones.contains(&Zone::Graveyard) {
        return line.to_string();
    }
    let normalize_battlefield_self_return = !line.contains(". Put ");
    let mut normalized = line.to_string();
    for subject in ["Aura", "card", "creature", "spell", "permanent", "source"] {
        normalized = normalized.replace(
            &format!("Return this {subject} from a graveyard to its owner's hand"),
            "Return this card from your graveyard to your hand",
        );
        normalized = normalized.replace(
            &format!("return this {subject} from a graveyard to its owner's hand"),
            "return this card from your graveyard to your hand",
        );
        if normalize_battlefield_self_return {
            normalized = normalized.replace(
                &format!("Return this {subject} from graveyard to the battlefield"),
                "Return this card from your graveyard to the battlefield",
            );
            normalized = normalized.replace(
                &format!("return this {subject} from graveyard to the battlefield"),
                "return this card from your graveyard to the battlefield",
            );
        }
    }
    let sole_creature_gate = matches!(
        &ability.kind,
        AbilityKind::Triggered(triggered)
            if matches!(
                triggered.intervening_if.as_ref(),
                Some(Condition::ValueComparison {
                    left,
                    operator: crate::effect::ValueComparisonOperator::Equal,
                    right,
                }) if matches!(left.unhinted(), Value::Count(filter)
                    if filter.zone == Some(Zone::Graveyard)
                        && filter.owner == Some(PlayerFilter::You)
                        && filter.card_types.as_slice() == [CardType::Creature])
                    && matches!(right.unhinted(), Value::Fixed(1))
            )
    );
    if sole_creature_gate {
        normalized = normalized.replace(
            "return this card from your graveyard to the battlefield",
            "return this card to the battlefield",
        );
    }
    normalized
}

pub(crate) fn rewrite_damage_phrases_for_permanent_abilities(
    effect_text: &str,
    subject: &str,
    rewrite_it_deals: bool,
) -> String {
    let capitalized_subject = capitalize_first(subject);
    let mut out = if let Some(rest) = effect_text.strip_prefix("Deal ") {
        format!("{subject} deals {rest}")
    } else if let Some(rest) = effect_text.strip_prefix("deal ") {
        format!("{subject} deals {rest}")
    } else if rewrite_it_deals {
        if let Some(rest) = effect_text.strip_prefix("It deals ") {
            format!("{subject} deals {rest}")
        } else if let Some(rest) = effect_text.strip_prefix("it deals ") {
            format!("{subject} deals {rest}")
        } else {
            effect_text.to_string()
        }
    } else {
        effect_text.to_string()
    };
    const CONDITIONAL_DEAL_MARKER: &str = "__ironsmith_keep_conditional_deal__";
    // Spell-resolution text sometimes intentionally keeps an imperative
    // negative branch. Permanent abilities, however, need their typed source
    // restored before the later pass contracts this phrase to `Otherwise`.
    if !subject.to_ascii_lowercase().starts_with("this ") {
        out = out
            .replace(
                "If that doesn't happen, Deal ",
                &format!("If that doesn't happen, {CONDITIONAL_DEAL_MARKER}"),
            )
            .replace(
                "If that doesn't happen, deal ",
                &format!("If that doesn't happen, {CONDITIONAL_DEAL_MARKER}"),
            );
    }
    out = out
        .replace(
            ". Otherwise, Deal ",
            &format!(". Otherwise, {capitalized_subject} deals "),
        )
        .replace(
            ". Otherwise, deal ",
            &format!(". Otherwise, {subject} deals "),
        )
        .replace(
            " Otherwise, Deal ",
            &format!(" Otherwise, {capitalized_subject} deals "),
        )
        .replace(
            " Otherwise, deal ",
            &format!(" Otherwise, {subject} deals "),
        )
        .replace(
            " otherwise, Deal ",
            &format!(" otherwise, {subject} deals "),
        )
        .replace(
            " otherwise, deal ",
            &format!(" otherwise, {subject} deals "),
        )
        .replace(". Deal ", &format!(". {capitalized_subject} deals "))
        .replace(". deal ", &format!(". {subject} deals "))
        .replace("| Deal ", &format!("| {capitalized_subject} deals "))
        .replace("| deal ", &format!("| {subject} deals "))
        .replace(", then Deal ", &format!(", then {subject} deals "))
        .replace(", then deal ", &format!(", then {subject} deals "))
        .replace(", Deal ", &format!(", {subject} deals "))
        .replace(", deal ", &format!(", {subject} deals "))
        .replace(" and Deal ", &format!(" and {subject} deals "))
        .replace(" and deal ", &format!(" and {subject} deals "));
    if rewrite_it_deals {
        out = out
            .replace(". It deals ", &format!(". {capitalized_subject} deals "))
            .replace(". it deals ", &format!(". {subject} deals "));
    }

    // Common oracle phrasing: "you may have this creature deal ..."
    out = out.replace("You may Deal ", &format!("You may have {subject} deal "));
    out = out.replace("you may Deal ", &format!("you may have {subject} deal "));
    out = out.replace("You may deal ", &format!("You may have {subject} deal "));
    out = out.replace("you may deal ", &format!("you may have {subject} deal "));
    out = out.replace(" has this source deal ", &format!(" has {subject} deal "));
    out = out.replace(CONDITIONAL_DEAL_MARKER, "deal ");
    out
}

pub(super) fn describe_damage_amount_with_revealed_count_where_x(
    value: &Value,
) -> Option<(String, String)> {
    if let Value::SurfaceHinted { value, .. } = value {
        return describe_damage_amount_with_revealed_count_where_x(value);
    }
    if matches!(value, Value::Count(_)) {
        let count_text = describe_value(value);
        if count_text.ends_with("revealed this way") {
            return Some(("X".to_string(), count_text));
        }
    }
    if let Value::Add(left, right) = value {
        let (count_value, offset) = match (left.as_ref(), right.as_ref()) {
            (Value::Count(_), Value::Fixed(offset)) => (left.as_ref(), *offset),
            (Value::Fixed(offset), Value::Count(_)) => (right.as_ref(), *offset),
            _ => return None,
        };
        let count_text = describe_value(count_value);
        if count_text.ends_with("revealed this way") {
            let amount_text = if offset == 0 {
                "X".to_string()
            } else if offset > 0 {
                format!("X plus {offset}")
            } else {
                format!("X minus {}", offset.abs())
            };
            return Some((amount_text, count_text));
        }
    }
    None
}

pub(crate) fn card_self_reference_phrase_for_card(card: &crate::card::Card) -> &'static str {
    if card.is_instant() || card.is_sorcery() {
        return "this spell";
    }
    if card.subtypes.contains(&Subtype::Aura) {
        return "this Aura";
    }
    if card.subtypes.contains(&Subtype::Equipment) {
        return "this Equipment";
    }
    if card.subtypes.contains(&Subtype::Fortification) {
        return "this Fortification";
    }
    if card.subtypes.contains(&Subtype::Class) {
        return "this Class";
    }
    if card.subtypes.contains(&Subtype::Saga) {
        return "this Saga";
    }
    if card.subtypes.contains(&Subtype::Siege) {
        return "this Siege";
    }
    if card.subtypes.contains(&Subtype::Vehicle) {
        return "this Vehicle";
    }
    if card.subtypes.contains(&Subtype::Spacecraft) {
        return "this Spacecraft";
    }

    let card_types = &card.card_types;
    if card_types.contains(&CardType::Creature) {
        "this creature"
    } else if card_types.contains(&CardType::Enchantment) {
        "this enchantment"
    } else if card_types.contains(&CardType::Battle) {
        "this battle"
    } else if card_types.contains(&CardType::Land) {
        "this land"
    } else if card_types.contains(&CardType::Artifact) {
        "this artifact"
    } else if card_types.contains(&CardType::Planeswalker) {
        "this planeswalker"
    } else {
        "this permanent"
    }
}

pub(crate) fn subject_for_card(card: &crate::card::Card) -> &'static str {
    card_self_reference_phrase_for_card(card)
}

pub(crate) fn describe_mana_activation_condition(condition: &crate::ConditionExpr) -> String {
    fn source_entered_this_turn_subject(condition: &crate::ConditionExpr) -> Option<String> {
        let crate::ConditionExpr::ObjectEnteredBattlefieldThisTurn(filter) = condition else {
            return None;
        };
        filter.source.then(|| filter.description())
    }

    fn activation_condition_body(condition: &crate::ConditionExpr) -> String {
        let described = describe_mana_activation_condition(condition);
        described
            .strip_prefix("Activate only if ")
            .or_else(|| described.strip_prefix("Activate only "))
            .unwrap_or(&described)
            .to_string()
    }

    fn flatten(condition: &crate::ConditionExpr, out: &mut Vec<crate::ConditionExpr>) {
        match condition {
            crate::ConditionExpr::And(left, right) => {
                flatten(left, out);
                flatten(right, out);
            }
            _ => out.push(condition.clone()),
        }
    }

    match condition {
        crate::ConditionExpr::And(_, _) => {
            let mut conditions = Vec::new();
            flatten(condition, &mut conditions);
            let clauses = conditions
                .iter()
                .map(describe_mana_activation_condition)
                .collect::<Vec<_>>();
            match clauses.len() {
                0 => String::new(),
                1 => clauses[0].clone(),
                _ => {
                    let mut iter = clauses.into_iter();
                    let first = iter.next().unwrap_or_default();
                    let mut line = first;
                    for clause in iter {
                        if let Some(rest) = clause.strip_prefix("Activate only ") {
                            line.push_str(" and only ");
                            line.push_str(rest);
                        } else {
                            line.push_str(" and ");
                            line.push_str(&clause);
                        }
                    }
                    line
                }
            }
        }
        crate::ConditionExpr::Or(left, right)
            if source_entered_this_turn_subject(left).is_some()
                || source_entered_this_turn_subject(right).is_some() =>
        {
            format!(
                "Activate only if {} or if {}",
                activation_condition_body(left),
                activation_condition_body(right)
            )
        }
        crate::ConditionExpr::ObjectEnteredBattlefieldThisTurn(filter) if filter.source => {
            format!(
                "Activate only if {} entered this turn",
                filter.description()
            )
        }
        crate::ConditionExpr::YouControl(filter) => {
            let described =
                with_indefinite_article(strip_indefinite_article(&filter.description()));
            format!("Activate only if you control {described}")
        }
        crate::ConditionExpr::PlayerHasAtLeast {
            player,
            filter,
            count: 1,
        } if filter.zone == Some(Zone::Battlefield)
            && (filter.controller.is_none()
                || filter
                    .controller
                    .as_ref()
                    .is_some_and(|controller| controller == player))
            && filter.card_types.is_empty()
            && filter.subtypes.len() == 1
            && filter.excluded_card_types.is_empty()
            && filter.excluded_subtypes.is_empty() =>
        {
            let subject = describe_player_filter(player);
            let mut described_filter = filter.clone();
            described_filter.controller = None;
            let described =
                with_indefinite_article(strip_indefinite_article(&described_filter.description()));
            format!(
                "Activate only if {} {} {}",
                subject,
                player_verb(&subject, "control", "controls"),
                described
            )
        }
        crate::ConditionExpr::ControlCreaturesTotalPowerAtLeast(power) => {
            format!("Activate only if creatures you control have total power {power} or greater")
        }
        crate::ConditionExpr::CardInYourGraveyard {
            card_types,
            subtypes,
        } => {
            let mut descriptors: Vec<String> = Vec::new();
            for subtype in subtypes {
                descriptors.push(subtype.to_string().to_ascii_lowercase());
            }
            for card_type in card_types {
                descriptors.push(card_type.name().to_string());
            }
            descriptors.retain(|entry| !entry.is_empty());
            descriptors.dedup();

            if descriptors.is_empty() {
                "Activate only if there is a card in your graveyard".to_string()
            } else if descriptors.len() == 1 {
                format!(
                    "Activate only if there is an {} card in your graveyard",
                    descriptors[0]
                )
            } else {
                let head = descriptors[..descriptors.len() - 1].join(" ");
                let tail = descriptors.last().expect("descriptor tail");
                format!("Activate only if there is a {head} {tail} card in your graveyard")
            }
        }
        crate::ConditionExpr::ActivationTiming(timing) => match timing {
            ActivationTiming::AnyTime => "Activate only as an instant".to_string(),
            ActivationTiming::SorcerySpeed => "Activate only as a sorcery".to_string(),
            ActivationTiming::DuringCombat => "Activate only during combat".to_string(),
            ActivationTiming::OncePerTurn => "Activate only once each turn".to_string(),
            ActivationTiming::DuringYourTurn => "Activate only during your turn".to_string(),
            ActivationTiming::DuringOpponentsTurn => {
                "Activate only during an opponent's turn".to_string()
            }
            ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep => {
                "Any player may activate this ability but only during their turn before the end step"
                    .to_string()
            }
            ActivationTiming::DuringSourceOwnersUpkeep => {
                "Activate only during this card's owner's upkeep".to_string()
            }
        },
        crate::ConditionExpr::MaxActivationsPerTurn(limit) => {
            if *limit == 1 {
                "Activate only once each turn".to_string()
            } else if *limit == 2 {
                "Activate no more than twice each turn".to_string()
            } else {
                let count = number_word(*limit as i32).unwrap_or_else(|| limit.to_string());
                format!("Activate no more than {count} times each turn")
            }
        }
        _ => {
            let described = describe_condition(condition);
            let described = described.trim().trim_end_matches('.');
            if described.is_empty() {
                "Activate only if this condition is met".to_string()
            } else {
                format!("Activate only if {}", lowercase_first(described))
            }
        }
    }
}

pub(crate) fn describe_enchant_filter(filter: &crate::object::AuraAttachmentFilter) -> String {
    match filter {
        crate::object::AuraAttachmentFilter::Object(filter) => {
            let aura_creature_gate = filter.card_types.len() == 1
                && filter.card_types[0] == CardType::Creature
                && filter.subtypes.len() == 1
                && filter.subtypes[0] == crate::types::Subtype::Aura
                && filter.controller.is_none()
                && filter.owner.is_none()
                && filter.zone == Some(Zone::Battlefield);
            if aura_creature_gate {
                return "creature with another Aura attached to it".to_string();
            }
            let desc = filter.description();
            if let Some(stripped) = desc.strip_prefix("a ") {
                stripped.to_string()
            } else if let Some(stripped) = desc.strip_prefix("an ") {
                stripped.to_string()
            } else {
                desc
            }
        }
        crate::object::AuraAttachmentFilter::Player(filter) => match filter {
            crate::target::PlayerFilter::Any => "player".to_string(),
            crate::target::PlayerFilter::Opponent => "opponent".to_string(),
            crate::target::PlayerFilter::You => "you".to_string(),
            other => crate::filter::describe_player_filter(other),
        },
    }
}

pub(crate) fn describe_additional_costs(costs: &[crate::costs::Cost]) -> String {
    fn normalize_additional_cost_surface(text: String) -> String {
        let mut text = if let Some(rest) = text.strip_prefix("may ") {
            format!("you may {rest}")
        } else if let Some(rest) = text.strip_prefix("May ") {
            format!("you may {rest}")
        } else {
            text
        };
        if text.contains("exile ") {
            text = text
                .replace(" cards in your graveyard", " cards from your graveyard")
                .replace(" card in your graveyard", " card from your graveyard");
        }
        text = text.replace("a untapped ", "an untapped ");
        if let Some(rest) = text.strip_prefix("you may Blight ") {
            text = format!("you may blight {rest}");
        }
        text
    }

    fn describe_blight_cost(amount: &str) -> String {
        format!("you may blight {amount}")
    }

    fn blight_amount_from_choose_and_put(
        choose: &crate::effects::ChooseObjectsEffect,
        put_counters: &crate::effects::PutCountersEffect,
    ) -> Option<String> {
        if choose.filter != crate::filter::ObjectFilter::creature().you_control()
            || choose.count.min != 1
            || choose.count.max != Some(1)
            || put_counters.counter_type != crate::object::CounterType::MinusOneMinusOne
        {
            return None;
        }
        let ChooseSpec::Tagged(tag) = &put_counters.target else {
            return None;
        };
        if *tag != choose.tag {
            return None;
        }
        Some(describe_value(&put_counters.amount))
    }

    if costs.len() == 2
        && let Some(choose_type) = costs[0]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseCreatureTypeEffect>())
        && let Some(behold) = costs[1]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        && choose_type.chooser == PlayerFilter::You
        && choose_type.family == crate::types::SubtypeFamily::Creature
        && choose_type.excluded_subtypes.is_empty()
        && behold.chooser == PlayerFilter::You
        && behold.count == crate::effect::ChoiceCount::exactly(2)
        && behold.zone == Some(Zone::Battlefield)
        && behold.additional_zones.as_slice() == [Zone::Hand]
        && behold.reveal
        && behold.filter.any_of.len() == 2
        && behold.filter.any_of.iter().all(|filter| {
            filter.card_types.as_slice() == [CardType::Creature]
                && filter.chosen_creature_type
                && matches!(filter.zone, Some(Zone::Battlefield | Zone::Hand))
        })
    {
        return "you may choose a creature type and behold two creatures of that type".to_string();
    }

    if costs.len() == 1
        && let Some(may) = costs[0]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::MayEffect>())
        && may.effects.len() == 1
        && let Some(put_counters) =
            may.effects[0].downcast_ref::<crate::effects::PutCountersEffect>()
        && put_counters.counter_type == crate::object::CounterType::MinusOneMinusOne
        && put_counters.target
            == ChooseSpec::Object(crate::filter::ObjectFilter::creature().you_control())
        && put_counters.target_count.is_none()
        && !put_counters.distributed
    {
        return describe_blight_cost(&describe_value(&put_counters.amount));
    }

    if costs.len() == 1
        && let Some(may) = costs[0]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::MayEffect>())
        && may.effects.len() == 2
        && let Some(choose) = may.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(put_counters) =
            may.effects[1].downcast_ref::<crate::effects::PutCountersEffect>()
        && let Some(amount) = blight_amount_from_choose_and_put(choose, put_counters)
    {
        return describe_blight_cost(&amount);
    }

    if costs.len() == 2
        && let Some(choose) = costs[0]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        && let Some(put_counters) = costs[1]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::PutCountersEffect>())
        && let Some(amount) = blight_amount_from_choose_and_put(choose, put_counters)
    {
        return describe_blight_cost(&amount);
    }

    if costs.len() == 1
        && let Some(choose_mode) = costs[0]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseModeEffect>())
    {
        if let Some(text) = describe_reveal_from_hand_or_pay_mode_cost(choose_mode) {
            return text;
        }

        let min = choose_mode.min_choose_count.clone();
        if choose_mode.choose_count == Value::Fixed(1) && min == Value::Fixed(1) {
            let mut options = Vec::new();
            for mode in &choose_mode.modes {
                let mut text = describe_effect_list(&mode.effects);
                text = text.trim().trim_end_matches('.').to_string();
                if let Some(rest) = text.strip_prefix("you ") {
                    text = normalize_you_verb_phrase(rest);
                }
                if let Some(rest) = text.strip_prefix("pay ") {
                    let normalized_cost = normalize_cost_amount_token(rest);
                    text = format!("pay {normalized_cost}");
                }
                if text.is_empty() {
                    continue;
                }
                options.push(text);
            }
            if options.len() >= 2 {
                return join_with_or(&options);
            }
        }
    }

    let described = join_with_and(
        &describe_cost_component_parts(costs)
            .into_iter()
            .map(|part| lowercase_first(&part))
            .collect::<Vec<_>>(),
    );
    if described == "may put a -1/-1 counter on a creature you control" {
        return describe_blight_cost("1");
    }
    if let Some(amount_text) = described
        .strip_prefix("may put ")
        .and_then(|rest| rest.strip_suffix(" -1/-1 counters on a creature you control"))
    {
        return describe_blight_cost(amount_text);
    }
    normalize_additional_cost_surface(described)
}

pub(super) fn describe_reveal_from_hand_or_pay_mode_cost(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() != 2
        || choose_mode.choose_count != Value::Fixed(1)
        || choose_mode.min_choose_count != Value::Fixed(1)
        || choose_mode.allow_repeated_modes
        || choose_mode.allow_repeat
        || choose_mode.random
    {
        return None;
    }

    let mut reveal: Option<String> = None;
    let mut pay: Option<String> = None;
    for mode in &choose_mode.modes {
        let source = mode.source_text.trim().trim_end_matches('.');
        let lower = source.to_ascii_lowercase();
        if lower.starts_with("reveal ") && lower.contains(" card from your hand") {
            reveal = Some(source.to_string());
        } else if let Some(rest) = lower.strip_prefix("pay ") {
            pay = Some(format!("pay {}", normalize_cost_amount_token(rest)));
        }
    }

    Some(format!("{} or {}", reveal?, pay?))
}

pub(crate) fn describe_alternative_costs(costs: &[crate::costs::Cost]) -> String {
    if costs.len() == 2
        && let Some(choose) = costs[0]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        && let Some(return_to_hand) = costs[1]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ReturnToHandEffect>())
        && let ChooseSpec::Target(inner) = &return_to_hand.spec
        && let ChooseSpec::Object(filter) = inner.as_ref()
    {
        let references_chosen = filter.tagged_constraints.len() == 1
            && filter.tagged_constraints[0].tag == choose.tag
            && filter.tagged_constraints[0].relation
                == crate::filter::TaggedOpbjectRelation::IsTaggedObject;
        if references_chosen {
            let mut described = choose.filter.clone();
            if described.zone == Some(Zone::Battlefield) {
                described.zone = None;
            }
            return format!("return {} to its owner's hand", described.description());
        }
    }

    if costs.iter().any(|cost| {
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            .is_some()
    }) {
        return describe_cost_list(costs);
    }

    let mut clauses = Vec::new();
    for cost in costs {
        let Some(effect) = cost.effect_ref() else {
            let clause = cost.display().trim().to_string();
            if !clause.is_empty() {
                clauses.push(clause);
            }
            continue;
        };
        if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>()
            && lose_life.player == ChooseSpec::Player(PlayerFilter::You)
        {
            clauses.push(format!("pay {} life", describe_value(&lose_life.amount)));
            continue;
        }
        if let Some((count, color_filter)) = effect.0.exile_from_hand_cost_info() {
            clauses.push(describe_exile_from_hand_as_cost_phrase(count, color_filter));
            continue;
        }

        let mut clause = describe_effect(effect)
            .trim()
            .trim_end_matches('.')
            .to_string();
        if let Some(rest) = clause.strip_prefix("you ") {
            clause = normalize_you_verb_phrase(rest);
        } else if let Some(rest) = clause.strip_prefix("You ") {
            clause = normalize_you_verb_phrase(rest);
        }
        clause = normalize_cost_phrase(&clause);
        if clause.is_empty() {
            continue;
        }
        clauses.push(clause);
    }

    if clauses.is_empty() {
        describe_cost_list(costs)
    } else {
        join_with_and(&clauses)
    }
}

pub(crate) fn describe_exile_from_hand_as_cost_phrase(
    count: u32,
    color_filter: Option<crate::color::ColorSet>,
) -> String {
    let count = count.max(1);
    let card_word = if count == 1 { "card" } else { "cards" };
    let amount = if count == 1 {
        "a".to_string()
    } else {
        small_number_word(count).unwrap_or_else(|| count.to_string())
    };
    let color_prefix = color_filter
        .map(|colors| describe_token_color_words(colors, false))
        .filter(|text| !text.is_empty())
        .map(|text| format!("{text} "))
        .unwrap_or_default();
    format!("exile {amount} {color_prefix}{card_word} from your hand")
}

pub(crate) fn describe_imprint_from_hand_phrase(
    imprint: &crate::effects::cards::ImprintFromHandEffect,
) -> String {
    let mut card_text = imprint.filter.description();
    if let Some((subject, zone_phrase)) = card_text.rsplit_once(" in ")
        && zone_phrase.to_ascii_lowercase().contains("hand")
    {
        card_text = format!("{subject} from {zone_phrase}");
    }
    let lower = card_text.to_ascii_lowercase();
    if !matches!(
        lower.split_whitespace().next(),
        Some("a" | "an" | "one" | "two" | "three" | "each" | "another")
    ) && (lower.ends_with("card") || lower.contains(" card "))
    {
        let article = if matches!(
            lower.split_whitespace().next(),
            Some("artifact" | "enchantment" | "instant")
        ) {
            "an"
        } else {
            "a"
        };
        card_text = format!("{article} {card_text}");
    }
    format!("you may exile {card_text}")
}

pub(crate) fn describe_optional_cost_line(cost: &crate::cost::OptionalCost) -> String {
    use crate::cost::OptionalCostKind;

    if let Some(line) = optional_additional_source_line(cost) {
        return line;
    }

    if cost.kind == OptionalCostKind::Additional
        && let Some(costs) = cost.cost.as_all()
        && describe_additional_costs(costs)
            == "you may choose a creature type and behold two creatures of that type"
    {
        return "As an additional cost to cast this spell, you may choose a creature type and behold two creatures of that type"
            .to_string();
    }

    let cost_text = cost
        .cost
        .as_all()
        .map(describe_cost_list)
        .unwrap_or_else(|| describe_total_cost_payment(&cost.cost));
    let label = cost.kind.canonical_label();
    if matches!(
        cost.kind,
        OptionalCostKind::Gift | OptionalCostKind::Waterbend
    ) {
        return cost.reference.display_label();
    }
    if matches!(
        cost.kind,
        OptionalCostKind::Additional | OptionalCostKind::Behold
    ) {
        let action = cost_text.trim().trim_end_matches('.');
        if action.is_empty() {
            return "As an additional cost to cast this spell, you may pay an additional cost"
                .to_string();
        }
        let action = normalize_you_verb_phrase(action.strip_prefix("You ").unwrap_or(action));
        let action = if cost.repeatable {
            repeatable_optional_cost_action(&action)
        } else {
            action
        };
        return format!("As an additional cost to cast this spell, you may {action}");
    }
    if matches!(cost.kind, OptionalCostKind::Conspire) {
        let reminder_cost = cost_text
            .trim()
            .trim_end_matches('.')
            .replacen("Tap ", "tap ", 1)
            .replace(" that each share", " that share")
            .replace("this spell", "it");
        format!(
            "Conspire (As you cast this spell, you may {reminder_cost}. When you do, copy it and you may choose a new target for the copy.)"
        )
    } else {
        match cost.kind {
            OptionalCostKind::Replicate => {
                if cost_text.trim().is_empty() {
                    label.to_string()
                } else if cost_text.trim_start().starts_with('{') {
                    format!("{label} {cost_text}")
                } else {
                    format!("{label}—{}", cost_text.trim())
                }
            }
            // Most optional-cost keywords render with a space-separated payload.
            OptionalCostKind::Bargain => label.to_string(),
            OptionalCostKind::Kicker
            | OptionalCostKind::Multikicker
            | OptionalCostKind::Buyback
            | OptionalCostKind::Entwine => {
                if cost_text.trim().is_empty() {
                    label.to_string()
                } else {
                    format!("{label} {cost_text}")
                }
            }
            OptionalCostKind::Squad => {
                if cost_text.trim().is_empty() {
                    label.to_string()
                } else if !cost_text.contains(',') {
                    format!("{label} {}", cost_text.trim_end_matches('.'))
                } else {
                    format!("{label}—{}", cost_text.trim_end_matches('.'))
                }
            }
            _ if cost.repeatable => {
                if cost_text.trim().is_empty() {
                    label.to_string()
                } else {
                    format!("{label}—{}.", cost_text.trim_end_matches('.'))
                }
            }
            _ => {
                if cost_text.trim().is_empty() {
                    label.to_string()
                } else {
                    format!("{label} {cost_text}")
                }
            }
        }
    }
}

pub(super) fn optional_additional_source_line(cost: &crate::cost::OptionalCost) -> Option<String> {
    use crate::cost::OptionalCostKind;

    if !matches!(
        cost.kind,
        OptionalCostKind::Additional | OptionalCostKind::Waterbend
    ) {
        return None;
    }
    let source = cost.source_label.trim().trim_end_matches('.');
    let lower = source.to_ascii_lowercase();
    if lower.starts_with("as an additional cost to cast this spell, you may waterbend ") {
        return Some(ensure_trailing_period(source));
    }
    if lower.starts_with("as an additional cost to cast this spell, you may sacrifice one or more ")
    {
        return Some(ensure_trailing_period(source));
    }
    if let Some(source_amount) =
        lower.strip_prefix("as an additional cost to cast this spell, you may blight ")
        && let Some(costs) = cost.cost.as_all()
        && describe_additional_costs(costs).to_ascii_lowercase()
            == format!("you may blight {source_amount}")
    {
        // The typed choose-and-counter payment proves Blight; the source
        // label preserves only its keyword spelling and must agree on the
        // amount before it can replace the expanded payment description.
        return Some(ensure_trailing_period(source));
    }
    if lower.starts_with("as an additional cost to cast this spell, reveal ")
        && lower.contains(" card from your hand or pay ")
    {
        return Some(ensure_trailing_period(source));
    }
    None
}

#[cfg(test)]
mod optional_blight_additional_cost_surface_tests {
    use super::*;

    #[test]
    fn public_route_keeps_typed_optional_blight_keyword_surface() {
        for oracle in [
            "As an additional cost to cast this spell, you may blight 1.\nDestroy target creature with mana value 2 or less. If this spell's additional cost was paid, you gain 2 life.",
            "As an additional cost to cast this spell, you may blight 2.\nDestroy target artifact.",
        ] {
            let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Optional Blight Probe",
            )
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("optional Blight additional cost should parse");
            let lines = crate::compiled_text::compiled_text_lines(&definition);
            assert_eq!(lines.first().map(String::as_str), oracle.lines().next());
        }
    }
}

pub(super) fn repeatable_optional_cost_action(action: &str) -> String {
    let action = action.trim();
    if let Some(rest) = action.strip_prefix("sacrifice a ") {
        return format!("sacrifice one or more {}", pluralize_noun_phrase(rest));
    }
    if let Some(rest) = action.strip_prefix("sacrifice an ") {
        return format!("sacrifice one or more {}", pluralize_noun_phrase(rest));
    }
    action.to_string()
}

#[cfg(test)]
mod activation_condition_surface_tests {
    use super::*;

    #[test]
    fn conjunctive_activation_timings_preserve_each_only_qualifier() {
        let condition = crate::ConditionExpr::And(
            Box::new(crate::ConditionExpr::ActivationTiming(
                ActivationTiming::DuringYourTurn,
            )),
            Box::new(crate::ConditionExpr::ActivationTiming(
                ActivationTiming::OncePerTurn,
            )),
        );

        assert_eq!(
            describe_mana_activation_condition(&condition),
            "Activate only during your turn and only once each turn"
        );
    }

    #[test]
    fn token_copy_uses_the_exact_tagged_sacrifice_cost_antecedent() {
        let sacrificed = TagKey::from("sacrifice_cost_0");
        let sacrifice = crate::costs::Cost::try_effect(
            Effect::sacrifice(
                ObjectFilter::default().with_subtype(crate::types::Subtype::Zombie),
                1,
            )
            .tag(sacrificed.clone()),
        )
        .expect("tagged sacrifice remains a cost");
        let copy = Effect::new(crate::effects::CreateTokenCopyEffect::new(
            ChooseSpec::Tagged(sacrificed),
            2,
            PlayerFilter::You,
        ));
        let ability = Ability::activated(crate::cost::TotalCost::from_cost(sacrifice), vec![copy]);
        let AbilityKind::Activated(activated) = &ability.kind else {
            unreachable!("fixture is activated")
        };

        assert_eq!(
            rewrite_sacrifice_cost_token_copy_reference(
                "Create two tokens that are copies of it".to_string(),
                activated,
            ),
            "Create two tokens that are copies of the sacrificed creature"
        );

        let unrelated_copy = Effect::new(crate::effects::CreateTokenCopyEffect::new(
            ChooseSpec::Tagged(TagKey::from("other")),
            2,
            PlayerFilter::You,
        ));
        let unrelated = Ability::activated(activated.mana_cost.clone(), vec![unrelated_copy]);
        let AbilityKind::Activated(unrelated) = &unrelated.kind else {
            unreachable!("fixture is activated")
        };
        assert_eq!(
            rewrite_sacrifice_cost_token_copy_reference(
                "Create two tokens that are copies of it".to_string(),
                unrelated,
            ),
            "Create two tokens that are copies of it"
        );
    }
}
