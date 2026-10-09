//! Create token copy effect implementation.

use crate::ability::Ability;
use crate::card::PtValue;
use crate::combat_state::AttackTarget;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::{CounterType, Object};
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;

use super::lifecycle::{
    TokenCleanupOptions, TokenEntryOptions, apply_token_battlefield_entry_with_outputs,
    create_replacement_additional_tokens, schedule_token_cleanup_with_outputs,
};

/// Effect that creates a token copy of a permanent.
///
/// # Fields
///
/// * `target` - Which permanent to copy
/// * `count` - How many copies to create
/// * `controller` - Who controls the tokens
/// * `enters_tapped` - Whether the copy enters tapped
/// * `has_haste` - Whether the copy has haste
/// * `enters_attacking` - Whether the copy enters attacking
/// * `attack_target_mode` - Optional custom attack-target selection when attacking
/// * `exile_at_end_of_combat` - Whether to exile at end of combat
///
/// # Example
///
/// ```ignore
/// // Create a token copy of target creature
/// let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
///
/// // Create a copy with haste that's exiled at end of combat (Kiki-Jiki style)
/// let effect = CreateTokenCopyEffect::kiki_jiki_style(ChooseSpec::creature());
/// ```
pub type CopyPtAdjustment = ironsmith_core::CopyPtAdjustment;
pub type CopyAttackTargetMode = ironsmith_core::CopyAttackTargetMode;
pub type TokenCopyReferenceSurface = ironsmith_core::TokenCopyReferenceSurface;
pub type CreateTokenCopyEffect = ironsmith_core::CreateTokenCopyEffect<StaticAbility>;

/// The authored PlayerOrPlaneswalkerControlledBy mode excludes battles.
/// Unqualified "attacking" uses combat::choose_enters_attacking_target instead.
pub(super) fn attack_targets_for_player(
    game: &GameState,
    player_id: PlayerId,
) -> Vec<AttackTarget> {
    let mut targets = Vec::new();
    if game
        .player(player_id)
        .is_some_and(|player| player.is_in_game())
    {
        targets.push(AttackTarget::Player(player_id));
    }

    for &object_id in &game.battlefield {
        if game.is_phased_out(object_id) {
            continue;
        }
        if let Some(object) = game.object(object_id) {
            if game.controller_of(object) == player_id
                && game.current_has_card_type(object_id, CardType::Planeswalker)
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            }
        }
    }

    targets
}

pub(super) fn choose_attack_target(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    targets: &[AttackTarget],
) -> Option<AttackTarget> {
    if targets.len() == 1 {
        return Some(targets[0].clone());
    }

    let player_name = game
        .player(player_id)
        .map(|player| player.name.to_string())
        .unwrap_or_else(|| "that player".to_string());
    let options: Vec<SelectableOption> = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let description = match target {
                AttackTarget::Player(_) => format!("Attack {player_name}"),
                AttackTarget::Planeswalker(planeswalker_id) => {
                    let walker_name = game
                        .object(*planeswalker_id)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a planeswalker".to_string());
                    format!("Attack {walker_name} controlled by {player_name}")
                }
                AttackTarget::Battle(battle_id) => {
                    let battle_name = game
                        .object(*battle_id)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a battle".to_string());
                    format!("Attack {battle_name} protected by {player_name}")
                }
                AttackTarget::Nothing { .. } => "Attack nothing".to_string(),
            };
            SelectableOption::new(index, description)
        })
        .collect();
    let choice_ctx = SelectOptionsContext::new(
        ctx.controller,
        Some(ctx.source),
        format!("Choose attack target for token copy attacking {player_name}"),
        options,
        1,
        1,
    );
    let chosen = ctx.decision_maker.decide_options(game, &choice_ctx);
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    chosen
        .first()
        .copied()
        .filter(|selected| *selected < targets.len())
        .and_then(|index| targets.get(index))
        .cloned()
}

fn ability_is_soulbond_pairing(ability: &Ability) -> bool {
    let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    triggered.effects.all_effects().into_iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::SoulbondPairEffect>()
            .is_some()
    })
}

