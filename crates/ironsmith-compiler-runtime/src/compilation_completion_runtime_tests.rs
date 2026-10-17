//! Focused runtime checks for cards in the compilation-completion task.
use super::compile_to_runtime_definition;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::effect::Effect;
use ironsmith::target::ChooseSpec;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

#[test]
fn bonds_of_faith_tracks_the_attached_creatures_current_human_type() {
    let definition = compile_to_runtime_definition("Bonds of Faith",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for host_owner in [alice, bob] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Attached creature witness")
            .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let host = game.create_object_from_card(&card, host_owner, Zone::Battlefield);
        let unrelated = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
        for human in [true, false, true] {
            game.object_mut(host).unwrap().subtypes = vec![if human { Subtype::Human } else { Subtype::Elf }].into();
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.calculated_power(host), Some(if human { 4 } else { 2 }));
            assert_eq!(game.calculated_toughness(host), Some(if human { 4 } else { 2 }));
            assert_eq!(game.can_attack(host), human, "current type chooses the combat restriction branch");
            assert_eq!(game.can_block(host), human);
            assert_eq!(game.calculated_power(unrelated), Some(2));
            assert!(game.can_attack(unrelated) && game.can_block(unrelated));
        }
        game.object_mut(host).unwrap().subtypes = vec![Subtype::Elf].into();
        game.move_object(aura, Zone::Graveyard, ironsmith::events::EventCause::effect());
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.calculated_power(host), Some(2));
        assert!(game.can_attack(host) && game.can_block(host), "both static branches end when the Aura leaves");
    }
}

struct ControllerReplacementChoice {
    controller: PlayerId,
    affected: PlayerId,
    accept: bool,
    questions: usize,
    orders: usize,
}
impl ironsmith::decision::DecisionMaker for ControllerReplacementChoice {
    fn decide_boolean(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
        assert_eq!(ctx.player, self.controller, "the replacement's controller decides whether to apply it");
        self.questions += 1;
        self.accept
    }
    fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        assert_eq!(ctx.player, self.affected, "the damaged player orders replacements");
        self.orders += 1;
        vec![ctx.options.iter().find(|option| option.legal).unwrap().index]
    }
}

#[test]
fn battletide_alchemist_controller_chooses_prevention_using_live_cleric_count() {
    let definition = compile_to_runtime_definition("Battletide Alchemist",
        "Mana cost: {3}{W}{W}\nType: Creature — Kithkin Cleric\nPower/Toughness: 3/4\nIf a source would deal damage to a player, you may prevent X of that damage, where X is the number of Clerics you control.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for copies in [1, 2] { for accept in [false, true] { for unpreventable in [false, true] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for _ in 0..copies { game.create_object_from_definition(&definition, alice, Zone::Battlefield); }
        let cleric = CardBuilder::new(CardId::new(), "Cleric witness")
            .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Cleric])
            .power_toughness(PowerToughness::fixed(10, 10)).build();
        let own_cleric = game.create_object_from_card(&cleric, alice, Zone::Battlefield);
        let source = game.create_object_from_card(&cleric, bob, Zone::Battlefield);
        let mut dm = ControllerReplacementChoice { controller: alice, affected: bob, accept, questions: 0, orders: 0 };
        let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(7, ChooseSpec::SpecificPlayer(bob))
            .with_unpreventable(unpreventable));
        {
            let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
            ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
        }
        let prevented = if accept && !unpreventable { copies * (copies + 1) } else { 0 };
        assert_eq!(game.player(bob).unwrap().life, 13 + prevented,
            "copies={copies}, accept={accept}, unpreventable={unpreventable}");
        if !unpreventable {
            assert_eq!(dm.questions, copies as usize);
            if copies == 2 { assert!(dm.orders > 0); }
        }
        if accept && !unpreventable {
            game.move_object(own_cleric, Zone::Graveyard, ironsmith::events::EventCause::effect());
            let before = game.player(bob).unwrap().life;
            let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
            ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
            assert_eq!(game.player(bob).unwrap().life, before - 7 + copies * copies, "count is evaluated again for each event");
        }
        let questions = dm.questions;
        let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
        let creature_damage = Effect::new(ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificObject(source)));
        ironsmith::effects::execute_effect(&mut game, &creature_damage, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(dm.questions, questions, "the rule protects players, not creatures");
        assert_eq!(game.damage_on(source), 1);
    } } }
    struct PauseController { pending: bool, calls: usize, pause_after: usize }
    impl ironsmith::decision::DecisionMaker for PauseController {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
            assert_eq!(ctx.player, PlayerId::from_index(0));
            self.calls += 1;
            self.pending = self.calls > self.pause_after;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    for pause_after in [0, 1] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for _ in 0..2 { game.create_object_from_definition(&definition, alice, Zone::Battlefield); }
        let card = CardBuilder::new(CardId::new(), "Suspended damage witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4)).build();
        let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(7, ChooseSpec::SpecificPlayer(bob)));
        let mut pause = PauseController { pending: false, calls: 0, pause_after };
        game.take_pending_trigger_events();
        {
            let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut pause);
            let outcome = ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
            assert!(ctx.decision_maker.awaiting_choice());
            assert!(outcome.events.is_empty());
        }
        assert_eq!(game.player(bob).unwrap().life, 20, "pending controller choice cannot commit a partial damage event");
        assert!(game.take_pending_trigger_events().is_empty(), "pending prevention must not publish committed events");
        let mut answered = ControllerReplacementChoice { controller: alice, affected: bob, accept: true, questions: 0, orders: 0 };
        let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut answered);
        ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(answered.questions, 2);
        assert_eq!(game.player(bob).unwrap().life, 17, "retry commits the event once with both replacements");
    }
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "One-shot replacement witness")
        .card_types(vec![CardType::Artifact]).build();
    let owner = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
    let id = game.effect_store.replacement_effects.add_one_shot_effect(
        ironsmith::replacement::ReplacementEffect::with_matcher(owner, alice,
            ironsmith::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
            ironsmith::replacement::ReplacementAction::PreventDamageAmount(1))
            .with_controller_option(true));
    let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(bob)));
    let mut dm = ControllerReplacementChoice { controller: alice, affected: bob, accept: false, questions: 0, orders: 0 };
    for _ in 0..2 {
        let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
        ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
    }
    assert_eq!(game.player(bob).unwrap().life, 18);
    assert_eq!(dm.questions, 2, "both declined events asked the replacement controller");
    assert!(game.effect_store.replacement_effects.get_effect(id).is_some(), "declining does not consume a one-shot effect");
    dm.accept = true;
    {
        let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
        ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
    }
    assert_eq!(game.player(bob).unwrap().life, 18);
    assert_eq!(dm.questions, 3);
    assert!(game.effect_store.replacement_effects.get_effect(id).is_none(), "accepting uses the one-shot effect");
    let questions = dm.questions;
    let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(bob).unwrap().life, 17);
    assert_eq!(dm.questions, questions);
}

#[test]
fn blood_of_the_martyr_controller_may_redirect_creature_damage_until_end_of_turn() {
    let definition = compile_to_runtime_definition("Blood of the Martyr",
        "Mana cost: {W}{W}{W}\nType: Instant\nUntil end of turn, if damage would be dealt to any creature, you may have that damage dealt to you instead.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for accept in [false, true] { for unpreventable in [false, true] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(source, alice));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let creature = CardBuilder::new(CardId::new(), "Creature damage witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(10, 10)).build();
        let victim = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let attacker = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let mut dm = ControllerReplacementChoice { controller: alice, affected: bob, accept, questions: 0, orders: 0 };
        let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(3, ChooseSpec::SpecificObject(victim))
            .with_unpreventable(unpreventable));
        for _ in 0..2 {
            let mut ctx = ironsmith::effects::EffectContext::new(attacker, bob, &mut dm);
            ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
        }
        assert_eq!(dm.questions, 2, "the spell's controller chooses for each event after the spell resolves");
        assert_eq!(game.player(alice).unwrap().life, if accept { 14 } else { 20 });
        assert_eq!(game.damage_on(victim), if accept { 0 } else { 6 });
        let questions = dm.questions;
        {
            let mut ctx = ironsmith::effects::EffectContext::new(attacker, bob, &mut dm);
            let player_damage = Effect::new(ironsmith::effects::DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(bob)));
            ironsmith::effects::execute_effect(&mut game, &player_damage, &mut ctx).unwrap();
        }
        assert_eq!(game.player(bob).unwrap().life, 19);
        assert_eq!(dm.questions, questions, "player damage is not redirected");
        game.effect_store.replacement_effects.clear_until_end_of_turn_effects();
        let before = game.damage_on(victim);
        let mut ctx = ironsmith::effects::EffectContext::new(attacker, bob, &mut dm);
        ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(game.damage_on(victim), before + 3);
        assert_eq!(dm.questions, questions, "the redirect expires at end of turn");
    } }
}

#[test]
fn bronze_horse_prevention_checks_other_creature_and_targeting_spell() {
    let definition = compile_to_runtime_definition("Bronze Horse",
        "Mana cost: {7}\nType: Artifact Creature — Horse\nPower/Toughness: 4/4\nTrample\nAs long as you control another creature, prevent all damage that would be dealt to this creature by spells that target it.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (other_creature, zone, targets_horse, unpreventable, expected) in [
        (true, Zone::Stack, true, false, 0),
        (false, Zone::Stack, true, false, 3),
        (true, Zone::Stack, false, false, 3),
        (true, Zone::Battlefield, true, false, 3),
        (true, Zone::Stack, true, true, 3),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let horse = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = CardBuilder::new(CardId::new(), "Creature witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4)).build();
        if other_creature { game.create_object_from_card(&creature, alice, Zone::Battlefield); }
        let other = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(), "Damage spell witness")
            .card_types(vec![CardType::Instant]).build();
        let source = game.create_object_from_card(if zone == Zone::Stack { &spell } else { &creature }, bob, zone);
        if zone == Zone::Stack {
            let mut entry = ironsmith::game_state::StackEntry::new(source, bob);
            entry.targets.push(ironsmith::game_state::Target::Object(if targets_horse { horse } else { other }));
            game.push_to_stack(entry);
        }
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, bob);
        let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(3, ChooseSpec::SpecificObject(horse))
            .with_unpreventable(unpreventable));
        ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
        assert_eq!(game.damage_on(horse), expected,
            "other={other_creature}, zone={zone:?}, targeting={targets_horse}, unpreventable={unpreventable}");
    }
}

#[test]
fn benevolent_unicorn_reduces_spell_damage_without_preventing_it() {
    let definition = compile_to_runtime_definition("Benevolent Unicorn",
        "Mana cost: {1}{W}\nType: Creature — Unicorn\nPower/Toughness: 1/2\nIf a spell would deal damage to a permanent or player, it deals that much damage minus 1 to that permanent or player instead.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (zone, unpreventable, amount, expected) in [
        (Zone::Stack, false, 3, 2),
        (Zone::Stack, true, 3, 2),
        (Zone::Battlefield, false, 3, 3),
        (Zone::Stack, false, 1, 0),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card_type = if zone == Zone::Stack { CardType::Instant } else { CardType::Creature };
        let card = CardBuilder::new(CardId::new(), "Damage source witness")
            .card_types(vec![card_type]).power_toughness(PowerToughness::fixed(10, 10)).build();
        let source = game.create_object_from_card(&card, alice, zone);
        let victim_card = CardBuilder::new(CardId::new(), "Damage recipient witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(10, 10)).build();
        let victim = game.create_object_from_card(&victim_card, bob, Zone::Battlefield);
        for target in [ChooseSpec::SpecificPlayer(bob), ChooseSpec::SpecificObject(victim)] {
            let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(amount, target)
                .with_unpreventable(unpreventable));
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
            ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
        }
        assert_eq!(game.player(bob).unwrap().life, 20 - expected, "{zone:?}/{unpreventable}");
        assert_eq!(game.damage_on(victim), expected as u32, "{zone:?}/{unpreventable}");
    }
}

#[test]
fn barbed_wire_finite_prevention_is_shared_by_recipients_and_scoped_to_its_source() {
    let definition = compile_to_runtime_definition("Barbed Wire",
        "Mana cost: {3}\nType: Artifact\nAt the beginning of each player's upkeep, this artifact deals 1 damage to that player.\n{2}: Prevent the next 1 damage that would be dealt by this artifact this turn.", false).unwrap();
    let shield = definition.abilities.iter().find_map(|ability| {
        let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind else { return None; };
        activated.effects.all_effects().iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PreventDamageEffect>().cloned())
    }).expect("finite prevention ability");
    assert!(shield.protect_all);
    assert_eq!(shield.amount, ironsmith::effect::Value::Fixed(1));
    assert_eq!(shield.damage_filter.from_source, Some(ironsmith::target::ObjectFilter::source()));
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let shield_effect = Effect::new(shield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &shield_effect, &mut ctx).unwrap();
    drop(ctx);
    let registered = game.effect_store.prevention_effects.shields().last().unwrap();
    assert_eq!(registered.protected, ironsmith::prevention::PreventionTarget::All);
    assert_eq!(registered.damage_filter.from_specific_source, Some(source));
    fn damage(game: &mut ironsmith::GameState, source: ironsmith::ObjectId, player: PlayerId, amount: i32) {
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, PlayerId::from_index(0));
        let effect = Effect::new(ironsmith::effects::DealDamageEffect::new(amount, ChooseSpec::SpecificPlayer(player)));
        ironsmith::effects::execute_effect(game, &effect, &mut ctx).unwrap();
    }
    damage(&mut game, other, alice, 1);
    assert_eq!(game.player(alice).unwrap().life, 19, "another copy cannot use this shield");
    damage(&mut game, source, bob, 2);
    assert_eq!(game.player(bob).unwrap().life, 19, "only one of two damage is prevented");
    damage(&mut game, source, alice, 1);
    assert_eq!(game.player(alice).unwrap().life, 18, "the finite budget is shared, not per player");
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &shield_effect, &mut ctx).unwrap();
    drop(ctx);
    game.effect_store.prevention_effects.cleanup_end_of_turn();
    damage(&mut game, source, bob, 1);
    assert_eq!(game.player(bob).unwrap().life, 18, "unused shield expires at end of turn");
}

#[test]
fn roku_retains_only_its_six_mana_through_steps_and_expires_at_combat_end() {
    let definition = compile_to_runtime_definition("Avatar Roku, Firebender",
        "Mana cost: {3}{R}{R}{R}\nType: Legendary Creature — Human Avatar\nPower/Toughness: 6/6\nWhenever a player attacks, add six {R}. Until end of combat, you don't lose this mana as steps end.\n{R}{R}{R}: Target creature gets +3/+0 until end of turn.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for attacker_player in [alice, bob] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = attacker_player;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = CardBuilder::new(CardId::new(), "Attack witness")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
        let mut attackers = Vec::new();
        for _ in 0..2 {
            let attacker = game.create_object_from_card(&creature, attacker_player, Zone::Battlefield);
            game.remove_summoning_sickness(attacker);
            attackers.push((attacker, ironsmith::combat_state::AttackTarget::Player(
                if attacker_player == alice { bob } else { alice })));
        }
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(&mut game, &Effect::new(ironsmith::effects::AddManaEffect::new(
            vec![ironsmith::mana::ManaSymbol::Red], ironsmith::target::PlayerFilter::You)), &mut ctx).unwrap();
        drop(ctx);
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::combat_state::declare_attackers(&mut game, &mut combat, attackers).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1, "one trigger per attacking player, not creature");
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 7);
        assert_eq!(game.player(bob).unwrap().mana_pool.total_wide(), 0);
        let mut later = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(&mut game, &Effect::new(ironsmith::effects::AddManaEffect::new(
            vec![ironsmith::mana::ManaSymbol::Red], ironsmith::target::PlayerFilter::You)), &mut later).unwrap();
        drop(later);
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 8);
        game.move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect());
        game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        game.empty_mana_pools().unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 6, "unrelated red mana empties");
        game.turn.step = Some(ironsmith::game_state::Step::CombatDamage);
        game.empty_mana_pools().unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 6);
        game.turn.step = Some(ironsmith::game_state::Step::EndCombat);
        game.empty_mana_pools().unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.total_wide(), 0);
    }
}

#[test]
fn oaths_give_the_upkeep_player_one_optional_reward_and_keep_other_players_unchanged() {
    struct Decide { accept: bool, upkeep: PlayerId, target: PlayerId, decisions: usize }
    impl ironsmith::decision::DecisionMaker for Decide {
        fn decide_boolean(&mut self, _game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
            assert_eq!(ctx.player, self.upkeep);
            self.decisions += 1;
            self.accept
        }
        fn decide_targets(&mut self, _game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            assert_eq!(ctx.player, self.upkeep);
            vec![ironsmith::game_state::Target::Player(self.target)]
        }
    }
    for (name, text, scholars) in [
        ("Oath of Mages", "Type: Enchantment\nAt the beginning of each player's upkeep, that player chooses target player who has more life than they do and is their opponent. The first player may have this enchantment deal 1 damage to the second player.", false),
        ("Oath of Scholars", "Type: Enchantment\nAt the beginning of each player's upkeep, that player chooses target player who has more cards in hand than they do and is their opponent. The first player may discard their hand and draw three cards.", true),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        for accept in [false, true] {
            let (alice, bob, carol) = (PlayerId::from_index(0), PlayerId::from_index(1), PlayerId::from_index(2));
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            game.turn.active_player = bob;
            game.player_mut(alice).unwrap().life = 10;
            game.player_mut(carol).unwrap().life = 30;
            let hand = CardBuilder::new(CardId::new(), "Old hand witness").card_types(vec![CardType::Instant]).build();
            for (player, count) in [(alice, 1), (bob, 2), (carol, 3)] {
                for _ in 0..count { game.create_object_from_card(&hand, player, Zone::Hand); }
            }
            let draw = CardBuilder::new(CardId::new(), "New draw witness").card_types(vec![CardType::Instant]).build();
            for _ in 0..3 { game.create_object_from_card(&draw, bob, Zone::Library); }
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let event = ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::phase::BeginningOfUpkeepEvent::new(bob), Default::default());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
            let mut dm = Decide { accept, upkeep: bob, target: carol, decisions: 0 };
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
            assert_eq!(dm.decisions, 1, "{name}: one choice covers the whole reward");
            assert_eq!(game.player(alice).unwrap().life, 10);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.player(carol).unwrap().life, if accept && !scholars { 29 } else { 30 });
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
            assert_eq!(game.player(carol).unwrap().hand.len(), 3);
            assert_eq!(game.player(bob).unwrap().hand.len(), if accept && scholars { 3 } else { 2 });
            assert_eq!(game.player(bob).unwrap().graveyard.len(), if accept && scholars { 2 } else { 0 });
            assert_eq!(game.player(bob).unwrap().library.len(), if accept && scholars { 0 } else { 3 });
        }
    }
}

#[test]
fn arna_doubles_the_attacker_counters_and_attaches_equipment_copies_to_that_attacker() {
    let definition = compile_to_runtime_definition("Arna Kennerüd, Skycaptain",
        "Mana cost: {2}{W}{U}{B}\nType: Legendary Creature — Human Knight\nPower/Toughness: 4/4\nFlying, lifelink\nWard—Discard a card.\nWhenever a modified creature you control attacks, double the number of each kind of counter on it. Then for each nontoken permanent attached to it, create a token that's a copy of that permanent attached to that creature.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::new(), "Modified attacker")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let attacker = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let equipment = CardBuilder::new(CardId::new(), "Attached equipment witness")
        .card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment]).build();
    let attached = game.create_object_from_card(&equipment, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(attached, ironsmith::object::AttachmentTarget::Object(attacker)));
    let mut token_equipment = CardBuilder::new(CardId::new(), "Attached equipment witness")
        .card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment]).build();
    token_equipment.is_token = true;
    let existing_token = game.create_object_from_card(&token_equipment, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(existing_token, ironsmith::object::AttachmentTarget::Object(attacker)));
    use ironsmith::object::CounterType;
    game.add_counters(attacker, CounterType::PlusOnePlusOne, 1);
    game.add_counters(attacker, CounterType::Charge, 2);
    game.add_counters(source, CounterType::Charge, 1);
    game.remove_summoning_sickness(attacker);
    game.refresh_continuous_state().unwrap();
    let mut combat = ironsmith::combat_state::CombatState::default();
    ironsmith::combat_state::declare_attackers(&mut game, &mut combat,
        vec![(attacker, ironsmith::combat_state::AttackTarget::Player(bob))]).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
    }
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "one modified-attacker trigger");
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(attacker, CounterType::PlusOnePlusOne), 2);
    assert_eq!(game.counter_count(attacker, CounterType::Charge), 4);
    assert_eq!(game.counter_count(source, CounterType::Charge), 1);
    let copies = game.battlefield.iter().filter_map(|id| game.object(*id))
        .filter(|object| object.id != existing_token && object.name.as_ref() == "Attached equipment witness"
            && object.kind == ironsmith::object::ObjectKind::Token)
        .collect::<Vec<_>>();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].attached_to, Some(ironsmith::object::AttachmentTarget::Object(attacker)));
}

#[test]
fn approach_win_requires_another_same_named_cast_by_you_and_the_current_cast_from_hand() {
    let definition = compile_to_runtime_definition(
        "Approach of the Second Sun",
        "Mana cost: {6}{W}\nType: Sorcery\nIf this spell was cast from your hand and you've cast another spell named Approach of the Second Sun this game, you win the game. Otherwise, put Approach of the Second Sun into its owner's library seventh from the top and you gain 7 life.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for previous in 0..4 { for origin in [Zone::Hand, Zone::Graveyard] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let witness = CardBuilder::new(CardId::new(), "Other spell witness")
            .card_types(vec![CardType::Instant]).build();
        for _ in 0..8 { game.create_object_from_card(&witness, alice, Zone::Library); }
        if previous != 0 {
            let caster = if previous == 2 { bob } else { alice };
            let prior = if previous == 3 {
                game.create_object_from_card(&witness, caster, Zone::Stack)
            } else { game.create_object_from_definition(&definition, caster, Zone::Stack) };
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, prior).unwrap();
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::SpellCastEvent::new_with_snapshot(prior, caster, Zone::Hand, snapshot),
                Default::default()));
            ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut ironsmith::triggers::TriggerQueue::new());
            // The earlier spell need not have resolved, and its cast survives
            // turn-history cleanup in the game-scoped named-cast collection.
            game.move_object(prior, Zone::Graveyard, ironsmith::events::EventCause::effect());
            game.next_turn();
            game.next_turn();
        }
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(source, alice));
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, source).unwrap();
        game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
            ironsmith::events::SpellCastEvent::new_with_snapshot(source, alice, origin, snapshot),
            Default::default()));
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut ironsmith::triggers::TriggerQueue::new());
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let wins = previous == 1 && origin == Zone::Hand;
        assert_eq!(game.player(bob).unwrap().has_lost, wins, "previous={previous}, origin={origin:?}");
        assert!(game.player(alice).unwrap().is_in_game());
        assert_eq!(game.player(alice).unwrap().life, if wins { 20 } else { 27 });
        if !wins {
            let seventh = game.player(alice).unwrap().library.iter().rev().nth(6).copied().unwrap();
            assert_eq!(game.object(seventh).unwrap().name.as_ref(), "Approach of the Second Sun");
        }
    } } 
}

#[test]
fn bounty_counters_are_distributed_then_removed_at_cleanup_without_removing_older_counters() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::object::CounterType;
    let definition = compile_to_runtime_definition("Bounty of the Hunt",
        "Mana cost: {3}{G}{G}\nType: Instant\nYou may exile a green card from your hand rather than pay this spell's mana cost.\nDistribute three +1/+1 counters among one, two, or three target creatures. For each +1/+1 counter you put on a creature this way, remove a +1/+1 counter from that creature at the beginning of the next cleanup step.", false).unwrap();
    assert!(!definition.alternative_casts.is_empty());
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Counter recipient")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(5, 5)).build();
    let first = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let bystander = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    game.add_counters(first, CounterType::PlusOnePlusOne, 2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let target_spec = definition.spell_effect.as_ref().unwrap().all_effects().into_iter()
        .find_map(|effect| effect.0.get_target_spec()).unwrap().clone();
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(first), ResolvedTarget::Object(second)])
        .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
            spec: target_spec.clone(), range: 0..2,
        }])
        .with_target_distributions(vec![ironsmith::game_state::TargetDistribution {
            spec: target_spec, range: 0..2,
            allocations: vec![(ironsmith::game_state::Target::Object(first), 2),
                (ironsmith::game_state::Target::Object(second), 1)],
        }]);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne)
        + game.counter_count(second, CounterType::PlusOnePlusOne), 5);
    assert_eq!(game.counter_count(bystander, CounterType::PlusOnePlusOne), 0);
    assert_eq!(game.effect_store.delayed_triggers.len(), 3);
    let end = ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::BeginningOfEndStepEvent::new(bob), Default::default());
    assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &end).is_empty());
    let cleanup = ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::BeginningOfCleanupStepEvent::new(bob), Default::default());
    let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &cleanup);
    assert_eq!(entries.len(), 3);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 2);
    assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 0);
    assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &cleanup).is_empty());
}

