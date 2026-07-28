#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[test]
pub(super) fn parse_oracle_warp_world_strict_parse_and_render_regression() {
    let def = parse_oracle_card_definition("Warp World");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("shuffleobjectsintolibraryeffect")
            && raw.contains("lookattopcardseffect")
            && raw.contains("foreachtaggedeffect")
            && raw.contains("conditionaleffect")
            && raw.contains("zone: battlefield")
            && raw.contains("zone: library"),
        "expected Warp World to compile to shuffle/reveal/distribute effects, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player shuffles all permanents they own into their library")
            && rendered.contains("reveals that many cards from the top of their library")
            && rendered.contains("artifact, creature, and land cards revealed this way")
            && rendered.contains("then does the same for enchantment cards")
            && rendered.contains("weren't put onto the battlefield on the bottom of their library"),
        "expected Warp World to keep the owned-shuffle and staged reveal distribution wording, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged '") && !rendered.contains("tagged object"),
        "expected Warp World to avoid internal tagged-object markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_discover_the_impossible_strict_parse_and_render_regression() {
    let def = parse_oracle_card_definition("Discover the Impossible");

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("lookattopcardseffect")
            && debug.contains("chooseobjectseffect")
            && debug.contains("exileeffect")
            && debug.contains("puttaggedremainderonlibrarybottomeffect")
            && debug.contains("casttaggedeffect")
            && debug.contains("conditionaleffect")
            && debug.contains("didnothappen"),
        "expected Discover the Impossible to compile to look/choose/exile/remainder/conditional-cast/fallback-hand effects, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Look at the top five cards of your library. Exile one of them face down and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost if it's an instant spell with mana value 2 or less. If you don't, put that card into your hand."
    );
    assert!(
        !rendered.contains("tagged '") && !rendered.contains("tagged object"),
        "Discover the Impossible rendered text should not leak internal tagged-object markers: {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_doomskar_warrior_strict_parse_and_render_regression() {
    assert_oracle_card_parses_strict("Doomskar Warrior");

    let def = parse_oracle_card_definition("Doomskar Warrior");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("OrTrigger")
            && debug.contains("ThisDealsCombatDamageToPlayerTrigger")
            && debug.contains("ThisDealsDamageToTrigger")
            && debug.contains("LookAtTopCardsEffect")
            && debug.contains("EventValue")
            && debug.contains("Amount")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected Doomskar Warrior to compile player-or-battle combat damage into event-count look/reveal/rest effects, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Backup 1")
            && rendered.contains("Trample")
            && rendered.contains("deals combat damage to a player")
            && rendered.contains("deals combat damage to a battle")
            && rendered.contains("look at that many cards from the top of your library")
            && rendered.contains("put that card into your hand")
            && rendered.contains("Put the rest on the bottom of your library in a random order"),
        "expected Doomskar Warrior rendered text to preserve backup, trample, player-or-battle trigger, event count, and reveal/rest clauses, got {rendered}"
    );
}

pub(super) fn doomskar_warrior_combat_damage_trigger(
    def: &CardDefinition,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            triggered
                .trigger
                .display()
                .to_ascii_lowercase()
                .contains("deals combat damage")
                .then_some(triggered)
        })
        .expect("Doomskar Warrior should have a combat damage look trigger")
}

pub(super) fn doomskar_warrior_backup_trigger(
    def: &CardDefinition,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            triggered
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::BackupEffect>()
                        .is_some()
                })
                .then_some(triggered)
        })
        .expect("Doomskar Warrior should have a backup trigger")
}

pub(super) fn resolve_doomskar_warrior_backup(
    game: &mut crate::game_state::GameState,
    warrior_id: ObjectId,
    controller: PlayerId,
    target: ObjectId,
    triggered: &crate::ability::TriggeredAbility,
) {
    let mut ctx = crate::effects::ExecutionContext::new_default(warrior_id, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Doomskar Warrior backup trigger should resolve");
    }
}

#[test]
pub(super) fn doomskar_warrior_backup_puts_counter_and_grants_following_abilities_to_another_creature()
 {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let backup = doomskar_warrior_backup_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ally_def = CardDefinitionBuilder::new(CardId::from_raw(90_540), "Backup Ally")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let ally_id = game.create_object_from_definition(&ally_def, alice, Zone::Battlefield);

    resolve_doomskar_warrior_backup(&mut game, warrior_id, alice, ally_id, backup);

    assert_eq!(
        game.counter_count(ally_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Doomskar Warrior backup should put one +1/+1 counter on the target creature"
    );
    assert!(
        game.object_has_static_ability_id(ally_id, StaticAbilityId::Trample),
        "Doomskar Warrior backup should grant trample to another target creature"
    );
    assert!(
        game.current_abilities(ally_id)
            .expect("backup target should have calculated abilities")
            .iter()
            .any(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => triggered
                    .trigger
                    .display()
                    .to_ascii_lowercase()
                    .contains("deals combat damage"),
                _ => false,
            }),
        "Doomskar Warrior backup should grant the following combat-damage trigger to another creature"
    );
    let ally_granted_ability_effect_count = game
        .effect_store
        .continuous_effects
        .effects()
        .iter()
        .filter(|effect| {
            matches!(
                &effect.applies_to,
                crate::continuous::EffectTarget::Specific(id) if *id == ally_id
            ) && matches!(
                &effect.modification,
                crate::continuous::Modification::AddAbilityGeneric(_)
            ) && matches!(effect.duration, crate::effect::Until::EndOfTurn)
        })
        .count();
    assert_eq!(
        ally_granted_ability_effect_count, 2,
        "Doomskar Warrior backup should grant exactly trample and the following trigger until end of turn"
    );

    let mut self_game = crate::tests::test_helpers::setup_two_player_game();
    let self_id = self_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let before_self_ability_count = self_game
        .current_abilities(self_id)
        .expect("Doomskar Warrior should have calculated abilities")
        .len();

    resolve_doomskar_warrior_backup(&mut self_game, self_id, alice, self_id, backup);

    assert_eq!(
        self_game.counter_count(self_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "Doomskar Warrior backup should still put a +1/+1 counter on itself"
    );
    assert_eq!(
        self_game
            .current_abilities(self_id)
            .expect("Doomskar Warrior should still have calculated abilities")
            .len(),
        before_self_ability_count,
        "Doomskar Warrior backup should not grant an extra copy of its following abilities when it targets itself"
    );
    let self_granted_ability_effect_count = self_game
        .effect_store
        .continuous_effects
        .effects()
        .iter()
        .filter(|effect| {
            matches!(
                &effect.applies_to,
                crate::continuous::EffectTarget::Specific(id) if *id == self_id
            ) && matches!(
                &effect.modification,
                crate::continuous::Modification::AddAbilityGeneric(_)
            )
        })
        .count();
    assert_eq!(
        self_granted_ability_effect_count, 0,
        "Doomskar Warrior backup should not register temporary ability grants when it targets itself"
    );
}

#[test]
pub(super) fn doomskar_warrior_backup_granted_trigger_uses_backup_target_damage_amount() {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let backup = doomskar_warrior_backup_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ally_def = CardDefinitionBuilder::new(CardId::from_raw(90_541), "Backup Trigger Ally")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let ally_id = game.create_object_from_definition(&ally_def, alice, Zone::Battlefield);
    let instant = CardBuilder::new(CardId::from_raw(90_542), "Granted Trigger Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let creature = CardBuilder::new(CardId::from_raw(90_543), "Granted Trigger Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let instant_id = game.create_object_from_card(&instant, alice, Zone::Library);
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Library);
    let looked_stables =
        [instant_id, creature_id].map(|id| game.object(id).expect("library card exists").stable_id);

    resolve_doomskar_warrior_backup(&mut game, warrior_id, alice, ally_id, backup);
    let granted_abilities = game
        .current_abilities(ally_id)
        .expect("backup target should have calculated abilities");
    let granted_trigger = granted_abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .display()
                .to_ascii_lowercase()
                .contains("deals combat damage")
                .then_some(triggered),
            _ => None,
        })
        .expect("backup target should receive Doomskar Warrior's following trigger");

    resolve_doomskar_warrior_trigger(
        &mut game,
        ally_id,
        alice,
        crate::events::DamageTarget::Player(bob),
        2,
        granted_trigger,
    );

    assert_eq!(
        looked_stables
            .iter()
            .filter(|&&stable_id| stable_zone(&game, stable_id) == Some(Zone::Hand))
            .count(),
        1,
        "a creature that received Doomskar Warrior's following trigger should use its combat damage amount to look and put one matching card into hand"
    );
    assert_eq!(
        looked_stables
            .iter()
            .filter(|&&stable_id| stable_zone(&game, stable_id) == Some(Zone::Library))
            .count(),
        1,
        "the unchosen looked-at card from the granted trigger should remain in the library"
    );
}

