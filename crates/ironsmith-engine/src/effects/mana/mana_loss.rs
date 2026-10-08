//! CR 106.4 / 106.6 mana loss/conversion. Original units remain paired with their
//! production snapshots, retention deadlines and exact spending restrictions.
use crate::ability::RestrictedManaUnit;
use crate::effect::EffectOutcome;
use crate::effects::{
    ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion,
    SimultaneousEffectProposal,
};
use crate::events::mana::POOL_SYMBOLS;
use crate::events::processing::{PreparedReplacementProgram, TraitEventResult};
use crate::events::{Event, ManaLostEvent};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::mana::ManaSymbol;
use crate::player::{ManaPool, ManaSourceProvenance};

#[derive(Debug, Clone)]
struct Unit {
    provenance: ManaSourceProvenance,
    restriction: Option<RestrictedManaUnit>,
    loses: bool,
}

/// Freeze the mana that is actually due to be lost, rather than all mana of
/// the same color. Boundary retention does not protect against forced loss.
#[derive(Debug)]
pub(crate) struct ManaLossProposal {
    player: PlayerId,
    original_pool: ManaPool,
    original_provenance: Vec<ManaSourceProvenance>,
    original_restricted: Vec<RestrictedManaUnit>,
    units: Vec<Unit>,
    untracked_retained: ManaPool,
    untracked_lost: ManaPool,
    event: ManaLostEvent,
    prepared: Option<TraitEventResult>,
}

fn add_checked(pool: &mut ManaPool, symbol: ManaSymbol, amount: u32) -> Result<(), ExecutionError> {
    let total = u128::from(pool.amount(symbol)) + u128::from(amount);
    if total > u128::from(u32::MAX) {
        return Err(ExecutionError::ResourceLimitExceeded {
            resource: "mana pool units of one type",
            requested: total,
            maximum: u128::from(u32::MAX),
        });
    }
    pool.add(symbol, amount);
    Ok(())
}
fn nonempty(pool: &ManaPool) -> bool {
    POOL_SYMBOLS.iter().any(|symbol| pool.amount(*symbol) != 0)
}