#[test]
fn alchemist_extra_turn_rules_follow_the_created_turn_and_cleave_removes_only_the_loss() {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| compile_to_runtime_definition("Alchemist's Gambit",
        "Mana cost: {1}{R}{R}\nType: Sorcery\nCleave {4}{U}{U}{R} (You may cast this spell for its cleave cost. If you do, remove the words in square brackets.)\nTake an extra turn after this one. During that turn, damage can't be prevented. [At the beginning of that turn's end step, you lose the game.]\nExile Alchemist's Gambit.", false));
    let definition = definition.unwrap_or_else(|error| panic!("{error:?}\n{}", trace.render()));
    let normal = definition.spell_effect.as_ref().unwrap().all_effects_owned();
    let cleaved = definition.alternative_casts.iter().find_map(|method| method.cleave_effects()).unwrap();
    for cleave in [false, true] { for interposed in [false, true] { for skipped in [false, true] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in if cleave { cleaved } else { &normal } {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(game.turn_store.extra_turns, vec![alice]);
        assert!(game.can_prevent_damage(), "not the created extra turn yet");
        if interposed {
            game.turn_store.extra_turns.push(bob);
            game.next_turn();
            assert_eq!(game.turn.active_player, bob);
            assert!(game.can_prevent_damage(), "a later-created extra turn is unrelated");
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(bob), Default::default());
            assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
        }
        if skipped { game.turn_store.skip_next_turn.insert(alice); }
        game.next_turn();
        if skipped {
            assert_eq!(game.turn.active_player, bob);
            assert!(game.can_prevent_damage(), "skipping the bound turn discards its rule");
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(bob), Default::default());
            assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
        } else {
            assert_eq!(game.turn.active_player, alice);
            assert!(!game.can_prevent_damage(), "damage is unpreventable in the bound turn");
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(alice), Default::default());
            let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
            assert_eq!(entries.len(), usize::from(!cleave), "cleave removes only the end-step loss");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for entry in entries { queue.add(entry); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            assert_eq!(game.player(alice).unwrap().has_lost, !cleave);
            if cleave {
                game.next_turn();
                assert!(game.can_prevent_damage(), "restriction ends with the created turn");
            }
        }
    } } }
}

#[test]
fn altar_devotion_bonus_counts_once_per_color_combination_and_tracks_current_abilities() {
    use ironsmith::{Color, ColorSet, Value};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = compile_to_runtime_definition("Altar of the Pantheon",
        "Mana cost: {3}\nType: Artifact\nYour devotion to each color and each combination of colors is increased by one.\n{T}: Add one mana of any color. If you control a God, a Demigod, or a legendary enchantment, you gain 1 life.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let first = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    let witness = CardBuilder::new(CardId::new(), "Hybrid witness")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White, ManaSymbol::Blue], vec![ManaSymbol::Red]]))
        .build();
    game.create_object_from_card(&witness, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let white_blue = ColorSet::WHITE.union(ColorSet::BLUE);
    assert_eq!(game.devotion_to_color(alice, Color::White), 2);
    assert_eq!(game.devotion_to_color(alice, Color::Blue), 2);
    assert_eq!(game.devotion_to_color(alice, Color::Green), 1);
    assert_eq!(game.devotion_to_colors(alice, white_blue), 2, "one hybrid pip plus one bonus");
    assert_eq!(game.devotion_to_colors(alice, Color::ALL.into_iter().collect()), 3);
    assert_eq!(game.devotion_to_colors(bob, white_blue), 1, "only Bob's own Altar");
    let devotee = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Devotion characteristic witness")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(0, 5))
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::characteristic_defining_pt(
                Value::devotion_to_colors(ironsmith::target::PlayerFilter::You, white_blue), Value::Fixed(5))))
        .build();
    let devotee = game.create_object_from_definition(&devotee, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(devotee), Some(2), "devotion can be read during continuous calculation");
    let gain = Effect::new(ironsmith::effects::GainLifeEffect {
        amount: Value::devotion_to_colors(ironsmith::target::PlayerFilter::You, white_blue),
        player: ChooseSpec::Player(ironsmith::target::PlayerFilter::You),
    });
    let mut ctx = ironsmith::effects::EffectContext::new_default(first, alice);
    ironsmith::effects::execute_effect(&mut game, &gain, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 22, "the typed value uses the same combination query");
    let second = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.devotion_to_colors(alice, white_blue), 3, "two bonuses stack");
    assert_eq!(game.calculated_power(devotee), Some(3));
    game.phase_out(first);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.devotion_to_colors(alice, white_blue), 2, "phased-out sources do not contribute");
    let removal = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Object(ironsmith::target::ObjectFilter::specific(second)),
        ironsmith::continuous::Modification::RemoveAllAbilities, ironsmith::effect::Until::EndOfTurn));
    ironsmith::effects::execute_effect(&mut game, &removal, &mut ctx).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.devotion_to_colors(alice, white_blue), 1, "ability loss removes the remaining bonus");
    assert_eq!(game.calculated_power(devotee), Some(1));
    game.phase_in(first);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.devotion_to_colors(alice, white_blue), 2);

    let mana_ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated), _ => None,
    }).unwrap();
    for (owner, types, subtypes, legendary, gains_life) in [
        (bob, vec![CardType::Creature], vec![Subtype::God], false, false),
        (alice, vec![CardType::Creature], vec![Subtype::God], false, true),
        (alice, vec![CardType::Creature], vec![Subtype::Demigod], false, true),
        (alice, vec![CardType::Enchantment], vec![], true, true),
        (alice, vec![CardType::Artifact], vec![], true, false),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let qualifying = CardBuilder::new(CardId::new(), "Mana rider witness").card_types(types).subtypes(subtypes)
            .supertypes(if legendary { vec![ironsmith::types::Supertype::Legendary] } else { vec![] })
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        game.create_object_from_card(&qualifying, owner, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in mana_ability.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 20 + i32::from(gains_life),
            "qualifying permanent owner {owner:?}, legendary {legendary}: {:?}", mana_ability.effects);
        assert_eq!(game.player(alice).unwrap().mana_pool.total_wide(), 1);
    }
}

#[test]
fn equipment_filtered_block_requirement_survives_recipient_ability_loss() {
    for (name, subtype, text) in [
        ("Ace's Baseball Bat", Subtype::Dalek, "Type: Legendary Artifact — Equipment\nEquipped creature gets +3/+0.\nAs long as equipped creature is attacking, it has first strike and must be blocked by a Dalek if able.\nEquip legendary creature {1}\nEquip {3}"),
        ("Slayer's Cleaver", Subtype::Eldrazi, "Type: Artifact — Equipment\nEquipped creature gets +3/+1 and must be blocked by an Eldrazi if able.\nEquip {4}"),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        let creature = CardBuilder::new(CardId::new(), "Equipped attacker")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
        let matching = CardBuilder::new(CardId::new(), "Eligible blocker")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let blocker = game.create_object_from_card(&matching, bob, Zone::Battlefield);
        let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(equipment, ironsmith::object::AttachmentTarget::Object(attacker)));
        game.remove_summoning_sickness(attacker);
        game.refresh_continuous_state().unwrap();
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::combat_state::declare_attackers(&mut game, &mut combat,
            vec![(attacker, ironsmith::combat_state::AttackTarget::Player(bob))]).unwrap();
        let removal = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Object(ironsmith::target::ObjectFilter::specific(attacker)),
            ironsmith::continuous::Modification::RemoveAllAbilities,
            ironsmith::effect::Until::EndOfTurn,
        ));
        let mut ctx = ironsmith::effects::EffectContext::new_default(attacker, alice);
        ironsmith::effects::execute_effect(&mut game, &removal, &mut ctx).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![]).is_err(), "{name}");
        ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![(blocker, attacker)]).unwrap();
        assert!(game.detach_object_from_current_target(equipment));
        game.refresh_continuous_state().unwrap();
        ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![]).unwrap();
    }
}

#[test]
fn entry_life_payment_and_token_stats_share_numeric_acquisition() {
    use ironsmith::effect::Value;
    let definition = compile_to_runtime_definition("Phyrexian Processor",
        "Type: Artifact\nAs this artifact enters, pay any amount of life.\n{4}, {T}: Create an X/X black Phyrexian Minion creature token, where X is the life paid as this artifact entered.", false)
        .expect("entry life payment and later token reader should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("token activation");
    let pair = activated.effects.source_number_pair.expect("linked numeric reader");
    let create = activated.effects.all_effects().into_iter().find_map(|effect|
        effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>()).expect("create token");
    let (power, toughness) = create.dynamic_power_toughness.as_ref().expect("dynamic token stats");
    for value in [power, toughness] {
        assert!(matches!(value.unhinted(), Value::SourceChosenNumber { if_unset: Some(0), pair: Some(binding) } if *binding == pair));
    }
    let debug = format!("{definition:#?}");
    assert!(debug.contains("source_owned: true"), "payment must retain its completed result: {debug}");
}

#[test]
fn repeated_returns_announce_one_player_and_preserve_four_choices() {
    let definition = compile_to_runtime_definition("Linessa, Zephyr Mage",
        "Type: Creature\n{T}: Target player returns a creature they control to its owner's hand, then repeats this process for an artifact, an enchantment, and a land.", false)
        .expect("repeated return should compile");
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
    }).unwrap();
    fn inspect(effect: &Effect, targets: &mut usize, choices: &mut Vec<ironsmith::effects::ChooseObjectsEffect>) {
        if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { *targets += 1; }
        if let Some(choice) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() { choices.push(choice.clone()); }
        effect.visit_child_effects(&mut |child| inspect(child, targets, choices));
    }
    let mut targets = 0;
    let mut choices = Vec::new();
    for effect in ability.effects.all_effects() { inspect(effect, &mut targets, &mut choices); }
    assert_eq!(targets, 1);
    assert_eq!(ability.choices.len(), 1);
    assert_eq!(choices.len(), 4);
    for (choice, kind) in choices.iter().zip([CardType::Creature, CardType::Artifact, CardType::Enchantment, CardType::Land]) {
        assert_eq!(choice.filter.card_types, vec![kind]);
        assert_eq!(choice.count.min, 1);
        assert_eq!(choice.count.max, Some(1));
        assert!(matches!(choice.chooser, ironsmith::target::PlayerFilter::Target(_) | ironsmith::target::PlayerFilter::AliasedTarget(_)));
        assert!(matches!(&choice.filter.controller,
            Some(ironsmith::target::PlayerFilter::Target(player) | ironsmith::target::PlayerFilter::AliasedTarget(player))
                if **player == ironsmith::target::PlayerFilter::Any));
    }
}

#[test]
fn auratouched_mage_moves_the_searched_aura_and_attaches_to_the_source() {
    let definition = compile_to_runtime_definition("Auratouched Mage",
        "Type: Creature — Human Wizard\nWhen this creature enters, search your library for an Aura card that could enchant it. If this creature is still on the battlefield, put that Aura card onto the battlefield attached to it. Otherwise, reveal the Aura card and put it into your hand. Then shuffle.", false).unwrap();
    fn inspect(effect: &Effect, moved: &mut bool, attached: &mut bool) {
        if let Some(movement) = effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
            && movement.zone == Zone::Battlefield
        {
            match movement.target.base() {
                ChooseSpec::Object(filter) => {
                    assert_eq!(filter.zone, None, "the searched card is in the library");
                    assert!(filter.tagged_constraints.iter().any(|tag| tag.tag.as_str() == "searched"));
                }
                ChooseSpec::Tagged(tag) => assert_eq!(tag.as_str(), "searched"),
                other => panic!("unexpected Aura origin: {other:?}"),
            }
            *moved = true;
        }
        if let Some(attachment) = effect.downcast_ref::<ironsmith::effects::AttachObjectsEffect>() {
            assert!(matches!(attachment.target.base(), ChooseSpec::Source), "host must be the entering creature: {:?}", attachment.target);
            *attached = true;
        }
        effect.visit_child_effects(&mut |child| inspect(child, moved, attached));
    }
    let mut moved = false;
    let mut attached = false;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() { inspect(effect, &mut moved, &mut attached); }
        }
    }
    assert!(moved && attached);
}

#[test]
fn targeted_reveal_checks_the_underlying_card_before_offering_face_up() {
    use ironsmith::snapshot::ObjectSnapshot;
    #[derive(Debug, Default)]
    struct Accept { offers: usize }
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.offers += 1;
            true
        }
    }
    let definition = compile_to_runtime_definition("Hauntwoods Shrieker",
        "Type: Creature — Beast Mutant\nWhenever this creature attacks, manifest dread.\n{1}{G}: Reveal target face-down permanent. If it's a creature card, you may turn it face up.", false).unwrap();
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
    }).unwrap();
    fn inspect(effect: &Effect, targets: &mut usize, reveal: &mut Option<Effect>, condition: &mut Option<Effect>) {
        if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { *targets += 1; }
        if effect.downcast_ref::<ironsmith::effects::RevealTaggedEffect>().is_some() { *reveal = Some(effect.clone()); }
        if effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some() { *condition = Some(effect.clone()); }
        effect.visit_child_effects(&mut |child| inspect(child, targets, reveal, condition));
    }
    let (mut targets, mut reveal, mut condition) = (0, None, None);
    for effect in ability.effects.all_effects() { inspect(effect, &mut targets, &mut reveal, &mut condition); }
    assert_eq!(targets, 1);
    assert_eq!(ability.choices.len(), 1);
    let reveal = reveal.unwrap();
    let tag = reveal.downcast_ref::<ironsmith::effects::RevealTaggedEffect>().unwrap().tag.clone();
    let condition = condition.unwrap();
    let alice = PlayerId::from_index(0);
    for (kind, token) in [(CardType::Creature, false), (CardType::Land, false), (CardType::Creature, true)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Revealed identity").card_types(vec![kind]).build();
        let target = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if token { game.object_mut(target).unwrap().kind = ironsmith::object::ObjectKind::Token; }
        assert!(game.set_face_down(target));
        game.refresh_continuous_state().unwrap();
        assert!(game.current_card_types(target).unwrap().contains(&CardType::Creature));
        let mut dm = Accept::default();
        let mut ctx = ironsmith::effects::EffectContext::new(target, alice, &mut dm);
        ctx.set_tagged_objects(tag.clone(), vec![ObjectSnapshot::from_object_id(&game, target).unwrap()]);
        ironsmith::effects::execute_effect(&mut game, &reveal, &mut ctx).unwrap();
        assert!(game.is_face_down(target), "revealing alone must preserve the overlay");
        ironsmith::effects::execute_effect(&mut game, &condition, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(dm.offers, usize::from(kind == CardType::Creature && !token));
        assert_eq!(game.is_face_down(target), kind != CardType::Creature || token);
    }
}

#[test]
fn renowned_conditions_follow_the_target_or_triggering_creature() {
    use ironsmith::snapshot::ObjectSnapshot;
    let mist = compile_to_runtime_definition("Enshrouding Mist",
        "Type: Instant\nTarget creature gets +1/+1 until end of turn. Prevent all damage that would be dealt to it this turn. If it's renowned, untap it.", false).unwrap();
    fn conditional(effect: &Effect, found: &mut Option<Effect>) {
        if effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some() { *found = Some(effect.clone()); }
        effect.visit_child_effects(&mut |child| conditional(child, found));
    }
    let mut found = None;
    for effect in mist.spell_effect.as_ref().unwrap().all_effects() { conditional(effect, &mut found); }
    let effect = found.unwrap();
    let condition = &effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().unwrap().condition;
    let ironsmith::effect::Condition::TaggedObjectMatches(tag, filter) = condition else { panic!("{condition:?}"); };
    assert!(filter.renowned);
    assert_eq!(tag.as_str(), "pumped_0");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Renown witness").card_types(vec![CardType::Creature]).build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let target = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.tap(target);
    game.set_renowned(source);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects(tag.clone(), vec![ObjectSnapshot::from_object_id(&game, target).unwrap()]);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert!(game.is_tapped(target), "the renowned spell source cannot substitute for its target");
    game.set_renowned(target);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert!(!game.is_tapped(target));

    let consul = compile_to_runtime_definition("Consul's Lieutenant",
        "Type: Creature — Human Soldier\nFirst strike\nRenown 1\nWhenever this creature attacks, if it's renowned, other attacking creatures you control get +1/+1 until end of turn.", false).unwrap();
    let condition = consul.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.as_ref()
            .filter(|condition| matches!(condition, ironsmith::effect::Condition::TaggedObjectMatches(_, filter) if filter.renowned)), _ => None,
    }).unwrap();
    let fresh = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(fresh, alice);
    ctx.set_tagged_objects("triggering", vec![ObjectSnapshot::from_object_id(&game, fresh).unwrap()]);
    assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
    game.set_renowned(fresh);
    assert!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
}

#[test]
fn exile_counter_upkeep_requires_the_source_zone_and_its_remaining_counter() {
    let definition = compile_to_runtime_definition("Darigaaz Reincarnated",
        "Type: Legendary Creature — Dragon\nFlying, trample, haste\nIf Darigaaz would die, instead exile it with three egg counters on it.\nAt the beginning of your upkeep, if this card is exiled with an egg counter on it, remove an egg counter from it. Then if this card has no egg counters on it, return it to the battlefield.", false).unwrap();
    let (condition, program, zones) = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.as_ref()
            .map(|condition| (condition, &trigger.effects, &ability.functional_zones)), _ => None,
    }).unwrap();
    assert!(zones.contains(&Zone::Exile));
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Exile, Zone::Battlefield] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, zone);
        let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
        game.add_counters(source, ironsmith::CounterType::Egg, 1);
        assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap(), zone == Zone::Exile);
        if zone == Zone::Exile {
            game.add_counters(source, ironsmith::CounterType::Egg, 2);
            let stable = game.object(source).unwrap().stable_id;
            let mut ctx = ctx;
            for remaining in [2, 1, 0] {
                for effect in program.all_effects() {
                    ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                let current = game.find_object_by_stable_id(stable).unwrap();
                let object = game.object(current).unwrap();
                assert_eq!(object.zone, if remaining == 0 { Zone::Battlefield } else { Zone::Exile });
                assert_eq!(object.counters.get(&ironsmith::CounterType::Egg).copied().unwrap_or(0), remaining);
            }
        }
    }
}

#[test]
fn hand_or_command_choice_keeps_owner_and_lesser_mana_value_constraints() {
    use ironsmith::snapshot::ObjectSnapshot;
    #[derive(Debug)]
    struct SelectCommand { preferred: ironsmith::ObjectId, legal: Vec<ironsmith::ObjectId> }
    impl ironsmith::decision::DecisionMaker for SelectCommand {
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.legal = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
            assert!(self.legal.contains(&self.preferred));
            vec![self.preferred]
        }
    }
    let definition = compile_to_runtime_definition("Next of Kin",
        "Type: Enchantment — Aura\nEnchant creature\nWhen enchanted creature dies, you may put a creature card you own with lesser mana value from your hand or from the command zone onto the battlefield. If you do, return this card to the battlefield attached to that creature at the beginning of the next end step.", false).unwrap();
    fn find(effect: &Effect, choice: &mut Option<ironsmith::effects::ChooseObjectsEffect>) {
        if let Some(found) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() { *choice = Some(found.clone()); }
        effect.visit_child_effects(&mut |child| find(child, choice));
    }
    let mut choice = None;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() { find(effect, &mut choice); }
        }
    }
    let choice = choice.unwrap();
    assert_eq!(choice.zone, Some(Zone::Hand));
    assert_eq!(choice.additional_zones, vec![Zone::Command]);
    assert_eq!(choice.filter.zone, None);
    assert_eq!(choice.filter.owner, Some(ironsmith::target::PlayerFilter::You));
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = |value| CardBuilder::new(CardId::new(), "Choice witness")
        .card_types(vec![CardType::Creature])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(value)])).build();
    let dead = game.create_object_from_card(&creature(4), alice, Zone::Graveyard);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand = game.create_object_from_card(&creature(2), alice, Zone::Hand);
    let command = game.create_object_from_card(&creature(3), alice, Zone::Command);
    let foreign = game.create_object_from_card(&creature(1), bob, Zone::Command);
    let expensive = game.create_object_from_card(&creature(4), alice, Zone::Command);
    let battlefield = game.create_object_from_card(&creature(1), alice, Zone::Battlefield);
    let mut dm = SelectCommand { preferred: command, legal: Vec::new() };
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    ctx.set_tagged_objects("triggering", vec![ObjectSnapshot::from_object_id(&game, dead).unwrap()]);
    let tag = choice.tag.clone();
    ironsmith::effects::execute_effect(&mut game, &Effect::new(choice), &mut ctx).unwrap();
    assert_eq!(ctx.get_tagged_all(&tag).unwrap()[0].object_id, command);
    drop(ctx);
    assert!(dm.legal.contains(&hand));
    for forbidden in [foreign, expensive, battlefield] { assert!(!dm.legal.contains(&forbidden)); }
}

#[test]
fn source_zone_alternatives_allow_the_graveyard_and_battlefield_upkeep() {
    let definition = compile_to_runtime_definition("Firemane Angel",
        "Type: Creature — Angel\nFlying, first strike\nAt the beginning of your upkeep, if Firemane Angel is in your graveyard or on the battlefield, you may gain 1 life.\n{6}{R}{R}{W}{W}: Return this card from your graveyard to the battlefield. Activate only during your upkeep.", false).unwrap();
    let (condition, zones) = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.as_ref().map(|condition| (condition, &ability.functional_zones)), _ => None,
    }).unwrap();
    assert!(zones.contains(&Zone::Graveyard));
    assert!(zones.contains(&Zone::Battlefield));
    assert_eq!(zones.len(), 2);
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Graveyard, Zone::Battlefield, Zone::Hand, Zone::Exile] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, zone);
        let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap(), matches!(zone, Zone::Graveyard | Zone::Battlefield));
    }
}

#[test]
fn commander_mana_value_choice_drives_both_draw_and_life_loss() {
    #[derive(Debug)]
    struct Select { preferred: Option<ironsmith::ObjectId>, forbidden: ironsmith::ObjectId }
    impl ironsmith::decision::DecisionMaker for Select {
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            assert!(!ctx.candidates.iter().any(|candidate| candidate.id == self.forbidden && candidate.legal));
            self.preferred.filter(|id| ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal)).into_iter().collect()
        }
    }
    let definition = compile_to_runtime_definition("Stinging Study",
        "Type: Instant\nYou draw X cards and you lose X life, where X is the mana value of a commander you own on the battlefield or in the command zone.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for chosen_zone in [None, Some(Zone::Battlefield), Some(Zone::Command)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let commander_card = CardBuilder::new(CardId::new(), "Commander choice witness")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(3)])).build();
        let foreign = game.create_object_from_card(&commander_card, bob, Zone::Command);
        game.set_as_commander(foreign, bob);
        let preferred = chosen_zone.map(|zone| {
            let commander = game.create_object_from_card(&commander_card, alice, zone);
            game.set_as_commander(commander, alice);
            commander
        });
        let library_card = CardBuilder::new(CardId::new(), "Draw witness").card_types(vec![CardType::Land]).build();
        for _ in 0..4 { game.create_object_from_card(&library_card, alice, Zone::Library); }
        let mut dm = Select { preferred, forbidden: foreign };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let amount = if chosen_zone.is_some() { 3 } else { 0 };
        assert_eq!(game.player(alice).unwrap().life, 20 - amount);
        assert_eq!(game.player(alice).unwrap().hand.len(), amount as usize);
    }
}