pub(super) fn doomskar_damage_event(
    source: ObjectId,
    target: crate::events::DamageTarget,
    amount: u32,
    is_combat: bool,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            target,
            amount,
            is_combat,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
pub(super) fn doomskar_warrior_trigger_matches_player_and_battle_combat_damage_only() {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let triggered = doomskar_warrior_combat_damage_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let battle = CardBuilder::new(CardId::from_raw(90_501), "Test Battle")
        .card_types(vec![CardType::Battle])
        .build();
    let battle_id = game.create_object_from_card(&battle, bob, Zone::Battlefield);
    let creature = CardBuilder::new(CardId::from_raw(90_502), "Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, bob, Zone::Battlefield);

    let ctx = crate::triggers::TriggerContext::for_source(warrior_id, alice, &game);
    let player_combat = doomskar_damage_event(
        warrior_id,
        crate::events::DamageTarget::Player(bob),
        4,
        true,
    );
    let battle_combat = doomskar_damage_event(
        warrior_id,
        crate::events::DamageTarget::Object(battle_id),
        4,
        true,
    );
    let creature_combat = doomskar_damage_event(
        warrior_id,
        crate::events::DamageTarget::Object(creature_id),
        4,
        true,
    );
    let battle_noncombat = doomskar_damage_event(
        warrior_id,
        crate::events::DamageTarget::Object(battle_id),
        4,
        false,
    );

    assert!(triggered.trigger.matches(&player_combat, &ctx));
    assert!(triggered.trigger.matches(&battle_combat, &ctx));
    assert!(!triggered.trigger.matches(&creature_combat, &ctx));
    assert!(!triggered.trigger.matches(&battle_noncombat, &ctx));
}

pub(super) fn resolve_doomskar_warrior_trigger(
    game: &mut crate::game_state::GameState,
    warrior_id: ObjectId,
    controller: PlayerId,
    target: crate::events::DamageTarget,
    damage_amount: i32,
    triggered: &crate::ability::TriggeredAbility,
) {
    let event = doomskar_damage_event(warrior_id, target, damage_amount as u32, true);
    let mut ctx = crate::effects::ExecutionContext::new_default(warrior_id, controller)
        .with_triggering_event(event)
        .with_event_value_amount(damage_amount);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Doomskar Warrior combat damage trigger should resolve");
    }
}

pub(super) fn stable_zone(
    game: &crate::game_state::GameState,
    stable_id: StableId,
) -> Option<Zone> {
    game.find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

#[test]
pub(super) fn doomskar_warrior_reveals_one_matching_looked_card_and_bottoms_the_rest() {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let triggered = doomskar_warrior_combat_damage_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let battle = CardBuilder::new(CardId::from_raw(90_510), "Doomskar Test Battle")
        .card_types(vec![CardType::Battle])
        .build();
    let battle_id = game.create_object_from_card(&battle, bob, Zone::Battlefield);

    let instant = CardBuilder::new(CardId::from_raw(90_511), "Looked Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let land = CardBuilder::new(CardId::from_raw(90_512), "Looked Land")
        .card_types(vec![CardType::Land])
        .build();
    let creature = CardBuilder::new(CardId::from_raw(90_513), "Looked Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let instant_id = game.create_object_from_card(&instant, alice, Zone::Library);
    let land_id = game.create_object_from_card(&land, alice, Zone::Library);
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Library);
    let looked_stables = [instant_id, land_id, creature_id]
        .map(|id| game.object(id).expect("library card exists").stable_id);

    resolve_doomskar_warrior_trigger(
        &mut game,
        warrior_id,
        alice,
        crate::events::DamageTarget::Object(battle_id),
        3,
        triggered,
    );

    let hand_count = looked_stables
        .iter()
        .filter(|&&stable_id| stable_zone(&game, stable_id) == Some(Zone::Hand))
        .count();
    let library_count = looked_stables
        .iter()
        .filter(|&&stable_id| stable_zone(&game, stable_id) == Some(Zone::Library))
        .count();
    assert_eq!(
        hand_count, 1,
        "exactly one revealed creature or land card should move to hand"
    );
    assert_eq!(
        library_count, 2,
        "the unchosen looked-at cards should remain in the library as the bottomed rest"
    );
}

#[test]
pub(super) fn doomskar_warrior_no_matching_looked_card_puts_none_into_hand() {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let triggered = doomskar_warrior_combat_damage_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let battle = CardBuilder::new(CardId::from_raw(90_520), "Doomskar Empty Battle")
        .card_types(vec![CardType::Battle])
        .build();
    let battle_id = game.create_object_from_card(&battle, bob, Zone::Battlefield);
    let first = CardBuilder::new(CardId::from_raw(90_521), "First Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let second = CardBuilder::new(CardId::from_raw(90_522), "Second Sorcery")
        .card_types(vec![CardType::Sorcery])
        .build();
    let first_id = game.create_object_from_card(&first, alice, Zone::Library);
    let second_id = game.create_object_from_card(&second, alice, Zone::Library);
    let looked_stables =
        [first_id, second_id].map(|id| game.object(id).expect("library card exists").stable_id);

    resolve_doomskar_warrior_trigger(
        &mut game,
        warrior_id,
        alice,
        crate::events::DamageTarget::Object(battle_id),
        2,
        triggered,
    );

    assert!(
        looked_stables
            .iter()
            .all(|&stable_id| stable_zone(&game, stable_id) == Some(Zone::Library)),
        "with no creature or land among the looked-at cards, none should move to hand"
    );
}

#[test]
pub(super) fn doomskar_warrior_that_many_limits_the_looked_cards() {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let triggered = doomskar_warrior_combat_damage_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let lower_land = CardBuilder::new(CardId::from_raw(90_525), "Lower Unlooked Land")
        .card_types(vec![CardType::Land])
        .build();
    let top_instant = CardBuilder::new(CardId::from_raw(90_526), "Top Looked Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let lower_land_id = game.create_object_from_card(&lower_land, alice, Zone::Library);
    let top_instant_id = game.create_object_from_card(&top_instant, alice, Zone::Library);
    let lower_land_stable = game
        .object(lower_land_id)
        .expect("lower library card exists")
        .stable_id;
    let top_instant_stable = game
        .object(top_instant_id)
        .expect("top library card exists")
        .stable_id;

    resolve_doomskar_warrior_trigger(
        &mut game,
        warrior_id,
        alice,
        crate::events::DamageTarget::Player(bob),
        1,
        triggered,
    );

    assert_eq!(
        stable_zone(&game, lower_land_stable),
        Some(Zone::Library),
        "a matching card below the one-card look window should not be chosen or moved"
    );
    assert_eq!(
        stable_zone(&game, top_instant_stable),
        Some(Zone::Library),
        "the nonmatching looked card should remain in the library as the bottomed rest"
    );
    assert_eq!(
        game.players[alice.index()].hand.len(),
        0,
        "Doomskar Warrior should not put a matching card into hand unless it was among the looked cards"
    );
}

#[test]
pub(super) fn doomskar_warrior_player_damage_uses_damage_amount_for_look_count() {
    let def = parse_oracle_card_definition("Doomskar Warrior");
    let triggered = doomskar_warrior_combat_damage_trigger(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let warrior_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let instant = CardBuilder::new(CardId::from_raw(90_531), "Player Branch Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let land = CardBuilder::new(CardId::from_raw(90_532), "Player Branch Land")
        .card_types(vec![CardType::Land])
        .build();
    let instant_id = game.create_object_from_card(&instant, alice, Zone::Library);
    let land_id = game.create_object_from_card(&land, alice, Zone::Library);
    let looked_stables =
        [instant_id, land_id].map(|id| game.object(id).expect("library card exists").stable_id);

    resolve_doomskar_warrior_trigger(
        &mut game,
        warrior_id,
        alice,
        crate::events::DamageTarget::Player(bob),
        2,
        triggered,
    );

    assert_eq!(
        looked_stables
            .iter()
            .filter(|&&stable_id| stable_zone(&game, stable_id) == Some(Zone::Hand))
            .count(),
        1,
        "player combat damage should look at that many cards and move one creature or land to hand"
    );
    assert_eq!(
        looked_stables
            .iter()
            .filter(|&&stable_id| stable_zone(&game, stable_id) == Some(Zone::Library))
            .count(),
        1,
        "the unchosen card from the player-damage look should remain in the library"
    );
}

pub(super) fn discover_the_impossible_definition() -> CardDefinition {
    parse_oracle_card_definition("Discover the Impossible")
}

pub(super) fn one_mana_instant_card(id: u32, name: &str) -> crate::card::Card {
    instant_card_with_mana_value(id, name, 1)
}

pub(super) fn three_mana_instant_card(id: u32, name: &str) -> crate::card::Card {
    instant_card_with_mana_value(id, name, 3)
}

pub(super) fn instant_card_with_mana_value(
    id: u32,
    name: &str,
    mana_value: u8,
) -> crate::card::Card {
    CardBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]))
        .build()
}

pub(super) fn one_mana_creature_card(id: u32, name: &str) -> crate::card::Card {
    CardBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

pub(super) fn resolve_discover_the_impossible_with<D: crate::decision::DecisionMaker>(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    decision_maker: &mut D,
) {
    let discover = discover_the_impossible_definition();
    let source_id = game.create_object_from_definition(&discover, controller, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(source_id, controller));
    crate::game_loop::resolve_stack_entry_with(game, decision_maker)
        .expect("Discover the Impossible should resolve");
}

pub(super) fn discover_test_zones_by_stable(
    game: &crate::game_state::GameState,
    stable_ids: &[StableId],
) -> Vec<String> {
    stable_ids
        .iter()
        .map(|&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .map(|object| format!("{}:{:?}", object.name, object.zone))
                .unwrap_or_else(|| format!("{stable_id:?}:missing"))
        })
        .collect()
}

#[test]
pub(super) fn discover_the_impossible_casts_chosen_small_instant_and_bottoms_rest() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mut library_stables = Vec::new();
    for idx in 0..5 {
        let card = one_mana_instant_card(90_100 + idx, &format!("Discover Instant {idx}"));
        let id = game.create_object_from_card(&card, alice, Zone::Library);
        library_stables.push(game.object(id).expect("library card exists").stable_id);
    }

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    resolve_discover_the_impossible_with(&mut game, alice, &mut dm);

    let cast_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Stack)
        })
        .count();
    let bottomed_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Library)
        })
        .count();
    assert_eq!(
        cast_count,
        1,
        "one selected small instant should be cast from exile; zones={:?}",
        discover_test_zones_by_stable(&game, &library_stables)
    );
    assert_eq!(
        bottomed_count, 4,
        "the four unchosen looked-at cards should remain in the library as the bottomed rest"
    );
}

#[test]
pub(super) fn discover_the_impossible_declined_cast_puts_exiled_card_into_hand() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mut library_stables = Vec::new();
    for idx in 0..5 {
        let card = one_mana_instant_card(90_200 + idx, &format!("Declined Discover Instant {idx}"));
        let id = game.create_object_from_card(&card, alice, Zone::Library);
        library_stables.push(game.object(id).expect("library card exists").stable_id);
    }

    let mut dm = crate::decision::AutoPassDecisionMaker;
    resolve_discover_the_impossible_with(&mut game, alice, &mut dm);

    let hand_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Hand)
        })
        .count();
    let library_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Library)
        })
        .count();
    assert_eq!(
        hand_count,
        1,
        "declining the free cast should put that card into hand; zones={:?}",
        discover_test_zones_by_stable(&game, &library_stables)
    );
    assert_eq!(
        library_count, 4,
        "declining should still bottom the unchosen cards"
    );
}

#[test]
pub(super) fn discover_the_impossible_noninstant_choice_goes_to_hand_not_stack() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mut library_stables = Vec::new();
    for idx in 0..5 {
        let card = one_mana_creature_card(90_300 + idx, &format!("Discover Creature {idx}"));
        let id = game.create_object_from_card(&card, alice, Zone::Library);
        library_stables.push(game.object(id).expect("library card exists").stable_id);
    }

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    resolve_discover_the_impossible_with(&mut game, alice, &mut dm);

    let stack_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Stack)
        })
        .count();
    let hand_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Hand)
        })
        .count();
    assert_eq!(
        stack_count, 0,
        "a noninstant exiled card should not be cast"
    );
    assert_eq!(
        hand_count,
        1,
        "a chosen noninstant should move to hand through the if-you-don't branch; zones={:?}",
        discover_test_zones_by_stable(&game, &library_stables)
    );
}

#[test]
pub(super) fn discover_the_impossible_large_instant_choice_goes_to_hand_not_stack() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mut library_stables = Vec::new();
    for idx in 0..5 {
        let card = three_mana_instant_card(90_400 + idx, &format!("Discover Large Instant {idx}"));
        let id = game.create_object_from_card(&card, alice, Zone::Library);
        library_stables.push(game.object(id).expect("library card exists").stable_id);
    }

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    resolve_discover_the_impossible_with(&mut game, alice, &mut dm);

    let stack_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Stack)
        })
        .count();
    let hand_count = library_stables
        .iter()
        .filter(|&&stable_id| {
            game.find_object_by_stable_id(stable_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Hand)
        })
        .count();
    assert_eq!(
        stack_count, 0,
        "an instant above the mana-value limit should not be cast"
    );
    assert_eq!(
        hand_count,
        1,
        "an instant above the mana-value limit should move to hand through the if-you-don't branch; zones={:?}",
        discover_test_zones_by_stable(&game, &library_stables)
    );
}

