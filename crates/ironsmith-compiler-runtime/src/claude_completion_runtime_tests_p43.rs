//! Claude package 43 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

fn p43_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

fn p43_walk(effect: &Effect, out: &mut Vec<Effect>) {
    out.push(effect.clone());
    effect.visit_child_effects(&mut |child| p43_walk(child, out));
}

/// Deals `amount` damage to `player` from a fresh instant Bob controls.
fn p43_damage_player(game: &mut GameState, player: PlayerId, amount: i32) {
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(
        &ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::ids::CardId::new(),
            "P43 Damage Source",
        )
        .card_types(vec![ironsmith::CardType::Instant])
        .build(),
        bob,
        Zone::Stack,
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
    ironsmith::effects::execute_effect(
        game,
        &Effect::deal_damage(amount, ChooseSpec::Player(ironsmith::PlayerFilter::Specific(player))),
        &mut ctx,
    )
    .unwrap();
}

const ELDERSCALE_WURM: &str = "Mana cost: {4}{G}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 7/7\nTrample\nWhen this creature enters, if your life total is less than 7, your life total becomes 7.\nAs long as you have 7 or more life, damage that would reduce your life total to less than 7 reduces it to 7 instead.";

#[test]
fn elderscale_wurm_floors_damage_at_seven_only_while_at_seven_or_more() {
    let definition = p43_compile("Elderscale Wurm", ELDERSCALE_WURM);
    let statics = format!("{:?}", definition.abilities);
    assert!(statics.contains("DamageReduceLifeBelow { player: You, floor: 7 }"), "{statics}");

    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 10);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    p43_damage_player(&mut game, alice, 5);
    assert_eq!(game.player(alice).unwrap().life, 7);

    // Below 7 life the floor no longer applies.
    game.player_mut(alice).unwrap().life = 6;
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    p43_damage_player(&mut game, alice, 3);
    assert_eq!(game.player(alice).unwrap().life, 3);
}

const EKUNDU_CYCLOPS: &str = "Mana cost: {3}{R}\nType: Creature — Cyclops\nPower/Toughness: 3/4\nIf a creature you control attacks, this creature also attacks if able.";

#[test]
fn ekundu_cyclops_must_attack_when_a_creature_you_control_attacks() {
    let definition = p43_compile("Ekundu Cyclops", EKUNDU_CYCLOPS);
    let abilities = format!("{:?}", definition.abilities);
    assert!(abilities.contains("ConditionalAttackRequirement"), "{abilities}");
    let (trigger, required) = abilities
        .split_once("ConditionalAttackRequirement { trigger: ")
        .and_then(|(_, rest)| rest.split_once(", required: "))
        .expect("trigger and required filters");
    assert!(trigger.contains("controller: Some(You)"), "{trigger}");
    assert!(trigger.contains("source: false"), "{trigger}");
    assert!(required.starts_with("ObjectFilter {") && required.contains("source: true"), "{required}");
}

fn p43_creature(name: &str, power: i32, toughness: i32) -> ironsmith::card::Card {
    ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), name)
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(power, toughness))
        .build()
}

const DWARVEN_CATAPULT: &str = "Mana cost: {X}{R}\nType: Instant\nDwarven Catapult deals X damage divided evenly, rounded down, among all creatures target opponent controls.";

#[test]
fn dwarven_catapult_divides_x_evenly_among_the_opponents_creatures() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    let definition = p43_compile("Dwarven Catapult", DWARVEN_CATAPULT);
    let effects = format!("{:?}", definition.spell_effect);
    assert!(effects.contains("DealDistributedDamageEffect"), "{effects}");
    assert!(effects.contains("EvenRoundedDown"), "{effects}");
    assert!(!effects.contains("DealDamageEachEffect"), "{effects}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let theirs: Vec<ObjectId> = (0..3)
        .map(|index| {
            game.create_object_from_card(&p43_creature(&format!("Bob creature {index}"), 5, 5), bob, Zone::Battlefield)
        })
        .collect();
    let mine = game.create_object_from_card(&p43_creature("Alice creature", 5, 5), alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Player(bob)])
        .with_x(7);
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    for creature in theirs {
        assert_eq!(game.damage_on(creature), 2, "7 divided among 3, rounded down");
    }
    assert_eq!(game.damage_on(mine), 0);
    assert_eq!(game.player(bob).unwrap().life, 20);
}

