//! Exact current-or-last combat actors (CR 508.5/508.7).
use super::GameState;
use crate::combat_state::{AttackTarget, AttackingRoleId, CombatState, DefendingPlayerReference,
    DefendingPlayersId, RetainedAttackingRole, defending_player_for_attack_target};
use crate::effects::ExecutionError;
use crate::ids::{ObjectId, PlayerId};

impl GameState {
    /// Current target of the exact attacking tenure retained by an event.
    /// A later attack by the same ObjectId is a different tenure.
    pub(crate) fn active_attack_target_for_reference(&self, reference: DefendingPlayerReference)
        -> Result<Option<&AttackTarget>, ExecutionError>
    {
        let DefendingPlayerReference::Attacker { attacker, role } = reference else {
            return Err(ExecutionError::IncompleteEvidence(
                "current attacking predicate requires the exact retained attacking tenure".into()));
        };
        self.combat_transients.attacking_roles.get(role.0)
            .filter(|record| record.attacker == attacker)
            .ok_or_else(|| ExecutionError::IncompleteEvidence(
                "current attacking predicate has no retained attacking tenure".into()))?;
        Ok((self.combat_transients.current_attacking_roles.get(&attacker) == Some(&role))
            .then(|| self.current_attack_target_across_lanes(attacker)).flatten())
    }

    /// Capture before replacement programs or combat removal discard evidence.
    pub(crate) fn retain_attacking_role(&mut self, attacker: ObjectId, target: &AttackTarget)
        -> DefendingPlayerReference
    {
        let defender = defending_player_for_attack_target(self, target);
        let state = self.combat_transients_mut();
        let role = match state.current_attacking_roles.get(&attacker).copied() {
            Some(role) => { state.attacking_roles[role.0].last_defender = defender; role }
            None => {
                let role = AttackingRoleId(state.attacking_roles.len());
                state.attacking_roles.push(RetainedAttackingRole { attacker, last_defender: defender });
                state.current_attacking_roles.insert(attacker, role);
                role
            }
        };
        DefendingPlayerReference::Attacker { attacker, role }
    }

    pub(crate) fn retain_combat_damage_role(&mut self, combat: &CombatState, source: ObjectId)
        -> DefendingPlayerReference
    {
        if let Some(target) = crate::combat_state::get_attack_target(combat, source) {
            self.retain_attacking_role(source, target)
        } else if crate::combat_state::is_blocking(combat, source) {
            let attacking_player = self.turn.active_player;
            let players = self.players.iter().filter(|player| player.is_in_game())
                .filter(|player| self.are_opponents(attacking_player, player.id)
                    && self.player_is_within_range(attacking_player, player.id)
                    && self.attack_direction_allows_defender(attacking_player, player.id))
                .map(|player| player.id).collect();
            let state = self.combat_transients_mut();
            let defenders = DefendingPlayersId(state.combat_defending_players.len());
            state.combat_defending_players.push((attacking_player, players));
            DefendingPlayerReference::CombatOpponents { attacking_player, defenders }
        } else { DefendingPlayerReference::Missing }
    }

    pub(crate) fn retire_attacking_role(&mut self, attacker: ObjectId) {
        if let Some(target) = self.current_attack_target_across_lanes(attacker).cloned() {
            self.retain_attacking_role(attacker, &target);
        }
        self.combat_transients_mut().current_attacking_roles.remove(&attacker);
    }

    /// The runner calls this after synchronizing its authoritative combat copy.
    pub(crate) fn retain_ending_combat(&mut self, combat: &CombatState) {
        for info in &combat.attackers {
            self.retain_attacking_role(info.creature, &info.target);
            self.combat_transients_mut().current_attacking_roles.remove(&info.creature);
        }
    }