#[test]
pub(super) fn warp_world_puts_revealed_permanents_onto_battlefield_and_rest_on_bottom() {
    use crate::card::CardBuilder;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::zone::Zone;

    let def = parse_oracle_card_definition("Warp World");
    let effects = def.spell_effect.as_ref().expect("spell effects");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let alice_old_perm = CardBuilder::new(CardId::from_raw(32_001), "Alice Old Permanent")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&alice_old_perm, alice, Zone::Battlefield);
    let alice_old_land = CardBuilder::new(CardId::from_raw(32_007), "Alice Old Land")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_card(&alice_old_land, alice, Zone::Battlefield);

    let bob_old_perm = CardBuilder::new(CardId::from_raw(32_002), "Bob Old Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&bob_old_perm, bob, Zone::Battlefield);
    let bob_old_creature = CardBuilder::new(CardId::from_raw(32_008), "Bob Old Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&bob_old_creature, bob, Zone::Battlefield);

    let alice_new_creature = CardBuilder::new(CardId::from_raw(32_003), "Alice New Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let alice_new_instant = CardBuilder::new(CardId::from_raw(32_004), "Alice New Instant")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_card(&alice_new_creature, alice, Zone::Library);
    game.create_object_from_card(&alice_new_instant, alice, Zone::Library);

    let bob_new_enchantment = CardBuilder::new(CardId::from_raw(32_005), "Bob New Enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let bob_new_sorcery = CardBuilder::new(CardId::from_raw(32_006), "Bob New Sorcery")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&bob_new_enchantment, bob, Zone::Library);
    game.create_object_from_card(&bob_new_sorcery, bob, Zone::Library);

    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in effects {
        execute_effect(&mut game, effect, &mut ctx).expect("execute Warp World effect");
    }

    let battlefield_names: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        battlefield_names
            .iter()
            .any(|name| name == "Alice New Creature"),
        "expected Warp World to put at least one revealed permanent onto the battlefield, got {battlefield_names:?}"
    );
    assert!(
        !battlefield_names
            .iter()
            .any(|name| name == "Alice New Instant")
            && !battlefield_names
                .iter()
                .any(|name| name == "Bob New Sorcery"),
        "expected nonpermanent revealed cards to stay off the battlefield, got {battlefield_names:?}"
    );

    let alice_library_names: Vec<_> = game
        .player(alice)
        .expect("alice")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    let bob_library_names: Vec<_> = game
        .player(bob)
        .expect("bob")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        alice_library_names
            .iter()
            .any(|name| name == "Alice New Instant")
            && bob_library_names
                .iter()
                .any(|name| name == "Bob New Sorcery"),
        "expected nonpermanent revealed cards on the bottom of their owners' libraries, got alice={alice_library_names:?}, bob={bob_library_names:?}"
    );

    let alice_graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    let bob_graveyard_names: Vec<_> = game
        .player(bob)
        .expect("bob")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        !alice_graveyard_names
            .iter()
            .any(|name| name == "Alice New Instant")
            && !bob_graveyard_names
                .iter()
                .any(|name| name == "Bob New Sorcery"),
        "expected Warp World leftovers to go to library bottom rather than graveyard, got alice={alice_graveyard_names:?}, bob={bob_graveyard_names:?}"
    );
}

#[test]
pub(super) fn parse_oracle_myr_landshaper_type_addition_render_regression() {
    let def = parse_oracle_card_definition("Myr Landshaper");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("addcardtypes") && raw.contains("artifact"),
        "expected raw compiled definition to keep artifact type addition, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "target land becomes an artifact in addition to its other types until end of turn"
        ),
        "expected Myr Landshaper type-addition wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Myr Landshaper to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_kavu_recluse_fixed_basic_land_type_regression() {
    let def = parse_oracle_card_definition("Kavu Recluse");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("becomebasiclandtypechoiceeffect")
            && raw.contains("fixed_subtype")
            && raw.contains("forest"),
        "expected raw compiled definition to use fixed basic land type lowering, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes")
            && rendered.contains("forest")
            && rendered.contains("until end of turn"),
        "expected Kavu Recluse basic-land-type wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Kavu Recluse to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_slimy_kavu_fixed_basic_land_type_regression() {
    let def = parse_oracle_card_definition("Slimy Kavu");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("becomebasiclandtypechoiceeffect")
            && raw.contains("fixed_subtype")
            && raw.contains("swamp"),
        "expected raw compiled definition to use fixed basic land type lowering, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes")
            && rendered.contains("swamp")
            && rendered.contains("until end of turn"),
        "expected Slimy Kavu basic-land-type wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Slimy Kavu to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_tidal_warrior_fixed_basic_land_type_regression() {
    let def = parse_oracle_card_definition("Tidal Warrior");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("becomebasiclandtypechoiceeffect")
            && raw.contains("fixed_subtype")
            && raw.contains("island"),
        "expected raw compiled definition to use fixed basic land type lowering, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target land becomes")
            && rendered.contains("island")
            && rendered.contains("until end of turn"),
        "expected Tidal Warrior basic-land-type wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Tidal Warrior to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_master_biomancer_etb_mutant_regression() {
    let def = parse_oracle_card_definition("Master Biomancer");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enterwithcountersforfilter")
            && raw.contains("power")
            && (raw.contains("added_subtypes") || raw.contains("subtypes"))
            && raw.contains("mutant"),
        "expected raw compiled definition to retain dynamic ETB counters and mutant subtype, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each other creature you control enters with")
            && rendered.contains("additional +1/+1 counters")
            && rendered.contains("equal to this creature's power")
            && rendered.contains("mutant"),
        "expected Master Biomancer ETB mutant wording, got {rendered}"
    );
    assert!(
        !rendered.contains("enter the battlefield with counters"),
        "expected Master Biomancer to avoid generic ETB counter placeholder text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_skanos_dragonheart_greatest_power_regression() {
    let def = parse_oracle_card_definition("Skanos Dragonheart");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("greatestpower") && raw.contains("dragon") && raw.contains("graveyard"),
        "expected raw compiled definition to retain the greatest-power source expression, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("greatest power")
            && rendered.contains("other dragons you control")
            && rendered.contains("dragon cards in your graveyard"),
        "expected Skanos Dragonheart to render its greatest-power explanation, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_ambitious_dragonborn_where_x_enters_counters_regression() {
    let def = parse_oracle_card_definition("Ambitious Dragonborn");

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("SurfaceHinted") && raw.contains("WhereXIs") && raw.contains("GreatestPower"),
        "expected Ambitious Dragonborn to retain a where-X greatest-power counter value, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "this creature enters with x +1/+1 counters on it, where x is the greatest power among creatures you control and creature cards in your graveyard"
        ),
        "expected Ambitious Dragonborn to render the where-X counter clause, got {rendered}"
    );
    assert!(
        !rendered.contains("creatures you control or creature cards in your graveyard"),
        "expected domain-union wording to use 'and', got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_accomplished_automaton_fabricate_one_regression() {
    let def = parse_oracle_card_definition("Accomplished Automaton");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("put a +1/+1 counter on this creature")
            && raw.contains("create a 1/1 colorless servo artifact creature token"),
        "expected raw compiled definition to singularize fabricate-1 mode descriptions, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("fabricate 1"),
        "expected Accomplished Automaton to preserve the keyword-only fabricate line, got {rendered}"
    );
    assert!(
        !rendered.contains("choose one"),
        "expected Accomplished Automaton raw compiled text to avoid expanded fabricate mode text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_ambitious_aetherborn_fabricate_one_regression() {
    let def = parse_oracle_card_definition("Ambitious Aetherborn");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("put a +1/+1 counter on this creature")
            && raw.contains("create a 1/1 colorless servo artifact creature token"),
        "expected raw compiled definition to singularize fabricate-1 mode descriptions, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("fabricate 1"),
        "expected Ambitious Aetherborn to preserve the keyword-only fabricate line, got {rendered}"
    );
    assert!(
        !rendered.contains("choose one"),
        "expected Ambitious Aetherborn raw compiled text to avoid expanded fabricate mode text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_glint_sleeve_artisan_fabricate_one_regression() {
    let def = parse_oracle_card_definition("Glint-Sleeve Artisan");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("put a +1/+1 counter on this creature")
            && raw.contains("create a 1/1 colorless servo artifact creature token"),
        "expected raw compiled definition to singularize fabricate-1 mode descriptions, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("fabricate 1"),
        "expected Glint-Sleeve Artisan to preserve the keyword-only fabricate line, got {rendered}"
    );
    assert!(
        !rendered.contains("choose one"),
        "expected Glint-Sleeve Artisan raw compiled text to avoid expanded fabricate mode text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_arwen_weaver_of_hope_dynamic_etb_counters_regression() {
    let def = parse_oracle_card_definition("Arwen, Weaver of Hope");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enterwithcountersforfilter")
            && raw.contains("toughness")
            && raw.contains("other: true")
            && raw.contains("card_types: [creature]"),
        "expected raw compiled definition to retain toughness-based ETB counters, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each other creature you control enters with")
            && rendered.contains("additional +1/+1 counters")
            && (rendered.contains("equal to arwen's toughness")
                || rendered.contains("equal to this creature's toughness")),
        "expected Arwen, Weaver of Hope to render toughness-based ETB counters, got {rendered}"
    );
    assert!(
        !rendered.contains("enter the battlefield with counters"),
        "expected Arwen, Weaver of Hope to avoid generic ETB counter placeholder text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_grumgully_the_generous_etb_counter_regression() {
    let def = parse_oracle_card_definition("Grumgully, the Generous");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enterwithcountersforfilter")
            && raw.contains("excluded_subtypes")
            && raw.contains("human")
            && raw.contains("plusoneplusone"),
        "expected raw compiled definition to retain the non-Human ETB counter filter, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each other non-human creature you control enters")
            && rendered.contains("with an additional +1/+1 counter on it"),
        "expected Grumgully, the Generous to render its ETB counter text, got {rendered}"
    );
    assert!(
        !rendered.contains("enter the battlefield with counters"),
        "expected Grumgully, the Generous to avoid generic ETB counter placeholder text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_biophagus_conditional_mana_bonus_regression() {
    let def = parse_oracle_card_definition("Biophagus");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("mana_usage_restrictions")
            && raw.contains("restrict_to_matching_spell: false")
            && raw.contains("plusoneplusone"),
        "expected Biophagus to retain its conditional mana bonus metadata, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("add one mana of any color")
            && rendered.contains("if this mana is spent to cast a creature spell")
            && rendered.contains("that creature enters with an additional +1/+1 counter on it"),
        "expected Biophagus to render both its mana ability and creature ETB bonus, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_arena_of_glory_spent_mana_haste_regression() {
    let def = parse_oracle_card_definition("Arena of Glory");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("exertcosteffect")
            && raw.contains("mana_usage_restrictions")
            && raw.contains("granted_abilities")
            && raw.contains("haste"),
        "expected Arena of Glory to retain exert cost and spent-mana haste metadata, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("{r}, {t}, exert this land: add {r}{r}")
            && rendered.contains(
                "if that mana is spent on a creature spell, it gains haste until end of turn"
            ),
        "expected Arena of Glory to render its exert mana ability and haste bonus, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Arena of Glory to render without unsupported placeholders, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_berg_strider_etb_snow_rider_regression() {
    let def = parse_oracle_card_definition("Berg Strider");

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("ZoneChangeTrigger") && raw.contains("this_object: true"),
        "expected Berg Strider to keep an ETB trigger, got {raw}"
    );
    assert!(
        !raw.contains("SpellCastTrigger"),
        "expected Berg Strider to avoid a spell-cast trigger fallback, got {raw}"
    );
    assert!(
        raw.contains("ManaSpentToCastThisSpellAtLeast"),
        "expected Berg Strider to keep its snow-mana condition, got {raw}"
    );
    assert!(
        raw.contains("TapEffect") && raw.contains("Untap("),
        "expected Berg Strider to keep both its tap effect and untap restriction, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "When this creature enters, tap target artifact or creature an opponent controls."
        ),
        "expected Berg Strider ETB tap clause, got {rendered}"
    );
    assert!(
        rendered.contains("If {S} was spent to cast this spell, that permanent doesn't untap during its controller's next untap step."),
        "expected Berg Strider snow-mana untap rider, got {rendered}"
    );
}