const DUST_OF_MOMENTS: &str = "Mana cost: {2}{W}\nType: Instant\nChoose one —\n• Remove two time counters from each permanent and each suspended card.\n• Put two time counters on each permanent with a time counter on it and each suspended card.";

#[test]
fn dust_of_moments_modes_cover_permanents_and_suspended_cards_in_one_action() {
    let definition = p43_compile("Dust of Moments", DUST_OF_MOMENTS);
    let effects = format!("{:?}", definition.spell_effect);
    // Both modes keep one counter action whose holder set is the union of
    // permanents and suspended (exiled, suspend, time-counter) cards.
    assert_eq!(effects.matches("Suspend").count() >= 2, true, "{effects}");
    assert!(effects.contains("Time"), "{effects}");
    assert!(effects.contains("Exile"), "{effects}");
    assert!(!effects.contains("Unsupported"), "{effects}");
}

const EARTHSHAPE: &str = "Mana cost: {2}{W}\nType: Instant\nEarthbend 3. Then each creature you control with power less than or equal to that land's power gains hexproof and indestructible until end of turn. You gain hexproof until end of turn.";

#[test]
fn earthshape_compares_power_against_the_earthbent_land() {
    let definition = p43_compile("Earthshape", EARTHSHAPE);
    let effects = format!("{:?}", definition.spell_effect);
    assert!(effects.contains("Earthbend"), "{effects}");
    assert!(effects.contains("LessThanOrEqualExpr(PowerOf("), "{effects}");
    assert!(effects.contains("Indestructible"), "{effects}");
    assert!(effects.matches("Hexproof").count() >= 2, "{effects}");
}

const ELEMENTAL_RESONANCE: &str = "Mana cost: {2}{G}{G}\nType: Enchantment — Aura\nEnchant permanent\nAt the beginning of your first main phase, add mana equal to enchanted permanent's mana cost. (Mana cost includes color. If a mana symbol has multiple colors, choose one.)";

#[test]
fn elemental_resonance_adds_the_enchanted_permanents_mana_cost() {
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = p43_compile("Elemental Resonance", ELEMENTAL_RESONANCE);
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        })
        .expect("main-phase trigger");
    let effects = format!("{:?}", trigger.effects);
    assert!(effects.contains("AddManaEqualToManaCostEffect"), "{effects}");

    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    // {X}{2}{G}{W/U}{R/P}: X adds nothing, generic two colorless, the hybrid
    // one of its colors, the Phyrexian symbol its color.
    let host_card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "P43 Host")
        .card_types(vec![ironsmith::CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::White, ManaSymbol::Blue],
            vec![ManaSymbol::Red, ManaSymbol::Life(2)],
        ]))
        .build();
    let host = game.create_object_from_card(&host_card, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
    game.refresh_continuous_state().unwrap();
    let mut ctx = EffectContext::new_default(aura, alice);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let pool = &game.player(alice).unwrap().mana_pool;
    assert_eq!(pool.colorless, 2, "{pool:?}");
    assert_eq!(pool.green, 1, "{pool:?}");
    assert_eq!(pool.red, 1, "{pool:?}");
    assert_eq!(pool.white + pool.blue, 1, "{pool:?}");
    assert_eq!(pool.black, 0, "{pool:?}");
}

const DRAGONS_FIRE: &str = "Mana cost: {1}{R}\nType: Instant\nAs an additional cost to cast this spell, you may reveal a Dragon card from your hand or choose a Dragon you control.\nDragon's Fire deals 3 damage to target creature or planeswalker. If you revealed a Dragon card or chose a Dragon as you cast this spell, Dragon's Fire deals damage equal to the power of that card or creature instead.";

#[test]
fn dragons_fire_is_an_optional_behold_whose_object_sets_the_damage() {
    let definition = p43_compile("Dragon's Fire", DRAGONS_FIRE);
    let optional = format!("{:?}", definition.optional_costs);
    assert!(optional.contains("Behold"), "{optional}");
    assert!(optional.contains("Dragon"), "{optional}");
    // The behold cost keeps the beheld object under its cost tag ...
    assert!(optional.contains("beheld_cost_0"), "{optional}");
    // ... and the replacement damage reads that object's power when the
    // behold label was paid.
    let effects = format!("{:?}", definition.spell_effect);
    assert!(effects.contains("beheld_cost_0"), "{effects}");
    assert!(effects.contains("PowerOf"), "{effects}");
    assert!(effects.contains("Behold"), "{effects}");
}

