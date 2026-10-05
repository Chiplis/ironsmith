//! Create token effect implementation.

use crate::cards::CardDefinition;
use crate::combat_state::AttackTarget;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::{ChooseSpec, SourceReferenceSurface};
use crate::zone::Zone;

use super::create_token_copy::{
    CopyAttackTargetMode, attack_targets_for_player, choose_attack_target,
};
use super::lifecycle::{
    TokenCleanupOptions, TokenEntryOptions, apply_token_battlefield_entry,
    create_replacement_additional_tokens, schedule_token_cleanup,
};

/// Effect that creates token creatures or other token permanents.
///
/// # Fields
///
/// * `token` - The token definition (use CardDefinitionBuilder with .token())
/// * `count` - How many tokens to create
/// * `controller` - Who controls the tokens
/// * `suppress_aura_attachment_choice` - Whether Aura token attachment is handled by a later effect
/// * `enters_tapped` - Whether the tokens enter tapped
/// * `enters_attacking` - Whether the tokens enter attacking
/// * `exile_at_end_of_combat` - Whether to exile the tokens at end of combat
/// * `sacrifice_at_end_of_combat` - Whether to sacrifice the tokens at end of combat
/// * `sacrifice_at_next_end_step` - Whether to sacrifice the tokens at the
///   beginning of the next end step.
/// * `exile_at_next_end_step` - Whether to exile the tokens at the beginning
///   of the next end step.
///
/// # Example
///
/// ```ignore
/// // Create two 1/1 white Soldier tokens
/// let soldier = CardDefinitionBuilder::new(CardId::new(), "Soldier")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Soldier])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(1, 1))
///     .build();
/// let effect = CreateTokenEffect::new(soldier, 2, PlayerFilter::You);
///
/// // Create a 4/4 Angel token that enters tapped and attacking, exiled at EOC
/// let angel = CardDefinitionBuilder::new(CardId::new(), "Angel")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Angel])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(4, 4))
///     .flying()
///     .build();
/// let effect = CreateTokenEffect::one(angel)
///     .tapped()
///     .attacking()
///     .exile_at_end_of_combat();
/// ```
pub type CreateTokenEffect = ironsmith_core::CreateTokenEffect<CardDefinition>;

/// Bind a proper-name reference in a token's quoted CDA to the object whose
/// effect created that token. Ordinary unmarked `Source` values remain local
/// to the token itself.
fn materialize_named_creator_source_spec(spec: &mut Box<ChooseSpec>, source: ObjectId) -> bool {
    if !matches!(spec.base(), ChooseSpec::Source)
        || !matches!(
            spec.source_reference_surface(),
            Some(SourceReferenceSurface::FullName(_) | SourceReferenceSurface::ShortName(_))
        )
    {
        return false;
    }

    let hints = spec.surface_hints().to_vec();
    **spec = ChooseSpec::SpecificObject(source).with_surface_hints(hints);
    true
}

fn materialize_named_creator_source_in_value(
    value: &mut crate::effect::Value,
    source: ObjectId,
) -> bool {
    use crate::effect::Value;

    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => materialize_named_creator_source_in_value(value, source),
        Value::Add(left, right) | Value::Min(left, right) => {
            let left_changed = materialize_named_creator_source_in_value(left, source);
            let right_changed = materialize_named_creator_source_in_value(right, source);
            left_changed || right_changed
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::CountersOn(spec, _) => materialize_named_creator_source_spec(spec, source),
        Value::ManaSymbolsInManaCostOf { spec, .. } => {
            materialize_named_creator_source_spec(spec, source)
        }
        _ => false,
    }
}

pub(crate) fn materialize_named_creator_source_in_token(
    token: &mut CardDefinition,
    source: ObjectId,
) {
    for ability in &mut token.abilities {
        let crate::ability::AbilityKind::Static(static_ability) = &mut ability.kind else {
            continue;
        };
        let Some(mut model) = static_ability.compiled_model().cloned() else {
            continue;
        };
        let ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } =
            &mut model.payload
        else {
            continue;
        };
        let power_changed = materialize_named_creator_source_in_value(power, source);
        let toughness_changed = materialize_named_creator_source_in_value(toughness, source);
        if power_changed || toughness_changed {
            *static_ability = crate::static_abilities::StaticAbility::from_model(model);
        }
    }
}

