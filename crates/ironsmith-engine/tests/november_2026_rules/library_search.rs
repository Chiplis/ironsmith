use super::{A, game};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use ironsmith::{
    DecisionMaker, GameState, ManaSymbol, ObjectId, PlayerId, StackEntry, Target, TriggerQueue,
    Zone,
};

const B: PlayerId = PlayerId(1);

fn put(g: &mut GameState, name: &str, text: &str, who: PlayerId, zone: Zone) -> ObjectId {
    let card =
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap();
    g.create_object_from_definition(&card, who, zone)
}
fn resolve(
    g: &mut GameState,
    name: &str,
    text: &str,
    targets: Vec<Target>,
    dm: &mut impl DecisionMaker,
) {
    let id = put(g, name, text, A, Zone::Stack);
    let mut entry = StackEntry::new(id, A);
    entry.targets = targets;
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(g, dm).unwrap();
}
fn watcher(g: &mut GameState) -> ObjectId {
    put(
        g,
        "Archivist of Oghma",
        "Type: Creature — Halfling Cleric\nPower/Toughness: 2/2\nFlash\nWhenever an opponent searches their library, you gain 1 life and draw a card.",
        B,
        Zone::Battlefield,
    )
}
fn mindcensor(g: &mut GameState) {
    put(
        g,
        "Aven Mindcensor",
        "Type: Creature — Bird Wizard\nPower/Toughness: 2/1\nFlash\nFlying\nIf an opponent would search a library, that player searches the top four cards of that library instead.",
        B,
        Zone::Battlefield,
    );
}
fn trigger_count(g: &mut GameState, source: ObjectId) -> usize {
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(g, &mut q).unwrap();
    q.entries
        .iter()
        .filter(|entry| entry.source == source)
        .count()
}

#[derive(Default)]
struct SearchChoices {
    minimum: bool,
    decline_cards: bool,
    viewed: Vec<ObjectId>,
    wurm_offer: bool,
}
impl DecisionMaker for SearchChoices {
    fn view_cards(
        &mut self,
        _: &GameState,
        _: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        if ctx.zone == Zone::Library && !ctx.public {
            self.viewed.extend(cards);
        }
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        if ctx.description.contains("cast Panglacial Wurm") {
            self.wurm_offer = true;
        }
        ctx.can_accept
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if self.decline_cards {
            Vec::new()
        } else if self.minimum {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .take(ctx.min)
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(g, ctx)
        }
    }
}

#[test]
fn successive_searches_share_the_original_top_four_and_trigger_once() {
    let mut g = game();
    mindcensor(&mut g);
    let w = watcher(&mut g);
    let fifth = put(
        &mut g,
        "Opt",
        "Mana cost: {U}\nType: Instant\nScry 1.\nDraw a card.",
        A,
        Zone::Library,
    );
    for name in ["Forest", "Forest"] {
        put(&mut g, name, "Type: Basic Land — Forest", A, Zone::Library);
    }
    for (name, cost) in [("Two", "{1}{U}"), ("Three", "{2}{U}")] {
        put(
            &mut g,
            name,
            &format!("Mana cost: {cost}\nType: Instant\nDraw a card."),
            A,
            Zone::Library,
        );
    }
    let mut dm = SearchChoices::default();
    resolve(
        &mut g,
        "Firemind's Foresight",
        "Type: Instant\nSearch your library for an instant card with mana value 3, reveal it, and put it into your hand. Then repeat this process for instant cards with mana values 2 and 1. Then shuffle.",
        vec![],
        &mut dm,
    );
    assert_eq!(trigger_count(&mut g, w), 1);
    assert_eq!(g.player(A).unwrap().hand.len(), 2);
    assert!(g.player(A).unwrap().library.contains(&fifth));
    assert!(!dm.viewed.contains(&fifth));
}

