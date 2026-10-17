//! Claude package 61 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

use ironsmith::{GameState, PlayerId, Zone};

const FORAGING_WICKERMAW: &str = "Mana cost: {2}\nType: Artifact Creature — Scarecrow\nPower/Toughness: 1/3\nWhen this creature enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)\n{1}: Add one mana of any color. This creature becomes that color until end of turn. Activate only once each turn.";

#[test]
fn p61_wickermaw_becomes_the_color_of_the_mana_it_added() {
    let definition = compile_to_runtime_definition("Foraging Wickermaw", FORAGING_WICKERMAW, false)
        .expect("Foraging Wickermaw should compile");
    let ability = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated)
                if format!("{:?}", activated.effects).contains("AddManaOfAnyColorEffect") =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("the mana ability");
    let debug = format!("{:?}", ability.effects);
    assert!(debug.contains("color_from: Effect("), "the become reads the mana's color: {debug}");
    assert!(!debug.contains("ChooseColorEffect"), "no second color choice: {debug}");

    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let wickermaw = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert_eq!(game.current_colors(wickermaw), Some(ironsmith::color::ColorSet::COLORLESS));
    // The default decision maker takes the first offered color (white).
    let mut ctx = ironsmith::effects::EffectContext::new_default(wickermaw, alice);
    for effect in ability.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().mana_pool.white, 1, "one white mana was added");
    assert_eq!(game.current_colors(wickermaw), Some(ironsmith::color::ColorSet::WHITE),
        "the creature becomes the color of that mana");
}

#[test]
fn p61_jarkeld_two_blocked_attackers_trade_their_single_blockers() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::ids::{CardId, ObjectId};
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("General Jarkeld",
        "Mana cost: {3}{W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 1/2\n{T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step.", false)
        .expect("General Jarkeld should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ExchangeBlocksEffect") && debug.contains("between_attackers: true"),
        "one attacker-side block exchange: {debug}");

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let bear = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let first = game.create_object_from_card(&bear("A1"), alice, Zone::Battlefield);
    let second = game.create_object_from_card(&bear("A2"), alice, Zone::Battlefield);
    let only_first = game.create_object_from_card(&bear("B1"), bob, Zone::Battlefield);
    let only_second = game.create_object_from_card(&bear("B2"), bob, Zone::Battlefield);
    let both = game.create_object_from_card(&bear("B3"), bob, Zone::Battlefield);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo { creature: first, target: AttackTarget::Player(bob) });
    combat.attackers.push(AttackerInfo { creature: second, target: AttackTarget::Player(bob) });
    combat.blockers.insert(first, vec![only_first, both]);
    combat.blockers.insert(second, vec![only_second, both]);
    combat.blocked_attackers.insert(first);
    combat.blocked_attackers.insert(second);
    game.combat = Some(combat);
    game.refresh_continuous_state().unwrap();
    let effect = ironsmith::effects::ExchangeBlocksEffect::new(ironsmith::target::ChooseSpec::All(
        ironsmith::target::ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .controlled_by(ironsmith::target::PlayerFilter::Specific(alice)),
    ))
    .between_attackers();
    let mut ctx = ironsmith::effects::EffectContext::new_default(ObjectId::from_raw(9_999), alice);
    ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::new(effect), &mut ctx).unwrap();
    let combat = game.combat.as_ref().unwrap();
    assert_eq!(combat.blockers.get(&first), Some(&vec![both, only_second]),
        "the shared blocker stays; the other attacker's single blocker moves over");
    assert_eq!(combat.blockers.get(&second), Some(&vec![both, only_first]));
    assert!(combat.blocked_attackers.contains(&first) && combat.blocked_attackers.contains(&second));
}

/// Answers yes/no prompts from a queue (then "no"); other choices take defaults.
struct P61Answers(std::collections::VecDeque<bool>);

impl ironsmith::decision::DecisionMaker for P61Answers {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.0.pop_front().unwrap_or(false)
    }
}

#[test]
fn p61_game_of_chaos_winner_decides_and_stakes_double() {
    let definition = compile_to_runtime_definition("Game of Chaos",
        "Mana cost: {R}{R}{R}\nType: Sorcery\nFlip a coin. If you win the flip, you gain 1 life and target opponent loses 1 life, and you decide whether to flip again. If you lose the flip, you lose 1 life and that opponent gains 1 life, and that player decides whether to flip again. Double the life stakes with each flip.", false)
        .expect("Game of Chaos should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("CoinFlipLifeStakesEffect") && debug.contains("initial_stakes: 1"), "{debug}");
    assert!(debug.contains("Opponent"), "the stakes are against target opponent: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.new_object_id();
    // Alice calls heads (first option): heads wins, then tails loses.
    game.force_next_coin_flip(ironsmith::CoinFace::Heads);
    game.force_next_coin_flip(ironsmith::CoinFace::Tails);
    let effect = ironsmith::effects::CoinFlipLifeStakesEffect::new(
        ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(bob)),
        1,
    );
    // Alice (the first winner) flips again; Bob (the second winner) stops.
    let mut answers = P61Answers([true, false].into_iter().collect());
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut answers);
    ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::new(effect), &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 20 + 1 - 2, "won 1, then lost the doubled 2");
    assert_eq!(game.player(bob).unwrap().life, 20 - 1 + 2);
}

