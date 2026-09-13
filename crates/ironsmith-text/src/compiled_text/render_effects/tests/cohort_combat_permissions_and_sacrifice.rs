use super::*;
use crate::ability::AbilityKind;
use crate::effects::{EffectContext, execute_effect};
use crate::ids::CardId;
use crate::card::PowerToughness;
use crate::game_state::{GameState, Target};

fn body(name: &str, kind: CardType) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(0)]))
        .power_toughness(PowerToughness::fixed(10, 10)).build()
}
struct PayChoice(bool);
impl crate::decision::DecisionMaker for PayChoice {
    fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool { self.0 }
}

#[test]
fn cohort_attack_bonus_uses_defending_player_relationship_and_one_group_trigger() {
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Resistance")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creature tokens you control have haste.\nWhenever one or more creatures attack, you may pay {1}{R}. If you do, creatures attacking your opponents and/or planeswalkers they control get +2/+0 until end of turn.").unwrap();
    for pay in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let (alice, bob, charlie) = (game.players[0].id, game.players[1].id, game.players[2].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let own_walker = game.create_object_from_definition(&body("Own walker", CardType::Planeswalker), alice, Zone::Battlefield);
        let other_walker = game.create_object_from_definition(&body("Other walker", CardType::Planeswalker), bob, Zone::Battlefield);
        let battle = game.create_object_from_definition(&body("Battle", CardType::Battle), bob, Zone::Battlefield);
        let mut combat = CombatState::default();
        let mut recipients = Vec::new();
        for (controller, target, expected) in [
            (alice, AttackTarget::Player(bob), true),
            (bob, AttackTarget::Player(charlie), true),
            (charlie, AttackTarget::Planeswalker(other_walker), true),
            (bob, AttackTarget::Player(alice), false),
            (bob, AttackTarget::Planeswalker(own_walker), false),
            (alice, AttackTarget::Battle(battle), false),
        ] {
            let creature = game.create_object_from_definition(&body("Attacker", CardType::Creature), controller, Zone::Battlefield);
            combat.attackers.push(AttackerInfo { creature, target });
            recipients.push((creature, expected));
        }
        let idle = game.create_object_from_definition(&body("Idle", CardType::Creature), alice, Zone::Battlefield);
        recipients.push((idle, false));
        game.combat = Some(combat.clone());
        let mut count = 0;
        for info in &combat.attackers {
            let target = match info.target {
                AttackTarget::Player(p) => crate::triggers::AttackEventTarget::Player(p),
                AttackTarget::Planeswalker(o) => crate::triggers::AttackEventTarget::Planeswalker(o),
                AttackTarget::Battle(o) => crate::triggers::AttackEventTarget::Battle(o),
            };
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::combat::CreatureAttackedEvent::with_total_attackers(info.creature, target, 6),
                crate::provenance::ProvNodeId::default(),
            );
            count += crate::triggers::check_triggers(&game, &event).len();
        }
        assert_eq!(count, 1, "one trigger for the attack group");
        game.player_mut(alice).unwrap().mana_pool.add(crate::mana::ManaSymbol::Red, 2);
        let ability = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(a) => Some(a), _ => None }).unwrap();
        let mut choices = PayChoice(pay);
        game.stack.push(crate::game_state::StackEntry::ability(source, alice, ability.effects.clone()));
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        for (id, expected) in recipients {
            assert_eq!(game.current_characteristics(id).unwrap().power, Some(if pay && expected { 12 } else { 10 }), "pay={pay} id={id:?} expected={expected}");
        }
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), if pay { 0 } else { 2 });
    }
}