/// The objects the resolving ability exiled to pay its cost, its own source
/// included when the source was exiled that way. A token created by that
/// ability remembers them ("all triggered abilities of the exiled cards").
fn cost_exiled_objects(
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<crate::snapshot::ObjectSnapshot> {
    let mut tags: Vec<_> = ctx
        .tagged_objects
        .keys()
        .filter(|tag| tag.as_str().starts_with("exile_cost_"))
        .cloned()
        .collect();
    if tags.is_empty() {
        return Vec::new();
    }
    tags.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    let mut objects: Vec<crate::snapshot::ObjectSnapshot> = Vec::new();
    let source_left_battlefield = game
        .object(ctx.source)
        .is_none_or(|object| object.zone != Zone::Battlefield);
    if source_left_battlefield && let Some(source) = ctx.source_snapshot.clone() {
        objects.push(source);
    }
    for tag in tags {
        for snapshot in ctx.tagged_objects.get(&tag).into_iter().flatten() {
            if objects
                .iter()
                .all(|existing| existing.stable_id != snapshot.stable_id)
            {
                objects.push(snapshot.clone());
            }
        }
    }
    objects
}

struct TokenProposal {
    effect: CreateTokenEffect,
    resolved_token: CardDefinition,
    token_preview: Option<crate::object::Object>,
    controller: crate::ids::PlayerId,
    count: u32,
    prepared: Option<crate::events::processing::PreparedTokenCreation>,
    charge_instruction: bool,
    instruction: Option<super::resources::TokenInstructionPermit>,
}
impl std::fmt::Debug for TokenProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenProposal")
            .field("controller", &self.controller)
            .field("count", &self.count)
            .finish_non_exhaustive()
    }
}
fn prepare_token_proposal(
    effect: &CreateTokenEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<TokenProposal, ExecutionError> {
    let controller_id =
        crate::effects::helpers::resolve_player_filter(game, &effect.controller, ctx)?;
    // CR 800.4b/800.4d: no token is created under the control of, or owned
    // by, a player who has left the game.
    if !game
        .player(controller_id)
        .is_some_and(|player| player.is_in_game())
    {
        return Ok(TokenProposal {
            effect: effect.clone(),
            resolved_token: effect.token.clone(),
            token_preview: None,
            controller: controller_id,
            count: 0,
            prepared: None,
            charge_instruction: false,
            instruction: None,
        });
    }
    let base_count = resolve_value(game, &effect.count, ctx)?.max(0) as u32;
    let mut resolved_token = effect.token.clone();
    if effect.use_source_chosen_color
        && let Some(color) = game.chosen_color(ctx.source)
    {
        resolved_token.card.color_indicator = Some(crate::color::ColorSet::from(color));
    }
    if effect.use_source_chosen_creature_type
        && let Some(subtype) = game.chosen_creature_type(ctx.source)
        && !resolved_token.card.subtypes.contains(&subtype)
    {
        resolved_token.card.subtypes.push(subtype);
    }
    materialize_named_creator_source_in_token(&mut resolved_token, ctx.source);
    let token_preview = crate::object::Object::from_token_definition(
        ObjectId::from_raw(0),
        &resolved_token,
        controller_id,
    );
    Ok(TokenProposal {
        effect: effect.clone(),
        resolved_token,
        token_preview: Some(token_preview),
        controller: controller_id,
        count: base_count,
        prepared: None,
        charge_instruction: false,
        instruction: None,
    })
}
impl crate::effects::SimultaneousEffectProposal for TokenProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.charge_instruction && self.instruction.is_none() {
            let (_, meter) = game.begin_token_resource_scope();
            self.instruction = Some(super::resources::TokenInstructionPermit::charge(meter)?);
        }
        let _phase = self
            .instruction
            .as_ref()
            .map(|permit| permit.enter_phase())
            .transpose()?;
        game.clear_pending_decision_controllers();
        self.prepared = Some(crate::events::processing::prepare_token_creation_deferred(
            game,
            self.controller,
            self.count,
            self.token_preview.clone(),
            ctx.cause.clone(),
            ctx,
        )?);
        Ok(())
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        commit_token_proposal(*self, game, ctx, true)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        commit_token_proposal(*self, game, ctx, false).map(|commit| commit.outcome)
    }
}
struct TokenCompletion {
    instruction: Option<super::resources::TokenInstructionPermit>,
    entries: Option<
        Vec<(
            ObjectId,
            crate::events::processing::PreparedEventOutcome<
                crate::effects::zones::AppliedZoneChange,
            >,
        )>,
    >,
    frozen: Option<crate::effects::zones::FrozenZoneChangeReceipts>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
}
impl crate::effects::SimultaneousEffectCompletion for TokenCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        let entries = self
            .entries
            .take()
            .ok_or_else(|| ExecutionError::InternalError("token entries already frozen".into()))?;
        self.frozen = Some(crate::effects::zones::freeze_zone_change_receipts(
            game, entries,
        ));
        Ok(())
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        let _phase = self
            .instruction
            .as_ref()
            .map(|permit| permit.enter_phase())
            .transpose()?;
        let frozen = self.frozen.ok_or_else(|| {
            ExecutionError::InternalError("token completion requires the original batch".into())
        })?;
        let original =
            crate::effects::zones::finish_zone_change_receipts_frozen(game, ctx, original, frozen)?;
        crate::effects::replacement::execute_deferred_replacement_programs(
            game,
            ctx,
            original,
            self.programs,
        )
    }
}
fn execute_token_instruction(
    effect: &CreateTokenEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    commit_token_proposal(prepare_token_proposal(effect, game, ctx)?, game, ctx, false)
        .map(|commit| commit.outcome)
}
fn commit_token_proposal(
    mut proposal: TokenProposal,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    defer_additions: bool,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    use crate::effects::SimultaneousEffectProposal;
    use crate::events::processing::PreparedTokenCreation;
    if proposal.prepared.is_none() {
        proposal.prepare_original(game, ctx)?;
    }
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::SimultaneousEffectCommit::finished(
            EffectOutcome::count(0),
        ));
    }
    let phase = proposal
        .instruction
        .as_ref()
        .map(|permit| permit.enter_phase())
        .transpose()?;
    let mut committed = match proposal.prepared.take().unwrap() {
        PreparedTokenCreation::Finished { outcome, programs } => {
            crate::effects::SimultaneousEffectCommit {
                outcome,
                completion: Some(Box::new(TokenCompletion {
                    instruction: proposal.instruction.take(),
                    entries: Some(Vec::new()),
                    frozen: None,
                    programs,
                })),
            }
        }
        PreparedTokenCreation::Proceed {
            event,
            provenance,
            programs,
        } => {
            let preview = proposal.token_preview.ok_or_else(|| {
                ExecutionError::InternalError("token original lost its preview".into())
            })?;
            ctx.provenance = provenance;
            commit_token_original(
                &proposal.effect,
                &proposal.resolved_token,
                preview,
                game,
                ctx,
                event,
                programs,
                proposal.instruction.take(),
            )?
        }
    };
    drop(phase);
    if !defer_additions && let Some(mut completion) = committed.completion.take() {
        game.freeze_completed_entry_events(committed.outcome.events.iter_mut())?;
        completion.freeze(game)?;
        committed.outcome = completion.complete(game, ctx, committed.outcome)?;
    }
    Ok(committed)
}
fn commit_token_original(
    effect: &CreateTokenEffect,
    resolved_token: &CardDefinition,
    token_preview: crate::object::Object,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    replacement: crate::events::CreateTokensEvent,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    instruction: Option<super::resources::TokenInstructionPermit>,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    let controller_id = replacement.controller;
    let token_preview = replacement.token.clone().unwrap_or(token_preview);
    game.reserve_token_creation(replacement.total_count())?;
    let count = replacement.count as usize;
    let cleanup_options = TokenCleanupOptions::new(
        effect.exile_at_end_of_combat,
        effect.sacrifice_at_end_of_combat,
        effect.sacrifice_at_next_end_step,
        effect.exile_at_next_end_step,
        effect.next_end_step_player.clone(),
    );
    // An attack target that can't be resolved (for example no defending
    // player in this context) doesn't stop the tokens being created; they
    // fall back to the ordinary CR 508.4 choice.
    let (configured_attack_player, attack_player_only) = match &effect.attack_target_mode {
        Some(CopyAttackTargetMode::Player(player_filter)) => {
            let player = resolve_player_filter(game, player_filter, ctx).ok();
            (player, player.is_some())
        }
        Some(CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player_filter)) => {
            (resolve_player_filter(game, player_filter, ctx).ok(), false)
        }
        None => (None, false),
    };
    let entry_options =
        TokenEntryOptions::new(effect.enters_attacking && configured_attack_player.is_none());

    // CR 509.4: "a token that's blocking <that creature>" names what it
    // blocks; resolve the attacker once for every token.
    let blocking_attacker = match &effect.enters_blocking {
        Some(spec) => crate::effects::helpers::resolve_objects_for_effect(game, ctx, spec)
            .ok()
            .and_then(|ids| ids.first().copied()),
        None => None,
    };
    let mut created_ids = super::resources::buffer(count)?;
    let mut events = super::resources::buffer(count)?;
    let mut entry_receipts = Vec::new();
    let pending_start = game.effect_store.pending_trigger_events.len();
    let cost_exiled = cost_exiled_objects(game, ctx);
    // CR 607.2a: a token whose ability returns "the exiled card" is linked
    // to the cards its creating resolution exiled with the source.
    let linked_exiles: Vec<ObjectId> = if effect.link_source_exiled_this_resolution {
        ctx.get_tagged_all(ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG)
            .map(|snapshots| {
                snapshots
                    .iter()
                    .map(|snapshot| snapshot.object_id)
                    .filter(|id| game.object(*id).is_some_and(|obj| obj.zone == Zone::Exile))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    for _ in 0..count {
        let id = game.new_object_id();
        let mut token_obj = game.object_from_token_definition(id, &resolved_token, controller_id);
        token_obj.zone = Zone::Command;
        if !cost_exiled.is_empty() {
            token_obj.cast_tagged_objects.insert(
                crate::tag::TagKey::from(crate::tag::COST_EXILED_TAG),
                cost_exiled.clone(),
            );
        }
        let token_is_creature = token_obj.is_creature();

        game.commit_token_resource_slot()?;
        game.add_object(token_obj);
        let entry_result = game.move_object_with_etb_processing_with_cause_and_entry_options(
            id,
            Zone::Battlefield,
            ctx.cause.clone(),
            &mut ctx.decision_maker,
            effect.enters_tapped,
            !effect.suppress_aura_attachment_choice,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                EffectOutcome::with_objects(Vec::new()),
            ));
        }
        let Some(entry_result) = super::lifecycle::retain_token_entry_receipt(
            game,
            id,
            entry_result,
            &mut entry_receipts,
        )?
        else {
            game.remove_object(id);
            continue;
        };
        let entered_id = entry_result.new_id;
        created_ids.push(entered_id);
        for &exiled_id in &linked_exiles {
            game.add_exiled_with_source_link(entered_id, exiled_id);
        }
        let entered_battlefield = game
            .object(entered_id)
            .is_some_and(|obj| obj.zone == Zone::Battlefield);

        if entered_battlefield {
            let entered_is_creature = game.current_is_creature(entered_id);
            let tracks_creature_etb = entered_is_creature || token_is_creature;
            apply_token_battlefield_entry(
                game,
                ctx,
                entered_id,
                controller_id,
                tracks_creature_etb,
                entry_options,
                Zone::Command,
                entry_result.enters_tapped,
                &mut events,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    EffectOutcome::with_objects(Vec::new()),
                ));
            }

            // CR 506.3a/b/f, 508.4: only a creature controlled by an
            // attacking player, during combat, becomes attacking.
            if let Some(attack_player) = configured_attack_player
                && crate::effects::combat::can_enter_attacking(game, entered_id)
            {
                let chosen_target = if attack_player_only {
                    game.player(attack_player)
                        .is_some_and(|player| player.is_in_game())
                        .then_some(AttackTarget::Player(attack_player))
                } else {
                    let targets = attack_targets_for_player(game, attack_player);
                    (!targets.is_empty())
                        .then(|| choose_attack_target(game, ctx, attack_player, &targets))
                        .flatten()
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::SimultaneousEffectCommit::finished(
                        EffectOutcome::with_objects(Vec::new()),
                    ));
                }
                if let Some(chosen_target) = chosen_target {
                    game.add_entering_attacker(entered_id, chosen_target);
                }
            }

            if let Some(attacker) = blocking_attacker {
                crate::effects::combat::put_onto_battlefield_blocking(game, entered_id, attacker);
            }

            schedule_token_cleanup(
                game,
                ctx,
                entered_id,
                controller_id,
                cleanup_options.clone(),
            )?;
        }
    }

    let mut actual_creation = replacement.clone();
    actual_creation.count = created_ids.len() as u32;
    actual_creation.token = Some(token_preview);

    let additional_ids = create_replacement_additional_tokens(
        game,
        ctx,
        controller_id,
        &mut actual_creation,
        &super::lifecycle::AdditionalTokenInstructions {
            enters_tapped: effect.enters_tapped,
            suppress_aura_attachment_choice: effect.suppress_aura_attachment_choice,
            entry: entry_options,
            attack_player: configured_attack_player,
            attack_player_only,
            blocking_attacker,
            cleanup: Some(cleanup_options.clone()),
            linked_exiles: linked_exiles.clone(),
            ..Default::default()
        },
        &mut events,
        &mut entry_receipts,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::SimultaneousEffectCommit::finished(
            EffectOutcome::with_objects(Vec::new()),
        ));
    }
    created_ids.extend(additional_ids);
    super::lifecycle::publish_created_token_groups(game, ctx, actual_creation, &mut events);

    if created_ids.len() > 1 {
        let batch_objects = created_ids.clone();
        let removed_events =
            game.remove_pending_trigger_events_matching_from(pending_start, |event| {
                event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|zone_change| {
                        zone_change.from == Zone::Command
                            && zone_change.to == Zone::Battlefield
                            && zone_change
                                .objects
                                .iter()
                                .all(|object_id| batch_objects.contains(object_id))
                    })
            });
        if !removed_events.is_empty() {
            let cause = removed_events
                .iter()
                .find_map(|event| {
                    event
                        .downcast::<crate::events::zones::ZoneChangeEvent>()
                        .map(|zone_change| zone_change.cause.clone())
                })
                .unwrap_or_else(crate::events::cause::EventCause::effect);
            let snapshots = removed_events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
                .flat_map(|zone_change| zone_change.snapshots().iter().cloned())
                .collect();
            let event = crate::events::zones::ZoneChangeEvent::batch_with_snapshots(
                created_ids.clone(),
                Zone::Command,
                Zone::Battlefield,
                cause,
                snapshots,
            );
            game.queue_trigger_event(
                ctx.provenance,
                crate::triggers::TriggerEvent::new_with_provenance(event, ctx.provenance),
            );
        }
    }

    let created_stable_ids: Vec<_> = created_ids
        .iter()
        .filter_map(|id| game.object(*id).map(|obj| obj.stable_id))
        .collect();
    if !created_stable_ids.is_empty() {
        game.record_ui_effect_event(
            "tokens_created",
            Some(controller_id),
            None,
            created_stable_ids,
            Some(created_ids.len() as i64),
            Some(effect.token.card.name.to_string()),
        );
    }

    let original = EffectOutcome::with_objects(created_ids.clone())
        .with_result_objects(created_ids.clone())
        .with_events(events)
        .with_affected_objects_from_game(game, created_ids);
    Ok(crate::effects::SimultaneousEffectCommit {
        outcome: original,
        completion: Some(Box::new(TokenCompletion {
            instruction,
            entries: Some(entry_receipts),
            frozen: None,
            programs,
        })),
    })
}

