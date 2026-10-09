//! Reselect which player or permanent an attacking creature is attacking.

use crate::combat_state::AttackTarget;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type ReselectAttackTargetEffect = ironsmith_core::ReselectAttackTargetEffect;

fn describe_attack_target(game: &GameState, target: &AttackTarget) -> String {
    match target {
        AttackTarget::Player(player) => game
            .player(*player)
            .map(|player| player.name.to_string())
            .unwrap_or_else(|| format!("player {}", player.0)),
        AttackTarget::Planeswalker(object_id) | AttackTarget::Battle(object_id) => game
            .object(*object_id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| format!("permanent #{}", object_id.0)),
        AttackTarget::Nothing { .. } => "nothing".to_string(),
    }
}

impl EffectExecutor for ReselectAttackTargetEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        crate::effects::composition::execute_result_transaction(game, ctx, |game, ctx| {
            let creatures = resolve_objects_for_effect(game, ctx, &self.target)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            // "Those creatures are now attacking that player": the new
            // attack is fixed when the effect resolves (CR 506.4).
            let fixed_player = match &self.attacked_player {
                Some(filter) => {
                    match crate::effects::helpers::resolve_player_filter(game, filter, ctx) {
                        Ok(player) => Some(player),
                        // The named player is gone (an illegal target):
                        // no attack changes.
                        Err(_) => return Ok(EffectOutcome::count(0)),
                    }
                }
                None => None,
            };
            let mut changed = 0;
            for creature in creatures {
                let Some(current) = game.combat.as_ref().and_then(|combat| {
                    combat
                        .attackers
                        .iter()
                        .find(|info| info.creature == creature)
                        .map(|info| info.target.clone())
                }) else {
                    // Only a creature still attacking has an attack to redirect.
                    continue;
                };
                let Some(controller) = game.object(creature).map(|object| game.controller_of(object))
                else {
                    continue;
                };
                // CR 508.1b: the players, planeswalkers and battles it could
                // attack; "which player" limits the choice to players.
                let mut targets =
                    super::enter_attacking::enters_attacking_targets(game, controller);
                if self.players_only {
                    targets.retain(|target| matches!(target, AttackTarget::Player(_)));
                }
                if let Some(player) = fixed_player {
                    // Only a player the creature's controller could attack
                    // with it; otherwise the creature keeps its attack.
                    targets.retain(|target| *target == AttackTarget::Player(player));
                }
                if targets.is_empty() {
                    continue;
                }
                let chosen = if targets.len() == 1 {
                    targets[0].clone()
                } else {
                    let options = targets
                        .iter()
                        .enumerate()
                        .map(|(index, target)| {
                            crate::decisions::DisplayOption::new(
                                index,
                                describe_attack_target(game, target),
                            )
                        })
                        .collect();
                    // "You may reselect ...": the effect's controller
                    // chooses, among what the creature's controller could
                    // attack with it.
                    let source = ctx.source;
                    let chooser = ctx.controller;
                    let selected = crate::decisions::make_decision(
                        game,
                        &mut *ctx.decision_maker,
                        chooser,
                        Some(source),
                        crate::decisions::ChoiceSpec::single(source, options),
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    let index = selected.into_iter().next().unwrap_or(0);
                    targets.get(index).cloned().unwrap_or_else(|| targets[0].clone())
                };
                if chosen == current {
                    continue;
                }
                if let Some(info) = game.combat.as_mut().and_then(|combat| {
                    combat
                        .attackers
                        .iter_mut()
                        .find(|info| info.creature == creature)
                }) {
                    info.target = chosen;
                    changed += 1;
                }
            }
            // CR 506.4e: a newly attacked planeswalker or battle records the
            // types it is attacked as.
            if changed > 0
                && let Some(mut combat) = game.combat.take()
            {
                combat.record_attacked_permanent_types(game);
                game.combat = Some(combat);
            }
            Ok(EffectOutcome::count(changed))
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "attacking creature"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::combat_state::AttackerInfo;
    use crate::ids::{CardId, PlayerId};
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn attack_of(game: &GameState, creature: crate::ids::ObjectId) -> AttackTarget {
        game.combat
            .as_ref()
            .and_then(|combat| combat.attackers.iter().find(|info| info.creature == creature))
            .map(|info| info.target.clone())
            .expect("still attacking")
    }

    #[test]
    fn now_attacking_redirects_to_the_named_player_without_a_choice() {
        // CR 506.4: "Those creatures are now attacking that player."
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let card = CardBuilder::new(CardId::from_raw(1), "Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let attacker = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let mut combat = crate::combat_state::CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        // A player the creature's controller can't attack leaves the attack.
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), bob);
        ReselectAttackTargetEffect::new(ChooseSpec::SpecificObject(attacker), false)
            .now_attacking(PlayerFilter::Specific(alice))
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(attack_of(&game, attacker), AttackTarget::Player(bob));

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), bob);
        let outcome = ReselectAttackTargetEffect::new(ChooseSpec::SpecificObject(attacker), false)
            .now_attacking(PlayerFilter::Specific(carol))
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(attack_of(&game, attacker), AttackTarget::Player(carol));
    }
}
