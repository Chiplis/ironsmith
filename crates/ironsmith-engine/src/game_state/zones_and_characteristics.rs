use super::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct PreparedEtbChoices {
    pub(crate) chosen_color: Option<crate::color::Color>,
    pub(crate) chosen_basic_land_type: Option<crate::types::Subtype>,
    pub(crate) chosen_land_type: Option<crate::types::Subtype>,
    pub(crate) chosen_creature_type: Option<crate::types::Subtype>,
    pub(crate) chosen_card_type: Option<crate::types::CardType>,
    pub(crate) chosen_player: Option<PlayerId>,
    pub(crate) chosen_named_option: Option<String>,
    pub(crate) noted_life_total: Option<i32>,
    pub(crate) power_toughness_choices:
        Vec<(i32, i32, Vec<crate::static_abilities::StaticAbility>)>,
    pub(crate) battle_protector: Option<PlayerId>,
    pub(crate) discard_hand: bool,
    pub(crate) as_enters_counters: Vec<(crate::object::CounterType, u32)>,
    pub(crate) as_enters_tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    pub(crate) transfer_as_enters_source_links: bool,
    pub(crate) as_enters_continuous_effects: Vec<crate::continuous::ContinuousEffectId>,
}

/// Real source cards represented by one prospective entering permanent.
#[derive(Debug, Clone)]
pub(crate) struct PreparedEntryComponent {
    pub(crate) snapshot: crate::snapshot::ObjectSnapshot,
    pub(crate) original_definition: crate::cards::CardDefinition,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedEtbEntry {
    pub(crate) result: crate::events::processing::EtbEventResult,
    pub(crate) choices: PreparedEtbChoices,
    /// Frozen by an owning zone operation before replacement programs run.
    pub(crate) zone_entry_lookback: Option<Vec<crate::snapshot::ObjectSnapshot>>,
    /// Authored entry face; movement establishes it before resolved copy and
    /// characteristic replacements are applied, never afterward.
    pub(crate) entry_definition: Option<crate::cards::CardDefinition>,
    pub(crate) physical_components: Vec<PreparedEntryComponent>,
    pub(crate) linked_face_mana_cost: Option<crate::mana::ManaCost>,
    /// Attachment specified by the authoring effect, validated during this commit.
    pub(crate) entry_attachment: Option<AttachmentTarget>,
    pub(crate) entry_attachment_requires_aura: bool,
}

fn named_color_creature_type_option(
    option: &str,
) -> Option<(crate::color::Color, crate::types::Subtype)> {
    let mut words = option.split_whitespace();
    let color = crate::color::Color::from_name(&words.next()?.to_ascii_lowercase())?;
    let subtype_name = words.collect::<Vec<_>>().join(" ");
    if subtype_name.is_empty() {
        return None;
    }
    let subtype = crate::types::Subtype::all_creature_types()
        .iter()
        .copied()
        .find(|subtype| subtype.to_string().eq_ignore_ascii_case(&subtype_name))?;
    Some((color, subtype))
}

/// A named as-enters option that is exactly one creature or land subtype.
fn named_subtype_option(option: &str) -> Option<crate::types::Subtype> {
    let name = option.trim();
    if name.is_empty() {
        return None;
    }
    [
        crate::types::SubtypeFamily::Creature,
        crate::types::SubtypeFamily::Land,
    ]
    .into_iter()
    .flat_map(|family| family.all_subtypes().iter().copied())
    .find(|subtype| subtype.to_string().eq_ignore_ascii_case(name))
}

fn as_enters_effect_program_from_ability(
    ability: &crate::ability::Ability,
) -> Option<(crate::resolution::ResolutionProgram, bool, bool)> {
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
        program,
        also_turns_face_up,
        turns_face_up_only,
        transforms_into,
        ..
    } = &static_ability.compiled_model()?.payload
    else {
        return None;
    };
    if transforms_into.is_some() {
        return None;
    }
    Some((program.clone(), *also_turns_face_up, *turns_face_up_only))
}

fn as_transforms_effect_program_from_ability(
    ability: &crate::ability::Ability,
) -> Option<crate::resolution::ResolutionProgram> {
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
        program,
        transforms_into: Some(_),
        ..
    } = &static_ability.compiled_model()?.payload
    else {
        return None;
    };
    Some(program.clone())
}

fn ability_retains_counters_moving_to(
    ability: &crate::ability::Ability,
    destination: Zone,
) -> bool {
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    let Some(model) = static_ability.compiled_model() else {
        return false;
    };
    matches!(
        &model.payload,
        ironsmith_core::StaticAbilityPayload::CountersRemainAcrossZoneChanges {
            excluded_destinations,
            ..
        } if !excluded_destinations.contains(&destination)
    )
}

#[derive(Debug, Clone, Default)]
struct AsEntersProgramExecution {
    ran: bool,
    continuous_effects: Vec<crate::continuous::ContinuousEffectId>,
    tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
}

fn merge_retained_tagged_objects(
    destination: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    source: &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
) {
    for (tag, snapshots) in source {
        let retained = destination.entry(tag.clone()).or_default();
        for snapshot in snapshots {
            if retained
                .iter()
                .all(|existing| existing.stable_id != snapshot.stable_id)
            {
                retained.push(snapshot.clone());
            }
        }
    }
}