#[test]
fn cohort_landfall_damage_reuses_two_recipients_and_their_controller_relation() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Paired Blaze")
        .card_types(vec![CardType::Instant])
        .parse_text("This spell deals 1 damage to target player or planeswalker and 1 damage to target creature that player or that planeswalker's controller controls.\nLandfall — If you had a land enter the battlefield under your control this turn, this spell deals 3 damage to that player or planeswalker and 3 damage to that creature instead.").unwrap();
    for land_owner in [None, Some(0), Some(1)] {
        for walker in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Stack);
            let recipient = game.create_object_from_definition(&body("Creature", CardType::Creature), bob, Zone::Battlefield);
            let wrong = game.create_object_from_definition(&body("Other creature", CardType::Creature), alice, Zone::Battlefield);
            let pw = game.create_object_from_definition(&body("Walker", CardType::Planeswalker), bob, Zone::Battlefield);
            game.object_mut(pw).unwrap().counters.insert(crate::CounterType::Loyalty, 10);
            game.turn_store.turn_history.clear_for_new_turn();
            if let Some(owner) = land_owner {
                let land = game.create_object_from_definition(&body("Land", CardType::Land), game.players[owner].id, Zone::Hand);
                game.move_object_by_effect(land, Zone::Battlefield).unwrap();
            }
            let program = card.spell_effect.as_ref().unwrap();
            let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(&game, program, alice, Some(source), None);
            let candidate_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(crate::target::ObjectFilter::creature().controlled_by(crate::target::PlayerFilter::TargetPlayerOrControllerOfTarget)));
            assert!(crate::game_loop::compute_legal_targets(&game, &candidate_spec, alice, Some(source)).contains(&Target::Object(recipient)));
            assert_eq!(requirements.len(), 2, "{requirements:#?}");
            let contexts = requirements.iter().map(|r| {
                let mut c = crate::decisions::context::TargetRequirementContext::single(r.description.clone(), r.legal_targets.clone());
                c.shared_player_group = r.shared_player_group.clone();
                c
            }).collect::<Vec<_>>();
            let first = if walker { Target::Object(pw) } else { Target::Player(bob) };
            assert!(crate::targeting::validate_flat_target_assignment(&contexts, &[first, Target::Object(recipient)]));
            assert!(!crate::targeting::validate_flat_target_assignment(&contexts, &[first, Target::Object(wrong)]));
            let automatic = crate::targeting::normalize_targets_for_requirements(&contexts, vec![first]).unwrap();
            assert_eq!(automatic, vec![first, Target::Object(recipient)]);
            let crate::target::ChooseSpec::Object(filter) = requirements[1].spec.base() else { panic!("creature requirement") };
            let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![
                if walker { crate::effects::ResolvedTarget::Object(pw) } else { crate::effects::ResolvedTarget::Player(bob) },
            ]);
            let tag = crate::effects::TagMatchingObjectsEffect::new(filter.clone(), "eligible");
            execute_effect(&mut game, &crate::effect::Effect::new(tag), &mut ctx).unwrap();
            let eligible = ctx.get_tagged_all("eligible").unwrap();
            assert!(eligible.iter().any(|s| s.object_id == recipient));
            assert!(!eligible.iter().any(|s| s.object_id == wrong));
            game.stack.push(crate::game_state::StackEntry::new(source, alice)
                .with_targets(vec![if walker { Target::Object(pw) } else { Target::Player(bob) }, Target::Object(recipient)])
                .with_target_assignments(requirements.iter().enumerate().map(|(i, r)| crate::game_state::TargetAssignment { spec: r.spec.clone(), range: i..i+1 }).collect()));
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            let amount = if land_owner == Some(0) { 3 } else { 1 };
            assert_eq!(game.damage_on(recipient), amount);
            assert_eq!(game.player(bob).unwrap().life, if walker { 20 } else { 20 - amount as i32 });
            if walker { assert_eq!(game.object(pw).unwrap().counters[&crate::CounterType::Loyalty], 10 - amount); }
        }
    }
}

#[test]
fn cohort_sacrificed_copy_uses_last_known_characteristics_and_retains_only_resolving_ability() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Sacrifice Copier")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("{1}, Sacrifice another creature: This creature becomes a copy of the sacrificed creature, except it has this ability.\nFlying").unwrap();
    let ability = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let sacrificed = game.create_object_from_definition(&body("Original sacrifice", CardType::Creature), alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    ctx.ability_index = Some(0);
    for cost in ability.mana_cost.as_all().unwrap() {
        if let Some(effect) = cost.effect_ref() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    }
    assert!(game.object(sacrificed).is_none());
    let moved = game.player(alice).unwrap().graveyard[0];
    game.object_mut(moved).unwrap().name = "Changed in graveyard".into();
    for effect in ability.effects.flattened_default_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    let chars = game.current_characteristics(source).unwrap();
    assert_eq!(chars.name.as_ref(), "Original sacrifice");
    assert_eq!(chars.power, Some(10));
    assert_eq!(chars.abilities.iter().filter(|a| matches!(a.kind, AbilityKind::Activated(_))).count(), 1);
    assert!(!game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Flying));
}