#[test]
fn shared_discard_types_compare_only_this_resolution_and_reveal_after_choices() {
    #[derive(Debug, Default)]
    struct DiscardChoices { chosen: usize, selected: Vec<ironsmith::ObjectId>, public_views: Vec<usize> }
    impl ironsmith::decision::DecisionMaker for DiscardChoices {
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            assert!(self.selected.iter().all(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Hand)),
                "all players must finish choosing before any selected card moves");
            let picked: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal).take(1).map(|candidate| candidate.id).collect();
            self.chosen += usize::from(!picked.is_empty());
            self.selected.extend(picked.iter().copied());
            picked
        }
        fn view_cards(&mut self, _: &ironsmith::GameState, _: PlayerId, _: &[ironsmith::ObjectId], ctx: &ironsmith::decisions::context::ViewCardsContext) {
            if ctx.public { self.public_views.push(self.chosen); }
        }
    }
    let definition = compile_to_runtime_definition("Creeping Dread",
        "Type: Enchantment\nAt the beginning of your upkeep, each player discards a card. Each opponent who discarded a card that shares a card type with the card you discarded loses 3 life. (Players reveal the discarded cards simultaneously.)", false).unwrap();
    let program = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(&trigger.effects), _ => None,
    }).unwrap();
    let cases = [
        (Some(vec![CardType::Creature]), Some(vec![CardType::Artifact, CardType::Creature]), Some(vec![CardType::Kindred, CardType::Instant]), [17, 20]),
        (None, Some(vec![CardType::Creature]), Some(vec![CardType::Creature]), [20, 20]),
        (Some(vec![CardType::Land]), Some(vec![CardType::Creature]), Some(vec![CardType::Instant]), [20, 20]),
        (Some(vec![CardType::Artifact, CardType::Creature]), Some(vec![CardType::Artifact, CardType::Creature]), Some(vec![CardType::Land]), [17, 20]),
        (Some(vec![CardType::Instant]), Some(vec![CardType::Instant]), Some(vec![CardType::Sorcery]), [17, 20]),
        (None, None, None, [20, 20]),
    ];
    for (your_types, bob_types, carol_types, expected) in cases {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = |types| CardBuilder::new(CardId::new(), "Discard witness")
            .card_types(types).subtypes(vec![Subtype::Elf])
            .supertypes(vec![ironsmith::types::Supertype::Legendary]).build();
        game.create_object_from_card(&card(vec![CardType::Creature]), alice, Zone::Graveyard);
        let hands = [your_types, bob_types, carol_types];
        let chosen_count = hands.iter().filter(|hand| hand.is_some()).count();
        for (index, types) in hands.into_iter().enumerate() {
            if let Some(types) = types { game.create_object_from_card(&card(types), PlayerId::from_index(index as u8), Zone::Hand); }
        }
        let mut dm = DiscardChoices::default();
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in program.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, expected[0]);
        assert_eq!(game.player(PlayerId::from_index(2)).unwrap().life, expected[1]);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(PlayerId::from_index(3)).unwrap().life, 20);
        for index in 0..4 { assert!(game.player(PlayerId::from_index(index)).unwrap().hand.is_empty()); }
        assert!(dm.public_views.iter().all(|choices| *choices == chosen_count), "discard openings must wait for every selection: {:?}", dm.public_views);
    }
}

#[test]
fn crafted_return_chooses_only_shared_exiled_material_types_before_entry() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::snapshot::ObjectSnapshot;
    let mut front = compile_to_runtime_definition("Eye of Ojer Taq", "Mana cost: {3}\nType: Artifact\n{T}: Add one mana of any color.\nCraft with two that share a card type {6}", false).unwrap();
    let mut back = compile_to_runtime_definition("Apex Observatory", "Type: Artifact\nThis artifact enters tapped. As it enters, choose a card type shared among two exiled cards used to craft it.\n{T}: The next spell you cast this turn of the chosen type can be cast without paying its mana cost.", false).unwrap();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    front.card.transforming_dfc = true;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.transforming_dfc = true;
    let craft = front.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(ability) if ability.effects.all_effects().iter().any(|effect|
            effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>().is_some()) => Some(ability),
        _ => None,
    }).expect("craft return");
    let material_choice = craft.mana_cost.costs().iter().find_map(|cost| cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>())).unwrap();
    assert!(material_choice.filter.shares_card_type);
    assert_eq!(material_choice.count.min, 2);
    assert_eq!(material_choice.count.max, Some(2));
    #[derive(Default)]
    struct Menu { types: Vec<String> }
    impl ironsmith::decision::DecisionMaker for Menu {
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            ironsmith::decision::DecisionMaker::decide_objects(&mut ironsmith::decision::SelectFirstDecisionMaker, game, ctx)
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.description == "Choose a card type" {
                self.types = ctx.options.iter().map(|option| option.description.to_ascii_lowercase()).collect();
                vec![ctx.options.last().unwrap().index]
            } else { vec![ctx.options.first().unwrap().index] }
        }
    }
    for (first, second, expected, chosen) in [
        (vec![CardType::Artifact, CardType::Creature], vec![CardType::Artifact, CardType::Creature], vec!["artifact", "creature"], Some(CardType::Creature)),
        (vec![CardType::Kindred, CardType::Instant], vec![CardType::Kindred, CardType::Sorcery], vec!["kindred"], Some(CardType::Kindred)),
        (vec![CardType::Instant], vec![CardType::Instant], vec!["instant"], Some(CardType::Instant)),
        (vec![CardType::Land], vec![CardType::Land], vec!["land"], Some(CardType::Land)),
        (vec![CardType::Artifact], vec![CardType::Creature], vec![], None),
    ] {
        let alice = PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.register_linked_face_definition(&back);
        // Resolution begins after the craft costs have exiled the source and materials.
        let source = game.create_object_from_definition(&front, alice, Zone::Exile);
        for types in [first, second] {
            let material = CardBuilder::new(CardId::new(), "Craft material").card_types(types).build();
            let id = game.create_object_from_card(&material, alice, Zone::Exile);
            game.add_exiled_with_source_link(source, id);
        }
        let unrelated = CardBuilder::new(CardId::new(), "Unrelated exiled card").card_types(vec![CardType::Land, CardType::Artifact]).build();
        let ignored = game.create_object_from_card(&unrelated, alice, Zone::Exile);
        let source_snapshot = ObjectSnapshot::from_object_id(&game, source).unwrap();
        let mut dm = Menu::default();
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(ironsmith::tag::SOURCE_COST_PUBLIC_ARRIVAL_TAG, vec![source_snapshot]);
        for effect in craft.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        let arrived = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Apex Observatory").unwrap();
        assert_eq!(dm.types, expected);
        assert_eq!(game.chosen_card_type(arrived), chosen);
        assert!(game.is_tapped(arrived));
        assert_eq!(game.object(ignored).unwrap().zone, Zone::Exile);
        assert_eq!(game.get_exiled_with_source_links(arrived).len(), 2);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let free_cast = back.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(&ability.effects), _ => None,
        }).unwrap();
        let source_memory = ObjectSnapshot::from_object_id(&game, arrived).unwrap();
        assert_eq!(source_memory.chosen_card_type, chosen);
        if chosen == Some(CardType::Kindred) {
            // The activated ability can resolve after its source is removed.
            let mut leave = ironsmith::effects::EffectContext::new_default(arrived, alice);
            ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::exile(ironsmith::target::ChooseSpec::Source), &mut leave).unwrap();
        }
        let mut grant_ctx = ironsmith::effects::EffectContext::new_default(arrived, alice);
        grant_ctx.source_snapshot = Some(source_memory);
        for effect in free_cast.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut grant_ctx).unwrap(); }
        drop(grant_ctx);
        let types = match chosen {
            Some(CardType::Kindred) => vec![CardType::Kindred, CardType::Instant],
            Some(kind) => vec![kind], None => vec![CardType::Creature],
        };
        let spell = CardBuilder::new(CardId::new(), "Matching spell").card_types(types)
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(9)]])).build();
        let matching = game.create_object_from_card(&spell, alice, Zone::Hand);
        let wrong_type = if chosen == Some(CardType::Creature) { CardType::Artifact } else { CardType::Creature };
        let wrong = CardBuilder::new(CardId::new(), "Wrong type").card_types(vec![wrong_type])
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(9)]])).build();
        let wrong = game.create_object_from_card(&wrong, alice, Zone::Hand);
        let has_free_cast = |game: &ironsmith::GameState, object| ironsmith::decision::compute_legal_actions(game, alice).unwrap()
            .iter().any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, casting_method, .. }
                if *spell_id == object && *casting_method != ironsmith::alternative_cast::CastingMethod::Normal));
        let expected_free = chosen.is_some() && chosen != Some(CardType::Land);
        assert_eq!(has_free_cast(&game, matching), expected_free, "the permission must retain the chosen card type");
        assert!(!has_free_cast(&game, wrong));
        if game.object(arrived).is_some() {
            let mut leave_ctx = ironsmith::effects::EffectContext::new_default(arrived, alice);
            ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::exile(ironsmith::target::ChooseSpec::Source), &mut leave_ctx).unwrap();
        }
        assert_eq!(has_free_cast(&game, matching), expected_free, "the resolved permission must survive its source leaving");
        assert!(!has_free_cast(&game, wrong));
    }

    // Also exercise the real activation/payment path, including source-ID changes.
    use ironsmith::game_loop::*;
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.register_linked_face_definition(&back);
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 6);
    let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
    let old_card = CardBuilder::new(CardId::new(), "Older linked artifact").card_types(vec![CardType::Artifact]).build();
    let old_card = game.create_object_from_card(&old_card, alice, Zone::Exile);
    game.add_exiled_with_source_link(source, old_card);
    for (types, zone) in [(vec![CardType::Creature], Zone::Battlefield), (vec![CardType::Artifact, CardType::Creature], Zone::Graveyard)] {
        let material = CardBuilder::new(CardId::new(), "Paid craft material").card_types(types).build();
        game.create_object_from_card(&material, alice, zone);
    }
    let ability_index = front.abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Activated(value) if std::ptr::eq(value, craft))).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = Menu::default();
    let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(ironsmith::decision::LegalAction::ActivateAbility { source, ability_index }), &mut dm).unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && !game.stack.is_empty() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let arrived = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Apex Observatory").unwrap();
    assert_eq!(dm.types, vec!["creature"]);
    assert_eq!(game.chosen_card_type(arrived), Some(CardType::Creature));
    assert!(game.is_tapped(arrived));
    assert_eq!(game.get_exiled_with_source_links(arrived).len(), 2);
    assert!(!game.get_exiled_with_source_links(arrived).contains(&old_card));
    assert_eq!(game.object(old_card).unwrap().zone, Zone::Exile);
}

#[test]
fn damage_recipient_choice_keeps_the_prior_player_and_only_their_planeswalkers() {
    use ironsmith::ability::AbilityKind;
    #[derive(Default)]
    struct Pick { walker: bool, menus: Vec<Vec<String>> }
    impl ironsmith::decision::DecisionMaker for Pick {
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.menus.push(ctx.options.iter().map(|option| option.description.clone()).collect());
            let name = if self.walker { "Bob's walker" } else { "Bob" };
            vec![ctx.options.iter().find(|option| option.description == name).expect("chosen damage recipient must be offered").index]
        }
    }
    for (name, oracle, amount) in [
        ("Curse of the Pierced Heart", "Type: Enchantment — Aura\nEnchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.", 1),
        ("Vial Smasher the Fierce", "Mana cost: {1}{B}{R}\nType: Legendary Creature — Goblin Berserker\nPower/Toughness: 2/3\nWhenever you cast your first spell each turn, choose an opponent at random. Vial Smasher deals damage equal to that spell's mana value to that player or a planeswalker that player controls.\nPartner", 4),
    ] {
        let definition = compile_to_runtime_definition(name, oracle, false).unwrap();
        let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(ability) => Some(ability), _ => None,
        }).unwrap();
        for choose_walker in [false, true] {
            let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            if amount == 1 { assert!(game.attach_object_to_target(source, ironsmith::object::AttachmentTarget::Player(bob))); }
            let walker = |name| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Planeswalker]).build();
            let bob_walker = game.create_object_from_card(&walker("Bob's walker"), bob, Zone::Battlefield);
            let wrong_walker = game.create_object_from_card(&walker("Alice's walker"), alice, Zone::Battlefield);
            for id in [bob_walker, wrong_walker] { game.object_mut(id).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty, 8); }
            let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(&game, &trigger.effects, alice, Some(source), None);
            assert!(requirements.is_empty(), "the damage recipient is chosen on resolution, not targeted");
            let spell = CardBuilder::new(CardId::new(), "Triggering spell").card_types(vec![CardType::Instant])
                .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(4)]])).build();
            let spell = game.create_object_from_card(&spell, alice, Zone::Stack);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, spell).unwrap();
            let mut dm = Pick { walker: choose_walker, ..Default::default() };
            let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
            if amount == 4 {
                ctx = ctx.with_triggering_event(ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::SpellCastEvent::new_with_snapshot(spell, alice, Zone::Hand, snapshot.clone()),
                    ironsmith::provenance::ProvNodeId::default(),
                ));
            }
            ctx.set_tagged_objects("triggering", vec![snapshot]);
            for effect in trigger.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            drop(ctx);
            assert_eq!(dm.menus.last().unwrap(), &vec!["Bob".to_string(), "Bob's walker".to_string()]);
            assert_eq!(game.player(bob).unwrap().life, if choose_walker { 20 } else { 20 - amount });
            assert_eq!(game.counter_count(bob_walker, ironsmith::object::CounterType::Loyalty), if choose_walker { 8 - amount as u32 } else { 8 });
            assert_eq!(game.counter_count(wrong_walker, ironsmith::object::CounterType::Loyalty), 8);
            assert_eq!(game.player(alice).unwrap().life, 20);
        }
    }
}

#[test]
fn returned_creature_controller_rewards_use_original_successful_moves_including_tokens() {
    let definition = compile_to_runtime_definition("Faerie Slumber Party", "Type: Sorcery\nReturn all creatures to their owners' hands. For each opponent who controlled a creature returned this way, you create two 1/1 blue Faerie creature tokens with flying and \"This token can block only creatures with flying.\"", false).unwrap();
    for case in 0..5 {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let dan = PlayerId::from_index(3);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let creature = CardBuilder::new(CardId::new(), "Returned creature").card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2,2)).build();
        game.create_object_from_card(&creature, bob, Zone::Hand); // Older unrelated return cannot count.
        if case <= 1 || case == 4 {
            let borrowed = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            game.set_current_controller(borrowed, bob).unwrap();
            if case == 0 { game.create_object_from_card(&creature, bob, Zone::Battlefield); }
            if case == 4 {
                game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
                    source, alice,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(borrowed), Some(Zone::Battlefield), Some(Zone::Hand)),
                    ironsmith::replacement::ReplacementAction::Instead(vec![Effect::new(ironsmith::effects::MoveToZoneEffect::to_exile(ChooseSpec::SpecificObject(borrowed)))]),
                ));
            }
        }
        if case == 0 || case == 2 {
            let token = game.create_object_from_card(&creature, dan, Zone::Battlefield);
            game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
        }
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let tokens = game.battlefield.iter().filter_map(|id| game.object(*id))
            .filter(|object| object.kind == ironsmith::object::ObjectKind::Token && object.subtypes.contains(&Subtype::Faerie)).collect::<Vec<_>>();
        assert_eq!(tokens.len(), match case {0 => 4, 1|2 => 2, _ => 0}, "case {case}");
        assert!(tokens.iter().all(|token| game.controller_of(token) == alice));
        for token in tokens {
            assert_eq!(game.current_power(token.id), Some(1));
            assert_eq!(game.current_toughness(token.id), Some(1));
            assert!(game.current_has_static_ability_id(token.id, ironsmith::static_abilities::StaticAbilityId::Flying));
            assert!(game.current_has_static_ability_id(token.id, ironsmith::static_abilities::StaticAbilityId::CanBlockOnlyFlying));
        }
    }
}

#[test]
fn lifetime_trigger_limit_keeps_tapped_condition_and_resets_only_for_new_incarnation() {
    let definition = compile_to_runtime_definition("Acrobatic Cheerleader", "Type: Creature — Human Survivor\nPower/Toughness: 2/2\nSurvival — At the beginning of your second main phase, if this creature is tapped, put a flying counter on it. This ability triggers only once.", false).unwrap();
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let event = |player, ordinal| ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent::new(player).with_main_phase_ordinal(ordinal),
        ironsmith::provenance::ProvNodeId::default(),
    );
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 2)).is_empty(), "untapped condition still applies");
    game.tap(source);
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 3)).is_empty(), "second main, not every later main");
    assert!(ironsmith::triggers::check_triggers(&game, &event(bob, 2)).is_empty(), "only the controller's main");
    let watcher = CardBuilder::new(CardId::new(), "Trigger watcher").card_types(vec![CardType::Enchantment]).build();
    let watcher = game.create_object_from_card(&watcher, alice, Zone::Battlefield);
    game.object_mut(watcher).unwrap().abilities = vec![ironsmith::ability::Ability::triggered(
        ironsmith::triggers::Trigger::another_ability_triggers(), vec![Effect::gain_life(1)],
    )].into();
    let matches = ironsmith::triggers::check_triggers(&game, &event(alice, 2));
    assert_eq!(matches.len(), 1);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    queue.add(matches[0].clone()); queue.add(matches[0].clone());
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 2, "one allowed occurrence and exactly one watcher notification");
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.player(alice).unwrap().life, 21, "suppressed triggers emit no extra notification");
    assert_eq!(game.counter_count(source, ironsmith::object::CounterType::Flying), 1, "the limit cannot stop its first registered trigger resolving");
    game.next_turn(); game.next_turn(); game.tap(source);
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 2)).is_empty(), "limit survives turns");
    game.set_current_controller(source, bob).unwrap();
    assert!(ironsmith::triggers::check_triggers(&game, &event(bob, 2)).is_empty(), "limit survives control change");
    let hand = game.move_object_by_effect(source, Zone::Hand).unwrap();
    let returned = game.move_object_by_effect(hand, Zone::Battlefield).unwrap();
    game.tap(returned);
    let matches = ironsmith::triggers::check_triggers(&game, &event(alice, 2));
    assert_eq!(matches.len(), 1, "a new incarnation may trigger again");
    queue.add(matches[0].clone());
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(returned, ironsmith::object::CounterType::Flying), 1);
    let fresh = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.tap(fresh);
    let matches = ironsmith::triggers::check_triggers(&game, &event(alice, 2));
    assert_eq!(matches.len(), 1);
    queue.add(matches[0].clone());
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    game.untap(fresh);
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(fresh, ironsmith::object::CounterType::Flying), 0, "tapped condition is checked again on resolution");
    game.tap(fresh);
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 2)).is_empty(), "the lifetime allowance was consumed when it triggered");
}

#[test]
fn search_offer_rewards_wait_for_all_offers_and_shuffle_each_actual_searcher_once() {
    #[derive(Default)]
    struct Answers { accepted: Vec<PlayerId>, selections: Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.accepted.contains(&context.player)
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.selections.push(context.player);
            context.candidates.iter().filter(|candidate| candidate.legal).take(1).map(|candidate| candidate.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("Tempt with Discovery", "Type: Sorcery\nTempting offer — Search your library for a land card and put it onto the battlefield. Each opponent may search their library for a land card and put it onto the battlefield. For each opponent who searches a library this way, search your library for a land card and put it onto the battlefield. Then each player who searched a library this way shuffles.", false).unwrap();
    fn participant_tag(effect: &Effect, found: &mut Option<ironsmith::tag::TagKey>) {
        if let Some(choice) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() {
            if let Some(tag) = &choice.search_participant_tag { *found = Some(tag.clone()); }
        }
        effect.visit_child_effects(&mut |child| participant_tag(child, found));
    }
    let mut tag = None;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { participant_tag(effect, &mut tag); }
    let tag = tag.expect("exact participant capture on the native search instruction");
    for case in 0..4 {
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let dan = PlayerId::from_index(3);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let land = CardBuilder::new(CardId::new(), "Searchable land").card_types(vec![CardType::Land]).build();
        for _ in 0..4 { game.create_object_from_card(&land, alice, Zone::Library); }
        game.create_object_from_card(&land, bob, Zone::Library);
        let blocked = match case {2 => Some(bob), 3 => Some(alice), _ => None};
        if let Some(player) = blocked {
            let lock = CardBuilder::new(CardId::new(), "Search restriction").card_types(vec![CardType::Enchantment]).build();
            let lock = game.create_object_from_card(&lock, alice, Zone::Battlefield);
            game.object_mut(lock).unwrap().abilities = vec![ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::restriction(
                    ironsmith::effect::Restriction::SearchLibraries(ironsmith::target::PlayerFilter::Specific(player)), "search restriction".to_string(),
                ),
            )].into();
        }
        game.refresh_continuous_state().unwrap();
        let mut dm = Answers { accepted: if case == 0 {vec![bob,dan]} else if case == 1 {vec![]} else {vec![bob]}, ..Default::default() };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        ctx.set_tagged_players(tag.clone(), vec![bob,dan]); // Previous repetitions cannot leak participants.
        let mut searches = Vec::new(); let mut shuffles = Vec::new();
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            let result = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            for event in result.events {
                if let Some(search) = event.downcast::<ironsmith::events::SearchLibraryEvent>() { searches.push(search.player); }
                if let Some(shuffle) = event.downcast::<ironsmith::events::ShuffleLibraryEvent>() { shuffles.push(shuffle.player); }
            }
        }
        let expected = match case {0 => vec![alice,bob,dan,alice,alice], 1 => vec![alice], 2 => vec![alice], _ => vec![bob]};
        assert_eq!(searches, expected, "actual searches and instruction order, case {case}");
        shuffles.sort();
        let mut expected = match case {0 => vec![alice,bob,dan], 1|2 => vec![alice], _ => vec![bob]}; expected.sort();
        assert_eq!(shuffles, expected, "one final shuffle per actual searcher, case {case}");
        let alice_lands = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| game.controller_of(object) == alice && object.card_types.contains(&CardType::Land)).count();
        assert_eq!(alice_lands, match case {0 => 3, 3 => 0, _ => 1}, "reward searches belong to the controller");
    }
}

#[test]
fn bounded_filtered_exile_casts_recheck_the_pool_and_preserve_owner_and_face_up_limits() {
    #[derive(Default)] struct Answers { limit: usize, accepted: usize, labels: Vec<String> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            if self.accepted < self.limit { self.accepted += 1; true } else { false }
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.labels.extend(context.options.iter().map(|option| option.description.clone()));
            vec![0]
        }
    }
    let definition = compile_to_runtime_definition("Ashiok, Nightmare Muse", "Type: Legendary Planeswalker — Ashiok\nLoyalty: 5\n+1: Create a 2/3 blue and black Nightmare creature token with \"Whenever this token attacks or blocks, each opponent exiles the top two cards of their library.\"\n−3: Return target nonland permanent to its owner's hand, then that player exiles a card from their hand.\n−7: You may cast up to three spells from among face-up cards your opponents own from exile without paying their mana costs.", false).unwrap();
    let activated = definition.abilities.iter().filter_map(|ability| match &ability.kind { ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None }).collect::<Vec<_>>();
    assert_eq!(activated.len(), 3);
    let ultimate = activated[2];
    for limit in [0, 1, 3, 5] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell = |name| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Instant])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(9)])).build();
        let mut eligible = Vec::new();
        for (owner, name) in [(bob,"Bob A"),(bob,"Bob B"),(carol,"Carol A"),(carol,"Carol B")] {
            eligible.push(game.create_object_from_card(&spell(name), owner, Zone::Exile));
        }
        let own = game.create_object_from_card(&spell("Forbidden own spell"), alice, Zone::Exile);
        let hidden = game.create_object_from_card(&spell("Forbidden hidden spell"), bob, Zone::Exile); game.set_face_down(hidden);
        let land = CardBuilder::new(CardId::new(), "Forbidden land").card_types(vec![CardType::Land]).build();
        let land = game.create_object_from_card(&land, bob, Zone::Exile);
        let mut dm = Answers { limit, ..Default::default() };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in ultimate.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        assert_eq!(game.stack.len(), limit.min(3), "bounded immediate casts, requested {limit}");
        assert_eq!(eligible.iter().filter(|id| game.object(**id).is_some_and(|object| object.zone == Zone::Exile)).count(), 4-limit.min(3), "each cast must leave the pool before the next choice");
        for id in [own, hidden, land] { assert_eq!(game.object(id).unwrap().zone, Zone::Exile); }
        for entry in &game.stack { assert_eq!(entry.controller, alice); }
        assert!(dm.labels.iter().all(|label| !label.contains("Forbidden")), "illegal owners, face-down cards and lands must never be offered");
    }
}

