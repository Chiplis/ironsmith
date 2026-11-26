//! A commander in the command zone is asked about at every priority. Under
//! the bounded menu policy (the Manabrew adapter) that question must stay
//! cheap and must not offer a commander the player plainly cannot pay for.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::LegalAction;
use ironsmith::game_loop::{
    analyze_priority_context, priority_analysis_bounded, priority_analysis_bounded_proof_budget,
    set_priority_analysis_bounded, set_priority_analysis_bounded_proof_budget,
};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId(0);

fn put(game: &mut GameState, name: &str, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, A, zone)
}

fn land(game: &mut GameState, name: &str, ability: &str) -> ObjectId {
    put(game, name, &format!("Type: Land\n{ability}"), Zone::Battlefield)
}

/// Forest, Island, a dual land, a bounce land and a Signet: five permanents
/// that make at most seven mana, with an eight-mana commander.
fn board(extra_lands: usize) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 40);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    land(&mut game, "Test Forest", "{T}: Add {G}.");
    land(&mut game, "Test Island", "{T}: Add {U}.");
    land(&mut game, "Test Vents", "{T}: Add {U} or {R}.");
    land(&mut game, "Test Boilerworks", "{T}: Add {U}{R}.");
    put(&mut game, "Test Signet", "Mana cost: {2}\nType: Artifact\n{1}, {T}: Add {R}{G}.", Zone::Battlefield);
    for index in 0..extra_lands {
        land(&mut game, &format!("Test Mountain {index}"), "{T}: Add {R}.");
    }
    let commander = put(
        &mut game,
        "Test Wanderer",
        "Mana cost: {5}{U}{R}{G}\nType: Legendary Creature — Elemental\nPower/Toughness: 7/5",
        Zone::Command,
    );
    game.set_as_commander(commander, A);
    (game, commander)
}

fn bounded_menu_offers(game: &GameState, commander: ObjectId) -> (bool, std::time::Duration) {
    struct Restore(bool, usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            set_priority_analysis_bounded(self.0);
            set_priority_analysis_bounded_proof_budget(self.1);
        }
    }
    let _restore = Restore(priority_analysis_bounded(), priority_analysis_bounded_proof_budget());
    set_priority_analysis_bounded(true);
    set_priority_analysis_bounded_proof_budget(128);
    let started = std::time::Instant::now();
    let context = analyze_priority_context(game, A).unwrap();
    let elapsed = started.elapsed();
    let offered = context.actions.iter_with_labels().any(|(action, _)| {
        matches!(action, LegalAction::CastSpell { spell_id, from_zone: Zone::Command, casting_method: CastingMethod::Normal }
            if *spell_id == commander)
    });
    (offered, elapsed)
}

#[test]
fn bounded_menu_leaves_out_an_unaffordable_commander_quickly() {
    let (game, commander) = board(0);
    let (offered, elapsed) = bounded_menu_offers(&game, commander);
    assert!(!offered, "seven mana at most cannot cast an eight-mana commander");
    assert!(elapsed < std::time::Duration::from_secs(5), "bounded menu took {elapsed:?}");
}

#[test]
fn bounded_menu_offers_an_affordable_commander() {
    let (game, commander) = board(2);
    let (offered, elapsed) = bounded_menu_offers(&game, commander);
    assert!(offered, "nine mana casts an eight-mana commander");
    assert!(elapsed < std::time::Duration::from_secs(5), "bounded menu took {elapsed:?}");
}

