//! Claude package 22 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::decision::DecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::types::CardType;
use ironsmith::{GameState, PlayerId, Zone};

fn p22_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p22_visit(child, f));
}

fn p22_creature(game: &mut GameState, name: &str, owner: PlayerId, power: i32, toughness: i32) -> ironsmith::ids::ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

/// Chooses every offered recipient, then puts the whole distributable amount
/// on the last one.
struct P22AllOnLast;

impl DecisionMaker for P22AllOnLast {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ids::ObjectId> {
        ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect()
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(ironsmith::game_state::Target, u32)> {
        ctx.targets.last().map(|target| vec![(target.target, ctx.total)]).unwrap_or_default()
    }
}

#[test]
fn p22_stumpsquall_hydra_always_gives_itself_a_counter_among_optional_commanders() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::object::CounterType;
    let definition = compile_to_runtime_definition("Stumpsquall Hydra",
        "Mana cost: {X}{G}{G}{G}\nType: Creature — Hydra\nPower/Toughness: 0/0\nWhen this creature enters, distribute X +1/+1 counters among it and any number of commanders.", false)
        .expect("Stumpsquall Hydra should compile");
    let mut put = None;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() {
                p22_visit(effect, &mut |effect| {
                    if let Some(found) = effect.downcast_ref::<ironsmith::effects::PutCountersEffect>() {
                        put = Some(found.clone());
                    }
                });
            }
        }
    }
    let put = put.expect("the enters trigger distributes counters");
    assert!(put.distributed, "{put:?}");
    assert!(matches!(put.required_recipient.as_ref().map(|spec| spec.base()),
        Some(ironsmith::target::ChooseSpec::Source)), "the Hydra is a mandatory recipient: {put:?}");
    assert!(!put.target.is_target(), "commanders are chosen at resolution, not targeted: {put:?}");
    assert!(format!("{:?}", put.target).contains("is_commander: true"), "{put:?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hydra = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut fixed = put.clone();
    fixed.amount = ironsmith::effect::Value::Fixed(3);

    // No commander on the battlefield: every counter still lands on the Hydra.
    let mut ctx = EffectContext::new_default(hydra, alice);
    fixed.execute(&mut game, &mut ctx).expect("distribution resolves");
    drop(ctx);
    assert_eq!(game.counter_count(hydra, CounterType::PlusOnePlusOne), 3);

    // With a commander, even a division favouring it leaves one on the Hydra.
    let commander = p22_creature(&mut game, "Commander", bob, 2, 2);
    game.set_commander(commander);
    let bystander = p22_creature(&mut game, "Bystander", bob, 2, 2);
    let mut all_on_last = P22AllOnLast;
    let mut ctx = EffectContext::new_default(hydra, alice).with_decision_maker(&mut all_on_last);
    fixed.execute(&mut game, &mut ctx).expect("distribution resolves");
    drop(ctx);
    assert_eq!(game.counter_count(hydra, CounterType::PlusOnePlusOne), 4, "the Hydra keeps its mandatory counter");
    assert_eq!(game.counter_count(commander, CounterType::PlusOnePlusOne), 2);
    assert_eq!(game.counter_count(bystander, CounterType::PlusOnePlusOne), 0, "non-commanders are not recipients");
}