#[test]
fn creature_payment_prohibition_applies_to_casts_and_activations_but_not_resolution_costs() {
    use ironsmith::costs::{Cost, CostContext, PaymentReason};
    #[derive(Default)] struct Picks;
    impl ironsmith::decision::DecisionMaker for Picks {
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let choices = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect::<Vec<_>>();
            assert!(choices.iter().all(|id| !game.current_has_card_type(*id, CardType::Creature)), "prohibited creatures must not reach the payment choice");
            choices.into_iter().take(context.min.max(1)).collect()
        }
    }
    let definition = compile_to_runtime_definition("Angel of Jubilation", "Mana cost: {1}{W}{W}{W}\nType: Creature — Angel\nPower/Toughness: 3/3\nFlying\nOther nonblack creatures you control get +1/+1.\nPlayers can't pay life or sacrifice creatures to cast spells or activate abilities.", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
    let angel = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let card = |types| CardBuilder::new(CardId::new(), "Payment subject").card_types(types).power_toughness(PowerToughness::fixed(2,2)).build();
    let creature = game.create_object_from_card(&card(vec![CardType::Creature]),bob,Zone::Battlefield);
    let land_creature = game.create_object_from_card(&card(vec![CardType::Land,CardType::Creature]),bob,Zone::Battlefield);
    let artifact = game.create_object_from_card(&card(vec![CardType::Artifact]),bob,Zone::Battlefield);
    let land = game.create_object_from_card(&card(vec![CardType::Land]),bob,Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(game.current_has_static_ability_id(angel, ironsmith::static_abilities::StaticAbilityId::Flying));
    for payer in [alice,bob] {
        for reason in [PaymentReason::CastSpell,PaymentReason::ActivateAbility,PaymentReason::ActivateManaAbility] {
            let mut dm = Picks;
            let ctx = CostContext::new(creature,payer,&mut dm).with_reason(reason);
            assert!(Cost::life(2).can_pay(&game,&ctx).is_err());
            assert!(Cost::life(0).can_pay(&game,&ctx).is_ok(), "zero-life payments remain legal");
        }
    }
    for reason in [PaymentReason::CastSpell,PaymentReason::ActivateAbility,PaymentReason::ActivateManaAbility] {
        for subject in [creature,land_creature] {
            let mut dm = Picks; let ctx = CostContext::new(subject,bob,&mut dm).with_reason(reason);
            assert!(Cost::sacrifice_self().can_pay(&game,&ctx).is_err());
        }
        for subject in [artifact,land] {
            let mut dm = Picks; let ctx = CostContext::new(subject,bob,&mut dm).with_reason(reason);
            assert!(Cost::sacrifice_self().can_pay(&game,&ctx).is_ok());
        }
        let mut paid = game.clone(); let mut dm = Picks;
        let mut ctx = CostContext::new(creature,bob,&mut dm).with_reason(reason);
        Cost::sacrifice(ironsmith::target::ObjectFilter::default()).pay(&mut paid,&mut ctx).unwrap();
        assert!(paid.object(creature).is_some()); assert!(paid.object(land_creature).is_some());
    }
    let mut paid = game.clone(); let mut dm = ironsmith::decision::AutoPassDecisionMaker;
    let mut ctx = CostContext::new(creature,bob,&mut dm).with_reason(PaymentReason::Effect);
    Cost::life(2).pay(&mut paid,&mut ctx).unwrap();
    Cost::sacrifice_self().pay(&mut paid,&mut ctx).unwrap();
    assert_eq!(paid.player(bob).unwrap().life,18);
    assert!(paid.object(creature).is_none(), "resolution costs may sacrifice creatures");
    game.move_object_by_effect(angel,Zone::Graveyard).unwrap();
    let mut dm = Picks; let ctx = CostContext::new(land_creature,bob,&mut dm).with_reason(PaymentReason::CastSpell);
    assert!(Cost::sacrifice_self().can_pay(&game,&ctx).is_ok(), "source leaving removes its prohibition");
    assert!(Cost::life(2).can_pay(&game,&ctx).is_ok());
}

#[test]
fn excess_damage_trigger_uses_historical_typed_sources_and_exact_recipient() {
    let definition = compile_to_runtime_definition("Aegar, the Freezing Flame", "Mana cost: {1}{U}{R}\nType: Legendary Creature — Giant Wizard\nPower/Toughness: 3/3\nWhenever a creature or planeswalker an opponent controls is dealt excess damage, if a Giant, Wizard, or spell you controlled dealt damage to it this turn, draw a card.", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for (label, types, subtypes, zone, owner, prior, current, expected) in [
        ("earlier Giant", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 1),
        ("earlier Wizard", vec![CardType::Creature], vec![Subtype::Wizard], Zone::Battlefield, alice, true, false, 1),
        ("earlier spell", vec![CardType::Instant], vec![], Zone::Stack, alice, true, false, 1),
        ("current Giant", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 1),
        ("current spell", vec![CardType::Instant], vec![], Zone::Stack, alice, false, true, 1),
        ("opponent spell", vec![CardType::Instant], vec![], Zone::Stack, bob, true, false, 0),
        ("opponent Giant", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, bob, true, false, 0),
        ("unqualified source", vec![CardType::Artifact], vec![], Zone::Battlefield, alice, true, false, 0),
        ("Giant permanent", vec![CardType::Kindred,CardType::Enchantment], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 1),
        ("Giant card", vec![CardType::Creature], vec![Subtype::Giant], Zone::Graveyard, alice, true, false, 0),
        ("different recipient", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("new incarnation", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("previous turn", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("prevented earlier damage", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("exact lethal", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 0),
        ("own recipient", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 0),
        ("planeswalker recipient", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 1),
        ("combat damage", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 1),
        ("lost Giant subtype", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 1),
        ("gained Giant subtype", vec![CardType::Creature], vec![], Zone::Battlefield, alice, true, false, 0),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let victim_types = if label == "planeswalker recipient" { vec![CardType::Planeswalker] } else { vec![CardType::Creature] };
        let toughness = if label == "exact lethal" { 5 } else { 4 };
        let victim_card = CardBuilder::new(CardId::new(),"Recipient").card_types(victim_types).power_toughness(PowerToughness::fixed(4,toughness)).build();
        let mut victim = game.create_object_from_card(&victim_card,if label == "own recipient" { alice } else { bob },Zone::Battlefield);
        if label == "planeswalker recipient" { game.object_mut(victim).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty,4); }
        let source_card = CardBuilder::new(CardId::new(),"Historical source").card_types(types).subtypes(subtypes).power_toughness(PowerToughness::fixed(1,1)).build();
        let source = game.create_object_from_card(&source_card,owner,zone);
        let other_card = CardBuilder::new(CardId::new(),"Other damage source").card_types(vec![CardType::Artifact]).build();
        let other = game.create_object_from_card(&other_card,bob,Zone::Battlefield);
        let library_card = CardBuilder::new(CardId::new(),"Drawn card").card_types(vec![CardType::Land]).build();
        game.create_object_from_card(&library_card,alice,Zone::Library);
        if prior {
            let mut ctx = ironsmith::effects::EffectContext::new_default(source,owner);
            if label == "prevented earlier damage" {
                ironsmith::effects::execute_effect(&mut game,&Effect::prevent_damage(1,ChooseSpec::SpecificObject(victim),ironsmith::effect::Until::EndOfTurn),&mut ctx).unwrap();
            }
            let prior_recipient = if label == "different recipient" { game.create_object_from_card(&victim_card,bob,Zone::Battlefield) } else { victim };
            ironsmith::effects::execute_effect(&mut game,&Effect::deal_damage(1,ChooseSpec::SpecificObject(prior_recipient)),&mut ctx).unwrap();
            // Historical controller and subtype survive later changes and a spell leaving the stack.
            if zone == Zone::Stack { game.move_object_by_effect(source,Zone::Graveyard).unwrap(); }
            else if !matches!(label,"lost Giant subtype" | "gained Giant subtype") { game.set_current_controller(source,if owner==alice {bob} else {alice}).unwrap(); }
            if label == "lost Giant subtype" { game.object_mut(source).unwrap().subtypes.clear(); }
            if label == "gained Giant subtype" { game.object_mut(source).unwrap().subtypes = vec![Subtype::Giant].into(); }
            if label == "new incarnation" {
                let graveyard = game.move_object_by_effect(victim,Zone::Graveyard).unwrap();
                victim = game.move_object_by_effect(graveyard,Zone::Battlefield).unwrap();
            }
            if label == "previous turn" { game.next_turn(); }
        }
        let damager = if current {source} else {other};
        let controller = if current {owner} else {bob};
        let mut ctx = ironsmith::effects::EffectContext::new_default(damager,controller);
        let mut damage = ironsmith::effects::DealDamageEffect::new(5,ChooseSpec::SpecificObject(victim));
        damage.source_is_combat = label == "combat damage";
        ironsmith::effects::execute_effect(&mut game,&Effect::new(damage),&mut ctx).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
        assert_eq!(queue.entries.len(),expected,"{label}: trigger matching");
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        if game.object(victim).is_some() { game.move_object_by_effect(victim,Zone::Graveyard).unwrap(); }
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        assert_eq!(game.player(alice).unwrap().hand.len(),expected,"{label}: resolution uses the damaged object's captured identity");
    }
}

#[test]
fn draft_noted_name_search_keeps_optional_zones_cast_and_shuffle_correlated() {
    struct Answers { zone: Zone, search: bool, cast: bool, fail: bool, zones: Vec<Zone>, booleans: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.booleans += 1;
            if self.booleans == 1 { self.search } else { self.cast }
        }
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            if self.fail { return vec![]; }
            context.candidates.iter().filter(|candidate| candidate.legal && game.object(candidate.id).is_some_and(|object| object.zone == self.zone && object.name == "Noted spell")).take(1).map(|candidate| candidate.id).collect()
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if context.description.to_lowercase().contains("zones to search") {
                return context.options.iter().enumerate().filter_map(|(index, option)| self.zones.iter().any(|zone| option.description.eq_ignore_ascii_case(&zone.to_string())).then_some(index)).collect();
            }
            vec![0]
        }
    }
    let definition = compile_to_runtime_definition("Aether Searcher", "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 6/4\nReveal this card as you draft it. Reveal the next card you draft and note its name.\nWhen this creature enters, you may search your hand and/or library for a card with a name noted as you drafted cards named Aether Searcher. You may cast it without paying its mana cost. If you search your library this way, shuffle.", false).unwrap();
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {ironsmith::ability::AbilityKind::Triggered(ability) => Some(ability), _ => None}).unwrap();
    fn optional_zones(effect: &Effect) -> bool {
        let mut found = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().is_some_and(|choice| choice.optional_search_zones);
        effect.visit_child_effects(&mut |child| found |= optional_zones(child));
        found
    }
    assert!(ability.effects.all_effects().into_iter().any(optional_zones), "authored and/or search zones must survive lowering");
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["hand", "library", "both", "decline search", "decline cast", "no note", "opponent note", "fail to find", "land", "blocked library"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(),"Noted spell").card_types(vec![if case == "land" {CardType::Land} else {CardType::Instant}]);
        let spell = if case == "land" { spell } else { spell.mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(9)])) }.build();
        let hand = game.create_object_from_card(&spell,alice,Zone::Hand);
        let library = game.create_object_from_card(&spell,alice,Zone::Library);
        let forbidden = CardBuilder::new(CardId::new(),"Unnoted spell").card_types(vec![CardType::Instant]).build();
        game.create_object_from_card(&forbidden,alice,Zone::Library);
        game.create_object_from_card(&spell,bob,Zone::Library);
        if case != "no note" { game.record_draft_noted_name(if case == "opponent note" { bob } else { alice },"Aether Searcher","Noted spell"); }
        if case == "blocked library" {
            let lock_card = CardBuilder::new(CardId::new(),"Search restriction").card_types(vec![CardType::Enchantment]).build();
            let lock = game.create_object_from_card(&lock_card,alice,Zone::Battlefield);
            game.object_mut(lock).unwrap().abilities = vec![ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::restriction(ironsmith::effect::Restriction::SearchLibraries(ironsmith::target::PlayerFilter::You), "Library search restriction".to_string()),
            )].into();
            game.refresh_continuous_state().unwrap();
        }
        let source_snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,source).unwrap();
        game.move_object_by_effect(source,Zone::Graveyard).unwrap(); // Trigger survives its source leaving.
        let zone = if matches!(case,"library" | "both" | "fail to find") {Zone::Library} else {Zone::Hand};
        let zones = if case == "both" {vec![Zone::Hand,Zone::Library]} else {vec![zone]};
        let mut dm = Answers {zone, search:case != "decline search",cast:case != "decline cast",fail:case == "fail to find",zones,booleans:0};
        let mut ctx = ironsmith::effects::EffectContext::new(source,alice,&mut dm);
        ctx.source_snapshot = Some(source_snapshot);
        let mut shuffles = 0;
        for effect in ability.effects.all_effects() {
            let result = ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();
            shuffles += result.events.iter().filter(|event| event.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count();
        }
        let cast = !matches!(case,"decline search" | "decline cast" | "no note" | "opponent note" | "fail to find" | "land");
        assert_eq!(game.stack.len(),usize::from(cast),"{case}: immediate free cast");
        assert_eq!(shuffles,usize::from(matches!(case,"library" | "both" | "fail to find")),"{case}: shuffle only an actually searched library");
        if cast { assert!(game.object(if zone == Zone::Hand {hand} else {library}).is_none_or(|object| object.zone == Zone::Stack)); }
    }
}

#[test]
fn tapped_to_pay_keyword_cost_triggers_only_for_actual_teamwork_contributions() {
    use ironsmith::costs::{Cost, CostContext, PaymentReason};
    struct Picks(Vec<ironsmith::ObjectId>);
    impl ironsmith::decision::DecisionMaker for Picks {
        fn decide_objects(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> { self.0.clone() }
    }
    let definition = compile_to_runtime_definition("Agent Maria Hill", "Mana cost: {W}\nType: Legendary Creature — Human Spy Hero\nPower/Toughness: 2/1\nWhenever Agent Maria Hill becomes tapped to pay a teamwork cost, put a +1/+1 counter on her and draw a card.", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["teamwork", "crew", "effect tap", "tap ability cost", "insufficient power", "multiple contributors", "other contributor", "untapped before resolution", "new incarnation"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let maria = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let other_maria = game.create_object_from_definition(&definition,bob,Zone::Battlefield);
        let support = CardBuilder::new(CardId::new(),"Support creature").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3,3)).build();
        let support = game.create_object_from_card(&support,alice,Zone::Battlefield);
        if case == "insufficient power" { game.move_object_by_effect(support,Zone::Hand).unwrap(); }
        let spell_card = CardBuilder::new(CardId::new(),"Payment source").card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Vehicle]).build();
        let spell = game.create_object_from_card(&spell_card,alice,Zone::Battlefield);
        let drawn = CardBuilder::new(CardId::new(),"Drawn card").card_types(vec![CardType::Land]).build();
        let drawn = game.create_object_from_card(&drawn,alice,Zone::Library);
        if case == "effect tap" {
            let mut ctx = ironsmith::effects::EffectContext::new_default(spell,alice);
            ironsmith::effects::execute_effect(&mut game,&Effect::tap(ChooseSpec::SpecificObject(maria)),&mut ctx).unwrap();
        } else if case == "tap ability cost" {
            let mut dm = Picks(vec![maria]);
            Cost::tap().pay(&mut game,&mut CostContext::new(maria,alice,&mut dm).with_reason(PaymentReason::ActivateAbility)).unwrap();
        } else {
            let cost = Cost::effect(if case == "crew" { ironsmith::effects::CrewCostEffect::new(2) } else { ironsmith::effects::CrewCostEffect::teamwork(if case == "insufficient power" {3} else if case == "multiple contributors" {5} else {2}) });
            let mut dm = Picks(if case == "multiple contributors" {vec![maria,support]} else if case == "other contributor" {vec![support]} else {vec![maria]});
            let mut ctx = CostContext::new(spell,alice,&mut dm).with_reason(PaymentReason::CastSpell);
            if case == "insufficient power" {
                assert!(cost.can_pay(&game,&ctx).is_err()); assert!(cost.pay(&mut game,&mut ctx).is_err());
            } else { cost.pay(&mut game,&mut ctx).unwrap(); }
        }
        let triggers = !matches!(case,"crew" | "effect tap" | "tap ability cost" | "insufficient power" | "other contributor");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
        assert_eq!(queue.entries.len(),usize::from(triggers),"{case}: exact source and typed cost");
        assert_eq!(game.counter_count(maria,ironsmith::object::CounterType::PlusOnePlusOne),0,"trigger can't increase power while costs are still being paid");
        assert!(game.player(alice).unwrap().library.contains(&drawn));
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        let mut returned = None;
        if case == "untapped before resolution" { game.untap(maria); }
        if case == "new incarnation" {
            let hand = game.move_object_by_effect(maria,Zone::Hand).unwrap();
            returned = Some(game.move_object_by_effect(hand,Zone::Battlefield).unwrap());
        }
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        assert_eq!(game.player(alice).unwrap().hand.iter().filter(|id| game.object(**id).is_some_and(|object| object.name == "Drawn card")).count(),usize::from(triggers),"{case}: draw");
        assert_eq!(game.counter_count(returned.unwrap_or(maria),ironsmith::object::CounterType::PlusOnePlusOne),u32::from(triggers && case != "new incarnation"),"{case}: source counter");
        assert_eq!(game.counter_count(other_maria,ironsmith::object::CounterType::PlusOnePlusOne),0);
    }
}

#[test]
fn conditional_landwalk_covers_all_basic_types_and_tracks_current_lands_and_controller() {
    let definition = compile_to_runtime_definition("Magnigoth Treefolk", "Mana cost: {4}{G}\nType: Creature — Treefolk\nPower/Toughness: 2/6\nDomain — For each basic land type among lands you control, this creature has landwalk of that type. (It can't be blocked as long as defending player controls a land of that type.)", false).unwrap();
    let types = [Subtype::Plains,Subtype::Island,Subtype::Swamp,Subtype::Mountain,Subtype::Forest];
    assert_eq!(definition.abilities.len(),6);
    assert!(matches!(&definition.abilities[0].kind,ironsmith::ability::AbilityKind::Static(ability) if matches!(&ability.compiled_model().unwrap().payload, ironsmith_core::StaticAbilityPayload::SourceLineStaticGroup {member_count:5})));
    for (ability, land_type) in definition.abilities[1..].iter().zip(types) {
        let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else {panic!("static conditional required")};
        let model = ability.compiled_model().unwrap();
        let ironsmith_core::static_ability_model::StaticAbilityPayload::Conditional {ability,condition,implied_words} = &model.payload else {panic!("typed conditional required")};
        assert!(*implied_words, "generated type cases must retain word provenance through lowering and transport");
        assert!(matches!(&ability.payload,ironsmith_core::static_ability_model::StaticAbilityPayload::Landwalk(ironsmith_core::LandwalkKind::Subtype{subtype,snow:false}) if *subtype == land_type));
        assert_eq!(*condition,ironsmith_core::Condition::YouControl(ironsmith_core::ObjectFilter::land().with_subtype(land_type)));
    }
    let surface = ironsmith::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(surface.contains("For each basic land type among lands you control"), "{surface}");
    for bad in ["partial", "duplicate", "snow", "authored", "extra qualifier", "different zone"] {
        let mut probe = definition.clone();
        probe.canonical_text = "unrelated cached text".into();
        if bad == "partial" { probe.abilities.pop(); }
        else if bad == "different zone" { probe.abilities[1].functional_zones = vec![Zone::Hand]; }
        else {
            let ironsmith::ability::AbilityKind::Static(ability) = &mut probe.abilities[1].kind else {unreachable!()};
            let mut model = ability.compiled_model().unwrap().clone();
            let ironsmith_core::StaticAbilityPayload::Conditional {ability: inner,condition,implied_words} = &mut model.payload else {unreachable!()};
            match bad {
                "duplicate" => {
                    inner.payload = ironsmith_core::StaticAbilityPayload::Landwalk(ironsmith_core::LandwalkKind::Subtype{subtype:Subtype::Island,snow:false});
                    *condition = ironsmith_core::Condition::YouControl(ironsmith_core::ObjectFilter::land().with_subtype(Subtype::Island));
                }
                "snow" => { inner.payload = ironsmith_core::StaticAbilityPayload::Landwalk(ironsmith_core::LandwalkKind::Subtype{subtype:Subtype::Plains,snow:true}); }
                "authored" => { *implied_words = false; }
                "extra qualifier" => {
                    let ironsmith_core::Condition::YouControl(filter) = condition else {unreachable!()};
                    filter.supertypes.push(ironsmith::types::Supertype::Basic);
                }
                _ => unreachable!(),
            }
            *ability = ironsmith::static_abilities::StaticAbility::from_model(model);
        }
        let text = ironsmith::runtime_display::compiled_text_lines(&probe).join("\n");
        assert!(!text.contains("For each basic land type"), "{bad} must not claim the complete generated family: {text}");
    }
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    let land = |subtypes| CardBuilder::new(CardId::new(),"Typed nonbasic land").card_types(vec![CardType::Land]).subtypes(subtypes).build();
    let blocker_card = CardBuilder::new(CardId::new(),"Blocker").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1,1)).build();
    let can_block = |game: &ironsmith::GameState, attacker, blocker| ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),game);
    for mask in 0..32 {
        for (index, defender_type) in types.iter().enumerate() {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
            let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
            let blocker = game.create_object_from_card(&blocker_card,bob,Zone::Battlefield);
            for (bit, land_type) in types.iter().enumerate() {
                if mask & (1 << bit) != 0 {game.create_object_from_card(&land(vec![*land_type]),alice,Zone::Battlefield);}
            }
            game.create_object_from_card(&land(vec![*defender_type]),bob,Zone::Battlefield);
            game.create_object_from_card(&land(types.to_vec()),carol,Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            assert_eq!(can_block(&game,source,blocker),mask & (1 << index) == 0,"own types {mask:05b}, defender {defender_type:?}; another player's lands never grant abilities");
        }
    }
    let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let blocker = game.create_object_from_card(&blocker_card,bob,Zone::Battlefield);
    let own = game.create_object_from_card(&land(vec![Subtype::Forest]),alice,Zone::Battlefield);
    let defending = game.create_object_from_card(&land(vec![Subtype::Forest]),bob,Zone::Battlefield);
    game.create_object_from_card(&land(vec![Subtype::Swamp]),carol,Zone::Battlefield);
    assert!(!can_block(&game,source,blocker));
    game.object_mut(own).unwrap().subtypes = vec![Subtype::Island].into();
    assert!(can_block(&game,source,blocker),"lost a land type");
    game.object_mut(defending).unwrap().subtypes = vec![Subtype::Island].into();
    assert!(!can_block(&game,source,blocker),"current defending land type");
    game.set_current_controller(source,carol).unwrap();
    assert!(can_block(&game,source,blocker),"new controller has only Swamp");
    game.set_current_controller(source,alice).unwrap();
    assert!(!can_block(&game,source,blocker));
    game.move_object_by_effect(own,Zone::Graveyard).unwrap();
    assert!(can_block(&game,source,blocker),"land leaves battlefield");
    game.create_object_from_card(&land(vec![Subtype::Island,Subtype::Forest]),alice,Zone::Battlefield);
    assert!(!can_block(&game,source,blocker),"a nonbasic dual land supplies both basic land types");
    game.object_mut(defending).unwrap().subtypes = vec![Subtype::Forest].into();
    assert!(!can_block(&game,source,blocker));
    let rewrite = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::SpecificObject(source),
        ironsmith::continuous::Modification::RewriteText(ironsmith_core::TextChange::basic_land_type(Subtype::Forest,Subtype::Mountain).unwrap()),
        ironsmith::effect::Until::EndOfTurn,
    ));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source,alice);
    ironsmith::effects::execute_effect(&mut game,&rewrite,&mut ctx).unwrap();
    assert!(!can_block(&game,source,blocker),"a text change cannot change unwritten land-type cases generated by the generic domain rule");
}

