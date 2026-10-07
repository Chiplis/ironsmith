//! Exile effect implementation.

use super::movement_instruction::{SelectedZoneMovement, ZoneMovementInstruction};
use crate::color::{Color, ColorSet};
use crate::effect::EffectOutcome;
use crate::effects::CompletedEffectOutputs;

use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::filter::FilterContext;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::object_query::for_each_candidate_id_for_filter;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that exiles permanents.
///
/// Exile moves an object to the exile zone, subject to replacement effects.
/// Unlike destroy, exile is not affected by indestructible.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Exile target creature (targeted - can fizzle)
/// let effect = ExileEffect::target(ChooseSpec::creature());
///
/// // Exile all creatures (non-targeted - cannot fizzle)
/// let effect = ExileEffect::all(ObjectFilter::creature());
/// ```
pub type ExileEffect = ironsmith_core::ExileEffect;

fn fixed_cost_filter(effect: &ExileEffect) -> Option<(&ObjectFilter, u32)> {
    let ChooseSpec::Object(filter) = effect.spec.base() else {
        return None;
    };
    let count = effect.spec.count();
    if count.min == 0 || count.max != Some(count.min) {
        return None;
    }
    Some((filter, count.min as u32))
}

fn material_cost_filter(effect: &ExileEffect) -> Option<(&ObjectFilter, usize)> {
    let ChooseSpec::Object(filter) = effect.spec.base() else {
        return None;
    };
    let count = effect.spec.count();
    if count.min == 0 {
        return None;
    }
    Some((filter, count.min))
}

fn exile_from_hand_cost_filter(effect: &ExileEffect) -> Option<(&ObjectFilter, u32)> {
    let (filter, count) = fixed_cost_filter(effect)?;
    (filter.zone == Some(Zone::Hand)).then_some((filter, count))
}

fn exile_from_graveyard_cost_filter(effect: &ExileEffect) -> Option<(&ObjectFilter, u32)> {
    let (filter, count) = fixed_cost_filter(effect)?;
    (filter.zone == Some(Zone::Graveyard)).then_some((filter, count))
}