#[test]
pub(super) fn oracle_render_regression_named_cards_compile_cleanly() {
    let cultivator =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Cultivator Colossus")).join("\n");
    assert!(
        cultivator.contains(
            "When this creature enters, you may put a land card from your hand onto the battlefield tapped. If you do, draw a card and repeat this process."
        ),
        "expected Cultivator Colossus repeat-process text, got {cultivator}"
    );
    assert!(
        !cultivator.to_ascii_lowercase().contains("unsupported"),
        "expected Cultivator Colossus to render without unsupported markers, got {cultivator}"
    );

    let one_ring =
        unprocessed_compiled_lines(&parse_oracle_card_definition("The One Ring")).join("\n");
    assert!(
        one_ring.contains("gain protection from everything until your next turn"),
        "expected The One Ring protection wording, got {one_ring}"
    );
    assert!(
        one_ring.contains("burden counter"),
        "expected The One Ring burden-counter text, got {one_ring}"
    );

    let boseiju = unprocessed_compiled_lines(&parse_oracle_card_definition("Boseiju, Who Endures"))
        .join("\n");
    assert!(
        boseiju.contains("Destroy target")
            && boseiju.contains("artifact")
            && boseiju.contains("enchantment")
            && boseiju.contains("land")
            && boseiju.contains("That player may search their library")
            && boseiju.contains(
                "This ability costs {1} less to activate for each legendary creature you control"
            ),
        "expected Boseiju channel rendering, got {boseiju}"
    );

    let hanweir =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Hanweir Battlements")).join("\n");
    assert!(
        hanweir.contains("Hanweir Garrison") || hanweir.contains("hanweir garrison"),
        "expected Hanweir Battlements meld clause to compile, got {hanweir}"
    );
    assert!(
        !hanweir.to_ascii_lowercase().contains("unsupported"),
        "expected Hanweir Battlements to render without unsupported markers, got {hanweir}"
    );

    let otawara =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Otawara, Soaring City"))
            .join("\n");
    assert!(
        otawara.contains("Return target")
            && otawara.contains("artifact")
            && otawara.contains("creature")
            && otawara.contains("enchantment")
            && otawara.contains("planeswalker")
            && otawara.contains(
                "This ability costs {1} less to activate for each legendary creature you control"
            ),
        "expected Otawara channel rendering, got {otawara}"
    );

    let tolaria =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Tolaria West")).join("\n");
    assert!(
        tolaria.contains("Transmute {1}{U}{U}"),
        "expected Tolaria West transmute rendering, got {tolaria}"
    );
    assert!(
        !tolaria.contains("permanent card"),
        "expected Tolaria West to avoid placeholder search text, got {tolaria}"
    );

    let solkanar =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Sol'Kanar the Tainted"))
            .join("\n");
    assert!(
        solkanar.contains("• Draw a card.")
            && solkanar.contains("• Each opponent loses 2 life and you gain 2 life.")
            && solkanar.contains(
                "• Exile Sol'Kanar, then return it to the battlefield under an opponent's control."
            ),
        "expected Sol'Kanar the Tainted to keep all modal bullet options, got {solkanar}"
    );

    let silverback =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Silverback Elder")).join("\n");
    assert!(
        (silverback.contains("• Destroy target artifact or enchantment.")
            || silverback.contains("choose one - Destroy target artifact or enchantment."))
            && silverback.contains("• Look at the top five cards of your library.")
            && silverback.contains("• You gain 4 life."),
        "expected Silverback Elder to keep all modal bullet options, got {silverback}"
    );

    let ojutai =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Ojutai Exemplars")).join("\n");
    assert!(
        (ojutai.contains("• Tap target creature.")
            || ojutai.contains("choose one - Tap target creature."))
            && ojutai.contains("• This creature gains first strike and lifelink until end of turn.")
            && ojutai.contains("• Exile this creature, then return it to the battlefield tapped under its owner's control."),
        "expected Ojutai Exemplars to keep all modal bullet options, got {ojutai}"
    );

    let pact = unprocessed_compiled_lines(&parse_oracle_card_definition("Demonic Pact")).join("\n");
    assert!(
        pact.contains("• This enchantment deals 4 damage to any target and you gain 4 life.")
            && pact.contains("• Target opponent discards two cards.")
            && pact.contains("• Draw two cards.")
            && pact.contains("• You lose the game."),
        "expected Demonic Pact to keep all modal bullet options, got {pact}"
    );
}

#[test]
pub(super) fn raw_render_regression_demonic_pact_keeps_modal_bullets() {
    let def = parse_oracle_card_definition("Demonic Pact");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("choose one that hasn't been chosen")
            && rendered
                .contains("• This enchantment deals 4 damage to any target and you gain 4 life.")
            && rendered.contains("• Target opponent discards two cards.")
            && rendered.contains("• Draw two cards.")
            && rendered.contains("• You lose the game."),
        "expected Demonic Pact raw compiled text to keep all modal bullet options, got {rendered}"
    );
}

pub(super) fn proud_pack_rhino_probe_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(91_000), "Proud Pack-Rhino")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rhino])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "When this creature enters, choose one —\n• Put a shield counter on target permanent.\n• Proliferate.",
        )
        .expect("Proud Pack-Rhino should parse")
}

pub(super) fn proud_pack_rhino_modal_effect(def: &CardDefinition) -> &ChooseModeEffect {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>()),
            _ => None,
        })
        .expect("Proud Pack-Rhino should have a modal ETB ability")
}

#[test]
pub(super) fn proud_pack_rhino_modal_etb_renders_with_bullets() {
    let def = proud_pack_rhino_probe_definition();
    let modal = proud_pack_rhino_modal_effect(&def);

    assert_eq!(modal.modes.len(), 2);
    assert_eq!(modal.min_choose_count, Value::Fixed(1));
    assert_eq!(modal.choose_count, Value::Fixed(1));

    let shield_effect = modal.modes[0].effects[0]
        .downcast_ref::<TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(&modal.modes[0].effects[0]);
    let shield_mode = shield_effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .expect("first mode should put a shield counter");
    assert_eq!(shield_mode.counter_type, crate::object::CounterType::Shield);
    assert_eq!(shield_mode.amount, Value::Fixed(1));
    match shield_mode.target.base() {
        ChooseSpec::Object(filter) => {
            assert_eq!(filter.zone, Some(Zone::Battlefield));
            assert!(
                [
                    CardType::Artifact,
                    CardType::Creature,
                    CardType::Enchantment,
                    CardType::Land,
                    CardType::Planeswalker,
                    CardType::Battle,
                ]
                .into_iter()
                .all(|card_type| filter.card_types.contains(&card_type)),
                "shield mode should target a permanent, got {filter:?}"
            );
        }
        other => panic!("shield mode should target a permanent, got {other:?}"),
    }
    let proliferate_effect = modal.modes[1].effects[0]
        .downcast_ref::<TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(&modal.modes[1].effects[0]);
    assert!(
        proliferate_effect
            .downcast_ref::<crate::effects::ProliferateEffect>()
            .is_some(),
        "second mode should proliferate"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("When this creature enters, choose one")
            && rendered.contains("\n• Put a shield counter on target permanent.")
            && rendered.contains("\n• Proliferate."),
        "expected Proud Pack-Rhino modal ETB to render as a header plus bullets, got {rendered}"
    );
    assert!(
        !rendered.contains("choose one - Put a shield counter"),
        "expected Proud Pack-Rhino modal rendering not to flatten the first mode, got {rendered}"
    );
}

#[test]
pub(super) fn proud_pack_rhino_shield_mode_puts_shield_counter_on_target_permanent() {
    let def = proud_pack_rhino_probe_definition();
    let modal = proud_pack_rhino_modal_effect(&def).clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rhino_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_001), "Training Idol")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(rhino_id, alice, &mut dm)
        .with_chosen_modes(Some(vec![0]))
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target_permanent(),
            range: 0..1,
        }]);

    modal
        .execute(&mut game, &mut ctx)
        .expect("Proud Pack-Rhino shield mode should resolve");

    assert_eq!(
        game.object(target_id).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::Shield)
            .copied()),
        Some(1),
        "target permanent should receive exactly one shield counter"
    );
}

#[test]
pub(super) fn proud_pack_rhino_proliferate_mode_increases_selected_counters() {
    struct SelectRhinoProliferateTargets {
        permanent: ObjectId,
        player: PlayerId,
    }

    impl crate::decision::DecisionMaker for SelectRhinoProliferateTargets {
        fn decide_proliferate(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::ProliferateContext,
        ) -> crate::decisions::specs::ProliferateResponse {
            crate::decisions::specs::ProliferateResponse {
                permanents: vec![self.permanent],
                players: vec![self.player],
            }
        }
    }

    let def = proud_pack_rhino_probe_definition();
    let modal = proud_pack_rhino_modal_effect(&def).clone();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rhino_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let countered_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_002), "Countered Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    assert!(
        game.add_counters(countered_id, crate::object::CounterType::PlusOnePlusOne, 1)
            .is_some(),
        "countered creature should be on the battlefield"
    );
    game.players[1].poison_counters = 1;

    let mut dm = SelectRhinoProliferateTargets {
        permanent: countered_id,
        player: bob,
    };
    let mut ctx = crate::effects::ExecutionContext::new(rhino_id, alice, &mut dm)
        .with_chosen_modes(Some(vec![1]));

    modal
        .execute(&mut game, &mut ctx)
        .expect("Proud Pack-Rhino proliferate mode should resolve");

    assert_eq!(
        game.object(countered_id).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()),
        Some(2),
        "selected permanent should get another counter of each kind it already has"
    );
    assert_eq!(
        game.players[1].poison_counters, 2,
        "selected player should get another poison counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_tekuthal_inquiry_dominus_compiles_strictly() {
    let def = parse_oracle_card_definition("Tekuthal, Inquiry Dominus");
    let rendered = canonical_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("if you would proliferate, proliferate twice instead"),
        "expected Tekuthal replacement clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "remove 3 counters from among other artifacts, creatures, and planeswalkers you control"
        ),
        "expected Tekuthal activated cost in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tekuthal_activation_cost_targets_other_countered_permanents_only() {
    use crate::ability::AbilityKind;
    let def = parse_oracle_card_definition("Tekuthal, Inquiry Dominus");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let _source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(98_001), "Other Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Tekuthal should have an activated ability");
    let cost_text = activated.mana_cost.display().to_ascii_lowercase();
    assert!(
        cost_text.contains(
            "remove 3 counters from among other artifacts, creatures, and planeswalkers you control"
        ),
        "Tekuthal activation cost should preserve among-other list, got {cost_text}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tekuthal_activated_effect_puts_indestructible_counter_on_source() {
    use crate::ability::AbilityKind;
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let def = parse_oracle_card_definition("Tekuthal, Inquiry Dominus");
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Tekuthal should have an activated ability");
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Tekuthal activated effect should resolve");
    }

    assert_eq!(
        game.object(source).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::Indestructible)
            .copied()),
        Some(1),
        "Tekuthal activated effect should put an indestructible counter on Tekuthal"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tekuthal_replacement_makes_single_proliferate_happen_twice() {
    struct SelectSameTargetsTwice {
        permanent: ObjectId,
        player: PlayerId,
    }

    impl crate::decision::DecisionMaker for SelectSameTargetsTwice {
        fn decide_proliferate(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::ProliferateContext,
        ) -> crate::decisions::specs::ProliferateResponse {
            crate::decisions::specs::ProliferateResponse {
                permanents: vec![self.permanent],
                players: vec![self.player],
            }
        }
    }

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tekuthal = parse_oracle_card_definition("Tekuthal, Inquiry Dominus");
    let proliferate_source =
        game.create_object_from_definition(&tekuthal, alice, Zone::Battlefield);
    let countered_creature =
        CardDefinitionBuilder::new(CardId::from_raw(98_101), "Countered Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
    let countered_id =
        game.create_object_from_definition(&countered_creature, alice, Zone::Battlefield);
    assert!(
        game.add_counters(countered_id, crate::object::CounterType::PlusOnePlusOne, 1)
            .is_some(),
        "countered creature should be on the battlefield"
    );
    game.players[1].poison_counters = 1;

    let mut dm = SelectSameTargetsTwice {
        permanent: countered_id,
        player: bob,
    };
    let mut ctx = crate::effects::ExecutionContext::new(proliferate_source, alice, &mut dm);
    crate::effects::execute_effect(&mut game, &crate::effect::Effect::proliferate(1), &mut ctx)
        .expect("proliferate should resolve");

    assert_eq!(
        game.object(countered_id).and_then(|object| object
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()),
        Some(3),
        "Tekuthal replacement should apply proliferate twice"
    );
    assert_eq!(
        game.players[1].poison_counters, 3,
        "Tekuthal replacement should apply proliferate to players twice"
    );
}

#[test]
pub(super) fn parse_oracle_remorseless_punishment_keeps_discard_or_sacrifice_unless_choice() {
    let def = parse_oracle_card_definition("Remorseless Punishment");
    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));

    assert!(
        spell_debug.contains("UnlessActionEffect"),
        "expected Remorseless Punishment to keep unless-action lowering, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("DiscardEffect"),
        "expected Remorseless Punishment to keep the discard branch, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("SacrificeEffect") || spell_debug.contains("SacrificePlayerEffect"),
        "expected Remorseless Punishment to keep the sacrifice branch, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target opponent loses 5 life unless")
            && rendered.contains(
                "discards two cards or sacrifices a creature or planeswalker of their choice"
            ),
        "expected Remorseless Punishment to preserve the discard-or-sacrifice choice with oracle-like pronouns, got {rendered}"
    );
    assert!(
        !rendered.contains("discards two creature or planeswalker cards"),
        "expected Remorseless Punishment not to collapse the sacrifice branch into a discard filter, got {rendered}"
    );
}