#[test]
fn draft_notes_creature_types_apply_one_bonus_to_current_matching_other_creatures() {
    let definition = compile_to_runtime_definition("Paliano Vanguard", "Mana cost: {1}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/2\nDraft this card face up.\nAs you draft a creature card, you may reveal it, note its creature types, then turn this card face down.\nOther creatures you control of a type you noted for cards named Paliano Vanguard get +1/+1.", false).unwrap();
    let surface = ironsmith::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(surface.contains("type you noted for cards named Paliano Vanguard"), "{surface}");
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let creature = |name, types| CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).subtypes(types).power_toughness(PowerToughness::fixed(2,2)).build();
    let matching = game.create_object_from_card(&creature("Two noted types",vec![Subtype::Human,Subtype::Soldier]),alice,Zone::Battlefield);
    let other = game.create_object_from_card(&creature("Unnoted type",vec![Subtype::Elf]),alice,Zone::Battlefield);
    let opposing = game.create_object_from_card(&creature("Opponent",vec![Subtype::Human]),bob,Zone::Battlefield);
    let typeless = game.create_object_from_card(&creature("Typeless",vec![]),alice,Zone::Battlefield);
    let artifact = CardBuilder::new(CardId::new(),"Noncreature").card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Human]).build();
    let artifact = game.create_object_from_card(&artifact,alice,Zone::Battlefield);
    let changeling = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(),"All creature types").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::changeling())).build();
    let changeling = game.create_object_from_definition(&changeling,alice,Zone::Battlefield);
    for id in [source,matching,other,opposing,typeless,changeling] {assert_eq!(game.calculated_power(id),Some(2),"empty notes");}
    game.record_draft_noted_creature_types(bob,"Paliano Vanguard",[Subtype::Elf]);
    game.record_draft_noted_creature_types(alice,"Different group",[Subtype::Elf]);
    assert_eq!(game.calculated_power(other),Some(2),"player and group isolation");
    game.record_draft_noted_creature_types(alice,"PALIANO VANGUARD",[Subtype::Human,Subtype::Soldier,Subtype::Human,Subtype::Equipment]);
    assert_eq!(game.draft_noted_creature_types(alice,"paliano vanguard"),&[Subtype::Human,Subtype::Soldier]);
    for id in [matching,changeling] {assert_eq!(game.calculated_power(id),Some(3),"one bonus even with multiple matching types");assert_eq!(game.calculated_toughness(id),Some(3));}
    for id in [source,other,opposing,typeless] {assert_eq!(game.calculated_power(id),Some(2),"source, unnoted, opponent, typeless excluded");}
    assert_eq!(game.calculated_power(artifact),None);
    game.object_mut(matching).unwrap().subtypes = vec![Subtype::Elf].into();
    assert_eq!(game.calculated_power(matching),Some(2),"current creature types");
    let change = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::SpecificObject(matching),
        ironsmith::continuous::Modification::AddSubtypes(vec![Subtype::Human]),
        ironsmith::effect::Until::EndOfTurn,
    ));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source,alice);
    ironsmith::effects::execute_effect(&mut game,&change,&mut ctx).unwrap();
    assert_eq!(game.object(matching).unwrap().subtypes.as_slice(),&[Subtype::Elf]);
    assert_eq!(game.calculated_power(matching),Some(3),"layered type change must affect the note filter");
    game.set_current_controller(source,bob).unwrap();
    assert_eq!(game.calculated_power(other),Some(2));
    let bob_elf = game.create_object_from_card(&creature("Bob's noted Elf",vec![Subtype::Elf]),bob,Zone::Battlefield);
    assert_eq!(game.calculated_power(bob_elf),Some(3),"new controller uses own notes");
    game.set_current_controller(source,alice).unwrap();
    let second = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    assert_eq!(game.calculated_power(source),Some(3),"another same-named source can boost this one");
    assert_eq!(game.calculated_power(second),Some(3));
    game.move_object_by_effect(second,Zone::Graveyard).unwrap();
    game.record_draft_noted_creature_types(alice,"Paliano Vanguard",Subtype::all_creature_types().iter().copied());
    assert_eq!(game.calculated_power(other),Some(3),"noting all creature types");
    assert_eq!(game.calculated_power(typeless),Some(2),"no type still cannot share a noted type");
    game.move_object_by_effect(source,Zone::Graveyard).unwrap();
    assert_eq!(game.calculated_power(other),Some(2),"bonus ends when source leaves");
}

#[test]
fn draft_notes_name_search_reveals_one_noted_card_and_shuffles_only_when_accepted() {
    struct Answers { accept: bool, fail: bool, legal: Vec<String> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.accept }
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.legal = context.candidates.iter().filter(|c|c.legal).map(|c|game.object(c.id).unwrap().name.to_string()).collect();
            if self.fail {vec![]} else {context.candidates.iter().filter(|c|c.legal).take(1).map(|c|c.id).collect()}
        }
    }
    let definition = compile_to_runtime_definition("Smuggler Captain", "Mana cost: {3}{B}\nType: Creature — Human Pirate\nPower/Toughness: 2/2\nDraft this card face up.\nAs you draft a card, you may reveal it, note its name, then turn this card face down.\nWhen this creature enters, you may search your library for a card with a name you noted for cards named Smuggler Captain, reveal it, put it into your hand, then shuffle.",false).unwrap();
    let ability = definition.abilities.iter().find_map(|a| match &a.kind {ironsmith::ability::AbilityKind::Triggered(a)=>Some(a), _=>None}).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["accept","decline","fail to find","no notes","source left"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        for name in ["First noted","Second noted","Other player's note","Other group's note","Unnoted"] {
            let card = CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Land]).build();
            game.create_object_from_card(&card,alice,Zone::Library);
            game.create_object_from_card(&card,bob,Zone::Library);
        }
        if case != "no notes" {
            game.record_draft_noted_name(alice,"SMUGGLER CAPTAIN","First noted");
            game.record_draft_noted_name(alice,"Smuggler Captain","Second noted");
        }
        game.record_draft_noted_name(bob,"Smuggler Captain","Other player's note");
        game.record_draft_noted_name(alice,"Other group","Other group's note");
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,source).unwrap();
        if case == "source left" {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
        let mut dm = Answers{accept:case != "decline",fail:case == "fail to find",legal:vec![]};
        let mut ctx = ironsmith::effects::EffectContext::new(source,alice,&mut dm);
        ctx.source_snapshot = Some(snapshot);
        let mut shuffles = 0; let mut reveals = 0; let mut searches = 0;
        for effect in ability.effects.all_effects() {
            let result = ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();
            shuffles += result.events.iter().filter(|e|e.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count();
            reveals += result.events.iter().filter(|e|e.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).count();
            searches += result.events.iter().filter(|e|e.downcast::<ironsmith::events::SearchLibraryEvent>().is_some()).count();
        }
        drop(ctx);
        let found = matches!(case,"accept"|"source left");
        assert_eq!(game.player(alice).unwrap().hand.len(),usize::from(found),"{case}: one card total");
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert_eq!(shuffles,usize::from(case != "decline"),"{case}");
        assert_eq!(searches,usize::from(case != "decline"),"{case}");
        assert_eq!(reveals,usize::from(found),"{case}");
        if case != "decline" && case != "no notes" {dm.legal.sort();assert_eq!(dm.legal,vec!["First noted","Second noted"],"only this player's named group");}
        if case == "no notes" {assert!(dm.legal.is_empty());}
    }
}

#[test]
fn sacrifice_combat_history_union_uses_current_opponent_and_conditional_count() {
    struct Picks { offered: Vec<String>, players: Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Picks {
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.players.push(context.player);
            self.offered.extend(context.candidates.iter().filter(|c|c.legal).map(|c|game.object(c.id).unwrap().name.to_string()));
            context.candidates.iter().filter(|c|c.legal).take(context.max.unwrap_or(context.candidates.len())).map(|c|c.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("Gideon's Triumph", "Mana cost: {1}{W}\nType: Instant\nTarget opponent sacrifices a creature of their choice that attacked or blocked this turn. If you control a Gideon planeswalker, that player sacrifices two of those creatures of their choice instead.",false).unwrap();
    let program = definition.spell_effect.as_ref().unwrap();
    assert_eq!(program.segments.len(),1);
    assert_eq!(program.segments[0].self_replacements.len(),1,"one replacement, not additive sacrifices");
    let choice = program.segments[0].default_effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()).unwrap();
    let replacement_choice = program.segments[0].self_replacements[0].replacement_effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()).unwrap();
    assert_eq!(replacement_choice.filter,choice.filter,"count replacement must preserve the full original selection domain");
    let compact = ironsmith_core::filter_model::describe_shared_combat_history_union(&choice.filter).unwrap();
    assert!(compact.contains("attacked or blocked this turn"),"{compact}");
    for bad in ["partial","extra","qualified"] {
        let mut filter = choice.filter.clone();
        match bad {
            "partial" => {filter.any_of.pop();}
            "extra" => {filter.any_of.push(ironsmith_core::ObjectFilter::default());}
            "qualified" => {filter.any_of[0].subtypes.push(Subtype::Elf);}
            _=>unreachable!(),
        }
        assert!(ironsmith_core::filter_model::describe_shared_combat_history_union(&filter).is_none(),"{bad}: different domains cannot be collapsed");
    }
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    for case in ["ordinary","own Gideon","opponent Gideon","not planeswalker","no eligible","one eligible","previous turn","new incarnation","Gideon left"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Stack);
        let creature = |name| CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build();
        let attacked = game.create_object_from_card(&creature("Attacked"),bob,Zone::Battlefield);
        let blocked = game.create_object_from_card(&creature("Blocked"),bob,Zone::Battlefield);
        let idle = game.create_object_from_card(&creature("Idle"),bob,Zone::Battlefield);
        let other_player = game.create_object_from_card(&creature("Other opponent"),carol,Zone::Battlefield);
        let own = game.create_object_from_card(&creature("Caster creature"),alice,Zone::Battlefield);
        if case != "no eligible" {
            for id in [attacked,other_player,own] {game.mark_creature_attacked_this_turn(id);}
            if case != "one eligible" {
                let event = ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::CreatureBlockedEvent::new(blocked,own),ironsmith::ProvNodeId::default());
                game.turn_store.turn_history.record_event(&event,None,None);
            }
        }
        if case == "previous turn" {game.turn_store.turn_history.clear_for_new_turn();}
        if case == "new incarnation" {
            let hand = game.move_object_by_effect(attacked,Zone::Hand).unwrap();
            game.move_object_by_effect(hand,Zone::Battlefield).unwrap();
        }
        if matches!(case,"own Gideon"|"opponent Gideon"|"not planeswalker"|"one eligible"|"Gideon left") {
            let gideon = CardBuilder::new(CardId::new(),"Planeswalker fixture").card_types(vec![if case == "not planeswalker" {CardType::Creature} else {CardType::Planeswalker}]).subtypes(vec![Subtype::Gideon]).build();
            let gideon = game.create_object_from_card(&gideon,if case == "opponent Gideon" {bob} else {alice},Zone::Battlefield);
            if case == "Gideon left" {game.move_object_by_effect(gideon,Zone::Graveyard).unwrap();}
        }
        let mut dm = Picks{offered:vec![],players:vec![]};
        game.push_to_stack(ironsmith::game_state::StackEntry::new(source,alice).with_targets(vec![ironsmith::game_state::Target::Player(bob)]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let count = if matches!(case,"no eligible"|"previous turn") {0} else if case == "own Gideon" {2} else {1};
        assert_eq!(game.player(bob).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|o|matches!(o.name.as_ref(),"Attacked"|"Blocked"|"Idle"))).count(),count,"{case}");
        assert!(game.object(idle).is_some(),"idle creature never eligible");
        assert!(game.object(other_player).is_some() && game.object(own).is_some(),"only targeted opponent sacrifices");
        assert!(dm.players.iter().all(|p|*p==bob),"sacrificing player chooses: {case}");
        assert!(!dm.offered.iter().any(|name|matches!(name.as_str(),"Idle"|"Other opponent"|"Caster creature")),"{case}: {:?}",dm.offered);
        if case == "ordinary" {dm.offered.sort();assert_eq!(dm.offered,vec!["Attacked","Blocked"],"either earlier combat role must qualify");}
    }
}

#[test]
fn wrecking_crew_x_entry_and_optional_modal_trigger_cover_every_mode_subset() {
    struct Answers { modes: Vec<usize>, bounds: Vec<(usize,usize)>, bob: PlayerId, token: ironsmith::ObjectId }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_options(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if context.description.to_lowercase().contains("mode") {
                self.bounds.push((context.min,context.max));
                assert_eq!(context.options.len(),4);
                assert!(context.options.iter().all(|o|o.legal));
                return self.modes.clone();
            }
            context.options.iter().filter(|o|o.legal).take(context.min).map(|o|o.index).collect()
        }
        fn decide_targets(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            context.requirements.iter().flat_map(|requirement| {
                let preferred = if requirement.legal_targets.contains(&ironsmith::game_state::Target::Object(self.token)) {ironsmith::game_state::Target::Object(self.token)} else {ironsmith::game_state::Target::Player(self.bob)};
                assert!(requirement.legal_targets.contains(&preferred));
                vec![preferred; requirement.min_targets]
            }).collect()
        }
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let mut legal = context.candidates.iter().filter(|c|c.legal).collect::<Vec<_>>();
            legal.sort_by_key(|c| usize::from(!game.object(c.id).unwrap().name.ends_with("fodder")));
            legal.into_iter().take(context.min).map(|c|c.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("The Ruinous Wrecking Crew", "Mana cost: {X}{B}{R}\nType: Legendary Creature — Human Villain\nPower/Toughness: 2/2\nThe Ruinous Wrecking Crew enters with X +1/+1 counters on it.\nWhen The Ruinous Wrecking Crew enters, choose up to X —\n• Discard a card, then draw a card.\n• Target opponent loses 2 life.\n• Destroy target token.\n• Each player sacrifices a creature of their choice.",false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    let creature = |name| CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build();
    for x in 0..=5u32 {
        for mask in 0..16u32 {
            if mask.count_ones() > x {continue;}
            let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
            let mut fodder = Vec::new();
            for (player,name) in [(alice,"Alice fodder"),(bob,"Bob fodder"),(carol,"Carol fodder")] {fodder.push(game.create_object_from_card(&creature(name),player,Zone::Battlefield));}
            let token = CardBuilder::new(CardId::new(),"Token fixture").card_types(vec![CardType::Artifact]).build();
            let token = game.create_object_from_card(&token,bob,Zone::Battlefield);
            game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
            let discarded = CardBuilder::new(CardId::new(),"Discard fixture").card_types(vec![CardType::Land]).build();
            let drawn = CardBuilder::new(CardId::new(),"Draw fixture").card_types(vec![CardType::Land]).build();
            game.create_object_from_card(&discarded,alice,Zone::Hand);
            game.create_object_from_card(&drawn,alice,Zone::Library);
            let spell = game.create_object_from_definition(&definition,alice,Zone::Stack);
            game.object_mut(spell).unwrap().x_value = Some(x);
            game.push_to_stack(ironsmith::game_state::StackEntry::new(spell,alice).with_x(x));
            let modes = (0..4usize).filter(|mode| mask & (1 << mode) != 0).collect::<Vec<_>>();
            let mut dm = Answers{modes:modes.clone(),bounds:vec![],bob,token};
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            let source = *game.battlefield.iter().find(|id|game.object(**id).unwrap().name == "The Ruinous Wrecking Crew").unwrap();
            assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),x,"X={x}, modes={mask:04b}: entry counters");
            assert_eq!(game.calculated_power(source),Some(2+x as i32));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
            assert_eq!(queue.entries.len(),1,"exactly one ETB trigger");
            assert_eq!(queue.entries[0].x_value,Some(x),"cast X must reach trigger choices");
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
            if mask == 0 {
                assert!(game.stack.is_empty(),"X={x}: CR 603.3c removes a modal trigger with no chosen modes");
            } else {
                assert_eq!(game.stack.len(),1,"X={x}, modes={mask:04b}");
                assert_eq!(game.stack[0].chosen_modes,Some(modes));
                ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            }
            assert!(dm.bounds.iter().all(|(min,max)|*min==0 && *max<=x as usize && *max>=mask.count_ones() as usize),"X={x}, bounds={:?}",dm.bounds);
            if x > 0 {assert_eq!(dm.bounds.len(),1,"modes are selected as the trigger goes onto the stack");}
            let selected = |mode: usize| -> bool { mask & (1u32 << mode) != 0u32 };
            assert_eq!(game.player(bob).unwrap().life,if selected(1) {18} else {20});
            assert_eq!(game.player(alice).unwrap().life,20); assert_eq!(game.player(carol).unwrap().life,20);
            assert_eq!(game.object(token).is_none_or(|o|o.zone!=Zone::Battlefield),selected(2));
            for id in fodder {assert_eq!(game.object(id).is_none_or(|o|o.zone!=Zone::Battlefield),selected(3),"each player sacrifices their chosen creature");}
            let hand_names = game.player(alice).unwrap().hand.iter().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
            assert_eq!(hand_names,vec![if selected(0) {"Draw fixture"} else {"Discard fixture"}],"discard then draw mode");
            assert!(game.object(source).is_some(),"sacrifice choice can preserve the source");
            if x == 3 && mask == 0 {
                let exile = game.move_object_by_effect(source,Zone::Exile).unwrap();
                let returned = game.move_object_by_effect(exile,Zone::Battlefield).unwrap();
                assert_eq!(game.counter_count(returned,ironsmith::object::CounterType::PlusOnePlusOne),0,"new noncast entry does not reuse cast X");
            }
        }
    }
}

#[test]
fn kicked_counter_entries_grant_recipient_owned_damage_triggers_with_exact_paid_costs() {
    let cards = [
        ("Necravolver", "Mana cost: {2}{B}\nType: Creature — Volver\nPower/Toughness: 2/2\nKicker {1}{G} and/or {W} (You may pay an additional {1}{G} and/or {W} as you cast this spell.)\nIf this creature was kicked with its {1}{G} kicker, it enters with two +1/+1 counters on it and with trample.\nIf this creature was kicked with its {W} kicker, it enters with a +1/+1 counter on it and with \"Whenever this creature deals damage, you gain that much life.\"", 1usize, ironsmith::static_abilities::StaticAbilityId::Trample),
        ("Rakavolver", "Mana cost: {2}{R}\nType: Creature — Volver\nPower/Toughness: 2/2\nKicker {1}{W} and/or {U} (You may pay an additional {1}{W} and/or {U} as you cast this spell.)\nIf this creature was kicked with its {1}{W} kicker, it enters with two +1/+1 counters on it and with \"Whenever this creature deals damage, you gain that much life.\"\nIf this creature was kicked with its {U} kicker, it enters with a +1/+1 counter on it and with flying.", 0usize, ironsmith::static_abilities::StaticAbilityId::Flying),
    ];
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    for (name,text,life_index,keyword) in cards {
        let definition = compile_to_runtime_definition(name,text,false).unwrap();
        assert!(definition.spell_effect.is_none(),"entry replacements remain entry rules");
        assert_eq!(definition.optional_costs.len(),2,"independent kicker choices");
        for mask in 0..4usize {
            for case in ["noncombat","combat","control change","source leaves","blink"] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
                let spell = game.create_object_from_definition(&definition,alice,Zone::Stack);
                let mut paid = ironsmith::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
                for index in 0..2 {if mask & (1usize << index) != 0 {paid.pay(index);}}
                game.object_mut(spell).unwrap().optional_costs_paid = paid.clone();
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell,alice).with_optional_costs_paid(paid));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let mut source = *game.battlefield.iter().find(|id|game.object(**id).unwrap().name == name).unwrap();
                let count = 2 * u32::from(mask & 1 != 0) + u32::from(mask & 2 != 0);
                assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),count,"{name}, mask={mask}, {case}");
                assert_eq!(game.calculated_power(source),Some(2+count as i32));
                assert_eq!(game.current_has_static_ability_id(source,keyword),mask & (1usize << (1-life_index)) != 0);
                assert!(!game.current_has_static_ability_id(source,ironsmith::static_abilities::StaticAbilityId::Lifelink),"life gain is triggered, not immediate lifelink");
                if case == "blink" {
                    let exile = game.move_object_by_effect(source,Zone::Exile).unwrap();
                    source = game.move_object_by_effect(exile,Zone::Battlefield).unwrap();
                    assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),0,"new entry has no kicker payments");
                    assert!(!game.current_has_static_ability_id(source,keyword));
                }
                if case == "control change" {game.set_current_controller(source,bob).unwrap();}
                let controller = if case == "control change" {bob} else {alice};
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
                assert!(queue.entries.is_empty(),"entry itself is not a damage trigger");
                let amount = if case == "combat" {game.calculated_power(source).unwrap()} else {3};
                let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(amount,ChooseSpec::SpecificPlayer(carol)).with_combat(case == "combat"));
                game.push_to_stack(ironsmith::game_state::StackEntry::ability(source,controller,vec![damage]));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(carol).unwrap().life,20-amount);
                assert_eq!(game.player(alice).unwrap().life,20,"gain waits for the trigger to resolve");
                assert_eq!(game.player(bob).unwrap().life,20);
                ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
                let triggers = case != "blink" && mask & (1usize << life_index) != 0;
                assert_eq!(queue.entries.len(),usize::from(triggers),"{name}, mask={mask}, {case}: only exact life kicker grants the trigger");
                if case == "source leaves" {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
                ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
                while !game.stack.is_empty() {ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();}
                for player in [alice,bob] {assert_eq!(game.player(player).unwrap().life,20+if triggers && player==controller {amount} else {0},"{name}, mask={mask}, {case}: controller at damage gets the actual amount");}
            }
        }
    }
}

#[test]
fn quoted_recipients_phoenix_token_targets_named_cards_and_preserves_resolution_dependencies() {
    struct Targets { preferred: ironsmith::ObjectId, seen: Vec<ironsmith::ObjectId> }
    impl ironsmith::decision::DecisionMaker for Targets {
        fn decide_targets(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            assert_eq!(context.requirements.len(),1);
            let requirement = &context.requirements[0];
            self.seen = requirement.legal_targets.iter().filter_map(|t| match t {ironsmith::game_state::Target::Object(id)=>Some(*id),_=>None}).collect();
            assert!(requirement.legal_targets.contains(&ironsmith::game_state::Target::Object(self.preferred)));
            vec![ironsmith::game_state::Target::Object(self.preferred)]
        }
    }
    let definition = compile_to_runtime_definition("Rekindling Phoenix", "Mana cost: {2}{R}{R}\nType: Creature — Phoenix\nPower/Toughness: 4/3\nFlying\nWhen this creature dies, create a 0/1 red Elemental creature token with \"At the beginning of your upkeep, sacrifice this token and return target card named Rekindling Phoenix from your graveyard to the battlefield. It gains haste until end of turn.\"",false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["normal","new controller","target gone","token left"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let phoenix = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        assert!(game.current_has_static_ability_id(phoenix,ironsmith::static_abilities::StaticAbilityId::Flying));
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(phoenix,alice,vec![Effect::destroy(ChooseSpec::SpecificObject(phoenix))]));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
        assert_eq!(queue.entries.len(),1,"death trigger");
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let token = *game.battlefield.iter().find(|id|game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token).unwrap();
        assert_eq!(game.calculated_power(token),Some(0)); assert_eq!(game.calculated_toughness(token),Some(1));
        assert!(game.current_subtypes(token).unwrap().contains(&Subtype::Elemental));
        assert_eq!(game.current_colors(token),Some(ironsmith::color::ColorSet::RED));
        assert!(!game.current_has_static_ability_id(token,ironsmith::static_abilities::StaticAbilityId::Flying));
        let controller = if case == "new controller" {bob} else {alice};
        if controller == bob {game.set_current_controller(token,bob).unwrap();}
        let first = game.create_object_from_definition(&definition,controller,Zone::Graveyard);
        let preferred = game.create_object_from_definition(&definition,controller,Zone::Graveyard);
        let foreign = game.create_object_from_definition(&definition,if controller==alice {bob} else {alice},Zone::Graveyard);
        let unrelated = CardBuilder::new(CardId::new(),"Unrelated creature").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1,1)).build();
        let unrelated = game.create_object_from_card(&unrelated,controller,Zone::Graveyard);
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(controller),ironsmith::ProvNodeId::default());
        for trigger in ironsmith::triggers::check_triggers(&game,&event) {queue.add(trigger);}
        assert_eq!(queue.entries.len(),1,"token owns the upkeep trigger");
        let mut dm = Targets{preferred,seen:vec![]};
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        assert_eq!(game.stack.len(),1,"{case}: a named graveyard target must be legal");
        assert!(dm.seen.contains(&first) && dm.seen.contains(&preferred),"any copy with that name");
        assert!(!dm.seen.contains(&foreign) && !dm.seen.contains(&unrelated),"only your named cards");
        if case == "target gone" {game.move_object_by_effect(preferred,Zone::Exile).unwrap();}
        if case == "token left" {game.move_object_by_effect(token,Zone::Exile).unwrap();}
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        if case == "target gone" {assert!(game.object(token).is_some_and(|o|o.zone==Zone::Battlefield),"illegal only target stops the whole ability before sacrifice");}
        else {
            assert!(game.object(token).is_none_or(|o|o.zone!=Zone::Battlefield));
            let returned = *game.battlefield.iter().find(|id|game.object(**id).unwrap().name == "Rekindling Phoenix").unwrap();
            assert_eq!(game.controller_of_id(returned),Some(controller));
            assert!(game.current_has_static_ability_id(returned,ironsmith::static_abilities::StaticAbilityId::Haste));
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state().unwrap();
            assert!(!game.current_has_static_ability_id(returned,ironsmith::static_abilities::StaticAbilityId::Haste));
        }
    }
}