impl EffectExecutor for CreateTokenEffect {
    fn visit_card_definitions(&self, visitor: &mut dyn FnMut(&CardDefinition)) {
        visitor(&self.token);
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let mut proposal = prepare_token_proposal(self, game, ctx)?;
        proposal.charge_instruction = true;
        Ok(Box::new(proposal))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::lifecycle::execute_token_instruction_atomically(game, ctx, |game, ctx| {
            execute_token_instruction(self, game, ctx)
        })
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.controller_target.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to create tokens"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::cards::definitions::tayam_luminous_enigma;
    use crate::cards::tokens::treasure_token_definition;
    use crate::color::{Color, ColorSet};
    use crate::ids::{CardId, PlayerId};
    use crate::object::{CounterType, ObjectKind};
    use crate::static_abilities::StaticAbility;
    use crate::test_prelude::*;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn pending_token_replacement_creates_neither_tokens_nor_copies() {
        use crate::decision::DecisionMaker;
        use crate::decisions::context::SelectOptionsContext;
        use crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        struct Choice {
            pause: bool,
            pending: bool,
        }
        impl DecisionMaker for Choice {
            fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
                self.pending = self.pause;
                if self.pause { vec![] } else { vec![1] }
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for case in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
            for _ in 0..2 {
                let replacement_source =
                    game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
                game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        replacement_source,
                        alice,
                        WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
                        ReplacementAction::Double,
                    ),
                );
            }
            let effect: Box<dyn EffectExecutor> = if case == 1 {
                Box::new(crate::effects::CreateTokenCopyEffect::one(
                    ChooseSpec::SpecificObject(source),
                ))
            } else if case == 2 {
                Box::new(crate::effects::IncubateEffect::you(2, 1))
            } else {
                Box::new(CreateTokenEffect::one(soldier_token()))
            };
            let initial = game.battlefield.len();
            let initial_objects = game.objects_in_deterministic_order().len();
            let mut dm = Choice {
                pause: true,
                pending: false,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert!(ctx.decision_maker.awaiting_choice());
            assert_eq!(
                game.battlefield.len(),
                initial,
                "pending creation cannot commit; case={case}"
            );
            assert!(outcome.events.is_empty());
            assert_eq!(
                game.objects_in_deterministic_order().len(),
                initial_objects,
                "pending creation cannot allocate provisional tokens; case={case}"
            );
            drop(ctx);
            dm.pause = false;
            dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(
                game.battlefield.len(),
                initial + 4,
                "both doublers apply once; case={case}"
            );
        }
    }