#[test]
pub(super) fn unprocessed_compiled_lines_normalize_remaining_tag_scaffolding_regressions() {
    let argivian =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Argivian Cavalier")).join("\n");
    assert!(
        argivian.contains("Enlist"),
        "expected Argivian Cavalier to keep the enlist keyword surface, got {argivian}"
    );
    assert!(
        !argivian.contains("enlist_attacker") && !argivian.contains("enlisted_creature"),
        "expected Argivian Cavalier to avoid raw enlist tags, got {argivian}"
    );

    let barkweave =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Barkweave Crusher")).join("\n");
    assert_eq!(
        barkweave, "Enlist",
        "expected Barkweave Crusher to collapse enlist back to the keyword surface, got {barkweave}"
    );

    let automate =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Accomplished Automaton"))
            .join("\n");
    assert!(
        automate.contains("Fabricate 1"),
        "expected Accomplished Automaton to keep the fabricate keyword surface, got {automate}"
    );
    assert!(
        !automate.contains("choose one"),
        "expected Accomplished Automaton to avoid fabricate reminder expansion, got {automate}"
    );

    let spikewheel =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Spikewheel Acrobat")).join("\n");
    assert!(
        spikewheel.contains("Spectacle {2}{R}"),
        "expected Spikewheel Acrobat to keep the spectacle keyword surface, got {spikewheel}"
    );
    assert!(
        !spikewheel.contains("rather than pay this spell's mana cost"),
        "expected Spikewheel Acrobat to avoid spectacle reminder expansion, got {spikewheel}"
    );

    let beamsaw =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Beamsaw Prospector")).join("\n");
    assert_eq!(
        beamsaw, "When this creature dies, create a Lander token.",
        "expected Beamsaw Prospector to keep the compact Lander token surface, got {beamsaw}"
    );
    assert!(
        !beamsaw.contains("Sacrifice this token:")
            && !beamsaw.contains("Search your library for a basic land card"),
        "expected Beamsaw Prospector to avoid leaking the Lander token reminder payload, got {beamsaw}"
    );

    let lithobraking =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Lithobraking")).join("\n");
    assert!(
        matches!(
            lithobraking.as_str(),
            "Create a Lander token. Then you may sacrifice an artifact. When you do, Lithobraking deals 2 damage to each creature."
                | "Create a Lander token. You may sacrifice an artifact. When you do, Lithobraking deals 2 damage to each creature."
        ),
        "expected Lithobraking to keep the compact Lander line and when-you-do phrasing, got {lithobraking}"
    );

    let zurgo =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Zurgo's Vanguard")).join("\n");
    assert!(
        zurgo.contains("Mobilize 1"),
        "expected Zurgo's Vanguard to keep the mobilize keyword surface, got {zurgo}"
    );
    assert!(
        !zurgo.contains("Warrior creature token that's tapped and attacking")
            && !zurgo.contains("Whenever Zurgo's Vanguard attacks, create"),
        "expected Zurgo's Vanguard to avoid expanded mobilize reminder text, got {zurgo}"
    );

    let mistform =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Mistform Ultimus")).join("\n");
    assert_eq!(
        mistform, "Mistform Ultimus is every creature type.",
        "expected Mistform Ultimus to keep prose oracle text instead of collapsing to a keyword, got {mistform}"
    );

    let adewale =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Adéwalé, Breaker of Chains"))
            .join("\n");
    assert!(
        adewale.contains("When Adéwalé enters, reveal the top six cards of your library")
            && adewale.contains("into your hand"),
        "expected Adéwalé helper chain to normalize, got {adewale}"
    );
    assert!(
        !adewale.contains("__sentence_helper_chosen_"),
        "expected Adéwalé to avoid sentence helper tags, got {adewale}"
    );

    let ainok =
        unprocessed_compiled_lines(&parse_oracle_card_definition("Ainok Wayfarer")).join("\n");
    assert!(
        ainok.contains(
            "When this creature enters, mill three cards. You may put a land card from among the milled cards into your hand. If you don't, put a +1/+1 counter on this creature."
        ),
        "expected Ainok Wayfarer helper chain to normalize, got {ainok}"
    );
    assert!(
        !ainok.contains("__sentence_helper_chosen_"),
        "expected Ainok Wayfarer to avoid sentence helper tags, got {ainok}"
    );

    let tempt = unprocessed_compiled_lines(&parse_oracle_card_definition("Tempt with Immortality"))
        .join("\n");
    assert!(
        tempt.contains(
            "Return a creature card from your graveyard to the battlefield. Each opponent may return a creature card from their graveyard to the battlefield. For each opponent who does, return a creature card from your graveyard to the battlefield."
        ),
        "expected Tempt with Immortality tempting-offer text to normalize, got {tempt}"
    );
    assert!(
        !tempt.contains("chosen_return_3"),
        "expected Tempt with Immortality to avoid chosen_return tags, got {tempt}"
    );
}

#[test]
pub(super) fn parse_oracle_tempt_with_immortality_uses_truthful_iterated_return_model() {
    let definition = parse_oracle_card_definition("Tempt with Immortality");
    let debug = format!("{definition:?}");
    assert!(
        debug.contains("decider: Some(IteratedPlayer)")
            && debug.contains("chooser: IteratedPlayer")
            && debug.contains("owner: Some(IteratedPlayer)"),
        "expected each opponent to decide and choose from their own graveyard, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&definition).join("\n");
    assert_eq!(
        rendered,
        "Return a creature card from your graveyard to the battlefield. Each opponent may return a creature card from their graveyard to the battlefield. For each opponent who does, return a creature card from your graveyard to the battlefield."
    );
}

#[test]
pub(super) fn parse_oracle_dawnbreak_reclaimer_keeps_linked_player_choice_and_plural_return() {
    let def = parse_oracle_card_definition("Dawnbreak Reclaimer");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("then that player chooses a creature card in your graveyard"),
        "expected Dawnbreak Reclaimer to bind the second chooser to the chosen opponent, got {rendered}"
    );
    assert!(
        rendered
            .contains("you may return those cards to the battlefield under their owners' control"),
        "expected Dawnbreak Reclaimer to keep the plural return clause, got {rendered}"
    );
    assert!(
        !rendered.contains("an opponent chooses exactly 1 creature card in your graveyard")
            && !rendered.contains("you may put that object onto the battlefield")
            && !rendered.contains("you may put it onto the battlefield under its owner's control"),
        "expected Dawnbreak Reclaimer to avoid generic-opponent/singular fallback wording, got {rendered}"
    );
}

#[test]
pub(super) fn unprocessed_compiled_lines_normalize_do_or_die_divvy_destroy_clause() {
    let def = parse_oracle_card_definition("Do or Die");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("Separate all creatures target player controls into two piles.")
            && rendered.contains("Destroy all creatures in the pile of that player's choice.")
            && rendered.contains("They can't be regenerated."),
        "expected Do or Die to render its two-pile destroy clause, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("divvy_chosen") && debug.contains("DestroyNoRegenerationEffect"),
        "expected Do or Die to keep the divvy destroy structure, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_chaos_warp_shuffle_clause_regression() {
    let def = parse_oracle_card_definition("Chaos Warp");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("shuffleobjectsintolibraryeffect"),
        "expected Chaos Warp to atomically move the permanent into its owner's library and shuffle, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("shuffle") && rendered.contains("library"),
        "expected Chaos Warp shuffle wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported"),
        "expected Chaos Warp to render without unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_oblation_shuffle_clause_regression() {
    let def = parse_oracle_card_definition("Oblation");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("shuffleobjectsintolibraryeffect"),
        "expected Oblation to atomically move the permanent into its owner's library and shuffle, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("shuffle") && rendered.contains("library"),
        "expected Oblation shuffle wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported"),
        "expected Oblation to render without unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_derevi_command_zone_put_regression() {
    let def = parse_oracle_card_definition("Derevi, Empyrial Tactician");

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("movetozoneeffect")
            && debug.contains("functional_zones: [command]")
            && debug.contains("zone: battlefield"),
        "expected Derevi to stay command-zone activatable and move onto the battlefield, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_onto_battlefield_under_your_control_tapped_preserves_behavior() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tapped Reanimate Variant")
        .parse_text("Put target creature card from a graveyard onto the battlefield tapped under your control.")
        .expect("tapped under-your-control battlefield move should parse");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("movetozoneeffect")
            && debug.contains("zone: battlefield")
            && debug.contains("battlefield_controller: you")
            && debug.contains("enters_tapped: true"),
        "expected tapped under-your-control battlefield behavior, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_ilharg_tapped_attacking_stays_deferred() {
    let def = parse_oracle_card_definition("Ilharg, the Raze-Boar");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("tapped and attacking") && rendered.contains("third from the top"),
        "expected Ilharg's strict parse to preserve tapped-attacking and library placement, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_winota_tapped_attacking_stays_deferred() {
    assert_oracle_card_fails_strict("Winota, Joiner of Forces");
}

#[test]
pub(super) fn parse_oracle_banefire_threshold_restrictions_regression() {
    let rendered = unprocessed_compiled_lines(&parse_oracle_card_definition("Banefire"))
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("deal x damage to any target")
            || rendered.contains("deals x damage to any target"),
        "expected Banefire damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("if x is 5 or more"),
        "expected Banefire threshold clause, got {rendered}"
    );
    assert!(
        rendered.contains("this spell can't be countered")
            || rendered.contains("this spell cant be countered"),
        "expected Banefire uncounterable clause, got {rendered}"
    );
    assert!(
        rendered.contains("damage can't be prevented")
            || rendered.contains("damage cant be prevented"),
        "expected Banefire damage-prevention clause, got {rendered}"
    );
}

#[test]
pub(super) fn gnarled_sage_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Gnarled Sage");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.id() == StaticAbilityId::Reach
        )),
        "Gnarled Sage should parse reach strictly, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("MaxCardsDrawnThisTurn")
            && ability_debug.contains("GreaterThanOrEqual")
            && ability_debug.contains("Vigilance"),
        "expected drawn-two-cards condition to guard vigilance structurally, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "As long as you've drawn two or more cards this turn, this creature gets +0/+2 and has vigilance"
        ),
        "expected Gnarled Sage conditional buff text, got {rendered}"
    );
}