/// The board of the playtest freeze, a few turns on: basic lands, a tapped
/// shock land, a bounce land, a Signet, X spells in hand and a commander with
/// cascade. Six untapped sources make at most seven mana.
fn playtest_board() -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 40);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for (name, text) in [
        ("Forest", "Type: Basic Land — Forest\n({T}: Add {G}.)"),
        ("Island", "Type: Basic Land — Island\n({T}: Add {U}.)"),
        ("Mountain", "Type: Basic Land — Mountain\n({T}: Add {R}.)"),
        ("Izzet Boilerworks", "Type: Land\n{T}: Add {U}{R}."),
        ("Gruul Signet", "Mana cost: {2}\nType: Artifact\n{1}, {T}: Add {R}{G}."),
    ] {
        put(&mut game, name, text, Zone::Battlefield);
    }
    let vents = put(
        &mut game,
        "Steam Vents",
        "Type: Land — Island Mountain\n({T}: Add {U} or {R}.)\nAs this land enters, you may pay 2 life. If you don't, it enters tapped.",
        Zone::Battlefield,
    );
    game.tap(vents);
    for (name, text) in [
        ("Banefire", "Mana cost: {X}{R}\nType: Sorcery\nBanefire deals X damage to any target.\nIf X is 5 or more, this spell can't be countered and the damage can't be prevented."),
        ("Blue Sun's Zenith", "Mana cost: {X}{U}{U}{U}\nType: Instant\nTarget player draws X cards. Shuffle Blue Sun's Zenith into its owner's library."),
        ("Opt", "Mana cost: {U}\nType: Instant\nScry 1.\nDraw a card."),
        ("Beast Within", "Mana cost: {2}{G}\nType: Instant\nDestroy target permanent. Its controller creates a 3/3 green Beast creature token."),
        ("Guttersnipe", "Mana cost: {2}{R}\nType: Creature — Goblin Shaman\nPower/Toughness: 2/2\nWhenever you cast an instant or sorcery spell, Guttersnipe deals 2 damage to each opponent."),
    ] {
        put(&mut game, name, text, Zone::Hand);
    }
    let commander = put(
        &mut game,
        "Maelstrom Wanderer",
        "Mana cost: {5}{U}{R}{G}\nType: Legendary Creature — Elemental\nPower/Toughness: 7/5\nCreatures you control have haste.\nCascade, cascade",
        Zone::Command,
    );
    game.set_as_commander(commander, A);
    (game, commander)
}

#[test]
fn bounded_menu_on_the_playtest_board() {
    let (game, commander) = playtest_board();
    let (offered, elapsed) = bounded_menu_offers(&game, commander);
    assert!(!offered, "seven mana at most cannot cast an eight-mana commander");
    assert!(elapsed < std::time::Duration::from_secs(5), "bounded menu took {elapsed:?}");
}

/// A spell's own marker abilities (cascade, "can't be countered if X is 5 or
/// more") must not make the supply bound decline: without it, proving this
/// commander unaffordable simulates every activation order.
#[test]
fn exact_check_rules_out_the_playtest_commander_without_a_search() {
    let (game, _) = playtest_board();
    let started = std::time::Instant::now();
    let actions = ironsmith::decision::compute_commander_actions(&game, A).unwrap();
    let elapsed = started.elapsed();
    assert!(actions.is_empty(), "seven mana at most cannot cast an eight-mana commander");
    assert!(elapsed < std::time::Duration::from_secs(5), "exact commander check took {elapsed:?}");
}

/// A Signet's two mana take one to make: with the shock land untapped the
/// board shows eight units of supply but nets seven.
#[test]
fn supply_bound_counts_what_a_mana_ability_costs() {
    let (mut game, commander) = playtest_board();
    let vents = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| game.object(id).is_some_and(|object| object.name.as_str() == "Steam Vents"))
        .unwrap();
    game.untap(vents);
    let started = std::time::Instant::now();
    let actions = ironsmith::decision::compute_commander_actions(&game, A).unwrap();
    let elapsed = started.elapsed();
    assert!(actions.is_empty(), "seven mana net cannot cast an eight-mana commander");
    assert!(elapsed < std::time::Duration::from_secs(5), "exact commander check took {elapsed:?}");
    let (offered, _) = bounded_menu_offers(&game, commander);
    assert!(!offered);
    // One more land and it is castable, through the Signet.
    land(&mut game, "Test Mountain", "{T}: Add {R}.");
    assert_eq!(ironsmith::decision::compute_commander_actions(&game, A).unwrap().len(), 1);
    let (offered, _) = bounded_menu_offers(&game, commander);
    assert!(offered);
}