#[test]
fn p22_sworn_defender_sets_its_stats_from_the_combat_partners_opposite_stats() {
    let definition = compile_to_runtime_definition("Sworn Defender",
        "Mana cost: {2}{W}{W}\nType: Creature — Human Knight\nPower/Toughness: 1/2\n{1}: This creature's power becomes the toughness of target creature blocking or being blocked by this creature minus 1 until end of turn, and its toughness becomes 1 plus the power of that creature until end of turn.", false)
        .expect("Sworn Defender should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("the {1} ability");
    let debug = format!("{:?}", activated.effects);
    assert!(debug.contains("in_combat_with_source: true"),
        "the target is a creature blocking or blocked by this creature: {debug}");
    let mut continuous = Vec::new();
    for effect in activated.effects.all_effects() {
        p22_visit(effect, &mut |effect| {
            let text = format!("{effect:?}");
            if (text.starts_with("Effect(ApplyContinuousEffect") || text.starts_with("ApplyContinuousEffect"))
                && (text.contains("SetPower") || text.contains("SetToughness"))
            {
                continuous.push(text);
            }
        });
    }
    assert!(!continuous.is_empty(), "{debug}");
    assert_eq!(continuous.len(), 2, "one setting effect per characteristic: {continuous:?}");
    assert!(continuous.iter().all(|text| {
        let head = &text[..text.find("modification").unwrap_or(text.len())];
        head.contains("Source") && !head.contains("Tagged")
    }), "both halves change this creature, not the partner: {continuous:?}");
    let all = continuous.join("\n");
    assert!(all.contains("ToughnessOf") && all.contains("Fixed(-1)"), "power = partner toughness minus 1: {all}");
    assert!(all.contains("PowerOf") && all.contains("Fixed(1)"), "toughness = 1 plus partner power: {all}");
    assert!(!all.contains("SourcePower") && !all.contains("SourceToughness"), "{all}");
}

#[test]
fn p22_spoils_of_war_defines_x_as_the_chosen_opponents_graveyard_count() {
    use ironsmith::effects::EffectContext;
    let definition = compile_to_runtime_definition("Spoils of War",
        "Mana cost: {X}{B}\nType: Sorcery\nX is the number of artifact and/or creature cards in an opponent's graveyard as you cast this spell.\nDistribute X +1/+1 counters among any number of target creatures.", false)
        .expect("Spoils of War should compile");
    let mut maximum = None;
    let mut minimum = None;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind {
            if let Some(value) = static_ability.this_spell_x_maximum_value() {
                assert!(ability.functional_zones.contains(&Zone::Stack), "{ability:?}");
                maximum = Some(value);
            }
            if let Some(value) = static_ability.this_spell_x_minimum_value() {
                assert!(ability.functional_zones.contains(&Zone::Stack), "{ability:?}");
                minimum = Some(value);
            }
        }
    }
    let (maximum, minimum) = (maximum.expect("X upper bound"), minimum.expect("X lower bound"));
    let put = definition.spell_effect.as_ref().unwrap().all_effects().into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PutCountersEffect>().cloned())
        .expect("distribution");
    assert!(put.distributed && put.target.is_target(), "{put:?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature_card = CardBuilder::new(CardId::new(), "Dead Creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let artifact_card = CardBuilder::new(CardId::new(), "Broken Relic")
        .card_types(vec![CardType::Artifact]).build();
    let land_card = CardBuilder::new(CardId::new(), "Spent Land")
        .card_types(vec![CardType::Land]).build();
    game.create_object_from_card(&creature_card, bob, Zone::Graveyard);
    game.create_object_from_card(&artifact_card, bob, Zone::Graveyard);
    game.create_object_from_card(&land_card, bob, Zone::Graveyard);
    // The caster's own graveyard does not count.
    game.create_object_from_card(&creature_card, alice, Zone::Graveyard);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let ctx = EffectContext::new_default(spell, alice);
    let resolve = |value: &ironsmith::effect::Value| ironsmith::effects::helpers::resolve_value(&game, value, &ctx).unwrap();
    assert_eq!(resolve(&maximum), 2, "{maximum:?}");
    assert_eq!(resolve(&minimum), 2, "X is defined, not chosen: {minimum:?}");
}

