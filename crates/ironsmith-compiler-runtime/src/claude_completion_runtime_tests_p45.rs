//! Claude package 45 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::target::ChooseSpec;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

fn p45_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

#[test]
fn flame_discharge_reads_modified_creatures_frozen_as_the_spell_was_cast() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::snapshot::ObjectSnapshot;
    let definition = p45_compile("Flame Discharge",
        "Mana cost: {X}{R}\nType: Instant\nFlame Discharge deals X damage to target creature or planeswalker. If you controlled a modified creature as you cast this spell, it deals X plus 2 damage instead. (Equipment, Auras you control, and counters are modifications.)");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = CardBuilder::new(CardId::new(), "Damage recipient")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(9, 9)).build();
    for cast_time_modified in [false, true] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let target = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        // Alice's creature is modified *now*, but only the cast-time snapshot
        // decides the branch.
        let mine = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.add_counters(mine, ironsmith::object::CounterType::PlusOnePlusOne, 1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = EffectContext::new_default(source, alice)
            .with_x(3)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        let frozen = if cast_time_modified {
            vec![ObjectSnapshot::from_object_id(&game, mine).unwrap()]
        } else {
            Vec::new()
        };
        ctx.set_tagged_objects(ironsmith_core::CAST_MODIFIED_CREATURES_TAG, frozen);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(game.damage_on(target), if cast_time_modified { 5 } else { 3 });
    }
}

/// CR 614.10: Fasting's "If you would begin your draw step, you may skip that
/// step instead. If you do, you gain 2 life." is offered by the turn runner
/// before the draw step begins.
#[test]
fn fasting_offers_to_skip_the_draw_step_and_gains_life_if_you_do() {
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let definition = p45_compile("Fasting",
        "Mana cost: {W}\nType: Enchantment\nAt the beginning of your upkeep, put a hunger counter on this enchantment. Then destroy this enchantment if it has five or more hunger counters on it.\nIf you would begin your draw step, you may skip that step instead. If you do, you gain 2 life.\nWhen you draw a card, destroy this enchantment.");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::MaySkipDrawStepWithFollowup)),
        "{:?}", definition.abilities);
    let alice = PlayerId::from_index(0);
    let filler = CardBuilder::new(CardId::new(), "Library card")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    for accept in [true, false] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        for _ in 0..3 {
            game.create_object_from_card(&filler, alice, Zone::Library);
        }
        let fasting = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Draw);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let action = runner.advance(&mut game, &mut queue).unwrap();
        let TurnAction::Decision(DecisionContext::Boolean(prompt)) = action else {
            panic!("expected the draw-step skip offer, got {action:?}");
        };
        assert_eq!((prompt.player, prompt.source), (alice, Some(fasting)));
        runner.respond_boolean(accept);
        let action = runner.advance(&mut game, &mut queue).unwrap();
        let player = game.player(alice).unwrap();
        if accept {
            assert!(matches!(action, TurnAction::Continue), "{action:?}");
            assert_eq!(player.life, 22, "if you do, you gain 2 life");
            assert!(player.hand.is_empty(), "the skipped step draws nothing");
            assert!(!matches!(runner.state(), TurnRunnerState::DrawPriority));
        } else {
            assert!(matches!(action, TurnAction::RunPriority), "{action:?}");
            assert_eq!(player.life, 20);
            assert_eq!(player.hand.len(), 1, "declined: the draw step happens");
        }
    }
}