fn build_token_copy_object(
    effect: &CreateTokenCopyEffect,
    id: ObjectId,
    controller_id: PlayerId,
    target_object: Option<&Object>,
    copy_snapshot: Option<&ObjectSnapshot>,
    resolved_target_id: ObjectId,
    half_power: i32,
    half_toughness: i32,
    resolved_base_power_toughness: Option<(i32, i32)>,
    static_abilities_to_grant: &[StaticAbility],
) -> Result<Object, ExecutionError> {
    let mut token = if let Some(snapshot) = copy_snapshot {
        Object::token_copy_from_snapshot(snapshot, id, controller_id)
    } else {
        let target = target_object.ok_or(ExecutionError::ObjectNotFound(resolved_target_id))?;
        Object::token_copy_of(target, id, controller_id)
    };

    if let Some(CopyPtAdjustment::HalfRoundUp) = effect.pt_adjustment {
        token.base_power = Some(PtValue::Fixed(half_power));
        token.base_toughness = Some(PtValue::Fixed(half_toughness));
    }
    if let Some((power, toughness)) = resolved_base_power_toughness {
        token.base_power = Some(PtValue::Fixed(power));
        token.base_toughness = Some(PtValue::Fixed(toughness));
    }
    if let Some(loyalty) = effect.starting_loyalty {
        token.base_loyalty = Some(loyalty);
        token.counters.remove(&CounterType::Loyalty);
        token.add_counters(CounterType::Loyalty, loyalty);
    }
    if let Some(colors) = effect.set_colors {
        token.color_override = Some(colors);
    }
    if effect.clear_mana_cost {
        token.mana_cost = None;
    }
    if let Some(card_types) = &effect.set_card_types {
        token.card_types = card_types.clone().into();
    }
    if let Some(subtypes) = &effect.set_subtypes {
        token.subtypes = subtypes.clone().into();
    }
    for card_type in &effect.added_card_types {
        if !token.card_types.contains(card_type) {
            token.card_types.push(*card_type);
        }
    }
    for subtype in &effect.added_subtypes {
        if !token.subtypes.contains(subtype) {
            token.subtypes.push(*subtype);
        }
    }
    if !effect.removed_supertypes.is_empty() {
        token
            .supertypes
            .retain(|supertype| !effect.removed_supertypes.contains(supertype));
    }
    // CR 707.9b: copy exceptions become part of the token's copiable values.
    for supertype in &effect.added_supertypes {
        if !token.supertypes.contains(supertype) {
            token.supertypes.push(*supertype);
        }
    }
    if let Some(name) = &effect.set_name {
        token.name = name.clone().into();
    }
    if effect.loses_soulbond {
        // "except it ... loses soulbond": the copy is created without the
        // soulbond pairing ability.
        token
            .abilities_mut()
            .retain(|ability| !ability_is_soulbond_pairing(ability));
    }
    for static_ability in static_abilities_to_grant {
        token
            .abilities_mut()
            .push(Ability::static_ability(static_ability.clone()));
    }
    // "except it has haste and 'At the beginning of the end step, sacrifice
    // this token.'" (Kindle the Inner Flame): the copy exception grants a
    // real, copiable triggered ability that fires at every end step, not a
    // one-shot cleanup.
    if grants_end_step_sacrifice_ability(effect) {
        token.abilities_mut().push(Ability::triggered(
            crate::triggers::Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
            vec![crate::effect::Effect::new(
                crate::effects::SacrificeTargetEffect::new(crate::target::ChooseSpec::Source),
            )],
        ));
    }

    Ok(token)
}

fn grants_end_step_sacrifice_ability(effect: &CreateTokenCopyEffect) -> bool {
    effect.sacrifice_at_next_end_step && effect.sacrifice_at_next_end_step_ability_text.is_some()
}