#[test]
fn blocked_mandatory_search_still_shuffles_and_finds_public_matches() {
    for (blocked, decline_cards) in [(false, false), (true, false), (true, true)] {
        let mut g = game();
        g.set_auto_choose_single_object_decisions(false);
        if blocked {
            put(
                &mut g,
                "Mindlock Orb",
                "Type: Artifact\nPlayers can't search libraries.",
                B,
                Zone::Battlefield,
            );
        }
        let w = watcher(&mut g);
        for _ in 0..4 {
            put(
                &mut g,
                "Island",
                "Type: Basic Land — Island",
                A,
                Zone::Library,
            );
        }
        for (name, text) in [
            ("Forest", "Type: Basic Land — Forest"),
            (
                "Brambleweft Behemoth",
                "Type: Creature — Elemental\nPower/Toughness: 6/6",
            ),
            (
                "Nissa, Genesis Mage",
                "Type: Planeswalker — Nissa\nLoyalty: 5",
            ),
        ] {
            put(&mut g, name, text, A, Zone::Graveyard);
        }
        let before = g.irreversible_random_count();
        resolve(
            &mut g,
            "Nissa's Encouragement",
            "Type: Sorcery\nSearch your library and graveyard for a card named Forest, a card named Brambleweft Behemoth, and a card named Nissa, Genesis Mage. Reveal those cards, put them into your hand, then shuffle.",
            vec![],
            &mut SearchChoices {
                minimum: true,
                decline_cards,
                ..Default::default()
            },
        );
        assert_eq!(g.player(A).unwrap().hand.len(), 3);
        assert!(g.irreversible_random_count() > before);
        assert_eq!(trigger_count(&mut g, w), usize::from(!blocked));
    }
}

#[test]
fn searching_an_opponents_library_grants_neither_a_wurm_cast_nor_an_archivist_trigger() {
    let mut g = game();
    let w = watcher(&mut g);
    g.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Green, 7);
    let wurm = put(
        &mut g,
        "Panglacial Wurm",
        "Mana cost: {5}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 9/5\nTrample\nWhile you're searching your library, you may cast this card from your library.",
        B,
        Zone::Library,
    );
    let mut dm = SearchChoices::default();
    resolve(
        &mut g,
        "Bribery",
        "Type: Sorcery\nSearch target opponent's library for a creature card and put that card onto the battlefield under your control. Then that player shuffles.",
        vec![Target::Player(B)],
        &mut dm,
    );
    assert!(dm.viewed.contains(&wurm));
    assert!(!dm.wurm_offer);
    assert!(g.stack.is_empty());
    assert_eq!(trigger_count(&mut g, w), 0);
}

fn opposition_agent(g: &mut GameState, controller: PlayerId) -> ObjectId {
    put(
        g,
        "Opposition Agent",
        "Type: Creature — Human Rogue\nPower/Toughness: 3/2\nFlash\nYou control your opponents while they're searching their libraries.\nWhile an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.",
        controller,
        Zone::Battlefield,
    )
}

fn resolve_tutor(g: &mut GameState) {
    resolve(
        g,
        "Demonic Tutor",
        "Type: Sorcery\nSearch your library for a card, put that card into your hand, then shuffle.",
        vec![],
        &mut SelectFirstDecisionMaker,
    );
}

#[test]
fn opposition_agent_controls_only_opponents_within_range() {
    for teams in [true, false] {
        for controller in [B, PlayerId(2)] {
            let seats = if teams { 4 } else { 5 };
            let mut g = GameState::new((0..seats).map(|seat| format!("Seat {seat}")).collect(), 20);
            if teams {
                g.enable_two_headed_giant(vec![vec![A, B], vec![PlayerId(2), PlayerId(3)]])
                    .unwrap();
            } else {
                g.enable_limited_range_of_influence(
                    (0..seats).map(PlayerId).collect(),
                    vec![1; seats as usize],
                )
                .unwrap();
            }
            opposition_agent(&mut g, controller);
            let card = put(
                &mut g,
                "Forest",
                "Type: Basic Land — Forest",
                A,
                Zone::Library,
            );
            let physical_card = g.object(card).unwrap().stable_id;
            resolve_tutor(&mut g);
            let found = g.find_object_by_stable_id(physical_card).unwrap();
            let should_exile = if teams {
                controller != B
            } else {
                controller == B
            };
            assert_eq!(
                g.object(found).unwrap().zone,
                if should_exile {
                    Zone::Exile
                } else {
                    Zone::Hand
                }
            );
        }
    }
}