#[test]
fn p61_ghostly_flame_red_source_is_colorless_only_as_a_damage_source() {
    use ironsmith::ability::{Ability, ProtectionFrom};
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::CardType;
    use ironsmith::ColorSet;
    let definition = compile_to_runtime_definition("Ghostly Flame",
        "Mana cost: {B}{R}\nType: Enchantment\nBlack and/or red permanents and spells are colorless sources of damage.", false)
        .expect("Ghostly Flame should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DamageSourcesAreColorless"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let red = CardBuilder::new(CardId::new(), "Red source").card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::RED).power_toughness(PowerToughness::fixed(2, 2)).build();
    let red_source = game.create_object_from_card(&red, alice, Zone::Battlefield);
    let warded = CardBuilder::new(CardId::new(), "Warded").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4)).build();
    let warded = game.create_object_from_card(&warded, bob, Zone::Battlefield);
    game.object_mut(warded).unwrap().abilities = std::sync::Arc::new(vec![
        Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED))),
    ]);
    game.refresh_continuous_state().unwrap();
    let damage = Effect::new(ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificObject(warded)));
    let mut ctx = ironsmith::effects::EffectContext::new_default(red_source, alice);
    ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
    assert_eq!(game.damage_on(warded), 0, "protection from red prevents a red source");

    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let mut ctx = ironsmith::effects::EffectContext::new_default(red_source, alice);
    ironsmith::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
    assert_eq!(game.damage_on(warded), 2, "a colorless source of damage gets through protection from red");
    assert_eq!(game.current_colors(red_source), Some(ColorSet::RED), "it stays red otherwise");
}

const GIANT_OYSTER: &str = "Mana cost: {2}{U}{U}\nType: Creature — Oyster\nPower/Toughness: 0/3\nYou may choose not to untap this creature during your untap step.\n{T}: For as long as this creature remains tapped, target tapped creature doesn't untap during its controller's untap step, and at the beginning of each of your draw steps, put a -1/-1 counter on that creature. When this creature leaves the battlefield or becomes untapped, remove all -1/-1 counters from the creature.";

#[test]
fn p61_giant_oyster_draw_step_counters_last_while_the_oyster_stays_tapped() {
    let definition = compile_to_runtime_definition("Giant Oyster", GIANT_OYSTER, false)
        .expect("Giant Oyster should compile");
    let ability = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("the tap ability");
    let debug = format!("{:?}", ability.effects);
    assert!(debug.contains("SourceUntaps"), "the untap restriction lasts while tapped: {debug}");
    assert!(debug.contains("WhileSourceTapped") && debug.contains("BeginningOfDrawStep"),
        "a recurring draw-step trigger scoped to the oyster staying tapped: {debug}");
    assert!(debug.contains("RemoveUpToAnyCountersEffect") || debug.contains("RemoveCountersEffect"), "{debug}");
    // The cleanup reads the targeted creature, never a freshly chosen one.
    assert!(!debug.contains("RemoveCountersEffect { counter_type: MinusOneMinusOne, count: CountersOn(Object("),
        "the cleanup must not choose an arbitrary creature: {debug}");

    fn find_schedule(effect: &Effect, out: &mut Vec<ironsmith::effects::ScheduleDelayedTriggerEffect>) {
        if let Some(schedule) = effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()
            && schedule.duration == ironsmith_core::DelayedTriggerDuration::WhileSourceTapped
        {
            out.push(schedule.clone());
        }
        effect.visit_child_effects(&mut |child| find_schedule(child, out));
    }
    let mut schedules = Vec::new();
    for effect in ability.effects.all_effects() {
        find_schedule(effect, &mut schedules);
    }
    let [schedule] = schedules.as_slice() else {
        panic!("one source-tapped registration: {debug}");
    };

    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let oyster = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let schedule = Effect::new(schedule.clone());
    // CR 611.2b: an untapped oyster registers nothing.
    let mut ctx = ironsmith::effects::EffectContext::new_default(oyster, alice);
    ironsmith::effects::execute_effect(&mut game, &schedule, &mut ctx).unwrap();
    assert!(game.effect_store.delayed_triggers.is_empty());
    game.tap(oyster);
    let mut ctx = ironsmith::effects::EffectContext::new_default(oyster, alice);
    ironsmith::effects::execute_effect(&mut game, &schedule, &mut ctx).unwrap();
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(game.effect_store.delayed_triggers[0].while_source_tapped, Some(oyster));
    assert!(!game.effect_store.delayed_triggers[0].one_shot, "it repeats each draw step");
    game.untap(oyster);
    assert!(game.effect_store.delayed_triggers.is_empty(), "untapping the oyster ends the registration");
}