/// Copy values and instruction bindings are fixed before any simultaneous
/// participant mutates the battlefield. The permit spans preparation, original
/// creation and completion without charging a sibling as a nested instruction.
struct TokenCopyProposal {
    effect: CreateTokenCopyEffect,
    token_preview: Option<Object>,
    controller: PlayerId,
    count: u32,
    configured_attack_player: Option<PlayerId>,
    attack_player_only: bool,
    required_attack_player: Option<PlayerId>,
    cleanup: TokenCleanupOptions,
    original_attack_targets: Vec<Option<AttackTarget>>,
    additional_attack_targets: Vec<Option<AttackTarget>>,
    prepared: Option<crate::events::processing::PreparedTokenCreation>,
    charge_instruction: bool,
    instruction: Option<super::resources::TokenInstructionPermit>,
}
impl std::fmt::Debug for TokenCopyProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenCopyProposal")
            .field("controller", &self.controller)
            .field("count", &self.count)
            .finish_non_exhaustive()
    }
}
fn prepare_token_copy_proposal(
    effect: &CreateTokenCopyEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<TokenCopyProposal, ExecutionError> {
    let controller_id = resolve_player_filter(game, &effect.controller, ctx)?;
    if !game
        .player(controller_id)
        .is_some_and(|player| player.is_in_game())
    {
        return Ok(TokenCopyProposal {
            effect: effect.clone(),
            token_preview: None,
            controller: controller_id,
            count: 0,
            configured_attack_player: None,
            attack_player_only: false,
            required_attack_player: None,
            cleanup: TokenCleanupOptions::default(),
            original_attack_targets: Vec::new(),
            additional_attack_targets: Vec::new(),
            prepared: None,
            charge_instruction: false,
            instruction: None,
        });
    }
    let base_count = resolve_value(game, &effect.count, ctx)?.max(0) as u32;
    // A sacrificed copy source has already left the battlefield. Its tag
    // carries the calculated snapshot captured while paying the cost, so
    // use that identity and LKI directly instead of relocating the object
    // by stable id into its new zone.
    let departed_snapshot =
        effect
            .target
            .sacrificed_object_kind()
            .and_then(|_| match effect.target.base() {
                ChooseSpec::Tagged(tag) => ctx.get_tagged(tag.as_str()).cloned(),
                _ => None,
            });
    // A source that left its zone is a new object even if the physical card
    // can still be found by stable id. Copy its recorded characteristics.
    let departed_snapshot = departed_snapshot.or_else(|| {
        (matches!(effect.target.base(), ChooseSpec::Source) && game.object(ctx.source).is_none())
            .then(|| ctx.source_snapshot.clone())
            .flatten()
    });
    let target_id = if let Some(snapshot) = departed_snapshot.as_ref() {
        snapshot.object_id
    } else {
        let resolved =
            match crate::effects::helpers::resolve_objects_from_spec(game, &effect.target, ctx) {
                Ok(ids) => ids,
                Err(ExecutionError::InvalidTarget) => Vec::new(),
                Err(error) => return Err(error),
            };
        match resolved.first() {
            Some(id) => *id,
            None => {
                // A tagged copy source may already have left its zone
                // ("if that creature dies this way" runs after the
                // destroy) — fall back to the tag's LKI snapshot. The tag
                // may sit on the spec itself or in filter constraints.
                let constraint_tag = match effect.target.base() {
                    ChooseSpec::Tagged(tag) => Some(tag.clone()),
                    ChooseSpec::Object(filter) => filter
                        .tagged_constraints
                        .iter()
                        .find(|constraint| {
                            constraint.relation
                                == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        })
                        .map(|constraint| constraint.tag.clone()),
                    _ => None,
                };
                if let Some(tag) = constraint_tag
                    && let Some(snapshot) = ctx.get_tagged(tag.as_str())
                {
                    snapshot.object_id
                } else {
                    return Err(ExecutionError::InvalidTarget);
                }
            }
        }
    };

    // Resolve target object, falling back to stored LKI snapshots when needed.
    let resolved_target_id = target_id;
    let target_object = departed_snapshot
        .is_none()
        .then(|| game.object(resolved_target_id).cloned())
        .flatten();
    let mut stored_snapshot = departed_snapshot;
    if target_object.is_none() {
        if stored_snapshot.is_some() {
            // Typed sacrificed sources always prefer the cost-time LKI.
        } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
            stored_snapshot = Some(snapshot.clone());
        } else {
            match effect.target.base() {
                ChooseSpec::Tagged(tag) => {
                    if let Some(snapshot) = ctx.get_tagged(tag.as_str()) {
                        stored_snapshot = Some(snapshot.clone());
                    }
                }
                ChooseSpec::Source => {
                    if let Some(snapshot) = &ctx.source_snapshot {
                        stored_snapshot = Some(snapshot.clone());
                    }
                }
                _ => {}
            }
        }
    }
    if stored_snapshot.is_none()
        && let Some(target) = target_object.as_ref()
    {
        stored_snapshot =
            Some(ObjectSnapshot::try_from_object_with_calculated_characteristics(target, game)?);
    }
    let copy_snapshot = stored_snapshot.as_ref();
    if target_object.is_none() && copy_snapshot.is_none() {
        return Err(ExecutionError::ObjectNotFound(target_id));
    }
    let (configured_attack_player, attack_player_only) = match &effect.attack_target_mode {
        Some(CopyAttackTargetMode::Player(player_filter)) => {
            (Some(resolve_player_filter(game, player_filter, ctx)?), true)
        }
        Some(CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player_filter)) => (
            Some(resolve_player_filter(game, player_filter, ctx)?),
            false,
        ),
        None => (None, false),
    };
    let required_attack_player = effect
        .must_attack_player_this_turn
        .as_ref()
        .map(|player| resolve_player_filter(game, player, ctx))
        .transpose()?;
    let cleanup_options = TokenCleanupOptions::new(
        effect.exile_at_end_of_combat,
        false,
        effect.sacrifice_at_next_end_step && !grants_end_step_sacrifice_ability(effect),
        effect.exile_at_next_end_step,
        effect.next_end_step_player.clone(),
    );
    let mut static_abilities_to_grant =
        Vec::with_capacity(effect.granted_static_abilities.len() + usize::from(effect.has_haste));
    if effect.has_haste && effect.haste_followup_reference_surface.is_none() {
        static_abilities_to_grant.push(StaticAbility::haste());
    }
    static_abilities_to_grant.extend(effect.granted_static_abilities.iter().cloned());

    let (half_power, half_toughness) = match effect.pt_adjustment {
        Some(CopyPtAdjustment::HalfRoundUp) => {
            let (power, toughness) = if let Some(snapshot) = copy_snapshot {
                (snapshot.power.unwrap_or(0), snapshot.toughness.unwrap_or(0))
            } else {
                let target = target_object
                    .as_ref()
                    .expect("target object should exist when no snapshot is available");
                (target.power().unwrap_or(0), target.toughness().unwrap_or(0))
            };
            ((power + 1) / 2, (toughness + 1) / 2)
        }
        None => (0, 0),
    };
    let resolved_base_power_toughness =
        if let Some((power, toughness)) = &effect.set_base_power_toughness_value {
            Some((
                resolve_value(game, power, ctx)?,
                resolve_value(game, toughness, ctx)?,
            ))
        } else {
            effect.set_base_power_toughness
        };

    let token_preview = build_token_copy_object(
        effect,
        ObjectId::from_raw(0),
        controller_id,
        target_object.as_ref(),
        copy_snapshot,
        resolved_target_id,
        half_power,
        half_toughness,
        resolved_base_power_toughness,
        &static_abilities_to_grant,
    )?;
    Ok(TokenCopyProposal {
        effect: effect.clone(),
        token_preview: Some(token_preview),
        controller: controller_id,
        count: base_count,
        configured_attack_player,
        attack_player_only,
        required_attack_player,
        cleanup: cleanup_options,
        original_attack_targets: Vec::new(),
        additional_attack_targets: Vec::new(),
        prepared: None,
        charge_instruction: false,
        instruction: None,
    })
}