impl ManaLossProposal {
    pub(crate) fn new(
        game: &GameState,
        player: PlayerId,
        boundary: bool,
    ) -> Result<Self, ExecutionError> {
        let data = game
            .player(player)
            .ok_or(ExecutionError::PlayerNotFound(player))?;
        let ending_combat = game.turn.phase == crate::game_state::Phase::Combat
            && game.turn.step == Some(crate::game_state::Step::EndCombat);
        let ending_turn = game.turn.phase == crate::game_state::Phase::Ending
            && game.turn.step == Some(crate::game_state::Step::Cleanup);
        let scopes = game.effect_store.cant_effects.retained_mana_scopes(player);
        let globally_retained = |symbol| {
            boundary
                && scopes.is_some_and(|scopes| {
                    scopes.contains(&None)
                        || match symbol {
                            ManaSymbol::White => scopes.contains(&Some(crate::Color::White)),
                            ManaSymbol::Blue => scopes.contains(&Some(crate::Color::Blue)),
                            ManaSymbol::Black => scopes.contains(&Some(crate::Color::Black)),
                            ManaSymbol::Red => scopes.contains(&Some(crate::Color::Red)),
                            ManaSymbol::Green => scopes.contains(&Some(crate::Color::Green)),
                            _ => false,
                        }
                })
        };
        let mut remaining = data.mana_pool.clone();
        let mut used_restricted = std::collections::HashSet::new();
        let mut units = Vec::new();
        let mut lost = ManaPool::default();
        for provenance in &data.mana_source_provenance {
            if !POOL_SYMBOLS.contains(&provenance.symbol) || !remaining.remove(provenance.symbol, 1)
            {
                return Err(ExecutionError::InternalError(
                    "mana loss found a detached production unit".into(),
                ));
            }
            // Pair before recoloring, while the original type distinguishes
            // units from a producer that supplied multiple colors. Rebuild the
            // restricted vector in this same order on commit, so later exact
            // payable-unit selection cannot cross-wire two converted units.
            let restriction = if provenance.restricted {
                let (index, unit) = data
                    .restricted_mana
                    .iter()
                    .enumerate()
                    .find(|(index, unit)| {
                        !used_restricted.contains(index)
                            && unit.source == provenance.source
                            && unit.symbol == provenance.symbol
                    })
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "mana loss found unpaired restricted provenance".into(),
                        )
                    })?;
                used_restricted.insert(index);
                Some(unit.clone())
            } else {
                None
            };
            let mut provenance = provenance.clone();
            if boundary
                && ((ending_combat
                    && provenance.retention
                        == Some(ironsmith_core::ManaRetentionDuration::EndOfCombat))
                    || (ending_turn
                        && provenance.retention
                            == Some(ironsmith_core::ManaRetentionDuration::EndOfTurn)))
            {
                provenance.retention = None;
            }
            let retained = globally_retained(provenance.symbol)
                || (boundary
                    && match provenance.retention {
                        Some(ironsmith_core::ManaRetentionDuration::EndOfCombat) => !ending_combat,
                        Some(ironsmith_core::ManaRetentionDuration::EndOfTurn) => !ending_turn,
                        None => false,
                    });
            if !retained {
                add_checked(&mut lost, provenance.symbol, 1)?;
            }
            units.push(Unit {
                provenance,
                restriction,
                loses: !retained,
            });
        }
        if used_restricted.len() != data.restricted_mana.len() {
            return Err(ExecutionError::InternalError(
                "mana loss found a restriction without its production unit".into(),
            ));
        }
        let mut untracked_retained = ManaPool::default();
        let mut untracked_lost = ManaPool::default();
        for symbol in POOL_SYMBOLS {
            let amount = remaining.amount(symbol);
            if globally_retained(symbol) {
                untracked_retained.add(symbol, amount);
            } else {
                untracked_lost.add(symbol, amount);
                add_checked(&mut lost, symbol, amount)?;
            }
        }
        Ok(Self {
            player,
            original_pool: data.mana_pool.clone(),
            original_provenance: data.mana_source_provenance.clone(),
            original_restricted: data.restricted_mana.clone(),
            units,
            untracked_retained,
            untracked_lost,
            event: ManaLostEvent {
                player,
                mana: lost,
                converted_to: None,
            },
            prepared: None,
        })
    }

    fn commit_resolved(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: TraitEventResult,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        let (original, programs) = result.into_expansion();
        let outcome = match original {
            TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                let loss = crate::events::downcast_event::<ManaLostEvent>(event.inner())
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "mana loss returned an incompatible event".into(),
                        )
                    })?;
                if loss.player != self.player || loss.mana != self.event.mana {
                    return Err(ExecutionError::InternalError(
                        "mana loss changed an unsupported unit identity".into(),
                    ));
                }
                let player = game
                    .player_mut(self.player)
                    .ok_or(ExecutionError::PlayerNotFound(self.player))?;
                if player.mana_pool != self.original_pool
                    || player.mana_source_provenance != self.original_provenance
                    || player.restricted_mana != self.original_restricted
                {
                    return Err(ExecutionError::InternalError(
                        "mana loss proposal became stale before original commit".into(),
                    ));
                }
                let mut pool = self.untracked_retained;
                if let Some(symbol) = loss.converted_to {
                    for old in POOL_SYMBOLS {
                        add_checked(&mut pool, symbol, self.untracked_lost.amount(old))?;
                    }
                }
                let mut provenance = Vec::new();
                let mut restricted = Vec::new();
                for mut unit in self.units {
                    if unit.loses {
                        let Some(symbol) = loss.converted_to else {
                            continue;
                        };
                        unit.provenance.symbol = symbol;
                        if let Some(restriction) = &mut unit.restriction {
                            restriction.symbol = symbol;
                        }
                    }
                    add_checked(&mut pool, unit.provenance.symbol, 1)?;
                    if let Some(restriction) = unit.restriction {
                        restricted.push(restriction);
                    }
                    provenance.push(unit.provenance);
                }
                player.mana_pool = pool;
                player.mana_source_provenance = provenance;
                player.restricted_mana = restricted;
                let amount = if loss.converted_to.is_some() {
                    0
                } else {
                    crate::events::damage::checked_damage_count(
                        POOL_SYMBOLS
                            .iter()
                            .map(|symbol| u128::from(loss.mana.amount(*symbol)))
                            .sum(),
                        "lost mana receipt",
                    )?
                };
                game.mark_continuous_state_dirty();
                let mut outcome = EffectOutcome::count(amount);
                if amount > 0 {
                    let observed = game.alloc_child_event_provenance(
                        event.provenance(),
                        crate::events::EventKind::ManaLost,
                    );
                    let mut notification =
                        crate::triggers::TriggerEvent::new_with_provenance(loss.clone(), observed);
                    if let Some(batch) = game.simultaneous_action_batch() {
                        notification = notification.with_simultaneous_batch(batch);
                    }
                    outcome.events.push(notification);
                }
                outcome
            }
            TraitEventResult::Prevented => {
                let player = game
                    .player_mut(self.player)
                    .ok_or(ExecutionError::PlayerNotFound(self.player))?;
                if player.mana_pool != self.original_pool
                    || player.mana_source_provenance != self.original_provenance
                    || player.restricted_mana != self.original_restricted
                {
                    return Err(ExecutionError::InternalError(
                        "prevented mana-loss proposal became stale".into(),
                    ));
                }
                // A prevented loss preserves the mana, but it cannot prolong
                // a duration that expired before this boundary's event.
                player.mana_source_provenance =
                    self.units.into_iter().map(|unit| unit.provenance).collect();
                EffectOutcome::prevented()
            }
            TraitEventResult::Replaced { .. } => return Err(ExecutionError::InternalError(
                "mana-loss replacement program has no simultaneous original/completion contract"
                    .into(),
            )),
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. }
                if ctx.decision_maker.awaiting_choice() =>
            {
                EffectOutcome::count(0)
            }
            _ => {
                return Err(ExecutionError::InternalError(
                    "mana loss has an unresolved replacement".into(),
                ));
            }
        };
        Ok(SimultaneousEffectCommit {
            outcome,
            completion: if programs.is_empty() {
                None
            } else {
                Some(Box::new(ManaLossCompletion { programs }))
            },
        })
    }
}
impl SimultaneousEffectProposal for ManaLossProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if !self
            .prepared
            .as_ref()
            .is_some_and(|original| !original.requires_replacement_input())
        {
            let event = Event::new_with_provenance(self.event.clone(), ctx.provenance);
            self.prepared = Some(if nonempty(&self.event.mana) {
                crate::events::processing::process_trait_event_with_execution_context(
                    game, event, ctx,
                )?
            } else {
                TraitEventResult::Proceed(event)
            });
        }
        let mut original = self.prepared.as_ref().expect("prepared loss result");
        while let TraitEventResult::Expanded {
            original: nested, ..
        } = original
        {
            original = nested;
        }
        if matches!(original, TraitEventResult::Replaced { .. }) {
            return Err(ExecutionError::InternalError(
                "mana-loss replacement program has no simultaneous original/completion contract"
                    .into(),
            ));
        }
        Ok(())
    }
    fn commit_original(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        if self.prepared.is_none() {
            self.prepare_original(game, ctx)?;
        }
        let prepared = self.prepared.take().expect("prepared mana loss");
        (*self).commit_resolved(game, ctx, prepared)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let receipt = self.commit_original(game, ctx)?;
        complete_loss(game, ctx, receipt)
    }
}
struct ManaLossCompletion {
    programs: Vec<PreparedReplacementProgram>,
}
impl SimultaneousEffectCompletion for ManaLossCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn freeze(&mut self, _game: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let outputs = crate::effects::CompletedEffectOutputs::aggregate_only(original);
        crate::effects::replacement::complete_replacement_programs_with_original_outputs(
            game,
            ctx,
            outputs,
            |game, ctx, original| {
                crate::effects::replacement::complete_deferred_replacement_programs(
                    game,
                    ctx,
                    original,
                    self.programs,
                )
            },
        )
    }
}
fn complete_loss(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: SimultaneousEffectCommit,
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcomes =
        crate::effects::composition::execute_simultaneous_originals(game, ctx, false, |_, _| {
            Ok(vec![receipt])
        })?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }
    outcomes
        .pop()
        .ok_or_else(|| ExecutionError::InternalError("mana loss lost its original receipt".into()))
}