    fn soldier_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Soldier")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .color_indicator(ColorSet::from(Color::White))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn goblin_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Goblin")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin])
            .color_indicator(ColorSet::from(Color::Red))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn zombie_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn beast_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Beast")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Beast])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn xorn_definition() -> CardDefinition {
        let oracle = "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.";
        CardDefinitionBuilder::new(CardId::new(), "Xorn")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elemental])
            .oracle_text(oracle)
            .with_ability(Ability::static_ability(
                StaticAbility::add_token_creation_replacement(
                    PlayerFilter::You,
                    ObjectFilter::default().with_subtype(Subtype::Treasure),
                    ironsmith_core::AdditionalTokenKind::Treasure,
                    1,
                    oracle.to_string(),
                ),
            ))
            .build()
    }

    fn fancy_treasure_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Fancy Treasure")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build()
    }

    #[test]
    fn xorn_strict_parser_and_compiled_text_regression() {
        let def = xorn_definition();
        let replacement = def
            .abilities
            .iter()
            .find_map(|ability| {
                match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::AddTokenCreationReplacement =>
                {
                    Some(static_ability)
                }
                _ => None,
            }
            })
            .expect("Xorn should carry the typed token-creation replacement");
        assert_eq!(
            replacement.display(),
            "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token."
        );
    }

    #[test]
    fn create_token_records_created_objects_as_affected_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .expect("tokens should be created");

        let affected = outcome
            .affected_objects()
            .expect("created tokens should be affected objects");
        assert_eq!(affected.len(), 2);
        let memory = outcome
            .affected_object_memory()
            .expect("created token LKI should be recorded");
        assert_eq!(memory.len(), 2);
        assert!(memory.iter().all(|m| m.controller == alice));
        assert!(memory.iter().all(|m| m.is_token));
    }

    #[test]
    fn test_create_single_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Soldier");
            assert_eq!(token.kind, ObjectKind::Token);
            assert!(token.is_creature());
            assert_eq!(token.power(), Some(1));
            assert_eq!(token.toughness(), Some(1));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn create_token_applies_source_chosen_color_and_creature_type() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_chosen_color(source, Color::Blue);
        game.set_chosen_creature_type(source, Subtype::Goblin);

        let blueprint = CardDefinitionBuilder::new(CardId::new(), "Creature")
            .token()
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::one(blueprint)
            .with_source_chosen_color()
            .with_source_chosen_creature_type()
            .execute(&mut game, &mut ctx)
            .expect("chosen-characteristic token should be created");

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected created token ids");
        };
        let token = game.object(ids[0]).expect("created token");
        assert_eq!(token.colors(), ColorSet::BLUE);
        assert!(token.subtypes.contains(&Subtype::Goblin), "{token:#?}");
    }

    #[test]
    fn named_source_token_cda_tracks_the_creating_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let slime = CounterType::Named("slime".into());
        let creator_definition = CardDefinitionBuilder::new(CardId::new(), "Slime Foundry")
            .card_types(vec![CardType::Enchantment])
            .build();
        let creator =
            game.create_object_from_definition(&creator_definition, alice, Zone::Battlefield);
        let other_creator =
            game.create_object_from_definition(&creator_definition, alice, Zone::Battlefield);
        game.add_counters(creator, slime, 2)
            .expect("creator should receive slime counters");

        let named_source = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                SourceReferenceSurface::FullName("Slime Foundry".to_string()),
            ),
        );
        let count = crate::effect::Value::CountersOn(Box::new(named_source), Some(slime));
        let token = CardDefinitionBuilder::new(CardId::new(), "Ooze")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Ooze])
            .power_toughness(PowerToughness::fixed(0, 0))
            .with_ability(Ability::static_ability(StaticAbility::from_model(
                crate::static_abilities::CompiledStaticAbility::characteristic_defining_pt(
                    count.clone(),
                    count,
                ),
            )))
            .build();
        let mut ctx = ExecutionContext::new_default(creator, alice);
        let result = CreateTokenEffect::one(token)
            .execute(&mut game, &mut ctx)
            .expect("creator-bound Ooze should be created");
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected a created token");
        };
        let [token_id] = ids.as_slice() else {
            panic!("expected exactly one created token: {ids:#?}");
        };

        game.refresh_continuous_state();
        assert_eq!(game.current_power(*token_id), Some(2));
        assert_eq!(game.current_toughness(*token_id), Some(2));

        game.add_counters(other_creator, slime, 4)
            .expect("unrelated permanent should receive slime counters");
        game.refresh_continuous_state();
        assert_eq!(
            game.current_power(*token_id),
            Some(2),
            "a similarly named permanent must not replace the creating object"
        );

        game.add_counters(creator, slime, 1)
            .expect("creator should receive another slime counter");
        game.refresh_continuous_state();
        assert_eq!(game.current_power(*token_id), Some(3));
        assert_eq!(game.current_toughness(*token_id), Some(3));

        let token_object = game.object(*token_id).expect("created token should exist");
        let materialized_value = token_object
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability) => {
                    let model = static_ability.compiled_model()?;
                    let ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt {
                        power,
                        ..
                    } = &model.payload
                    else {
                        return None;
                    };
                    Some(power)
                }
                _ => None,
            })
            .expect("created token should retain its compiled CDA");
        let crate::effect::Value::CountersOn(spec, Some(counter_type)) =
            materialized_value.unhinted()
        else {
            panic!("expected creator counter value: {materialized_value:#?}");
        };
        assert_eq!(*counter_type, slime);
        assert_eq!(spec.base(), &ChooseSpec::SpecificObject(creator));
        assert_eq!(
            spec.source_reference_surface(),
            Some(&SourceReferenceSurface::FullName(
                "Slime Foundry".to_string()
            ))
        );
    }

    #[test]
    fn token_local_cda_source_is_not_rebound_to_the_creator() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creator_definition = CardDefinitionBuilder::new(CardId::new(), "Creator")
            .card_types(vec![CardType::Enchantment])
            .build();
        let creator =
            game.create_object_from_definition(&creator_definition, alice, Zone::Battlefield);
        game.add_counters(creator, CounterType::Charge, 3)
            .expect("creator should receive charge counters");

        let count = crate::effect::Value::CountersOnSource(CounterType::Charge);
        let token = CardDefinitionBuilder::new(CardId::new(), "Construct")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Construct])
            .power_toughness(PowerToughness::fixed(0, 0))
            .with_ability(Ability::static_ability(StaticAbility::from_model(
                crate::static_abilities::CompiledStaticAbility::characteristic_defining_pt(
                    count.clone(),
                    count,
                ),
            )))
            .build();
        let mut ctx = ExecutionContext::new_default(creator, alice);
        let result = CreateTokenEffect::one(token)
            .execute(&mut game, &mut ctx)
            .expect("token-local Construct should be created");
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected a created token");
        };
        let [token_id] = ids.as_slice() else {
            panic!("expected exactly one created token: {ids:#?}");
        };

        game.refresh_continuous_state();
        assert_eq!(
            game.current_power(*token_id),
            Some(0),
            "an unmarked source value must remain local to the token"
        );
        game.add_counters(*token_id, CounterType::Charge, 1)
            .expect("token should receive a charge counter");
        game.refresh_continuous_state();
        assert_eq!(game.current_power(*token_id), Some(1));
        assert_eq!(game.current_toughness(*token_id), Some(1));
    }

    #[test]
    fn test_create_multiple_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(goblin_token(), 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Goblin");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn create_token_replacement_doubles_tokens_created_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let doubler = CardDefinitionBuilder::new(CardId::new(), "Token Doubler")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_token_creation_replacement(
                    PlayerFilter::You,
                    "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".to_string(),
                ),
            ))
            .build();
        game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::one(soldier_token())
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|id| {
            game.object(*id)
                .is_some_and(|token| token.name == "Soldier" && token.kind == ObjectKind::Token)
        }));
    }

    #[test]
    fn create_token_replacement_does_not_double_other_players_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let doubler = CardDefinitionBuilder::new(CardId::new(), "Token Doubler")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_token_creation_replacement(
                    PlayerFilter::You,
                    "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".to_string(),
                ),
            ))
            .build();
        game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::new(soldier_token(), 1, PlayerFilter::Specific(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(game.controller_of(token), bob);
    }

    #[test]
    fn xorn_adds_one_treasure_token_to_your_treasure_creation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let xorn = xorn_definition();
        game.create_object_from_definition(&xorn, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::you(fancy_treasure_token(), 2)
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 3, "Xorn should add exactly one Treasure token");
        let fancy_count = ids
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|token| token.name == "Fancy Treasure")
            })
            .count();
        let normal_count = ids
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|token| token.name == "Treasure")
            })
            .count();
        assert_eq!(
            fancy_count, 2,
            "the original token batch should be preserved"
        );
        assert_eq!(normal_count, 1, "Xorn should add one normal Treasure token");
        assert!(ids.iter().all(|id| {
            game.object(*id)
                .is_some_and(|token| token.subtypes.contains(&Subtype::Treasure))
        }));
    }

    #[test]
    fn xorn_does_not_add_tokens_to_non_treasure_token_creation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let xorn = xorn_definition();
        game.create_object_from_definition(&xorn, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2, "Xorn should ignore non-Treasure tokens");
    }

    #[test]
    fn xorn_does_not_add_tokens_to_other_players_treasure_creation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let xorn = xorn_definition();
        game.create_object_from_definition(&xorn, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result =
            CreateTokenEffect::new(treasure_token_definition(), 2, PlayerFilter::Specific(bob))
                .execute(&mut game, &mut ctx)
                .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(
            ids.len(),
            2,
            "Xorn should only affect its controller's Treasure creation"
        );
        assert!(ids.iter().all(|id| {
            game.object(*id)
                .is_some_and(|token| game.controller_of(token) == bob)
        }));
    }

    #[test]
    fn create_token_preserves_exact_counts_above_500() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = CreateTokenEffect::you(soldier_token(), 501)
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 501);

        let result = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2);
        assert_eq!(game.battlefield.len(), 503);
    }

    #[test]
    fn test_create_zero_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(zombie_token(), 0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert!(ids.is_empty());
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_tracks_creature_etb() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(beast_token(), 2);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have tracked 2 creatures entering
        assert_eq!(
            game.turn_store
                .turn_history
                .creatures_entered_under_controller(alice),
            2
        );
    }

    #[test]
    fn test_create_token_for_other_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        // Use Specific instead of Opponent since Opponent requires targeting context
        let effect = CreateTokenEffect::new(spirit_token(), 1, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).unwrap();
            assert_eq!(game.controller_of(token), bob);
            assert_eq!(
                token.owner, bob,
                "the player who creates the token should own it"
            );
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn multiplayer_800_4b_d_does_not_create_tokens_for_player_who_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        assert!(
            game.leave_game(alice)
                .expect("checked designation/departure fixture")
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = CreateTokenEffect::you(soldier_token(), 2)
            .execute(&mut game, &mut ctx)
            .expect("the creation instruction should be skipped cleanly");

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected an empty object outcome");
        };
        assert!(ids.is_empty());
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn test_create_token_clone_box() {
        let effect = CreateTokenEffect::one(soldier_token());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenEffect"));
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn test_created_creature_token_gets_etb_replacement_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _tayam =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let created_id = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => {
                *ids.first().expect("expected created token")
            }
            other => panic!("expected created token object ids, got {other:?}"),
        };

        let token = game.object(created_id).expect("created token should exist");
        assert_eq!(
            token.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "token creature should get Tayam's additional vigilance counter on entry"
        );
    }
    #[test]
    fn token_instead_payload_executes_without_creating_original_tokens() {
        use crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for case in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
                    ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(3)]),
                ),
            );
            let effect: Box<dyn EffectExecutor> = match case {
                1 => Box::new(crate::effects::CreateTokenCopyEffect::one(
                    ChooseSpec::SpecificObject(source),
                )),
                2 => Box::new(crate::effects::IncubateEffect::you(2, 1)),
                _ => Box::new(CreateTokenEffect::one(soldier_token())),
            };
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(
                game.player(alice).unwrap().life,
                23,
                "payload must execute for token path {case}"
            );
            assert_eq!(
                game.battlefield.len(),
                1,
                "original token event must not commit for path {case}"
            );
            assert_eq!(outcome.count_or_zero(), 0);
            assert!(outcome.events.iter().any(|event| {
                event
                    .downcast::<crate::events::LifeGainEvent>()
                    .is_some_and(|gain| gain.amount == 3)
            }));
            assert!(outcome.events.iter().all(|event| {
                event
                    .downcast::<crate::events::CreateTokensEvent>()
                    .is_none()
            }));
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
            assert!(game.take_pending_trigger_events().iter().all(|event| {
                event
                    .downcast::<crate::events::CreateTokensEvent>()
                    .is_none()
            }));
        }
    }

    fn token_effect_for_case(case: u8, source: ObjectId, count: i32) -> Box<dyn EffectExecutor> {
        match case {
            1 => Box::new(crate::effects::CreateTokenCopyEffect::new(
                ChooseSpec::SpecificObject(source),
                count,
                PlayerFilter::You,
            )),
            2 => Box::new(crate::effects::IncubateEffect::you(2, count)),
            _ => Box::new(CreateTokenEffect::new(
                soldier_token(),
                count,
                PlayerFilter::You,
            )),
        }
    }

    #[test]
    fn token_payload_preserves_modified_event_and_suppresses_prior_replacements() {
        use crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for case in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
            for action in [
                ReplacementAction::Double,
                ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(crate::effect::Value::EventValue(
                        crate::effect::EventValueSpec::Amount,
                    )),
                    crate::effect::Effect::new(CreateTokenEffect::one(soldier_token())),
                ]),
            ] {
                game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        source,
                        alice,
                        WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
                        action,
                    ),
                );
            }
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let outcome = token_effect_for_case(case, source, 1)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(
                game.player(alice).unwrap().life,
                22,
                "payload must read the doubled proposal for path {case}"
            );
            assert_eq!(
                game.battlefield.len(),
                2,
                "nested token is created exactly once without reapplying either replacement for path {case}"
            );
            let observed_tokens = outcome
                .execution_facts
                .iter()
                .find_map(|fact| match fact {
                    crate::effect::ExecutionFact::ResultObjects(ids) => Some(ids.as_slice()),
                    _ => None,
                })
                .expect("complete observation retains the replacement-created token");
            assert_eq!(observed_tokens.len(), 1);
            assert!(
                outcome.result_objects().is_none_or(|ids| ids.is_empty()),
                "the replaced original instruction creates no token"
            );
            assert!(
                !outcome
                    .affected_object_memory()
                    .unwrap_or(&[])
                    .iter()
                    .any(|memory| observed_tokens.contains(&memory.object_id))
            );
            assert_eq!(
                game.object(observed_tokens[0]).unwrap().zone,
                Zone::Battlefield
            );
            assert_eq!(outcome.count_or_zero(), 0);
            assert_eq!(
                outcome
                    .events
                    .iter()
                    .filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                    .count(),
                1
            );
            let created = outcome
                .events
                .iter()
                .filter_map(|event| {
                    event
                        .downcast::<crate::events::CreateTokensEvent>()
                        .cloned()
                })
                .collect::<Vec<_>>();
            assert_eq!(created.len(), 1);
            assert_eq!(created[0].count, 1);
        }
    }

    #[test]
    fn token_payload_error_restores_prior_payload_and_one_shot() {
        use crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for case in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
                    ReplacementAction::Instead(vec![
                        crate::effect::Effect::gain_life(3),
                        crate::effect::Effect::gain_life(crate::effect::Value::X),
                    ]),
                ),
            );
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let error = token_effect_for_case(case, source, 1)
                .execute(&mut game, &mut ctx)
                .unwrap_err();
            assert!(matches!(error, ExecutionError::UnresolvableValue(_)));
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.battlefield.len(), 1);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }

    #[test]
    fn token_payload_pause_restores_prior_payload_then_replays_once() {
        use crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.calls += 1;
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for case in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
                    ReplacementAction::Instead(vec![
                        crate::effect::Effect::gain_life(3),
                        crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(1)]),
                    ]),
                ),
            );
            game.take_pending_trigger_events();
            let effect = token_effect_for_case(case, source, 1);
            let mut dm = Answers {
                pause: true,
                pending: false,
                calls: 0,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert!(ctx.decision_maker.awaiting_choice());
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.battlefield.len(), 1);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(outcome.events.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            drop(ctx);
            assert_eq!(dm.calls, 1);
            dm.pause = false;
            dm.pending = false;
            dm.calls = 0;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 24);
            assert_eq!(game.battlefield.len(), 1);
            assert_eq!(
                outcome
                    .events
                    .iter()
                    .filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                    .count(),
                2
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
            drop(ctx);
            assert_eq!(dm.calls, 1);
        }
    }

    #[test]
    fn later_token_entry_pause_restores_earlier_tokens_payments_and_allocations() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        struct Answers {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.calls += 1;
                self.pending = self.pause && self.calls == 2;
                !self.pending
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for case in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&soldier_token(), alice, Zone::Battlefield);
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                    ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost: 2 },
                ),
            );
            game.take_pending_trigger_events();
            let first_allocated_id = game.next_object_id_counter();
            let effect = token_effect_for_case(case, source, 3);
            let mut dm = Answers {
                pause: true,
                pending: false,
                calls: 0,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert!(ctx.decision_maker.awaiting_choice());
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.battlefield.len(), 1);
            assert_eq!(game.next_object_id_counter(), first_allocated_id);
            assert!(outcome.events.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            drop(ctx);
            assert_eq!(dm.calls, 2);
            dm.pause = false;
            dm.pending = false;
            dm.calls = 0;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 14);
            assert_eq!(game.battlefield.len(), 4);
            assert_eq!(outcome.result_objects().unwrap().len(), 3);
            drop(ctx);
            assert_eq!(dm.calls, 3);
        }
    }
}