/// Flaming Gambit: the damaged player (or the damaged planeswalker's
/// controller) may choose a creature they control as the spell resolves, and
/// the damage is dealt once — to that creature, or else to the target.
#[test]
fn flaming_gambit_lets_the_damaged_player_redirect_the_damage_to_their_creature() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    struct Redirect { accept: bool, choosers: Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Redirect {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.choosers.push(context.player);
            self.accept
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.choosers.push(context.player);
            context.candidates.iter().filter(|candidate| candidate.legal).take(1).map(|candidate| candidate.id).collect()
        }
    }
    let definition = p45_compile("Flaming Gambit",
        "Mana cost: {X}{R}\nType: Instant\nFlaming Gambit deals X damage to target player or planeswalker. That player or that planeswalker's controller may choose a creature they control and have Flaming Gambit deal that damage to it instead.\nFlashback {X}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = CardBuilder::new(CardId::new(), "Bob's creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(5, 5)).build();
    for accept in [true, false] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mine = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let theirs = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut dm = Redirect { accept, choosers: Vec::new() };
        let mut ctx = EffectContext::new(source, alice, &mut dm)
            .with_x(3)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(!dm.choosers.is_empty() && dm.choosers.iter().all(|player| *player == bob),
            "the damaged player chooses: {:?}", dm.choosers);
        assert_eq!(game.damage_on(mine), 0, "only the damaged player's creatures qualify");
        if accept {
            assert_eq!(game.damage_on(theirs), 3);
            assert_eq!(game.player(bob).unwrap().life, 20, "the damage was redirected, not doubled");
        } else {
            assert_eq!(game.damage_on(theirs), 0);
            assert_eq!(game.player(bob).unwrap().life, 17);
        }
    }
}

fn p45_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() { walk(effect, &mut out); }
    }
    out
}

/// "Whenever a card is drawn this way": one reflexive trigger follows each
/// single-card draw — yours and each other player's optional draw — and each
/// targets an opponent chosen at random.
#[test]
fn explosion_of_riches_creates_one_reflexive_trigger_per_card_drawn() {
    let definition = p45_compile("Explosion of Riches",
        "Mana cost: {5}{R}\nType: Sorcery\nDraw a card, then each other player may draw a card. Whenever a card is drawn this way, Explosion of Riches deals 5 damage to target opponent chosen at random from among your opponents.");
    let effects = p45_effects(&definition);
    let reflexive: Vec<_> = effects.iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::ReflexiveTriggerEffect>())
        .collect();
    assert_eq!(reflexive.len(), 2, "your draw and each other player's draw: {reflexive:?}");
    for trigger in &reflexive {
        let debug = format!("{trigger:?}");
        assert!(debug.contains("DealDamageEffect") && debug.contains("random: true"), "{debug}");
        assert!(debug.contains("Opponent"), "{debug}");
    }
    let mays = effects.iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::MayEffect>())
        .collect::<Vec<_>>();
    assert!(mays.iter().any(|may| format!("{may:?}").contains("ReflexiveTriggerEffect")),
        "the other players' trigger is inside their optional draw: {mays:?}");
}

/// Extrapolate the Impossible: two distinctly named owned cards from outside
/// the game are revealed, an opponent picks one, and only that card goes to
/// your hand.
#[test]
fn extrapolate_the_impossible_moves_the_opponents_pick_of_two_outside_game_cards() {
    use ironsmith::effects::{EffectContext, execute_effect};
    struct Answers { opponent_picks: Vec<PlayerId>, alice: PlayerId }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            true
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let legal: Vec<_> = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
            if context.player == self.alice {
                legal.into_iter().take(context.min.max(2)).collect()
            } else {
                self.opponent_picks.push(context.player);
                legal.into_iter().rev().take(1).collect()
            }
        }
    }
    let definition = p45_compile("Extrapolate the Impossible",
        "Mana cost: {1}{B}\nType: Sorcery\nYou may reveal exactly two cards you own with different names from outside the game. An opponent chooses one of them. You put that card into your hand.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("OutsideGame") && debug.contains("distinct_names: true"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Instant]).build();
    let first = game.create_object_from_card(&card("Wish A"), alice, Zone::OutsideGame);
    let second = game.create_object_from_card(&card("Wish B"), alice, Zone::OutsideGame);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut dm = Answers { opponent_picks: Vec::new(), alice };
    let mut ctx = EffectContext::new(source, alice, &mut dm);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(dm.opponent_picks, vec![bob], "the opponent chooses one of them");
    let hand = &game.player(alice).unwrap().hand;
    assert_eq!(hand.len(), 1, "only the chosen card is put into your hand");
    let in_hand = hand.iter().next().copied().unwrap();
    let name = game.object(in_hand).unwrap().name.to_string();
    assert!(name == "Wish A" || name == "Wish B", "{name}");
    let _ = (first, second);
}