#[test]
fn quoted_recipients_sparkshaper_animation_grants_owned_combat_scry_and_expires() {
    struct Answers { selected: Vec<ironsmith::ObjectId>, viewers: Vec<PlayerId>, targets: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            self.targets += 1;
            assert_eq!(ctx.requirements.len(),1);
            let requirement = &ctx.requirements[0];
            assert_eq!(requirement.min_targets,0);
            assert!(self.selected.iter().all(|id|requirement.legal_targets.contains(&ironsmith::game_state::Target::Object(*id))));
            self.selected.iter().map(|id|ironsmith::game_state::Target::Object(*id)).collect()
        }
        fn view_cards(&mut self, _: &ironsmith::GameState, player: PlayerId, _: &[ironsmith::ObjectId], _: &ironsmith::decisions::context::ViewCardsContext) {self.viewers.push(player);}
    }
    let definition = compile_to_runtime_definition("Sparkshaper Visionary", "Mana cost: {2}{U}\nType: Creature — Human Wizard\nPower/Toughness: 0/5\nAt the beginning of combat on your turn, choose any number of target planeswalkers you control. Until end of turn, they become 3/3 blue Bird creatures with flying, hexproof, and \"Whenever this creature deals combat damage to a player, scry 1.\" (They're no longer planeswalkers. Loyalty abilities can still be activated.)",false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    let mut loyalty = ironsmith::ability::Ability::activated_with_timing(
        ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::remove_counters(ironsmith::object::CounterType::Loyalty,1)),
        vec![Effect::draw(1)],ironsmith::ability::ActivationTiming::SorcerySpeed,
    );
    let ironsmith::ability::AbilityKind::Activated(activated) = &mut loyalty.kind else {unreachable!()}; activated.is_loyalty_ability = true;
    let walker = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Walker fixture").card_types(vec![CardType::Planeswalker]).with_ability(loyalty).build();
    for case in ["normal","source leaves","recipient control","parent damage","noncombat","end turn","zero targets"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let first = game.create_object_from_definition(&walker,alice,Zone::Battlefield);
        let second = game.create_object_from_definition(&walker,alice,Zone::Battlefield);
        let foreign = game.create_object_from_definition(&walker,bob,Zone::Battlefield);
        for id in [first,second,foreign] {game.add_counters(id,ironsmith::object::CounterType::Loyalty,4);}
        let library = CardBuilder::new(CardId::new(),"Scry fixture").card_types(vec![CardType::Land]).build();
        for player in [alice,bob] {game.create_object_from_card(&library,player,Zone::Library);}
        let event = |player| ironsmith::triggers::TriggerEvent::new(ironsmith::events::phase::BeginningOfCombatEvent::new(player),ironsmith::ProvNodeId::default());
        assert!(ironsmith::triggers::check_triggers(&game,&event(bob)).is_empty(),"your turn");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for trigger in ironsmith::triggers::check_triggers(&game,&event(alice)) {queue.add(trigger);}
        assert_eq!(queue.entries.len(),1);
        let selected = if case == "zero targets" {vec![]} else {vec![first,second]};
        let mut dm = Answers{selected:selected.clone(),viewers:vec![],targets:0};
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        assert_eq!(dm.targets,1,"targets are announced with the trigger");
        assert_eq!(game.stack[0].targets,selected.iter().map(|id|ironsmith::game_state::Target::Object(*id)).collect::<Vec<_>>());
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        assert_eq!(game.current_card_types(foreign),Some(vec![CardType::Planeswalker]));
        assert_eq!(game.calculated_power(source),Some(0),"grant does not animate its source");
        for id in selected {
            assert_eq!(game.current_card_types(id),Some(vec![CardType::Creature]));
            assert_eq!(game.calculated_power(id),Some(3)); assert_eq!(game.calculated_toughness(id),Some(3));
            assert_eq!(game.current_colors(id),Some(ironsmith::color::ColorSet::BLUE));
            assert!(game.current_subtypes(id).unwrap().contains(&Subtype::Bird));
            for keyword in [ironsmith::static_abilities::StaticAbilityId::Flying,ironsmith::static_abilities::StaticAbilityId::Hexproof] {assert!(game.current_has_static_ability_id(id,keyword));}
            assert_eq!(game.counter_count(id,ironsmith::object::CounterType::Loyalty),4);
            assert!(game.current_abilities(id).unwrap().iter().any(|a|matches!(&a.kind,ironsmith::ability::AbilityKind::Activated(a) if a.is_loyalty_ability())));
        }
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        assert!(ironsmith::decision::compute_legal_actions(&game,alice).unwrap().iter().any(|a|matches!(a,ironsmith::decision::LegalAction::ActivateAbility{source,..} if *source==first)),"loyalty activation remains legal on a creature");
        if case == "source leaves" {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
        if case == "recipient control" {game.set_current_controller(first,bob).unwrap();}
        if case == "end turn" {game.effect_store.continuous_effects.cleanup_end_of_turn();game.refresh_continuous_state().unwrap();assert_eq!(game.current_card_types(first),Some(vec![CardType::Planeswalker]));}
        if case == "zero targets" {assert_eq!(game.current_card_types(first),Some(vec![CardType::Planeswalker]));continue;}
        let damager = if case == "parent damage" {source} else {first};
        let controller = if case == "recipient control" {bob} else {alice};
        let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(3,ChooseSpec::SpecificPlayer(carol)).with_combat(case != "noncombat"));
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(damager,controller,vec![damage]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
        let triggers = !matches!(case,"parent damage"|"noncombat"|"end turn");
        assert_eq!(queue.entries.len(),usize::from(triggers),"{case}: only the recipient's combat damage");
        if triggers {assert_eq!(queue.entries[0].controller,controller);}
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
        while !game.stack.is_empty() {ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        assert_eq!(dm.viewers,if triggers {vec![controller]} else {vec![]},"{case}: recipient controller scries");
    }
}

#[test]
fn attack_damage_swordjack_counts_live_artifacts_and_keeps_the_attacked_recipient() {
    use ironsmith::events::combat::{AttackEventTarget, CreatureAttackedEvent};
    let definition = compile_to_runtime_definition("Fathom Fleet Swordjack", "Mana cost: {3}{R}\nType: Creature — Orc Pirate\nPower/Toughness: 4/3\nWhenever this creature attacks, it deals damage to the player or planeswalker it's attacking equal to the number of artifacts you control.\nEncore {5}{R} ({5}{R}, Exile this card from your graveyard: For each opponent, create a token copy that attacks that opponent this turn if able. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery.)", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    for count in [0, 1, 3] {
        for case in ["player", "walker", "battle", "source leaves", "artifact changes", "walker leaves"] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let artifact = CardBuilder::new(CardId::new(), "Count fixture").card_types(vec![CardType::Artifact]).build();
            for _ in 0..count { game.create_object_from_card(&artifact, alice, Zone::Battlefield); }
            for _ in 0..4 { game.create_object_from_card(&artifact, carol, Zone::Battlefield); }
            game.create_object_from_card(&artifact, alice, Zone::Hand);
            let walker = CardBuilder::new(CardId::new(), "Attacked fixture").card_types(vec![CardType::Planeswalker]).build();
            let walker = game.create_object_from_card(&walker, bob, Zone::Battlefield);
            game.add_counters(walker, ironsmith::object::CounterType::Loyalty, 10);
            let battle = CardBuilder::new(CardId::new(), "Battle fixture").card_types(vec![CardType::Battle]).build();
            let battle = game.create_object_from_card(&battle, bob, Zone::Battlefield);
            let attacked = match case {
                "walker" | "walker leaves" => AttackEventTarget::Planeswalker(walker),
                "battle" => AttackEventTarget::Battle(battle),
                _ => AttackEventTarget::Player(bob),
            };
            let event = ironsmith::triggers::TriggerEvent::new(CreatureAttackedEvent::new(source, attacked), ironsmith::ProvNodeId::default());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
            assert_eq!(queue.entries.len(), 1);
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert!(game.stack[0].targets.is_empty(), "attacked recipient is not a target choice");
            if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            if case == "artifact changes" { game.create_object_from_card(&artifact, alice, Zone::Battlefield); }
            if case == "walker leaves" { game.move_object_by_effect(walker, Zone::Graveyard).unwrap(); }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let amount = count + i32::from(case == "artifact changes");
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(carol).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, 20 - if matches!(case, "player" | "source leaves" | "artifact changes") { amount } else { 0 }, "{case}, {count}");
            if case != "walker leaves" { assert_eq!(game.counter_count(walker, ironsmith::object::CounterType::Loyalty), (10 - if case == "walker" { amount } else { 0 }) as u32, "{case}, {count}"); }
        }
    }
}

#[test]
fn attack_damage_ian_announces_one_target_and_checks_modified_at_both_times() {
    struct Answers { target: ironsmith::game_state::Target, accept: bool, declarations: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            self.declarations += 1;
            assert_eq!(ctx.requirements.len(), 1);
            assert!(ctx.requirements[0].legal_targets.contains(&self.target));
            vec![self.target]
        }
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.accept }
    }
    let definition = compile_to_runtime_definition("Ian the Reckless", "Mana cost: {1}{R}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 2/1\nWhenever Ian the Reckless attacks, if it's modified, you may have it deal damage equal to its power to you and any target. (Equipment, Auras you control, and counters are modifications.)", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["normal", "unmodified", "removed counters", "power changes", "decline", "self player", "creature", "target gone"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if case != "unmodified" { game.add_counters(source, ironsmith::object::CounterType::PlusOnePlusOne, 1); }
        let card = CardBuilder::new(CardId::new(), "Damage recipient").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(0, 10)).build();
        let recipient = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let target = match case {
            "self player" => ironsmith::game_state::Target::Player(alice),
            "creature" | "target gone" => ironsmith::game_state::Target::Object(recipient),
            _ => ironsmith::game_state::Target::Player(bob),
        };
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::combat::CreatureAttackedEvent::new(source, ironsmith::events::combat::AttackEventTarget::Player(bob)), ironsmith::ProvNodeId::default());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
        assert_eq!(queue.entries.len(), usize::from(case != "unmodified"), "intervening if at trigger time");
        if case == "unmodified" { continue; }
        let mut dm = Answers { target, accept: case != "decline", declarations: 0 };
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(dm.declarations, 1, "announce the explicit target before the may decision");
        assert_eq!(game.stack[0].targets, vec![target]);
        if case == "removed counters" { game.remove_counters(source, ironsmith::object::CounterType::PlusOnePlusOne, 1, Some(source), Some(alice)).unwrap(); }
        if case == "power changes" { game.add_counters(source, ironsmith::object::CounterType::PlusOnePlusOne, 2); }
        if case == "target gone" { game.move_object_by_effect(recipient, Zone::Exile).unwrap(); }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let amount = if matches!(case, "removed counters" | "decline" | "target gone") { 0 } else if case == "power changes" { 5 } else { 3 };
        assert_eq!(game.player(alice).unwrap().life, 20 - amount * if case == "self player" { 2 } else { 1 }, "{case}: fixed recipient and explicit recipient share the amount");
        assert_eq!(game.player(bob).unwrap().life, 20 - if matches!(case, "normal" | "power changes") { amount } else { 0 }, "{case}");
        if case == "creature" { assert_eq!(game.damage_on(recipient), amount as u32); }
    }
}

#[test]
fn draft_number_references_read_the_controllers_named_group_and_recheck_targets() {
    let fire = compile_to_runtime_definition("Garbage Fire", "Mana cost: {2}{R}\nType: Instant\nReveal this card as you draft it and note how many cards you've drafted this draft round, including this card.\nGarbage Fire deals damage to target creature equal to the highest number you noted for cards named Garbage Fire.", false).unwrap();
    let keeper = compile_to_runtime_definition("Custodi Peacekeeper", "Mana cost: {2}{W}\nType: Creature — Human Cleric\nPower/Toughness: 2/3\nReveal this card as you draft it and note how many cards you've drafted this draft round, including this card.\n{W}, {T}: Tap target creature with power less than or equal to the highest number you noted for cards named Custodi Peacekeeper.", false).unwrap();
    let activation = keeper.abilities.iter().find_map(|a| match &a.kind { ironsmith::ability::AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
    assert!(ironsmith_text::compiled_text::compiled_text_lines(&keeper).join(" ").contains("highest number you noted for cards named"));
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for (definition, is_fire) in [(&fire, true), (&keeper, false)] {
        for case in ["normal", "no notes", "notes change", "target changes", "different controller"] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let name = if is_fire { "Garbage Fire" } else { "Custodi Peacekeeper" };
            if case != "no notes" { game.set_draft_noted_highest_number(alice, name.to_uppercase(), 3); }
            game.set_draft_noted_highest_number(bob, name, 1);
            game.set_draft_noted_highest_number(alice, "Other draft group", 50);
            let source = game.create_object_from_definition(definition, alice, if is_fire { Zone::Stack } else { Zone::Battlefield });
            let controller = if case == "different controller" { bob } else { alice };
            if controller == bob { game.set_current_controller(source, bob).unwrap(); }
            let note = if controller == bob { 1 } else if case == "no notes" { 0 } else { 3 };
            let mut fixtures = Vec::new();
            for power in [-1, 0, 1, 3, 4, 7] {
                let card = CardBuilder::new(CardId::new(), format!("Ordinary creature {power}")).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(power, 20)).build();
                let id = game.create_object_from_card(&card, bob, Zone::Battlefield);
                fixtures.push((power, id));
            }
            let program = if is_fire { definition.spell_effect.as_ref().unwrap() } else { &activation.effects };
            let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(&game, program, controller, Some(source), None);
            assert_eq!(requirements.len(), 1, "{name}, {case}: {program:#?}");
            for (power, id) in &fixtures {
                assert_eq!(requirements[0].legal_targets.contains(&ironsmith::game_state::Target::Object(*id)), is_fire || *power <= note, "{name}, {case}, power={power}: the draft group is not the target's name");
            }
            let recipient = fixtures.iter().find(|(power, _)| *power == note).unwrap().1;
            let entry = if is_fire { ironsmith::game_state::StackEntry::new(source, controller) } else { ironsmith::game_state::StackEntry::ability(source, controller, program.clone()) };
            game.push_to_stack(entry.with_targets(vec![ironsmith::game_state::Target::Object(recipient)]));
            if case == "notes change" { game.set_draft_noted_highest_number(controller, name, 7); }
            if case == "target changes" {
                if is_fire { game.object_mut(recipient).unwrap().card_types = vec![CardType::Land].into(); }
                else { game.add_counters(recipient, ironsmith::object::CounterType::PlusOnePlusOne, 4); }
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            if is_fire {
                let amount = if case == "target changes" { 0 } else if case == "notes change" { 7 } else { note };
                assert_eq!(game.damage_on(recipient), amount as u32, "{name}, {case}: use the ability controller's own named history");
            } else {
                assert_eq!(game.is_tapped(recipient), case != "target changes", "{case}: target legality is checked again at resolution");
                if case == "notes change" {
                    let fresh = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(&game, program, controller, Some(source), None);
                    assert!(fixtures.iter().all(|(_, id)| fresh[0].legal_targets.contains(&ironsmith::game_state::Target::Object(*id))));
                }
            }
        }
    }
}

#[test]
fn multikicker_damage_batroc_announces_up_to_kick_count_and_deals_current_power_to_each() {
    struct Answers { selected: Vec<ironsmith::game_state::Target>, budget: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            assert_eq!(ctx.requirements.len(), 1);
            assert_eq!(ctx.requirements[0].min_targets, 0);
            assert_eq!(ctx.requirements[0].max_targets, Some(self.budget));
            assert!(self.selected.iter().all(|target| ctx.requirements[0].legal_targets.contains(target)));
            self.selected.clone()
        }
    }
    let definition = compile_to_runtime_definition("Batroc the Leaper", "Mana cost: {1}{R}\nType: Legendary Creature — Human Villain\nPower/Toughness: 2/2\nMultikicker {2} (You may pay an additional {2} any number of times as you cast this spell.)\nBatroc enters with a +1/+1 counter on him for each time he was kicked.\nWhen Batroc enters, he deals damage equal to his power to each of up to X targets, where X is the number of times he was kicked.", false).unwrap();
    assert_eq!(definition.optional_costs.len(), 1);
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    for kicks in 0..=3u32 {
        for chosen in 0..=kicks as usize {
            for case in ["normal", "power changes", "source leaves", "target gone"] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let card = CardBuilder::new(CardId::new(), "Damage fixture").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(0, 20)).build();
                let recipient = game.create_object_from_card(&card, bob, Zone::Battlefield);
                let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
                let mut paid = ironsmith::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
                paid.pay_times(0, kicks);
                assert_eq!(paid.kick_count(), kicks);
                game.object_mut(spell).unwrap().optional_costs_paid = paid.clone();
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice).with_optional_costs_paid(paid));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let source = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Batroc the Leaper").unwrap();
                assert_eq!(game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne), kicks);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
                assert_eq!(queue.entries.len(), 1);
                let options = [ironsmith::game_state::Target::Object(recipient), ironsmith::game_state::Target::Player(bob), ironsmith::game_state::Target::Player(carol)];
                let mut dm = Answers { selected: options[..chosen].to_vec(), budget: kicks as usize };
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                assert_eq!(game.stack[0].targets, dm.selected, "targets are announced when the trigger is stacked");
                if case == "power changes" { game.add_counters(source, ironsmith::object::CounterType::PlusOnePlusOne, 2); }
                if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                if case == "target gone" { game.move_object_by_effect(recipient, Zone::Exile).unwrap(); }
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let amount = 2 + kicks as i32 + if case == "power changes" { 2 } else { 0 };
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(bob).unwrap().life, 20 - if chosen >= 2 { amount } else { 0 }, "kicks={kicks}, chosen={chosen}, {case}");
                assert_eq!(game.player(carol).unwrap().life, 20 - if chosen >= 3 { amount } else { 0 }, "kicks={kicks}, chosen={chosen}, {case}");
                if case != "target gone" { assert_eq!(game.damage_on(recipient), if chosen >= 1 { amount as u32 } else { 0 }, "kicks={kicks}, chosen={chosen}, {case}: each recipient gets the full power"); }
                if kicks == 3 && chosen == 0 && case == "normal" {
                    let exile = game.move_object_by_effect(source, Zone::Exile).unwrap();
                    let returned = game.move_object_by_effect(exile, Zone::Battlefield).unwrap();
                    assert_eq!(game.counter_count(returned, ironsmith::object::CounterType::PlusOnePlusOne), 0);
                    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
                    assert_eq!(queue.entries.len(), 1);
                    dm.budget = 0;
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                    assert!(game.stack[0].targets.is_empty(), "a fresh noncast entry cannot reuse kicker payments");
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                }
            }
        }
    }
}

#[test]
fn dynamic_support_crowd_uses_cast_x_assist_payments_and_snapshot_trample_recipients() {
    use ironsmith::decision::{DecisionMaker, LegalAction};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    use ironsmith::mana::ManaSymbol;
    struct Answers { x: u32, help: u32, selected: Vec<ironsmith::game_state::Target> }
    impl DecisionMaker for Answers {
        fn decide_number(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::NumberContext) -> u32 {
            assert_eq!(ctx.player, PlayerId::from_index(0), "the caster chooses X");
            assert!(ctx.is_x_value);
            self.x
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            let index = if ctx.description.starts_with("Choose another player to assist") {
                assert_eq!(ctx.player, PlayerId::from_index(0));
                usize::from(self.help > 0)
            } else if ctx.description.starts_with("Choose how much generic mana") {
                assert_eq!(ctx.player, PlayerId::from_index(1));
                self.help as usize
            } else if ctx.description.starts_with("Confirm mana payment for") { 1 }
            else { return ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default(); };
            assert!(ctx.options.iter().any(|option| option.index == index && option.legal), "{}: index={index}", ctx.description);
            vec![index]
        }
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            assert_eq!(ctx.requirements.len(), 1);
            assert_eq!(ctx.requirements[0].min_targets, 0);
            assert_eq!(ctx.requirements[0].max_targets, Some(self.x as usize));
            assert!(self.selected.iter().all(|target| ctx.requirements[0].legal_targets.contains(target)));
            self.selected.clone()
        }
    }
    let (artifact, direct) = super::compile_to_artifact("The Crowd Goes Wild", "Mana cost: {X}{G}\nType: Sorcery\nAssist (Another player can pay up to {X} of this spell's cost. You choose the value of X.)\nSupport X. (Put a +1/+1 counter on each of up to X target creatures.)\nEach creature with a +1/+1 counter on it gains trample until end of turn.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        for x in 0..=3u32 {
            for help in 0..=x {
                for chosen in 0..=x as usize {
                    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    game.turn.active_player = alice;
                    game.turn.priority_player = Some(alice);
                    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Green, 1);
                    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, x - help);
                    game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Colorless, help);
                    let fixture = CardBuilder::new(CardId::new(), "Support recipient").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 4)).build();
                    let mut creatures = Vec::new();
                    for (player, counters) in [(alice, 0), (bob, 0), (alice, 1), (bob, 1), (bob, 0)] {
                        let id = game.create_object_from_card(&fixture, player, Zone::Battlefield);
                        if counters > 0 { game.add_counters(id, ironsmith::object::CounterType::PlusOnePlusOne, counters); }
                        creatures.push((id, counters));
                    }
                    let other = CardBuilder::new(CardId::new(), "Noncreature counter recipient").card_types(vec![CardType::Artifact]).build();
                    let other = game.create_object_from_card(&other, alice, Zone::Battlefield);
                    game.add_counters(other, ironsmith::object::CounterType::PlusOnePlusOne, 1);
                    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
                    let stable = game.object(hand).unwrap().stable_id;
                    assert!(game.current_has_static_ability_id(hand, ironsmith::static_abilities::StaticAbilityId::Assist));
                    let action = LegalAction::CastSpell { spell_id: hand, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal };
                    assert!(ironsmith::decision::compute_legal_actions(&game, alice).unwrap().contains(&action), "Assist makes the announced generic cost payable");
                    let mut dm = Answers { x, help, selected: creatures[..chosen].iter().map(|(id, _)| ironsmith::game_state::Target::Object(*id)).collect() };
                    let mut state = PriorityLoopState::new(game.players.len());
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                    for _ in 0..60 {
                        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
                        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending cast without decision"); };
                        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
                    }
                    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
                    let cast = game.find_object_by_stable_id(stable).unwrap();
                    assert_eq!(game.object(cast).unwrap().x_value, Some(x));
                    assert_eq!(game.object(cast).unwrap().mana_spent_to_cast.total(), x + 1);
                    assert_eq!(game.object(cast).unwrap().caster_mana_spent_to_cast, Some(x - help + 1));
                    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                    assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
                    assert_eq!(game.stack[0].targets, dm.selected);
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    for (index, (id, before)) in creatures.iter().enumerate() {
                        let count = before + u32::from(index < chosen);
                        assert_eq!(game.counter_count(*id, ironsmith::object::CounterType::PlusOnePlusOne), count);
                        assert_eq!(game.current_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Trample), count > 0, "x={x}, help={help}, chosen={chosen}, index={index}: includes opponents' creatures with existing counters");
                    }
                    assert!(!game.current_has_static_ability_id(other, ironsmith::static_abilities::StaticAbilityId::Trample));
                    let late = creatures[4].0;
                    game.add_counters(late, ironsmith::object::CounterType::PlusOnePlusOne, 1);
                    assert!(!game.current_has_static_ability_id(late, ironsmith::static_abilities::StaticAbilityId::Trample), "the resolving grant samples recipients once");
                    game.effect_store.continuous_effects.cleanup_end_of_turn();
                    game.refresh_continuous_state().unwrap();
                    assert!(creatures.iter().all(|(id, _)| !game.current_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Trample)));
                }
            }
        }
    }
}

