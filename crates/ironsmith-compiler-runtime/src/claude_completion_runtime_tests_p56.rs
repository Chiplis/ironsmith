//! Claude package 56 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p56_compile(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:?}")
}

/// "draw a card for each kind of counter on it": distinct counter kinds on
/// the source, not a counter named "of".
#[test]
fn p56_blitzball_draws_per_kind_of_counter() {
    let debug = p56_compile(
        "Blitzball Stadium",
        "Mana cost: {X}{U}\nType: Artifact\nWhen this artifact enters, support X. (Put a +1/+1 counter on each of up to X target creatures.)\nGo for the Goal! — {3}, {T}: Until end of turn, target creature gains \"Whenever this creature deals combat damage to a player, draw a card for each kind of counter on it\" and it can't be blocked this turn.",
    );
    assert!(debug.contains("DistinctCounterTypesAmong"), "{debug}");
    assert!(!debug.contains("Named(\"of\")"), "{debug}");
    assert!(debug.contains("SupportEffect"), "{debug}");
}

/// "doesn't untap during your untap step unless you have an enduring story"
/// is the untap restriction while the story is absent.
#[test]
fn p56_bombur_untap_restriction_unless_enduring_story() {
    let debug = p56_compile(
        "Bombur, Gentle Dreamer",
        "Mana cost: {2}{R}\nType: Legendary Creature — Dwarf Bard\nPower/Toughness: 4/4\nStoried (If you control three or more artifacts, legendaries, and/or Sagas, you have an enduring story for the rest of the game.)\nBombur doesn't untap during your untap step unless you have an enduring story.",
    );
    assert!(debug.contains("DoesntUntap"), "{debug}");
    assert!(debug.contains("Not("), "{debug}");
    assert!(debug.to_lowercase().contains("enduring"), "{debug}");
}