    pub fn defending_player_candidates(&self, reference: DefendingPlayerReference)
        -> Result<Vec<PlayerId>, ExecutionError>
    {
        let missing = || ExecutionError::IncompleteEvidence(
            "defending-player reference lacks the exact combat participant's retained role".into());
        let player = match reference {
            DefendingPlayerReference::Selected(player) => return Ok(self.player(player)
                .filter(|player| player.is_in_game()).map(|_| vec![player]).unwrap_or_default()),
            DefendingPlayerReference::KnownAbsent => return Ok(Vec::new()),
            DefendingPlayerReference::Attacker { attacker, role } => {
                let record = self.combat_transients.attacking_roles.get(role.0)
                    .filter(|record| record.attacker == attacker).ok_or_else(missing)?;
                if self.combat_transients.current_attacking_roles.get(&attacker) == Some(&role) {
                    if let Some(target) = self.current_attack_target_across_lanes(attacker) {
                        defending_player_for_attack_target(self, target).ok_or_else(missing)?
                    } else { record.last_defender.ok_or_else(missing)? }
                } else { record.last_defender.ok_or_else(missing)? }
            }
            DefendingPlayerReference::LegacyAttack { attacker, player } => {
                // The old event proves its direct destination, but cannot name
                // a tracked attacking tenure. Do not silently attach it to a
                // later attack by the same ObjectId, even without a zone move.
                if self.current_attack_target_across_lanes(attacker).is_some()
                    || self.combat_transients.attacking_roles.iter().any(|record| record.attacker == attacker)
                { return Err(missing()); }
                player
            }
            DefendingPlayerReference::CombatOpponents { attacking_player, defenders } => {
                let (_, players) = self.combat_transients.combat_defending_players.get(defenders.0)
                    .filter(|(actor, _)| *actor == attacking_player).ok_or_else(missing)?;
                return Ok(players.iter().copied().filter(|player|
                    self.player(*player).is_some_and(|player| player.is_in_game())).collect());
            }
            DefendingPlayerReference::Missing => return Err(missing()),
        };
        // A departed player is known absence, not missing recovery evidence.
        // CR 508.5 / 805.10e: an exact attacking creature identifies one
        // defending player even in shared-team turns. Its teammate is not an
        // alternative actor for this reference.
        Ok(self.player(player).filter(|player| player.is_in_game())
            .map(|_| vec![player]).unwrap_or_default())
    }

    /// Compatibility uses only evidence present in the old event. Never
    /// reconstruct a departed permanent's role from its new controller or
    /// substitute a redirected damage recipient.
    pub(crate) fn defending_reference_for_event(&self, event: &crate::triggers::TriggerEvent)
        -> Option<DefendingPlayerReference>
    {
        use crate::triggers::AttackEventTarget;
        if let Some(reference) = event.defending_player_reference() { return Some(reference); }
        let attack = if let Some(attack) = event.downcast::<crate::events::CreatureAttackedEvent>() {
            Some((attack.attacker, Some(attack.target)))
        } else if let Some(attack) = event.downcast::<crate::events::CreatureAttackedAndUnblockedEvent>() {
            Some((attack.attacker, Some(attack.target)))
        } else { event.downcast::<crate::events::CreatureBecameBlockedEvent>()
            .map(|attack| (attack.attacker, attack.attack_target)) };
        if let Some((attacker, target)) = attack {
            return Some(match target {
                Some(AttackEventTarget::Player(player)) => DefendingPlayerReference::LegacyAttack { attacker, player },
                _ => DefendingPlayerReference::Missing,
            });
        }
        event.downcast::<crate::events::DamageEvent>().filter(|damage| damage.is_combat)
            .map(|_| DefendingPlayerReference::Missing)
    }
}