#[test]
fn dynamic_support_legacy_payload_preserves_fixed_target_metadata() {
    let legacy: ironsmith_core::SupportEffect = serde_json::from_str(r#"{"amount":2}"#).unwrap();
    assert!(legacy.amount_value.is_none());
    assert_eq!(serde_json::to_value(&legacy).unwrap(), serde_json::json!({"amount":2}));
    let native = ironsmith::effects::SupportEffect::with_value(legacy.value());
    assert_eq!(ironsmith::effects::EffectExecutor::get_target_count(&native), Some(ironsmith::effect::ChoiceCount::up_to(2)));
    let ironsmith::target::ChooseSpec::Target(inner) = &native.target else { panic!("fixed target declaration"); };
    let ironsmith::target::ChooseSpec::Object(filter) = inner.as_ref() else { panic!("creature filter"); };
    assert!(filter.other);
    assert_eq!(filter.card_types, [CardType::Creature]);
}

#[test]
fn counter_kinds_animation_module_selects_one_existing_kind_on_objects_or_players() {
    use ironsmith::game_loop::*;
    use ironsmith::object::CounterType;
    struct Answers { target: ironsmith::game_state::Target, kind: CounterType, choices: usize, kinds: usize, accept: bool }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            assert_eq!(ctx.requirements.len(), 1);
            assert!(ctx.requirements[0].legal_targets.contains(&self.target));
            if matches!(self.target, ironsmith::game_state::Target::Player(_)) { assert!(ctx.requirements[0].description.contains("player")); }
            vec![self.target]
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.description == "Choose a counter kind" {
                self.choices += 1;
                assert_eq!(ctx.options.len(), self.kinds, "choose only among the target's existing kinds");
                let label = format!("{:?}", self.kind).to_ascii_lowercase();
                return vec![ctx.options.iter().find(|o| o.description.contains(&label)).expect("only a present counter kind can be chosen").index];
            }
            if ctx.description.starts_with("Confirm mana payment for") { return vec![1]; }
            ctx.options.iter().find(|o| o.legal).map(|o| vec![o.index]).unwrap_or_default()
        }
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.accept }
    }
    let definition = compile_to_runtime_definition("Animation Module", "Mana cost: {1}\nType: Artifact\nWhenever one or more +1/+1 counters are put on a permanent you control, you may pay {1}. If you do, create a 1/1 colorless Servo artifact creature token.\n{3}, {T}: Choose a counter on target permanent or player. Give that permanent or player another counter of that kind.", false).unwrap();
    let index = definition.abilities.iter().position(|a| matches!(&a.kind, ironsmith::ability::AbilityKind::Activated(_))).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["own permanent", "opponent permanent", "own player", "opponent player", "no object counters", "no player counters", "one kind", "source leaves", "target gone", "decline token"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 4);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let fixture = CardBuilder::new(CardId::new(), "Counter recipient").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 4)).build();
        let object = game.create_object_from_card(&fixture, if case == "opponent permanent" { bob } else { alice }, Zone::Battlefield);
        let is_player = matches!(case, "own player" | "opponent player" | "no player counters");
        let player = if case == "own player" { alice } else { bob };
        let absent = matches!(case, "no object counters" | "no player counters");
        if is_player && !absent {
            game.add_player_counters_with_source(player, CounterType::Energy, 2, None, None).unwrap();
            game.add_player_counters_with_source(player, CounterType::Poison, 3, None, None).unwrap();
        } else if !absent {
            game.object_mut(object).unwrap().counters.insert(CounterType::Charge, 3);
            if case != "one kind" { game.object_mut(object).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2); }
        }
        let kind = match case {
            "own player" => CounterType::Energy,
            "opponent player" => CounterType::Poison,
            "own permanent" | "opponent permanent" | "decline token" => CounterType::PlusOnePlusOne,
            _ => CounterType::Charge,
        };
        let target = if is_player { ironsmith::game_state::Target::Player(player) } else { ironsmith::game_state::Target::Object(object) };
        let mut dm = Answers { target, kind, choices: 0, kinds: if case == "one kind" { 1 } else { 2 }, accept: case != "decline token" };
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let action = ironsmith::decision::LegalAction::ActivateAbility { source, ability_index: index };
        assert!(ironsmith::decision::compute_legal_actions(&game, alice).unwrap().contains(&action), "{case}: a counterless recipient is still a legal target");
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..40 {
            if state.pending_activation.is_none() && !game.stack.is_empty() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(state.pending_activation.is_none());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![target]);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
        if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
        if case == "target gone" { game.move_object_by_effect(object, Zone::Exile).unwrap(); }
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.choices, usize::from(!absent && case != "target gone"), "{case}: choose one kind at resolution, never every kind");
        if is_player {
            for (counter, before) in [(CounterType::Energy, 2), (CounterType::Poison, 3)] {
                assert_eq!(game.player(player).unwrap().counter_count(counter), if absent { 0 } else { before + u32::from(counter == kind) });
            }
        } else if case != "target gone" {
            for (counter, before) in [(CounterType::Charge, 3), (CounterType::PlusOnePlusOne, if case == "one kind" { 0 } else { 2 })] {
                assert_eq!(game.counter_count(object, counter), if absent { 0 } else { before + u32::from(counter == kind) }, "{case}: add exactly one of the chosen kind");
            }
        }
        drain_pending_trigger_events(&mut game, &mut queue);
        let trigger = matches!(case, "own permanent" | "decline token");
        assert_eq!(queue.entries.len(), usize::from(trigger), "{case}: only your permanent receiving a +1/+1 counter triggers");
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        while !game.stack.is_empty() { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        let tokens = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token).collect::<Vec<_>>();
        assert_eq!(tokens.len(), usize::from(case == "own permanent"));
        if let Some(token) = tokens.first() {
            assert_eq!(game.calculated_power(*token), Some(1)); assert_eq!(game.calculated_toughness(*token), Some(1));
            assert_eq!(game.current_colors(*token), Some(ironsmith::color::ColorSet::default()));
            assert!(game.current_subtypes(*token).unwrap().contains(&Subtype::Servo));
            let types = game.current_card_types(*token).unwrap();
            assert!(types.contains(&CardType::Artifact) && types.contains(&CardType::Creature));
        }
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), if case == "own permanent" { 0 } else { 1 });
    }
}

#[test]
fn counter_kinds_bribe_taker_captures_each_distinct_kind_and_chooses_independently() {
    use ironsmith::object::CounterType;
    struct Answers { mask: usize, kinds: Vec<CounterType>, seen: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.player, PlayerId::from_index(0), "the queued ability controller chooses");
            assert_eq!(ctx.options.len(), 3);
            assert!(ctx.options.iter().all(|o| !o.description.contains("Remove")));
            let expected = format!("Choose for {:?} counter", self.kinds[self.seen]).to_ascii_lowercase();
            assert_eq!(ctx.description.to_ascii_lowercase(), expected);
            let choice = self.mask / 3usize.pow(self.seen as u32) % 3;
            self.seen += 1;
            vec![choice]
        }
    }
    let (artifact, direct) = super::compile_to_artifact("Bribe Taker", "Mana cost: {5}{G}\nType: Creature — Rhino Warrior\nPower/Toughness: 6/6\nTrample\nWhen this creature enters, for each kind of counter on permanents you control, you may put your choice of a +1/+1 counter or a counter of that kind on this creature.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        let trigger = definition.abilities.iter().find_map(|a| match &a.kind { ironsmith::ability::AbilityKind::Triggered(t) => Some(t), _ => None }).unwrap();
        let kernel = trigger.effects.all_effects().into_iter().find_map(|e| e.downcast_ref::<ironsmith::effects::ForEachCounterKindPutOrRemoveEffect>()).unwrap();
        assert_eq!(kernel.alternative_put_counter, Some(CounterType::PlusOnePlusOne));
        assert!(kernel.all_kinds && kernel.optional_action);
        assert!(!kernel.put_only && !kernel.choose_target_per_kind);
        assert!(kernel.counter_source.as_ref().is_some_and(|spec| matches!(spec.base(), ChooseSpec::All(filter) if filter.controller == Some(ironsmith::target::PlayerFilter::You))));
        for case in ["normal", "controller changes", "new kind", "no kinds", "sources change", "source leaves"] {
            let masks = if matches!(case, "normal" | "controller changes") { 27 } else { 1 };
            for ordinal in 0..masks {
                let mask = if case == "new kind" { 13 } else { ordinal };
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let artifact = CardBuilder::new(CardId::new(), "Counter source artifact").card_types(vec![CardType::Artifact]).build();
                let first = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
                game.object_mut(first).unwrap().counters.insert(CounterType::Age, 2);
                game.object_mut(first).unwrap().counters.insert(CounterType::Charge, 4);
                game.object_mut(first).unwrap().counters.insert(CounterType::Vigilance, 0);
                let creature = CardBuilder::new(CardId::new(), "Duplicate counter kind source").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 4)).build();
                let second = game.create_object_from_card(&creature, alice, Zone::Battlefield);
                game.object_mut(second).unwrap().counters.insert(CounterType::Charge, 7);
                if case != "new kind" { game.object_mut(second).unwrap().counters.insert(CounterType::PlusOnePlusOne, 1); }
                let foreign = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
                game.object_mut(foreign).unwrap().counters.insert(CounterType::Energy, 9);
                let exiled = game.create_object_from_card(&artifact, alice, Zone::Exile);
                game.object_mut(exiled).unwrap().counters.insert(CounterType::Deathtouch, 3);
                game.add_player_counters_with_source(alice, CounterType::Poison, 2, None, None).unwrap();
                if case == "no kinds" { game.object_mut(first).unwrap().counters.clear(); game.object_mut(second).unwrap().counters.clear(); }
                let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let source = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Bribe Taker").unwrap();
                assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Trample));
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
                assert_eq!(queue.entries.len(), 1);
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                assert!(game.stack[0].targets.is_empty());
                if case == "controller changes" { game.set_current_controller(source, bob).unwrap(); }
                if case == "sources change" { game.object_mut(first).unwrap().counters.clear(); }
                let graveyard = if case == "source leaves" { Some(game.move_object_by_effect(source, Zone::Graveyard).unwrap()) } else { None };
                let kinds = match case {
                    "no kinds" | "source leaves" => vec![],
                    "new kind" => vec![CounterType::Age, CounterType::Charge],
                    "sources change" => vec![CounterType::Charge, CounterType::PlusOnePlusOne],
                    _ => vec![CounterType::Age, CounterType::Charge, CounterType::PlusOnePlusOne],
                };
                let mut dm = Answers { mask, kinds: kinds.clone(), seen: 0 };
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.seen, kinds.len(), "{case}, mask={mask}: quantities, duplicates, zero counts, foreign permanents and player counters do not add iterations");
                let mut expected = std::collections::BTreeMap::new();
                for (index, kind) in kinds.iter().enumerate() {
                    let choice = mask / 3usize.pow(index as u32) % 3;
                    if choice != 2 { *expected.entry(if choice == 1 { CounterType::PlusOnePlusOne } else { *kind }).or_insert(0u32) += 1; }
                }
                if let Some(graveyard) = graveyard { assert!(game.object(graveyard).unwrap().counters.is_empty(), "the new graveyard incarnation cannot receive these counters"); }
                else {
                    for kind in [CounterType::Age, CounterType::Charge, CounterType::PlusOnePlusOne, CounterType::Vigilance, CounterType::Energy, CounterType::Deathtouch, CounterType::Poison] {
                        assert_eq!(game.counter_count(source, kind), *expected.get(&kind).unwrap_or(&0), "{case}, mask={mask}, kind={kind:?}");
                    }
                }
                assert_eq!(game.counter_count(foreign, CounterType::Energy), 9);
                assert_eq!(game.player(alice).unwrap().counter_count(CounterType::Poison), 2);
            }
        }
    }
}

#[test]
fn entry_counter_kinds_blue_uses_live_source_and_preserves_entry_replacements() {
    use ironsmith::object::CounterType;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    let (artifact, direct) = super::compile_to_artifact("Blue, Loyal Raptor", "Mana cost: {2}{G}{U}\nType: Legendary Creature — Dinosaur\nPower/Toughness: 5/4\nPartner with Owen Grady, Raptor Trainer (When this creature enters, target player may put Owen into their hand from their library, then shuffle.)\nFor each kind of counter on Blue, Loyal Raptor, each other Dinosaur you control enters with a counter of that kind on it.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        let model = definition.abilities.iter().find_map(|a| match &a.kind {
            ironsmith::ability::AbilityKind::Static(s) => s.compiled_model().filter(|m| matches!(m.payload, ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter { .. })), _ => None,
        }).unwrap();
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter { filter, counter_kinds_from, .. } => {
                assert!(filter.other);
                assert!(filter.subtypes.contains(&Subtype::Dinosaur));
                assert_eq!(filter.controller, Some(PlayerFilter::You));
                assert!(matches!(counter_kinds_from.as_ref().unwrap().base(), ChooseSpec::Source));
            }
            other => panic!("unexpected entry payload: {other:?}"),
        }
        for case in ["creature", "kindred", "opponent", "wrong subtype", "no kinds", "control changes", "source leaves", "live kinds", "tokens", "double", "additional counters"] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            for (kind, count) in [(CounterType::Charge, 5), (CounterType::Flying, 2), (CounterType::PlusOnePlusOne, 3), (CounterType::Vigilance, 0)] {
                game.object_mut(source).unwrap().counters.insert(kind, count);
            }
            let foreign = CardBuilder::new(CardId::new(), "Unrelated counter source").card_types(vec![CardType::Artifact]).build();
            let foreign = game.create_object_from_card(&foreign, alice, Zone::Battlefield);
            game.object_mut(foreign).unwrap().counters.insert(CounterType::Age, 9);
            game.add_player_counters_with_source(alice, CounterType::Poison, 2, None, None).unwrap();
            let mut filter = ObjectFilter::default(); filter.controller = Some(PlayerFilter::You); filter.zone = Some(Zone::Battlefield);
            if matches!(case, "double" | "additional counters") {
                let rule = if case == "double" { StaticAbility::double_counters_replacement(filter, None, "Double counters".into()) }
                    else { StaticAbility::enters_with_counters_for_filter(filter, CounterType::Charge, 3) };
                let fixture = CardDefinitionBuilder::new(CardId::new(), "Entry modifier").card_types(vec![CardType::Enchantment]).with_ability(ironsmith::ability::Ability::static_ability(rule)).build();
                game.create_object_from_definition(&fixture, alice, Zone::Battlefield);
            }
            if case == "no kinds" { game.object_mut(source).unwrap().counters.clear(); }
            if case == "control changes" { game.set_current_controller(source, bob).unwrap(); }
            let owner = if matches!(case, "opponent" | "control changes") { bob } else { alice };
            let entrant = CardDefinitionBuilder::new(CardId::new(), "Entry fixture")
                .card_types(if case == "kindred" { vec![CardType::Kindred, CardType::Artifact] } else { vec![CardType::Creature] })
                .subtypes(vec![if case == "wrong subtype" { Subtype::Human } else { Subtype::Dinosaur }])
                .power_toughness(PowerToughness::fixed(2, 2)).build();
            if case == "tokens" {
                let token = CardDefinitionBuilder::new(CardId::new(), "Entry token").token().card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Dinosaur]).power_toughness(PowerToughness::fixed(2, 2)).build();
                game.push_to_stack(ironsmith::game_state::StackEntry::ability(source, alice, vec![Effect::new(ironsmith::effects::CreateTokenEffect::new(token, 3, PlayerFilter::You))]));
            } else {
                let spell = game.create_object_from_definition(&entrant, owner, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, owner));
            }
            if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let entrants: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().name.starts_with("Entry fixture") || game.object(*id).unwrap().name == "Entry token").collect();
            assert_eq!(entrants.len(), if case == "tokens" { 3 } else { 1 }, "{case}");
            let eligible = !matches!(case, "opponent" | "wrong subtype" | "no kinds" | "source leaves");
            let count = if case == "double" { 2 } else { u32::from(eligible) };
            for id in &entrants {
                assert_eq!(game.counter_count(*id, CounterType::Charge), count + if case == "additional counters" { 3 } else { 0 }, "{case}");
                for kind in [CounterType::Flying, CounterType::PlusOnePlusOne] { assert_eq!(game.counter_count(*id, kind), count, "{case}: {kind:?}"); }
                for kind in [CounterType::Age, CounterType::Vigilance, CounterType::Poison] { assert_eq!(game.counter_count(*id, kind), 0, "{case}: {kind:?}"); }
                assert_eq!(game.current_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Flying), eligible, "{case}");
            }
            if case == "live kinds" {
                game.object_mut(source).unwrap().counters.remove(&CounterType::Flying);
                game.object_mut(source).unwrap().counters.insert(CounterType::Age, 7);
                let spell = game.create_object_from_definition(&entrant, alice, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let second = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Entry fixture" && !entrants.contains(id)).unwrap();
                assert_eq!(game.counter_count(second, CounterType::Age), 1);
                assert_eq!(game.counter_count(second, CounterType::Flying), 0);
                assert_eq!(game.counter_count(second, CounterType::Charge), 1);
                assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 1);
                assert_eq!(game.counter_count(entrants[0], CounterType::Flying), 1);
                assert_eq!(game.counter_count(entrants[0], CounterType::Age), 0);
            }
        }
        struct PartnerAnswers { accept: bool, chosen: ironsmith::ids::ObjectId, booleans: usize }
        impl ironsmith::decision::DecisionMaker for PartnerAnswers {
            fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
                let target = ironsmith::game_state::Target::Player(PlayerId::from_index(1));
                assert!(ctx.requirements[0].legal_targets.contains(&target));
                vec![target]
            }
            fn decide_boolean(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
                assert_eq!(ctx.player, PlayerId::from_index(1));
                self.booleans += 1;
                self.accept
            }
            fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
                assert_eq!(ctx.player, PlayerId::from_index(1));
                let legal: Vec<_> = ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).collect();
                assert_eq!(legal, vec![self.chosen]);
                vec![self.chosen]
            }
        }
        for case in ["accept", "decline", "source leaves"] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let owen = CardBuilder::new(CardId::new(), "Owen Grady, Raptor Trainer").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 2)).build();
            let chosen = game.create_object_from_card(&owen, bob, Zone::Library);
            game.create_object_from_card(&owen, alice, Zone::Library);
            let decoy = CardBuilder::new(CardId::new(), "Other Dinosaur").card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Dinosaur]).power_toughness(PowerToughness::fixed(2, 2)).build();
            game.create_object_from_card(&decoy, bob, Zone::Library);
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let source = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Blue, Loyal Raptor").unwrap();
            assert!(game.object(source).unwrap().counters.is_empty(), "the source is excluded from its entry rule");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            assert_eq!(queue.entries.len(), 1, "Partner with must retain its entry search trigger");
            let mut dm = PartnerAnswers { accept: case != "decline", chosen, booleans: 0 };
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack[0].targets, vec![ironsmith::game_state::Target::Player(bob)]);
            if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.booleans, 1);
            assert_eq!(game.player(bob).unwrap().hand.len(), usize::from(case != "decline"));
            if case != "decline" { assert_eq!(game.object(game.player(bob).unwrap().hand[0]).unwrap().name, "Owen Grady, Raptor Trainer"); }
            assert!(game.player(alice).unwrap().hand.is_empty());
            assert_eq!(game.player(alice).unwrap().library.len(), 1);
        }
    }
}

#[test]
fn source_filtered_counter_autumn_protects_only_against_matching_spells_until_cleanup() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::color::ColorSet;
    let (artifact, direct) = super::compile_to_artifact("Autumn's Veil", "Mana cost: {G}\nType: Instant\nSpells you control can't be countered by blue or black spells this turn, and creatures you control can't be the targets of blue or black spells this turn.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        let debug = format!("{definition:?}");
        assert!(debug.contains("BeCounteredFrom") && debug.contains("BeTargetedFrom"));
        for color in [ColorSet::BLUE, ColorSet::BLACK, ColorSet::GREEN, ColorSet::RED, ColorSet::new(), ColorSet::BLUE.union(ColorSet::GREEN)] {
            for ability in [false, true] { for opponent in [false, true] { for expired in [false, true] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let veil = game.create_object_from_definition(&definition, alice, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(veil, alice));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                // The resolution must protect spells cast and creatures entering later.
                let owner = if opponent { bob } else { alice };
                let victim = CardBuilder::new(CardId::new(), "Spell to counter").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
                let victim = game.create_object_from_card(&victim, owner, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(victim, owner));
                let creature = CardBuilder::new(CardId::new(), "Protected target").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
                let creature = game.create_object_from_card(&creature, owner, Zone::Battlefield);
                let artifact = CardBuilder::new(CardId::new(), "Unprotected artifact").card_types(vec![CardType::Artifact]).build();
                let artifact = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
                let counter = Effect::new(ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(victim)));
                let source = if ability {
                    let card = CardBuilder::new(CardId::new(), "Counter ability source").card_types(vec![CardType::Artifact]).color_indicator(color).build();
                    let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
                    game.push_to_stack(ironsmith::game_state::StackEntry::ability(source, bob, vec![counter]));
                    source
                } else {
                    let card = CardDefinitionBuilder::new(CardId::new(), "Counter spell").card_types(vec![CardType::Instant]).color_indicator(color).with_spell_effect(vec![counter]).build();
                    let source = game.create_object_from_definition(&card, bob, Zone::Stack);
                    game.push_to_stack(ironsmith::game_state::StackEntry::new(source, bob));
                    source
                };
                if expired { game.cleanup_restrictions_end_of_turn(); }
                game.refresh_continuous_state().unwrap();
                let protected = !ability && !opponent && !expired && (color.contains(ironsmith::color::Color::Blue) || color.contains(ironsmith::color::Color::Black));
                assert_eq!(game.can_target_object_from_source(creature, source), !protected, "color={color:?}, ability={ability}, opponent={opponent}, expired={expired}");
                assert!(game.can_target_object_from_source(artifact, source));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.stack.iter().any(|entry| entry.object_id == victim), protected, "color={color:?}, ability={ability}, opponent={opponent}, expired={expired}");
                if protected { assert_eq!(game.object(victim).unwrap().zone, Zone::Stack); }
                else { assert!(game.player(owner).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name == "Spell to counter")); }
            } } }
        }
    }
}