#[cfg(test)]
mod replacement_token_entry_owner_contract_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, PlayerId};
    use crate::object::{CounterType, ObjectKind};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    struct Answers {
        pause: bool,
        pending: bool,
        calls: usize,
        binding: bool,
        base: u32,
    }
    impl DecisionMaker for Answers {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.calls += 1;
            let tokens = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
                .collect::<Vec<_>>();
            assert_eq!(tokens.len(), 1);
            assert_eq!(
                game.current_controller(tokens[0]),
                Some(PlayerId::from_index(0))
            );
            if self.binding {
                assert_eq!(
                    game.counter_count(tokens[0], CounterType::PlusOnePlusOne),
                    self.base + 1
                );
            }
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn check(kind: u8, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = crate::CardDefinitionBuilder::new(CardId::new(), "Token owner fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let parent = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let actions = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![
                Effect::new(crate::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    1,
                    ChooseSpec::tagged("it"),
                )),
                Effect::may(vec![Effect::gain_life(0)]),
            ],
            _ => vec![
                Effect::gain_life(3),
                Effect::may(vec![Effect::gain_life(4)]),
            ],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                    ObjectFilter::default(),
                ),
                ReplacementAction::Additionally(actions),
            ),
        );
        game.take_pending_trigger_events();
        let ids = game.next_object_id_counter();
        let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers {
            pause: mode == 2,
            pending: false,
            calls: 0,
            binding: mode == 3,
            base: if kind == 2 { 2 } else { 0 },
        };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
        ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let effect: Box<dyn EffectExecutor> = match kind {
            1 => Box::new(crate::effects::CreateTokenCopyEffect::one(
                ChooseSpec::SpecificObject(parent),
            )),
            2 => Box::new(crate::effects::IncubateEffect::you(2, 1)),
            _ => Box::new(CreateTokenEffect::one(
                crate::CardDefinitionBuilder::new(CardId::new(), "Created fixture")
                    .token()
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                    .build(),
            )),
        };
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
        } else if mode == 2 {
            assert!(ctx.decision_maker.awaiting_choice());
            assert!(result.unwrap().events.is_empty());
        } else {
            let outcome = result.unwrap();
            let arrived = outcome.explicit_objects().unwrap();
            assert_eq!(arrived.len(), 1);
            assert!(game.battlefield.contains(&arrived[0]));
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.player(bob).unwrap().life,
                if mode == 3 { 20 } else { 27 }
            );
            if mode == 3 {
                assert_eq!(
                    game.counter_count(arrived[0], CounterType::PlusOnePlusOne),
                    if kind == 2 { 3 } else { 1 }
                );
            } else {
                assert_eq!(
                    outcome
                        .events
                        .iter()
                        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                        .map(|event| (event.player, event.amount))
                        .collect::<Vec<_>>(),
                    vec![(bob, 3), (bob, 4)]
                );
            }
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
        assert_eq!(ctx.source, parent);
        assert_eq!(ctx.controller, alice);
        assert_eq!(
            ctx.get_tagged_all("it").unwrap()[0].object_id,
            sentinel.object_id
        );
        assert_eq!(game.counter_count(parent, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(game.command_zone.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 0 || mode == 3 {
            assert_eq!(dm.calls, 1);
        }
        if mode == 2 {
            assert_eq!(dm.calls, 1);
            dm.pause = false;
            dm.pending = false;
            let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(outcome.explicit_objects().unwrap().len(), 1);
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(!ctx.decision_maker.awaiting_choice());
            drop(ctx);
            assert_eq!(dm.calls, 2);
        }
    }
    #[test]
    fn ordinary_additions() {
        check(0, 0);
    }
    #[test]
    fn ordinary_error() {
        check(0, 1);
    }
    #[test]
    fn ordinary_pending_replay() {
        check(0, 2);
    }
    #[test]
    fn ordinary_binding() {
        check(0, 3);
    }
    #[test]
    fn copied_additions() {
        check(1, 0);
    }
    #[test]
    fn copied_error() {
        check(1, 1);
    }
    #[test]
    fn copied_pending_replay() {
        check(1, 2);
    }
    #[test]
    fn copied_binding() {
        check(1, 3);
    }
    #[test]
    fn incubated_additions() {
        check(2, 0);
    }
    #[test]
    fn incubated_error() {
        check(2, 1);
    }
    #[test]
    fn incubated_pending_replay() {
        check(2, 2);
    }
    #[test]
    fn incubated_binding() {
        check(2, 3);
    }
}

#[cfg(test)]
mod surviving_added_token_group_tests {
    use super::*;
    use crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::target::{ObjectFilter, PlayerFilter};
    fn check_creation(mode: u8) {
        struct Ordered(crate::ids::ObjectId, crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for Ordered {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                let option = [self.0, self.1]
                    .into_iter()
                    .find_map(|source| {
                        ctx.options
                            .iter()
                            .find(|option| option.legal && option.object_id == Some(source))
                    })
                    .or_else(|| ctx.options.iter().find(|option| option.legal))
                    .unwrap();
                vec![option.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source_def = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Token replacement source",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
        let adder = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let remover = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let doubler = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let reviver = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let creature_matcher = || {
            WouldCreateTokensUnderControlMatcher::new(PlayerFilter::Any)
                .with_token_filter(ObjectFilter::creature())
        };
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                adder,
                alice,
                creature_matcher(),
                ReplacementAction::AddTokens {
                    token: ironsmith_core::AdditionalTokenKind::Treasure,
                    count: 1,
                },
            ),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                remover,
                alice,
                creature_matcher(),
                ReplacementAction::Modify(EventModification::ReduceToZero),
            ),
        );
        let unused_creature = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                reviver,
                alice,
                creature_matcher(),
                ReplacementAction::Modify(EventModification::Add(1)),
            ),
        );
        let used_treasure = (mode != 0).then(|| {
            let matcher = WouldCreateTokensUnderControlMatcher::new(PlayerFilter::Any);
            let (matcher, action) = if mode == 1 {
                (
                    matcher.with_token_filter(
                        ObjectFilter::default().with_subtype(crate::types::Subtype::Treasure),
                    ),
                    ReplacementAction::Double,
                )
            } else {
                (
                    matcher,
                    ReplacementAction::Modify(EventModification::Add(1)),
                )
            };
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(doubler, alice, matcher, action),
            )
        });
        let token = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Original creature token",
        )
        .token()
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .build();
        let mut chooser = Ordered(adder, remover);
        let mut ctx = ExecutionContext::new(adder, alice, &mut chooser);
        let outcome = CreateTokenEffect::you(token, 1)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let ids = outcome.result_objects().unwrap();
        assert!(
            ids.iter().all(|id| game
                .object(*id)
                .unwrap()
                .subtypes
                .contains(&crate::types::Subtype::Treasure)),
            "removed original creature group must not be revived by a stale primary-token filter"
        );
        assert_eq!(
            ids.len(),
            if mode != 0 { 2 } else { 1 },
            "positive added Treasure group survives removal of original group and remains replaceable"
        );
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.object(**id).unwrap().kind == crate::object::ObjectKind::Token)
                .count(),
            ids.len()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(unused_creature)
                .is_some()
        );
        if let Some(shield) = used_treasure {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
    }
    #[test]
    fn added_treasure_group_is_doubled_after_original_creature_group_is_removed() {
        check_creation(1);
    }
    #[test]
    fn removed_primary_creature_group_does_not_consume_its_unused_replacement() {
        check_creation(0);
    }
    #[test]
    fn unfiltered_extra_token_uses_surviving_group_without_reviving_removed_primary() {
        check_creation(2);
    }
    #[test]
    fn combined_count_adjustment_skips_empty_primary_group() {
        let alice = crate::ids::PlayerId::from_index(0);
        let event = crate::events::CreateTokensEvent::with_token_cause(
            alice,
            0,
            crate::events::tokens::additional_token_object(
                ironsmith_core::AdditionalTokenKind::Squirrel,
                alice,
            ),
            crate::events::cause::EventCause::effect(),
        )
        .with_additional_tokens(ironsmith_core::AdditionalTokenKind::Treasure, 1)
        .unwrap();
        let modified = event
            .adjusted_covered_total(|_| true, |count| u128::from(count) + 1)
            .unwrap();
        assert_eq!(
            modified.count, 0,
            "a removed primary group is not a destination for additional tokens"
        );
        assert_eq!(
            modified.additional_tokens,
            vec![(ironsmith_core::AdditionalTokenKind::Treasure, 2)]
        );
    }
    #[test]
    fn positive_added_group_remains_matchable_without_matching_zero_primary_group() {
        use crate::events::ReplacementMatcher;
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let ctx = crate::events::EventContext::for_controller(alice, &game);
        let event = crate::events::CreateTokensEvent::with_token_cause(
            alice,
            0,
            crate::events::tokens::additional_token_object(
                ironsmith_core::AdditionalTokenKind::Squirrel,
                alice,
            ),
            crate::events::cause::EventCause::effect(),
        )
        .with_additional_tokens(ironsmith_core::AdditionalTokenKind::Treasure, 1)
        .unwrap();
        let creature = WouldCreateTokensUnderControlMatcher::new(PlayerFilter::Any)
            .with_token_filter(ObjectFilter::creature());
        let treasure = WouldCreateTokensUnderControlMatcher::new(PlayerFilter::Any)
            .with_token_filter(
                ObjectFilter::default().with_subtype(crate::types::Subtype::Treasure),
            );
        assert!(!creature.matches_event(&event, &ctx).unwrap());
        assert!(
            treasure.matches_event(&event, &ctx).unwrap(),
            "positive added group still creates tokens"
        );
        assert!(
            WouldCreateTokensUnderControlMatcher::new(PlayerFilter::Any)
                .matches_event(&event, &ctx)
                .unwrap()
        );
    }
}