/// Query the creation group's prospective characteristics without allocating an
/// identity in the actual game or running its entry programs. Resolve destinations
/// against this shared pre-original world, then retain the answers in the proposal.
fn prepare_copy_attack_targets(
    game: &GameState,
    ctx: &mut ExecutionContext,
    token: &Object,
    controller: PlayerId,
    count: u32,
    configured_player: Option<PlayerId>,
    player_only: bool,
) -> Result<Vec<Option<AttackTarget>>, ExecutionError> {
    let mut targets = super::resources::buffer(count as usize)?;
    if count == 0 {
        return Ok(targets);
    }
    let mut preview_game = game.clone();
    let preview_id = preview_game.new_object_id();
    let mut preview = Object::token_copy_of(token, preview_id, controller);
    preview.zone = Zone::Command;
    preview_game.add_object(preview);
    let prospective = crate::events::EnterBattlefieldEvent::new(preview_id, Zone::Command)
        .try_prospective_game_state(&preview_game)
        .map_err(ExecutionError::ContinuousDiscovery)?
        .ok_or_else(|| ExecutionError::InternalError("copy attack preview disappeared".into()))?;
    if !crate::effects::combat::can_enter_attacking(&prospective, preview_id) {
        targets.resize(count as usize, None);
        return Ok(targets);
    }
    for _ in 0..count {
        let target = if let Some(player) = configured_player {
            if player_only {
                game.player(player)
                    .is_some_and(|player| player.is_in_game())
                    .then_some(AttackTarget::Player(player))
            } else {
                let options = attack_targets_for_player(game, player);
                if options.is_empty() {
                    None
                } else {
                    choose_attack_target(game, ctx, player, &options)
                }
            }
        } else {
            crate::effects::combat::choose_enters_attacking_target(&prospective, ctx, preview_id)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        targets.push(target);
    }
    Ok(targets)
}

impl crate::effects::SimultaneousEffectProposal for TokenCopyProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.prepared.is_some() {
            return Ok(());
        }
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
        let prepared = crate::events::processing::prepare_token_creation_deferred(
            game,
            self.controller,
            self.count,
            self.token_preview.clone(),
            ctx.cause.clone(),
            ctx,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if let crate::events::processing::PreparedTokenCreation::Proceed { event, .. } = &prepared {
            // Reserve once, before allocating per-token choice buffers. The
            // original phase consumes these slots rather than reserving again.
            game.reserve_token_creation(event.total_count())?;
            if self.effect.enters_attacking || self.configured_attack_player.is_some() {
                use crate::events::tokens::TokenGroupKey;
                self.additional_attack_targets = super::resources::buffer(
                    (event.total_count() - u128::from(event.count)) as usize,
                )?;
                for key in event.group_keys() {
                    let token = event.group_object(key).ok_or_else(|| {
                        ExecutionError::InternalError(
                            "copy creation group lost its template".into(),
                        )
                    })?;
                    let choices = prepare_copy_attack_targets(
                        game,
                        ctx,
                        &token,
                        event.controller,
                        event.group_count(key),
                        self.configured_attack_player,
                        self.attack_player_only,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(());
                    }
                    match key {
                        TokenGroupKey::Original => self.original_attack_targets = choices,
                        _ => self.additional_attack_targets.extend(choices),
                    }
                }
            }
        }
        self.prepared = Some(prepared);
        Ok(())
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        commit_token_copy_proposal(*self, game, ctx)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        complete_token_copy_proposal(*self, game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

fn execute_token_instruction(
    effect: &CreateTokenCopyEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    complete_token_copy_proposal(prepare_token_copy_proposal(effect, game, ctx)?, game, ctx)
}

fn complete_token_copy_proposal(
    proposal: TokenCopyProposal,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let receipt = commit_token_copy_proposal(proposal, game, ctx)?;
    crate::effects::composition::complete_standalone_original_with_outputs(game, ctx, receipt)
}

fn commit_token_copy_proposal(
    mut proposal: TokenCopyProposal,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    use crate::effects::SimultaneousEffectProposal;
    use crate::events::processing::PreparedTokenCreation;
    if proposal.prepared.is_none() {
        proposal.prepare_original(game, ctx)?;
    }
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::SimultaneousEffectCommit::finished(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    let phase = proposal
        .instruction
        .as_ref()
        .map(|permit| permit.enter_phase())
        .transpose()?;
    let committed = match proposal.prepared.take().ok_or_else(|| {
        ExecutionError::InternalError("copy proposal has no prepared creation".into())
    })? {
        PreparedTokenCreation::Finished { outputs, programs } => {
            crate::effects::SimultaneousEffectCommit {
                outcome: outputs,
                completion: Some(super::lifecycle::token_instruction_completion(
                    proposal.instruction.take(),
                    Vec::new(),
                    programs,
                )),
            }
        }
        PreparedTokenCreation::Proceed {
            event,
            provenance,
            programs,
        } => {
            ctx.provenance = provenance;
            commit_token_copy_original(proposal, game, ctx, event, programs)?
        }
    };
    drop(phase);
    Ok(committed)
}
fn commit_token_copy_original(
    mut proposal: TokenCopyProposal,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    replacement: crate::events::CreateTokensEvent,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    let effect = &proposal.effect;
    let controller_id = replacement.controller;
    let token_preview = replacement
        .token
        .clone()
        .or(proposal.token_preview.take())
        .ok_or_else(|| ExecutionError::InternalError("copy original lost its template".into()))?;
    let count = replacement.count as usize;
    // Attack choices were retained during preparation, so entry cannot prompt
    // again or use a different destination after a sibling's original mutation.
    let entry_options = TokenEntryOptions::default();
    let mut created_ids = super::resources::buffer(count)?;
    let mut events = super::resources::buffer(count)?;
    let mut entry_receipts = Vec::new();
    let mut lifecycle_children = Vec::new();
    let mut entry_outputs = Vec::new();

    for index in 0..count {
        let id = game.new_object_id();
        let mut token = Object::token_copy_of(&token_preview, id, controller_id);
        token.zone = Zone::Command;
        let token_is_creature = token.is_creature();
        game.commit_token_resource_slot()?;
        game.add_object(token);
        let entry_result = game.move_created_token_with_entry_instructions(
            id,
            ctx.cause.clone(),
            &mut ctx.decision_maker,
            effect.enters_tapped,
            true,
            Vec::new(),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let Some(entry_result) = super::lifecycle::retain_token_entry_receipt(
            game,
            id,
            entry_result,
            &mut entry_outputs,
            &mut entry_receipts,
        )?
        else {
            game.remove_object(id);
            continue;
        };
        let entered_id = entry_result.new_id;
        created_ids.push(entered_id);
        if game
            .object(entered_id)
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
        {
            let entered_is_creature = game.current_is_creature(entered_id);
            let entry_observation = apply_token_battlefield_entry_with_outputs(
                game,
                ctx,
                entered_id,
                controller_id,
                entered_is_creature || token_is_creature,
                entry_options,
                Zone::Command,
                entry_result.enters_tapped,
                &mut events,
            )?;
            super::lifecycle::retain_token_child(
                &mut events,
                &mut lifecycle_children,
                entry_observation,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::with_objects(Vec::new()),
                    ),
                ));
            }
            if let Some(Some(target)) = proposal.original_attack_targets.get(index)
                && crate::effects::combat::can_enter_attacking(game, entered_id)
            {
                game.add_entering_attacker(entered_id, target.clone());
            }
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
            entry: entry_options,
            prepared_attack_targets: (effect.enters_attacking
                || proposal.configured_attack_player.is_some())
            .then_some(proposal.additional_attack_targets),
            ..Default::default()
        },
        &mut events,
        &mut entry_receipts,
        &mut lifecycle_children,
        &mut entry_outputs,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::SimultaneousEffectCommit::finished(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    created_ids.extend(additional_ids);
    super::lifecycle::publish_created_token_groups(game, ctx, actual_creation, &mut events);
    // Cleanup belongs to every original, including added/substituted groups,
    // before a completion is allowed to remove the effect's source or a copy.
    for &id in &created_ids {
        if game
            .object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            if let Some(player) = proposal.required_attack_player {
                game.effect_store.attack_player_requirements.push((
                    id,
                    player,
                    game.turn.turn_number,
                ));
            }
            let cleanup = schedule_token_cleanup_with_outputs(
                game,
                ctx,
                id,
                controller_id,
                proposal.cleanup.clone(),
            )?;
            super::lifecycle::retain_token_child(&mut events, &mut lifecycle_children, cleanup);
        }
    }
    let haste_recipients = if effect.has_haste && effect.haste_followup_reference_surface.is_some()
    {
        created_ids.clone()
    } else {
        Vec::new()
    };
    let mut outcome = super::lifecycle::compose_token_original(
        EffectOutcome::with_objects(created_ids.clone())
            .with_result_objects(created_ids)
            .with_events(events),
        lifecycle_children,
    );
    outcome.retain_published_references(entry_outputs);
    Ok(crate::effects::SimultaneousEffectCommit {
        outcome,
        completion: Some(super::lifecycle::token_instruction_completion_with_haste(
            proposal.instruction.take(),
            entry_receipts,
            programs,
            haste_recipients,
        )),
    })
}

impl EffectExecutor for CreateTokenCopyEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for ability in &self.granted_static_abilities {
            crate::ability::visit_static_owned_effects(ability, visitor);
        }
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let mut proposal = prepare_token_copy_proposal(self, game, ctx)?;
        proposal.charge_instruction = true;
        Ok(Box::new(proposal))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::lifecycle::execute_token_instruction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| execute_token_instruction(self, game, ctx),
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to copy"
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::{CardDefinition, CardDefinitionBuilder};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::object::ObjectKind;
    use crate::snapshot::ObjectSnapshot;
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::tag::TagKey;
    use crate::target::{ChooseSpecSurfaceHint, ObjectFilter, SacrificedObjectKind};
    use crate::test_prelude::*;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_planeswalker(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Planeswalker])
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn treasure_token_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Treasure")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build()
    }

    fn fancy_treasure_token_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Fancy Treasure")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build()
    }

    fn clue_token_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Clue")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Clue])
            .build()
    }

    fn xorn_definition() -> CardDefinition {
        let oracle = "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.";
        CardDefinitionBuilder::new(CardId::new(), "Xorn")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elemental])
            .oracle_text(oracle)
            .with_ability(crate::ability::Ability::static_ability(
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

    #[test]
    fn copy_followups_apply_to_additional_replacement_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        let original = game.create_object_from_definition(
            &treasure_token_definition(),
            alice,
            Zone::Battlefield,
        );
        game.refresh_continuous_state();
        let mut effect = CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(original))
            .sacrifice_at_next_end_step(true);
        effect.must_attack_player_this_turn = Some(PlayerFilter::Specific(bob));
        let mut ctx = ExecutionContext::new_default(original, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("missing tokens")
        };
        assert_eq!(ids.len(), 2);
        assert_eq!(game.effect_store.delayed_triggers.len(), 2);
        for id in ids {
            assert_eq!(
                game.required_attack_players_this_turn(id)
                    .collect::<Vec<_>>(),
                vec![bob]
            );
        }
    }

    #[test]
    fn test_create_token_copy() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Grizzly Bears");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn sacrificed_copy_source_uses_cost_time_lki_after_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Battlefield Form", alice);
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(creature_id).expect("sacrifice source exists"),
            &game,
        );
        let graveyard_id = game
            .move_object(
                creature_id,
                Zone::Graveyard,
                crate::events::EventCause::effect(),
            )
            .expect("sacrifice source moves");
        game.object_mut(graveyard_id)
            .expect("moved card exists")
            .name = "Graveyard Form".into();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("sacrifice_cost_0", vec![snapshot]);
        let target = ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
            ChooseSpecSurfaceHint::SacrificedObject(SacrificedObjectKind::Creature),
        );

        let result = CreateTokenCopyEffect::one(target)
            .execute(&mut game, &mut ctx)
            .expect("copy sacrificed creature from LKI");
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected copied token");
        };
        assert_eq!(
            game.object(ids[0]).expect("token exists").name,
            "Battlefield Form"
        );
    }

    #[test]
    fn test_create_token_copy_with_haste() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Baneslayer Angel", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::with_haste(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).unwrap();
            // Token should have haste ability
            let has_haste = token.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            assert!(has_haste, "Token should have haste");
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_can_clear_mana_cost_and_add_embalm_modifiers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(100), "Angel of Sanctions")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![crate::types::Subtype::Angel])
            .power_toughness(PowerToughness::fixed(3, 4))
            .build();
        let source_id = game.new_object_id();
        let source = Object::from_card(source_id, &source_card, alice, Zone::Graveyard);
        game.add_object(source);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        let effect = CreateTokenCopyEffect::new(ChooseSpec::Source, 1, PlayerFilter::You)
            .set_colors(crate::color::ColorSet::WHITE)
            .added_subtype(crate::types::Subtype::Zombie)
            .without_mana_cost();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(token.name, "Angel of Sanctions");
        assert_eq!(token.mana_cost, None);
        assert_eq!(token.colors(), crate::color::ColorSet::WHITE);
        assert!(token.subtypes.contains(&crate::types::Subtype::Angel));
        assert!(token.subtypes.contains(&crate::types::Subtype::Zombie));
    }

    #[test]
    fn test_create_token_copy_with_haste_is_seen_by_etb_replacements() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Swift Probe", alice);
        let source = game.new_object_id();

        let haste_matters = CardDefinitionBuilder::new(CardId::new(), "Haste Matters")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::enters_with_counters_for_filter(
                    ObjectFilter::creature().with_static_ability(StaticAbilityId::Haste),
                    CounterType::Vigilance,
                    1,
                ),
            ))
            .build();
        game.create_object_from_definition(&haste_matters, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::with_haste(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(
            token.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "ETB replacement effects should see the token's granted haste while it is being created"
        );
    }

    #[test]
    fn test_create_token_copy_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Serra Angel", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::tapped(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert!(game.is_tapped(ids[0]), "Token should enter tapped");
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_multiple_token_copies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Llanowar Elves", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::new(ChooseSpec::creature(), 3, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Llanowar Elves");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn create_token_copy_replacement_doubles_token_copies_created_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
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

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = CreateTokenCopyEffect::one(ChooseSpec::creature())
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|id| {
            game.object(*id).is_some_and(|token| {
                token.name == "Grizzly Bears" && token.kind == ObjectKind::Token
            })
        }));
    }

    #[test]
    fn xorn_adds_one_token_when_copying_a_treasure_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let treasure_id = game.create_object_from_definition(
            &fancy_treasure_token_definition(),
            alice,
            Zone::Battlefield,
        );
        let source = game.new_object_id();
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(treasure_id)]);
        let effect = CreateTokenCopyEffect::new(
            ChooseSpec::Object(ObjectFilter::artifact().with_subtype(Subtype::Treasure)),
            1,
            PlayerFilter::You,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2, "Xorn should add one Treasure token");
        let copied_count = ids
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
            copied_count, 1,
            "the original token copy should be preserved"
        );
        assert_eq!(normal_count, 1, "Xorn should add one normal Treasure token");
        assert!(ids.iter().all(|id| {
            game.object(*id).is_some_and(|token| {
                token.kind == ObjectKind::Token
                    && token.subtypes.contains(&Subtype::Treasure)
                    && game.controller_of(token) == alice
            })
        }));
    }

    #[test]
    fn xorn_does_not_add_tokens_when_copying_non_treasure_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let clue_id =
            game.create_object_from_definition(&clue_token_definition(), alice, Zone::Battlefield);
        let source = game.new_object_id();
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(clue_id)]);
        let effect = CreateTokenCopyEffect::new(
            ChooseSpec::Object(ObjectFilter::artifact().with_subtype(Subtype::Clue)),
            1,
            PlayerFilter::You,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1, "Xorn should ignore non-Treasure token copies");
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(token.name, "Clue");
        assert!(token.subtypes.contains(&Subtype::Clue));
    }

    #[test]
    fn xorn_does_not_add_tokens_to_other_players_treasure_token_copies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let treasure_id = game.create_object_from_definition(
            &treasure_token_definition(),
            bob,
            Zone::Battlefield,
        );
        let source = game.new_object_id();
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(treasure_id)]);
        let effect = CreateTokenCopyEffect::new(
            ChooseSpec::Object(ObjectFilter::artifact().with_subtype(Subtype::Treasure)),
            1,
            PlayerFilter::Specific(bob),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(
            ids.len(),
            1,
            "Xorn should only affect its controller's Treasure token copies"
        );
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(token.name, "Treasure");
        assert_eq!(game.controller_of(token), bob);
    }

    #[test]
    fn test_create_token_copy_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err(), "Should fail without target");
    }

    #[test]
    fn test_create_token_copy_clone_box() {
        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenCopyEffect"));
    }

    #[test]
    fn test_create_token_copy_kiki_jiki_style() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Pestermite", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::kiki_jiki_style(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token_id = ids[0];
            let token = game.object(token_id).unwrap();

            // Token should have haste
            let has_haste = token.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            assert!(has_haste, "Token should have haste");

            // Should have delayed trigger to exile at end of combat
            assert_eq!(game.effect_store.delayed_triggers.len(), 1);
            let delayed = &game.effect_store.delayed_triggers[0];
            assert!(delayed.trigger.display().contains("end of combat"));
            assert!(delayed.one_shot);
            assert_eq!(delayed.target_objects, vec![token_id]);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_enters_attacking() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature(&mut game, "Goblin Guide", alice);
        let source = create_creature(&mut game, "Source Attacker", alice);

        // Set up combat with source attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);
        game.turn.phase = crate::game_state::Phase::Combat;

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature()).attacking(true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token_id = ids[0];
            // Token should be added to combat attackers
            let combat = game.combat.as_ref().expect("Combat should still be active");
            assert!(
                combat
                    .attackers
                    .iter()
                    .any(|info| info.creature == token_id),
                "Token should be in combat attackers"
            );
            // Token should be attacking the same target as source
            let token_attacker = combat
                .attackers
                .iter()
                .find(|info| info.creature == token_id)
                .expect("Token should be attacking");
            assert_eq!(
                token_attacker.target,
                AttackTarget::Player(bob),
                "Token should attack the same target as source"
            );
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_attacks_chosen_planeswalker_of_iterated_player() {
        use crate::combat_state::{AttackTarget, CombatState};
        use crate::decision::DecisionMaker;

        struct ChooseLastOptionDecisionMaker;
        impl DecisionMaker for ChooseLastOptionDecisionMaker {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                vec![ctx.options.last().map(|option| option.index).unwrap_or(0)]
            }
        }

        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let creature_id = create_creature(&mut game, "Goblin Guide", alice);
        let source = create_creature(&mut game, "Source Attacker", alice);
        let charlie_walker = create_planeswalker(&mut game, "Charlie Walker", charlie);
        game.combat = Some(CombatState::default());
        game.turn.phase = crate::game_state::Phase::Combat;

        let mut dm = ChooseLastOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.iteration.iterated_player = Some(charlie);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature())
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token_id = ids[0];
            let combat = game.combat.as_ref().expect("Combat should still be active");
            let token_attacker = combat
                .attackers
                .iter()
                .find(|info| info.creature == token_id)
                .expect("Token should be attacking");
            assert_eq!(
                token_attacker.target,
                AttackTarget::Planeswalker(charlie_walker),
                "Token should attack the chosen planeswalker"
            );
        } else {
            panic!("Expected Objects result");
        }
        assert_ne!(bob, charlie, "sanity check");
    }

    #[test]
    fn player_only_attack_mode_does_not_offer_that_players_planeswalkers() {
        use crate::combat_state::{AttackTarget, CombatState};

        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let charlie = PlayerId::from_index(2);
        let creature_id = create_creature(&mut game, "Copy Source", alice);
        let source = create_creature(&mut game, "Ability Source", alice);
        let _charlie_walker = create_planeswalker(&mut game, "Charlie Walker", charlie);
        game.combat = Some(CombatState::default());
        game.turn.phase = crate::game_state::Phase::Combat;

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.iteration.iterated_player = Some(charlie);
        let result = CreateTokenCopyEffect::one(ChooseSpec::creature())
            .enters_tapped(true)
            .attacking_player(PlayerFilter::IteratedPlayer)
            .execute(&mut game, &mut ctx)
            .expect("create attacking copy");

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected copied token");
        };
        let combat = game.combat.as_ref().expect("combat should remain active");
        let token_attacker = combat
            .attackers
            .iter()
            .find(|info| info.creature == ids[0])
            .expect("token should be attacking");
        assert_eq!(token_attacker.target, AttackTarget::Player(charlie));
        assert!(game.is_tapped(ids[0]), "token should enter tapped");
    }

    #[test]
    fn test_composed_myriad_effect_creates_for_each_other_opponent_and_exiles_at_eoc() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
        use crate::decision::DecisionMaker;
        use crate::effect::Effect;
        use crate::effects::execute_effect;
        use crate::events::phase::EndOfCombatEvent;
        use crate::triggers::TriggerEvent;

        struct AlwaysYesDecisionMaker;
        impl DecisionMaker for AlwaysYesDecisionMaker {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }

        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Dana".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let dana = PlayerId::from_index(3);
        let source = create_creature(&mut game, "Myriad Source", alice);
        let other_attacker = create_creature(&mut game, "Other Attacker", alice);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: source,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other_attacker,
                    target: AttackTarget::Player(charlie),
                },
            ],
            ..CombatState::default()
        });

        game.turn.phase = crate::game_state::Phase::Combat;

        let composed_myriad = Effect::for_players(
            PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending),
            vec![Effect::may(vec![Effect::new(
                CreateTokenCopyEffect::new(ChooseSpec::Source, 1, PlayerFilter::You)
                    .enters_tapped(true)
                    .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer)
                    .exile_at_eoc(true),
            )])],
        );

        let mut dm = AlwaysYesDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_defending_player(bob);
        let outcome = execute_effect(&mut game, &composed_myriad, &mut ctx).unwrap();
        assert!(
            outcome.something_happened(),
            "expected composed myriad effect to create at least one token"
        );

        let combat = game.combat.as_ref().expect("combat should exist");
        let token_attackers: Vec<_> = combat
            .attackers
            .iter()
            .filter_map(|info| {
                (info.creature != source && info.creature != other_attacker)
                    .then_some((info.creature, info.target.clone()))
            })
            .collect();
        assert_eq!(token_attackers.len(), 2);

        let mut attacked_players: Vec<_> = token_attackers
            .iter()
            .filter_map(|(_, target)| match target {
                AttackTarget::Player(player) => Some(*player),
                AttackTarget::Planeswalker(_)
                | AttackTarget::Battle(_)
                | AttackTarget::Nothing { .. } => None,
            })
            .collect();
        attacked_players.sort();
        assert_eq!(attacked_players, vec![charlie, dana]);

        let token_ids: Vec<_> = token_attackers.iter().map(|(id, _)| *id).collect();
        let cleanup_trigger_count = game
            .effect_store
            .delayed_triggers
            .iter()
            .filter(|delayed| {
                delayed.trigger.display().contains("end of combat")
                    && delayed.target_objects.len() == 1
                    && token_ids.contains(&delayed.target_objects[0])
            })
            .count();
        assert_eq!(cleanup_trigger_count, 2);

        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        let event = TriggerEvent::new_with_provenance(EndOfCombatEvent::new(), ctx.provenance);
        for entry in crate::triggers::check_delayed_triggers(&mut game, &event) {
            trigger_queue.add(entry);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("put delayed triggers on stack");
        while !game.stack_is_empty() {
            crate::game_loop::resolve_stack_entry(&mut game).expect("resolve delayed trigger");
        }

        for token_id in token_ids {
            assert!(
                !game.battlefield.contains(&token_id),
                "myriad token should be exiled at end of combat"
            );
        }
    }

    #[test]
    fn test_create_token_copy_uses_source_snapshot_after_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Offspring Source", alice);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source exists"),
            &game,
        );

        let moved_id = game
            .move_object_by_effect(source, Zone::Graveyard)
            .expect("source should move to graveyard");
        assert_ne!(
            moved_id, source,
            "zone change should create a new object id"
        );

        let mut ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(source_snapshot);
        let effect = CreateTokenCopyEffect::one(ChooseSpec::Source);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).expect("token should exist");
            assert_eq!(token.name, "Offspring Source");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn explicit_starting_loyalty_replaces_copied_loyalty_and_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Copied Walker", alice);
        let walker = game.object_mut(source).expect("source exists");
        walker.card_types = vec![CardType::Planeswalker].into();
        walker.base_loyalty = Some(4);
        walker.counters.insert(CounterType::Loyalty, 4);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = CreateTokenCopyEffect::one(ChooseSpec::Source)
            .starting_loyalty(1)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected copied token ids");
        };
        let token = game.object(ids[0]).expect("token exists");
        assert_eq!(token.base_loyalty, Some(1));
        assert_eq!(token.counters.get(&CounterType::Loyalty), Some(&1));
    }

    #[test]
    fn test_create_token_copy_uses_target_snapshot_after_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Returned Copy Target", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let moved_id = game
            .move_object_by_effect(creature_id, Zone::Hand)
            .expect("target should move to hand");
        assert_ne!(
            moved_id, creature_id,
            "zone change should create a new object id"
        );

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).expect("token should exist");
            assert_eq!(token.name, "Returned Copy Target");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }
}