#[test]
fn recipient_damage_baki_counts_only_auras_on_each_live_recipient() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let (artifact, direct) = super::compile_to_artifact("Baki's Curse", "Mana cost: {2}{U}{U}\nType: Sorcery\nBaki's Curse deals 2 damage to each creature for each Aura attached to that creature.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DealDamageEachEffect") && debug.contains("AttachedToTaggedObject"));
        for count in 0..=4 {
            for changed in [false, true] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let creature = CardDefinitionBuilder::new(CardId::new(), "Damage recipient").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 100)).with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::shroud())).build();
                let first = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
                let second = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
                let bare = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
                let aura = CardDefinitionBuilder::new(CardId::new(), "Attached Aura").card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura]).build();
                let aura_token = CardDefinitionBuilder::new(CardId::new(), "Aura token").token().card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura]).build();
                let mut first_auras = Vec::new();
                for index in 0..count {
                    let id = game.create_object_from_definition(if index == 0 { &aura_token } else { &aura }, if index % 2 == 0 { bob } else { alice }, Zone::Battlefield);
                    assert!(game.attach_object_to_target(id, ironsmith::object::AttachmentTarget::Object(first)));
                    first_auras.push(id);
                }
                for _ in 0..(4-count) {
                    let id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
                    assert!(game.attach_object_to_target(id, ironsmith::object::AttachmentTarget::Object(second)));
                }
                let equipment = CardBuilder::new(CardId::new(), "Equipment decoy").card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment]).build();
                let equipment = game.create_object_from_card(&equipment, alice, Zone::Battlefield);
                assert!(game.attach_object_to_target(equipment, ironsmith::object::AttachmentTarget::Object(first)));
                let enchantment = CardBuilder::new(CardId::new(), "Non-Aura enchantment").card_types(vec![CardType::Enchantment]).build();
                let enchantment = game.create_object_from_card(&enchantment, alice, Zone::Battlefield);
                game.create_object_from_definition(&aura, alice, Zone::Battlefield);
                let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
                game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
                if changed {
                    if let Some(aura) = first_auras.first() { assert!(game.attach_object_to_target(*aura, ironsmith::object::AttachmentTarget::Object(second))); }
                    else {
                        let extra = game.create_object_from_definition(&aura, bob, Zone::Battlefield);
                        assert!(game.attach_object_to_target(extra, ironsmith::object::AttachmentTarget::Object(first)));
                    }
                }
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let first_count = if changed { if count == 0 { 1 } else { count-1 } } else { count };
                let second_count = 4-count + u32::from(changed && count > 0);
                assert_eq!(game.damage_on(first), 2*first_count, "count={count}, changed={changed}");
                assert_eq!(game.damage_on(second), 2*second_count, "count={count}, changed={changed}");
                assert_eq!(game.damage_on(bare), 0, "a creature without Auras receives no damage");
                assert_eq!(game.damage_on(equipment), 0);
                assert_eq!(game.damage_on(enchantment), 0);
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(bob).unwrap().life, 20);
            }
        }
    }
}

#[test]
fn divided_damage_avacyn_reuses_mixed_targets_and_announced_madness_division() {
    use ironsmith::decision::{DecisionMaker, LegalAction};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    use ironsmith::game_state::Target;
    use ironsmith::mana::ManaSymbol;
    struct Answers { x: u32, total: u32, selected: Vec<Target>, divisions: usize }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
        fn decide_number(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::NumberContext) -> u32 {
            if ctx.is_x_value { assert!(self.x <= ctx.max); self.x }
            else { ironsmith::decision::SelectFirstDecisionMaker.decide_number(game, ctx) }
        }
        fn decide_options(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Confirm mana payment for") { vec![1] }
            else { ironsmith::decision::SelectFirstDecisionMaker.decide_options(game, ctx) }
        }
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            ironsmith::decision::SelectFirstDecisionMaker.decide_objects(game, ctx)
        }
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
            assert_eq!(ctx.requirements.len(), 1);
            assert_eq!(ctx.requirements[0].min_targets, 0);
            assert!(self.selected.iter().all(|target| ctx.requirements[0].legal_targets.contains(target)));
            self.selected.clone()
        }
        fn decide_distribute(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::DistributeContext) -> Vec<(Target, u32)> {
            self.divisions += 1;
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(ctx.total, self.total);
            assert_eq!(ctx.min_per_target, 1);
            assert_eq!(ctx.targets.len(), self.selected.len());
            self.selected.iter().enumerate().map(|(index, target)| (*target, 1 + if index == 0 { self.total - self.selected.len() as u32 } else { 0 })).collect()
        }
    }
    let (artifact, direct) = super::compile_to_artifact("Avacyn's Judgment", "Mana cost: {1}{R}\nType: Sorcery\nMadness {X}{R} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)\nAvacyn's Judgment deals 2 damage divided as you choose among any number of targets. If this spell's madness cost was paid, it deals X damage divided as you choose among those permanents and/or players instead.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        assert!(definition.alternative_casts.iter().any(|method| method.name() == "Madness"));
        for madness in [false, true] { for x in 0..=if madness { 4 } else { 0 } {
            let total = if madness { x } else { 2 };
            for chosen in 0..=total { for target_leaves in [false, true] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.active_player = alice; game.turn.priority_player = Some(alice);
                game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 20);
                let creature = CardBuilder::new(CardId::new(), "Damage creature").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 100)).build();
                let creature = game.create_object_from_card(&creature, bob, Zone::Battlefield);
                let walker = CardBuilder::new(CardId::new(), "Damage walker").card_types(vec![CardType::Planeswalker]).build();
                let walker = game.create_object_from_card(&walker, bob, Zone::Battlefield);
                game.object_mut(walker).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty, 20);
                let pool = [Target::Object(creature), Target::Player(bob), Target::Object(walker), Target::Player(alice)];
                let selected = pool[..chosen as usize].to_vec();
                let mut dm = Answers { x, total, selected: selected.clone(), divisions: 0 };
                let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
                if madness {
                    let host = CardBuilder::new(CardId::new(), "Discard source").card_types(vec![CardType::Artifact]).build();
                    let host = game.create_object_from_card(&host, alice, Zone::Battlefield);
                    game.push_to_stack(ironsmith::game_state::StackEntry::ability(host, alice, vec![Effect::discard(1)]));
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                    assert_eq!(game.stack.len(), 1, "discard must queue the linked madness trigger");
                    assert!(game.stack[0].is_ability);
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                } else {
                    let action = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal };
                    assert!(ironsmith::decision::compute_legal_actions(&game, alice).unwrap().contains(&action));
                    let mut state = PriorityLoopState::new(2); let mut queue = ironsmith::triggers::TriggerQueue::new();
                    let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                    for _ in 0..60 {
                        if state.pending_cast.is_none() { break; }
                        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
                        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                    }
                    assert!(state.pending_cast.is_none());
                }
                assert_eq!(game.stack.len(), 1);
                assert!(!game.stack[0].is_ability);
                assert_eq!(game.stack[0].targets, selected);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 20 - if madness { x+1 } else { 2 });
                let divisions = dm.divisions;
                assert_eq!(divisions, usize::from(chosen > 0));
                if target_leaves { game.move_object_by_effect(creature, Zone::Exile).unwrap(); }
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.divisions, divisions, "the division is announced on casting and never redone or redistributed");
                let first_damage = if chosen > 0 { total-chosen+1 } else { 0 };
                if !target_leaves { assert_eq!(game.damage_on(creature), first_damage, "madness={madness}, X={x}, chosen={chosen}"); }
                assert_eq!(game.player(bob).unwrap().life, 20 - i32::from(chosen >= 2));
                assert_eq!(game.counter_count(walker, ironsmith::object::CounterType::Loyalty), 20 - u32::from(chosen >= 3));
                assert_eq!(game.player(alice).unwrap().life, 20 - i32::from(chosen >= 4));
            } }
        } }
    }
}

#[test]
fn counter_prohibition_solemnity_blocks_placements_without_removing_existing_counters() {
    let (artifact, direct) = super::compile_to_artifact("Solemnity", "Mana cost: {2}{W}\nType: Enchantment\nPlayers can't get counters.\nCounters can't be put on artifacts, creatures, enchantments, or lands.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] { for owner in [alice, bob] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let fixture = CardBuilder::new(CardId::new(), "Counter source").card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&fixture, alice, Zone::Battlefield);
        let mut recipients = Vec::new();
        for card_type in [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker, CardType::Battle] {
            let card = CardBuilder::new(CardId::new(), "Counter recipient").card_types(vec![card_type]).power_toughness(PowerToughness::fixed(2, 3)).build();
            let object = game.create_object_from_card(&card, owner, Zone::Battlefield);
            game.add_counters(object, ironsmith::object::CounterType::Charge, 1);
            if card_type == CardType::Planeswalker { game.add_counters(object, ironsmith::object::CounterType::Loyalty, 5); }
            if card_type == CardType::Battle { game.add_counters(object, ironsmith::object::CounterType::Defense, 5); }
            recipients.push((object, !matches!(card_type, CardType::Planeswalker | CardType::Battle)));
        }
        let counters = [ironsmith::object::CounterType::Poison, ironsmith::object::CounterType::Energy, ironsmith::object::CounterType::Experience, ironsmith::object::CounterType::Rad, ironsmith::object::CounterType::Charge];
        for player in [alice, bob] { for counter in counters {
            game.add_player_counters_with_source(player, counter, 1, Some(source), Some(alice)).unwrap();
        } }
        let solemnity = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        for present in [true, false] {
            if !present { game.move_object_by_effect(solemnity, Zone::Graveyard).unwrap(); game.refresh_continuous_state().unwrap(); }
            for player in [alice, bob] { for counter in counters {
                let out = game.add_player_counters_with_source(player, counter, 2, Some(source), Some(alice)).unwrap();
                assert_eq!(out.as_count(), Some(if present { 0 } else { 2 }));
                assert_eq!(game.player(player).unwrap().counter_count(counter), if present { 1 } else { 3 });
            } }
            for (object, covered) in &recipients {
                game.push_to_stack(ironsmith::game_state::StackEntry::ability(source, alice, vec![Effect::new(ironsmith::effects::PutCountersEffect::new(ironsmith::object::CounterType::Charge, 2, ChooseSpec::SpecificObject(*object)))]));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.counter_count(*object, ironsmith::object::CounterType::Charge), if *covered { if present { 1 } else { 3 } } else { if present { 3 } else { 5 } });
            }
        }
    } }
}

#[test]
fn player_identity_ghoul_checks_the_announced_player_and_retains_trigger_controller() {
    struct Answers { target: PlayerId, declarations: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            self.declarations += 1;
            assert_eq!(ctx.requirements.len(), 1);
            let target = ironsmith::game_state::Target::Player(self.target);
            assert!(ctx.requirements[0].legal_targets.contains(&target));
            vec![target]
        }
    }
    let (artifact, direct) = super::compile_to_artifact("The Ghoul, Gunslinger", "Mana cost: {1}{B}{B}\nType: Legendary Creature — Zombie Mutant Rogue\nPower/Toughness: 2/3\nFirst strike\nWhenever The Ghoul or another nontoken Zombie or Mutant you control dies, target player gets two rad counters. If that player is you, create a Treasure token.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    for definition in [direct, roundtrip] { for controller in [alice, bob] { for target in [alice, bob, carol] {
        for death in ["source", "zombie", "mutant", "both", "token zombie", "token mutant", "opponent zombie", "human"] {
            for after in ["normal", "source leaves", "control changes"] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                game.set_current_controller(source, controller).unwrap();
                assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::FirstStrike));
                let subtypes = match death { "mutant" | "token mutant" => vec![Subtype::Mutant], "both" => vec![Subtype::Zombie, Subtype::Mutant], "human" => vec![Subtype::Human], _ => vec![Subtype::Zombie] };
                let fixture = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Death fixture").card_types(vec![CardType::Creature]).subtypes(subtypes).power_toughness(PowerToughness::fixed(2, 2));
                let fixture = if death.starts_with("token ") { fixture.token() } else { fixture }.build();
                let victim = if death == "source" { source } else { game.create_object_from_definition(&fixture, if death == "opponent zombie" { carol } else { controller }, Zone::Battlefield) };
                game.push_to_stack(ironsmith::game_state::StackEntry::ability(source, controller, vec![Effect::destroy(ChooseSpec::SpecificObject(victim))]));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
                let triggers = matches!(death, "source" | "zombie" | "mutant" | "both");
                assert_eq!(queue.entries.len(), usize::from(triggers), "controller={controller:?}, {death}");
                if !triggers { continue; }
                let mut dm = Answers { target, declarations: 0 };
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                assert_eq!(dm.declarations, 1);
                assert_eq!(game.stack[0].controller, controller);
                if death != "source" {
                    if after == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                    if after == "control changes" { game.set_current_controller(source, carol).unwrap(); }
                }
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                for player in [alice, bob, carol] {
                    assert_eq!(game.player(player).unwrap().counter_count(ironsmith::object::CounterType::Rad), if player == target { 2 } else { 0 }, "{death}, {after}");
                    let treasures = game.battlefield.iter().filter(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token && game.current_subtypes(**id).is_some_and(|types| types.contains(&Subtype::Treasure)) && game.current_controller(**id) == Some(player)).count();
                    assert_eq!(treasures, usize::from(player == controller && target == controller), "controller={controller:?}, target={target:?}, {death}, {after}");
                }
            }
        }
    } } }
}

#[test]
fn library_replacement_aladdin_keeps_one_and_draws_without_replacing_itself() {
    use ironsmith::decision::{DecisionMaker, LegalAction};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct Answers { x: u32, keep: Option<ironsmith::ids::ObjectId>, pool: Vec<ironsmith::ids::ObjectId>, choices: usize }
    impl DecisionMaker for Answers {
        fn decide_number(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::NumberContext) -> u32 {
            if ctx.is_x_value { assert_eq!(ctx.min, 1); assert!(self.x <= ctx.max); self.x }
            else { ironsmith::decision::SelectFirstDecisionMaker.decide_number(game, ctx) }
        }
        fn decide_options(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Confirm mana payment for") { vec![1] }
            else { ironsmith::decision::SelectFirstDecisionMaker.decide_options(game, ctx) }
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            self.choices += 1;
            assert_eq!(ctx.player, PlayerId::from_index(0));
            let actual: std::collections::BTreeSet<_> = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
            assert_eq!(actual, self.pool.iter().copied().collect());
            assert_eq!(ctx.min, usize::from(!self.pool.is_empty()));
            assert_eq!(ctx.max, Some(1));
            self.keep.into_iter().collect()
        }
    }
    let (artifact, direct) = super::compile_to_artifact("Aladdin's Lamp", "Mana cost: {10}\nType: Artifact\n{X}, {T}: The next time you would draw a card this turn, instead look at the top X cards of your library, put all but one of them on the bottom of your library in a random order, then draw a card. X can't be 0.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for definition in [direct, roundtrip] {
        let index = definition.abilities.iter().position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))).unwrap();
        let activated = match &definition.abilities[index].kind { ironsmith::ability::AbilityKind::Activated(ability) => ability, _ => unreachable!() };
        assert_eq!(activated.activation_x_minimum(), 1);
        for x in 1..=4 { for size in [0usize, 2, 5] { for chosen in 0..(x as usize).min(size).max(1) {
            for case in ["normal", "source leaves", "control changes", "expired", "opponent draw first", "draw two"] {
                if case == "draw two" && size <= x as usize { continue; }
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.priority_player = Some(alice);
                game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 10);
                let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                for ordinal in 0..size {
                    let card = CardBuilder::new(CardId::new(), format!("Library card {ordinal}")).card_types(vec![CardType::Artifact]).build();
                    game.create_object_from_card(&card, alice, Zone::Library);
                    game.create_object_from_card(&card, bob, Zone::Library);
                }
                let original = game.player(alice).unwrap().library.clone();
                let pool = original.iter().rev().take(x as usize).copied().collect::<Vec<_>>();
                let keep = pool.get(chosen).copied();
                let keep_name = keep.map(|id| game.object(id).unwrap().name.clone());
                let mut dm = Answers { x, keep, pool, choices: 0 };
                let action = LegalAction::ActivateAbility { source, ability_index: index };
                assert!(ironsmith::decision::compute_legal_actions(&game, alice).unwrap().contains(&action));
                let mut state = PriorityLoopState::new(2); let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..60 {
                    if state.pending_activation.is_none() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(state.pending_activation.is_none());
                assert!(game.is_tapped(source));
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 10-x);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.choices, 0, "the library choice belongs to the future draw");
                if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                if case == "control changes" { game.set_current_controller(source, bob).unwrap(); }
                if case == "expired" { ironsmith::turn::execute_cleanup_step(&mut game); }
                if case == "opponent draw first" && size > 0 {
                    game.push_to_stack(ironsmith::game_state::StackEntry::ability(source, alice, vec![Effect::new(ironsmith::effects::DrawCardsEffect::new(1, ironsmith::target::PlayerFilter::Specific(bob)))]));
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(game.player(bob).unwrap().hand.len(), 1);
                    assert_eq!(dm.choices, 0);
                }
                let draws = if case == "draw two" { 2 } else { 1 };
                game.push_to_stack(ironsmith::game_state::StackEntry::ability(source, alice, vec![Effect::draw(draws as u32)]));
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let hand = &game.player(alice).unwrap().hand;
                assert_eq!(hand.len(), (draws as usize).min(size), "X={x}, size={size}, {case}");
                if size > 0 {
                    let expected = if case == "expired" { format!("Library card {}", size-1) } else { keep_name.unwrap().to_string() };
                    assert_eq!(game.object(hand[0]).unwrap().name.to_string(), expected, "X={x}, size={size}, chosen={chosen}, {case}");
                }
                if case == "draw two" { assert_eq!(game.object(hand[1]).unwrap().name, format!("Library card {}", size-x as usize-1), "only the first draw is replaced"); }
                if case == "expired" { assert_eq!(dm.choices, 0); }
                assert_eq!(game.player(alice).unwrap().library.len(), size-hand.len());
                if case != "expired" {
                    let remaining = game.player(alice).unwrap().library.iter().map(|id| game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
                    let moved = (x as usize).min(size).saturating_sub(1);
                    let bottom = remaining[..moved].iter().cloned().collect::<std::collections::BTreeSet<_>>();
                    let expected_bottom = dm.pool.iter().filter(|id| Some(**id) != keep).map(|id| format!("Library card {}", original.iter().position(|original| original == id).unwrap())).collect::<std::collections::BTreeSet<_>>();
                    assert_eq!(bottom, expected_bottom, "all unkept looked-at cards go to the bottom");
                    let untouched = size.saturating_sub(x as usize).saturating_sub(usize::from(case == "draw two"));
                    assert_eq!(remaining[moved..], (0..untouched).map(|ordinal| format!("Library card {ordinal}")).collect::<Vec<_>>(), "unlooked-at cards retain their order above the random remainder");
                }
                assert_eq!(game.player(alice).unwrap().attempted_draw_from_empty_library, size == 0, "an empty-library draw still fails normally");
                assert!(dm.choices <= 1, "the replacement's own draw must not apply itself again");
                assert!(game.effect_store.replacement_effects.one_shot_effects_snapshot().is_empty());
            }
        } } }
    }
}

#[test]
fn delayed_exile_dragonhawk_counts_current_batch_at_resolution_and_keeps_its_controller() {
    let (artifact, direct) = super::compile_to_artifact("Dragonhawk, Fate's Tempest", "Mana cost: {3}{R}{R}\nType: Legendary Creature — Bird Dragon\nPower/Toughness: 5/5\nFlying\nWhenever Dragonhawk enters or attacks, exile the top X cards of your library, where X is the number of creatures you control with power 4 or greater. You may play those cards until your next end step. At the beginning of your next end step, Dragonhawk deals 2 damage to each opponent for each of those cards that are still exiled.", false).unwrap();
    let restored: ironsmith_compiled_artifact::CompiledCardArtifact = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let roundtrip = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let charlie = PlayerId::from_index(2);
    for definition in [direct, roundtrip] {
        for extra in 0..=2 { for case in ["remain", "one leaves", "new incarnation", "leaves after trigger", "source leaves", "source control changes", "zero source", "attack again"] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            game.turn.active_player = alice; game.turn.priority_player = Some(alice);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            let big = CardBuilder::new(CardId::new(), "Own large creature").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 100)).build();
            for _ in 0..extra { game.create_object_from_card(&big, alice, Zone::Battlefield); }
            game.create_object_from_card(&big, bob, Zone::Battlefield);
            let small = CardBuilder::new(CardId::new(), "Small creature").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 100)).build();
            game.create_object_from_card(&small, alice, Zone::Battlefield);
            let decoy = CardBuilder::new(CardId::new(), "Noncreature power decoy").card_types(vec![CardType::Artifact]).power_toughness(PowerToughness::fixed(9, 100)).build();
            game.create_object_from_card(&decoy, alice, Zone::Battlefield);
            let library = CardBuilder::new(CardId::new(), "Batch artifact").card_types(vec![CardType::Artifact]).mana_cost(ironsmith::mana::ManaCost::new()).build();
            for _ in 0..8 { game.create_object_from_card(&library, alice, Zone::Library); }
            let foreign = game.create_object_from_card(&library, alice, Zone::Exile);
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let source = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Dragonhawk, Fate's Tempest").unwrap();
            assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            assert_eq!(queue.entries.len(), 1);
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            if case == "zero source" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let batch: Vec<_> = game.exile.iter().copied().filter(|id| *id != foreign).collect();
            let initial_count = extra + usize::from(case != "zero source");
            assert_eq!(batch.len(), initial_count, "only own creatures with current power >= 4 determine X");
            assert_eq!(game.effect_store.delayed_triggers.len(), 1);
            assert!(batch.is_empty() || game.effect_store.delayed_triggers[0].tagged_objects.values().any(|snapshots| snapshots.len() == batch.len() && snapshots.iter().all(|snapshot| batch.contains(&snapshot.object_id))), "the delayed trigger must retain the exact exile result incarnations");
            assert_eq!(game.player(alice).unwrap().library.len(), 8-initial_count);
            let legal = ironsmith::decision::compute_legal_actions(&game, alice).unwrap();
            for card in &batch { assert!(legal.iter().any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if spell_id == card)), "the batch has a real play permission"); }
            assert!(!legal.iter().any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == foreign)));
            let mut second_batch = Vec::new();
            if case == "attack again" {
                let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::combat::CreatureAttackedEvent::new(source, ironsmith::events::combat::AttackEventTarget::Player(bob)), ironsmith::ProvNodeId::default());
                for entry in ironsmith::triggers::check_triggers(&game, &event) { queue.add(entry); }
                assert_eq!(queue.entries.len(), 1);
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                second_batch = game.exile.iter().copied().filter(|id| *id != foreign && !batch.contains(id)).collect();
                assert_eq!(second_batch.len(), initial_count);
                game.move_object_by_effect(batch[0], Zone::Hand).unwrap();
            }
            if matches!(case, "one leaves" | "new incarnation") {
                let moved = game.move_object_by_effect(batch[0], Zone::Hand).unwrap();
                if case == "new incarnation" { game.move_object_by_effect(moved, Zone::Exile).unwrap(); }
            }
            if case == "source leaves" { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            if case == "source control changes" { game.set_current_controller(source, bob).unwrap(); }
            if matches!(case, "source leaves" | "source control changes" | "attack again") {
                let legal = ironsmith::decision::compute_legal_actions(&game, alice).unwrap();
                for card in batch.iter().chain(&second_batch).filter(|id| game.object(**id).is_some_and(|object| object.zone == Zone::Exile)) {
                    assert!(legal.iter().any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if spell_id == card)), "{case}: the original controller retains every batch's play permission until its deadline");
                }
            }
            // An opponent's end step is not the registered controller's next end step.
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(bob), ironsmith::ProvNodeId::default());
            assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
            game.turn.active_player = alice;
            game.turn.phase = ironsmith::game_state::Phase::Ending;
            game.turn.step = Some(ironsmith::game_state::Step::End);
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(alice), ironsmith::ProvNodeId::default());
            for entry in ironsmith::triggers::check_delayed_triggers(&mut game, &event) { queue.add(entry); }
            assert_eq!(queue.entries.len(), 1 + usize::from(case == "attack again"), "{case}");
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert!(game.stack[0].targets.is_empty(), "the delayed damage doesn't target opponents");
            if case == "leaves after trigger" { game.move_object_by_effect(batch[0], Zone::Hand).unwrap(); }
            while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            let remaining = initial_count + second_batch.len() - usize::from(matches!(case, "one leaves" | "new incarnation" | "leaves after trigger" | "attack again"));
            assert_eq!(game.player(alice).unwrap().life, 20, "{case}");
            for player in [bob, charlie] { assert_eq!(game.player(player).unwrap().life, 20 - 2*remaining as i32, "extra={extra}, {case}"); }
        } }
    }
}