#[test]
pub(super) fn gnarled_sage_drawn_two_cards_condition_controls_buff_and_vigilance() {
    fn stage_cards_drawn(game: &mut crate::game_state::GameState, player: PlayerId, count: u32) {
        let cards = (0..count).map(|_| game.new_object_id()).collect();
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::CardsDrawnEvent::new(player, cards, count > 0),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    let oracle = oracle_text_by_name()
        .get("Gnarled Sage")
        .expect("Gnarled Sage oracle text")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Gnarled Sage")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Treefolk, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(oracle)
        .expect("Gnarled Sage should parse strictly");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sage_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert_eq!(game.calculated_power(sage_id), Some(4));
    assert_eq!(game.calculated_toughness(sage_id), Some(4));
    assert!(!game.object_has_static_ability_id(sage_id, StaticAbilityId::Vigilance));

    stage_cards_drawn(&mut game, bob, 2);
    assert_eq!(game.calculated_toughness(sage_id), Some(4));
    assert!(
        !game.object_has_static_ability_id(sage_id, StaticAbilityId::Vigilance),
        "opponent drawing two cards should not satisfy Gnarled Sage's 'you've drawn' condition"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    stage_cards_drawn(&mut game, alice, 1);
    assert_eq!(game.calculated_toughness(sage_id), Some(4));
    assert!(
        !game.object_has_static_ability_id(sage_id, StaticAbilityId::Vigilance),
        "drawing only one card should not satisfy Gnarled Sage's condition"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    stage_cards_drawn(&mut game, alice, 2);
    assert_eq!(game.calculated_power(sage_id), Some(4));
    assert_eq!(game.calculated_toughness(sage_id), Some(6));
    assert!(game.object_has_static_ability_id(sage_id, StaticAbilityId::Vigilance));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_drakuseth_maw_of_flames_multi_target_regression() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Drakuseth target phrase")
        .parse_text("Whenever this creature attacks, it deals 4 damage to any target.")
        .expect("primary damage clause should still parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("4 damage to any target"),
        "expected Drakuseth-style primary target clause, got {rendered}"
    );
}
#[derive(serde::Deserialize)]
pub(super) struct RegressionCardFaceJson {
    pub(super) name: String,
    pub(super) oracle_text: Option<String>,
}

#[derive(serde::Deserialize)]
pub(super) struct RegressionCardJson {
    pub(super) name: String,
    pub(super) oracle_text: Option<String>,
    pub(super) type_line: Option<String>,
    pub(super) card_faces: Option<Vec<RegressionCardFaceJson>>,
    pub(super) lang: Option<String>,
}

#[derive(Clone)]
pub(super) struct RegressionOracleCardInfo {
    pub(super) oracle_text: String,
    pub(super) type_line: Option<String>,
}

pub(super) fn oracle_card_info_by_name() -> &'static HashMap<String, RegressionOracleCardInfo> {
    static ORACLE_BY_NAME: OnceLock<HashMap<String, RegressionOracleCardInfo>> = OnceLock::new();
    ORACLE_BY_NAME.get_or_init(|| {
        let cards_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("cards.json");
        let raw = std::fs::read_to_string(&cards_path).unwrap_or_else(|err| {
            panic!("read {} for regression tests: {err}", cards_path.display())
        });
        let cards: Vec<RegressionCardJson> =
            serde_json::from_str(&raw).expect("parse cards.json for regression tests");
        let mut out = HashMap::new();
        for card in cards {
            if card.lang.as_deref().unwrap_or("en") != "en" {
                continue;
            }

            let full_name = card.name;
            let root_text = card.oracle_text.and_then(|text| {
                let trimmed = text.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            });

            let mut face_entries = Vec::new();
            if let Some(faces) = card.card_faces {
                for face in faces {
                    let Some(text) = face.oracle_text.and_then(|text| {
                        let trimmed = text.trim();
                        (!trimmed.is_empty()).then(|| trimmed.to_string())
                    }) else {
                        continue;
                    };
                    face_entries.push((face.name, text));
                }
            }

            let Some(primary_text) = root_text
                .clone()
                .or_else(|| face_entries.first().map(|(_, text)| text.clone()))
            else {
                continue;
            };

            out.entry(full_name.clone())
                .or_insert(RegressionOracleCardInfo {
                    oracle_text: primary_text.clone(),
                    type_line: card.type_line.clone(),
                });
            // A real face entry is more specific than the convenience aliases
            // derived from the combined `Front // Back` name. Register faces
            // first so the back-face name retains its own oracle text.
            for (face_name, face_text) in face_entries {
                out.entry(face_name).or_insert(RegressionOracleCardInfo {
                    oracle_text: face_text,
                    type_line: card.type_line.clone(),
                });
            }
            if full_name.contains(" // ") {
                for part in full_name.split(" // ") {
                    out.entry(part.to_string())
                        .or_insert(RegressionOracleCardInfo {
                            oracle_text: primary_text.clone(),
                            type_line: card.type_line.clone(),
                        });
                }
            }
        }
        out
    })
}

pub(super) fn oracle_text_by_name() -> &'static HashMap<String, String> {
    static ORACLE_TEXT_BY_NAME: OnceLock<HashMap<String, String>> = OnceLock::new();
    ORACLE_TEXT_BY_NAME.get_or_init(|| {
        oracle_card_info_by_name()
            .iter()
            .map(|(name, info)| (name.clone(), info.oracle_text.clone()))
            .collect()
    })
}

pub(super) fn parse_oracle_card_definition(name: &str) -> CardDefinition {
    let info = oracle_card_info_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"));
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name);
    if let Some(type_line) = info.type_line.as_deref() {
        let (supertypes, card_types, subtypes) = parse_type_line(type_line)
            .unwrap_or_else(|err| panic!("type-line regression failed for '{name}': {err:?}"));
        if !supertypes.is_empty() {
            builder = builder.supertypes(supertypes);
        }
        if !card_types.is_empty() {
            builder = builder.card_types(card_types);
        }
        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }
    }
    builder
        .parse_text(info.oracle_text.clone())
        .unwrap_or_else(|err| panic!("strict parser regression failed for '{name}': {err:?}"))
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn topography_tracker_replacement_makes_a_controlled_creature_explore_twice() {
    let definition = parse_oracle_card_definition("Topography Tracker");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("KeywordActionReplacement") && debug.matches("ExploreEffect").count() >= 2,
        "Topography Tracker must compile to a two-action explore replacement: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let tracker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Empty-Library Explorer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let explorer = game.create_object_from_definition(&creature, alice, Zone::Battlefield);

    crate::effect::Effect::explore(ChooseSpec::SpecificObject(explorer))
        .0
        .execute(
            &mut game,
            &mut crate::effects::ExecutionContext::new_default(tracker, alice),
        )
        .expect("replacement-backed explore should resolve");

    assert_eq!(
        game.counter_count(explorer, crate::object::CounterType::PlusOnePlusOne),
        2,
        "an empty-library explore puts one counter each time, proving both replacement actions resolved"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kami_of_whispered_hopes_adds_one_counter_only_to_your_matching_permanents() {
    let definition = parse_oracle_card_definition("Kami of Whispered Hopes");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("AddCountersPlacementReplacement")
            && debug.contains("Permanent")
            && debug.contains("additional: 1"),
        "Kami must retain the permanent filter and one-counter increment: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Counter Vessel")
        .card_types(vec![CardType::Artifact])
        .build();
    let yours = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let opponents = game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
    game.update_replacement_effects();

    assert_eq!(
        crate::events::processing::process_put_counters_with_event(
            &mut game,
            yours,
            crate::object::CounterType::PlusOnePlusOne,
            2,
            crate::events::EventCause::effect(),
        ),
        3,
        "Kami should add exactly one +1/+1 counter to your permanent"
    );
    assert_eq!(
        crate::events::processing::process_put_counters_with_event(
            &mut game,
            opponents,
            crate::object::CounterType::PlusOnePlusOne,
            2,
            crate::events::EventCause::effect(),
        ),
        2,
        "Kami must not modify counters placed on an opponent's permanent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn heart_of_light_keeps_one_combined_attached_prevention_ability() {
    let definition = parse_oracle_card_definition("Heart of Light");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("AttachedAbilityGrant")
            && debug.contains("PreventAllDamageDealtToAndByThisPermanent"),
        "Heart of Light must grant the combined event-layer prevention ability: {debug}"
    );
    let rendered = canonical_compiled_lines(&definition).join(" ");
    assert!(
        rendered
            .contains("Prevent all damage that would be dealt to and dealt by enchanted creature"),
        "Heart of Light must preserve both prevention directions in compiled text: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ocelot_pride_city_blessing_clause_copies_each_matching_recent_token() {
    fn record_entered_this_turn(game: &mut crate::game_state::GameState, object: ObjectId) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(object).expect("entered object should exist"),
            game,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                object,
                Zone::Command,
                Zone::Battlefield,
                crate::events::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&event);
    }

    fn create_token_fixture(
        game: &mut crate::game_state::GameState,
        definition: &CardDefinition,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let token = game.object_from_token_definition(id, definition, controller);
        game.add_object(token);
        id
    }

    let definition = parse_oracle_card_definition("Ocelot Pride");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:#?}", triggered.effects).contains("CreateTokenCopyEffect") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Ocelot Pride should retain its token-copy trigger");
    let effects_debug = format!("{:#?}", triggered.effects);
    assert!(
        effects_debug.contains("PlayerHasCitysBlessing")
            && effects_debug.contains("ForEachObject")
            && effects_debug.contains("entered_battlefield_this_turn: true")
            && effects_debug.contains("CreateTokenCopyEffect"),
        "Ocelot's conditional fanout must retain its predicate and entered-token filter: {effects_debug}"
    );
    let conditional = triggered
        .effects
        .flattened_default_effects()
        .into_iter()
        .find(|effect| effect.downcast_ref::<ConditionalEffect>().is_some())
        .expect("Ocelot's city's-blessing follow-up should lower to a conditional effect")
        .clone();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ocelot = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.grant_citys_blessing(alice);
    let cat = CardDefinitionBuilder::new(CardId::new(), "Test Cat")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Cat])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let old_token = create_token_fixture(&mut game, &cat, alice);
    game.turn_store.turn_history.clear_for_new_turn();
    let recent_token = create_token_fixture(&mut game, &cat, alice);
    let opponent_token = create_token_fixture(&mut game, &cat, bob);
    record_entered_this_turn(&mut game, recent_token);
    record_entered_this_turn(&mut game, opponent_token);

    conditional
        .0
        .execute(
            &mut game,
            &mut crate::effects::ExecutionContext::new_default(ocelot, alice),
        )
        .expect("Ocelot's conditional token-copy fanout should resolve");

    let alice_cats = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.name == "Test Cat"
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(
        alice_cats, 3,
        "only the one recent token you control should be copied (old={old_token:?}, recent={recent_token:?})"
    );
    let bob_cats = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.name == "Test Cat"
                && game.controller_of(object) == bob
        })
        .count();
    assert_eq!(bob_cats, 1, "an opponent's recent token must not be copied");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn semantic_repairs_hidden_by_normalizer_shims_have_truthful_models() {
    fn debug_for(
        name: &str,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
    ) -> (CardDefinition, String) {
        let oracle = oracle_text_by_name()
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"))
            .clone();
        let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types);
        if !subtypes.is_empty() {
            builder = builder.subtypes(subtypes);
        }
        let definition = builder
            .parse_text(oracle)
            .unwrap_or_else(|err| panic!("strict parser regression failed for '{name}': {err:?}"));
        let debug = format!("{definition:#?}");
        (definition, debug)
    }

    let (_, quenchable) = debug_for("Quenchable Fire", vec![CardType::Sorcery], vec![]);
    assert!(
        quenchable.contains("TargetPlayerOrControllerOfTarget"),
        "Quenchable Fire delayed damage should point back at the original player/planeswalker target, got {quenchable}"
    );

    let (pick_def, pick) = debug_for("Pick the Brain", vec![CardType::Sorcery], vec![]);
    assert!(
        pick.contains("__source_exiled__") && pick.contains("SameNameAsTagged"),
        "Pick the Brain search should be tied to the hand card exiled earlier, got {pick}"
    );
    assert_eq!(
        pick.matches("ChooseObjectsEffect").count(),
        2,
        "Pick the Brain should choose the hand card and the cross-zone search results without choosing the already-exiled reference again, got {pick}"
    );
    let pick_rendered = unprocessed_compiled_lines(&pick_def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        pick_rendered.contains("same name as the exiled card")
            && pick_rendered.contains("graveyard, hand, and library"),
        "Pick the Brain should preserve the tagged same-name cross-zone search, got {pick_rendered}"
    );

    let (prowling_def, prowling) = debug_for("Prowling Pangolin", vec![CardType::Creature], vec![]);
    let sacrificed_tag_mentions = prowling.matches("__sentence_helper_sacrificed_").count();
    assert!(
        prowling.contains("stop_after_first_happened: true")
            && prowling.contains("chooser: IteratedPlayer")
            && prowling.contains("player: IteratedPlayer")
            && sacrificed_tag_mentions >= 2,
        "Prowling Pangolin should let one iterated player sacrifice their own chosen creatures, got {prowling}"
    );
    let prowling_rendered = unprocessed_compiled_lines(&prowling_def).join(" ");
    assert!(
        prowling_rendered.contains("any player may sacrifice two creatures of their choice")
            && !prowling_rendered.contains("Sacrifice all permanents"),
        "Prowling Pangolin rendered text should not expose the old sacrifice artifact, got {prowling_rendered}"
    );

    let (_, tunnel) = debug_for("Tunnel Ignus", vec![CardType::Creature], vec![]);
    assert!(
        tunnel.contains("LandsEnteredBattlefieldThisTurn") && tunnel.contains("GreaterThanOrEqual"),
        "Tunnel Ignus should require that opponent's second land this turn, got {tunnel}"
    );

    let (_, kusari) = debug_for(
        "Kusari-Gama",
        vec![CardType::Artifact],
        vec![Subtype::Equipment],
    );
    assert!(
        kusari.contains("DealsDamageToTrigger") && kusari.contains("blocking: true"),
        "Kusari-Gama should trigger only when equipped creature damages a blocking creature, got {kusari}"
    );

    let (_, multani) = debug_for("Multani's Presence", vec![CardType::Enchantment], vec![]);
    assert!(
        multani.contains("SpellCounteredTrigger") && !multani.contains("SpellCastTrigger"),
        "Multani's Presence should trigger on countered spells, not casts, got {multani}"
    );

    let (_, shadow) = debug_for("Shadow of the Grave", vec![CardType::Instant], vec![]);
    assert!(
        shadow.contains("discarded_or_cycled_this_turn_by: Some"),
        "Shadow of the Grave should filter graveyard cards by this-turn cycling/discard history, got {shadow}"
    );

    let (_, telemin) = debug_for("Telemin Performance", vec![CardType::Sorcery], vec![]);
    assert!(
        telemin.contains("ConsultTopOfLibraryEffect")
            && telemin.contains("PutOntoBattlefieldEffect"),
        "Telemin Performance should both reveal/mill and put the revealed creature onto the battlefield, got {telemin}"
    );

    let (_, kynaios) = debug_for(
        "Kynaios and Tiro of Meletis",
        vec![CardType::Creature],
        vec![],
    );
    assert!(
        kynaios.contains("ForPlayersEffect")
            && kynaios.contains("predicate: DidNotHappen")
            && kynaios.contains("player: IteratedPlayer"),
        "Kynaios and Tiro should draw for each individual opponent who did not put a land in, got {kynaios}"
    );

    let (_, tempt) = debug_for("Tempt with Mayhem", vec![CardType::Instant], vec![]);
    assert!(
        tempt.contains("CopySpellEffect")
            && tempt.contains("copier: IteratedPlayer")
            && tempt.contains("PlayersWithPositiveCount"),
        "Tempt with Mayhem should tie opponent copies and your copy count to per-opponent outcomes, got {tempt}"
    );

    let (_, twist) = debug_for("Twist Allegiance", vec![CardType::Sorcery], vec![]);
    assert!(
        twist.contains("ChangeControllerToEffectController")
            && twist.contains("ChangeControllerToPlayer")
            && twist.contains("__twist_your_creatures__")
            && twist.contains("__twist_opponent_creatures__"),
        "Twist Allegiance should exchange control reciprocally with target opponent, got {twist}"
    );

    let (_, fistful) = debug_for("Fistful of Force", vec![CardType::Instant], vec![]);
    assert!(
        fistful.contains("ClashEffect")
            && fistful.contains("IfEffect")
            && fistful.contains("Trample"),
        "Fistful of Force should gate the extra pump and trample behind winning the clash, got {fistful}"
    );

    let (_, hisoka) = debug_for("Hisoka's Guard", vec![CardType::Creature], vec![]);
    assert!(
        hisoka.contains("SourceUntaps")
            && hisoka.contains("Shroud")
            && hisoka.contains("other: true"),
        "Hisoka's Guard should grant shroud to another controlled creature while this source remains tapped, got {hisoka}"
    );

    let (_, emet) = debug_for(
        "Emet-Selch of the Third Seat",
        vec![CardType::Creature],
        vec![],
    );
    assert!(
        emet.contains("PlayerLosesLifeTrigger") && emet.contains("one_or_more: true"),
        "Emet-Selch should use the aggregate one-or-more-opponents life-loss trigger, got {emet}"
    );

    let (_, serpentine) = debug_for("Serpentine Spike", vec![CardType::Sorcery], vec![]);
    let serpentine_compact = serpentine.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        serpentine.matches("DealDamageEffect").count() >= 3
            && serpentine_compact.contains("amount: Fixed( 2, )")
            && serpentine_compact.contains("amount: Fixed( 3, )")
            && serpentine_compact.contains("amount: Fixed( 4, )"),
        "Serpentine Spike should emit distinct 2, 3, and 4 damage effects, got {serpentine}"
    );

    let (_, hunt) = debug_for("Hunt Down", vec![CardType::Sorcery], vec![]);
    assert!(
        hunt.contains("targeted_blocker")
            && hunt.contains("targeted_attacker")
            && hunt.contains("MustBlockSpecificAttacker"),
        "Hunt Down should target blocker and attacker separately, got {hunt}"
    );

    let (_, tusker) = debug_for("Avalanche Tusker", vec![CardType::Creature], vec![]);
    assert!(
        tusker.contains("targeted_blocker")
            && tusker.contains("triggering")
            && tusker.contains("MustBlockSpecificAttacker"),
        "Avalanche Tusker should force the target blocker to block the attacking source, got {tusker}"
    );

    let (_, rimehorn) = debug_for("Rimehorn Aurochs", vec![CardType::Creature], vec![]);
    assert!(
        rimehorn.contains("targeted_blocker")
            && rimehorn.contains("targeted_attacker")
            && rimehorn.contains("MustBlockSpecificAttacker"),
        "Rimehorn Aurochs should target blocker and attacker separately, got {rimehorn}"
    );

    let (_, impetuous) = debug_for("Impetuous Devils", vec![CardType::Creature], vec![]);
    assert!(
        impetuous.contains("targeted_blocker")
            && impetuous.contains("triggering")
            && impetuous.contains("MustBlockSpecificAttacker"),
        "Impetuous Devils should preserve its optional blocker target and attacking source, got {impetuous}"
    );

    let (_, march) = debug_for("March from Velis Vel", vec![CardType::Instant], vec![]);
    assert!(
        march.contains("ChooseLandTypeEffect")
            && march.contains("exclude_basic: true")
            && march.contains("chosen_land_type: true"),
        "March from Velis Vel should choose a nonbasic land type and filter lands by that chosen type, got {march}"
    );
}