impl GameState {
    /// The rest of an enter-as-copy choice once the copy entered: "If you do,
    /// it gains haste until end of turn" applies as part of the replacement;
    /// "When you do, exile that card" is a reflexive triggered ability
    /// (CR 603.12) whose "that card" is the copied object as it was.
    fn apply_enter_as_copy_followups(
        &mut self,
        new_id: ObjectId,
        copy_source_id: ObjectId,
        followups: &[ironsmith_core::EnterAsCopyFollowup],
    ) {
        let Some(controller) = self.controller_of_id(new_id) else {
            return;
        };
        for followup in followups {
            match followup {
                ironsmith_core::EnterAsCopyFollowup::GainsHasteUntilEndOfTurn => {
                    let effect = crate::continuous::ContinuousEffect::new(
                        new_id,
                        controller,
                        crate::continuous::EffectTarget::Specific(new_id),
                        crate::continuous::Modification::AddAbility(
                            crate::static_abilities::StaticAbility::haste(),
                        ),
                    )
                    .until(crate::effect::Until::EndOfTurn)
                    .with_expires_end_of_turn(self.turn.turn_number)
                    .with_source_type(crate::continuous::EffectSourceType::Resolution {
                        locked_targets: vec![new_id],
                    });
                    self.effect_store.continuous_effects.add_effect(effect);
                    self.mark_continuous_state_dirty();
                }
                ironsmith_core::EnterAsCopyFollowup::ExileCopiedObject => {
                    let Some(copied) = self.object(copy_source_id) else {
                        continue;
                    };
                    let copied_tag = crate::tag::TagKey::from("copied_object");
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        copied, self,
                    );
                    let mut tagged_objects = std::collections::HashMap::new();
                    tagged_objects.insert(copied_tag.clone(), vec![snapshot]);
                    crate::effects::composition::queue_reflexive_trigger(
                        self,
                        new_id,
                        controller,
                        vec![crate::effect::Effect::exile(crate::target::ChooseSpec::Tagged(
                            copied_tag,
                        ))],
                        tagged_objects,
                    );
                }
                ironsmith_core::EnterAsCopyFollowup::TapCopiedObjectFrozenWhileYouControlSource => {
                    // "When you do, tap the copied creature and it doesn't
                    // untap during its controller's untap step for as long as
                    // you control this creature" (CR 603.12): the reflexive
                    // ability refers to the copied object as it was.
                    let Some(copied) = self.object(copy_source_id) else {
                        continue;
                    };
                    let copied_tag = crate::tag::TagKey::from("copied_object");
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        copied, self,
                    );
                    let mut tagged_objects = std::collections::HashMap::new();
                    tagged_objects.insert(copied_tag.clone(), vec![snapshot]);
                    let frozen = crate::target::ObjectFilter::default().match_tagged(
                        copied_tag.clone(),
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                    );
                    crate::effects::composition::queue_reflexive_trigger(
                        self,
                        new_id,
                        controller,
                        vec![
                            crate::effect::Effect::tap(crate::target::ChooseSpec::Tagged(
                                copied_tag,
                            )),
                            crate::effect::Effect::cant_until(
                                crate::effect::Restriction::untap(frozen),
                                crate::effect::Until::YouStopControllingThis,
                            ),
                        ],
                        tagged_objects,
                    );
                }
            }
        }
    }

    /// CR 400.4a: an instant or sorcery card cannot enter the battlefield.
    ///
    /// This is a zone-change rule, not a replacement effect, so callers must
    /// check it before proposing ETB replacements or collecting entry choices.
    pub(crate) fn card_cannot_enter_battlefield(&self, object_id: ObjectId) -> bool {
        self.token_cannot_change_zones(object_id, Zone::Battlefield)
            || self.object(object_id).is_some_and(|object| {
                (object.kind == crate::object::ObjectKind::Card
                    && object.card_types.iter().any(|card_type| {
                        matches!(card_type, CardType::Instant | CardType::Sorcery)
                    }))
                    || self.entry_prohibited_by_cant_effect(object)
            })
    }

    /// CR 111.8: a token that has left the battlefield can't move to another
    /// zone or come back onto the battlefield; it stays where it is until
    /// state-based actions remove it (CR 704.5d).
    ///
    /// Tokens are staged in the command zone while being created, and a copy
    /// of a card (CR 707.12) is created as a token in the card's zone and then
    /// cast, so those two moves are not departures from the battlefield.
    pub(crate) fn token_cannot_change_zones(&self, object_id: ObjectId, new_zone: Zone) -> bool {
        self.object(object_id).is_some_and(|object| {
            if object.kind != crate::object::ObjectKind::Token || new_zone == Zone::Stack {
                return false;
            }
            match object.zone {
                Zone::Battlefield | Zone::Stack => false,
                // A token staged in the command zone during creation may still
                // enter; one that got there by leaving the battlefield may not.
                Zone::Command => self
                    .auxiliary_tracking
                    .departed_command_zone_tokens
                    .contains(&object_id),
                _ => true,
            }
        })
    }

    /// CR 614.17-style prohibitions such as Grafdigger's Cage: the filter is
    /// matched against the card where it currently is, so zone-bearing
    /// filters only stop entries from the named zones.
    fn entry_prohibited_by_cant_effect(&self, object: &crate::object::Object) -> bool {
        if object.zone == Zone::Battlefield {
            return false;
        }
        self.effect_store
            .cant_effects
            .cant_enter_battlefield
            .iter()
            .any(|restriction| {
                let mut ctx = crate::target::FilterContext::default();
                if let Some(source) = restriction.source {
                    ctx = ctx.with_source(source);
                }
                restriction.filter.matches(object, &ctx, self)
            })
    }

    fn execute_immediate_effect_programs(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        programs: Vec<crate::resolution::ResolutionProgram>,
        preparing_entry: bool,
        entry_event: Option<&crate::events::EnterBattlefieldEvent>,
        entry_reserved_objects: &std::collections::HashSet<ObjectId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<AsEntersProgramExecution, crate::effects::ExecutionError> {
        if programs.is_empty() {
            return Ok(AsEntersProgramExecution::default());
        }
        // Replay begins with the entire list. Per-program checkpoints cannot
        // undo a preceding program's payments or one-shot consumption.
        let checkpoint = self.clone();
        let result = self.execute_immediate_effect_programs_inner(
            source,
            controller,
            programs,
            preparing_entry,
            entry_event,
            entry_reserved_objects,
            decision_maker,
        );
        if result.is_err() || decision_maker.awaiting_choice() {
            self.restore_execution_checkpoint(checkpoint, result.is_ok() && decision_maker.awaiting_choice());
            return result.map(|_| AsEntersProgramExecution::default());
        }
        result
    }

    fn execute_immediate_effect_programs_inner(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        programs: Vec<crate::resolution::ResolutionProgram>,
        preparing_entry: bool,
        entry_event: Option<&crate::events::EnterBattlefieldEvent>,
        entry_reserved_objects: &std::collections::HashSet<ObjectId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<AsEntersProgramExecution, crate::effects::ExecutionError> {
        let initial_effect_ids: std::collections::HashSet<_> = self
            .effect_store
            .continuous_effects
            .effects()
            .iter()
            .map(|effect| effect.id)
            .collect();
        let mut execution = AsEntersProgramExecution {
            ran: true,
            ..AsEntersProgramExecution::default()
        };
        let optional_costs_paid = self
            .object(source)
            .map(|object| object.optional_costs_paid.clone())
            .unwrap_or_default();
        for program in programs {
            let provenance = self.provenance_graph_mut().alloc_root(
                crate::provenance::ProvenanceNodeKind::EffectExecution { source, controller },
            );
            let mut context =
                crate::effects::ExecutionContext::new(source, controller, decision_maker)
                    .with_optional_costs_paid(optional_costs_paid.clone())
                    .with_cause(crate::events::cause::EventCause::from_effect(
                        source, controller,
                    ))
                    .with_provenance(provenance);
            context.replacement.entry_counter_source = preparing_entry.then_some(source);
            context.replacement.entry_event = entry_event.cloned().map(Box::new);
            context.replacement.entry_reserved_objects = entry_reserved_objects.clone();
            let events = crate::game_loop::execute_resolution_program_typed(
                self,
                &mut context,
                controller,
                source,
                &program,
                None,
                &[],
            )?;
            for event in events {
                self.queue_trigger_event(provenance, event);
            }
            execution.continuous_effects = self
                .effect_store
                .continuous_effects
                .effects()
                .iter()
                .filter(|effect| !initial_effect_ids.contains(&effect.id))
                .map(|effect| effect.id)
                .collect();
            let awaiting_choice = context.decision_maker.awaiting_choice();
            merge_retained_tagged_objects(&mut execution.tagged_objects, &context.tagged_objects);
            if awaiting_choice {
                return Ok(execution);
            }
        }
        Ok(execution)
    }

    pub(crate) fn execute_entry_programs(
        &mut self, source: ObjectId, controller: PlayerId,
        programs: Vec<crate::resolution::ResolutionProgram>,
        entry_event: Option<&crate::events::EnterBattlefieldEvent>,
        dm: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<Option<PreparedEtbChoices>, crate::effects::ExecutionError> {
        self.execute_entry_programs_with_reservations(source, controller, programs,
            entry_event, &std::collections::HashSet::from([source]), dm)
    }

    pub(crate) fn execute_entry_programs_with_reservations(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        programs: Vec<crate::resolution::ResolutionProgram>,
        entry_event: Option<&crate::events::EnterBattlefieldEvent>,
        entry_reserved_objects: &std::collections::HashSet<ObjectId>,
        dm: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<Option<PreparedEtbChoices>, crate::effects::ExecutionError> {
        let checkpoint = self.clone();
        let result = self.execute_entry_programs_inner(source, controller, programs, entry_event, entry_reserved_objects, dm);
        if result.is_err() || dm.awaiting_choice() {
            *self = checkpoint;
            return result.map(|_| None);
        }
        result
    }

    fn execute_entry_programs_inner(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        programs: Vec<crate::resolution::ResolutionProgram>,
        entry_event: Option<&crate::events::EnterBattlefieldEvent>,
        entry_reserved_objects: &std::collections::HashSet<ObjectId>,
        dm: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<Option<PreparedEtbChoices>, crate::effects::ExecutionError> {
        let before = self.object(source).ok_or(crate::effects::ExecutionError::InvalidTarget)?.counters.clone();
        let execution = self.execute_immediate_effect_programs(source, controller, programs, true, entry_event, entry_reserved_objects, dm)?;
        if dm.awaiting_choice() {
            return Ok(None);
        }
        let after = self.object(source).ok_or(crate::effects::ExecutionError::InvalidTarget)?.counters.clone();
        let counters = after.iter().filter_map(|(kind, count)| {
            let previous = before.get(kind).copied().unwrap_or(0);
            (*count > previous).then(|| (*kind, *count - previous))
        }).collect();
        self.object_mut(source).ok_or(crate::effects::ExecutionError::InvalidTarget)?.counters = before;
        Ok(Some(PreparedEtbChoices {
            as_enters_counters: counters,
            as_enters_tagged_objects: execution.tagged_objects,
            transfer_as_enters_source_links: execution.ran,
            as_enters_continuous_effects: execution.continuous_effects,
            ..Default::default()
        }))
    }

    fn entry_text_abilities(&self, source: ObjectId) -> Option<Vec<crate::ability::Ability>> {
        let from = self.object(source)?.zone;
        let preview =
            crate::events::EnterBattlefieldEvent::new(source, from).prospective_game_state(self)?;
        let effects = preview.all_continuous_effects();
        let chars = crate::continuous::text_box_characteristics_with_effects(
            source,
            preview.objects_map(),
            &effects,
            &preview.battlefield,
            preview.commander_objects(),
            &preview,
        )?;
        Some(chars.abilities.iter().cloned().collect())
    }

    fn execute_as_enters_effect_programs_from_abilities(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        abilities: &[crate::ability::Ability],
        for_turn_face_up: bool,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<AsEntersProgramExecution, crate::effects::ExecutionError> {
        let checkpoint = self.clone();
        let result = (|| {
            let mut current_abilities = abilities.to_vec();
            let mut applied = std::collections::HashSet::new();
            let mut combined = AsEntersProgramExecution::default();
            loop {
                let next = current_abilities.iter().find_map(|ability| {
                    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                        return None;
                    };
                    if applied.contains(&static_ability.instance_id()) {
                        return None;
                    }
                    let (program, also_face_up, only_face_up) =
                        as_enters_effect_program_from_ability(ability)?;
                    let applies = if for_turn_face_up {
                        also_face_up
                    } else {
                        !only_face_up
                    };
                    applies.then_some((static_ability.instance_id(), program))
                });
                let Some((identity, program)) = next else {
                    break;
                };
                applied.insert(identity);
                let execution = self
                    .execute_immediate_effect_programs(
                        source,
                        controller,
                        vec![program],
                        !for_turn_face_up,
                        None,
                        &std::collections::HashSet::new(),
                        decision_maker,
                    )?;
                combined.ran |= execution.ran;
                combined
                    .continuous_effects
                    .extend(execution.continuous_effects.iter().copied());
                merge_retained_tagged_objects(&mut combined.tagged_objects, &execution.tagged_objects);
                if decision_maker.awaiting_choice() {
                    return Ok(combined);
                }
                // A text exchange can remove a not-yet-applied program and supply
                // another one. Each ability instance applies at most once to this
                // event, even if a later exchange brings its text back.
                let changed_text =
                    self.effect_store
                        .continuous_effects
                        .effects()
                        .iter()
                        .any(|effect| {
                            execution.continuous_effects.contains(&effect.id)
                                && matches!(
                                    effect.modification,
                                    crate::continuous::Modification::SetTextBox(_)
                                )
                                && matches!(&effect.source_type,
                            crate::continuous::EffectSourceType::Resolution { locked_targets }
                            if locked_targets.contains(&source))
                        });
                if changed_text {
                    if let Some(abilities) = self.entry_text_abilities(source) {
                        current_abilities = abilities;
                    }
                }
            }
            Ok(combined)
        })();
        if result.is_err() || decision_maker.awaiting_choice() {
            self.restore_execution_checkpoint(checkpoint, result.is_ok() && decision_maker.awaiting_choice());
            return result.map(|_| AsEntersProgramExecution::default());
        }
        result
    }

    pub(crate) fn execute_as_transforms_effect_programs(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::effects::ExecutionError> {
        let abilities = self
            .object(source)
            .map(|object| object.abilities_vec())
            .unwrap_or_default();
        let programs = abilities
            .iter()
            .filter_map(as_transforms_effect_program_from_ability)
            .collect::<Vec<_>>();
        let execution = self
            .execute_immediate_effect_programs(
                source,
                controller,
                programs,
                false,
                None,
                &std::collections::HashSet::new(),
                decision_maker,
            )?;
        if let Some(object) = self.object_mut(source) {
            merge_retained_tagged_objects(
                &mut object.cast_tagged_objects,
                &execution.tagged_objects,
            );
        }
        Ok(())
    }

    pub(crate) fn execute_as_enters_effect_programs_for_turn_face_up(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::effects::ExecutionError> {
        let abilities = self
            .object(source)
            .map(|object| object.abilities_vec())
            .unwrap_or_default();
        let execution = self.execute_as_enters_effect_programs_from_abilities(
            source,
            controller,
            &abilities,
            true,
            decision_maker,
        )?;
        if let Some(object) = self.object_mut(source) {
            merge_retained_tagged_objects(
                &mut object.cast_tagged_objects,
                &execution.tagged_objects,
            );
        }
        Ok(())
    }

    /// "If this enchantment leaves the battlefield, this effect continues
    /// until end of turn" (Titania's Song): as the permanent leaves, the
    /// continuous effects its other static abilities generate are registered
    /// until end of turn, keeping their timestamp and applying to whatever
    /// their filters match, as they did while the source was on the
    /// battlefield. They form one effect for CR 613.6.
    fn register_lingering_static_effects_on_leave(&mut self, source: ObjectId) {
        use crate::static_abilities::StaticAbilityId;
        if !self.current_has_static_ability_id(
            source,
            StaticAbilityId::StaticEffectsContinueUntilEndOfTurnAfterLeaving,
        ) {
            return;
        }
        let Some(abilities) = self.current_abilities(source) else {
            return;
        };
        let Some(object) = self.object(source) else {
            return;
        };
        let controller = self.controller_of(object);
        let zone = object.zone;
        let mut effects = Vec::new();
        for ability in &abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if !ability.functions_in(&zone)
                || static_ability.id()
                    == StaticAbilityId::StaticEffectsContinueUntilEndOfTurnAfterLeaving
            {
                continue;
            }
            effects.extend(static_ability.generate_effects(source, controller, self));
        }
        if effects.is_empty() {
            return;
        }
        let timestamp = self
            .effect_store
            .continuous_effects
            .get_object_timestamp(source)
            .unwrap_or(0);
        let group = self.effect_store.continuous_effects.next_effect_group_id();
        for mut effect in effects {
            effect.duration = crate::effect::Until::EndOfTurn;
            effect.timestamp = timestamp;
            effect.group = Some(group);
            effect.source_type = crate::continuous::EffectSourceType::StaticAbility;
            effect.originating_static_ability = None;
            effect.originating_ability = None;
            self.effect_store.continuous_effects.add_effect(effect);
        }
        self.mark_continuous_state_dirty();
    }

    pub fn move_object(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
    ) -> Option<ObjectId> {
        self.move_object_with_snapshot(old_id, new_zone, cause, None)
    }

    pub(crate) fn move_object_with_snapshot(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Option<ObjectId> {
        self.move_object_with_snapshot_and_entry_prevalidation(
            old_id,
            new_zone,
            cause,
            lki_snapshot,
            false,
        )
    }

    fn move_object_with_snapshot_and_entry_prevalidation(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
        entry_prevalidated: bool,
    ) -> Option<ObjectId> {
        let pre_event_lookback_source_snapshots = if self
            .may_have_triggered_abilities_for_event_kind(crate::events::EventKind::ZoneChange)
        {
            self.trigger_source_lookback_snapshots()
        } else {
            Vec::new()
        };
        self.move_object_with_snapshot_and_pre_event_lookback_internal(
            old_id,
            new_zone,
            cause,
            lki_snapshot,
            &pre_event_lookback_source_snapshots,
            entry_prevalidated,
            None,
            &[],
            None,
        )
    }

    fn move_prepared_zone_entry_with_lookback(
        &mut self,
        old_id: ObjectId,
        zone: Zone,
        cause: crate::events::cause::EventCause,
        snapshot: Option<crate::snapshot::ObjectSnapshot>,
        lookback: Option<&[crate::snapshot::ObjectSnapshot]>,
        entry_prevalidated: bool,
        entry_definition: Option<&crate::cards::CardDefinition>,
        physical_components: &[PreparedEntryComponent],
        entry_linked_face_mana_cost: Option<&crate::mana::ManaCost>,
    ) -> Option<ObjectId> {
        let owned_lookback;
        let lookback = match lookback {
            Some(lookback) => lookback,
            None => {
                owned_lookback = if self.may_have_triggered_abilities_for_event_kind(crate::events::EventKind::ZoneChange) {
                    self.trigger_source_lookback_snapshots()
                } else { Vec::new() };
                &owned_lookback
            }
        };
        if !physical_components.is_empty() && zone != Zone::Battlefield {
            let opened = self.open_simultaneous_action();
            let mut results = Vec::new();
            for component in physical_components {
                let id = component.snapshot.object_id;
                if let Some(object) = self.object_mut(id) {
                    object.apply_definition_face(&component.original_definition);
                }
                if let Some(new_id) = self.move_object_with_snapshot_and_pre_event_lookback_internal(
                    id, zone, cause.clone(), Some(component.snapshot.clone()), lookback,
                    entry_prevalidated, None, &[], None,
                ) { results.push(new_id); }
            }
            self.close_simultaneous_action(opened);
            self.record_zone_change_results(old_id, results.clone());
            return results.first().copied();
        }
        self.move_object_with_snapshot_and_pre_event_lookback_internal(
            old_id, zone, cause, snapshot, lookback, entry_prevalidated, entry_definition, physical_components, entry_linked_face_mana_cost,
        )
    }

    pub(crate) fn move_object_with_snapshot_and_pre_event_lookback(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
        pre_event_lookback_source_snapshots: &[crate::snapshot::ObjectSnapshot],
    ) -> Option<ObjectId> {
        self.move_object_with_snapshot_and_pre_event_lookback_internal(
            old_id,
            new_zone,
            cause,
            lki_snapshot,
            pre_event_lookback_source_snapshots,
            false,
            None,
            &[],
            None,
        )
    }

    fn move_object_with_snapshot_and_pre_event_lookback_internal(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
        pre_event_lookback_source_snapshots: &[crate::snapshot::ObjectSnapshot],
        entry_prevalidated: bool,
        entry_definition: Option<&crate::cards::CardDefinition>,
        physical_components: &[PreparedEntryComponent],
        entry_linked_face_mana_cost: Option<&crate::mana::ManaCost>,
    ) -> Option<ObjectId> {
        // CR 311.2/312.2: planar cards remain in the command zone even if an
        // effect attempts to move them. Turning them face down is handled by
        // the Planechase procedure because it creates a new command-zone object.
        if self.is_planar_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        if self.is_vanguard_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        // CR 314.2: scheme cards remain in the command zone. Setting one in
        // motion or turning it face down changes its status, not its zone.
        if self.is_scheme_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        // CR 315.3: conspiracy cards remain in command. Turning an agenda
        // conspiracy face up changes only its status.
        if self.is_conspiracy_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        // CR 717.6: an Attraction card goes to its owner's junkyard in the
        // command zone instead of a graveyard, hand or library.
        let new_zone = self.attraction_card_destination(old_id, new_zone);
        // One already in the command zone (its Attraction deck or junkyard)
        // stays where it is.
        if new_zone == Zone::Command
            && self.is_attraction_card(old_id)
            && self
                .objects
                .get(&old_id)
                .is_some_and(|object| object.zone == Zone::Command)
        {
            return Some(old_id);
        }
        if self.objects.get(&old_id).is_some_and(|object| object.zone == new_zone)
            && !matches!(new_zone, Zone::Exile | Zone::Command)
        {
            // Preserve identity and all object state. Library positioning is
            // an explicit instruction handled by the caller, not a zone change.
            return Some(old_id);
        }
        // A simultaneous exchange checks entry legality before either member
        // leaves its original zone. Its prepared commit must preserve that
        // answer instead of evaluating dynamic filters against a partial move.
        if new_zone == Zone::Battlefield
            && !entry_prevalidated
            && self.card_cannot_enter_battlefield(old_id)
        {
            return None;
        }
        if self.token_cannot_change_zones(old_id, new_zone) {
            return None;
        }
        // Use the object's current typed abilities while it is still in the
        // origin zone. This honors ability-loss effects on the battlefield and
        // still lets an all-zone retention ability operate from other zones.
        let retain_counters = lki_snapshot.as_ref().map_or_else(
            || {
                self.current_abilities(old_id).is_some_and(|abilities| {
                    abilities
                        .iter()
                        .any(|ability| ability_retains_counters_moving_to(ability, new_zone))
                })
            },
            |snapshot| {
                snapshot
                    .abilities
                    .iter()
                    .any(|ability| ability_retains_counters_moving_to(ability, new_zone))
            },
        );
        let was_face_down = self.is_face_down(old_id);
        let was_foretold = self.is_foretold(old_id);
        let preserved_exile_viewers = if self
            .objects
            .get(&old_id)
            .is_some_and(|obj| obj.zone == Zone::Exile)
        {
            self.exile_tracking_mut()
                .face_down_exile_viewers
                .remove(&old_id)
        } else {
            None
        };
        // Capture a full pre-move snapshot for LKI-based trigger matching.
        let pre_move_snapshot = lki_snapshot.or_else(|| {
            self.objects
                .get(&old_id)
                .map(|obj| self.cached_object_snapshot_with_calculated_characteristics(obj))
        });
        if self
            .objects
            .get(&old_id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            && new_zone != Zone::Battlefield
        {
            self.register_lingering_static_effects_on_leave(old_id);
            self.release_phase_out_holds_for_source(old_id);
            self.note_attraction_left_battlefield(old_id);
            // CR 506.4: a planeswalker or battle that leaves the battlefield
            // stops being attacked.
            self.remove_attacked_permanent_from_combat(old_id, None);
            // CR 506.4: an attacking or blocking creature that leaves the
            // battlefield is removed from combat. The new object in the
            // destination zone is unrelated to the departed combatant (CR
            // 400.7), so its old ID must not linger in the combat state.
            self.remove_object_from_combat(old_id);
        }
        if let Some(snapshot) = pre_move_snapshot.as_ref() {
            for entry in &mut self.stack {
                if entry.is_ability
                    && entry
                        .triggering_event
                        .as_ref()
                        .and_then(|event| event.object_id())
                        .is_some_and(|object_id| object_id == old_id)
                {
                    entry.tagged_objects.insert(
                        crate::tag::TagKey::from("triggering"),
                        vec![snapshot.clone()],
                    );
                    entry
                        .tagged_objects
                        .entry(crate::tag::TagKey::from("__it__"))
                        .or_insert_with(|| vec![snapshot.clone()]);
                }
                for tagged_snapshots in entry.tagged_objects.values_mut() {
                    for tagged_snapshot in tagged_snapshots {
                        if (tagged_snapshot.object_id == old_id
                            || tagged_snapshot.stable_id == snapshot.stable_id)
                            && tagged_snapshot.zone == snapshot.zone
                        {
                            *tagged_snapshot = snapshot.clone();
                        }
                    }
                }
                if entry.is_ability
                    && (entry.object_id == old_id
                        || entry
                            .source_stable_id
                            .is_some_and(|id| id == snapshot.stable_id))
                {
                    let should_update_source_lki = entry
                        .source_snapshot
                        .as_ref()
                        .is_none_or(|source_snapshot| source_snapshot.zone == snapshot.zone);
                    if !should_update_source_lki {
                        continue;
                    }
                    entry.source_stable_id = Some(snapshot.stable_id);
                    entry
                        .source_name
                        .get_or_insert_with(|| snapshot.name.to_string());
                    entry.source_snapshot = Some(snapshot.clone());
                }
            }
        }

        self.object_store.changes.record(old_id);
        self.object_store.render_changes.record(old_id);
        let old_object = ObjectStore::into_owned_object(self.objects.remove(&old_id)?);
        self.turn_store.forecast_revealed_hand_cards.remove(&old_id);
        let hidden_card_info = self.auxiliary_tracking_mut().hidden_cards.remove(&old_id);
        if hidden_card_info.is_some()
            && old_object.zone.is_public()
            && !was_face_down
            && !was_foretold
        {
            // Every peer had to open this card while it sat face up in a
            // public zone, checking its claims then: settle them now,
            // identically on every peer.
            self.settle_public_hidden_identity_obligations(old_object.stable_id);
        }
        self.auxiliary_tracking_mut()
            .sector_designations
            .remove(&old_id);
        // A publicly revealed hidden card becomes a new object (CR 400.7).
        self.forget_public_hidden_card_reveal(old_id);
        self.clear_hidden_face_down_cast_claim(old_id);
        self.stable_id_index.remove(&old_object.stable_id);
        self.commander_tracking_mut()
            .declined_command_zone_moves
            .remove(&old_id);
        let old_zone = old_object.zone;
        let owner = old_object.owner;

        let preserves_exile_grants_for_adventure_stack_cast = old_zone == Zone::Exile
            && new_zone == Zone::Stack
            && crate::decision::spell_has_adventure_half(self, &old_object);
        if old_zone != new_zone && !preserves_exile_grants_for_adventure_stack_cast {
            self.effect_store
                .grant_registry
                .remove_stable_card_grants_for_zone(old_object.stable_id, old_zone);
        }
        if old_zone == Zone::Stack && new_zone != Zone::Exile {
            self.effect_store
                .grant_registry
                .remove_stable_card_grants_for_zone(old_object.stable_id, Zone::Exile);
        }

        if let Some(target) = old_object.attached_to {
            match target {
                AttachmentTarget::Object(id) => {
                    if let Some(parent) = self.object_mut(id) {
                        parent.attachments.retain(|existing| *existing != old_id);
                    }
                }
                AttachmentTarget::Player(id) => {
                    if let Some(player) = self.player_mut(id) {
                        player.attachments.retain(|existing| *existing != old_id);
                    }
                }
            }
        }

        // Remove from old zone index
        self.remove_from_zone_index(old_id, old_zone, owner);

        // Clear state from old zone's extension maps
        if old_zone == Zone::Battlefield {
            self.clear_battlefield_state(old_id);
            self.clear_player_control_from_source(old_object.stable_id);
        }
        if old_zone == Zone::Exile {
            self.clear_exile_state(old_id);
        }
        if old_zone == Zone::Stack {
            self.exile_tracking_mut()
                .cast_origin_snapshots
                .remove(&old_id);
        }

        if old_zone == Zone::Battlefield
            && new_zone != Zone::Battlefield
            && let Some(merged) = self.merged_permanent(old_object.stable_id).cloned()
        {
            let component_destinations = self
                .commander_tracking_mut()
                .pending_merged_component_destinations
                .remove(&old_object.stable_id);
            let mut result_object_ids = Vec::with_capacity(merged.components.len());
            for (index, component) in merged.components.iter().enumerate() {
                let component_zone = component_destinations
                    .as_ref()
                    .and_then(|destinations| destinations.get(index))
                    .copied()
                    .unwrap_or(new_zone);
                let new_component_id =
                    self.create_merged_component_object(component, component_zone)?;
                result_object_ids.push(new_component_id);
            }
            self.commander_tracking_mut()
                .merged_permanents
                .remove(&old_object.stable_id);

            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            let event = ZoneChangeEvent::with_results(
                old_id,
                result_object_ids.clone(),
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot,
            );
            let event_provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance)
                    .with_lookback_source_snapshots(pre_event_lookback_source_snapshots.to_vec()),
            );
            self.record_zone_change_results(old_id, result_object_ids.clone());
            if old_zone != new_zone {
                for result_object_id in &result_object_ids {
                    self.record_ui_zone_transition(old_id, *result_object_id, old_zone, new_zone);
                }
            }

            #[cfg(debug_assertions)]
            self.debug_assert_zone_consistency();
            self.reconcile_ring_bearers();
            return result_object_ids.first().copied();
        }

        if old_zone == Zone::Battlefield
            && new_zone != Zone::Battlefield
            && let Some(melded) = self.melded_permanent(old_object.stable_id).cloned()
        {
            // CR 903.9c: per-component commander destinations, if any.
            let component_destinations = self
                .commander_tracking_mut()
                .pending_merged_component_destinations
                .remove(&old_object.stable_id);
            let mut result_object_ids = Vec::with_capacity(melded.components.len());
            for (index, component) in melded.components.iter().enumerate() {
                let component_zone = component_destinations
                    .as_ref()
                    .and_then(|destinations| destinations.get(index))
                    .copied()
                    .unwrap_or(new_zone);
                let new_component_id =
                    self.create_meld_component_object(component, component_zone)?;
                result_object_ids.push(new_component_id);
            }
            self.commander_tracking_mut()
                .melded_permanents
                .remove(&old_object.stable_id);

            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            let event = ZoneChangeEvent::with_results(
                old_id,
                result_object_ids.clone(),
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot,
            );
            let event_provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance)
                    .with_lookback_source_snapshots(pre_event_lookback_source_snapshots.to_vec()),
            );
            self.record_zone_change_results(old_id, result_object_ids.clone());
            if old_zone != new_zone {
                for result_object_id in &result_object_ids {
                    self.record_ui_zone_transition(old_id, *result_object_id, old_zone, new_zone);
                }
            }

            #[cfg(debug_assertions)]
            self.debug_assert_zone_consistency();

            self.reconcile_ring_bearers();

            return result_object_ids.first().copied();
        }

        // Create new object with new ID (zone change = new object per rule 400.7)
        let new_id = self.new_object_id();
        let mut new_object = old_object;
        new_object.id = new_id;
        new_object.zone = new_zone;
        if old_zone == Zone::Command {
            self.auxiliary_tracking_mut()
                .departed_command_zone_tokens
                .remove(&old_id);
        }
        if new_object.kind == crate::object::ObjectKind::Token
            && old_zone == Zone::Battlefield
            && new_zone == Zone::Command
        {
            self.auxiliary_tracking_mut()
                .departed_command_zone_tokens
                .insert(new_id);
        }
        if old_zone == Zone::Stack && new_zone != Zone::Stack {
            new_object.end_splice_cast_overlay();
        }
        if old_zone == Zone::Stack
            && new_zone == Zone::Battlefield
            && matches!(new_object.kind, crate::object::ObjectKind::SpellCopy)
        {
            new_object.kind = crate::object::ObjectKind::Token;
        }
        // Counters are tied to the object instance, not to the physical card.
        // `move_object` always creates the new object for the destination.
        if !retain_counters {
            new_object.counters.clear();
        }

        // Reset zone-specific state on the object
        new_object.attached_to = None;
        new_object.attachments.clear();
        // Casting-contribution state should not persist across arbitrary zone changes.
        // Preserve it only for Stack -> Battlefield (a spell resolving into a permanent).
        let preserve_face_down_overlay =
            new_zone == Zone::Battlefield && new_object.face_down_cast_state.is_some();
        let preserve_bestow_overlay =
            new_zone == Zone::Battlefield && new_object.bestow_cast_state.is_some();
        let preserve_prototype_overlay = matches!(new_zone, Zone::Stack | Zone::Battlefield)
            && new_object.prototype_cast_state.is_some();
        let preserve_temporary_static_ability_grants =
            old_zone == Zone::Stack && new_zone == Zone::Battlefield;
        let preserve_cast_tags =
            new_zone == Zone::Stack || (old_zone == Zone::Stack && new_zone == Zone::Battlefield);
        let preserve_optional_costs_paid = old_zone == Zone::Stack && new_zone == Zone::Battlefield;
        let preserve_x_value = old_zone == Zone::Stack && new_zone == Zone::Battlefield;
        // Restore enters-as characteristics first, so the printed values saved
        // by the face-down, bestow and prototype overlays below always win.
        if old_zone == Zone::Battlefield && new_zone != Zone::Battlefield {
            new_object.end_enters_as_copy_overlay();
        }
        if !preserve_prototype_overlay {
            new_object.end_prototype_cast_overlay();
        }
        if !preserve_face_down_overlay && !preserve_bestow_overlay {
            new_object.keyword_payment_contributions_to_cast.clear();
            // The face-down and bestow overlays overwrote the object's
            // copiable fields; restore the printed card as it leaves (CR 708.9,
            // 400.7, 702.103). Merely dropping the saved state would leave the
            // card in its new zone as a "Face-down creature" / Aura.
            new_object.end_bestow_cast_overlay();
            new_object.end_face_down_cast_overlay();
        }
        if !preserve_x_value {
            new_object.x_value = None;
        }
        if !(old_zone == Zone::Stack && new_zone == Zone::Battlefield) {
            new_object.snow_mana_spent_to_cast = crate::player::ManaPool::default();
        }
        if !preserve_cast_tags {
            new_object.cast_tagged_objects.clear();
        }
        if !preserve_temporary_static_ability_grants {
            new_object.temporary_static_ability_grants.clear();
        }
        if !preserve_optional_costs_paid {
            new_object.optional_costs_paid = crate::cost::OptionalCostsPaid::default();
        }
        let ends_stack_text_or_effect_overlay = old_zone == Zone::Stack
            && new_zone != Zone::Stack
            && new_object
                .cast_alternative_method
                .as_deref()
                .is_some_and(|method| {
                    method.overload_effects().is_some()
                        || method.cleave_effects().is_some()
                        || method.awaken_effects().is_some()
                });
        if ends_stack_text_or_effect_overlay
            && let Some(card_id) = new_object.card
            && let Some(handles) = self.object_store.card_shared.get(&card_id)
        {
            new_object.restore_printed_spell_effect(handles);
        }
        new_object.cast_alternative_method = None;
        new_object.cast_play_from_constraints = None;
        // A card cast through a granted "it gains suspend" trigger carries a
        // synthetic "Suspend 0—{0}" permission only for that cast; it isn't a
        // printed ability and must not follow the card (CR 400.7, 702.62a).
        if old_zone == Zone::Stack
            && new_zone != Zone::Stack
            && new_object
                .alternative_casts
                .iter()
                .any(is_synthetic_granted_suspend)
        {
            new_object.alternative_casts = new_object
                .alternative_casts
                .iter()
                .filter(|method| !is_synthetic_granted_suspend(method))
                .cloned()
                .collect::<Vec<_>>()
                .into();
        }

        if old_zone == Zone::Stack
            && new_zone != Zone::Stack
            && (new_object.subtypes.contains(&Subtype::Adventure)
                || new_object.subtypes.contains(&Subtype::Omen))
            && let Some(front_def) = self.linked_face_definition_by_name_or_id(
                new_object.other_face_name.as_deref(),
                new_object.other_face,
            )
        {
            let handles = self.object_store.shared_handles_for_definition(&front_def);
            new_object.apply_definition_face_with_shared(&front_def, &handles);
        }
        // CR 712.8a / 712.14: outside the battlefield and the stack a
        // transforming double-faced card has only its front face, and it
        // enters the battlefield front face up unless an effect says otherwise.
        // A flipped flip card likewise leaves as the unflipped card (CR 710.2,
        // 710.4).
        let leaves_battlefield_or_stack = matches!(old_zone, Zone::Battlefield | Zone::Stack)
            && !matches!(new_zone, Zone::Battlefield | Zone::Stack);
        if (leaves_battlefield_or_stack
            || (entry_definition.is_some() && !matches!(new_zone, Zone::Battlefield | Zone::Stack))
            || (old_zone == Zone::Exile && new_zone == Zone::Battlefield))
            && matches!(
                new_object.linked_face_layout,
                LinkedFaceLayout::TransformLike | LinkedFaceLayout::Flip
            )
            && let Some(front_def) =
                self.default_face_definition_for_transform_like_return(&new_object)
        {
            let handles = self.object_store.shared_handles_for_definition(&front_def);
            new_object.apply_definition_face_with_shared(&front_def, &handles);
        }

        if new_zone == Zone::Battlefield && let Some(definition) = entry_definition {
            let handles = self.object_store.shared_handles_for_definition(definition);
            new_object.apply_definition_face_with_shared(definition, &handles);
        }

        // Set battlefield state for new permanents
        if new_zone == Zone::Battlefield {
            self.set_summoning_sick(new_id);
        }

        if !physical_components.is_empty() {
            // A combined permanent is one new representation of all sources,
            // rather than an extra physical card with a fabricated origin.
            new_object.stable_id = crate::ids::StableId::from(new_id);
            if let Some(definition) = entry_definition { new_object.card = Some(definition.card.id); }
            for component in physical_components {
                if component.snapshot.object_id != old_id {
                    self.remove_object(component.snapshot.object_id);
                }
            }
        }
        if new_zone == Zone::Battlefield {
            new_object.linked_face_mana_cost = entry_linked_face_mana_cost.cloned().map(Into::into)
                .or(new_object.linked_face_mana_cost);
        }
        let sticker_identity = new_object.stable_id;
        self.add_object(new_object);
        if old_zone == Zone::Stack && new_zone == Zone::Battlefield {
            self.effect_store.continuous_effects.retarget_resolved_permanent_spell(old_id, new_id);
        }
        self.move_stickers_to_new_object(sticker_identity, new_id, new_zone);
        if new_zone == Zone::Battlefield {
            self.note_attraction_entered_battlefield(new_id);
        }
        if let Some(mut info) = hidden_card_info {
            let audit_info = info.clone();
            let entering_library = (new_zone == Zone::Library && old_zone != Zone::Library)
                .then(|| audit_info.clone());
            info.zone = new_zone;
            self.auxiliary_tracking_mut()
                .hidden_cards
                .insert(new_id, info);
            self.hydrate_verified_library_replay_zone(new_id, new_zone);
            self.push_hidden_info_operation(HiddenInfoOperation::HiddenMove {
                owner: audit_info.owner,
                old_object_id: old_id,
                new_object_id: new_id,
                from: old_zone,
                to: new_zone,
                slot: audit_info.slot,
                commitment: audit_info.commitment,
            });
            if let Some(entering) = entering_library {
                // A claim subject entering a library is anchored to its
                // durable ziffle ciphertext (checked at the end-of-match
                // disclosure), since reshuffles break the object link.
                self.anchor_hidden_card_entering_library(new_id, &entering);
            } else if new_zone == Zone::Graveyard {
                // A card put into a graveyard is face up and opened on every
                // peer, which checks its claims then; it needs no anchor.
                self.forget_hidden_claim_subject(sticker_identity);
            }
        }

        // CR 406.3 / 400.7: a face-down exiled card that leaves exile is a new
        // object that enters face up (a Hideaway land played from exile, a
        // Praetor's Grasp card put onto the battlefield) unless the effect
        // itself says face down, which uses the face-down cast overlay. Only a
        // face-down permanent or spell carries its status onto the battlefield.
        if new_zone == Zone::Battlefield
            && ((was_face_down && matches!(old_zone, Zone::Battlefield | Zone::Stack))
                || self
                    .object(new_id)
                    .is_some_and(|obj| obj.face_down_cast_state.is_some()))
        {
            self.set_face_down(new_id);
        }
        if old_zone == Zone::Exile && new_zone == Zone::Exile && was_face_down {
            self.set_face_down(new_id);
            if let Some(viewers) = preserved_exile_viewers {
                for viewer in viewers {
                    self.grant_face_down_exile_view(new_id, viewer);
                }
            }
        }

        if old_zone != new_zone {
            self.record_ui_zone_transition(old_id, new_id, old_zone, new_zone);
        }
        // CR 708.9: a face-down permanent or spell that moves to another zone
        // is revealed by its owner (it isn't when it stays face down, e.g. a
        // face-down spell resolving). An identity this peer can't open yet is
        // disclosed by the hidden-card protocol instead.
        if was_face_down
            && matches!(old_zone, Zone::Battlefield | Zone::Stack)
            && old_zone != new_zone
            && !self.is_face_down(new_id)
            && !self.is_hidden_card_placeholder(new_id)
        {
            self.record_face_down_reveal(new_id, owner);
        }

        // Record entry timestamp per Rule 613.7d when entering the battlefield
        if new_zone == Zone::Battlefield {
            self.effect_store.continuous_effects.record_entry(new_id);
            self.handle_day_night_object_entered(new_id);
        }

        // Queue zone change event for triggers.
        if old_zone != new_zone {
            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            // For LTB-style moves we keep the pre-move object ID; for all others use
            // the destination object ID so ETB/"this enters" matching remains stable.
            let event_object_id = if old_zone == Zone::Battlefield {
                old_id
            } else {
                new_id
            };
            let event = if physical_components.is_empty() {
                ZoneChangeEvent::with_cause(event_object_id, old_zone, new_zone, cause, pre_move_snapshot.clone())
            } else {
                let mut event = ZoneChangeEvent::batch_with_snapshots(vec![new_id], old_zone, new_zone,
                    cause, physical_components.iter().map(|component| component.snapshot.clone()).collect());
                event.result_objects = vec![new_id];
                event
            };
            let mut event = event;
            if old_zone == Zone::Battlefield {
                event.result_objects = vec![new_id];
                if let Some(snapshot) = pre_move_snapshot.as_ref() {
                    for attachment_id in &snapshot.attachments {
                        if let Some(attachment) = self.object(*attachment_id) {
                            event = event.with_object_tag(
                                crate::tag::TagKey::from("attached_source"),
                                crate::snapshot::ObjectSnapshot::from_object(attachment, self),
                            );
                        }
                    }
                }
            }
            let event_provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance)
                    .with_lookback_source_snapshots(pre_event_lookback_source_snapshots.to_vec()),
            );
        }
        self.record_zone_change_results(old_id, vec![new_id]);
        for component in physical_components {
            self.record_zone_change_results(component.snapshot.object_id, vec![new_id]);
        }

        // Validate zone consistency in debug builds
        #[cfg(debug_assertions)]
        self.debug_assert_zone_consistency();

        self.reconcile_ring_bearers();

        Some(new_id)
    }

    pub(crate) fn default_face_definition_for_transform_like_return(
        &self,
        object: &Object,
    ) -> Option<crate::cards::CardDefinition> {
        let other_def = self.linked_face_definition_by_name_or_id(
            object.other_face_name.as_deref(),
            object.other_face,
        )?;
        let returns_to_default_face = |layout: LinkedFaceLayout| {
            matches!(
                layout,
                LinkedFaceLayout::TransformLike | LinkedFaceLayout::Flip
            )
        };
        if !returns_to_default_face(other_def.card.linked_face_layout) {
            return None;
        }
        let current_def =
            self.linked_face_definition_by_name_or_id(Some(&object.name), object.card)?;
        if !returns_to_default_face(current_def.card.linked_face_layout) {
            return None;
        }

        (current_def.card.id.0 > other_def.card.id.0).then_some(other_def)
    }

    pub fn move_object_by_effect(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        self.move_object(old_id, new_zone, crate::events::cause::EventCause::effect())
    }

    pub fn move_object_by_game_rule(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
    ) -> Option<ObjectId> {
        self.move_object(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_game_rule(),
        )
    }

    /// Put an object its owner owns into the ante zone (CR 407.4).
    ///
    /// Ante-specific card effects should use this entrypoint so the
    /// owner-only restriction is enforced centrally.
    pub fn ante_owned_object(
        &mut self,
        owner: PlayerId,
        object_id: ObjectId,
    ) -> Result<ObjectId, String> {
        let Some(object) = self.object(object_id) else {
            return Err("cannot ante a missing object".to_string());
        };
        if object.owner != owner {
            return Err("a player can ante only an object they own".to_string());
        }
        if object.zone == Zone::Ante {
            return Ok(object_id);
        }
        self.move_object_by_game_rule(object_id, Zone::Ante)
            .ok_or_else(|| "the object could not be moved to ante".to_string())
    }

    /// Select a random card from a player's library and ante it (CR 407.2).
    pub fn ante_random_library_card(&mut self, owner: PlayerId) -> Result<ObjectId, String> {
        let mut candidates = self
            .player(owner)
            .ok_or_else(|| "cannot ante for a missing player".to_string())?
            .library
            .to_vec();
        if candidates.is_empty() {
            return Err("cannot ante from an empty library".to_string());
        }
        self.shuffle_slice(&mut candidates);
        self.ante_owned_object(owner, candidates[0])
    }

    /// Transfer ownership of every card in ante to the winning player at the
    /// end of the game (CR 407.2). Returns the number of changed owners and is
    /// deliberately idempotent so duplicate terminal-result observations are
    /// harmless.
    pub fn finalize_ante_ownership(&mut self, winner: PlayerId) -> usize {
        if self.player(winner).is_none() {
            return 0;
        }
        let ante_ids = self.ante.clone();
        let mut changed = 0;
        for id in ante_ids {
            if self.object(id).is_some_and(|object| object.owner != winner) {
                if let Some(object) = self.object_mut(id) {
                    object.owner = winner;
                }
                changed += 1;
            }
        }
        changed
    }

    pub fn move_object_by_sba(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        self.move_object(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_sba(),
        )
    }

    pub(crate) fn move_object_by_sba_with_snapshot(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Option<ObjectId> {
        self.move_object_with_snapshot(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_sba(),
            snapshot,
        )
    }

    /// Move an object to the battlefield with ETB replacement effect processing.
    ///
    /// This processes replacement effects that modify how a permanent enters the battlefield:
    /// - "Enters tapped" effects (from the permanent itself or other sources)
    /// - "Enters with N counters" effects
    /// - "If this would enter the battlefield, exile it instead"
    ///
    /// For moves TO the battlefield, this should be used instead of `move_object`
    /// to ensure replacement effects are properly applied.
    pub fn move_object_with_etb_processing(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        self.move_object_with_etb_processing_with_dm(old_id, new_zone, &mut dm)
    }

    /// Move an object to the battlefield with ETB replacement processing and decisions.
    pub fn move_object_with_etb_processing_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
        )
    }

    /// Move an object with an authored tapped instruction in the original entry
    /// event, before replacement effects modify it.
    pub(crate) fn move_object_with_etb_processing_with_entry_options(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        initial_enters_tapped: bool,
        choose_aura_attachment: bool,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_cause_and_entry_options(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            initial_enters_tapped,
            choose_aura_attachment,
        )
    }

    pub(crate) fn move_object_with_etb_processing_with_cause_and_entry_options(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        initial_enters_tapped: bool,
        choose_aura_attachment: bool,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            cause,
            decision_maker,
            choose_aura_attachment,
            Vec::new(),
            None,
            initial_enters_tapped,
            None,
        )
    }

    pub(crate) fn move_object_with_etb_processing_with_cause_and_entry_options_and_controller(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        entering_controller: Option<PlayerId>,
        initial_enters_tapped: bool,
        choose_aura_attachment: bool,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            cause,
            decision_maker,
            choose_aura_attachment,
            Vec::new(),
            entering_controller,
            initial_enters_tapped,
            None,
        )
    }

    /// Move an object to the battlefield with ETB replacement processing and an explicit cause.
    pub fn move_object_with_etb_processing_with_dm_and_cause(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            cause,
            decision_maker,
            true,
            Vec::new(),
            None,
            false,
            None,
        )
    }

    pub fn move_object_with_etb_processing_with_initial_counters_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            true,
            initial_enters_with_counters,
            None,
            false,
            None,
        )
    }

    pub(crate) fn move_object_with_etb_processing_with_initial_counters_and_controller_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            true,
            initial_enters_with_counters,
            entering_controller,
            false,
            None,
        )
    }

    pub fn move_object_with_etb_processing_without_aura_attachment_choice(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            false,
            Vec::new(),
            None,
            false,
            None,
        )
    }

    /// Resolve every choice and action that forms part of an object's entry
    /// before the destination object is created.
    ///
    /// The returned record is independent of the source-zone object ID and can
    /// therefore be collected for every member of a simultaneous-entry batch
    /// before any member is committed.
    pub(crate) fn prepare_etb_entry_with_controller_and_dm(
        &mut self,
        old_id: ObjectId,
        result: crate::events::processing::EtbEventResult,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<Option<PreparedEtbEntry>, crate::effects::ExecutionError> {
        self.prepare_etb_entry_after_programs(
            old_id,
            result,
            entering_controller,
            decision_maker,
            None,
        )
    }

    pub(crate) fn prepare_etb_entry_after_programs(
        &mut self,
        old_id: ObjectId,
        result: crate::events::processing::EtbEventResult,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        completed_programs: Option<(PreparedEtbChoices, Vec<crate::ability::Ability>)>,
    ) -> Result<Option<PreparedEtbEntry>, crate::effects::ExecutionError> {
        let checkpoint = self.clone();
        let outcome = self.prepare_etb_entry_after_programs_inner(
            old_id, result, entering_controller, decision_maker, completed_programs,
        );
        if outcome.is_err() || decision_maker.awaiting_choice() {
            *self = checkpoint;
            return outcome.map(|_| None);
        }
        outcome
    }

    fn prepare_etb_entry_after_programs_inner(
        &mut self,
        old_id: ObjectId,
        mut result: crate::events::processing::EtbEventResult,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        completed_programs: Option<(PreparedEtbChoices, Vec<crate::ability::Ability>)>,
    ) -> Result<Option<PreparedEtbEntry>, crate::effects::ExecutionError> {
        if result.prevented {
            return Ok(Some(PreparedEtbEntry {
                result,
                choices: PreparedEtbChoices::default(),
                zone_entry_lookback: None,
                entry_definition: None,
                physical_components: Vec::new(),
                linked_face_mana_cost: None,
                entry_attachment: None, entry_attachment_requires_aura: false,
            }));
        }
        if let Some(choices) = result.prepared_choices.clone() {
            return Ok(Some(PreparedEtbEntry { result, choices, zone_entry_lookback: None, entry_definition: None, physical_components: Vec::new(), linked_face_mana_cost: None, entry_attachment: None, entry_attachment_requires_aura: false }));
        }

        let prospective_source = result
            .enters_as_copy_of
            .and_then(|copy_source| self.object(copy_source))
            .or_else(|| self.object(old_id));
        let prospective_controller = result
            .controller_override
            .or(entering_controller)
            .or_else(|| self.current_controller(old_id))
            .or_else(|| self.object(old_id).map(|object| object.owner))
            .ok_or(crate::effects::ExecutionError::InvalidTarget)?;
        let mut prospective_card_types = prospective_source
            .map(|object| object.card_types.clone())
            .unwrap_or_default();
        for card_type in &result.added_card_types {
            if !prospective_card_types.contains(card_type) {
                prospective_card_types.push(*card_type);
            }
        }
        if result.removes_other_card_types {
            prospective_card_types.retain(|card_type| result.added_card_types.contains(card_type));
        }
        let mut prospective_subtypes = prospective_source
            .map(|object| object.subtypes.clone())
            .unwrap_or_default();
        for subtype in &result.added_subtypes {
            if !prospective_subtypes.contains(subtype) {
                prospective_subtypes.push(*subtype);
            }
        }
        let mut prospective_abilities = prospective_source
            .map(|object| object.abilities_vec())
            .unwrap_or_default();
        for ability in &result.added_abilities {
            if !prospective_abilities.contains(ability) {
                prospective_abilities.push(ability.clone());
            }
        }

        let mut program_choices = if let Some((choices, abilities)) = completed_programs {
            prospective_abilities = abilities;
            choices
        } else {
            let programs = prospective_abilities
                .iter()
                .filter_map(|ability| {
                    let (program, _, face_up_only) =
                        as_enters_effect_program_from_ability(ability)?;
                    (!face_up_only).then_some(program)
                })
                .collect();
            let Some(choices) = self.execute_entry_programs(
                old_id,
                prospective_controller,
                programs,
                None,
                decision_maker,
            )? else {
                return Ok(None);
            };
            if !choices.as_enters_continuous_effects.is_empty() {
                if let Some(abilities) = self.entry_text_abilities(old_id) {
                    prospective_abilities = abilities;
                }
            }
            choices
        };

        let battle_protector = if prospective_card_types.contains(&crate::types::CardType::Battle) {
            let legal = self.legal_battle_protectors_for(
                prospective_controller,
                prospective_subtypes.contains(&Subtype::Siege),
            );
            if legal.len() <= 1 {
                legal.first().copied()
            } else {
                let options = legal
                    .iter()
                    .enumerate()
                    .map(|(index, player)| {
                        crate::decisions::context::SelectableOption::new(
                            index,
                            self.player(*player)
                                .map(|player| player.name.clone())
                                .unwrap_or_else(|| format!("Player {}", player.0)),
                        )
                    })
                    .collect();
                let context = crate::decisions::context::SelectOptionsContext::new(
                    prospective_controller,
                    Some(old_id),
                    "Choose a player to protect this battle",
                    options,
                    1,
                    1,
                );
                let selected = decision_maker
                    .decide_options(self, &context)
                    .into_iter()
                    .find_map(|index| legal.get(index).copied());
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                selected.or_else(|| legal.first().copied())
            }
        } else {
            None
        };

        program_choices.battle_protector = battle_protector;
        let mut choices = program_choices;
        for ability in prospective_abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(spec) = static_ability.color_choice_as_enters() {
                let mut options = vec![
                    crate::color::Color::White,
                    crate::color::Color::Blue,
                    crate::color::Color::Black,
                    crate::color::Color::Red,
                    crate::color::Color::Green,
                ];
                if let Some(excluded) = spec.excluded {
                    options.retain(|color| *color != excluded);
                }
                if !options.is_empty() {
                    let choice_spec = crate::decisions::specs::ManaColorsSpec::restricted(
                        old_id,
                        1,
                        true,
                        options.clone(),
                    );
                    let mut chosen = crate::decisions::make_decision(
                        self,
                        decision_maker,
                        prospective_controller,
                        Some(old_id),
                        choice_spec,
                    );
                    if decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    choices.chosen_color = chosen.pop().filter(|color| options.contains(color));
                }
            }
            if static_ability.basic_land_type_choice_as_enters().is_some() {
                let options = [
                    crate::types::Subtype::Plains,
                    crate::types::Subtype::Island,
                    crate::types::Subtype::Swamp,
                    crate::types::Subtype::Mountain,
                    crate::types::Subtype::Forest,
                ];
                let display_options = options
                    .iter()
                    .enumerate()
                    .map(|(idx, subtype)| {
                        crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                choices.chosen_basic_land_type = chosen
                    .pop()
                    .filter(|idx| *idx < options.len())
                    .map(|idx| options[idx]);
            }
            if static_ability.land_type_choice_as_enters().is_some() {
                let options = crate::types::Subtype::all_land_types();
                let display_options = options
                    .iter()
                    .enumerate()
                    .map(|(idx, subtype)| {
                        crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                choices.chosen_land_type = chosen
                    .pop()
                    .filter(|idx| *idx < options.len())
                    .map(|idx| options[idx]);
            }
            if static_ability.creature_type_choice_as_enters().is_some() {
                let options = crate::effects::BecomeCreatureTypeChoiceEffect::all_creature_types();
                let display_options = options
                    .iter()
                    .enumerate()
                    .map(|(idx, subtype)| {
                        crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                choices.chosen_creature_type = chosen
                    .pop()
                    .filter(|idx| *idx < options.len())
                    .map(|idx| options[idx]);
            }
            if let Some(spec) = static_ability.player_choice_as_enters() {
                let filter_ctx = self.filter_context_for(prospective_controller, Some(old_id));
                let options = self
                    .players
                    .iter()
                    .filter(|player| {
                        player.is_in_game() && spec.filter.matches_player(player.id, &filter_ctx)
                    })
                    .map(|player| player.id)
                    .collect::<Vec<_>>();
                if !options.is_empty() {
                    let display_options = options
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, player_id)| {
                            self.player(*player_id).map(|player| {
                                crate::decisions::spec::DisplayOption::new(
                                    idx,
                                    player.name.to_string(),
                                )
                            })
                        })
                        .collect::<Vec<_>>();
                    let choice_spec =
                        crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                    let mut chosen = crate::decisions::make_decision(
                        self,
                        decision_maker,
                        prospective_controller,
                        Some(old_id),
                        choice_spec,
                    );
                    if decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    choices.chosen_player = chosen
                        .pop()
                        .filter(|idx| *idx < options.len())
                        .map(|idx| options[idx]);
                }
            }
            if let Some(spec) = static_ability.reveal_from_hand_choice_as_enters() {
                choices.as_enters_tagged_objects.insert(
                    crate::tag::TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
                    Vec::new(),
                );
                let filter_ctx = self.filter_context_for(prospective_controller, Some(old_id));
                let hand: Vec<ObjectId> = self
                    .player(prospective_controller)
                    .map(|player| player.hand.to_vec())
                    .unwrap_or_default();
                // Only the owner knows which hidden hand cards match: peers
                // offer their placeholders too, every peer asks, and the
                // chosen cards are opened before the answer replays (see
                // `game_state::hidden_hand_choices`).
                let hidden_hand_choice = self
                    .hand_choice_depends_on_hidden_identity(&spec.filter, hand.iter().copied());
                let placeholders = if hidden_hand_choice {
                    self.hidden_hand_placeholder_candidates(
                        &spec.filter,
                        &filter_ctx,
                        hand.iter().copied(),
                    )
                } else {
                    Vec::new()
                };
                let candidates = hand
                    .into_iter()
                    .filter_map(|candidate_id| {
                        self.object(candidate_id)
                            .filter(|object| {
                                placeholders.contains(&candidate_id)
                                    || spec.filter.matches(object, &filter_ctx, self)
                            })
                            .map(|object| {
                                crate::decisions::context::SelectableObject::new(
                                    candidate_id,
                                    object.name.to_string(),
                                )
                            })
                    })
                    .collect::<Vec<_>>();
                if !candidates.is_empty() || hidden_hand_choice {
                    // A hidden choice may be answered short: the owner's real
                    // match count is unknown here.
                    let min = if spec.optional || hidden_hand_choice {
                        0
                    } else {
                        spec.count.min.min(candidates.len())
                    };
                    let max = spec
                        .count
                        .max
                        .unwrap_or(candidates.len())
                        .min(candidates.len());
                    let context = crate::decisions::context::SelectObjectsContext::new(
                        prospective_controller,
                        Some(old_id),
                        "Reveal cards from your hand",
                        candidates.clone(),
                        min,
                        Some(max),
                    );
                    let context = if hidden_hand_choice {
                        context.require_explicit_choice().with_reveal_policy(
                            crate::decisions::context::SelectionRevealPolicy::Public,
                        )
                    } else {
                        context
                    };
                    let candidate_ids = candidates
                        .iter()
                        .map(|candidate| candidate.id)
                        .collect::<Vec<_>>();
                    let selected = if !hidden_hand_choice
                        && min == candidates.len()
                        && max == candidates.len()
                    {
                        candidate_ids.clone()
                    } else {
                        decision_maker.decide_objects(self, &context)
                    };
                    if decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    let revealed = selected
                        .into_iter()
                        .filter(|selected| candidate_ids.contains(selected))
                        .take(max)
                        .collect::<Vec<_>>();
                    if hidden_hand_choice {
                        self.record_hidden_identity_obligations(
                            &revealed,
                            &spec.filter,
                            &filter_ctx,
                            "reveal cards matching the filter",
                        );
                        self.mark_hidden_cards_publicly_revealed(&revealed);
                    }
                    if revealed.len() >= min {
                        let snapshots = revealed
                            .iter()
                            .filter_map(|id| self.object(*id))
                            .map(|object| {
                                crate::snapshot::ObjectSnapshot::from_object(object, self)
                            })
                            .collect();
                        choices.as_enters_tagged_objects.insert(
                            crate::tag::TagKey::from(crate::effects::PUBLIC_REVEALED_TAG),
                            snapshots,
                        );
                    }
                    if revealed.len() >= min && !revealed.is_empty() {
                        for viewer_idx in 0..self.players.len() {
                            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                            let view_ctx = crate::decisions::context::ViewCardsContext::new(
                                viewer,
                                prospective_controller,
                                Some(old_id),
                                Zone::Hand,
                                "Reveal cards from hand",
                            )
                            .with_public(true);
                            decision_maker.view_cards(self, viewer, &revealed, &view_ctx);
                        }
                    }
                }
            }
            if let Some(spec) = static_ability.card_name_choice_as_enters() {
                if spec.reveal_opponents_hands {
                    let opponent_ids = self
                        .players
                        .iter()
                        .filter(|player| player.is_in_game() && player.id != prospective_controller)
                        .map(|player| player.id)
                        .collect::<Vec<_>>();
                    for opponent_id in opponent_ids {
                        let cards = self
                            .player(opponent_id)
                            .map(|player| player.hand.clone())
                            .unwrap_or_default();
                        for viewer_idx in 0..self.players.len() {
                            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                            let mut view_ctx =
                                crate::decisions::context::ViewCardsContext::look_at_hand(
                                    viewer,
                                    opponent_id,
                                    Some(old_id),
                                );
                            view_ctx.description = "Reveal that player's hand".to_string();
                            view_ctx.public = true;
                            decision_maker.view_cards(self, viewer, &cards, &view_ctx);
                        }
                    }
                }
                let choice_ctx = crate::decisions::context::TextInputContext::new(
                    prospective_controller,
                    Some(old_id),
                    "Choose a card name",
                )
                .with_placeholder("Enter a card name")
                .require_known_value(true);
                let chosen_name = decision_maker.decide_text(self, &choice_ctx);
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                let chosen_name = chosen_name.trim();
                if !chosen_name.is_empty() {
                    let mut registry = CardRegistry::new();
                    registry.ensure_cards_loaded([chosen_name]);
                    let canonical_name = registry
                        .get(chosen_name)
                        .map(|definition| definition.name().to_string())
                        .unwrap_or_else(|| chosen_name.to_string());
                    let legal = !spec.require_nonland_from_revealed_opponents
                        || self
                            .players
                            .iter()
                            .filter(|player| {
                                player.is_in_game() && player.id != prospective_controller
                            })
                            .flat_map(|player| player.hand.iter().copied())
                            .filter_map(|object_id| self.object(object_id))
                            .any(|object| {
                                !object.is_land()
                                    && object.name.eq_ignore_ascii_case(&canonical_name)
                            });
                    if legal {
                        choices.chosen_named_option = Some(canonical_name);
                    }
                }
            }
            if let Some(spec) = static_ability.named_option_choice_as_enters()
                && spec.at_random
                && !spec.options.is_empty()
            {
                // "As this enters, choose 2, 3, or 4 at random": no player
                // makes this choice; the game's deterministic RNG picks one
                // listed option uniformly.
                let mut indices = (0..spec.options.len()).collect::<Vec<_>>();
                self.shuffle_slice(&mut indices);
                if let Some(option) = indices.first().map(|idx| spec.options[*idx].clone()) {
                    choices.chosen_named_option = Some(option);
                }
            } else if let Some(spec) = static_ability.named_option_choice_as_enters()
                && !spec.options.is_empty()
            {
                let display_options = spec
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, option)| {
                        crate::decisions::spec::DisplayOption::new(idx, option.clone())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                if let Some(option) = chosen
                    .pop()
                    .filter(|idx| *idx < spec.options.len())
                    .map(|idx| spec.options[idx].clone())
                {
                    choices.chosen_card_type = match option.as_str() {
                        "artifact" => Some(crate::types::CardType::Artifact),
                        "creature" => Some(crate::types::CardType::Creature),
                        "enchantment" => Some(crate::types::CardType::Enchantment),
                        "instant" => Some(crate::types::CardType::Instant),
                        "sorcery" => Some(crate::types::CardType::Sorcery),
                        "planeswalker" => Some(crate::types::CardType::Planeswalker),
                        "land" => Some(crate::types::CardType::Land),
                        _ => None,
                    };
                    if let Some((color, subtype)) = named_color_creature_type_option(&option) {
                        choices.chosen_color = Some(color);
                        choices.chosen_creature_type = Some(subtype);
                    } else if let Some(subtype) = named_subtype_option(&option) {
                        // "choose Elemental, Elf, ... or Treefolk" / "choose
                        // Island or Swamp": the option is itself the chosen
                        // type that "of the chosen type" filters read.
                        choices.chosen_creature_type = Some(subtype);
                    }
                    choices.chosen_named_option = Some(option);
                }
            }
            if static_ability.life_total_note_as_enters().is_some() {
                choices.noted_life_total = self
                    .player(prospective_controller)
                    .map(|player| player.life);
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::DiscardHandAsEnters
            {
                choices.discard_hand = true;
            }
            if let Some(spec) = static_ability.power_toughness_choice_as_enters_or_turns_face_up()
                && !spec.options.is_empty()
            {
                let display_options = spec
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, option)| {
                        crate::decisions::spec::DisplayOption::new(
                            idx,
                            format!("{}/{}", option.power, option.toughness),
                        )
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                if let Some(option) = chosen
                    .pop()
                    .filter(|idx| *idx < spec.options.len())
                    .map(|idx| spec.options[idx].clone())
                {
                    choices.power_toughness_choices.push((
                        option.power,
                        option.toughness,
                        option.abilities,
                    ));
                }
            }
        }

        result.prepared_choices = Some(choices.clone());
        Ok(Some(PreparedEtbEntry { result, choices, zone_entry_lookback: None, entry_definition: None, physical_components: Vec::new(), linked_face_mana_cost: None, entry_attachment: None, entry_attachment_requires_aura: false }))
    }

    /// Commit an ETB proposal whose replacement choices were already resolved.
    ///
    /// Batch-entry callers use this after every entrant's immutable proposal has
    /// been collected, so no entrant can make another entrant's replacement
    /// effects visible before the simultaneous event is committed.
    pub(crate) fn commit_prepared_etb_with_controller_and_dm(
        &mut self,
        old_id: ObjectId,
        prepared_entry: PreparedEtbEntry,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            true,
            Vec::new(),
            entering_controller,
            false,
            Some(prepared_entry),
        )
    }

    pub(crate) fn commit_prepared_etb_with_cause_and_options_and_dm(
        &mut self,
        old_id: ObjectId,
        prepared_entry: PreparedEtbEntry,
        entering_controller: Option<PlayerId>,
        cause: crate::events::cause::EventCause,
        choose_aura_attachment: bool,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            Zone::Battlefield,
            cause,
            decision_maker,
            choose_aura_attachment,
            Vec::new(),
            entering_controller,
            false,
            Some(prepared_entry),
        )
    }

    /// Commit a prepared entry whose attachment the effect fixes ("onto the
    /// battlefield attached to X"). The caller attaches it; an Aura must not
    /// choose something else to enchant (CR 303.4f).
    pub(crate) fn commit_prepared_etb_with_fixed_attachment_and_dm(
        &mut self,
        old_id: ObjectId,
        prepared_entry: PreparedEtbEntry,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            false,
            Vec::new(),
            entering_controller,
            false,
            Some(prepared_entry),
        )
    }

    /// Commit a prevalidated exchange entrant with a fixed attachment target.
    /// Its caller stages the entire exchange and supplies the attachment; do
    /// not choose a different target or recheck entry prohibitions after the
    /// other member of the simultaneous exchange has already departed.
    pub(crate) fn commit_prepared_exchange_etb_with_dm(
        &mut self,
        old_id: ObjectId,
        prepared_entry: PreparedEtbEntry,
        entering_controller: PlayerId,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        self.move_object_with_etb_processing_with_dm_and_cause_body(
            old_id,
            Zone::Battlefield,
            cause,
            decision_maker,
            false,
            Vec::new(),
            Some(entering_controller),
            false,
            Some(prepared_entry),
            true,
        )
    }

    fn move_object_with_etb_processing_with_dm_and_cause_internal(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        choose_aura_attachment: bool,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        initial_enters_tapped: bool,
        prepared_entry: Option<PreparedEtbEntry>,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        if new_zone == Zone::Battlefield && self.card_cannot_enter_battlefield(old_id) {
            let programs = prepared_entry.map(|mut entry| std::mem::take(&mut entry.result.additional_programs)).unwrap_or_default();
            return Ok(super::EntryCommitResult {
                original: crate::events::processing::EventOutcome::NotApplicable, programs, pending: false,
            });
        }
        if new_zone == Zone::Battlefield {
            let mut working = self.clone();
            let outcome = working.move_object_with_etb_processing_with_dm_and_cause_body(
                old_id,
                new_zone,
                cause,
                decision_maker,
                choose_aura_attachment,
                initial_enters_with_counters,
                entering_controller,
                initial_enters_tapped,
                prepared_entry,
                false,
            );
            if outcome.is_err() {
                return outcome;
            }
            if decision_maker.awaiting_choice() {
                return Ok(super::EntryCommitResult::pending());
            }
            // Validate the completed entry before publishing it. Query
            // preparation performs no day/night, ascend or other game rules
            // procedures while this operation's working state is isolated.
            working = working.continuous_query_snapshot()
                .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
            *self = working;
            return outcome;
        }

        self.move_object_with_etb_processing_with_dm_and_cause_body(
            old_id,
            new_zone,
            cause,
            decision_maker,
            choose_aura_attachment,
            initial_enters_with_counters,
            entering_controller,
            initial_enters_tapped,
            prepared_entry,
            false,
        )
    }

    fn move_object_with_etb_processing_with_dm_and_cause_body(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        choose_aura_attachment: bool,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        initial_enters_tapped: bool,
        prepared_entry: Option<PreparedEtbEntry>,
        entry_prevalidated: bool,
    ) -> Result<super::EntryCommitResult, crate::effects::ExecutionError> {
        use crate::events::processing::EventOutcome;
        let checkpoint = self.clone();
        let mut programs = Vec::new();
        let mut original_verdict = EventOutcome::NotApplicable;
        let result = self.move_object_with_etb_processing_with_dm_and_cause_body_inner(
            old_id, new_zone, cause, decision_maker, choose_aura_attachment,
            initial_enters_with_counters, entering_controller, initial_enters_tapped,
            prepared_entry, entry_prevalidated, &mut programs, &mut original_verdict,
        );
        if result.is_err() || decision_maker.awaiting_choice() { *self = checkpoint; }
        let entered = result?;
        if decision_maker.awaiting_choice() { return Ok(super::EntryCommitResult::pending()); }
        let original = match entered {
            Some(entered) => EventOutcome::Proceed(entered),
            None => match original_verdict {
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
                EventOutcome::Prevented | EventOutcome::Proceed(()) => EventOutcome::Prevented,
            },
        };
        Ok(super::EntryCommitResult { original, programs, pending: false })
    }

    fn move_object_with_etb_processing_with_dm_and_cause_body_inner(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        choose_aura_attachment: bool,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        initial_enters_tapped: bool,
        mut prepared_entry: Option<PreparedEtbEntry>,
        entry_prevalidated: bool,
        programs: &mut Vec<crate::events::processing::PreparedReplacementProgram>,
        original_verdict: &mut crate::events::processing::EventOutcome<()>,
    ) -> Result<Option<EntersResult>, crate::effects::ExecutionError> {
        use crate::events::processing::EventOutcome;
        if let Some(entry) = &mut prepared_entry {
            programs.append(&mut entry.result.additional_programs);
            *original_verdict = if entry.result.replaced { EventOutcome::Replaced } else { EventOutcome::Prevented };
        }
        // Preparation may have completed an Instead program which moved the
        // original source. There is no source event left to validate or commit.
        if prepared_entry.as_ref().is_some_and(|entry|
            entry.result.prevented && entry.result.new_destination.is_none())
        {
            return Ok(None);
        }
        let old_zone = self.object(old_id).ok_or(crate::effects::ExecutionError::ObjectNotFound(old_id))?.zone;

        if old_zone == new_zone && !matches!(new_zone, Zone::Exile | Zone::Command) {
            return Ok(None);
        }
        *original_verdict = EventOutcome::Prevented;
        // Only process ETB replacement for moves TO the battlefield
        if new_zone != Zone::Battlefield {
            let Some(new_id) = self.move_object(old_id, new_zone, cause.clone()) else { return Ok(None); };
            return Ok(Some(EntersResult {
                new_id,
                enters_tapped: false,
            }));
        }

        // Process through ETB replacement effects
        let prepared_entry = if let Some(prepared_entry) = prepared_entry {
            prepared_entry
        } else {
            let mut result = crate::events::processing::process_etb_with_event_and_dm_with_initial_counters_and_controller(
                self,
                old_id,
                old_zone,
                decision_maker,
                initial_enters_with_counters,
                entering_controller,
                initial_enters_tapped,
                cause.clone(),
            )?;
            programs.append(&mut result.additional_programs);
            if result.replaced { *original_verdict = EventOutcome::Replaced; }
            match self.prepare_etb_entry_with_controller_and_dm(
                old_id,
                result,
                entering_controller,
                decision_maker,
            )? {
                Some(prepared) => prepared,
                None => return Ok(None),
            }
        };
        let PreparedEtbEntry { result, choices, zone_entry_lookback, entry_definition, physical_components, linked_face_mana_cost, entry_attachment, entry_attachment_requires_aura } = prepared_entry;
        // A completed Instead payload leaves no entry proposal to commit.
        // Its legitimate movement can invalidate old source IDs without being
        // an execution error; validation remains mandatory for actual commits.
        if result.prevented && result.new_destination.is_none() {
            return Ok(None);
        }
        for component in &physical_components {
            if !self.object(component.snapshot.object_id).is_some_and(|object|
                object.zone == component.snapshot.zone && object.stable_id == component.snapshot.stable_id)
            {
                return Err(crate::effects::ExecutionError::InternalError(
                    "composite entry source changed before commit".into()));
            }
        }
        let original_zone_snapshot = result.replacement_context.as_ref()
            .and_then(|context| context.zone_change_context.as_ref())
            .and_then(|zone| zone.snapshot.clone());

        // If ETB was prevented or redirected to a different zone
        if result.prevented {
            if let Some(dest) = result.new_destination {
                // Move to the alternate destination
                let Some(new_id) = self.move_prepared_zone_entry_with_lookback(
                    old_id, dest, cause.clone(), original_zone_snapshot.clone(),
                    zone_entry_lookback.as_deref(), entry_prevalidated, entry_definition.as_ref(), &physical_components, linked_face_mana_cost.as_ref(),
                ) else { return Ok(None); };
                return Ok(Some(EntersResult {
                    new_id,
                    enters_tapped: false,
                }));
            }
            return Ok(None);
        }

        let prospective_aura_entry = choose_aura_attachment
            && (old_zone != Zone::Stack || result.enters_as_copy_of.is_some())
            && (self
                .object(old_id)
                .is_some_and(|object| object.subtypes.contains(&Subtype::Aura))
                || result.enters_as_copy_of.is_some_and(|copy_id| {
                    self.object(copy_id)
                        .is_some_and(|object| object.subtypes.contains(&Subtype::Aura))
                })
                || result.added_subtypes.contains(&Subtype::Aura));
        // CR 303.4g is not a second zone change. If attachment proves
        // impossible, restore this exact pre-entry state.
        let mut aura_entry_checkpoint = (prospective_aura_entry || entry_attachment.is_some()).then(|| self.clone());

        if choices.discard_hand {
            let controller = result
                .controller_override
                .or(entering_controller)
                .or_else(|| self.current_controller(old_id))
                .or_else(|| self.object(old_id).map(|object| object.owner))
                .ok_or(crate::effects::ExecutionError::ObjectNotFound(old_id))?;
            let hand = self
                .player(controller)
                .map(|player| player.hand.clone())
                .unwrap_or_default();
            // Hidden-information matches: the owner reveals the hand publicly
            // before any card moves, so every peer applies Madness (CR
            // 702.35a) and discard triggers to the same identities, as the
            // discard-hand effect does (see `hidden_hand_choices`). This body
            // runs on a working copy that is dropped while a decision is
            // awaited (the Madness choice below already relies on that), so
            // pausing here and re-running with the owner's answer is safe.
            // Never prompts outside hidden-information matches.
            let to_reveal: Vec<ObjectId> =
                hand.iter().copied().filter(|id| *id != old_id).collect();
            if to_reveal
                .iter()
                .any(|id| self.hidden_identity_is_private(*id))
                && self
                    .reveal_private_hidden_cards_publicly(
                        &mut *decision_maker,
                        controller,
                        old_id,
                        &to_reveal,
                        "Reveal the cards you discard",
                        false,
                    )
                    .is_none()
            {
                return Ok(None);
            }
            let cards = hand.into_iter().filter(|id| *id != old_id).collect();
            let provenance = self.provenance_graph_mut().alloc_root_event(crate::events::EventKind::Discard);
            let snapshot = self.object(old_id).map(|object|
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, self));
            let mut ctx = crate::effects::ExecutionContext::new(old_id, controller, decision_maker)
                .with_cause(cause.clone()).with_provenance(provenance);
            ctx.source_snapshot = snapshot;
            if let Some(context) = &result.replacement_context {
                context.apply_to(&mut ctx);
                ctx.provenance = provenance;
            }
            let mut outcome = crate::effects::cards::discard_hand_cards(self, &mut ctx, controller, cards)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(None); }
            crate::effects::retain_unmatched_outcome_events(self, &mut outcome.events);
            for event in outcome.events { self.queue_trigger_event(event.provenance(), event); }

        }

        // Preserve a simultaneous exchange's entry-legality result through
        // the final zone mutation, not just the outer ETB preparation layer.
        let Some(new_id) = self.move_prepared_zone_entry_with_lookback(
            old_id, Zone::Battlefield, cause.clone(), original_zone_snapshot,
            zone_entry_lookback.as_deref(), entry_prevalidated, entry_definition.as_ref(), &physical_components, linked_face_mana_cost.as_ref(),
        ) else { return Ok(None); };
        if let Some(object) = self.object_mut(new_id) {
            merge_retained_tagged_objects(
                &mut object.cast_tagged_objects,
                &choices.as_enters_tagged_objects,
            );
        }
        self.effect_store.continuous_effects.retarget_entry_effects(
            &choices.as_enters_continuous_effects,
            old_id,
            new_id,
        );
        // As-enters effect programs execute against the pre-move object id;
        // migrate any choices they recorded to the battlefield id.
        if new_id != old_id {
            let choice_store = self.choice_store_mut();
            if let Some(color) = choice_store.chosen_colors.remove(&old_id) {
                choice_store.chosen_colors.insert(new_id, color);
            }
            if let Some(land_type) = choice_store.chosen_land_types.remove(&old_id) {
                choice_store.chosen_land_types.insert(new_id, land_type);
            }
            if let Some(creature_type) = choice_store.chosen_creature_types.remove(&old_id) {
                choice_store
                    .chosen_creature_types
                    .insert(new_id, creature_type);
            }
            if let Some(creature_types) = choice_store.chosen_creature_type_sets.remove(&old_id) {
                choice_store
                    .chosen_creature_type_sets
                    .insert(new_id, creature_types);
            }
            if let Some(card_type) = choice_store.chosen_card_types.remove(&old_id) {
                choice_store.chosen_card_types.insert(new_id, card_type);
            }
            if let Some(player) = choice_store.chosen_players.remove(&old_id) {
                choice_store.chosen_players.insert(new_id, player);
            }
            if let Some(object) = choice_store.chosen_objects.remove(&old_id) {
                choice_store.chosen_objects.insert(new_id, object);
            }
            if let Some(names) = choice_store.chosen_named_options.remove(&old_id) {
                choice_store.chosen_named_options.insert(new_id, names);
            }
            // Devour runs as the permanent enters (CR 702.82a) and records the
            // devoured count on the pre-move id.
            let devoured = self.devoured_count(old_id);
            if devoured > 0 {
                self.set_devoured_count(old_id, 0);
                self.set_devoured_count(new_id, devoured);
            }
            let devoured_objects = self.devoured_objects(old_id).to_vec();
            if !devoured_objects.is_empty() {
                self.set_devoured_objects(old_id, Vec::new());
                self.set_devoured_objects(new_id, devoured_objects);
            }
        }
        if choices.transfer_as_enters_source_links {
            self.transfer_exiled_with_source_links(old_id, new_id);
            let imprinted_cards = self.get_imprinted_cards(old_id).to_vec();
            self.clear_imprinted_cards(old_id);
            for imprinted_card in imprinted_cards {
                self.imprint_card(new_id, imprinted_card);
            }
        }
        if let Some(controller) = result.controller_override.or(entering_controller) {
            self.stage_controller_change_for_assembly(new_id, controller);
        }

        // Apply "enters as copy" before tapped/counter modifications. Ordinary
        // enter-as-copy effects replace the object's copiable values. A copy
        // with an explicit duration instead becomes a locked layer-1 effect,
        // preserving the underlying permanent so it can revert when it expires.
        let temporary_copy_duration = result.copy_duration.clone();
        // These entry modifications overwrite the permanent's own copiable
        // fields. They belong to this permanent only, so remember the printed
        // values and restore them when it leaves the battlefield (CR 707.2,
        // 400.7): a Clone that copied Grave Titan is just Clone again in its
        // new zone.
        if temporary_copy_duration.is_none()
            && (result.enters_as_copy_of.is_some()
                || !result.added_colors.is_empty()
                || !result.added_card_types.is_empty()
                || !result.added_supertypes.is_empty()
                || !result.removed_supertypes.is_empty()
                || !result.added_subtypes.is_empty()
                || !result.added_abilities.is_empty()
                || result.set_base_power_toughness.is_some())
            && let Some(new_obj) = self.object_mut(new_id)
        {
            new_obj.capture_enters_as_copy_restore_state();
        }
        if let Some(copy_source_id) = result.enters_as_copy_of {
            if let Some(duration) = temporary_copy_duration.clone() {
                let effects = self.all_continuous_effects();
                let copiable_values = crate::continuous::copiable_values_with_effects(
                    copy_source_id,
                    self.objects_map(),
                    &effects,
                    &self.battlefield,
                    self.commander_objects(),
                    self,
                );
                if let Some(mut copiable_values) = copiable_values {
                    let controller = self.current_controller(new_id).ok_or(crate::effects::ExecutionError::ObjectNotFound(new_id))?;
                    if let Some(name) = &result.copy_name_override {
                        copiable_values.name = name.clone();
                    }
                    copiable_values.colors = copiable_values.colors.union(result.added_colors);
                    for card_type in &result.added_card_types {
                        if !copiable_values.card_types.contains(card_type) {
                            copiable_values.card_types.push(*card_type);
                        }
                    }
                    if result.removes_other_card_types {
                        copiable_values
                            .card_types
                            .retain(|card_type| result.added_card_types.contains(card_type));
                    }
                    copiable_values
                        .supertypes
                        .retain(|supertype| !result.removed_supertypes.contains(supertype));
                    for supertype in &result.added_supertypes {
                        if !copiable_values.supertypes.contains(supertype) {
                            copiable_values.supertypes.push(*supertype);
                        }
                    }
                    for subtype in &result.added_subtypes {
                        if !copiable_values.subtypes.contains(subtype) {
                            copiable_values.subtypes.push(*subtype);
                        }
                    }
                    for ability in &result.added_abilities {
                        let abilities = std::sync::Arc::make_mut(&mut copiable_values.abilities);
                        if !abilities.contains(ability) {
                            abilities.push(ability.clone());
                        }
                    }
                    if let Some((power, toughness)) = result.set_base_power_toughness {
                        copiable_values.power = Some(power);
                        copiable_values.toughness = Some(toughness);
                    }

                    let modification = crate::continuous::Modification::CopyOf {
                        target_id: copy_source_id,
                        copiable_values: Box::new(copiable_values),
                        preserve_source_abilities: false,
                        name_override: None,
                        name_override_surface: None,
                        add_supertypes: Vec::new(),
                    };
                    let expires_end_of_turn = matches!(
                        &duration,
                        crate::effect::Until::EndOfTurn
                            | crate::effect::Until::YourNextTurn
                            | crate::effect::Until::YourNextUpkeep
                            | crate::effect::Until::ControllersNextUntapStep
                    )
                    .then_some(self.turn.turn_number)
                    .unwrap_or(u32::MAX);
                    let effect = crate::continuous::ContinuousEffect::new(
                        new_id,
                        controller,
                        crate::continuous::EffectTarget::Specific(new_id),
                        modification,
                    )
                    .until(duration)
                    .with_expires_end_of_turn(expires_end_of_turn)
                    .with_source_type(
                        crate::continuous::EffectSourceType::Resolution {
                            locked_targets: vec![new_id],
                        },
                    );
                    // Stage the copy with the remaining entry fields. Its
                    // counters and prepared choices are not assembled yet.
                    self.effect_store.continuous_effects.add_effect(effect);
                }
            } else {
                let copy_source = self.object(copy_source_id).cloned();
                let effects = self.all_continuous_effects();
                let copiable_values = crate::continuous::copiable_values_with_effects(
                    copy_source_id,
                    self.objects_map(),
                    &effects,
                    &self.battlefield,
                    self.commander_objects(),
                    self,
                );
                if let (Some(source_obj), Some(new_obj)) = (copy_source, self.object_mut(new_id)) {
                    new_obj.copy_copiable_values_from(&source_obj);
                    if let Some(values) = copiable_values.as_ref() {
                        new_obj.copy_copiable_values_from_values(values);
                    }
                    if let Some(name) = &result.copy_name_override {
                        new_obj.name = name.clone().into();
                    }
                }
            }
        }
        if temporary_copy_duration.is_none() {
            if !result.added_colors.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                new_obj.color_override = Some(new_obj.colors().union(result.added_colors));
            }
            if !result.added_card_types.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for card_type in &result.added_card_types {
                    if !new_obj.card_types.contains(card_type) {
                        new_obj.card_types.push(*card_type);
                    }
                }
                if result.removes_other_card_types {
                    new_obj
                        .card_types
                        .retain(|card_type| result.added_card_types.contains(card_type));
                }
            }
            if !result.added_supertypes.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for supertype in &result.added_supertypes {
                    if !new_obj.supertypes.contains(supertype) {
                        new_obj.supertypes.push(*supertype);
                    }
                }
            }
            if !result.removed_supertypes.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                new_obj
                    .supertypes
                    .retain(|supertype| !result.removed_supertypes.contains(supertype));
            }
            if !result.added_subtypes.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for subtype in &result.added_subtypes {
                    if !new_obj.subtypes.contains(subtype) {
                        new_obj.subtypes.push(*subtype);
                    }
                }
            }
            if !result.added_abilities.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for ability in &result.added_abilities {
                    if !new_obj.abilities.contains(ability) {
                        new_obj.abilities_mut().push(ability.clone());
                    }
                }
            }
            if let Some((power, toughness)) = result.set_base_power_toughness
                && let Some(new_obj) = self.object_mut(new_id)
            {
                new_obj.base_power = Some(crate::card::PtValue::Fixed(power));
                new_obj.base_toughness = Some(crate::card::PtValue::Fixed(toughness));
            }
        }

        // Publish the choices collected against the prospective permanent only
        // after the destination object exists. No decision is made in this
        // section, so neither synchronous nor suspended callers can observe a
        // battlefield permanent whose mandatory entry choice is unresolved.
        if let Some(color) = choices.chosen_color {
            self.set_chosen_color(new_id, color);
        }
        if let Some(subtype) = choices.chosen_basic_land_type {
            self.set_chosen_basic_land_type(new_id, subtype);
        }
        if let Some(subtype) = choices.chosen_land_type {
            self.set_chosen_land_type(new_id, subtype);
        }
        if let Some(subtype) = choices.chosen_creature_type {
            self.set_chosen_creature_type(new_id, subtype);
        }
        if let Some(card_type) = choices.chosen_card_type {
            self.set_chosen_card_type(new_id, card_type);
        }
        if let Some(player) = choices.chosen_player {
            self.set_chosen_player(new_id, player);
        }
        if let Some(option) = choices.chosen_named_option.clone() {
            self.set_chosen_named_option(new_id, option);
        }
        if let Some(life_total) = choices.noted_life_total {
            self.object_annotations_mut()
                .noted_life_totals
                .insert(new_id, life_total);
        }
        for (power, toughness, abilities) in &choices.power_toughness_choices {
            if let Some(object) = self.object_mut(new_id) {
                object.base_power = Some(crate::card::PtValue::Fixed(*power));
                object.base_toughness = Some(crate::card::PtValue::Fixed(*toughness));
                for granted in abilities {
                    let ability = crate::ability::Ability::static_ability(granted.clone());
                    if !object.abilities.contains(&ability) {
                        object.abilities_mut().push(ability);
                    }
                }
                self.mark_continuous_state_dirty();
            }
        }

        // Apply enters tapped
        if result.enters_tapped {
            self.tap(new_id);
        }

        // Apply enters with counters. Counters an object is given as it
        // enters are "put" on it (CR 122.6): each kind gets a counter
        // timestamp (613.7c) and a counters-put event, which "whenever
        // counters are put on" triggers and turn history see. The object's
        // controller puts them (122.6a).
        let mut entry_counters: Vec<(crate::object::CounterType, u32)> = Vec::new();
        for (counter_type, count) in result
            .enters_with_counters
            .iter()
            .chain(&choices.as_enters_counters)
        {
            if let Some(obj) = self.object_mut(new_id) {
                obj.add_counters(*counter_type, *count);
            }
            if *count == 0 {
                continue;
            }
            match entry_counters
                .iter_mut()
                .find(|(existing, _)| existing == counter_type)
            {
                Some((_, total)) => *total = total.saturating_add(*count),
                None => entry_counters.push((*counter_type, *count)),
            }
        }
        if !entry_counters.is_empty() {
            self.mark_continuous_state_dirty();
            let entering_controller = self.object(new_id).map(|object| self.controller_of(object));
            for (counter_type, count) in entry_counters {
                self.effect_store
                    .continuous_effects
                    .record_counter_change(new_id, counter_type);
                let count_after = self.counter_count(new_id, counter_type);
                let event_provenance = self
                    .provenance_graph_mut()
                    .alloc_root_event(crate::events::EventKind::MarkersChanged);
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::MarkersChangedEvent::added(
                        counter_type,
                        new_id,
                        count,
                        Some(new_id),
                        entering_controller,
                    )
                    .with_count_after(count_after),
                    event_provenance,
                );
                self.queue_trigger_event(event_provenance, event);
            }
        }

        if let Some(protector) = choices.battle_protector {
            let _ = self.set_battle_protector(new_id, protector);
        }

        if !result.paid_labels.is_empty()
            && let Some(obj) = self.object_mut(new_id)
        {
            for label in &result.paid_labels {
                obj.optional_costs_paid.mark_label_paid(label);
            }
        }

        for linked_old_id in &result.linked_exile_with_entering {
            if self.object(*linked_old_id).is_none() {
                continue;
            }
            let Some(exiled_id) = self.move_object(*linked_old_id, Zone::Exile, cause.clone())
            else {
                continue;
            };
            self.add_exiled_with_source_link(new_id, exiled_id);
            self.record_zone_change_results(*linked_old_id, vec![exiled_id]);
        }

        if let Some(copy_source_id) = result.enters_as_copy_of
            && !result.copy_followups.is_empty()
        {
            self.apply_enter_as_copy_followups(new_id, copy_source_id, &result.copy_followups);
        }

        // If this is an Aura entering from a non-stack zone, choose what to attach to
        if choose_aura_attachment
            && (old_zone != Zone::Stack || result.enters_as_copy_of.is_some())
            && let Some(obj) = self.object(new_id)
            && obj.subtypes.contains(&Subtype::Aura)
            && obj.attached_to.is_none()
            && let Some(filter) = obj.aura_attach_filter_owned()
        {
            let chooser = self.current_controller(new_id).unwrap_or(obj.owner);
            let filter_ctx = self.filter_context_for(chooser, Some(new_id));
            let chosen_target = match filter {
                AuraAttachmentFilter::Object(filter) => {
                    // CR 303.4f: the object needn't be a permanent. An enchant
                    // ability naming another zone ("enchant creature card in a
                    // graveyard") picks from that zone (Animate Dead returned
                    // by Sun Titan or found by Zur).
                    let candidate_zone = filter.zone.unwrap_or(Zone::Battlefield);
                    let mut candidates = Vec::new();
                    for (id, candidate) in &self.objects {
                        // CR 702.26b: phased-out permanents are treated as
                        // though they don't exist.
                        if *id == new_id
                            || candidate.zone != candidate_zone
                            || self.is_phased_out(*id)
                        {
                            continue;
                        }
                        // CR 303.4f: the object must be legal to enchant under
                        // "any other applicable effects", which includes
                        // protection (702.16c) but not hexproof or shroud.
                        if filter.matches(candidate, &filter_ctx, self)
                            && !crate::targeting::has_protection_from_source(self, *id, new_id)
                        {
                            candidates.push(crate::decisions::context::SelectableObject::new(
                                *id,
                                candidate.name.to_string(),
                            ));
                        }
                    }

                    // The object map's iteration order is hash-based; offer
                    // the choice (and its fallback) in a stable order.
                    candidates.sort_by_key(|candidate| candidate.id);
                    if candidates.is_empty() {
                        None
                    } else {
                        let fallback_target = candidates.first().map(|candidate| candidate.id);
                        let ctx = crate::decisions::context::SelectObjectsContext::new(
                            chooser,
                            Some(new_id),
                            "Attach Aura to",
                            candidates,
                            1,
                            Some(1),
                        );
                        decision_maker
                            .decide_objects(self, &ctx)
                            .first()
                            .copied()
                            .or(fallback_target)
                            .map(AttachmentTarget::Object)
                    }
                }
                AuraAttachmentFilter::Player(filter) => {
                    let candidates = self
                        .players
                        .iter()
                        .filter(|player| {
                            player.is_in_game()
                                && filter.matches_player(player.id, &filter_ctx)
                                && !match self.object(new_id) {
                                    Some(aura) => {
                                        crate::effects::permanents::player_has_protection_from_object(
                                            self, player.id, aura,
                                        )
                                    }
                                    None => crate::effects::permanents::player_has_protection_from_everything(
                                        self, player.id,
                                    ),
                                }
                        })
                        .map(|player| (player.id, player.name.to_string()))
                        .collect::<Vec<_>>();
                    if candidates.is_empty() {
                        None
                    } else if candidates.len() == 1 {
                        Some(AttachmentTarget::Player(candidates[0].0))
                    } else {
                        let choice_spec = crate::decisions::specs::ChoiceSpec::single(
                            new_id,
                            candidates
                                .iter()
                                .enumerate()
                                .map(|(idx, (_, name))| {
                                    crate::decisions::spec::DisplayOption::new(idx, name.clone())
                                })
                                .collect(),
                        );
                        let mut chosen = crate::decisions::make_decision(
                            self,
                            decision_maker,
                            chooser,
                            Some(new_id),
                            choice_spec,
                        );
                        chosen
                            .pop()
                            .and_then(|idx| candidates.get(idx).map(|(player_id, _)| *player_id))
                            .map(AttachmentTarget::Player)
                            .or_else(|| Some(AttachmentTarget::Player(candidates[0].0)))
                    }
                }
            };

            let attached = chosen_target.is_some_and(|target| {
                if !self.attach_object_to_target(new_id, target) {
                    return false;
                }
                self.effect_store
                    .continuous_effects
                    .record_attachment(new_id);
                true
            });
            if !attached && let Some(checkpoint) = aura_entry_checkpoint.take() {
                *self = checkpoint;
                if old_zone == Zone::Stack {
                    let Some(graveyard_id) = self.move_object(old_id, Zone::Graveyard, cause) else { return Ok(None); };
                    return Ok(Some(EntersResult {
                        new_id: graveyard_id,
                        enters_tapped: false,
                    }));
                }
                return Ok(Some(EntersResult {
                    new_id: old_id,
                    enters_tapped: false,
                }));
            }
        }

        if let Some(target) = entry_attachment
            && (!entry_attachment_requires_aura || self.calculated_subtypes(new_id).contains(&Subtype::Aura))
        {
            // Inspect the resolved entrant: copy/type replacements can make the
            // authored object an Aura or make a requested Aura a non-Aura.
            let is_aura = self.calculated_subtypes(new_id).contains(&Subtype::Aura);
            let already_attached = self.object(new_id).is_some_and(|object| object.attached_to == Some(target));
            let attached = already_attached || crate::effects::permanents::attach_battlefield_object_to_target(self, new_id, target);
            if is_aura && !attached {
                let checkpoint = aura_entry_checkpoint.take().ok_or_else(|| crate::effects::ExecutionError::InternalError(
                    "fixed Aura attachment lost its precommit checkpoint".into()))?;
                *self = checkpoint;
                if old_zone == Zone::Stack {
                    let Some(graveyard_id) = self.move_object(old_id, Zone::Graveyard, cause) else { return Ok(None); };
                    return Ok(Some(EntersResult { new_id: graveyard_id, enters_tapped: false }));
                }
                *original_verdict = EventOutcome::Prevented;
                return Ok(None);
            }
        }

        // "This creature enters prepared." The permanent is on the battlefield
        // by now, which is what the prepare spell copy's existence is tied to.
        //
        // This reads the printed abilities rather than the calculated ones:
        // every battlefield entry runs through here, and asking for calculated
        // characteristics would force a per-entry recomputation instead of the
        // batched layer rebuild. Entering prepared is a printed property, and
        // CR keeps the designation even if the permanent later loses abilities.
        if self.object_printed_static_ability(
            new_id,
            crate::static_abilities::StaticAbilityId::EntersPrepared,
        ) {
            self.set_prepared(new_id);
        }

        // CR 714.3a / 702.155b: a Saga gets its lore counter(s) as it enters,
        // however it enters.
        crate::game_loop::add_entry_lore_counters(self, new_id, decision_maker)?;

        // CR 709.5d: a Room gets the unlocked designation for the half that was
        // cast; one entering any other way has neither door unlocked. CR
        // 709.5h: "when you unlock this door" also triggers for a door that
        // entered unlocked.
        if let Some(room_controller) = self
            .object(new_id)
            .filter(|object| {
                object.linked_face_layout == LinkedFaceLayout::Split
                    && object.subtypes.contains(&Subtype::Room)
            })
            .map(|object| self.controller_of(object))
        {
            // A permanent entering as a copy of a Room (Clone) wasn't cast as
            // either half, so it enters with neither door unlocked.
            let cast_as_spell = old_zone == Zone::Stack
                && result.enters_as_copy_of.is_none()
                && self
                    .object(new_id)
                    .is_some_and(|object| object.kind == crate::object::ObjectKind::Card);
            if cast_as_spell {
                let provenance = self
                    .provenance_graph_mut()
                    .alloc_root_event(crate::events::EventKind::KeywordAction);
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::KeywordActionEvent::new(
                        crate::events::KeywordActionKind::UnlockDoor,
                        room_controller,
                        new_id,
                        1,
                    ),
                    provenance,
                );
                self.queue_trigger_event(provenance, event);
            } else {
                self.mark_room_entered_with_no_unlocked_door(new_id);
            }
        }

        Ok(Some(EntersResult {
            new_id,
            enters_tapped: result.enters_tapped,
        }))
    }

    /// Removes an object from the game completely (e.g., tokens ceasing to exist).
    /// This does NOT create a new object - the object is simply gone.
    pub fn remove_object(&mut self, id: ObjectId) {
        if !self.objects.contains_key(&id) {
            return;
        }
        self.battlefield_flags_mut().saga_entry_lore_processed.remove(&id);
        if let Some(stable_id) = self.object(id).map(|object| object.stable_id) {
            self.remove_stickers(stable_id);
        }
        self.mark_continuous_state_dirty();
        self.bump_mutation_revision();
        self.object_store.changes.record(id);
        self.object_store.render_changes.record(id);
        if let Some(obj) = self.objects.remove(&id).map(ObjectStore::into_owned_object) {
            if let Some(target) = obj.attached_to {
                match target {
                    AttachmentTarget::Object(parent_id) => {
                        if let Some(parent) = self.object_mut(parent_id) {
                            parent.attachments.retain(|existing| *existing != id);
                        }
                    }
                    AttachmentTarget::Player(player_id) => {
                        if let Some(player) = self.player_mut(player_id) {
                            player.attachments.retain(|existing| *existing != id);
                        }
                    }
                }
            }
            self.stable_id_index.remove(&obj.stable_id);
            self.auxiliary_tracking_mut()
                .sector_designations
                .remove(&id);
            {
                let commander_tracking = self.commander_tracking_mut();
                commander_tracking.melded_permanents.remove(&obj.stable_id);
                commander_tracking.merged_permanents.remove(&obj.stable_id);
                commander_tracking
                    .pending_merged_component_destinations
                    .remove(&obj.stable_id);
                commander_tracking.declined_command_zone_moves.remove(&id);
            }
            self.remove_from_zone_index(id, obj.zone, obj.owner);
        }
    }

    /// Removes an object ID from its zone index.
    fn remove_from_zone_index(&mut self, id: ObjectId, zone: Zone, owner: PlayerId) {
        let mut removed = false;
        match zone {
            Zone::Battlefield => {
                let before = self.battlefield.len();
                self.battlefield.remove_id(id);
                removed = self.battlefield.len() != before;
            }
            Zone::Command => {
                let before = self.command_zone.len();
                self.command_zone.remove_id(id);
                removed = self.command_zone.len() != before;
            }
            Zone::Exile => {
                let before = self.exile.len();
                self.exile.remove_id(id);
                removed = self.exile.len() != before;
            }
            Zone::Ante => {
                let before = self.ante.len();
                self.ante.remove_id(id);
                removed = self.ante.len() != before;
            }
            Zone::Library => {
                let was_top = self
                    .player(owner)
                    .and_then(|player| player.library.last().copied())
                    == Some(id);
                if let Some(player) = self.player_mut(owner) {
                    let before = player.library.len();
                    player.library.remove_id(id);
                    removed = player.library.len() != before;
                }
                if was_top {
                    self.bump_library_top_revision(owner);
                }
            }
            Zone::Hand => {
                if let Some(player) = self.player_mut(owner) {
                    let before = player.hand.len();
                    player.hand.remove_id(id);
                    removed = player.hand.len() != before;
                }
            }
            Zone::Graveyard => {
                if let Some(player) = self.player_mut(owner) {
                    let before = player.graveyard.len();
                    player.graveyard.remove_id(id);
                    removed = player.graveyard.len() != before;
                }
            }
            Zone::OutsideGame => {
                if let Some(player) = self.player_mut(owner) {
                    let before = player.sideboard.len();
                    player.sideboard.remove_id(id);
                    removed = player.sideboard.len() != before;
                }
            }
            Zone::Stack => {
                let before = self.stack.len();
                // A spell leaving the stack also leaves its zone index.
                // Triggered/activated abilities sharing its source ID remain
                // independent stack objects and must survive that move.
                self.stack
                    .retain(|entry| entry.object_id != id || entry.is_ability);
                removed = self.stack.len() != before;
            }
        }
        if removed {
            self.bump_zone_revision(zone);
        }
    }

    // =========================================================================
    // Zone Consistency Validation (Debug Only)
    // =========================================================================

    /// Validate that zone indexes are consistent with the canonical objects HashMap.
    ///
    /// This checks that:
    /// - Every ID in denormalized zone indexes (battlefield, exile, etc.) exists in objects
    /// - Every object's zone field matches exactly one denormalized index
    /// - No ID appears in multiple zone indexes
    ///
    /// Only runs in debug builds or paranoid invariant builds to avoid release performance impact.
    #[cfg(any(debug_assertions, feature = "paranoid-invariants"))]
    pub fn validate_zone_consistency(&self) -> Result<(), String> {
        use std::collections::HashSet;

        let mut seen_ids: HashSet<ObjectId> = HashSet::new();

        // Check battlefield
        for &id in &self.battlefield {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Battlefield => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in battlefield index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in battlefield index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check exile
        for &id in &self.exile {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Exile => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in exile index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in exile index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check command zone
        for &id in &self.command_zone {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Command => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in command zone index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in command zone index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check ante
        for &id in &self.ante {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Ante => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in ante index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in ante index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check player zones
        for player in &self.players {
            // Library
            for &id in &player.library {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Library => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s library has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s library doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Hand
            for &id in &player.hand {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Hand => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s hand has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s hand doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Graveyard
            for &id in &player.graveyard {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Graveyard => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s graveyard has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s graveyard doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Sideboard / outside the game
            for &id in &player.sideboard {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::OutsideGame => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s sideboard has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s sideboard doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }
        }

        // Check that all objects with non-Stack zones are in exactly one index
        for (&id, obj) in &self.objects {
            if obj.zone == Zone::Stack {
                // Stack objects are managed via StackEntry, not indexed
                continue;
            }
            if !seen_ids.contains(&id) {
                return Err(format!(
                    "Object #{} with zone {} is not in any zone index",
                    id.0, obj.zone
                ));
            }
        }

        Ok(())
    }

    /// Debug assertion for zone consistency. Panics if zones are inconsistent.
    #[cfg(any(debug_assertions, feature = "paranoid-invariants"))]
    pub fn debug_assert_zone_consistency(&self) {
        if let Err(e) = self.validate_zone_consistency() {
            panic!("Zone consistency violation: {}", e);
        }
    }

    /// Gets a reference to an object by ID.
    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.object_store.object(id)
    }

    /// Gets a mutable reference to an object by ID.
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        if !self.objects.contains_key(&id) {
            return None;
        }
        let counter = &self.runtime_cache.work_counters.generic_object_mutations;
        counter.set(counter.get().saturating_add(1));
        self.mark_continuous_state_dirty();
        let revision = self.bump_mutation_revision();
        self.runtime_cache
            .characteristics_cache
            .bump_object_revision(id, revision);
        let object = self.object_store.object_mut(id)?;
        object.last_modified = revision;
        Some(object)
    }

    pub(crate) fn objects_map(&self) -> &ObjectMap {
        self.object_store.objects_map()
    }

    /// Whether an attachment destination still exists. Aura enchant filters
    /// determine its permitted zone; Equipment and Fortifications additionally
    /// require a battlefield destination in attachment legality checks.
    pub fn attachment_target_exists(&self, target: AttachmentTarget) -> bool {
        match target {
            AttachmentTarget::Object(id) => self.object(id).is_some(),
            AttachmentTarget::Player(id) => {
                self.player(id).is_some_and(|player| player.is_in_game())
            }
        }
    }

    pub fn attachment_target_exists_on_battlefield(&self, target: AttachmentTarget) -> bool {
        match target {
            AttachmentTarget::Object(id) => self
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield),
            AttachmentTarget::Player(id) => {
                self.player(id).is_some_and(|player| player.is_in_game())
            }
        }
    }

    pub fn detach_object_from_current_target(&mut self, attachment_id: ObjectId) -> bool {
        let lookback_source_snapshots = self.trigger_source_lookback_snapshots();
        let attachment_snapshot = self
            .object(attachment_id)
            .map(|object| self.cached_object_snapshot_with_calculated_characteristics(object));
        self.mark_continuous_state_dirty();
        let Some(current_target) = self
            .object(attachment_id)
            .and_then(|object| object.attached_to)
        else {
            return false;
        };

        match current_target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id) {
                    parent
                        .attachments
                        .retain(|existing| *existing != attachment_id);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id) {
                    player
                        .attachments
                        .retain(|existing| *existing != attachment_id);
                }
            }
        }

        if let Some(object) = self.object_mut(attachment_id) {
            object.attached_to = None;
        }

        if let Some(snapshot) = attachment_snapshot {
            let provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ObjectBecameUnattached);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::ObjectBecameUnattachedEvent::new(
                    attachment_id,
                    current_target,
                    snapshot.controller,
                    Some(snapshot),
                ),
                provenance,
            )
            .with_lookback_source_snapshots(lookback_source_snapshots);
            self.queue_trigger_event(provenance, event);
        }

        true
    }

    pub fn attach_object_to_target(
        &mut self,
        attachment_id: ObjectId,
        target: AttachmentTarget,
    ) -> bool {
        self.mark_continuous_state_dirty();
        if !self
            .object(attachment_id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            || !self.attachment_target_exists(target)
        {
            return false;
        }

        self.detach_object_from_current_target(attachment_id);

        if let Some(object) = self.object_mut(attachment_id) {
            object.attached_to = Some(target);
        } else {
            return false;
        }

        match target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id)
                    && !parent.attachments.contains(&attachment_id)
                {
                    parent.attachments.push(attachment_id);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id)
                    && !player.attachments.contains(&attachment_id)
                {
                    player.attachments.push(attachment_id);
                }
            }
        }

        true
    }

    // =========================================================================
    // Counter Management
    // =========================================================================

    /// Add counters to an object and return a CounterPlaced event for trigger checking.
    ///
    /// This method adds the counters and returns the event that should be used
    /// to check for triggers (like saga chapter abilities).
    ///
    /// Returns None for zero counters or an absent or phased-out object.
    pub fn add_counters(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
    ) -> Option<crate::triggers::TriggerEvent> {
        // CR 702.26b: ordinary effects cannot change phased-out permanents.
        if amount == 0 || self.is_phased_out(id) {
            return None;
        }
        self.mark_continuous_state_dirty();
        let obj = self.object_mut(id)?;
        let previous_count = obj.counters.get(&counter_type).copied().unwrap_or(0);
        obj.add_counters(counter_type, amount);
        if amount > 0 {
            self.effect_store
                .continuous_effects
                .record_counter_change(id, counter_type);
        }
        self.record_counter_ui_effect_event("counters_added", id, counter_type, amount);

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CounterPlaced);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::CounterPlacedEvent::new(id, counter_type, amount)
                .with_previous_count(previous_count),
            event_provenance,
        ))
    }

    /// Remove counters from an object.
    ///
    /// Returns the actual number of counters removed and a trigger event.
    /// The actual removed amount may be less than requested if there weren't enough.
    pub fn remove_counters(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<(u32, crate::triggers::TriggerEvent)> {
        // CR 702.26b: ordinary effects cannot change phased-out permanents.
        if self.is_phased_out(id) {
            return None;
        }
        self.mark_continuous_state_dirty();
        let location_snapshot = self.object(id).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, self,
            )
        });
        let obj = self.object_mut(id)?;
        let removed = obj.remove_counters(counter_type, amount);
        let count_after = obj.counters.get(&counter_type).copied().unwrap_or(0);

        if removed == 0 {
            return None;
        }
        self.record_counter_ui_effect_event("counters_removed", id, counter_type, removed);

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        let mut event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::removed(
                counter_type,
                id,
                removed,
                source,
                source_controller,
            )
            .with_count_after(count_after),
            event_provenance,
        );
        if let Some(snapshot) = location_snapshot {
            event = event.with_lookback_source_snapshots(vec![snapshot]);
        }

        Some((removed, event))
    }

    /// Add counters with full tracking (source, controller) for the unified marker system.
    ///
    /// Returns a MarkersChangedEvent for trigger checking.
    pub fn add_counters_with_source(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<crate::triggers::TriggerEvent> {
        // CR 702.26b: ordinary effects cannot change phased-out permanents.
        if self.is_phased_out(id) {
            return None;
        }
        self.mark_continuous_state_dirty();
        if amount == 0 {
            return None;
        }

        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);
        let count_after = obj.counters.get(&counter_type).copied().unwrap_or(0);
        self.effect_store
            .continuous_effects
            .record_counter_change(id, counter_type);
        self.record_counter_ui_effect_event("counters_added", id, counter_type, amount);

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        // The event-time count lets chapter abilities (CR 714.2b) and
        // "Nth counter" triggers (122.7) compare the exact before/after pair.
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                id,
                amount,
                source,
                source_controller,
            )
            .with_count_after(count_after),
            event_provenance,
        ))
    }

    /// Record a UI-only counter change event for battlefield objects.
    fn record_counter_ui_effect_event(
        &mut self,
        kind: &str,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
    ) {
        if amount == 0 {
            return;
        }
        let Some(stable_id) = self
            .object(id)
            .filter(|obj| obj.zone == Zone::Battlefield)
            .map(|obj| obj.stable_id)
        else {
            return;
        };
        self.record_ui_effect_event(
            kind,
            None,
            None,
            vec![stable_id],
            Some(i64::from(amount)),
            Some(counter_type.description().into_owned()),
        );
    }

    /// Get the number of counters of a specific type on an object.
    pub fn counter_count(&self, id: ObjectId, counter_type: crate::object::CounterType) -> u32 {
        self.object(id)
            .and_then(|obj| obj.counters.get(&counter_type).copied())
            .unwrap_or(0)
    }

    /// Add player counters, returning every notification and replacement outcome.
    pub fn add_player_counters_with_source(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        self.add_player_counters_with_source_with_dm(player_id, counter_type, amount, source, source_controller, &mut dm)
    }

    /// Resolve player-counter choices and commit only the resolved event.
    /// Pending/error operations restore their state and return no partial events.
    pub fn add_player_counters_with_source_with_dm(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
        dm: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let mut cause = crate::events::cause::EventCause::effect();
        cause.source = source;
        cause.source_controller = source_controller;
        let mut ctx = crate::effects::ExecutionContext::new(source.unwrap_or(ObjectId::from_raw(0)), source_controller.unwrap_or(player_id), dm).with_cause(cause.clone());
        let event = crate::events::Event::put_player_counters(player_id, counter_type, amount, cause);
        crate::effects::counters::execute_player_counter_placement(self, &mut ctx, event)
    }

    /// Remove counters from a player and emit a unified marker event when applicable.
    ///
    /// Returns the actual number removed and the corresponding event.
    pub fn remove_player_counters_with_source(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<(u32, crate::triggers::TriggerEvent)> {
        if amount == 0 {
            return None;
        }

        let removed = if matches!(counter_type, crate::object::CounterType::Poison) {
            let current = self.player(player_id)?.poison_counters;
            let removed = current.min(amount);
            self.write_shared_poison(player_id, current.saturating_sub(removed));
            removed
        } else {
            self.player_mut(player_id)?
                .remove_counters(counter_type, amount)
        };

        if removed == 0 {
            return None;
        }

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some((
            removed,
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::MarkersChangedEvent::removed(
                    counter_type,
                    player_id,
                    removed,
                    source,
                    source_controller,
                ),
                event_provenance,
            ),
        ))
    }

    /// Check if an object has any counters of a specific type.
    pub fn has_counters(&self, id: ObjectId, counter_type: crate::object::CounterType) -> bool {
        self.counter_count(id, counter_type) > 0
    }

    // =========================================================================
    // Calculated Characteristics (with continuous effects applied)
    // =========================================================================

    /// Calculate all characteristics for an object, applying continuous effects.
    ///
    /// This includes effects from:
    /// - Registered continuous effects (from resolved spells/abilities)
    /// - Static abilities on permanents (generated dynamically)
    pub fn all_continuous_effects(&self) -> Vec<ContinuousEffect> {
        self.all_continuous_effects_arc().as_ref().clone()
    }

    /// Complete snapshot for fallible queries. A legacy clean cache is not
    /// sufficient evidence: only checked publication can establish trust.
    pub fn try_all_continuous_effects(
        &self,
    ) -> Result<Vec<ContinuousEffect>, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let revision = self.effect_store.continuous_effects.revision();
        if self.continuous_state_is_clean()
            && self.runtime_cache.static_effects_cache.borrow().has_checked_snapshot(revision)
        {
            return Ok(self.cached_continuous_effects_snapshot());
        }
        crate::static_ability_processor::try_get_all_continuous_effects(self, Default::default())
    }

    /// Shared form of [`Self::all_continuous_effects`].
    pub(crate) fn all_continuous_effects_arc(&self) -> Arc<Vec<ContinuousEffect>> {
        if self.continuous_state_is_clean() {
            return self.cached_continuous_effects_snapshot_arc();
        }
        Arc::new(crate::static_ability_processor::get_all_continuous_effects(
            self,
        ))
    }

    /// Combine registered and cached static-ability continuous effects.
    ///
    /// Unlike `all_continuous_effects`, this does not regenerate static-ability
    /// effects dynamically. Callers must only use this after
    /// `refresh_continuous_state` (or `update_static_ability_effects`) for the
    /// current state.
    pub(crate) fn cached_continuous_effects_snapshot(&self) -> Vec<ContinuousEffect> {
        self.cached_continuous_effects_snapshot_arc()
            .as_ref()
            .clone()
    }

    pub(crate) fn cached_continuous_effects_snapshot_arc(&self) -> Arc<Vec<ContinuousEffect>> {
        let revision = self.effect_store.continuous_effects.revision();
        if let Some((cached_revision, effects)) =
            self.runtime_cache.effects_snapshot.borrow().as_ref()
            && *cached_revision == revision
        {
            return Arc::clone(effects);
        }

        let mut effects: Vec<ContinuousEffect> = self
            .effect_store
            .continuous_effects
            .effects_sorted()
            .into_iter()
            .cloned()
            .collect();
        effects.reserve(
            self.effect_store
                .continuous_effects
                .static_ability_effects()
                .len(),
        );
        effects.extend(
            self.effect_store
                .continuous_effects
                .static_ability_effects()
                .iter()
                .cloned(),
        );
        let effects = Arc::new(effects);
        *self.runtime_cache.effects_snapshot.borrow_mut() = Some((revision, Arc::clone(&effects)));
        effects
    }

    /// Calculate all characteristics for an object using precomputed continuous effects.
    ///
    /// This avoids rebuilding/allocating the full effect list when multiple
    /// characteristic lookups happen in the same operation.
    pub fn calculated_characteristics_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        if let Some(chars) = self.face_down_conspiracy_characteristics(id) {
            return Some(chars);
        }
        if let Some(chars) = crate::continuous::in_progress_characteristics(self, id) {
            return Some(chars);
        }
        crate::continuous::calculate_characteristics_with_effects(
            id,
            &self.objects,
            effects,
            &self.battlefield,
            self.commander_objects(),
            self,
        )
    }

    pub(crate) fn calculated_characteristics_batch_with_effects(
        &self,
        ids: &[ObjectId],
        effects: &[ContinuousEffect],
    ) -> HashMap<ObjectId, crate::continuous::CalculatedCharacteristics> {
        let mut calculated = crate::continuous::calculate_characteristics_batch_with_effects(
            ids,
            &self.objects,
            effects,
            &self.battlefield,
            self.commander_objects(),
            self,
        );
        for id in ids {
            if let Some(chars) = self.face_down_conspiracy_characteristics(*id) {
                calculated.insert(*id, chars);
            }
        }
        calculated
    }

    /// Precompute calculated characteristics for a set of objects in one batch.
    ///
    /// This is useful for external snapshot builders that are about to inspect
    /// many battlefield objects and want to avoid repeated one-object layer
    /// calculations. The cache is transient and automatically invalidated by
    /// continuous-effect revision changes.
    pub fn prewarm_calculated_characteristics(&self, ids: &[ObjectId]) {
        if !self.continuous_state_is_clean() {
            return;
        }

        let effects_revision = self.effect_store.continuous_effects.revision();
        let missing: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| {
                // A nested prewarm can include its enclosing partial source.
                !crate::continuous::characteristics_calculation_in_progress(self, *id)
                    && !self
                    .runtime_cache
                    .characteristics_cache
                    .contains_valid_entry(*id, effects_revision)
            })
            .collect();
        if missing.is_empty() {
            return;
        }

        let effects = self.cached_continuous_effects_snapshot();
        let marker = crate::continuous::provisional_condition_marker();
        let calculated = self.calculated_characteristics_batch_with_effects(&missing, &effects);
        if crate::continuous::computed_provisionally(marker) {
            return;
        }
        for id in missing {
            self.runtime_cache.characteristics_cache.insert(
                id,
                effects_revision,
                calculated.get(&id).cloned(),
            );
        }
    }

    pub fn calculated_characteristics_arc(
        &self,
        id: ObjectId,
    ) -> Option<Arc<crate::continuous::CalculatedCharacteristics>> {
        if self
            .object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield && self.is_phased_out(id))
        {
            return None;
        }
        if let Some(chars) = crate::continuous::in_progress_characteristics(self, id) {
            return Some(Arc::new(chars));
        }
        let effects_revision = self.effect_store.continuous_effects.revision();
        if self.continuous_state_is_clean()
            && let Some(cached) = self
                .runtime_cache
                .characteristics_cache
                .get(id, effects_revision)
        {
            self.runtime_cache
                .work_counters
                .bump_characteristics_cache_hits();
            #[cfg(feature = "paranoid-invariants")]
            self.assert_cached_characteristics_fresh(id, cached.as_deref());
            return cached;
        }

        let all_effects = self.all_continuous_effects();
        self.runtime_cache
            .work_counters
            .bump_characteristics_full_recomputes();
        self.runtime_cache
            .work_counters
            .add_effects_considered(all_effects.len() as u64);
        if self.continuous_state_is_clean() {
            let mut scope = self.battlefield.clone();
            if !scope.contains(&id) {
                scope.push(id);
            }
            let missing: Vec<_> = scope
                .into_iter()
                .filter(|candidate| {
                    // A condition query can pull an enclosing source into
                    // this batch. Its current layer view is not a final result.
                    !crate::continuous::characteristics_calculation_in_progress(self, *candidate)
                        && !self
                        .runtime_cache
                        .characteristics_cache
                        .contains_valid_entry(*candidate, effects_revision)
                })
                .collect();
            if !missing.is_empty() {
                let marker = crate::continuous::provisional_condition_marker();
                let calculated_batch =
                    self.calculated_characteristics_batch_with_effects(&missing, &all_effects);
                // A self-dependent condition an enclosing evaluation treated
                // provisionally (CR 613.8) makes these results unfit to cache.
                if crate::continuous::computed_provisionally(marker) {
                    return calculated_batch.get(&id).cloned().map(Arc::new);
                }
                let mut calculated = None;
                for candidate in missing {
                    let cached = self.runtime_cache.characteristics_cache.insert(
                        candidate,
                        effects_revision,
                        calculated_batch.get(&candidate).cloned(),
                    );
                    if candidate == id {
                        calculated = cached;
                    }
                }
                return calculated;
            }
        }

        let marker = crate::continuous::provisional_condition_marker();
        let calculated = self.calculated_characteristics_with_effects(id, &all_effects);
        if self.continuous_state_is_clean() && !crate::continuous::computed_provisionally(marker) {
            return self.runtime_cache.characteristics_cache.insert(
                id,
                effects_revision,
                calculated,
            );
        }
        calculated.map(Arc::new)
    }

    pub fn calculated_characteristics(
        &self,
        id: ObjectId,
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        self.calculated_characteristics_arc(id)
            .map(|chars| chars.as_ref().clone())
    }

    #[cfg(feature = "paranoid-invariants")]
    fn assert_cached_characteristics_fresh(
        &self,
        id: ObjectId,
        cached: Option<&crate::continuous::CalculatedCharacteristics>,
    ) {
        if id.0 % 16 != 0 || !self.continuous_state_is_clean() {
            return;
        }
        let effects = self.cached_continuous_effects_snapshot();
        let recomputed = self.calculated_characteristics_with_effects(id, &effects);
        assert_eq!(
            format!("{cached:?}"),
            format!("{:?}", recomputed.as_ref()),
            "stale calculated-characteristics cache entry for object #{}",
            id.0
        );
    }

    /// Return the object's current characteristics in its zone.
    ///
    /// This view reflects continuous effects across all zones and expands
    /// semantic subtype implications like changeling.
    pub fn current_characteristics(&self, id: ObjectId) -> Option<CalculatedCharacteristics> {
        let object = self.object(id)?;
        if object.zone == Zone::Battlefield && self.is_phased_out(id) {
            return None;
        }
        let mut chars =
            self.calculated_characteristics(id)
                .unwrap_or_else(|| CalculatedCharacteristics {
                    name: object.name.clone(),
                    mana_cost: object.mana_cost_owned(),
                    linked_face_mana_value: object.linked_face_mana_value(),
                    compiled_card_text: object.compiled_card_text.clone(),
                    ability_labels: object.ability_labels.clone(),
                    power: object.power(),
                    toughness: object.toughness(),
                    card_types: object.zone_card_types().to_vec().into(),
                    subtypes: object.zone_subtypes().to_vec().into(),
                    supertypes: object.zone_supertypes().to_vec().into(),
                    world_supertype_since: object
                        .supertypes
                        .contains(&crate::types::Supertype::World)
                        .then_some(0),
                    colors: object.colors(),
                    loyalty: object.base_loyalty,
                    abilities: object.abilities.clone().into(),
                    static_abilities: object
                        .abilities
                        .iter()
                        .filter_map(|ability| match &ability.kind {
                            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                            _ => None,
                        })
                        .chain(object.level_granted_abilities().iter().cloned())
                        .chain(
                            object
                                .temporary_static_ability_grants
                                .iter()
                                .filter(|grant| !grant.is_expired(self.turn.turn_number))
                                .filter_map(|grant| grant.materialize()),
                        )
                        .collect::<Vec<_>>()
                        .into(),
                    ability_gain_prohibitions: Vec::new(),
                    aura_attach_filter: object.aura_attach_filter_owned(),
                    controller: self.controller_of(object),
                });

        Self::normalize_current_characteristic_subtypes(object, &mut chars);

        Some(chars)
    }

    fn normalize_current_characteristic_subtypes(object: &Object, chars: &mut CalculatedCharacteristics) {
        let has_changeling = chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Changeling);
        let can_have_creature_subtypes = chars.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                crate::types::CardType::Creature | crate::types::CardType::Kindred
            )
        });
        if object.zone != crate::zone::Zone::Battlefield
            && has_changeling
            && can_have_creature_subtypes
        {
            for subtype in crate::types::Subtype::all_creature_types() {
                if !chars.subtypes.contains(subtype) {
                    chars.subtypes.push(*subtype);
                }
            }
        }

    }

    /// Missing/phased objects remain absence; unresolved computation is an
    /// error and cannot reintroduce printed abilities through a fallback.
    pub fn try_current_characteristics(
        &self,
        id: ObjectId,
    ) -> Result<Option<CalculatedCharacteristics>, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let Some(object) = self.object(id) else { return Ok(None); };
        if object.zone == Zone::Battlefield && self.is_phased_out(id) { return Ok(None); }
        if let Some(mut chars) = crate::continuous::in_progress_characteristics(self, id) {
            Self::normalize_current_characteristic_subtypes(object, &mut chars);
            return Ok(Some(chars));
        }
        let effects = self.try_all_continuous_effects()?;
        self.try_current_characteristics_with_effects(id, &effects)
    }

    pub(crate) fn try_current_characteristics_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Result<Option<CalculatedCharacteristics>, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let Some(object) = self.object(id) else { return Ok(None); };
        if object.zone == Zone::Battlefield && self.is_phased_out(id) { return Ok(None); }
        let mut chars = self.calculated_characteristics_with_effects(id, effects)
            .ok_or(crate::static_ability_processor::StaticEffectDiscoveryError::UnavailableCharacteristics { object: id })?;
        Self::normalize_current_characteristic_subtypes(object, &mut chars);
        Ok(Some(chars))
    }

    /// Return the object's current name in its zone.
    pub fn current_name(&self, id: ObjectId) -> Option<String> {
        Some(self.current_characteristics(id)?.name.to_owned_string())
    }

    /// Return the object's current controller in its zone.
    pub fn current_controller(&self, id: ObjectId) -> Option<PlayerId> {
        self.current_controller_excluding_change_effect(id, None)
    }

    pub(crate) fn current_controller_excluding_change_effect(
        &self,
        id: ObjectId,
        skipped_effect: Option<ContinuousEffectId>,
    ) -> Option<PlayerId> {
        let Some(object) = self.object(id) else {
            // An ability on the stack is controlled by the player who put it
            // there (CR 113.8), whatever now controls its source.
            return self.stack_ability_entry(id).map(|entry| entry.controller);
        };
        if self.is_face_up_planar_object(id) {
            return if self.grand_melee().is_some() {
                self.planar_controller_of_face(id)
            } else {
                self.planar_controller()
            };
        }
        if self.is_vanguard_card(id) {
            return Some(object.owner);
        }
        if self.is_face_up_scheme(id) {
            return Some(object.owner);
        }
        if self.is_conspiracy_card(id) {
            return Some(object.owner);
        }
        // A nested layer query must use its enclosing view without publishing
        // that partial controller into the game-level resolved cache.
        if skipped_effect.is_none()
            && let Some(chars) = crate::continuous::in_progress_characteristics(self, id)
        {
            return Some(chars.controller);
        }
        if skipped_effect.is_none()
            && self.continuous_state_is_clean()
            && let Some(controller) = self.cached_current_controller(id, object)
        {
            return Some(controller);
        }

        let effects = self.effects_for_controller_query();
        self.controller_from_known_effects(id, object, &effects, skipped_effect)
    }

    fn cached_current_controller(&self, id: ObjectId, object: &Object) -> Option<PlayerId> {
        {
            let needs_rebuild = self
                .runtime_cache
                .controller_cache
                .borrow()
                .as_ref()
                .is_none_or(|cache| !cache.matches_state(self));
            if needs_rebuild {
                let change_effects = Arc::new(self.cached_effects_for_controller_query());
                *self.runtime_cache.controller_cache.borrow_mut() = Some(ControllerCache {
                    revision: self.effect_store.continuous_effects.revision(),
                    turn_number: self.turn.turn_number,
                    active_player: self.turn.active_player,
                    phase: self.turn.phase,
                    step: self.turn.step,
                    change_effects,
                    resolved: RefCell::new(Default::default()),
                });
            }
        }

        if let Some(controller) = self
            .runtime_cache
            .controller_cache
            .borrow()
            .as_ref()
            .and_then(|cache| cache.resolved.borrow().get(&id).copied())
        {
            return Some(controller);
        }

        let change_effects = {
            let cache = self.runtime_cache.controller_cache.borrow();
            Arc::clone(&cache.as_ref()?.change_effects)
        };
        let controller = self.controller_from_known_effects(id, object, &change_effects, None)?;
        if let Some(cache) = self.runtime_cache.controller_cache.borrow().as_ref() {
            cache.resolved.borrow_mut().insert(id, controller);
        }
        Some(controller)
    }

    fn cached_effects_for_controller_query(&self) -> Vec<ContinuousEffect> {
        // Control predicates can inspect copy/type/ability facts. Retain the
        // full known snapshot rather than querying discovery from a filter.
        self.cached_continuous_effects_snapshot_arc().iter().cloned().collect()
    }

    fn effects_for_controller_query(&self) -> Vec<ContinuousEffect> {
        if self.continuous_state_is_clean() {
            self.cached_effects_for_controller_query()
        } else {
            self.effect_store.continuous_effects.effects_sorted()
                .into_iter().cloned().collect()
        }
    }

    fn controller_from_known_effects(
        &self,
        id: ObjectId,
        object: &Object,
        effects: &[ContinuousEffect],
        skipped_effect: Option<ContinuousEffectId>,
    ) -> Option<PlayerId> {
        if !effects.iter().any(|effect|
            skipped_effect != Some(effect.id)
                && matches!(effect.modification, Modification::ChangeController(_)))
        {
            return Some(object.owner);
        }
        let query_effects: std::borrow::Cow<'_, [ContinuousEffect]> = match skipped_effect {
            Some(skipped) => std::borrow::Cow::Owned(effects.iter()
                .filter(|effect| effect.id != skipped).cloned().collect()),
            None => std::borrow::Cow::Borrowed(effects),
        };
        // Duration checks that omit one control effect need an independent
        // layer frame, not the enclosing frame that includes that effect.
        let isolated;
        let query_game = if skipped_effect.is_some() {
            isolated = self.clone();
            &isolated
        } else { self };
        crate::continuous::calculate_characteristics_with_effects(
            id, query_game.objects_map(), &query_effects, &query_game.battlefield,
            query_game.commander_objects(), query_game,
        ).map(|chars| chars.controller)
    }

    /// Return the object's current controller, falling back to its owner if the
    /// object cannot be evaluated through continuous effects.
    pub fn controller_of(&self, object: &Object) -> PlayerId {
        self.current_controller(object.id).unwrap_or(object.owner)
    }

    /// Return the object's current controller by object id.
    pub fn controller_of_id(&self, id: ObjectId) -> Option<PlayerId> {
        let Some(object) = self.object(id) else {
            return self.stack_ability_entry(id).map(|entry| entry.controller);
        };
        Some(self.controller_of(object))
    }

    /// Commit a controller change only after complete discovery succeeds.
    /// The original game is preserved on both preflight and post-change errors.
    pub fn set_current_controller(
        &mut self,
        id: ObjectId,
        controller: PlayerId,
    ) -> Result<(), crate::static_ability_processor::StaticEffectDiscoveryError> {
        if self.object(id).is_none() { return Ok(()); }
        let mut working = self.continuous_query_snapshot()?;
        if working.current_controller(id) == Some(controller) { return Ok(()); }
        working.stage_controller_change_for_assembly(id, controller);
        working.refresh_continuous_state()?;
        *self = working;
        Ok(())
    }

    /// Stage an ordinary control-changing effect while assembling an isolated
    /// proposal. The caller must validate the complete proposal before publishing
    /// it. This is not an initial/default controller assignment.
    pub fn stage_controller_change_for_assembly(&mut self, id: ObjectId, controller: PlayerId) {
        if self.object(id).is_none() || self.current_controller(id) == Some(controller) { return; }
        self.set_summoning_sick(id);
        let effect = ContinuousEffect::new(
            id, controller, EffectTarget::Specific(id), Modification::ChangeController(controller),
        ).until(Until::Forever);
        self.effect_store.continuous_effects.add_effect(effect);
    }

    /// Return the object's current card types in its zone.
    pub fn current_card_types(&self, id: ObjectId) -> Option<Vec<crate::types::CardType>> {
        Some(self.current_characteristics(id)?.card_types.to_vec())
    }

    /// Return the object's current subtypes in its zone.
    pub fn current_subtypes(&self, id: ObjectId) -> Option<Vec<crate::types::Subtype>> {
        Some(self.current_characteristics(id)?.subtypes.to_vec())
    }

    /// Return the object's current supertypes in its zone.
    pub fn current_supertypes(&self, id: ObjectId) -> Option<Vec<crate::types::Supertype>> {
        Some(self.current_characteristics(id)?.supertypes.to_vec())
    }

    /// Return the object's current colors in its zone.
    pub fn current_colors(&self, id: ObjectId) -> Option<crate::color::ColorSet> {
        Some(self.current_characteristics(id)?.colors)
    }

    /// Return the object's current power in its zone, if any.
    pub fn current_power(&self, id: ObjectId) -> Option<i32> {
        self.current_characteristics(id)?.power
    }

    /// Return the object's current toughness in its zone, if any.
    pub fn current_toughness(&self, id: ObjectId) -> Option<i32> {
        self.current_characteristics(id)?.toughness
    }

    /// Return the abilities an object currently has in its zone.
    pub fn current_abilities(&self, id: ObjectId) -> Option<Vec<Ability>> {
        Some(self.current_characteristics(id)?.abilities.to_vec())
    }

    /// Return a specific current ability by index.
    pub fn current_ability(&self, id: ObjectId, ability_index: usize) -> Option<Ability> {
        self.current_abilities(id)?.get(ability_index).cloned()
    }

    /// Return a specific current activated ability by index.
    pub fn current_activated_ability(
        &self,
        id: ObjectId,
        ability_index: usize,
    ) -> Option<ActivatedAbility> {
        let ability = self.current_ability(id, ability_index)?;
        match ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }
    }

    /// Check if an object has a specific static ability using precomputed effects.
    pub fn object_has_ability_with_effects(
        &self,
        id: ObjectId,
        ability: &StaticAbility,
        effects: &[ContinuousEffect],
    ) -> bool {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.static_abilities.contains(ability))
            .unwrap_or(false)
    }

    /// Check if an object has a specific card type using precomputed effects.
    pub fn object_has_card_type_with_effects(
        &self,
        id: ObjectId,
        card_type: crate::types::CardType,
        effects: &[ContinuousEffect],
    ) -> bool {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.card_types.contains(&card_type))
            .unwrap_or(false)
    }

    /// Get calculated subtypes using precomputed effects.
    pub fn calculated_subtypes_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Vec<crate::types::Subtype> {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.subtypes.to_vec())
            .unwrap_or_default()
    }

    /// Get calculated toughness using precomputed effects.
    pub fn calculated_toughness_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Option<i32> {
        self.calculated_characteristics_with_effects(id, effects)
            .and_then(|c| c.toughness)
    }

    /// Get the calculated power of a creature (with continuous effects applied).
    pub fn calculated_power(&self, id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(id).and_then(|c| c.power)
    }

    /// Get the calculated toughness of a creature (with continuous effects applied).
    pub fn calculated_toughness(&self, id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(id)
            .and_then(|c| c.toughness)
    }

    /// Check if an object has a specific static ability (with continuous effects applied).
    pub fn object_has_ability(&self, id: ObjectId, ability: &StaticAbility) -> bool {
        self.calculated_characteristics(id)
            .map(|c| c.static_abilities.contains(ability))
            .unwrap_or(false)
    }

    /// Whether an object's printed abilities include a static ability id.
    ///
    /// Unlike [`Self::current_has_static_ability_id`] this never asks for
    /// calculated characteristics, so it is safe on hot paths that run for
    /// every object.
    pub(crate) fn object_printed_static_ability(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        self.object(id).is_some_and(|object| {
            object.abilities.iter().any(|ability| {
                matches!(&ability.kind, crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id() == ability_id)
            })
        })
    }

    /// Check if an object has a static ability with the given ID.
    pub fn object_has_static_ability_id(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        self.current_has_static_ability_id(id, ability_id)
    }

    /// Check if an object currently has a static ability with the given ID.
    /// Whether a battlefield static ("You may activate abilities of creatures
    /// you control as though those creatures had haste") lets this object's
    /// {T}/{Q} abilities ignore summoning sickness.
    pub fn activates_abilities_as_though_haste(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        for perm_id in self.battlefield.iter().copied() {
            let Some(perm) = self.object(perm_id) else {
                continue;
            };
            let controller = self.controller_of(perm);
            let filter_ctx = self.filter_context_for(controller, Some(perm_id));
            let matches_grant = |ability: &crate::static_abilities::StaticAbility| {
                ability.is_active(self, perm_id)
                    && ability
                        .activate_abilities_as_though_haste()
                        .is_some_and(|grant| grant.filter.matches(object, &filter_ctx, self))
            };
            let granted = if let Some(chars) = self.calculated_characteristics(perm_id) {
                chars
                    .static_abilities
                    .iter()
                    .any(|ability| matches_grant(ability))
            } else {
                perm.abilities.iter().any(|ability| {
                    matches!(&ability.kind, crate::ability::AbilityKind::Static(static_ability)
                        if ability.functions_in(&perm.zone) && matches_grant(static_ability))
                })
            };
            if granted {
                return true;
            }
        }
        false
    }

    pub fn current_has_static_ability_id(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        // CR 702.26b: absence from the characteristic cache must not make a
        // phased-out permanent fall back to its printed abilities.
        if self.is_phased_out(id) {
            return false;
        }


        if let Some(chars) = self.calculated_characteristics(id) {
            return chars
                .static_abilities
                .iter()
                .any(|ability| ability.id() == ability_id && ability.is_active(self, id));
        }

        self.object(id).is_some_and(|object| {
            object.abilities.iter().any(|ability| {
                matches!(&ability.kind, crate::ability::AbilityKind::Static(static_ability)
                    if ability.functions_in(&object.zone)
                        && static_ability.id() == ability_id
                        && static_ability.is_active(self, id))
            }) || object
                .temporary_static_ability_grants
                .iter()
                .filter(|grant| !grant.is_expired(self.turn.turn_number))
                .filter_map(|grant| grant.materialize())
                .any(|ability| ability.id() == ability_id && ability.is_active(self, id))
        })
    }

    /// Get the calculated subtypes of an object (with continuous effects applied).
    pub fn calculated_subtypes(&self, id: ObjectId) -> Vec<crate::types::Subtype> {
        self.calculated_characteristics(id)
            .map(|c| c.subtypes.to_vec())
            .unwrap_or_default()
    }

    /// Get the calculated card types of an object (with continuous effects applied).
    pub fn calculated_card_types(&self, id: ObjectId) -> Vec<crate::types::CardType> {
        self.calculated_characteristics(id)
            .map(|c| c.card_types.to_vec())
            .unwrap_or_default()
    }

    /// Check if an object has a specific card type (with continuous effects applied).
    pub fn object_has_card_type(&self, id: ObjectId, card_type: crate::types::CardType) -> bool {
        self.current_card_types(id)
            .is_some_and(|card_types| card_types.contains(&card_type))
    }

    /// Check if an object currently has a specific card type.
    pub fn current_has_card_type(&self, id: ObjectId, card_type: crate::types::CardType) -> bool {
        self.object_has_card_type(id, card_type)
    }

    /// Check if an object currently has a specific subtype.
    pub fn current_has_subtype(&self, id: ObjectId, subtype: crate::types::Subtype) -> bool {
        self.current_subtypes(id)
            .is_some_and(|subtypes| subtypes.contains(&subtype))
    }

    /// Check if an object currently has a specific supertype.
    pub fn current_has_supertype(&self, id: ObjectId, supertype: crate::types::Supertype) -> bool {
        self.current_supertypes(id)
            .is_some_and(|supertypes| supertypes.contains(&supertype))
    }

    /// Check if an object is currently a creature.
    pub fn current_is_creature(&self, id: ObjectId) -> bool {
        self.current_has_card_type(id, crate::types::CardType::Creature)
    }

    // =========================================================================
    // "Can't" Effect Tracking (Rule 614.17)
    // =========================================================================

    /// Update the CantEffectTracker by scanning static abilities on the battlefield.
    ///
    /// Per Rule 614.17, "can't" effects are not replacement effects - they must
    /// be checked BEFORE attempting an action or event. This function scans all
    /// permanents on the battlefield and populates the tracker based on their
    /// static abilities.
    ///
    /// Call this after:
    /// - State-based actions are checked
    /// - Before processing any event that might be affected by "can't" effects
    /// - After any permanent enters or leaves the battlefield
    pub fn update_cant_effects(&mut self) {
        use crate::ability::AbilityKind;
        use crate::static_abilities::StaticAbility;

        // A duration ends permanently at its first false transition.
        let expired: std::collections::HashSet<usize> = self
            .effect_store
            .restriction_effects
            .iter()
            .enumerate()
            .filter_map(|(index, effect)| match &effect.duration {
                crate::effect::Until::ForAsLongAs(predicate)
                    if !crate::continuous::continuous_duration_predicate_matches(
                        predicate, self,
                    ) =>
                {
                    Some(index)
                }
                _ => None,
            })
            .collect();
        let mut index = 0;
        self.effect_store.restriction_effects.retain(|_| {
            let retain = !expired.contains(&index);
            index += 1;
            retain
        });

        // Clear existing tracker
        self.effect_store.cant_effects.clear();
        self.effect_store
            .mana_spend_effects
            .retain_effect_permissions(self.turn.turn_number);
        self.battlefield_flags_mut().damage_persists.clear();
        let vanguard_hand_modifiers = self
            .vanguard
            .as_ref()
            .map(|state| state.hand_modifiers.clone())
            .unwrap_or_default();
        for player in self.players.get_mut_for_derived_update().iter_mut() {
            player.max_hand_size = 7_i32.saturating_add(
                vanguard_hand_modifiers
                    .get(&player.id)
                    .copied()
                    .unwrap_or(0),
            );
            player.land_plays_per_turn = 1;
        }

        let all_effects = if self.continuous_state_is_clean() {
            self.cached_continuous_effects_snapshot_arc()
        } else {
            Arc::new(self.all_continuous_effects())
        };
        if self.cant_effects_static_scan_can_stay_empty(all_effects.as_slice()) {
            return;
        }

        // If no continuous effect can add or remove a restriction-producing
        // static ability, only printed/temporary sources with such an ability
        // need inspection. This avoids deriving every permanent merely because
        // one source (for example Mycosynth Lattice) changes a player rule.
        let effects_can_change_cant_abilities = all_effects
            .iter()
            .any(Self::continuous_effect_requires_cant_update);
        let abilities_to_apply: Vec<(StaticAbility, ObjectId, PlayerId)> =
            if !effects_can_change_cant_abilities {
                self.objects
                    .iter()
                    .flat_map(|(&object_id, object)| {
                        let zone = object.zone;
                        let controller = self.controller_of(object);
                        let mut abilities = object
                            .abilities
                            .iter()
                            .filter_map(|ability| match &ability.kind {
                                AbilityKind::Static(static_ability)
                                    if ability.functions_in(&zone)
                                        && Self::static_ability_requires_cant_update(
                                            static_ability,
                                        )
                                        && static_ability.is_active(self, object_id) =>
                                {
                                    Some((static_ability.clone(), object_id, controller))
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        if zone == Zone::Battlefield {
                            abilities.extend(
                                object
                                    .level_granted_abilities()
                                    .into_iter()
                                    .filter(|static_ability| {
                                        Self::static_ability_requires_cant_update(static_ability)
                                            && static_ability.is_active(self, object_id)
                                    })
                                    .map(|static_ability| (static_ability, object_id, controller)),
                            );
                            abilities.extend(
                                object
                                    .temporary_static_ability_grants
                                    .iter()
                                    .filter(|grant| !grant.is_expired(self.turn.turn_number))
                                    .filter_map(|grant| grant.materialize())
                                    .filter(|static_ability| {
                                        Self::static_ability_requires_cant_update(static_ability)
                                            && static_ability.is_active(self, object_id)
                                    })
                                    .map(|static_ability| (static_ability, object_id, controller)),
                            );
                        }
                        abilities
                    })
                    .collect()
            } else {
                // Ability-changing effects require the fully layered view so
                // grants and removals are reflected in restriction tracking.
                let battlefield_ids: Vec<_> = self
                    .objects
                    .iter()
                    .filter_map(|(&object_id, object)| {
                        (object.zone == Zone::Battlefield).then_some(object_id)
                    })
                    .collect();
                if self.continuous_state_is_clean() {
                    self.prewarm_calculated_characteristics(&battlefield_ids);
                }
                self.objects
                    .iter()
                    .flat_map(|(&object_id, object)| {
                        let zone = object.zone;
                        let controller = self.controller_of(object);
                        match zone {
                            Zone::Battlefield => if self.continuous_state_is_clean() {
                                self.calculated_characteristics_arc(object_id)
                            } else {
                                self.calculated_characteristics_with_effects(
                                    object_id,
                                    all_effects.as_slice(),
                                )
                                .map(Arc::new)
                            }
                            .map(|chars| {
                                chars
                                    .static_abilities
                                    .iter()
                                    .filter(|static_ability| {
                                        static_ability.is_active(self, object_id)
                                    })
                                    .cloned()
                                    .map(|static_ability| (static_ability, object_id, controller))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default(),
                            _ => object
                                .abilities
                                .iter()
                                .filter_map(|ability| {
                                    if let AbilityKind::Static(static_ability) = &ability.kind {
                                        if ability.functions_in(&zone)
                                            && static_ability.is_active(self, object_id)
                                        {
                                            Some((static_ability.clone(), object_id, controller))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>(),
                        }
                    })
                    .collect()
            };

        // Now apply each ability's restrictions using the trait method.
        // Maximum-hand-size modifications are rule changes applied in
        // timestamp order (CR 613.11, 402.2) together with the spell-created
        // "no maximum hand size" effects below, so defer them.
        let mut hand_size_modifications: Vec<(u64, HandSizeModification)> = Vec::new();
        for (static_ability, permanent_id, controller) in abilities_to_apply {
            if Self::static_ability_modifies_maximum_hand_size(&static_ability) {
                let timestamp = self
                    .effect_store
                    .continuous_effects
                    .get_object_timestamp(permanent_id)
                    .unwrap_or(0);
                hand_size_modifications.push((
                    timestamp,
                    HandSizeModification::Static(static_ability, permanent_id, controller),
                ));
                continue;
            }
            static_ability.apply_restrictions(self, permanent_id, controller);
        }

        // Apply active restriction effects from spells/abilities.
        let current_turn = self.turn.turn_number;
        let mut retained_restrictions = Vec::new();
        let mut active_restrictions = Vec::new();
        for effect in &self.effect_store.restriction_effects {
            if effect.is_active(self, current_turn) {
                retained_restrictions.push(effect.clone());
                active_restrictions.push(effect.clone());
            } else if effect.is_pending()
                || (matches!(
                    effect.duration,
                    crate::effect::Until::ControllersNextUntapStep
                ) && !effect.is_expired(current_turn))
            {
                retained_restrictions.push(effect.clone());
            }
        }
        self.effect_store.restriction_effects = retained_restrictions;

        let mut active_goad = Vec::new();
        for effect in &self.effect_store.goad_effects {
            if effect.is_active(self, current_turn) {
                active_goad.push(effect.clone());
            }
        }
        self.effect_store.goad_effects = active_goad;

        let mut restriction_tracker = CantEffectTracker::default();
        for effect in active_restrictions {
            if matches!(
                effect.restriction,
                crate::effect::Restriction::NoMaximumHandSize(_)
            ) {
                hand_size_modifications
                    .push((effect.timestamp, HandSizeModification::Restriction(effect)));
                continue;
            }
            effect.restriction.apply_with_tagged_objects(
                self,
                &mut restriction_tracker,
                effect.controller,
                Some(effect.source),
                effect.iterated_player,
                &effect.tagged_objects,
            );
        }
        hand_size_modifications.sort_by_key(|(timestamp, _)| *timestamp);
        for (_, modification) in hand_size_modifications {
            match modification {
                HandSizeModification::Static(static_ability, permanent_id, controller) => {
                    static_ability.apply_restrictions(self, permanent_id, controller);
                }
                HandSizeModification::Restriction(effect) => {
                    effect.restriction.apply_with_tagged_objects(
                        self,
                        &mut restriction_tracker,
                        effect.controller,
                        Some(effect.source),
                        effect.iterated_player,
                        &effect.tagged_objects,
                    );
                }
            }
        }
        self.effect_store.cant_effects.merge(restriction_tracker);

        // "Can't be regenerated" restrictions disable both new and existing shields.
        let cant_be_regenerated: Vec<_> = self
            .effect_store
            .cant_effects
            .cant_be_regenerated
            .iter()
            .copied()
            .collect();
        for object_id in cant_be_regenerated {
            self.effect_store
                .replacement_effects
                .remove_regeneration_shields_from_source(object_id);
            self.clear_regeneration_shields(object_id);
        }
    }

    fn static_ability_modifies_maximum_hand_size(static_ability: &StaticAbility) -> bool {
        use crate::static_abilities::StaticAbilityId;
        matches!(
            static_ability.id(),
            StaticAbilityId::NoMaximumHandSize
                | StaticAbilityId::SetMaximumHandSize
                | StaticAbilityId::ReduceMaximumHandSize
                | StaticAbilityId::IncreaseMaximumHandSize
                | StaticAbilityId::MaximumHandSizeSevenMinusYourGraveyardCardTypes
        )
    }

    fn cant_effects_static_scan_can_stay_empty(&self, all_effects: &[ContinuousEffect]) -> bool {
        use crate::ability::AbilityKind;

        if !self.effect_store.restriction_effects.is_empty()
            || !self.effect_store.goad_effects.is_empty()
        {
            return false;
        }

        if all_effects
            .iter()
            .any(Self::continuous_effect_requires_cant_update)
        {
            return false;
        }

        self.objects.values().all(|object| {
            let printed_abilities_are_irrelevant = object.abilities.iter().all(|ability| {
                if !ability.functions_in(&object.zone) {
                    return true;
                }
                match &ability.kind {
                    AbilityKind::Static(static_ability) => {
                        !Self::static_ability_requires_cant_update(static_ability)
                    }
                    _ => true,
                }
            });
            if !printed_abilities_are_irrelevant || object.zone != Zone::Battlefield {
                return printed_abilities_are_irrelevant;
            }

            let level_abilities_are_irrelevant = object
                .level_granted_abilities()
                .iter()
                .all(|ability| !Self::static_ability_requires_cant_update(ability));
            let temporary_abilities_are_irrelevant = object
                .temporary_static_ability_grants
                .iter()
                .filter(|grant| !grant.is_expired(self.turn.turn_number))
                .filter_map(|grant| grant.materialize())
                .all(|ability| !Self::static_ability_requires_cant_update(&ability));

            level_abilities_are_irrelevant && temporary_abilities_are_irrelevant
        })
    }

    fn continuous_effect_requires_cant_update(effect: &ContinuousEffect) -> bool {
        Self::modification_requires_cant_update(&effect.modification)
    }

    fn modification_requires_cant_update(modification: &Modification) -> bool {
        // Exhaustive on purpose: answering `false` for a variant that can add
        // or remove cant-relevant static abilities silently skips restriction
        // tracking (e.g. an aura's "doesn't untap" never taking effect).
        match modification {
            Modification::CopyOf { .. }
            | Modification::ChangeText { .. }
            | Modification::SetTextBox(_)
            | Modification::CopyStaticAbilityVariants { .. }
            // Restriction modifications materialize as cant-relevant static
            // abilities in calculated characteristics.
            | Modification::Restriction(_)
            // Removals can strip cant-relevant statics granted by other
            // effects; rerun the scan rather than reason about ordering.
            | Modification::RemoveAbility(_)
            | Modification::RemoveStaticAbilityFamily(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana => true,
            Modification::AddAbility(static_ability) => {
                Self::static_ability_requires_cant_update(static_ability)
            }
            Modification::AddAbilityGeneric(ability) => Self::ability_requires_cant_update(ability),
            // Replacing the ability list can remove a printed restriction even
            // when none of the replacement abilities creates one.
            Modification::SetAbilities(_) => true,
            // Activated/triggered ability additions and pure characteristic
            // changes cannot introduce cant-relevant statics.
            Modification::CopyActivatedAbilities { .. }
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::ChangeController(_)
            | Modification::SetName(_) | Modification::InsertNameWords { .. }
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAuraAttachmentFilter(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
            | Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless
            | Modification::SetPower { .. }
            | Modification::SetToughness { .. }
            | Modification::SetPowerToughness { .. }
            | Modification::ModifyPower(_)
            | Modification::ModifyToughness(_)
            | Modification::ModifyPowerToughness { .. }
            | Modification::ModifyPowerToughnessValue { .. }
            | Modification::ModifyPowerToughnessByColorCount { .. }
            | Modification::SwitchPowerToughness => false,
        }
    }

    fn ability_requires_cant_update(ability: &crate::ability::Ability) -> bool {
        match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                Self::static_ability_requires_cant_update(static_ability)
            }
            _ => false,
        }
    }

    pub(crate) fn static_ability_requires_cant_update(
        static_ability: &crate::static_abilities::StaticAbility,
    ) -> bool {
        use crate::static_abilities::StaticAbilityId;

        !matches!(
            static_ability.id(),
            StaticAbilityId::Flying
                | StaticAbilityId::FirstStrike
                | StaticAbilityId::DoubleStrike
                | StaticAbilityId::Deathtouch
                | StaticAbilityId::Flash
                | StaticAbilityId::Haste
                | StaticAbilityId::Intimidate
                | StaticAbilityId::Lifelink
                | StaticAbilityId::Menace
                // Protection legality is evaluated directly by targeting,
                // attachment, blocking, and damage paths; it does not
                // populate CantEffectTracker through apply_restrictions.
                | StaticAbilityId::Protection
                | StaticAbilityId::Reach
                | StaticAbilityId::Trample
                | StaticAbilityId::TrampleOverPlaneswalkers
                | StaticAbilityId::Vigilance
                | StaticAbilityId::Fear
                | StaticAbilityId::Skulk
                | StaticAbilityId::Prowess
                | StaticAbilityId::Flanking
                | StaticAbilityId::UmbraArmor
                | StaticAbilityId::Landwalk
                | StaticAbilityId::Shadow
                | StaticAbilityId::Horsemanship
                | StaticAbilityId::Wither
                | StaticAbilityId::Infect
                | StaticAbilityId::Changeling
                | StaticAbilityId::Partner
                | StaticAbilityId::PartnerWith
                | StaticAbilityId::Toxic
                | StaticAbilityId::DoctorsCompanion
                | StaticAbilityId::Assist
                | StaticAbilityId::ReadAhead
                | StaticAbilityId::Anthem
                | StaticAbilityId::GrantAbility
                | StaticAbilityId::GrantObjectAbilityForFilter
                | StaticAbilityId::EquipmentGrant
                | StaticAbilityId::AttachedAbilityGrant
                | StaticAbilityId::CharacteristicDefiningPT
                | StaticAbilityId::SetBasePowerToughnessForFilter
                | StaticAbilityId::AddCardTypes
                | StaticAbilityId::RemoveCardTypes
                | StaticAbilityId::SetCardTypes
                | StaticAbilityId::AddSubtypes
                | StaticAbilityId::AddAllSubtypesOfFamily
                | StaticAbilityId::SetLandSubtypes
                | StaticAbilityId::SetCreatureSubtypes
                | StaticAbilityId::AddColors
                | StaticAbilityId::SetColors
                | StaticAbilityId::SetName
                | StaticAbilityId::MakeColorless
                | StaticAbilityId::AddSupertypes
                | StaticAbilityId::RemoveSupertypes
                | StaticAbilityId::CostReduction
                | StaticAbilityId::ActivatedAbilityCostReduction
                | StaticAbilityId::ActivatedAbilityCostIncrease
                | StaticAbilityId::ThisSpellCostReduction
                | StaticAbilityId::ThisSpellCostReductionManaCost
                | StaticAbilityId::CostIncrease
                | StaticAbilityId::CostReductionManaCost
                | StaticAbilityId::CostIncreaseManaCost
                | StaticAbilityId::CostIncreasePerAdditionalTarget
                | StaticAbilityId::CostIncreaseManaCostPerAdditionalTarget
                | StaticAbilityId::Affinity
                | StaticAbilityId::AffinityForArtifacts
                | StaticAbilityId::Delve
                | StaticAbilityId::Convoke
                | StaticAbilityId::Improvise
                | StaticAbilityId::BlackManaMayBePaidWithLife
                | StaticAbilityId::MinimumSpellTotalMana
        )
    }

    pub fn keep_damage_marked(&mut self, object: ObjectId) {
        self.battlefield_flags_mut().damage_persists.insert(object);
    }

    pub fn damage_persists_on(&self, object: ObjectId) -> bool {
        self.battlefield_flags.damage_persists.contains(&object)
    }
}

#[cfg(test)]
mod chosen_option_tests {
    use super::*;

    #[test]
    fn named_color_creature_type_option_preserves_the_pair() {
        assert_eq!(
            named_color_creature_type_option("blue Camarid"),
            Some((crate::color::Color::Blue, crate::types::Subtype::Camarid))
        );
        assert_eq!(named_color_creature_type_option("blue artifact"), None);
    }

    #[test]
    fn linked_play_permission_can_force_one_land_entry_tapped() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = crate::ids::PlayerId::from_index(0);
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Exiled Land")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let exiled_id = game.create_object_from_card(&land, alice, Zone::Exile);
        let mut decisions = crate::decision::SelectFirstDecisionMaker;

        let result = game
            .move_object_with_etb_processing_with_entry_options(
                exiled_id,
                Zone::Battlefield,
                &mut decisions,
                true,
                true,
            ).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().expect("land should enter the battlefield");

        assert!(result.enters_tapped);
        assert!(game.is_tapped(result.new_id));
    }

    #[test]
    fn zero_counter_placement_does_not_mutate_or_publish_an_event() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter recipient")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let recipient = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.take_pending_trigger_events();
        let counter = crate::object::CounterType::PlusOnePlusOne;
        assert!(game.add_counters(recipient, counter, 0).is_none());
        assert!(!game.object(recipient).unwrap().counters.contains_key(&counter));
        assert!(game.take_pending_trigger_events().is_empty());
        let event = game.add_counters(recipient, counter, 1).unwrap();
        let placed = event.downcast::<crate::events::CounterPlacedEvent>().unwrap();
        assert_eq!((placed.permanent, placed.counter_type, placed.amount), (recipient, counter, 1));
        assert_eq!(game.counter_count(recipient, counter), 1);
    }

    fn counter_count(game: &GameState, object: ObjectId) -> u32 {
        game.object(object)
            .and_then(|object| {
                object
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne)
                    .copied()
            })
            .unwrap_or(0)
    }

    fn counter_retaining_definition() -> crate::cards::CardDefinition {
        let ability = crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::from_model(
                ironsmith_core::StaticAbility::counters_remain_across_zone_changes(
                    vec![Zone::Hand, Zone::Library],
                    "Counters remain on this creature as it moves to any zone other than a player's hand or library.",
                ),
            ),
        )
        .in_zones(vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Stack,
            Zone::Exile,
            Zone::Command,
            Zone::Ante,
            Zone::OutsideGame,
        ]);
        crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Counter Traveler",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .with_ability(ability)
        .build()
    }

    #[test]
    fn typed_counter_retention_survives_nonexcluded_zone_changes() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = crate::ids::PlayerId::from_index(0);
        let object = game.create_object_from_definition(
            &counter_retaining_definition(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(object)
            .expect("counter traveler should exist")
            .counters
            .insert(crate::object::CounterType::PlusOnePlusOne, 3);

        let graveyard = game
            .move_object(
                object,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_game_rule(),
            )
            .expect("counter traveler should move to the graveyard");
        assert_eq!(counter_count(&game, graveyard), 3);
        let exiled = game
            .move_object(
                graveyard,
                Zone::Exile,
                crate::events::cause::EventCause::from_game_rule(),
            )
            .expect("counter traveler should move to exile");
        assert_eq!(counter_count(&game, exiled), 3);
    }

    #[test]
    fn typed_counter_retention_clears_for_excluded_destinations() {
        for destination in [Zone::Hand, Zone::Library] {
            let mut game = GameState::new(vec!["Alice".to_string()], 20);
            let alice = crate::ids::PlayerId::from_index(0);
            let object = game.create_object_from_definition(
                &counter_retaining_definition(),
                alice,
                Zone::Battlefield,
            );
            game.object_mut(object)
                .expect("counter traveler should exist")
                .counters
                .insert(crate::object::CounterType::PlusOnePlusOne, 2);

            let moved = game
                .move_object(
                    object,
                    destination,
                    crate::events::cause::EventCause::from_game_rule(),
                )
                .expect("counter traveler should move");
            assert_eq!(counter_count(&game, moved), 0, "{destination:?}");
        }
    }

    #[test]
    fn ordinary_objects_still_lose_counters_when_they_change_zones() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = crate::ids::PlayerId::from_index(0);
        let ordinary = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Ordinary Creature",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .build();
        let object = game.create_object_from_definition(&ordinary, alice, Zone::Battlefield);
        game.object_mut(object)
            .expect("ordinary creature should exist")
            .counters
            .insert(crate::object::CounterType::PlusOnePlusOne, 2);

        let moved = game
            .move_object(
                object,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_game_rule(),
            )
            .expect("ordinary creature should move");
        assert_eq!(counter_count(&game, moved), 0);
    }
}

/// The zero-time, zero-cost suspend entry `CastSourceEffect` adds for a card
/// cast via granted suspend (no printed card has "Suspend 0—{0}").
fn is_synthetic_granted_suspend(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    matches!(
        method,
        crate::alternative_cast::AlternativeCastingMethod::Suspend { cost, time: 0 }
            if cost.is_empty()
    )
}

/// One maximum-hand-size modification awaiting timestamp-ordered application.
enum HandSizeModification {
    Static(StaticAbility, ObjectId, PlayerId),
    Restriction(super::RestrictionEffectInstance),
}

impl GameState {
    /// Snapshot of the object whose effect granted `source` its ability at
    /// `ability_index` — the Equipment in `Equipped creature has "... Return
    /// Trusty Boomerang to its owner's hand."` — captured when that ability
    /// is activated or triggers so its resolution can name the grantor under
    /// [`crate::tag::GRANTING_SOURCE_TAG`]. `None` for the object's own
    /// abilities.
    pub(crate) fn granting_source_snapshot(
        &self,
        source: ObjectId,
        ability_index: usize,
    ) -> Option<crate::snapshot::ObjectSnapshot> {
        let chars = self.current_characteristics(source)?;
        let granting = chars.abilities.origin(ability_index)?.granting_source()?;
        if granting == source {
            return None;
        }
        let object = self.object(granting)?;
        Some(crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            object, self,
        ))
    }

    /// The tag map entry exposing [`Self::granting_source_snapshot`].
    pub(crate) fn insert_granting_source_tag(
        &self,
        source: ObjectId,
        ability_index: usize,
        tagged_objects: &mut std::collections::HashMap<
            crate::tag::TagKey,
            Vec<crate::snapshot::ObjectSnapshot>,
        >,
    ) {
        if let Some(snapshot) = self.granting_source_snapshot(source, ability_index) {
            tagged_objects.insert(
                crate::tag::TagKey::from(crate::tag::GRANTING_SOURCE_TAG),
                vec![snapshot],
            );
        }
    }
}

#[cfg(test)]
mod replacement_direct_entry_zone_cause_contract_tests {
    use crate::events::cause::{CauseFilter, CauseType, ControllerFilter, EventCause};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;
    fn check(matching_cause: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let land_definition = crate::CardDefinitionBuilder::new(CardId::new(), "Entry proposal").card_types(vec![CardType::Land]).build();
        let source_definition = crate::CardDefinitionBuilder::new(CardId::new(), "Entry replacement").card_types(vec![CardType::Artifact]).build();
        let land = game.create_object_from_definition(&land_definition, alice, Zone::Hand);
        let stable = game.object(land).unwrap().stable_id;
        let source = game.create_object_from_definition(&source_definition, bob, Zone::Battlefield);
        let filter = CauseFilter::exact(CauseType::SpecialAction).with_source(ObjectFilter::specific(land)).with_controller(ControllerFilter::Player(alice));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(land), Some(Zone::Hand), Some(Zone::Battlefield)).with_cause_filter(filter), ReplacementAction::EnterTapped));
        let cause = if matching_cause { EventCause::from_special_action(Some(land), alice) } else { EventCause::from_cost(land, alice) };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = game.move_object_with_etb_processing_with_cause_and_entry_options_and_controller(land, Zone::Battlefield, cause, &mut dm, Some(alice), false, true);
        assert!(result.is_ok());
        let arrival = game.objects_in_deterministic_order().into_iter().find(|object| object.stable_id == stable).unwrap();
        assert_eq!(arrival.zone, Zone::Battlefield); assert_eq!(game.controller_of(arrival), alice);
        assert_eq!(game.is_tapped(arrival.id), matching_cause, "zone replacement must inspect the originating entry cause");
        assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_none(), matching_cause, "only the applicable shield is consumed");
    }
    #[test] fn direct_entry_preserves_zone_cause_source_and_controller() { check(true); }
    #[test] fn direct_entry_rejects_nonmatching_zone_cause() { check(false); }
}