#[cfg(test)]
mod tests {
    // Reconstructed source scenarios; UNRUN.
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;
    use crate::types::CardType;
    use crate::zone::Zone;
    fn creature(game:&mut GameState,controller:PlayerId)->ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(),"Combat participant")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,4)).build(),controller,Zone::Battlefield)
    }
    #[test]
    fn removed_and_reentered_attacker_does_not_rebind_its_earlier_tenure() {
        let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let attacker=creature(&mut game,PlayerId(0));
        game.add_entering_attacker(attacker,AttackTarget::Player(PlayerId(1)));let first=game.retain_attacking_role(attacker,&AttackTarget::Player(PlayerId(1)));let saved=game.clone();
        game.combat.as_mut().unwrap().attackers[0].target=AttackTarget::Player(PlayerId(2));assert_eq!(game.defending_player_candidates(first).unwrap(),vec![PlayerId(2)]);
        game.remove_object_from_combat(attacker);game.add_entering_attacker(attacker,AttackTarget::Player(PlayerId(1)));let second=game.retain_attacking_role(attacker,&AttackTarget::Player(PlayerId(1)));
        assert_ne!(first,second);assert_eq!(game.defending_player_candidates(first).unwrap(),vec![PlayerId(2)]);assert_eq!(game.defending_player_candidates(second).unwrap(),vec![PlayerId(1)]);
        assert_eq!(saved.defending_player_candidates(first).unwrap(),vec![PlayerId(1)]);
        let DefendingPlayerReference::Attacker{role,..}=first else{unreachable!()};let other=creature(&mut game,PlayerId(0));
        assert!(matches!(game.defending_player_candidates(DefendingPlayerReference::Attacker{attacker:other,role}),Err(ExecutionError::IncompleteEvidence(_))));
    }
    #[test]
    fn legacy_direct_player_evidence_does_not_follow_a_returned_stable_card() {
        let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let attacker=creature(&mut game,PlayerId(0));
        let event=crate::triggers::TriggerEvent::new_with_provenance(crate::events::CreatureAttackedEvent::new(attacker,crate::triggers::AttackEventTarget::Player(PlayerId(1))),Default::default());
        let reference=game.defending_reference_for_event(&event).unwrap();let graveyard=game.move_object_by_effect(attacker,Zone::Graveyard).unwrap();let returned=game.move_object_by_effect(graveyard,Zone::Battlefield).unwrap();
        game.add_entering_attacker(returned,AttackTarget::Player(PlayerId(2)));assert_eq!(game.defending_player_candidates(reference).unwrap(),vec![PlayerId(1)]);
        let missing=crate::triggers::TriggerEvent::new_with_provenance(crate::events::CreatureAttackedEvent::new(attacker,crate::triggers::AttackEventTarget::Planeswalker(ObjectId::from_raw(99999))),Default::default());
        assert_eq!(game.defending_reference_for_event(&missing),Some(DefendingPlayerReference::Missing));
    }
    #[test]
    fn unstamped_legacy_event_cannot_rebind_to_a_later_tenure_of_the_same_object() {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let attacker = creature(&mut game, PlayerId(0));
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::CreatureAttackedEvent::new(attacker,
                crate::triggers::AttackEventTarget::Player(PlayerId(1))), Default::default());
        game.add_entering_attacker(attacker, AttackTarget::Player(PlayerId(1)));
        game.remove_object_from_combat(attacker);
        game.add_entering_attacker(attacker, AttackTarget::Player(PlayerId(2)));
        let reference = game.defending_reference_for_event(&event).unwrap();
        assert!(matches!(reference, DefendingPlayerReference::LegacyAttack { player: PlayerId(1), .. }));
        assert!(matches!(game.defending_player_candidates(reference), Err(ExecutionError::IncompleteEvidence(_))));
        game.remove_object_from_combat(attacker);
        assert!(matches!(game.defending_player_candidates(reference), Err(ExecutionError::IncompleteEvidence(_))));
    }

    #[test]
    fn exact_shared_team_attacker_and_scalar_bind_without_choosing_a_teammate() {
        struct NoChoice;
        impl crate::decision::DecisionMaker for NoChoice {
            fn decide_options(&mut self, _: &GameState, _: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                panic!("an exact defender is not a teammate choice");
            }
        }
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        game.set_teams(vec![vec![PlayerId(0), PlayerId(1)], vec![PlayerId(2), PlayerId(3)]]).unwrap();
        game.enable_shared_team_turns().unwrap();
        let attacker = creature(&mut game, PlayerId(0));
        game.add_entering_attacker(attacker, AttackTarget::Player(PlayerId(2)));
        let reference = game.retain_attacking_role(attacker, &AttackTarget::Player(PlayerId(2)));
        assert_eq!(game.defending_player_candidates(reference).unwrap(), vec![PlayerId(2)]);
        let mut dm = NoChoice;
        let mut ctx = crate::effects::ExecutionContext::new(attacker, PlayerId(0), &mut dm);
        ctx.combat.defending_player_reference = Some(reference);
        assert!(ctx.bind_defending_player(&game).unwrap());
        assert_eq!(ctx.defending_players(&game).unwrap(), vec![PlayerId(2)]);
        let checkpoint = crate::effects::ExecutionContextCheckpoint::capture(&ctx);
        ctx.combat.defending_player = Some(PlayerId(3));
        checkpoint.restore(&mut ctx);
        assert_eq!(ctx.defending_players(&game).unwrap(), vec![PlayerId(2)]);
        ctx.combat.defending_player_reference = None;
        assert!(ctx.bind_defending_player(&game).unwrap());
        assert_eq!(ctx.defending_players(&game).unwrap(), vec![PlayerId(2)]);
    }
    #[test]
    fn shared_team_reselection_tracks_the_exact_new_player_even_within_the_same_team() {
        let mut game = GameState::new((0..6).map(|id| format!("Seat {id}")).collect(), 20);
        game.set_teams(vec![vec![PlayerId(0), PlayerId(1)], vec![PlayerId(2), PlayerId(3)], vec![PlayerId(4), PlayerId(5)]]).unwrap();
        game.enable_shared_team_turns().unwrap();
        let attacker = creature(&mut game, PlayerId(0));
        game.add_entering_attacker(attacker, AttackTarget::Player(PlayerId(2)));
        let reference = game.retain_attacking_role(attacker, &AttackTarget::Player(PlayerId(2)));
        for player in [PlayerId(2), PlayerId(3), PlayerId(4)] {
            game.combat.as_mut().unwrap().attackers[0].target = AttackTarget::Player(player);
            assert_eq!(game.defending_player_candidates(reference).unwrap(), vec![player]);
        }
        game.remove_object_from_combat(attacker);
        assert_eq!(game.defending_player_candidates(reference).unwrap(), vec![PlayerId(4)]);
    }
    #[test]
    fn grand_melee_keeps_the_event_lane_defenders_and_removes_suspended_participants() {
        let seats=(0..8).map(PlayerId).collect::<Vec<_>>();let mut game=GameState::new(seats.iter().map(|player|format!("Seat {}",player.0)).collect(),20);game.restore_grand_melee(seats).unwrap();
        let attacker=creature(&mut game,PlayerId(0));let blocker=creature(&mut game,PlayerId(1));game.add_entering_attacker(attacker,AttackTarget::Player(PlayerId(1)));game.combat.as_mut().unwrap().blockers.insert(attacker,vec![blocker]);
        let combat=game.combat.clone().unwrap();let attacking=game.retain_combat_damage_role(&combat,attacker);let blocking=game.retain_combat_damage_role(&combat,blocker);let candidates=game.defending_player_candidates(blocking).unwrap();assert_eq!(candidates,vec![PlayerId(1)]);
        game.select_grand_melee_turn_marker(2).unwrap();assert_eq!(game.turn.active_player,PlayerId(4));assert_eq!(game.defending_player_candidates(attacking).unwrap(),vec![PlayerId(1)]);assert_eq!(game.defending_player_candidates(blocking).unwrap(),candidates);
        game.set_current_controller(attacker,PlayerId(4)).unwrap();assert_eq!(game.defending_player_candidates(attacking).unwrap(),vec![PlayerId(1)]);game.select_grand_melee_turn_marker(1).unwrap();assert!(game.combat.as_ref().unwrap().attackers.is_empty());
    }
    #[test]
    fn suspended_planeswalker_destination_keeps_its_previous_controller() {
        let seats=(0..8).map(PlayerId).collect::<Vec<_>>();let mut game=GameState::new(seats.iter().map(|player|format!("Seat {}",player.0)).collect(),20);game.restore_grand_melee(seats).unwrap();let attacker=creature(&mut game,PlayerId(0));
        let planeswalker=game.create_object_from_card(&CardBuilder::new(CardId::new(),"Planeswalker").card_types(vec![CardType::Planeswalker]).build(),PlayerId(1),Zone::Battlefield);
        game.add_entering_attacker(attacker,AttackTarget::Planeswalker(planeswalker));let reference=game.retain_attacking_role(attacker,&AttackTarget::Planeswalker(planeswalker));game.refresh_continuous_state().unwrap();
        game.select_grand_melee_turn_marker(2).unwrap();game.set_current_controller(planeswalker,PlayerId(4)).unwrap();assert_eq!(game.defending_player_candidates(reference).unwrap(),vec![PlayerId(1)]);game.select_grand_melee_turn_marker(1).unwrap();
        assert!(matches!(game.combat.as_ref().unwrap().attackers[0].target,AttackTarget::Nothing{defending_player:Some(PlayerId(1)),..}));
    }
}