#[cfg(test)]
mod simultaneous_copy_resource_contract {
    use super::*;
    use crate::effect::Effect;
    use crate::effects::tokens::TokenCreationLimits;
    use crate::target::PlayerFilter;

    fn fixture() -> (GameState, ObjectId) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Frozen copy source")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 3)).build();
        let source = game.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
        game.take_pending_trigger_events();
        (game, source)
    }
    fn action(source: ObjectId) -> crate::effects::ForPlayersEffect {
        let mut copy = CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(source));
        copy.controller = PlayerFilter::IteratedPlayer;
        crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(copy)])
    }
    #[test]
    fn copy_siblings_charge_one_instruction_each_across_all_phases() {
        for dispatcher in [false, true] { for maximum in [0, 1, 3] {
            let (mut game, source) = fixture();
            game.set_token_creation_limits(TokenCreationLimits {max_instructions: maximum,
                max_nesting: 1, ..Default::default()});
            let next = game.next_object_id_counter(); let graph = game.provenance_graph().node_count();
            let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
            let effect = action(source);
            let result = if dispatcher { crate::effects::execute_effect(&mut game, &Effect::new(effect), &mut ctx) }
                else { effect.execute(&mut game, &mut ctx) };
            if maximum < 3 {
                assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded {resource: "token instruction work", ..})), "{result:?}");
                assert_eq!(game.battlefield, vec![source]);
                assert_eq!(game.next_object_id_counter(), next);
                assert_eq!(game.provenance_graph().node_count(), graph);
                assert!(game.effect_store.pending_trigger_events.is_empty());
            } else {
                result.unwrap();
                for player in [PlayerId(0), PlayerId(1), PlayerId(2)] {
                    let copies: Vec<_> = game.battlefield.iter().copied().filter(|id|
                        game.object(*id).is_some_and(|object| matches!(object.kind, crate::object::ObjectKind::Token)) && game.current_controller(*id) == Some(player)).collect();
                    assert_eq!(copies.len(), 1);
                    assert_eq!(game.object(copies[0]).unwrap().name.as_ref(), "Frozen copy source");
                }
            }
        } }
    }
    #[test]
    fn copy_completion_nested_work_keeps_its_permit_and_rolls_back_every_original_on_failure() {
        for maximum in [1, 2] {
            let (mut game, source) = fixture();
            game.set_token_creation_limits(TokenCreationLimits {max_instructions: 4,
                max_nesting: maximum, ..Default::default()});
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(source, PlayerId(0),
                    crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
                    crate::replacement::ReplacementAction::Additionally(vec![Effect::new(
                        CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(source)),
                    )]),
                ),
            );
            let next = game.next_object_id_counter();
            let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
            let result = action(source).execute(&mut game, &mut ctx);
            if maximum == 1 {
                assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded {resource: "nested token instructions", ..})), "{result:?}");
                assert_eq!(game.battlefield, vec![source]); assert_eq!(game.next_object_id_counter(), next);
                assert!(game.effect_store.replacement_effects.get_effect(replacement).is_some());
                assert!(game.effect_store.pending_trigger_events.is_empty());
            } else {
                result.unwrap();
                assert_eq!(game.battlefield.iter().filter(|id| matches!(game.object(**id).unwrap().kind, crate::object::ObjectKind::Token)).count(), 4);
                assert!(game.effect_store.replacement_effects.get_effect(replacement).is_none());
            }
        }
    }
}