fn describe_card_type_cost_list(card_types: &[crate::types::CardType]) -> String {
    match card_types {
        [] => "card".to_string(),
        [one] => one.card_phrase().to_string(),
        [left, right] => format!(
            "{} and/or {} card",
            left.to_string().to_ascii_lowercase(),
            right.to_string().to_ascii_lowercase()
        ),
        _ => {
            let names = card_types
                .iter()
                .map(|card_type| card_type.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>();
            format!("{} card", names.join(", "))
        }
    }
}

fn matching_cost_candidates(
    game: &GameState,
    filter: &ObjectFilter,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) -> Vec<crate::ids::ObjectId> {
    let filter_ctx = FilterContext::new(controller).with_source(source);
    let mut candidates = Vec::new();
    let mut hand_ids = Vec::new();
    for_each_candidate_id_for_filter(game, filter, |id| {
        let Some(obj) = game.object(id) else {
            return;
        };
        if obj.zone == crate::zone::Zone::Hand {
            hand_ids.push(id);
        }
        if filter.matches(obj, &filter_ctx, game) {
            candidates.push(id);
        }
    });
    // Peers cannot evaluate hidden hand cards ("exile a blue card from your
    // hand"): count their placeholders as payable (see
    // `game_state::hidden_hand_choices`).
    for id in game.hidden_hand_payable_placeholders(filter, &filter_ctx, hand_ids) {
        if id != source && !candidates.contains(&id) {
            candidates.push(id);
        }
    }
    candidates
}

impl EffectExecutor for ExileEffect {
    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        vec![self.spec.clone()]
    }

    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        crate::effects::CostChoiceBindings::from_spec(&self.spec)
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Exiled)
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::movement_instruction::prepare_movement_instruction(
            self.clone(),
            ctx,
        ))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_replacement_draw_continuation(&self) -> bool { true }
    fn prepare_replacement_draw_continuation(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.prepare_replacement_draw_continuation_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }
    fn prepare_replacement_draw_continuation_with_outputs(&self, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        super::movement_instruction::prepare_movement_draw_continuation(self.clone(), game, ctx)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::movement_instruction::execute_movement_instruction(self.clone(), game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        // Non-targeted selections still need pre-action snapshots for linked
        // results, including their source zones and calculated characteristics.
        vec![self.spec.clone()]
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to exile"
    }

    fn exile_from_hand_cost_info(&self) -> Option<(u32, Option<ColorSet>)> {
        let (filter, count) = exile_from_hand_cost_filter(self)?;
        Some((count, filter.colors))
    }

    fn cost_description(&self) -> Option<String> {
        if let ChooseSpec::All(filter) = self.spec.base()
            && filter.zone == Some(Zone::Graveyard)
            && filter.owner == Some(crate::target::PlayerFilter::You)
        {
            return Some(format!("Exile all {}", filter.description()));
        }
        if matches!(self.spec.base(), ChooseSpec::Source) {
            return Some("Exile this source".to_string());
        }

        if let Some((filter, count)) = exile_from_hand_cost_filter(self) {
            let color_prefix = filter
                .colors
                .map(|colors| {
                    let mut pieces = Vec::new();
                    if colors.contains(Color::White) {
                        pieces.push("white");
                    }
                    if colors.contains(Color::Blue) {
                        pieces.push("blue");
                    }
                    if colors.contains(Color::Black) {
                        pieces.push("black");
                    }
                    if colors.contains(Color::Red) {
                        pieces.push("red");
                    }
                    if colors.contains(Color::Green) {
                        pieces.push("green");
                    }
                    if pieces.is_empty() {
                        String::new()
                    } else {
                        format!("{} ", pieces.join(" and "))
                    }
                })
                .unwrap_or_default();
            let amount = if count == 1 {
                "a".to_string()
            } else {
                count.to_string()
            };
            let noun = if count == 1 { "card" } else { "cards" };
            return Some(format!(
                "Exile {amount} {color_prefix}{noun} from your hand"
            ));
        }

        if let Some((filter, count)) = exile_from_graveyard_cost_filter(self) {
            let type_str = describe_card_type_cost_list(&filter.card_types);
            return Some(if count == 1 {
                format!("Exile a {type_str} from your graveyard")
            } else {
                format!("Exile {count} {type_str}s from your graveyard")
            });
        }

        None
    }
}

