//! Real cards against library position knowledge: who learns which slot, and
//! when a random placement teaches only a span.
use super::*;
use ironsmith_registry_test::compile_to_runtime_definition;

const ALICE: PlayerId = PlayerId::from_index(0);
const BOB: PlayerId = PlayerId::from_index(1);

fn definition(name: &str, header: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, format!("{header}\n{text}"), false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn instant(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    definition(name, "Mana cost: {U}\nType: Instant", text)
}

fn bear() -> ironsmith::cards::CardDefinition {
    definition("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2", "")
}

fn game() -> WasmGame {
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
    wasm
}

/// `names` bottom first, so the last one is the top card.
fn library(wasm: &mut WasmGame, owner: PlayerId, names: &[&str]) -> Vec<ObjectId> {
    names.iter().map(|name| {
        wasm.game.create_object_from_definition(&instant(name, "Draw a card."), owner, Zone::Library)
    }).collect()
}

fn put(wasm: &mut WasmGame, owner: PlayerId, name: &str, zone: Zone) -> ObjectId {
    let definition = if name == "Grizzly Bears" { bear() } else { instant(name, "Draw a card.") };
    wasm.game.create_object_from_definition(&definition, owner, zone)
}

/// Resolves `spell` and answers its prompts in order; any prompt left over fails.
fn resolve(
    wasm: &mut WasmGame,
    spell: &ironsmith::cards::CardDefinition,
    controller: PlayerId,
    targets: Vec<Target>,
    x: Option<u32>,
    answers: &[crate::ReplayDecisionAnswer],
) {
    let id = wasm.game.create_object_from_definition(spell, controller, Zone::Stack);
    let mut entry = StackEntry::new(id, controller);
    entry.targets = targets;
    entry.x_value = x;
    wasm.game.push_to_stack(entry);
    let mut dm = WasmReplayDecisionMaker::new(answers);
    ironsmith::game_loop::resolve_stack_entry_with(&mut wasm.game, &mut dm)
        .unwrap_or_else(|error| panic!("{} should resolve: {error:?}", spell.name()));
    let (pending, _, _, _) = dm.finish();
    assert!(pending.is_none(), "{} left a prompt: {pending:#?}", spell.name());
}

/// The library top first, with what `viewer` knows about each slot.
fn knowledge(wasm: &WasmGame, viewer: PlayerId, owner: PlayerId) -> Vec<(String, Option<(usize, usize)>)> {
    let known = wasm.game.known_library_positions(viewer, owner);
    wasm.game.player(owner).unwrap().library.iter().rev()
        .map(|id| (wasm.game.object(*id).unwrap().name.to_string(), known.get(id).copied()))
        .collect()
}

fn exact(entries: &[(&str, usize)]) -> Vec<(String, Option<(usize, usize)>)> {
    entries.iter().map(|(name, slot)| (name.to_string(), Some((*slot, *slot)))).collect()
}

/// What `viewer`'s snapshot sends for `owner`'s library.
fn memory(wasm: &mut WasmGame, viewer: PlayerId, owner: PlayerId) -> Vec<(String, usize, Option<usize>, Option<usize>, Option<usize>, bool)> {
    let mut snapshot = GameSnapshot::from_game(
        &wasm.game, viewer, None, None, None, None, None, Vec::new(), None, false, None, 0,
    );
    wasm.hidden_card_memory.apply(&wasm.game, &[], &mut snapshot);
    snapshot.players[owner.index()].library_memory.iter()
        .map(|row| (row.name.clone(), row.count, row.position, row.range_start, row.range_end, row.id.is_some()))
        .collect()
}

#[test]
fn unexpectedly_absent_teaches_everyone_the_exact_slot_and_draws_shift_it() {
    let _ids = crate::test_id_counter_guard();
    let absent = definition("Unexpectedly Absent", "Mana cost: {X}{W}{W}\nType: Instant",
        "Put target nonland permanent into its owner's library just beneath the top X cards of that library.");
    let mut wasm = game();
    library(&mut wasm, BOB, &["B5", "B4", "B3", "B2", "B1"]);
    let bears = put(&mut wasm, BOB, "Grizzly Bears", Zone::Battlefield);
    resolve(&mut wasm, &absent, ALICE, vec![Target::Object(bears)], Some(2), &[]);

    for viewer in [ALICE, BOB] {
        let known = knowledge(&wasm, viewer, BOB);
        assert_eq!(known[2], ("Grizzly Bears".to_string(), Some((2, 2))), "{viewer:?}: {known:?}");
        assert!(known.iter().enumerate().all(|(slot, (_, bounds))| slot == 2 || bounds.is_none()));
    }
    // Alice sees only the Bears in Bob's deck; Bob also lists his other cards.
    assert_eq!(memory(&mut wasm, ALICE, BOB), vec![("Grizzly Bears".to_string(), 1, Some(2), Some(2), Some(2), true)]);
    let own = memory(&mut wasm, BOB, BOB);
    assert_eq!(own.iter().filter(|row| row.2.is_none()).map(|row| row.1).sum::<usize>(), 5);
    assert!(own.iter().filter(|row| row.2.is_none()).all(|row| !row.5), "unknown slots carry no ids: {own:?}");

    wasm.game.draw_cards(BOB, 1);
    assert_eq!(knowledge(&wasm, ALICE, BOB)[1].1, Some((1, 1)), "a draw moves the Bears up");

    // X past the library's size puts it on the bottom.
    let mut wasm = game();
    library(&mut wasm, BOB, &["B2", "B1"]);
    let bears = put(&mut wasm, BOB, "Grizzly Bears", Zone::Battlefield);
    resolve(&mut wasm, &absent, ALICE, vec![Target::Object(bears)], Some(10), &[]);
    assert_eq!(knowledge(&wasm, ALICE, BOB).last().unwrap(), &("Grizzly Bears".to_string(), Some((2, 2))));
}

#[test]
fn endurance_teaches_only_the_bottom_span_and_hides_the_order() {
    let _ids = crate::test_id_counter_guard();
    // Endurance's trigger, as a spell.
    let endurance = instant("Endurance Trigger",
        "Up to one target player puts all the cards from their graveyard on the bottom of their library in a random order.");
    let mut wasm = game();
    library(&mut wasm, ALICE, &["A3", "A2", "A1"]);
    for name in ["Brainstorm", "Endurance", "Ponder"] { put(&mut wasm, ALICE, name, Zone::Graveyard); }
    resolve(&mut wasm, &endurance, ALICE, vec![Target::Player(ALICE)], None, &[]);

    for viewer in [ALICE, BOB] {
        let known = knowledge(&wasm, viewer, ALICE);
        assert!(known[..3].iter().all(|(_, bounds)| bounds.is_none()), "{viewer:?}: {known:?}");
        assert!(known[3..].iter().all(|(_, bounds)| *bounds == Some((3, 5))), "{viewer:?}: {known:?}");
        let rows = memory(&mut wasm, viewer, ALICE);
        let span: Vec<_> = rows.iter().filter(|row| row.3 == Some(3)).collect();
        assert_eq!(span.len(), 3, "{viewer:?}: {rows:?}");
        assert!(span.iter().all(|row| row.2.is_none() && !row.5), "spans carry no slot or id: {rows:?}");
        let mut names: Vec<_> = span.iter().map(|row| row.0.as_str()).collect();
        names.sort();
        assert_eq!(names, ["Brainstorm", "Endurance", "Ponder"]);
    }
}

#[test]
fn brainstorm_teaches_its_caster_but_not_the_opponent() {
    let _ids = crate::test_id_counter_guard();
    let brainstorm = instant("Brainstorm",
        "Draw three cards, then put two cards from your hand on top of your library in any order.");
    let mut wasm = game();
    library(&mut wasm, ALICE, &["A5", "A4", "A3", "A2", "A1"]);
    let force = put(&mut wasm, ALICE, "Force", Zone::Hand);
    let daze = put(&mut wasm, ALICE, "Daze", Zone::Hand);
    resolve(&mut wasm, &brainstorm, ALICE, Vec::new(), None, &[
        crate::ReplayDecisionAnswer::Objects(vec![force, daze]),
        crate::ReplayDecisionAnswer::Order(vec![daze, force]),
    ]);
    let alice = knowledge(&wasm, ALICE, ALICE);
    assert_eq!(alice[..2], exact(&[("Daze", 0), ("Force", 1)])[..], "{alice:?}");
    assert!(alice[2..].iter().all(|(_, bounds)| bounds.is_none()));
    assert!(knowledge(&wasm, BOB, ALICE).iter().all(|(_, bounds)| bounds.is_none()));
    assert!(memory(&mut wasm, BOB, ALICE).is_empty(), "Bob sees only backs");
}

#[test]
fn ponder_keeps_the_order_unless_it_shuffles() {
    let _ids = crate::test_id_counter_guard();
    let ponder = instant("Ponder",
        "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card.");
    for shuffle in [false, true] {
        let mut wasm = game();
        let cards = library(&mut wasm, ALICE, &["A5", "A4", "A3", "A2", "A1"]);
        let order = vec![cards[2], cards[4], cards[3]];
        resolve(&mut wasm, &ponder, ALICE, Vec::new(), None, &[
            crate::ReplayDecisionAnswer::Order(order),
            crate::ReplayDecisionAnswer::Boolean(shuffle),
        ]);
        let alice = knowledge(&wasm, ALICE, ALICE);
        if shuffle {
            assert!(alice.iter().all(|(_, bounds)| bounds.is_none()), "{alice:?}");
        } else {
            assert_eq!(alice[..2], exact(&[("A1", 0), ("A2", 1)])[..], "{alice:?}");
        }
        assert!(knowledge(&wasm, BOB, ALICE).iter().all(|(_, bounds)| bounds.is_none()));
    }
}

#[test]
fn scry_knows_both_ends_and_commune_knows_a_bottom_span() {
    let _ids = crate::test_id_counter_guard();
    let preordain = instant("Preordain", "Scry 2, then draw a card.");
    let mut wasm = game();
    let cards = library(&mut wasm, ALICE, &["A5", "A4", "A3", "A2", "A1"]);
    resolve(&mut wasm, &preordain, ALICE, Vec::new(), None, &[
        crate::ReplayDecisionAnswer::Partition(vec![cards[4]]),
    ]);
    // A1 went to the bottom, A2 stayed and was drawn.
    let alice = knowledge(&wasm, ALICE, ALICE);
    assert_eq!(alice.last().unwrap(), &("A1".to_string(), Some((3, 3))), "{alice:?}");
    assert!(knowledge(&wasm, BOB, ALICE).iter().all(|(_, bounds)| bounds.is_none()));

    let commune = instant("Commune with Nature",
        "Look at the top five cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.");
    let mut wasm = game();
    library(&mut wasm, ALICE, &["A7", "A6", "A5", "A4", "A3", "A2", "A1"]);
    resolve(&mut wasm, &commune, ALICE, Vec::new(), None, &[crate::ReplayDecisionAnswer::Objects(Vec::new())]);
    let alice = knowledge(&wasm, ALICE, ALICE);
    assert!(alice[..2].iter().all(|(_, bounds)| bounds.is_none()), "{alice:?}");
    assert!(alice[2..].iter().all(|(_, bounds)| *bounds == Some((2, 6))), "{alice:?}");
    assert!(knowledge(&wasm, BOB, ALICE).iter().all(|(_, bounds)| bounds.is_none()));
}

#[test]
fn griptide_then_fateseal_moves_the_known_card_out_of_its_owners_reach() {
    let _ids = crate::test_id_counter_guard();
    let griptide = instant("Griptide", "Put target creature on top of its owner's library.");
    let mut wasm = game();
    library(&mut wasm, BOB, &["B3", "B2", "B1"]);
    let bears = put(&mut wasm, BOB, "Grizzly Bears", Zone::Battlefield);
    resolve(&mut wasm, &griptide, ALICE, vec![Target::Object(bears)], None, &[]);
    for viewer in [ALICE, BOB] {
        assert_eq!(knowledge(&wasm, viewer, BOB)[0], ("Grizzly Bears".to_string(), Some((0, 0))));
    }

    // Alice fateseals the Bears to the bottom: she knows, Bob doesn't.
    let fateseal = instant("Fateseal", "Fateseal 2.");
    let top = wasm.game.player(BOB).unwrap().library.last().copied().unwrap();
    resolve(&mut wasm, &fateseal, ALICE, Vec::new(), None, &[
        crate::ReplayDecisionAnswer::Partition(vec![top]),
    ]);
    let alice = knowledge(&wasm, ALICE, BOB);
    assert_eq!(alice.last().unwrap(), &("Grizzly Bears".to_string(), Some((3, 3))), "{alice:?}");
    assert!(knowledge(&wasm, BOB, BOB).iter().all(|(_, bounds)| bounds.is_none()),
        "Bob can't know where Alice put it: {:?}", knowledge(&wasm, BOB, BOB));
}

#[test]
fn tutors_forget_and_mystical_tutor_teaches_the_new_top() {
    let _ids = crate::test_id_counter_guard();
    let brainstorm = instant("Brainstorm",
        "Draw three cards, then put two cards from your hand on top of your library in any order.");
    let mut wasm = game();
    library(&mut wasm, ALICE, &["A6", "A5", "A4", "A3", "Lightning Bolt"]);
    let force = put(&mut wasm, ALICE, "Force", Zone::Hand);
    let daze = put(&mut wasm, ALICE, "Daze", Zone::Hand);
    resolve(&mut wasm, &brainstorm, ALICE, Vec::new(), None, &[
        crate::ReplayDecisionAnswer::Objects(vec![force, daze]),
        crate::ReplayDecisionAnswer::Order(vec![daze, force]),
    ]);
    let tutor = instant("Mystical Tutor",
        "Search your library for an instant or sorcery card, reveal it, then shuffle and put that card on top.");
    let target = wasm.game.player(ALICE).unwrap().library.iter().copied()
        .find(|id| wasm.game.object(*id).unwrap().name == "A6").unwrap();
    resolve(&mut wasm, &tutor, ALICE, Vec::new(), None, &[crate::ReplayDecisionAnswer::Objects(vec![target])]);
    for viewer in [ALICE, BOB] {
        let known = knowledge(&wasm, viewer, ALICE);
        assert_eq!(known[0], ("A6".to_string(), Some((0, 0))), "{viewer:?}: {known:?}");
        assert!(known[1..].iter().all(|(_, bounds)| bounds.is_none()), "the shuffle forgets Brainstorm: {known:?}");
    }
}