const DURNAN: &str = "Mana cost: {3}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/3\nWhenever Durnan attacks, look at the top four cards of your library. You may exile a creature card from among them. Put the rest on the bottom of your library in any order. For as long as that card remains exiled, you may cast it. That spell has undaunted. (It costs {1} less to cast for each opponent.)\nChoose a Background (You can have a Background as a second commander.)";

#[test]
fn durnan_exile_permission_carries_undaunted() {
    let definition = p43_compile("Durnan of the Yawning Portal", DURNAN);
    let abilities = format!("{:?}", definition.abilities);
    assert!(abilities.contains("GrantPlayTaggedEffect"), "{abilities}");
    assert!(abilities.contains("spell_cost_reduction_per_opponent: 1"), "{abilities}");
    assert!(abilities.contains("ForAsLongAsExiled"), "{abilities}");
}

const DREAM_DEVOURER: &str = "Mana cost: {1}{B}\nType: Creature — Demon Cleric\nPower/Toughness: 0/3\nEach nonland card in your hand without foretell has foretell. Its foretell cost is equal to its mana cost reduced by {2}. (During your turn, you may pay {2} and exile it from your hand face down. Cast it on a later turn for its foretell cost.)\nWhenever you foretell a card, this creature gets +2/+0 until end of turn.";

#[test]
fn dream_devourer_grants_hand_foretell_and_triggers_on_foretelling() {
    use ironsmith::mana::{ManaCost, ManaSymbol};
    let definition = p43_compile("Dream Devourer", DREAM_DEVOURER);
    let abilities = format!("{:?}", definition.abilities);
    assert!(abilities.contains("ForetellFromCardManaCostReducedBy { reduction: 2 }"), "{abilities}");
    assert!(!abilities.contains("KeywordFallbackText"), "{abilities}");
    assert!(abilities.contains("Foretell"), "{abilities}");

    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "P43 Hand Creature")
        .card_types(vec![ironsmith::CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)], vec![ManaSymbol::Green]]))
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3))
        .build();
    let land = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "P43 Hand Land")
        .card_types(vec![ironsmith::CardType::Land])
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let land_id = game.create_object_from_card(&land, alice, Zone::Hand);
    game.refresh_continuous_state().unwrap();
    assert!(
        ironsmith::special_actions::can_perform_check(
            &ironsmith::SpecialAction::Foretell { card_id: spell_id },
            &game,
            alice,
        )
        .is_ok(),
        "a nonland hand card gains foretell"
    );
    assert!(
        ironsmith::special_actions::can_perform_check(
            &ironsmith::SpecialAction::Foretell { card_id: land_id },
            &game,
            alice,
        )
        .is_err(),
        "lands are not granted foretell"
    );
}

const DONT_BLINK: &str = "Mana cost: {1}{U}\nType: Instant\nUntil end of turn, if one or more creatures would enter from exile or after being cast from exile, their owners shuffle them into their libraries instead.\nCycling {2} ({2}, Discard this card: Draw a card.)";

#[test]
fn dont_blink_registers_a_turn_long_exile_entry_shuffle_replacement() {
    let definition = p43_compile("Don't Blink", DONT_BLINK);
    let mut effects = Vec::new();
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p43_walk(effect, &mut effects);
    }
    let replacement = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RegisterFutureZoneReplacementEffect>())
        .expect("future zone replacement");
    assert_eq!(replacement.replacement_zone, Zone::Library);
    assert_eq!(replacement.to_zone, Some(Zone::Battlefield));
    assert_eq!(replacement.filter.zone, Some(Zone::Exile), "entering from, or cast from, exile");
    assert_eq!(replacement.filter.card_types, vec![ironsmith::CardType::Creature]);
    assert_eq!(replacement.mode, ironsmith::effects::ReplacementApplyMode::UntilEndOfTurn);
}

const DRAIN_POWER: &str = "Mana cost: {U}{U}\nType: Sorcery\nTarget player activates a mana ability of each land they control. Then that player loses all unspent mana and you add the mana lost this way.";