#[test]
fn cohort_combat_hand_choice_is_made_by_source_controller_from_damaged_players_hand() {
    struct Pick { chooser: crate::ids::PlayerId, card: crate::ids::ObjectId }
    impl crate::decision::DecisionMaker for Pick {
        fn decide_objects(&mut self, _: &GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
            assert_eq!(ctx.player, self.chooser);
            assert!(ctx.candidates.iter().any(|c| c.id == self.card && c.legal));
            vec![self.card]
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Hand Exiler")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("This creature can't be blocked.\nWhenever this creature deals combat damage to a player, that player reveals their hand. You choose a card from it. That player exiles that card.").unwrap();
    for kind in [CardType::Land, CardType::Creature] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let chosen = game.create_object_from_definition(&body("Selected", kind), bob, Zone::Hand);
        let stable = game.object(chosen).unwrap().stable_id;
        let own = game.create_object_from_definition(&body("Own hand", CardType::Land), alice, Zone::Hand);
        let event = crate::triggers::TriggerEvent::new_with_provenance(crate::events::DamageEvent::with_cause(source, crate::events::DamageTarget::Player(bob), 4, true, crate::events::EventCause::effect()), crate::provenance::ProvNodeId::default());
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers { queue.add(trigger); }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut Pick { chooser: alice, card: chosen }).unwrap();
        assert!(game.exile.iter().any(|id| game.object(*id).unwrap().stable_id == stable));
        assert_eq!(game.object(own).unwrap().zone, Zone::Hand);
        assert!(game.player(bob).unwrap().hand.is_empty());
    }
}

#[test]
fn cohort_exiled_spell_permission_checks_cast_face_and_expires_at_next_turn_start() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Library Observatory")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}.\n{2}{U}{R}, {T}: Exile the top card of your library. Until your next turn, you may cast it if it's an instant or sorcery spell.").unwrap();
    let ability = card.abilities.iter().filter_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).last().unwrap();
    for (kind, adventure) in [(CardType::Instant, false), (CardType::Sorcery, false), (CardType::Creature, false), (CardType::Land, false), (CardType::Creature, true)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        game.turn.phase = crate::game_state::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let mut candidate = body("Exiled candidate", kind);
        if adventure {
            let mut spell = body("Adventure spell", CardType::Sorcery);
            candidate.card.other_face = Some(spell.card.id);
            candidate.card.other_face_name = Some(spell.card.name.clone());
            candidate.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
            spell.card.other_face = Some(candidate.card.id);
            spell.card.other_face_name = Some(candidate.card.name.clone());
            spell.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
            spell.card.subtypes.push(crate::types::Subtype::Adventure);
            game.register_linked_face_definition(&candidate); game.register_linked_face_definition(&spell);
        }
        let id = game.create_object_from_definition(&candidate, alice, Zone::Library);
        let stable = game.object(id).unwrap().stable_id;
        let mut ctx = EffectContext::new_default(source, alice);
        for effect in ability.effects.flattened_default_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        let exiled = game.exile.iter().copied().find(|id| game.object(*id).unwrap().stable_id == stable).unwrap();
        let can_cast = |game: &GameState| crate::decision::compute_legal_actions(game, alice).iter().any(|action| matches!(action, crate::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled));
        let expected = adventure || matches!(kind, CardType::Instant | CardType::Sorcery);
        assert_eq!(can_cast(&game), expected, "{kind:?} adventure={adventure}");
        assert!(!game.effect_store.grant_registry.card_can_play_from_zone(&game, exiled, Zone::Exile, bob));
        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
        assert_eq!(game.effect_store.grant_registry.card_can_play_from_zone(&game, exiled, Zone::Exile, alice), matches!(kind, CardType::Instant | CardType::Sorcery));
        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
        assert!(!can_cast(&game));
    }
}