/// "you may cast the exiled nonland card until end of turn and mana of any
/// type can be spent": the permission is over the card that ended the loop.
#[test]
fn p56_black_widow_casts_the_exiled_nonland_card() {
    let debug = p56_compile(
        "Black Widow, Super Spy",
        "Mana cost: {1}{B}\nType: Legendary Creature — Human Spy Hero\nPower/Toughness: 2/2\nMenace\nWhenever Black Widow deals combat damage to a player, that player exiles cards from the top of their library until they exile a nonland card. You may put a +1/+1 counter on Black Widow. If you don't, you may cast the exiled nonland card until end of turn and mana of any type can be spent to cast that spell.",
    );
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.contains("consult_match"), "permission names the stopping card: {debug}");
    assert!(debug.contains("AnyType"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
}

/// "pay an amount of {E} equal to target nonland permanent's mana value":
/// the target is chosen with the trigger and destroyed by the reflexive
/// trigger.
#[test]
fn p56_behemoth_pays_energy_equal_to_target_mana_value() {
    let debug = p56_compile(
        "Behemoth of Vault 0",
        "Mana cost: {6}\nType: Artifact Creature — Robot\nPower/Toughness: 7/7\nTrample\nWhen this creature enters, you get {E}{E}{E}{E} (four energy counters).\nWhen this creature dies, you may pay an amount of {E} equal to target nonland permanent's mana value. When you do, destroy that permanent.",
    );
    assert!(debug.contains("PayEnergyEffect"), "{debug}");
    assert!(debug.contains("ManaValueOf("), "{debug}");
    assert!(debug.contains("TargetOnlyEffect"), "{debug}");
    assert!(debug.contains("DestroyEffect"), "{debug}");
}

/// "When you win the flip, ... When you lose the flip, ..." stay inside the
/// cast trigger as reflexive triggers on the flip.
#[test]
fn p56_breeches_flip_result_reflexive_triggers_stay_in_ability() {
    let definition = compile_to_runtime_definition(
        "Breeches, the Blastmaker",
        "Mana cost: {1}{U}{R}\nType: Legendary Creature — Goblin Pirate\nPower/Toughness: 3/3\nMenace\nWhenever you cast your second spell each turn, you may sacrifice an artifact. If you do, flip a coin. When you win the flip, copy that spell. You may choose new targets for the copy. When you lose the flip, Breeches deals damage equal to that spell's mana value to any target.",
        false,
    )
    .expect("Breeches should compile");
    let triggered = definition
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
        .count();
    assert_eq!(triggered, 1, "flip results are not sibling abilities: {definition:?}");
    let debug = format!("{definition:?}");
    assert!(debug.contains("FlipCoin"), "{debug}");
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    assert!(debug.contains("ReflexiveTrigger"), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
}

/// "for each {E} you've paid or lost this turn": energy removed from you this
/// turn reduces the cost.
#[test]
fn p56_blaster_hulk_cost_reduction_counts_energy_paid_or_lost() {
    let debug = p56_compile(
        "Blaster Hulk",
        "Mana cost: {6}{R}{R}\nType: Artifact Creature — Pirate\nPower/Toughness: 6/6\nThis spell costs {1} less to cast for each {E} (energy counter) you've paid or lost this turn.\nHaste\nWhenever this creature attacks, you get {E}{E}, then you may pay eight {E}. When you do, this creature deals 8 damage divided as you choose among up to eight targets.",
    );
    assert!(debug.contains("PlayerCountersRemoved"), "{debug}");
    assert!(debug.contains("Energy"), "{debug}");
    assert!(debug.contains("PayEnergyEffect"), "{debug}");
}

/// "cast instant and sorcery spells from the top of your graveyard" is a
/// top-only graveyard grant whose spells are exiled instead of going to a
/// graveyard.
#[test]
fn p56_bosium_strip_top_of_graveyard_permission_with_exile_rider() {
    let debug = p56_compile(
        "Bösium Strip",
        "Mana cost: {3}\nType: Artifact\n{3}, {T}: Until end of turn, you may cast instant and sorcery spells from the top of your graveyard. If a spell cast this way would be put into a graveyard, exile it instead.",
    );
    assert!(debug.contains("GrantBySpecEffect"), "{debug}");
    assert!(debug.contains("top_card_only: true"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(!debug.contains("cast_this_way_grants: []"), "exile rider is attached: {debug}");
}

/// "When a player other than this creature's owner controls it, that player
/// sacrifices it. If the player does, ...": a state trigger on the
/// controller not owning the source; the player is the controller.
#[test]
fn p56_bronze_bombshell_state_trigger_on_non_owner_control() {
    let debug = p56_compile(
        "Bronze Bombshell",
        "Mana cost: {4}\nType: Artifact Creature — Construct\nPower/Toughness: 4/1\nWhen a player other than this creature's owner controls it, that player sacrifices it. If the player does, this creature deals 7 damage to the player.",
    );
    assert!(debug.contains("StateTrigger"), "{debug}");
    assert!(debug.contains("NotYou"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(!debug.contains("IteratedPlayer"), "{debug}");
}

/// "For each player, exile up to one target non-Saga, nonland permanent that
/// player controls until this Saga leaves the battlefield."
#[test]
fn p56_helvault_exiles_non_saga_nonland_target_per_player() {
    let debug = p56_compile(
        "Battle at the Helvault",
        "Mana cost: {4}{W}{W}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI, II — For each player, exile up to one target non-Saga, nonland permanent that player controls until this Saga leaves the battlefield.\nIII — Create Avacyn, a legendary 8/8 white Angel creature token with flying, vigilance, and indestructible.",
    );
    assert!(debug.contains("Saga"), "{debug}");
    assert!(debug.contains("excluded_subtypes: [Saga]") || debug.contains("excluded_subtypes: [\n"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    assert!(debug.contains("Avacyn"), "{debug}");
}

/// "then target opponent destroys four lands they control": that opponent
/// chooses four of their own lands.
#[test]
fn p56_burning_of_xinye_opponent_chooses_lands_to_destroy() {
    let debug = p56_compile(
        "Burning of Xinye",
        "Mana cost: {4}{R}{R}\nType: Sorcery\nYou destroy four lands you control, then target opponent destroys four lands they control. Then Burning of Xinye deals 4 damage to each creature.",
    );
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("chooser: Target(Opponent)"), "{debug}");
    assert!(!debug.contains("controller: Some(IteratedPlayer)"), "{debug}");
    assert!(debug.contains("DealDamageEachEffect"), "{debug}");
}

/// "Until end of combat on your next turn, you may play that card."
#[test]
fn p56_brazen_cannonade_play_until_end_of_combat_next_turn() {
    let debug = p56_compile(
        "Brazen Cannonade",
        "Mana cost: {3}{R}\nType: Enchantment\nWhenever an attacking creature you control dies, this enchantment deals 2 damage to each opponent.\nRaid — At the beginning of each of your postcombat main phases, if you attacked this turn, exile the top card of your library. Until end of combat on your next turn, you may play that card.",
    );
    assert!(debug.contains("UntilEndOfCombatOnYourNextTurn"), "{debug}");
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
}

/// The end-of-combat boundary: valid before and during the holder's next
/// combat, gone in that turn's postcombat main phase.
#[test]
fn p56_until_end_of_combat_grant_source_expires_after_next_combat() {
    use ironsmith::card::CardBuilder;
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let next_turn = game.turn.turn_number + 2;
    let grant = GrantSource::until_end_of_combat_on_player_next_turn(source, alice, next_turn);
    assert!(grant.is_valid(&game), "valid during the creating turn");
    game.turn.turn_number = next_turn;
    game.turn.active_player = alice;
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    assert!(grant.is_valid(&game), "valid through that turn's combat");
    game.turn.phase = ironsmith::game_state::Phase::NextMain;
    assert!(!grant.is_valid(&game), "ends after that combat");
}