#[test]
fn agent_spending_permission_follows_the_current_exile_object_including_land_fronts() {
    let mut g = game();
    let agent = opposition_agent(&mut g, B);
    let mut town = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Midgar, City of Mako",
        "Type: Land — Town\nThis land enters tapped.\n{T}: Add {B}.",
        false,
    )
    .unwrap();
    let mut raid = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Reactor Raid", "Mana cost: {2}{B}\nType: Sorcery — Adventure\nYou may sacrifice an artifact or creature. If you do, draw two cards.", false,
    ).unwrap();
    town.card.other_face = Some(raid.card.id);
    raid.card.other_face = Some(town.card.id);
    town.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    raid.card.linked_face_layout = town.card.linked_face_layout;
    g.register_linked_face_definition(&town);
    g.register_linked_face_definition(&raid);
    let card = g.create_object_from_definition(&town, A, Zone::Library);
    let physical_card = g.object(card).unwrap().stable_id;
    resolve_tutor(&mut g);
    let exiled = g.find_object_by_stable_id(physical_card).unwrap();
    assert_eq!(g.object(exiled).unwrap().zone, Zone::Exile);
    assert!(g.can_spend_mana_as_any_color(B, Some(exiled)));
    g.move_object_by_effect(agent, Zone::Graveyard).unwrap();
    assert!(g.can_spend_mana_as_any_color(B, Some(exiled)));
    g.turn.active_player = B;
    g.turn.priority_player = Some(B);
    g.player_mut(B)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    assert!(ironsmith::compute_legal_actions(&g, B).unwrap().into_iter().any(|action| {
        matches!(action, ironsmith::LegalAction::CastSpell { spell_id, .. } if spell_id == exiled)
    }));
    let returned = g.move_object_by_effect(exiled, Zone::Library).unwrap();
    let exiled_again = g.move_object_by_effect(returned, Zone::Exile).unwrap();
    assert!(!g.can_spend_mana_as_any_color(B, Some(exiled_again)));
}

#[test]
fn an_intervening_shuffle_starts_a_new_search() {
    let mut g = game();
    let w = watcher(&mut g);
    put(
        &mut g,
        "Forest",
        "Type: Basic Land — Forest",
        A,
        Zone::Library,
    );
    put(
        &mut g,
        "Opt",
        "Mana cost: {U}\nType: Instant\nScry 1.\nDraw a card.",
        A,
        Zone::Library,
    );
    resolve(
        &mut g,
        "Two shuffles",
        "Type: Sorcery\nSearch your library for a basic land card, reveal it, put it into your hand, then shuffle. Search your library for an instant card, reveal it, put it into your hand, then shuffle.",
        vec![],
        &mut SelectFirstDecisionMaker,
    );
    assert_eq!(g.player(A).unwrap().hand.len(), 2);
    assert_eq!(trigger_count(&mut g, w), 2);
}

#[test]
fn wurm_can_be_cast_from_within_the_searchable_four() {
    for wurm_on_top in [true, false] {
        let mut g = game();
        mindcensor(&mut g);
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 7);
        for position in 0..5 {
            if position == if wurm_on_top { 4 } else { 0 } {
                put(
                    &mut g,
                    "Panglacial Wurm",
                    "Mana cost: {5}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 9/5\nTrample\nWhile you're searching your library, you may cast this card from your library.",
                    A,
                    Zone::Library,
                );
            } else {
                put(
                    &mut g,
                    "Forest",
                    "Type: Basic Land — Forest",
                    A,
                    Zone::Library,
                );
            }
        }
        let mut dm = SearchChoices::default();
        resolve(
            &mut g,
            "Cultivate",
            "Type: Sorcery\nSearch your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle.",
            vec![],
            &mut dm,
        );
        assert_eq!(dm.wurm_offer, wurm_on_top);
        assert_eq!(
            g.stack
                .iter()
                .any(|spell| g.current_name(spell.object_id).as_deref() == Some("Panglacial Wurm")),
            wurm_on_top
        );
    }
}