#[test]
fn drain_power_adds_the_mana_its_target_lost() {
    let definition = p43_compile("Drain Power", DRAIN_POWER);
    let mut effects = Vec::new();
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p43_walk(effect, &mut effects);
    }
    let debug = format!("{effects:?}");
    assert!(debug.contains("ActivateManaAbilityOfEachLandEffect"), "{debug}");
    assert!(debug.contains("EmptyManaPoolEffect"), "{debug}");
    let add = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AddManaLostThisWayEffect>())
        .expect("add the mana lost this way");
    assert!(
        matches!(
            add.amount,
            ironsmith::effect::Value::PriorEffectMetric {
                query: ironsmith::effect::PriorEffectMetricQuery {
                    action: Some(ironsmith::effect::PriorEffectAction::ManaLost),
                    ..
                },
                ..
            }
        ),
        "the add reads the bound mana-loss producer: {:?}",
        add.amount
    );
    assert_eq!(add.player, ironsmith::PlayerFilter::You);
}

const DILUVIAN_PRIMORDIAL: &str = "Mana cost: {5}{U}{U}\nType: Creature — Avatar\nPower/Toughness: 5/5\nFlying\nWhen this creature enters, for each opponent, you may cast up to one target instant or sorcery card from that player's graveyard without paying its mana cost. If a spell cast this way would be put into a graveyard, exile it instead.";

#[test]
fn diluvian_primordial_repeats_the_graveyard_cast_for_each_opponent() {
    let definition = p43_compile("Diluvian Primordial", DILUVIAN_PRIMORDIAL);
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        })
        .expect("enters trigger");
    let mut effects = Vec::new();
    for effect in trigger.effects.all_effects() {
        p43_walk(effect, &mut effects);
    }
    let debug = format!("{effects:?}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("filter: Opponent"), "{debug}");
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    let replacement = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RegisterFutureZoneReplacementEffect>())
        .expect("exile-instead replacement on the cast spell");
    assert_eq!(replacement.replacement_zone, Zone::Exile);
    assert_eq!(replacement.to_zone, Some(Zone::Graveyard));
}

const DREAM_LEASH: &str = "Mana cost: {3}{U}{U}\nType: Enchantment — Aura\nEnchant permanent\nYou can't choose an untapped permanent as this spell's target as you cast it.\nYou control enchanted permanent.";

#[test]
fn dream_leash_forbids_untapped_targets_only_while_cast() {
    let definition = p43_compile("Dream Leash", DREAM_LEASH);
    let abilities = format!("{:?}", definition.abilities);
    assert!(abilities.contains("CastTargetRestriction"), "{abilities}");
    // The enchant target itself stays "permanent": untapping before it
    // resolves doesn't make the target illegal (CR 608.2b).
    let attach = format!("{:?}", definition.aura_attach_filter);
    assert!(!attach.contains("tapped: true"), "{attach}");
}

const DROUGHT: &str = "Mana cost: {2}{W}{W}\nType: Enchantment\nAt the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.\nSpells cost an additional \"Sacrifice a Swamp\" to cast for each black mana symbol in their mana costs.\nActivated abilities cost an additional \"Sacrifice a Swamp\" to activate for each black mana symbol in their activation costs.";

#[test]
fn drought_adds_a_swamp_sacrifice_per_black_symbol_to_spells_and_abilities() {
    let definition = p43_compile("Drought", DROUGHT);
    let payloads: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(static_ability) => {
                static_ability.compiled_model().map(|model| model.payload.clone())
            }
            _ => None,
        })
        .filter_map(|payload| match payload {
            ironsmith_core::StaticAbilityPayload::AdditionalSacrificePerManaSymbol {
                activated_abilities,
                symbol,
                sacrifice,
                ..
            } => Some((activated_abilities, symbol, sacrifice)),
            _ => None,
        })
        .collect();
    assert_eq!(payloads.len(), 2, "{:?}", definition.abilities);
    for (activated, symbol, sacrifice) in &payloads {
        assert_eq!(*symbol, ironsmith::mana::ManaSymbol::Black);
        assert_eq!(sacrifice.controller, Some(ironsmith::PlayerFilter::You));
        assert!(format!("{sacrifice:?}").contains("Swamp"), "{sacrifice:?} ({activated})");
    }
    assert!(payloads.iter().any(|(activated, ..)| *activated));
    assert!(payloads.iter().any(|(activated, ..)| !*activated));
}
