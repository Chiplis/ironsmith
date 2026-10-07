//! Current combat predicates retain their event's actor or attacking tenure.
//! Past-tense declaration predicates read only the completed declaration.
use super::*;
use crate::combat_state::AttackTarget;
use crate::events::{CreatureAttackedEvent, PlayerAttackDeclarationEvent};
use crate::triggers::AttackEventTarget;
use ironsmith_core::CombatParticipantCondition as Combat;

pub(super) fn evaluate(
    game: &GameState,
    condition: Combat,
    ctx: &ConditionContext<'_, '_>,
) -> Result<bool, ExecutionError> {
    let shared = ctx.shared();
    let missing = || ExecutionError::IncompleteEvidence(
        "combat participant condition requires its completed attack declaration".into());
    let event = shared.triggering_event.ok_or_else(missing)?;
    match condition {
        Combat::YouAreDefendingPlayer => {
            // CR 508.5: exact current-or-last defender of the attacking
            // creature, including planeswalkers and Battles. Never substitute
            // the active player, another attacker, or a later incarnation.
            if event.downcast::<CreatureAttackedEvent>().is_none() { return Err(missing()); }
            let reference = game.defending_reference_for_event(event).ok_or_else(missing)?;
            let players = game.defending_player_candidates(reference)?;
            Ok(players.contains(&shared.controller))
        }
        Combat::TriggeringCreatureAttacksMostLifePlayer => {
            if event.downcast::<CreatureAttackedEvent>().is_none() { return Err(missing()); }
            let reference = game.defending_reference_for_event(event).ok_or_else(missing)?;
            let Some(AttackTarget::Player(player)) = game.active_attack_target_for_reference(reference)? else {
                return Ok(false);
            };
            let Some(defender) = game.player(*player).filter(|player| player.is_in_game()) else {
                return Ok(false);
            };
            Ok(game.players.iter().filter(|player| player.is_in_game())
                .all(|player| defender.life >= player.life))
        }
        Combat::AttackingPlayerAttackedYouOrYourPlaneswalker
        | Combat::AttackingPlayerIsNotAttackingYou
        | Combat::AnyAttackedPlayerIsPoisoned => {
            let attack = event.downcast::<PlayerAttackDeclarationEvent>().ok_or_else(missing)?;
            let declaration = attack.declaration.as_deref().ok_or_else(missing)?;
            // The receipt must contain the event's exact actor/defender pair.
            // An empty or mismatched retained set is damaged evidence.
            if !declaration.iter().any(|participant| participant.controller == attack.attacker
                && participant.defending_player == attack.defender
                && matches!(participant.target, AttackEventTarget::Player(_)) == attack.directly_attacked_player)
            { return Err(missing()); }
            match condition {
                Combat::AttackingPlayerAttackedYouOrYourPlaneswalker => Ok(declaration.iter().any(|participant|
                    participant.controller == attack.attacker && participant.defending_player == shared.controller
                        && matches!(participant.target, AttackEventTarget::Player(_) | AttackEventTarget::Planeswalker(_)))),
                Combat::AttackingPlayerIsNotAttackingYou => Ok(!game.combat.as_ref().is_some_and(|combat|
                    combat.attackers.iter().any(|attacker|
                        game.controller_of_id(attacker.creature) == Some(attack.attacker)
                            && attacker.target == AttackTarget::Player(shared.controller)))),
                Combat::AnyAttackedPlayerIsPoisoned => Ok(game.combat.as_ref().is_some_and(|combat|
                    combat.attackers.iter().any(|attacker| match attacker.target {
                        AttackTarget::Player(player) => game.player(player)
                            .is_some_and(|player| player.is_in_game() && player.poison_counters > 0),
                        _ => false,
                    }))),
                _ => unreachable!("three declaration predicates"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Source-authored regressions; deliberately UNRUN.
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::combat::DeclaredAttackParticipant;
    use crate::ids::CardId;
    use crate::types::CardType;
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);
    const C: PlayerId = PlayerId(2);
    fn game() -> GameState {
        GameState::new(vec!["A".into(), "B".into(), "C".into()], 20)
    }
    fn creature(game: &mut GameState, controller: PlayerId) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Participant")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build(),
            controller, Zone::Battlefield)
    }
    fn check(game: &GameState, source: ObjectId, event: &TriggerEvent, condition: Combat)
        -> Result<bool, ExecutionError>
    {
        evaluate_condition_external_checked(game, &Condition::CombatParticipant(condition),
            &ExternalEvaluationContext { source, controller: A, triggering_event: Some(event),
                ..Default::default() }, None)
    }
    fn declaration(attacker: ObjectId, target: AttackEventTarget, defender: PlayerId) -> TriggerEvent {
        TriggerEvent::new_with_provenance(PlayerAttackDeclarationEvent {
            attacker: B, defender, turn_number: 1, combat_phase: 1,
            directly_attacked_player: matches!(target, AttackEventTarget::Player(_)),
            declaration: Some(vec![DeclaredAttackParticipant {
                creature: attacker, controller: B, target, defending_player: defender,
            }].into()),
        }, Default::default())
    }
    #[test]
    fn past_declaration_survives_departure_and_native_branch_restore() {
        let mut game = game();
        let source = creature(&mut game, A);
        let attacker = creature(&mut game, B);
        let walker = creature(&mut game, A);
        let event = declaration(attacker, AttackEventTarget::Planeswalker(walker), A);
        let saved = game.clone();
        game.move_object_by_effect(walker, Zone::Graveyard).unwrap();
        game.move_object_by_effect(attacker, Zone::Graveyard).unwrap();
        for branch in [&game, &saved] {
            assert_eq!(check(branch, source, &event,
                Combat::AttackingPlayerAttackedYouOrYourPlaneswalker), Ok(true));
        }
        let battle = declaration(attacker, AttackEventTarget::Battle(walker), A);
        assert_eq!(check(&game, source, &battle,
            Combat::AttackingPlayerAttackedYouOrYourPlaneswalker), Ok(false));
    }
    #[test]
    fn current_roles_and_life_do_not_reuse_a_prior_attacking_tenure() {
        let mut game = game();
        let source = creature(&mut game, A);
        game.add_entering_attacker(source, AttackTarget::Player(B));
        let reference = game.retain_attacking_role(source, &AttackTarget::Player(B));
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source, AttackEventTarget::Player(B)), Default::default())
            .with_defending_player_reference(reference);
        assert_eq!(check(&game, source, &event, Combat::TriggeringCreatureAttacksMostLifePlayer), Ok(true));
        game.player_mut(C).unwrap().life = 21;
        assert_eq!(check(&game, source, &event, Combat::TriggeringCreatureAttacksMostLifePlayer), Ok(false));
        game.player_mut(B).unwrap().life = 22;
        assert_eq!(check(&game, source, &event, Combat::TriggeringCreatureAttacksMostLifePlayer), Ok(true));
        game.remove_object_from_combat(source);
        game.add_entering_attacker(source, AttackTarget::Player(B));
        game.retain_attacking_role(source, &AttackTarget::Player(B));
        assert_eq!(check(&game, source, &event, Combat::TriggeringCreatureAttacksMostLifePlayer), Ok(false));
    }
    #[test]
    fn present_attack_and_poison_predicates_recheck_direct_players_only() {
        let mut game = game();
        let source = creature(&mut game, A);
        let attacker = creature(&mut game, B);
        let event = declaration(attacker, AttackEventTarget::Player(C), C);
        game.add_entering_attacker(attacker, AttackTarget::Player(C));
        game.player_mut(C).unwrap().poison_counters = 1;
        assert_eq!(check(&game, source, &event, Combat::AttackingPlayerIsNotAttackingYou), Ok(true));
        assert_eq!(check(&game, source, &event, Combat::AnyAttackedPlayerIsPoisoned), Ok(true));
        game.player_mut(C).unwrap().poison_counters = 0;
        assert_eq!(check(&game, source, &event, Combat::AnyAttackedPlayerIsPoisoned), Ok(false));
        game.combat.as_mut().unwrap().attackers[0].target = AttackTarget::Player(A);
        assert_eq!(check(&game, source, &event, Combat::AttackingPlayerIsNotAttackingYou), Ok(false));
        game.player_mut(A).unwrap().poison_counters = 1;
        game.combat.as_mut().unwrap().attackers[0].target = AttackTarget::Planeswalker(source);
        assert_eq!(check(&game, source, &event, Combat::AnyAttackedPlayerIsPoisoned), Ok(false));
    }
    #[test]
    fn missing_declaration_and_negative_predicates_fail_with_typed_evidence() {
        let mut game = game();
        let source = creature(&mut game, A);
        let missing = TriggerEvent::new_with_provenance(PlayerAttackDeclarationEvent {
            attacker: B, defender: A, turn_number: 1, combat_phase: 1,
            directly_attacked_player: true, declaration: None,
        }, Default::default());
        for condition in [Combat::AttackingPlayerIsNotAttackingYou,
            Combat::AttackingPlayerAttackedYouOrYourPlaneswalker, Combat::AnyAttackedPlayerIsPoisoned] {
            assert!(matches!(check(&game, source, &missing, condition), Err(ExecutionError::IncompleteEvidence(_))));
        }
        let incomplete = TriggerEvent::new_with_provenance(CreatureAttackedEvent::new(source,
            AttackEventTarget::Planeswalker(source)), Default::default());
        assert!(matches!(check(&game, source, &incomplete, Combat::YouAreDefendingPlayer),
            Err(ExecutionError::IncompleteEvidence(_))));
        let poison = Condition::ValueComparison {
            left: Value::PlayerCounters(PlayerFilter::Defending, crate::object::CounterType::Poison),
            operator: crate::effect::ValueComparisonOperator::GreaterThan,
            right: Value::Fixed(0),
        };
        // A previously inferred live defender must not cover missing event
        // evidence or turn a negated numeric predicate into true.
        assert!(matches!(evaluate_condition_external_checked(&game, &Condition::Not(Box::new(poison)),
            &ExternalEvaluationContext { source, controller: A, defending_player: Some(A),
                triggering_event: Some(&incomplete), ..Default::default() }, None),
            Err(ExecutionError::IncompleteEvidence(_))));
    }
}