#[cfg(test)]
mod simultaneous_token_resource_contract {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::tokens::TokenCreationLimits;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn action() -> crate::effects::ForPlayersEffect {
        crate::effects::ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(CreateTokenEffect::new(
                crate::cards::tokens::treasure_token_definition(),
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        )
    }
    #[test]
    fn native_and_dispatch_each_player_tokens_charge_one_instruction_per_sibling() {
        for dispatcher in [false, true] {
            for maximum in [0, 1, 3] {
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                game.set_token_creation_limits(TokenCreationLimits {
                    max_instructions: maximum,
                    max_nesting: 1,
                    ..Default::default()
                });
                let source = game.new_object_id();
                let next = game.next_object_id_counter();
                let graph = game.provenance_graph().node_count();
                let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
                let effect = action();
                let result = if dispatcher {
                    crate::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx)
                } else {
                    effect.execute(&mut game, &mut ctx)
                };
                if maximum < 3 {
                    assert!(
                        matches!(
                            result,
                            Err(ExecutionError::ResourceLimitExceeded {
                                resource: "token instruction work",
                                ..
                            })
                        ),
                        "{result:?}"
                    );
                    assert!(game.battlefield.is_empty());
                    assert_eq!(game.next_object_id_counter(), next);
                    assert_eq!(game.provenance_graph().node_count(), graph);
                    assert!(game.effect_store.pending_trigger_events.is_empty());
                    assert!(game.turn_store.turn_history.staged_event_records.is_empty());
                } else {
                    result.unwrap();
                    assert_eq!(
                        game.battlefield.len(),
                        3,
                        "sibling preparation is not nesting"
                    );
                    for player in [PlayerId(0), PlayerId(1), PlayerId(2)] {
                        assert_eq!(
                            game.battlefield
                                .iter()
                                .filter(|id| game.current_controller(**id) == Some(player))
                                .count(),
                            1
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn completion_token_work_is_nested_under_its_participant_permit() {
        for maximum in [1, 2] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            game.set_token_creation_limits(TokenCreationLimits {
                max_instructions: 4,
                max_nesting: maximum,
                ..Default::default()
            });
            let source = game.new_object_id();
            game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    PlayerId(0),
                    crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                        PlayerFilter::You,
                    ),
                    crate::replacement::ReplacementAction::Additionally(vec![Effect::new(
                        CreateTokenEffect::one(crate::cards::tokens::treasure_token_definition()),
                    )]),
                ),
            );
            let next = game.next_object_id_counter();
            let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
            let result = action().execute(&mut game, &mut ctx);
            if maximum == 1 {
                assert!(
                    matches!(
                        result,
                        Err(ExecutionError::ResourceLimitExceeded {
                            resource: "nested token instructions",
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert!(game.battlefield.is_empty());
                assert_eq!(game.next_object_id_counter(), next);
            } else {
                result.unwrap();
                assert_eq!(game.battlefield.len(), 4);
            }
        }
    }
}