impl ZoneMovementInstruction for ExileEffect {
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SelectedZoneMovement, ExecutionError> {
        let retained_self = matches!(self.spec.base(), ChooseSpec::Source)
            .then(|| ObjectSnapshot::from_object_id(game, ctx.source))
            .flatten();
        let objects = match super::resolve_zone_move_objects(game, ctx, &self.spec) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) => {
                return Ok(SelectedZoneMovement::Finished(
                    if self.spec.count().min == 0 {
                        EffectOutcome::count(0)
                    } else {
                        EffectOutcome::target_invalid()
                    },
                ));
            }
            Err(error) => return Err(error),
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedZoneMovement::Finished(EffectOutcome::count(0)));
        }
        let snapshots = objects
            .iter()
            .filter_map(|id| {
                game.object(*id).map(|object| ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)
                    .map(|snapshot| (*id, snapshot)))
            })
            .collect::<Result<std::collections::HashMap<_, _>, ExecutionError>>()?;
        let requests = objects
            .into_iter()
            .filter_map(|id| {
                snapshots.get(&id).map(|snapshot| {
                    super::PreparedZoneMove::capture(
                        game,
                        id,
                        snapshot.zone,
                        ctx.simultaneous_zone_destination(id).unwrap_or(Zone::Exile),
                        ctx.cause.clone(),
                        Some(snapshot.clone()),
                    )
                })
            })
            .collect();

        let effect = self.clone();
        Ok(SelectedZoneMovement::moves(
            requests,
            move |game, ctx, receipts, pending_start| {
                let originals = super::observe_zone_move_originals(
                    game,
                    ctx,
                    receipts,
                    &snapshots,
                    pending_start,
                )?;
                let mut affected = Vec::new();
                let mut memories = Vec::new();
                let mut moved_source = None;
                for (id, change, snapshot) in &originals {
                    affected.extend(change.new_object_ids.iter().copied());
                    memories.push(snapshot.clone());
                    // Explicit self-exile exports its exact successor to later
                    // instructions; unrelated same-card incarnations do not qualify.
                    if matches!(effect.spec.base(), ChooseSpec::Source) && *id == ctx.source {
                        moved_source = change.new_object_id;
                    }
                    if change.final_zone != Zone::Exile {
                        continue;
                    }
                    for &new_id in &change.new_object_ids {
                        if effect.face_down {
                            game.set_face_down(new_id);
                            if !effect.exclude_prior_zone_viewers
                                && let Some(viewers) = ctx.face_down_exile_viewers_for(*id) {
                                for &viewer in viewers {
                                    game.grant_face_down_exile_view(new_id, viewer);
                                }
                            }
                        }
                        if let Some(owner) = &ctx.linked_exile_owner {
                            game.add_linked_exile_pair_member(owner.clone(), new_id);
                        }
                        game.add_exiled_with_source_link(ctx.source, new_id);
                        if effect.source_controller_may_look {
                            game.grant_face_down_exile_source_controller_view(new_id, ctx.source);
                        }
                        if let Some(object) = game.object(new_id) {
                            ctx.tag_source_exiled_result(ObjectSnapshot::from_object(object, game));
                        }
                    }
                }
                if let Some(before) = retained_self.as_ref() {
                    // A legally started cost remains paid when its original is
                    // replaced/prevented. This binding follows only the receipt's
                    // successors; it is independent of the parent Exiled result.
                    let after = receipts
                        .iter()
                        .find(|(id, _)| *id == before.object_id)
                        .map(|(id, receipt)| super::movement_arrivals(game, *id, receipt))
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|id| game.object(id))
                        .find(|object| object.stable_id == before.stable_id)
                        .map(|object| ObjectSnapshot::from_object(object, game))
                        .unwrap_or_else(|| before.clone());
                    ctx.set_tagged_objects(crate::tag::SOURCE_EXILED_SELF_TAG, vec![after]);
                }
                if let Some(new_source) = moved_source {
                    ctx.source = new_source;
                }
                let outcome = if effect.spec.is_target()
                    && effect.spec.is_single()
                    && matches!(
                        effect.spec.base(),
                        ChooseSpec::Object(_)
                            | ChooseSpec::Player(_)
                            | ChooseSpec::AnyTarget
                            | ChooseSpec::AnyOtherTarget
                    ) {
                    match receipts.first().map(|(_, receipt)| &receipt.original) {
                        Some(EventOutcome::Proceed(_)) => EffectOutcome::resolved(),
                        Some(EventOutcome::Prevented) => EffectOutcome::prevented(),
                        Some(EventOutcome::Replaced) => EffectOutcome::replaced(),
                        Some(EventOutcome::NotApplicable) | None => EffectOutcome::target_invalid(),
                    }
                } else {
                    EffectOutcome::count(originals.len() as i32)
                };
                Ok(outcome
                    .with_result_objects(affected.clone())
                    .with_affected_objects(affected)
                    .with_affected_object_memory(memories))
            },
        ))
    }
}