pub(crate) fn execute_mana_losses(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut players: Vec<PlayerId>,
    boundary: bool,
) -> Result<EffectOutcome, ExecutionError> {
    crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let order = game.team_apnap_player_order();
        players.sort_by_key(|player| {
            order
                .iter()
                .position(|id| id == player)
                .unwrap_or(usize::MAX)
        });
        players.dedup();
        let mut proposals = players
            .into_iter()
            .map(|player| ManaLossProposal::new(game, player, boundary))
            .collect::<Result<Vec<_>, _>>()?;
        for proposal in &mut proposals {
            proposal.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        let outcomes = crate::effects::composition::execute_simultaneous_originals(
            game,
            ctx,
            true,
            |game, ctx| {
                let mut receipts = Vec::with_capacity(proposals.len());
                for proposal in proposals {
                    receipts.push(Box::new(proposal).commit_original(game, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(Vec::new());
                    }
                }
                Ok(receipts)
            },
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        crate::events::damage::checked_damage_count(
            outcomes
                .iter()
                .filter_map(|outcome| outcome.instruction_result().as_count())
                .map(|count| count.max(0) as u128)
                .sum(),
            "simultaneous lost mana receipt",
        )?;
        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::effects::EffectExecutor;
    use crate::ids::{CardId, ObjectId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::PlayerFilter;
    use crate::{CardType, Zone};
    const A: PlayerId = PlayerId(0); const B: PlayerId = PlayerId(1);
    fn game() -> GameState { crate::tests::test_helpers::setup_two_player_game() }
    fn converter(game: &mut GameState, player: PlayerId, symbol: ManaSymbol) {
        game.effect_store.replacement_effects.add_effect(ReplacementEffect::with_matcher(
            ObjectId::from_raw(91), player, crate::events::mana::matchers::ManaLossMatcher { player: PlayerFilter::Specific(player) },
            ReplacementAction::ConvertUnspentMana(symbol)));
    }
    fn restricted(symbol: ManaSymbol, source: ObjectId, kind: CardType) -> RestrictedManaUnit {
        RestrictedManaUnit { source_controller: Some(A), symbol, source, source_chosen_creature_type: None,
            restrictions: vec![crate::ability::ManaUsageRestriction::CastSpell { card_types: vec![kind], subtype_requirement: None,
                restrict_to_matching_spell: true, grant_uncounterable: false, enters_with_counters: vec![], granted_abilities: vec![] }] }
    }
    #[test]
    fn conversion_pairs_colliding_colors_with_their_exact_snapshots_and_restrictions() {
        let mut game = game();
        let card = crate::CardBuilder::new(CardId::new(), "Producer").card_types(vec![CardType::Creature]).build();
        let source = game.create_object_from_card(&card, A, Zone::Battlefield);
        let mut snow = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        snow.supertypes.push(crate::types::Supertype::Snow);
        let ordinary = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let player = game.player_mut(A).unwrap();
        player.add_restricted_mana_with_snapshot_and_retention(restricted(ManaSymbol::Blue, source, CardType::Creature), Some(snow.clone()),
            Some(ironsmith_core::ManaRetentionDuration::EndOfCombat));
        player.add_restricted_mana_with_snapshot_and_retention(restricted(ManaSymbol::Red, source, CardType::Instant), Some(ordinary.clone()), None);
        converter(&mut game, A, ManaSymbol::Black);
        let outcome = execute_mana_losses(&mut game, &mut ExecutionContext::new_default(source, A), vec![A], false).unwrap();
        assert_eq!(outcome.as_count(), Some(0));
        assert!(outcome.events.iter().all(|event| event.downcast::<crate::events::ManaAddedEvent>().is_none()));
        let player = game.player(A).unwrap();
        assert_eq!(player.mana_pool.black, 2);
        assert_eq!(player.mana_source_provenance[0].snapshot, Some(snow));
        assert_eq!(player.mana_source_provenance[1].snapshot, Some(ordinary));
        assert_eq!(player.mana_source_provenance[0].retention, Some(ironsmith_core::ManaRetentionDuration::EndOfCombat));
        assert_eq!(player.restricted_mana[0].symbol, ManaSymbol::Black);
        assert_eq!(player.restricted_mana[1].symbol, ManaSymbol::Black);
        let spell = game.create_object_from_card(&card, A, Zone::Stack);
        let cost = crate::mana::ManaCost::from_pips(vec![vec![ManaSymbol::Snow]]);
        assert!(game.try_pay_mana_cost_with_reason(A, Some(spell), &cost, 0, crate::costs::PaymentReason::CastSpell).expect("checked fixture mana payment"));
        let player = game.player(A).unwrap();
        assert_eq!(player.mana_pool.black, 1);
        assert_eq!(player.restricted_mana, vec![restricted(ManaSymbol::Black, source, CardType::Instant)]);
        assert!(!player.mana_source_provenance[0].snapshot.as_ref().unwrap().supertypes.contains(&crate::types::Supertype::Snow));
        assert_eq!(player.mana_source_provenance[0].retention, None);
    }
    #[test]
    fn representation_exhaustion_rolls_back_instead_of_truncating_conversion() {
        let mut game = game(); converter(&mut game, A, ManaSymbol::Colorless);
        game.player_mut(A).unwrap().mana_pool.blue = u32::MAX;
        game.player_mut(A).unwrap().mana_pool.red = 1;
        let before = game.player(A).unwrap().mana_pool.clone();
        let result = execute_mana_losses(&mut game, &mut ExecutionContext::new_default(ObjectId::from_raw(92), A), vec![A], false);
        assert!(result.as_ref().err().is_some_and(ExecutionError::is_incomplete_execution));
        assert_eq!(game.player(A).unwrap().mana_pool, before);
    }
    struct Pause { pending: bool, players: Vec<PlayerId> }
    impl DecisionMaker for Pause {
        fn decide_options(&mut self, _: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.pending = true; self.players.push(ctx.player); Vec::new()
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    #[test]
    fn affected_player_choice_holds_the_whole_multi_player_original() {
        let mut game = game(); converter(&mut game, B, ManaSymbol::Black); converter(&mut game, B, ManaSymbol::Red);
        game.player_mut(A).unwrap().mana_pool.green = 2;
        game.player_mut(B).unwrap().mana_pool.blue = 3;
        let mut dm = Pause { pending: false, players: vec![] };
        game.empty_mana_pools_with_dm(&mut dm).unwrap();
        assert_eq!(dm.players, vec![B]); assert_eq!(game.player(A).unwrap().mana_pool.green, 2);
        assert_eq!(game.player(B).unwrap().mana_pool.blue, 3);
        assert!(matches!(game.empty_mana_pools(), Err(ExecutionError::UnresolvedPlayerDecision { player: B, .. })));
        assert_eq!(game.player(A).unwrap().mana_pool.green, 2);
        game.empty_mana_pools_with_dm(&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 3);
    }
    #[test]
    fn added_program_waits_until_every_original_pool_was_emptied() {
        let mut game = game();
        let replacement = ReplacementEffect::with_matcher(ObjectId::from_raw(99), A,
            crate::events::mana::matchers::ManaLossMatcher { player: PlayerFilter::Specific(A) },
            ReplacementAction::Additionally(vec![crate::effect::Effect::gain_life(crate::effect::Value::UnspentMana(PlayerFilter::Specific(B)))]));
        game.effect_store.replacement_effects.add_effect(replacement);
        game.player_mut(A).unwrap().mana_pool.green = 2; game.player_mut(B).unwrap().mana_pool.red = 3;
        let life = game.player(A).unwrap().life;
        game.empty_mana_pools_with_dm(&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().life, life);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
    }
    #[test]
    fn boundary_expiration_precedes_conversion_and_never_revives_a_unit_duration() {
        let mut game = game(); let source = ObjectId::from_raw(92);
        converter(&mut game, A, ManaSymbol::Black);
        game.player_mut(A).unwrap().add_unrestricted_mana_with_retention(ManaSymbol::Red, source, None,
            Some(ironsmith_core::ManaRetentionDuration::EndOfCombat));
        game.turn.phase = crate::Phase::Combat; game.turn.step = Some(crate::Step::DeclareAttackers);
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.red, 1);
        game.turn.step = Some(crate::Step::EndCombat); game.empty_mana_pools().unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.black, 1);
        assert_eq!(game.player(A).unwrap().mana_source_provenance[0].retention, None);
        game.effect_store.replacement_effects = Default::default();
        game.turn.phase = crate::Phase::NextMain; game.turn.step = None;
        game.empty_mana_pools().unwrap(); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[cfg(test)]
mod direct_owner_resource_tests {
    use super::*;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::ids::{CardId, ObjectId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::PlayerFilter;
    #[test]
    fn direct_boundary_shares_one_budget_across_all_players_added_programs() {
        for limit in [1, 2] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let token = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Loss token")
                .token().card_types(vec![crate::CardType::Artifact]).build();
            for player in [PlayerId(0), PlayerId(1)] {
                game.player_mut(player).unwrap().mana_pool.green = 1;
                game.effect_store.replacement_effects.add_effect(ReplacementEffect::with_matcher(
                    ObjectId::from_raw(90 + player.index() as u64), player,
                    crate::events::mana::matchers::ManaLossMatcher { player: PlayerFilter::Specific(player) },
                    ReplacementAction::Additionally(vec![crate::effect::Effect::create_tokens(token.clone(), 1)])));
            }
            let before_id = game.next_object_id_counter();
            game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits { max_created_tokens: limit, ..Default::default() });
            let result = game.empty_mana_pools_with_dm(&mut SelectFirstDecisionMaker);
            if limit == 1 {
                assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. })));
                assert!(game.battlefield.is_empty()); assert_eq!(game.next_object_id_counter(), before_id);
                assert!(game.players.iter().all(|player| player.mana_pool.green == 1));
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                result.unwrap(); assert_eq!(game.battlefield.len(), 2);
                assert!(game.players.iter().all(|player| player.mana_pool.total() == 0));
            }
        }
    }
    #[test]
    fn prevented_loss_cannot_revive_expired_unit_retention() {
        for (phase, step, duration) in [
            (crate::Phase::Combat, crate::Step::EndCombat, ironsmith_core::ManaRetentionDuration::EndOfCombat),
            (crate::Phase::Ending, crate::Step::Cleanup, ironsmith_core::ManaRetentionDuration::EndOfTurn),
        ] {
            let mut game = crate::tests::test_helpers::setup_two_player_game(); let player = PlayerId(0);
            game.player_mut(player).unwrap().add_unrestricted_mana_with_retention(
                ManaSymbol::Green, ObjectId::from_raw(91), None, Some(duration));
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                ObjectId::from_raw(90), player,
                crate::events::mana::matchers::ManaLossMatcher { player: PlayerFilter::Specific(player) }, ReplacementAction::Prevent));
            game.turn.phase = phase; game.turn.step = Some(step);
            game.empty_mana_pools().unwrap();
            assert_eq!(game.player(player).unwrap().mana_pool.green, 1);
            assert_eq!(game.player(player).unwrap().mana_source_provenance[0].retention, None);
            game.turn.phase = crate::Phase::FirstMain; game.turn.step = None;
            game.empty_mana_pools().unwrap(); assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        }
    }

    #[test]
    fn unsupported_instead_program_is_rejected_before_any_original_pool_changes() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        for player in [PlayerId(0), PlayerId(1)] { game.player_mut(player).unwrap().mana_pool.green = 2; }
        game.effect_store.replacement_effects.add_effect(ReplacementEffect::with_matcher(
            ObjectId::from_raw(90), PlayerId(1),
            crate::events::mana::matchers::ManaLossMatcher { player: PlayerFilter::Specific(PlayerId(1)) },
            ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(3)])));
        assert!(matches!(game.empty_mana_pools_with_dm(&mut SelectFirstDecisionMaker), Err(ExecutionError::InternalError(message))
            if message.contains("simultaneous original/completion")));
        assert!(game.players.iter().all(|player| player.mana_pool.green == 2 && player.life == 20));
    }
}