#[test]
fn p22_spire_phantasm_draws_only_after_a_recorded_correct_draft_guess() {
    use ironsmith::effects::EffectContext;
    let definition = compile_to_runtime_definition("Spire Phantasm",
        "Mana cost: {2}{U}{U}\nType: Creature — Gargoyle Illusion\nPower/Toughness: 2/2\nReveal this card as you draft it. The next time a player drafts a card from this booster pack, guess that card's name. Then that player reveals the drafted card.\nFlying\nWhen this creature enters, if you guessed correctly for a card named Spire Phantasm, draw a card.", false)
        .expect("Spire Phantasm should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("the enters trigger");
    let debug = format!("{trigger:?}");
    assert!(debug.contains("DraftCorrectGuessCount") && debug.to_ascii_lowercase().contains("spire phantasm"),
        "the intervening if reads the recorded draft guess: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let phantasm = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let guessed = ironsmith::effect::Value::DraftCorrectGuessCount { card_name: "spire phantasm".into() };
    let read = |game: &GameState| {
        let ctx = EffectContext::new_default(phantasm, alice);
        ironsmith::effects::helpers::resolve_value(game, &guessed, &ctx).unwrap()
    };
    assert_eq!(read(&game), 0, "outside a draft nobody guessed");
    game.record_draft_correct_guess(bob, "Spire Phantasm");
    assert_eq!(read(&game), 0, "another player's correct guess is not yours");
    game.record_draft_correct_guess(alice, "SPIRE PHANTASM");
    assert_eq!(read(&game), 1);
}

#[test]
fn p22_stronghold_gambit_reveals_private_hand_choices_and_lands_the_cheapest_creature() {
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = compile_to_runtime_definition("Stronghold Gambit",
        "Mana cost: {1}{R}\nType: Sorcery\nEach player chooses a card in their hand. Then each player reveals their chosen card. The owner of each creature card revealed this way with the lowest mana value puts it onto the battlefield.", false)
        .expect("Stronghold Gambit should compile");
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{program:?}");
    assert!(debug.contains("ForPlayersEffect") && debug.contains("chooser: IteratedPlayer"),
        "each player makes their own choice: {debug}");
    assert!(debug.contains("RevealTaggedEffect"), "the chosen cards are revealed together: {debug}");
    assert!(debug.contains("ReturnAllToBattlefieldEffect") && debug.contains("LeastManaValue"),
        "the lowest mana value among the revealed creature cards: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = |name: &str, generic: u8| CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic), ManaSymbol::Red]))
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let expensive = game.create_object_from_card(&creature("Expensive", 3), alice, Zone::Hand);
    let cheap = game.create_object_from_card(&creature("Cheap", 1), bob, Zone::Hand);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = EffectContext::new_default(spell, alice);
    for effect in program.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let on_battlefield = |game: &GameState, name: &str| game.battlefield.iter().copied()
        .filter_map(|id| game.object(id)).find(|object| object.name == name).map(|object| object.id);
    let landed = on_battlefield(&game, "Cheap").expect("the cheapest revealed creature enters");
    assert_eq!(game.controller_of(game.object(landed).unwrap()), bob, "under its owner's control");
    assert!(on_battlefield(&game, "Expensive").is_none(), "the costlier creature stays in hand");
    assert!(game.player(alice).unwrap().hand.contains(&expensive));
    let _ = cheap;
}

#[test]
fn p22_stromgald_spy_reveals_the_defending_hand_while_it_remains_on_the_battlefield() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    let definition = compile_to_runtime_definition("Stromgald Spy",
        "Mana cost: {3}{B}\nType: Creature — Human Rogue\nPower/Toughness: 2/4\nWhenever this creature attacks and isn't blocked, you may have defending player play with their hand revealed for as long as this creature remains on the battlefield. If you do, this creature assigns no combat damage this turn.", false)
        .expect("Stromgald Spy should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("the unblocked-attack trigger");
    let debug = format!("{trigger:?}");
    assert!(debug.contains("May"), "the reveal is optional: {debug}");
    assert!(debug.contains("Defending"), "{debug}");
    let mut reveal = None;
    for effect in trigger.effects.all_effects() {
        p22_visit(effect, &mut |effect| {
            if let Some(found) = effect.downcast_ref::<ironsmith::effects::PlayWithHandRevealedEffect>() {
                reveal = Some(found.clone());
            }
        });
    }
    let reveal = reveal.expect("a player-scoped hand reveal");
    assert_eq!(reveal.player, ironsmith::target::PlayerFilter::Defending);
    let after_reveal = &debug[debug.find("PlayWithHandRevealed").unwrap()..];
    assert!(after_reveal.contains("AssignNoCombatDamage") || after_reveal.contains("PreventAllCombatDamage"),
        "if you do, it assigns no combat damage: {after_reveal}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let spy = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(spy, alice);
    ctx.combat.defending_player = Some(bob);
    reveal.execute(&mut game, &mut ctx).expect("the reveal resolves");
    drop(ctx);
    assert!(game.hand_revealed_by_effect(bob), "the defending player's hand is revealed");
    assert!(!game.hand_revealed_by_effect(alice), "only the defending player's hand");
    game.move_object(spy, Zone::Graveyard, ironsmith::events::EventCause::effect());
    assert!(!game.hand_revealed_by_effect(bob), "the reveal ends once the Spy leaves the battlefield");
}

#[test]
fn p22_tapestry_warden_makes_high_toughness_creatures_station_with_toughness() {
    use ironsmith::effects::EffectContext;
    let definition = compile_to_runtime_definition("Tapestry Warden",
        "Mana cost: {3}{G}\nType: Artifact Creature — Robot Soldier\nPower/Toughness: 3/4\nVigilance\nEach creature you control with toughness greater than its power assigns combat damage equal to its toughness rather than its power.\nEach creature you control with toughness greater than its power stations permanents using its toughness rather than its power.", false)
        .expect("Tapestry Warden should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("stations permanents using its toughness rather than its power"),
        "the station rule is granted to the qualifying creatures: {debug}");
    assert!(debug.contains("ToughnessGreaterThanPower"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let wall = p22_creature(&mut game, "Sturdy", alice, 1, 4);
    let brute = p22_creature(&mut game, "Brute", alice, 4, 1);
    let theirs = p22_creature(&mut game, "Their Sturdy", bob, 1, 4);
    let station = |game: &GameState, id| {
        let value = ironsmith::effect::Value::StationContributionOf(Box::new(
            ironsmith::target::ChooseSpec::SpecificObject(id),
        ));
        let ctx = EffectContext::new_default(id, alice);
        ironsmith::effects::helpers::resolve_value(game, &value, &ctx).unwrap()
    };
    assert_eq!(station(&game, wall), 1, "without the Warden a creature stations with its power");
    let _warden = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert_eq!(station(&game, wall), 4, "toughness greater than power: stations with toughness");
    assert_eq!(station(&game, brute), 4, "power greater than toughness keeps its power");
    assert_eq!(station(&game, theirs), 1, "only creatures you control");
}

#[test]
fn p22_sen_triplets_locks_out_reveals_and_opens_the_chosen_opponents_hand_this_turn() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    let definition = compile_to_runtime_definition("Sen Triplets",
        "Mana cost: {2}{W}{U}{B}\nType: Legendary Artifact Creature — Human Wizard\nPower/Toughness: 3/3\nAt the beginning of your upkeep, choose target opponent. This turn, that player can't cast spells or activate abilities and plays with their hand revealed. You may play lands and cast spells from that player's hand this turn.", false)
        .expect("Sen Triplets should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("the upkeep trigger");
    let mut cants = Vec::new();
    let mut reveal = None;
    let mut grant = None;
    for effect in trigger.effects.all_effects() {
        p22_visit(effect, &mut |effect| {
            if let Some(cant) = effect.downcast_ref::<ironsmith::effects::CantEffect>() {
                cants.push(format!("{:?}", cant.restriction));
            }
            if let Some(found) = effect.downcast_ref::<ironsmith::effects::PlayWithHandRevealedEffect>() {
                reveal = Some(found.clone());
            }
            if let Some(found) = effect.downcast_ref::<ironsmith::effects::GrantBySpecEffect>() {
                grant = Some(found.clone());
            }
        });
    }
    assert!(cants.iter().any(|text| text.contains("CastSpells")), "{cants:?}");
    assert!(cants.iter().any(|text| text.contains("ActivateAbilities")), "{cants:?}");
    let reveal = reveal.expect("a this-turn hand reveal");
    assert_eq!(reveal.duration, ironsmith_core::HandRevealDuration::ThisTurn);
    assert!(matches!(reveal.player,
        ironsmith::target::PlayerFilter::Target(_) | ironsmith::target::PlayerFilter::AliasedTarget(_)),
        "the revealed hand is the chosen opponent's: {:?}", reveal.player);
    let grant = grant.expect("a play permission over that hand");
    assert_eq!(grant.spec.zone, Zone::Hand);
    assert!(matches!(grant.spec.filter.owner,
        Some(ironsmith::target::PlayerFilter::Target(_) | ironsmith::target::PlayerFilter::AliasedTarget(_))),
        "{:?}", grant.spec.filter);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Hand card").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let bobs = game.create_object_from_card(&card, bob, Zone::Hand);
    let alices = game.create_object_from_card(&card, alice, Zone::Hand);
    let triplets = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    // Bind "that player" to Bob, as the chosen target would.
    let mut bound_reveal = reveal.clone();
    bound_reveal.player = ironsmith::target::PlayerFilter::IteratedPlayer;
    let mut bound_grant = grant.clone();
    bound_grant.spec.filter.owner = Some(ironsmith::target::PlayerFilter::IteratedPlayer);
    let mut ctx = EffectContext::new_default(triplets, alice);
    ctx.iteration.iterated_player = Some(bob);
    bound_reveal.execute(&mut game, &mut ctx).expect("the reveal resolves");
    ironsmith::effects::execute_effect(&mut game, &Effect::new(bound_grant), &mut ctx).unwrap();
    drop(ctx);

    assert!(game.hand_revealed_by_effect(bob), "Bob plays with his hand revealed this turn");
    assert!(!game.hand_revealed_by_effect(alice));
    let registry = &game.effect_store.grant_registry;
    assert!(!registry.granted_play_from_for_card(&game, bobs, Zone::Hand, alice).is_empty(),
        "Alice may cast spells from Bob's hand");
    assert!(registry.granted_play_from_for_card(&game, alices, Zone::Hand, alice).is_empty());
    assert!(registry.granted_play_from_for_card(&game, bobs, Zone::Hand, bob).is_empty());
    // The reveal is not source-bound: it ends with the turn, not with the Triplets.
    game.move_object(triplets, Zone::Graveyard, ironsmith::events::EventCause::effect());
    assert!(game.hand_revealed_by_effect(bob));
    game.turn.turn_number += 1;
    assert!(!game.hand_revealed_by_effect(bob), "the reveal lasts only this turn");
}