pub(super) fn blood_tyrant_game() -> (crate::game_state::GameState, PlayerId, PlayerId, ObjectId) {
    let def = parse_oracle_card_definition("Blood Tyrant");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tyrant_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    (game, alice, bob, tyrant_id)
}

pub(super) fn resolve_triggers_for_source(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> usize {
    let triggers = crate::triggers::check_triggers(game, event);
    let matching_count = triggers
        .iter()
        .filter(|entry| entry.source == source)
        .count();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers.into_iter().filter(|entry| entry.source == source) {
        trigger_queue.add(trigger);
    }
    if matching_count > 0 {
        crate::game_loop::put_triggers_on_stack(game, &mut trigger_queue)
            .expect("trigger should go on the stack");
        crate::game_loop::resolve_stack_entry(game).expect("trigger should resolve");
    }
    matching_count
}

pub(super) fn assert_oracle_card_parses_strict(name: &str) {
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for regression card '{name}'"))
        .clone();
    let result = CardDefinitionBuilder::new(CardId::new(), name).parse_text(oracle.clone());
    assert!(
        result.is_ok(),
        "strict parser regression failed for '{name}': {:?}\nOracle text:\n{}",
        result.err(),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aggregate_choice_wipes_and_sorcerer_class_compile_to_truthful_models() {
    for name in ["Destined Confrontation", "Slaughter the Strong"] {
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{:#?}", definition.spell_effect);
        assert!(
            debug.contains("ChooseObjectsEffect")
                && debug.contains("aggregate_constraint: Some")
                && debug.contains("Power")
                && debug.contains("maximum: Fixed(")
                && debug.contains("4")
                && debug.contains("SacrificePlayerEffect"),
            "{name} must retain the any-number total-power choice and sacrifice its complement: {debug}"
        );
    }

    let sorcerer = parse_oracle_card_definition("Sorcerer Class");
    let debug = format!("{sorcerer:#?}");
    assert!(
        debug.contains("SpellsCastThisTurnMatching")
            && debug.contains("Instant")
            && debug.contains("Sorcery")
            && debug.contains("TagTriggeringObjectEffect")
            && debug.contains("ExecuteWithSourceEffect")
            && debug.contains("ForPlayersEffect"),
        "Sorcerer Class must count matching casts, use the triggering spell as the damage source, and fan out to opponents: {debug}"
    );

    let stick_together = parse_oracle_card_definition("Stick Together");
    let debug = format!("{stick_together:#?}");
    assert_eq!(
        debug.matches("ChooseObjectsEffect").count(),
        4,
        "Stick Together must create one optional choice slot for each party role: {debug}"
    );
    for role in ["Cleric", "Rogue", "Warrior", "Wizard"] {
        assert!(
            debug.contains(role),
            "Stick Together is missing its {role} slot: {debug}"
        );
    }
    assert!(
        debug.contains("SacrificePlayerEffect"),
        "Stick Together must sacrifice the complement of the chosen party: {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&stick_together),
        vec![
            "Each player chooses a party from among creatures they control, then sacrifices the rest."
        ]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn slaughter_the_strong_keeps_each_players_legal_power_group_and_sacrifices_the_rest() {
    struct PowerGroupDecisionMaker {
        alice: PlayerId,
        seen_constraints: Vec<(PlayerId, crate::effect::ChoiceAggregateConstraint)>,
    }

    impl crate::decision::DecisionMaker for PowerGroupDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>();
            let Some(constraint) = ctx.aggregate_constraint.clone() else {
                return legal;
            };
            self.seen_constraints.push((ctx.player, constraint));
            legal
                .into_iter()
                .filter(|id| {
                    if ctx.player == self.alice {
                        game.current_power(*id) == Some(2)
                    } else {
                        game.current_power(*id) == Some(4)
                    }
                })
                .collect()
        }
    }

    fn creature(name: &str, power: i32) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 2))
            .build()
    }

    let definition = parse_oracle_card_definition("Slaughter the Strong");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Slaughter the Strong should have a spell effect");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    for (controller, name, power) in [
        (alice, "Alice Two A", 2),
        (alice, "Alice Two B", 2),
        (alice, "Alice Three", 3),
        (bob, "Bob Four", 4),
        (bob, "Bob One", 1),
    ] {
        game.create_object_from_definition(&creature(name, power), controller, Zone::Battlefield);
    }

    let mut dm = PowerGroupDecisionMaker {
        alice,
        seen_constraints: Vec::new(),
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Slaughter the Strong should resolve");
    drop(ctx);

    assert_eq!(dm.seen_constraints.len(), 2);
    assert!(dm.seen_constraints.iter().all(|(_, constraint)| {
        constraint == &crate::effect::ChoiceAggregateConstraint::total_power_at_most(4)
    }));
    let battlefield_names = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    assert!(battlefield_names.contains(&"Alice Two A".to_string()));
    assert!(battlefield_names.contains(&"Alice Two B".to_string()));
    assert!(battlefield_names.contains(&"Bob Four".to_string()));
    assert!(!battlefield_names.contains(&"Alice Three".to_string()));
    assert!(!battlefield_names.contains(&"Bob One".to_string()));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn triggering_spell_deals_matching_cast_count_damage_to_each_opponent() {
    fn cast_event(
        game: &crate::game_state::GameState,
        spell: ObjectId,
        caster: PlayerId,
    ) -> crate::triggers::TriggerEvent {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(spell).expect("spell should exist"),
            game,
        );
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new_with_snapshot(
                spell,
                caster,
                Zone::Hand,
                snapshot,
            ),
            crate::provenance::ProvNodeId::default(),
        )
    }

    let definition = CardDefinitionBuilder::new(CardId::new(), "Triggering Spell Damage Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you cast an instant or sorcery spell, that spell deals damage to each opponent equal to the number of instant and sorcery spells you've cast this turn.",
        )
        .expect("Sorcerer Class-style trigger should parse");
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Counted Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let first_spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    let first_event = cast_event(&game, first_spell, alice);
    game.record_turn_history_event(&first_event);
    let triggering_spell =
        game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    let triggering_event = cast_event(&game, triggering_spell, alice);
    game.record_turn_history_event(&triggering_event);

    let entries = crate::triggers::check_triggers(&game, &triggering_event);
    let entry = entries
        .iter()
        .find(|entry| entry.source == source)
        .expect("the matching cast should trigger the probe");
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_triggering_event(entry.triggering_event.clone());
    let mut damage_events = Vec::new();
    for effect in &entry.ability.effects {
        let outcome = crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Sorcerer Class-style trigger should resolve");
        damage_events.extend(outcome.events.into_iter().filter_map(|event| {
            event
                .downcast::<crate::events::DamageEvent>()
                .map(|damage| (damage.source, damage.target, damage.amount))
        }));
    }

    assert_eq!(game.player(bob).expect("Bob").life, 18);
    assert_eq!(game.player(charlie).expect("Charlie").life, 18);
    assert_eq!(damage_events.len(), 2, "{damage_events:?}");
    assert!(
        damage_events.iter().all(|(damage_source, _, amount)| {
            *damage_source == triggering_spell && *amount == 2
        })
    );
}