impl CostExecutableEffect for ExileEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if let ChooseSpec::All(filter) = self.spec.base()
            && filter.zone == Some(Zone::Graveyard)
            && filter.owner == Some(crate::target::PlayerFilter::You)
        {
            // The entire current set is the cost, including an empty set.
            // Choosing to pay it remains distinct from declining the offer.
            return if game.player(controller).is_some_and(|player| player.is_in_game()) { Ok(()) }
                else { Err(crate::effects::CostValidationError::Other("payer not in game".into())) };
        }
        if matches!(self.spec.base(), ChooseSpec::Source) && game.object(source).is_some() {
            return Ok(());
        }

        if let Some((filter, count)) = material_cost_filter(self) {
            let matching = matching_cost_candidates(game, filter, source, controller);
            if matching.len() < count {
                return Err(crate::effects::CostValidationError::NotEnoughCards);
            }
            return Ok(());
        }

        Err(crate::effects::CostValidationError::Other(
            "unsupported exile cost".to_string(),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::decision::DecisionMaker;
    use crate::effect::ChoiceCount;
    use crate::effects::ChooseObjectsEffect;
    use crate::effects::LookAtTopCardsEffect;
    use crate::effects::ShuffleGraveyardIntoLibraryEffect;
    use crate::effects::{ExecutionContext, ResolvedTarget};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::tag::TagKey;
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_card(
        card_id: u32,
        name: &str,
        mana_symbols: Vec<ManaSymbol>,
        card_type: CardType,
    ) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![mana_symbols]))
            .card_types(vec![card_type])
            .build()
    }

    fn add_card_to_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
        mana_symbols: Vec<ManaSymbol>,
        card_type: CardType,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_card(id.0 as u32, name, mana_symbols, card_type);
        let obj = Object::from_card(id, &card, owner, zone);
        game.add_object(obj);
        id
    }

    fn add_card_with_types_to_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(card_types)
            .build();
        game.add_object(Object::from_card(id, &card, owner, zone));
        id
    }

    #[test]
    fn self_exile_preserves_source_for_return_during_spell_cast_trigger() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = add_card_to_zone(
            &mut game,
            alice,
            Zone::Battlefield,
            "Self Exile Probe",
            vec![],
            CardType::Creature,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(ObjectId(999), alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let mut stale_trigger = ExecutionContext::new_default(source, alice)
            .with_triggering_event(event.clone())
            .with_source_snapshot(ObjectSnapshot::from_object(
                game.object(source).unwrap(),
                &game,
            ));
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        ExileEffect::with_spec(ChooseSpec::Source)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_ne!(ctx.source, source);
        assert_eq!(game.object(ctx.source).unwrap().zone, Zone::Exile);

        crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Battlefield, false)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.object(ctx.source).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.exile.is_empty());

        ExileEffect::with_spec(ChooseSpec::Source)
            .execute(&mut game, &mut stale_trigger)
            .unwrap();
        assert_eq!(
            game.object(ctx.source).unwrap().zone,
            Zone::Battlefield,
            "another trigger from the old object must not exile the returned permanent"
        );
        assert!(game.exile.is_empty());
    }

    #[test]
    fn test_exile_from_hand_cost_uses_generic_exile_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = add_card_to_zone(
            &mut game,
            alice,
            Zone::Hand,
            "Source",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );
        add_card_to_zone(
            &mut game,
            alice,
            Zone::Hand,
            "Pitch",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );

        let effect = ExileEffect::with_spec(
            ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(crate::target::PlayerFilter::You)
                    .with_colors(ColorSet::from(Color::Blue))
                    .other(),
            )
            .with_count(ChoiceCount::exactly(1)),
        );

        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_ok()
        );
        assert_eq!(
            effect.exile_from_hand_cost_info(),
            Some((1, Some(ColorSet::from(Color::Blue))))
        );
    }

    #[test]
    fn test_exile_from_graveyard_cost_executes_via_generic_exile_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Spell",
            vec![ManaSymbol::Generic(1)],
            CardType::Instant,
        );

        let effect = ExileEffect::with_spec(
            ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(crate::target::PlayerFilter::You)
                    .with_type(CardType::Instant),
            )
            .with_count(ChoiceCount::exactly(1)),
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(card_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.exile.len(), 1);
    }

    #[test]
    fn targeted_exile_preserves_lki_for_type_filtered_followups() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Exiled Creature",
            vec![ManaSymbol::Generic(1)],
            CardType::Creature,
        );

        let effect = ExileEffect::with_spec(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default().in_zone(Zone::Graveyard),
        )));
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(card_id)]);

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("targeted exile should resolve");
        let memory = result
            .affected_object_memory()
            .expect("targeted exile should preserve the moved card's LKI");

        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].zone, Zone::Graveyard);
        assert!(memory[0].card_types.contains(&CardType::Creature));
    }

    #[test]
    fn exile_one_per_card_type_uses_distinct_assignable_type_slots() {
        struct SelectAll;

        impl DecisionMaker for SelectAll {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let artifact_creature = add_card_with_types_to_zone(
            &mut game,
            bob,
            Zone::Graveyard,
            "Artifact Creature",
            vec![CardType::Creature, CardType::Artifact],
        );
        let creature = add_card_with_types_to_zone(
            &mut game,
            bob,
            Zone::Graveyard,
            "Creature",
            vec![CardType::Creature],
        );
        let first_instant = add_card_with_types_to_zone(
            &mut game,
            bob,
            Zone::Graveyard,
            "First Instant",
            vec![CardType::Instant],
        );
        let second_instant = add_card_with_types_to_zone(
            &mut game,
            bob,
            Zone::Graveyard,
            "Second Instant",
            vec![CardType::Instant],
        );

        let mut filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::Specific(bob));
        filter.one_per_card_type = true;
        let effect = ExileEffect::with_spec(
            ChooseSpec::Object(filter).with_count(ChoiceCount::any_number()),
        );
        let source = game.new_object_id();
        let mut dm = SelectAll;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("one-per-type exile should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert!(game.object(artifact_creature).is_none());
        assert!(game.object(creature).is_none());
        assert!(game.object(first_instant).is_none());
        assert!(game.object(second_instant).is_some());
    }

    #[test]
    fn red_instant_or_sorcery_material_cost_counts_multicolor_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = add_card_to_zone(
            &mut game,
            alice,
            Zone::Battlefield,
            "Craft Source",
            vec![ManaSymbol::Generic(1)],
            CardType::Artifact,
        );

        add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Arc Lightning",
            vec![ManaSymbol::Generic(2), ManaSymbol::Red],
            CardType::Sorcery,
        );
        add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Lightning Helix",
            vec![ManaSymbol::Red, ManaSymbol::White],
            CardType::Instant,
        );
        add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Lightning Strike",
            vec![ManaSymbol::Generic(1), ManaSymbol::Red],
            CardType::Instant,
        );
        add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Lightning Bolt",
            vec![ManaSymbol::Red],
            CardType::Instant,
        );

        let effect = ExileEffect::with_spec(
            ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_colors(ColorSet::from(Color::Red))
                    .with_type(CardType::Instant)
                    .with_type(CardType::Sorcery),
            )
            .with_count(ChoiceCount::at_least(4)),
        );

        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_ok()
        );
    }

    #[test]
    fn choose_from_library_then_exile_face_down_grants_searcher_visibility() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            bob,
            Zone::Library,
            "Hidden Trophy",
            vec![ManaSymbol::Generic(2)],
            CardType::Artifact,
        );

        let choose = ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Library)
                .owned_by(PlayerFilter::Specific(bob)),
            1,
            PlayerFilter::Specific(alice),
            "chosen",
        )
        .in_zone(Zone::Library)
        .as_search();
        let exile =
            ExileEffect::with_spec(ChooseSpec::Tagged(TagKey::from("chosen"))).with_face_down(true);
        let mut ctx = ExecutionContext::new_default(source, alice);

        choose
            .execute(&mut game, &mut ctx)
            .expect("choose should resolve");
        exile
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");

        let exiled_id = *game.exile.last().expect("card should be in exile");
        assert_ne!(exiled_id, card_id, "exiled card should be a new object");
        assert!(game.is_face_down(exiled_id));
        assert!(
            game.can_player_look_at_face_down_exiled_card(exiled_id, alice),
            "searcher should keep access to the chosen face-down exiled card"
        );
        assert!(
            !game.can_player_look_at_face_down_exiled_card(exiled_id, bob),
            "library owner should not automatically gain access to an opponent-chosen face-down exiled card"
        );
    }

    #[test]
    fn look_at_top_then_exile_face_down_grants_viewer_visibility() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Library,
            "Hidden Insight",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );

        let look = LookAtTopCardsEffect::new(PlayerFilter::You, 1, "looked");
        let exile =
            ExileEffect::with_spec(ChooseSpec::Tagged(TagKey::from("looked"))).with_face_down(true);
        let mut ctx = ExecutionContext::new_default(source, alice);

        look.execute(&mut game, &mut ctx)
            .expect("look should resolve");
        exile
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");

        let exiled_id = *game.exile.last().expect("card should be in exile");
        assert_ne!(exiled_id, card_id, "exiled card should be a new object");
        assert!(game.is_face_down(exiled_id));
        assert!(
            game.can_player_look_at_face_down_exiled_card(exiled_id, alice),
            "player who looked at the card should keep access after it is exiled face down"
        );
    }

    #[test]
    fn exile_all_library_face_down_then_shuffle_graveyard_matches_inverter_behavior() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let library_one = add_card_to_zone(
            &mut game,
            alice,
            Zone::Library,
            "Library One",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );
        let library_two = add_card_to_zone(
            &mut game,
            alice,
            Zone::Library,
            "Library Two",
            vec![ManaSymbol::Black],
            CardType::Sorcery,
        );
        let graveyard_one = add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Graveyard One",
            vec![ManaSymbol::Green],
            CardType::Creature,
        );
        let graveyard_two = add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Graveyard Two",
            vec![ManaSymbol::Red],
            CardType::Artifact,
        );

        let exile = ExileEffect::all(
            ObjectFilter::default()
                .in_zone(Zone::Library)
                .owned_by(PlayerFilter::You),
        )
        .with_face_down(true);
        let shuffle = ShuffleGraveyardIntoLibraryEffect::new(PlayerFilter::You);
        let mut ctx = ExecutionContext::new_default(source, alice);

        exile
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");
        assert_eq!(game.exile.len(), 2);
        assert!(game.object(library_one).is_none());
        assert!(game.object(library_two).is_none());
        assert!(game.exile.iter().all(|card_id| game.is_face_down(*card_id)));

        shuffle
            .execute(&mut game, &mut ctx)
            .expect("graveyard shuffle should resolve");
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 0);
        assert_eq!(game.player(alice).unwrap().library.len(), 2);
        assert!(game.object(graveyard_one).is_none());
        assert!(game.object(graveyard_two).is_none());
    }

    #[test]
    fn reexiling_face_down_card_in_exile_preserves_face_down_and_visibility() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Exile,
            "Still Hidden",
            vec![ManaSymbol::Generic(1)],
            CardType::Artifact,
        );
        game.set_face_down(card_id);
        game.grant_face_down_exile_view(card_id, alice);

        let new_id = game
            .move_object_by_effect(card_id, Zone::Exile)
            .expect("re-exile should create a new object");

        assert_ne!(new_id, card_id);
        assert!(game.is_face_down(new_id));
        assert!(
            game.can_player_look_at_face_down_exiled_card(new_id, alice),
            "re-exiled face-down cards should keep their existing look permission"
        );
        assert!(game.object(card_id).is_none(), "old object should be gone");
    }
}