pub(super) fn azorius_guildmage_activated_ability_matching(
    def: &CardDefinition,
    predicate: impl Fn(&crate::ability::ActivatedAbility) -> bool,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            predicate(activated).then_some(activated)
        })
        .expect("Azorius Guildmage should have the requested activated ability")
}

pub(super) fn azorius_guildmage_tap_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    azorius_guildmage_activated_ability_matching(def, |activated| {
        activated
            .effects
            .flattened_default_effects()
            .iter()
            .any(|effect| effect.downcast_ref::<crate::effects::TapEffect>().is_some())
    })
}

pub(super) fn azorius_guildmage_counter_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    azorius_guildmage_activated_ability_matching(def, |activated| {
        activated
            .effects
            .flattened_default_effects()
            .iter()
            .any(|effect| {
                effect
                    .downcast_ref::<crate::effects::CounterEffect>()
                    .is_some()
            })
    })
}

pub(super) fn azorius_guildmage_counter_target_filter(
    activated: &crate::ability::ActivatedAbility,
) -> &crate::target::ObjectFilter {
    let target_spec = activated
        .choices
        .first()
        .expect("counter ability should declare a target");
    let ChooseSpec::Object(filter) = target_spec.base() else {
        panic!("counter target should lower to an object filter, got {target_spec:?}");
    };
    filter
}

pub(super) fn pay_azorius_guildmage_activation(
    game: &mut crate::game_state::GameState,
    player: PlayerId,
    source: ObjectId,
    activated: &crate::ability::ActivatedAbility,
    colored_mana: ManaSymbol,
) {
    game.player_mut(player)
        .expect("activating player exists")
        .mana_pool
        .add(colored_mana, 3);
    crate::cost::can_pay_cost(game, source, player, &activated.mana_cost)
        .expect("Azorius Guildmage activation cost should be payable");
    let mut dm = crate::decision::AutoPassDecisionMaker::default();
    crate::special_actions::pay_total_cost_with_choice(
        game,
        player,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("Azorius Guildmage activation cost should be paid");
    assert_eq!(
        game.player(player)
            .expect("activating player exists")
            .mana_pool
            .total(),
        0,
        "activation should consume the supplied colored mana for its colored and generic costs"
    );
}

#[test]
pub(super) fn azorius_guildmage_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Azorius Guildmage");
    let def = parse_oracle_card_definition("Azorius Guildmage");
    let rendered = unprocessed_compiled_lines(&def);
    assert_eq!(
        rendered,
        vec![
            "{2}{W}: Tap target creature.",
            "{2}{U}: Counter target activated ability.",
        ],
        "Azorius Guildmage should render both activated abilities exactly"
    );

    let activated_count = def
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .count();
    assert_eq!(
        activated_count, 2,
        "Azorius Guildmage should have exactly two activated abilities"
    );

    let tap = azorius_guildmage_tap_ability(&def);
    let counter = azorius_guildmage_counter_ability(&def);
    assert!(
        format!("{:?}", tap.mana_cost).contains("White"),
        "tap ability should keep its white activation cost, got {:?}",
        tap.mana_cost
    );
    assert!(
        format!("{:?}", counter.mana_cost).contains("Blue"),
        "counter ability should keep its blue activation cost, got {:?}",
        counter.mana_cost
    );

    let filter = azorius_guildmage_counter_target_filter(counter);
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::ActivatedAbility),
        "counter ability should structurally target activated abilities on the stack"
    );
}

#[test]
pub(super) fn azorius_guildmage_tap_activation_taps_target_creature_after_cost_payment() {
    let def = parse_oracle_card_definition("Azorius Guildmage");
    let activated = azorius_guildmage_tap_ability(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target_def = CardDefinitionBuilder::new(CardId::new(), "Azorius Guildmage Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_definition(&target_def, bob, Zone::Battlefield);

    pay_azorius_guildmage_activation(&mut game, alice, source, activated, ManaSymbol::White);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: activated
                .choices
                .first()
                .expect("tap ability should declare a target")
                .clone(),
            range: 0..1,
        }]);
    ctx.snapshot_targets(&game);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Azorius Guildmage tap ability should resolve");
    }

    assert!(
        game.is_tapped(target),
        "tap activation should tap the targeted creature"
    );
}

#[test]
pub(super) fn azorius_guildmage_counter_activation_counters_activated_ability_only() {
    let def = parse_oracle_card_definition("Azorius Guildmage");
    let activated = azorius_guildmage_counter_ability(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ability_source_def = CardDefinitionBuilder::new(CardId::new(), "Stack Ability Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let ability_source =
        game.create_object_from_definition(&ability_source_def, bob, Zone::Battlefield);
    game.stack.push(crate::game_state::StackEntry::ability(
        ability_source,
        bob,
        vec![crate::effect::Effect::draw(1)],
    ));

    let filter = azorius_guildmage_counter_target_filter(activated);
    let filter_ctx = crate::filter::FilterContext::new(alice).with_source(source);
    assert!(
        filter.matches(
            game.object(ability_source)
                .expect("activated ability source should exist"),
            &filter_ctx,
            &game,
        ),
        "counter target filter should match an activated ability on the stack"
    );

    pay_azorius_guildmage_activation(&mut game, alice, source, activated, ManaSymbol::Blue);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(ability_source)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: activated
                .choices
                .first()
                .expect("counter ability should declare a target")
                .clone(),
            range: 0..1,
        }]);
    ctx.snapshot_targets(&game);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Azorius Guildmage counter ability should resolve");
    }

    assert!(
        game.stack.is_empty(),
        "counter activation should remove the targeted activated ability from the stack"
    );
    assert_eq!(
        game.object(ability_source)
            .expect("ability source should remain on battlefield")
            .zone,
        Zone::Battlefield,
        "countering an activated ability should not move its source permanent"
    );

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Ordinary Stack Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let spell = game.create_object_from_definition(&spell_def, bob, Zone::Stack);
    game.stack
        .push(crate::game_state::StackEntry::new(spell, bob));

    let triggered_source_def =
        CardDefinitionBuilder::new(CardId::new(), "Triggered Ability Source")
            .card_types(vec![CardType::Enchantment])
            .build();
    let triggered_source =
        game.create_object_from_definition(&triggered_source_def, bob, Zone::Battlefield);
    game.stack.push(
        crate::game_state::StackEntry::ability(
            triggered_source,
            bob,
            vec![crate::effect::Effect::draw(1)],
        )
        .with_triggering_event(crate::events::RawEvent::new(
            crate::events::AbilityActivatedEvent::new(triggered_source, bob, false),
            crate::provenance::ProvNodeId::default(),
        )),
    );

    assert!(
        !filter.matches(
            game.object(spell).expect("spell stack object should exist"),
            &filter_ctx,
            &game,
        ),
        "counter target filter should reject ordinary spells"
    );
    assert!(
        !filter.matches(
            game.object(triggered_source)
                .expect("triggered ability source should exist"),
            &filter_ctx,
            &game,
        ),
        "counter target filter should reject triggered abilities"
    );
}

#[test]
pub(super) fn azorius_guildmage_counter_activation_rejects_target_that_left_stack() {
    let def = parse_oracle_card_definition("Azorius Guildmage");
    let activated = azorius_guildmage_counter_ability(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let ability_source_def = CardDefinitionBuilder::new(CardId::new(), "Resolved Ability Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let ability_source =
        game.create_object_from_definition(&ability_source_def, bob, Zone::Battlefield);

    pay_azorius_guildmage_activation(&mut game, alice, source, activated, ManaSymbol::Blue);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(ability_source)]);
    for effect in activated.effects.flattened_default_effects() {
        let result = crate::effects::execute_effect(&mut game, effect, &mut ctx);
        assert_eq!(
            result,
            Err(crate::effects::ExecutionError::InvalidTarget),
            "counter ability should reject a target that left the stack"
        );
    }

    assert!(
        game.stack.is_empty(),
        "no stack entry should be removed when the target activated ability has already left the stack"
    );
    assert_eq!(
        game.object(ability_source)
            .expect("ability source should remain on battlefield")
            .zone,
        Zone::Battlefield,
        "invalid counter target should not move the source permanent"
    );
}

#[test]
pub(super) fn splintering_wind_preserves_the_authored_token_ability_sentences() {
    let definition = parse_oracle_card_definition("Splintering Wind");

    assert_eq!(
        unprocessed_compiled_lines(&definition),
        vec![
            "{2}{G}: This enchantment deals 1 damage to target creature. Create a 1/1 green Splinter creature token. It has flying and \"Cumulative upkeep {G}.\" When it leaves the battlefield, it deals 1 damage to you and each creature you control."
                .to_string()
        ]
    );
}

#[test]
pub(super) fn voice_of_many_preserves_the_relative_opponent_count() {
    let definition = parse_oracle_card_definition("Voice of Many");

    assert_eq!(
        unprocessed_compiled_lines(&definition),
        vec![
            "When this creature enters, draw a card for each opponent who controls fewer creatures than you."
                .to_string()
        ],
        "{definition:#?}"
    );
}

#[test]
pub(super) fn arabella_shares_one_dynamic_count_across_damage_and_life_gain() {
    let definition = parse_oracle_card_definition("Arabella, Abandoned Doll");

    assert_eq!(
        unprocessed_compiled_lines(&definition),
        vec![
            "Whenever Arabella attacks, it deals X damage to each opponent and you gain X life, where X is the number of creatures you control with power 2 or less."
                .to_string()
        ]
    );
}
