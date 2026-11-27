//! Simultaneous search timing, including which library identities are visible at each choice.
use super::{A, card};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, ViewCardsContext};
use ironsmith::{
    AbilityKind, CardDefinition, DecisionMaker, GameState, ObjectId, PlayerId, StackEntry, Target,
    Zone,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Offer(PlayerId),
    Look(PlayerId, PlayerId),
    Find(PlayerId),
    Reveal(PlayerId),
}
use Step::*;

#[derive(Default)]
struct SearchChoices {
    steps: Vec<Step>,
    views: Vec<(PlayerId, PlayerId, Vec<ObjectId>)>,
    libraries_at_choice: Vec<Vec<Vec<ObjectId>>>,
    hands_at_reveal: Vec<Vec<usize>>,
    declined: Vec<PlayerId>,
    pause_on: Option<PlayerId>,
    paused: bool,
    chosen: Vec<(PlayerId, ObjectId)>,
}

impl DecisionMaker for SearchChoices {
    fn awaiting_choice(&self) -> bool {
        self.paused
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        if ctx.player == A {
            1.clamp(ctx.min, ctx.max)
        } else {
            ctx.min
        }
    }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.steps.push(Offer(ctx.player));
        !self.declined.contains(&ctx.player)
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &ViewCardsContext,
    ) {
        if ctx.zone != Zone::Library {
            return;
        }
        if ctx.public {
            if viewer == A {
                self.steps.push(Reveal(ctx.subject));
                self.hands_at_reveal
                    .push(g.players.iter().map(|p| p.hand.len()).collect());
            }
        } else {
            // A decision may display already inspected cards again when asking which one to find.
            if !self
                .views
                .iter()
                .any(|(v, s, _)| (*v, *s) == (viewer, ctx.subject))
            {
                self.steps.push(Look(viewer, ctx.subject));
            }
            self.views.push((viewer, ctx.subject, cards.to_vec()));
        }
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.steps.push(Find(ctx.player));
        self.libraries_at_choice
            .push(g.players.iter().map(|p| p.library.to_vec()).collect());
        self.paused = self.pause_on == Some(ctx.player);
        let result: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .take(1)
            .map(|c| c.id)
            .collect();
        self.chosen
            .extend(result.iter().map(|id| (ctx.player, *id)));
        result
    }
}

fn board(seats: u8) -> GameState {
    let mut g = GameState::new((0..seats).map(|i| format!("P{i}")).collect(), 20);
    let waste = card("wastes");
    let bear = card("grizzly-bears");
    for i in 0..seats {
        for def in [&waste, &bear, &waste] {
            g.create_object_from_definition(def, PlayerId(i), Zone::Library);
        }
    }
    g
}
fn ability(g: &mut GameState, def: &CardDefinition) -> StackEntry {
    let source = g.create_object_from_definition(def, A, Zone::Battlefield);
    let effects = g
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t.effects.clone()),
            _ => None,
        })
        .unwrap();
    StackEntry::ability(source, A, effects)
}
fn resolution(g: &mut GameState, entry: StackEntry, dm: &mut SearchChoices) {
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(g, dm).unwrap();
}

#[test]
fn group_searches_across_seating_and_acceptance_choices() {
    let cases = [
        ("veteran-explorer", true, false),
        ("jungle-wayfinder", true, false),
        ("noble-benefactor", true, false),
        ("avatar-of-growth", false, false),
        ("old-growth-dryads", true, true),
        ("boldwyr-heavyweights", true, true),
        ("hired-giant", true, true),
        ("rootweaver-druid", true, true),
    ]
    .map(|(slug, optional, opponents)| (slug, card(slug), optional, opponents));
    for seats in 2..=5 {
        let board = board(seats);
        let libraries: Vec<_> = board.players.iter().map(|p| p.library.to_vec()).collect();
        for (route, def, optional, opponents) in &cases {
            let mut base = board.clone();
            let entry = ability(&mut base, def);
            for active in 0..seats {
                let eligible: Vec<_> = (active..seats)
                    .chain(0..active)
                    .map(PlayerId)
                    .filter(|p| !*opponents || *p != A)
                    .collect();
                let all = (1u32 << eligible.len()) - 1;
                for mask in (if *optional { 0 } else { all })..=all {
                    let mut yes = Vec::new();
                    let mut no = Vec::new();
                    for (bit, p) in eligible.iter().copied().enumerate() {
                        if mask & (1 << bit) != 0 {
                            yes.push(p);
                        } else {
                            no.push(p);
                        }
                    }
                    let mut g = base.clone();
                    g.turn.active_player = PlayerId(active);
                    let mut dm = SearchChoices {
                        declined: no,
                        ..Default::default()
                    };
                    resolution(&mut g, entry.clone(), &mut dm);
                    let mut expected = Vec::new();
                    if *optional {
                        expected.extend(eligible.iter().copied().map(Offer));
                    }
                    expected.extend(yes.iter().map(|&p| Look(p, p)));
                    expected.extend(yes.iter().copied().map(Find));
                    if *route == "jungle-wayfinder" {
                        expected.extend(yes.iter().copied().map(Reveal));
                    }
                    assert_eq!(
                        dm.steps, expected,
                        "{route} seats={seats} active={active} mask={mask}"
                    );
                    assert!(
                        dm.libraries_at_choice.iter().all(|l| l == &libraries),
                        "{route}"
                    );
                    assert!(
                        dm.hands_at_reveal.iter().flatten().all(|count| *count == 0),
                        "{route}"
                    );
                    for p in &g.players {
                        assert_eq!(
                            p.library.len(),
                            3 - usize::from(yes.contains(&p.id)),
                            "{route}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn optional_and_mandatory_searchers_share_their_inspection_step() {
    for (route, optional, opponents) in [
        ("avatar-of-growth", false, false),
        ("old-growth-dryads", true, true),
        ("boldwyr-heavyweights", true, true),
        ("hired-giant", true, true),
        ("noble-benefactor", true, false),
        ("rootweaver-druid", true, true),
        ("jungle-wayfinder", true, false),
    ] {
        let mut g = board(3);
        g.turn.active_player = PlayerId(1);
        let entry = ability(&mut g, &card(route));
        let order: Vec<_> = [PlayerId(1), PlayerId(2), A]
            .into_iter()
            .filter(|p| !opponents || *p != A)
            .collect();
        let physical: std::collections::HashMap<_, _> = g
            .players
            .iter()
            .flat_map(|p| {
                p.library
                    .iter()
                    .map(|id| (*id, g.object(*id).unwrap().stable_id))
            })
            .collect();
        let mut dm = SearchChoices::default();
        resolution(&mut g, entry, &mut dm);
        let mut expected = Vec::new();
        if optional {
            expected.extend(order.iter().copied().map(Offer));
        }
        expected.extend(order.iter().map(|&p| Look(p, p)));
        expected.extend(order.iter().copied().map(Find));
        if route == "jungle-wayfinder" {
            expected.extend(order.iter().copied().map(Reveal));
            assert!(
                dm.hands_at_reveal.iter().all(|h| h == &[0, 0, 0]),
                "{route}"
            );
            assert!(g.players.iter().all(|p| p.hand.len() == 1));
        }
        for (searcher, chosen) in &dm.chosen {
            let id = g.find_object_by_stable_id(physical[chosen]).unwrap();
            let obj = g.object(id).unwrap();
            if matches!(route, "jungle-wayfinder" | "noble-benefactor") {
                assert!(g.player(*searcher).unwrap().hand.contains(&id), "{route}");
            } else {
                assert_eq!(obj.zone, Zone::Battlefield, "{route}");
                assert_eq!(
                    g.controller_of(obj),
                    if route == "rootweaver-druid" {
                        A
                    } else {
                        *searcher
                    },
                    "{route}"
                );
                assert_eq!(
                    g.is_tapped(id),
                    matches!(route, "old-growth-dryads" | "rootweaver-druid"),
                    "{route}"
                );
            }
        }
        assert_eq!(dm.steps, expected, "{route}");
        assert!(
            dm.libraries_at_choice.windows(2).all(|w| w[0] == w[1]),
            "{route}"
        );
    }
}

#[test]
fn scheming_symmetry_searches_only_the_two_targets_and_places_their_exact_cards_on_top() {
    let mut base = board(4);
    let source = base.create_object_from_definition(&card("scheming-symmetry"), A, Zone::Stack);
    for first in 0..4 {
        for a in 0..4 {
            for b in 0..4 {
                if a == b {
                    continue;
                }
                let mut g = base.clone();
                g.turn.active_player = PlayerId(first);
                let mut entry = StackEntry::new(source, A);
                entry.targets = vec![Target::Player(PlayerId(a)), Target::Player(PlayerId(b))];
                let mut dm = SearchChoices::default();
                let before: Vec<_> = g.players.iter().map(|p| p.library.to_vec()).collect();
                resolution(&mut g, entry, &mut dm);
                let order: Vec<_> = (first..4)
                    .chain(0..first)
                    .filter(|p| *p == a || *p == b)
                    .map(PlayerId)
                    .collect();
                assert_eq!(
                    dm.steps,
                    order
                        .iter()
                        .map(|&p| Look(p, p))
                        .chain(order.iter().copied().map(Find))
                        .collect::<Vec<_>>(),
                    "active={first} targets={a},{b}"
                );
                for p in &g.players {
                    if order.contains(&p.id) {
                        let found = dm.chosen.iter().find(|(who, _)| *who == p.id).unwrap().1;
                        assert_eq!(p.library.last(), Some(&found));
                    } else {
                        assert_eq!(p.library.to_vec(), before[p.id.0 as usize]);
                    }
                }
            }
        }
    }
}

#[test]
fn searchers_keep_their_own_restrictions_cast_offers_and_control() {
    let b = PlayerId(1);
    let c = PlayerId(2);
    for (route, source) in [
        ("aven-mindcensor", b),
        ("stranglehold", b),
        ("opposition-agent", b),
    ] {
        let mut g = board(3);
        g.turn.active_player = c;
        let basic = card("wastes");
        for player in [A, b, c] {
            for _ in 0..3 {
                g.create_object_from_definition(&basic, player, Zone::Library);
            }
        }
        g.create_object_from_definition(&card(route), source, Zone::Battlefield);
        let entry = ability(&mut g, &card("avatar-of-growth"));
        let libraries: Vec<_> = g.players.iter().map(|p| p.library.to_vec()).collect();
        let mut dm = SearchChoices::default();
        resolution(&mut g, entry, &mut dm);
        let first_find = dm.steps.iter().position(|s| matches!(s, Find(_))).unwrap();
        for player in [A, b, c] {
            let expected = match route {
                "stranglehold" if player != b => 0,
                "aven-mindcensor" if player != b => 4,
                _ => 6,
            };
            let seen = dm
                .views
                .iter()
                .find(|(viewer, owner, _)| *viewer == player && *owner == player);
            if expected == 0 {
                assert!(seen.is_none());
                continue;
            }
            let seen = &seen.unwrap().2;
            assert_eq!(seen.len(), expected, "{route} {player:?}");
            let allowed = &libraries[player.0 as usize][6 - expected..];
            assert!(seen.iter().all(|id| allowed.contains(id)));
            assert!(dm.steps[..first_find].contains(&Look(player, player)));
            if route == "opposition-agent" && player != b {
                assert!(dm.steps[..first_find].contains(&Look(b, player)));
            }
        }
        if route == "opposition-agent" {
            assert_eq!(g.exile.len(), 2);
        }
        if route == "stranglehold" {
            assert_eq!(dm.steps.iter().filter(|s| matches!(s, Find(_))).count(), 1);
        }
    }
    let mut g = board(3);
    g.turn.active_player = b;
    let wurm = g.create_object_from_definition(&card("panglacial-wurm"), b, Zone::Library);
    g.player_mut(b)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::Green, 7);
    let entry = ability(&mut g, &card("avatar-of-growth"));
    let mut dm = SearchChoices::default();
    resolution(&mut g, entry, &mut dm);
    let cast = dm.steps.iter().position(|s| *s == Offer(b)).unwrap();
    for p in [A, b, c] {
        assert!(dm.steps[..cast].contains(&Look(p, p)));
    }
    assert!(g.stack.iter().any(|entry| {
        g.object(entry.object_id)
            .is_some_and(|obj| obj.name == "Panglacial Wurm")
    }));
    assert!(!g.player(b).unwrap().library.contains(&wurm));
}

#[test]
fn targeted_variable_and_conditional_searches_wait_for_every_eligible_player() {
    let b = PlayerId(1);
    let c = PlayerId(2);
    for route in [
        "field-of-ruin",
        "collective-voyage",
        "pir-s-whim",
        "turtle-tracks",
        "weird-harvest",
        "natural-balance",
        "new-frontiers",
        "scholarship-sponsor",
    ] {
        let mut g = board(3);
        g.turn.active_player = b;
        if route == "natural-balance" {
            let basic = card("wastes");
            for (player, count) in [(A, 5), (b, 4)] {
                for _ in 0..count {
                    g.create_object_from_definition(&basic, player, Zone::Battlefield);
                }
            }
        }
        if route == "collective-voyage" {
            g.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Green, 1);
        }
        let entry = match route {
            "field-of-ruin" => {
                let def = card(route);
                let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
                let target = g.create_object_from_definition(&def, c, Zone::Battlefield);
                let program = g
                    .object(source)
                    .unwrap()
                    .abilities
                    .iter()
                    .find_map(|a| match &a.kind {
                        AbilityKind::Activated(t) if t.mana_output.is_none() => {
                            Some(t.effects.clone())
                        }
                        _ => None,
                    })
                    .unwrap();
                let mut entry = StackEntry::ability(source, A, program);
                entry.targets = vec![Target::Object(target)];
                entry
            }
            "scholarship-sponsor" => {
                g.create_object_from_definition(&card("wastes"), A, Zone::Battlefield);
                ability(&mut g, &card(route))
            }
            _ => {
                let source = g.create_object_from_definition(&card(route), A, Zone::Stack);
                let mut entry = StackEntry::new(source, A);
                entry.x_value = Some(1);
                if route == "turtle-tracks" {
                    entry.targets = vec![Target::Player(A), Target::Player(c)];
                }
                entry
            }
        };
        let mut dm = SearchChoices::default();
        resolution(&mut g, entry, &mut dm);
        let expected = match route {
            "turtle-tracks" => vec![c, A],
            "scholarship-sponsor" | "natural-balance" => vec![b, c],
            _ => vec![b, c, A],
        };
        let first = dm.steps.iter().position(|s| matches!(s, Find(_))).unwrap();
        for &p in &expected {
            assert!(
                dm.steps[..first].contains(&Look(p, p)),
                "{route} {:?}",
                dm.steps
            );
        }
        let finds: Vec<_> = dm
            .steps
            .iter()
            .filter_map(|s| match s {
                Find(p) => Some(*p),
                _ => None,
            })
            .collect();
        assert_eq!(finds, expected, "{route}");
        if route == "natural-balance" {
            assert_eq!(
                dm.steps,
                [Offer(b), Offer(c), Look(b, b), Look(c, c), Find(b), Find(c)]
            );
            for (player, expected_lands) in [(A, 5), (b, 5), (c, 1)] {
                let lands = g
                    .battlefield
                    .iter()
                    .filter(|id| {
                        g.object(**id)
                            .is_some_and(|o| o.name == "Wastes" && g.controller_of(o) == player)
                    })
                    .count();
                assert_eq!(lands, expected_lands);
            }
        }
    }
}

#[test]
fn suspending_the_last_find_keeps_all_found_cards_private_and_in_their_libraries() {
    use ironsmith::effects::{ForPlayersEffect, SearchLibraryEffect};
    use ironsmith::{Effect, EffectContext, EffectExecutor, ObjectFilter, PlayerFilter};
    let mut g = board(3);
    let original: Vec<_> = g.players.iter().map(|p| p.library.to_vec()).collect();
    let src = g.new_object_id();
    let search = ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::new(SearchLibraryEffect::to_hand(
            ObjectFilter::land().in_zone(Zone::Library),
            PlayerFilter::IteratedPlayer,
            true,
        ))],
    );
    let mut dm = SearchChoices {
        pause_on: Some(PlayerId(2)),
        ..Default::default()
    };
    let result = search
        .execute(&mut g, &mut EffectContext::new(src, A, &mut dm))
        .unwrap();
    assert!(dm.paused);
    assert!(result.events.is_empty());
    assert!(!dm.steps.iter().any(|s| matches!(s, Reveal(_))));
    for p in &g.players {
        assert_eq!(p.library.to_vec(), original[p.id.0 as usize]);
        assert!(p.hand.is_empty());
    }
    let mut dm = SearchChoices::default();
    let result = search
        .execute(&mut g, &mut EffectContext::new(src, A, &mut dm))
        .unwrap();
    let searches = result
        .events
        .iter()
        .filter_map(|e| e.downcast::<ironsmith::events::SearchLibraryEvent>())
        .map(|e| e.player)
        .collect::<Vec<_>>();
    let history: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| {
            if let Some(e) = event.downcast::<ironsmith::events::SearchLibraryEvent>() {
                return Some(('s', e.player));
            }
            if let Some(e) = event.downcast::<ironsmith::events::CardRevealedEvent>() {
                return Some(('r', e.player));
            }
            event
                .downcast::<ironsmith::events::ShuffleLibraryEvent>()
                .map(|e| ('h', e.player))
        })
        .collect();
    let expected: Vec<_> = ['s', 'r', 'h']
        .into_iter()
        .flat_map(|step| [A, PlayerId(1), PlayerId(2)].map(|p| (step, p)))
        .collect();
    assert_eq!(history, expected);
    assert_eq!(searches, [A, PlayerId(1), PlayerId(2)]);
    assert!(g.players.iter().all(|p| p.hand.len() == 1));
    assert!(dm.hands_at_reveal.iter().all(|h| h == &[0, 0, 0]));
}

#[test]
fn borderland_explorer_offers_the_search_only_to_players_who_discarded() {
    let mut g = board(3);
    let b = PlayerId(1);
    let c = PlayerId(2);
    g.turn.active_player = c;
    for p in [A, c] {
        g.create_object_from_definition(&card("grizzly-bears"), p, Zone::Hand);
    }
    let entry = ability(&mut g, &card("borderland-explorer"));
    let mut dm = SearchChoices {
        declined: vec![b],
        ..Default::default()
    };
    resolution(&mut g, entry, &mut dm);
    let start = dm.steps.iter().position(|s| matches!(s, Look(..))).unwrap();
    assert_eq!(
        &dm.steps[start..],
        &[
            Look(c, c),
            Look(A, A),
            Find(c),
            Find(A),
            Reveal(c),
            Reveal(A)
        ]
    );
    for p in [A, c] {
        let hand = &g.player(p).unwrap().hand;
        assert_eq!(hand.len(), 1);
        assert_eq!(g.object(hand[0]).unwrap().name, "Wastes");
    }
    assert!(g.player(b).unwrap().hand.is_empty());
}

#[test]
fn distant_players_neither_view_cards_nor_choose_them() {
    let mut g = board(5);
    g.enable_limited_range_of_influence((0..5).map(PlayerId).collect(), vec![1; 5])
        .unwrap();
    g.turn.active_player = PlayerId(3);
    let entry = ability(&mut g, &card("avatar-of-growth"));
    let mut dm = SearchChoices::default();
    resolution(&mut g, entry, &mut dm);
    let order = [PlayerId(4), A, PlayerId(1)];
    let expected: Vec<_> = order
        .iter()
        .map(|&p| Look(p, p))
        .chain(order.into_iter().map(Find))
        .collect();
    assert_eq!(dm.steps, expected);
    for p in [PlayerId(2), PlayerId(3)] {
        assert_eq!(g.player(p).unwrap().library.len(), 3);
    }
}

#[test]
fn two_selections_before_one_shuffle_reveal_a_card_and_trigger_one_search() {
    use ironsmith::effects::{ChooseObjectsEffect, SearchLibraryEffect};
    use ironsmith::{Effect, ObjectFilter, PlayerFilter, TriggerQueue};
    let mut g = board(2);
    let source = g.create_object_from_definition(&card("wastes"), A, Zone::Battlefield);
    let observer = g.create_object_from_definition(
        &card("archivist-of-oghma"),
        PlayerId(1),
        Zone::Battlefield,
    );
    let filter = ObjectFilter::land()
        .in_zone(Zone::Library)
        .owned_by(PlayerFilter::You);
    let find =
        ChooseObjectsEffect::new(filter.clone(), 1, PlayerFilter::You, "first_search_choice")
            .in_zone(Zone::Library)
            .as_search();
    let search = SearchLibraryEffect::to_hand(filter, PlayerFilter::You, true);
    let entry = StackEntry::ability(source, A, vec![Effect::new(find), Effect::new(search)]);
    let mut dm = SearchChoices::default();
    resolution(&mut g, entry, &mut dm);
    assert_eq!(g.player(A).unwrap().hand.len(), 1);
    assert_eq!(
        dm.steps.iter().filter(|s| matches!(s, Reveal(_))).count(),
        1
    );
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|t| t.source == observer)
            .count(),
        1
    );
}

#[test]
fn tempting_offer_resolves_the_controllers_own_search_before_the_opponents_search_together() {
    let b = PlayerId(1);
    let c = PlayerId(2);
    let mut base = board(3);
    base.turn.active_player = c;
    let basic = card("wastes");
    for _ in 0..4 {
        base.create_object_from_definition(&basic, A, Zone::Library);
    }
    let source = base.create_object_from_definition(&card("tempt-with-discovery"), A, Zone::Stack);
    for mask in 0..4 {
        let mut g = base.clone();
        let (yes, no): (Vec<_>, Vec<_>) = [c, b].into_iter().partition(|p| {
            if *p == b {
                mask & 1 != 0
            } else {
                mask & 2 != 0
            }
        });
        let before: Vec<_> = g.players.iter().map(|p| p.library.len()).collect();
        let mut dm = SearchChoices {
            declined: no,
            ..Default::default()
        };
        resolution(&mut g, StackEntry::new(source, A), &mut dm);
        let mut expected = vec![Look(A, A), Find(A), Offer(c), Offer(b)];
        expected.extend(yes.iter().map(|&p| Look(p, p)));
        expected.extend(yes.iter().copied().map(Find));
        expected.extend(yes.iter().map(|_| Find(A)));
        assert_eq!(dm.steps, expected, "mask={mask}");
        for p in &g.players {
            let removed = if p.id == A {
                1 + yes.len()
            } else {
                usize::from(yes.contains(&p.id))
            };
            assert_eq!(p.library.len() + removed, before[p.id.0 as usize]);
        }
    }
}

#[test]
fn individual_searches_preserve_reveals_destinations_and_the_final_shuffle() {
    use ironsmith::effects::SearchLibraryEffect;
    use ironsmith::{EffectContext, EffectExecutor, ObjectFilter, PlayerFilter};
    struct Finder(bool);
    impl DecisionMaker for Finder {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
            if self.0 {
                ctx.candidates
                    .iter()
                    .rev()
                    .find(|c| c.legal)
                    .map(|c| c.id)
                    .into_iter()
                    .collect()
            } else {
                vec![]
            }
        }
    }
    let basic = card("wastes");
    let bear = card("grizzly-bears");
    for count in 0..=5 {
        let mut base = GameState::new(vec!["A".into(), "B".into()], 20);
        let src = base.new_object_id();
        let mut lands = Vec::new();
        for i in 0..count {
            let land = i % 2 == 0;
            let id = base.create_object_from_definition(
                if land { &basic } else { &bear },
                A,
                Zone::Library,
            );
            if land {
                lands.push(id);
            }
        }
        for destination in [
            Zone::Hand,
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
        ] {
            for reveal in [false, true] {
                for choose in [false, true] {
                    let mut g = base.clone();
                    let effect = SearchLibraryEffect::new(
                        ObjectFilter::land().in_zone(Zone::Library),
                        destination,
                        PlayerFilter::You,
                        PlayerFilter::You,
                        reveal,
                    );
                    let mut dm = Finder(choose);
                    let outcome = effect
                        .execute(&mut g, &mut EffectContext::new(src, A, &mut dm))
                        .unwrap();
                    let found = lands.last().filter(|_| choose);
                    let observations: Vec<_> = outcome
                        .events
                        .iter()
                        .filter_map(|e| {
                            if e.downcast::<ironsmith::events::SearchLibraryEvent>()
                                .is_some()
                            {
                                Some("search")
                            } else if e
                                .downcast::<ironsmith::events::CardRevealedEvent>()
                                .is_some()
                            {
                                Some("reveal")
                            } else if e
                                .downcast::<ironsmith::events::ShuffleLibraryEvent>()
                                .is_some()
                            {
                                Some("shuffle")
                            } else {
                                None
                            }
                        })
                        .collect();
                    let expected = if reveal && found.is_some() {
                        vec!["search", "reveal", "shuffle"]
                    } else {
                        vec!["search", "shuffle"]
                    };
                    assert_eq!(
                        observations, expected,
                        "n={count}, {destination:?}, reveal={reveal}, find={choose}"
                    );
                    if let Some(old) = found {
                        let stable = base.object(*old).unwrap().stable_id;
                        let id = g.find_object_by_stable_id(stable).unwrap();
                        assert_eq!(g.object(id).unwrap().zone, destination);
                        if destination == Zone::Library {
                            assert_eq!(g.player(A).unwrap().library.last(), Some(&id));
                        }
                    }
                    assert_eq!(
                        g.player(A).unwrap().library.len(),
                        count - usize::from(found.is_some() && destination != Zone::Library)
                    );
                }
            }
        }
    }
}

#[test]
fn a_single_searcher_can_inspect_every_library_before_choosing_for_jaces_ultimate() {
    struct JaceChoices {
        looks: Vec<(PlayerId, PlayerId)>,
        finds: Vec<(PlayerId, usize, usize)>,
        finished_libraries: Vec<PlayerId>,
        find_in: u8,
        cast_found: bool,
        cast_offers: Vec<PlayerId>,
        play_order: Vec<PlayerId>,
        order_prompts: usize,
    }
    impl DecisionMaker for JaceChoices {
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.player, A);
            assert_eq!(
                self.looks.len(),
                3,
                "all libraries can be inspected before choosing the next one"
            );
            ctx.options
                .iter()
                .rev()
                .find(|o| o.legal)
                .map(|o| vec![o.index])
                .unwrap_or_default()
        }
        fn decide_order(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            assert_eq!(ctx.player, A);
            self.order_prompts += 1;
            let ids: Vec<_> = ctx.items.iter().rev().map(|(id, _)| *id).collect();
            self.play_order = ids
                .iter()
                .map(|id| game.object(*id).unwrap().owner)
                .collect();
            ids
        }
        fn view_cards(
            &mut self,
            _: &GameState,
            viewer: PlayerId,
            _: &[ObjectId],
            ctx: &ViewCardsContext,
        ) {
            if !ctx.public
                && ctx.zone == Zone::Library
                && !self.looks.contains(&(viewer, ctx.subject))
            {
                self.looks.push((viewer, ctx.subject));
            }
        }
        fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
            let candidate = ctx
                .candidates
                .iter()
                .find(|c| c.legal && g.object(c.id).is_some_and(|o| o.zone == Zone::Library));
            if let Some(card) = candidate {
                let owner = g.object(card.id).unwrap().owner;
                self.finished_libraries.push(owner);
                self.finds
                    .push((ctx.player, self.looks.len(), g.exile.len()));
                if self.find_in & (1 << owner.0) != 0 {
                    vec![card.id]
                } else {
                    vec![]
                }
            } else {
                let cards: Vec<_> = ctx
                    .candidates
                    .iter()
                    .filter(|c| c.legal && g.object(c.id).is_some_and(|o| o.zone == Zone::Exile))
                    .map(|c| c.id)
                    .collect();
                self.cast_offers
                    .extend(cards.iter().map(|id| g.object(*id).unwrap().owner));
                if self.cast_found {
                    self.play_order = self.cast_offers.clone();
                    cards
                } else {
                    vec![]
                }
            }
        }
    }
    let mut base = board(3);
    let source = base.create_object_from_definition(
        &card("jace-architect-of-thought"),
        A,
        Zone::Battlefield,
    );
    let program = base
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) => Some(a.effects.clone()),
            _ => None,
        })
        .last()
        .unwrap();
    for active in [0, 1, 2] {
        for find_in in 0u8..8 {
            for cast_found in [false, true] {
                let mut g = base.clone();
                g.turn.active_player = PlayerId(active);
                let mut dm = JaceChoices {
                    looks: Vec::new(),
                    finds: Vec::new(),
                    finished_libraries: Vec::new(),
                    find_in,
                    cast_found,
                    play_order: Vec::new(),
                    order_prompts: 0,
                    cast_offers: Vec::new(),
                };
                g.push_to_stack(StackEntry::ability(source, A, program.clone()));
                ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                assert_eq!(
                    dm.finds,
                    vec![(A, 3, 0); 3],
                    "active={active} private views={:?}",
                    dm.looks
                );
                let expected: Vec<_> = (active..3).chain(0..active).rev().map(PlayerId).collect();
                assert_eq!(dm.finished_libraries, expected);
                assert_eq!(
                    dm.order_prompts,
                    usize::from(cast_found && find_in.count_ones() > 1)
                );
                let actual: Vec<_> = g
                    .stack
                    .iter()
                    .map(|entry| {
                        assert_eq!(entry.controller, A);
                        g.object(entry.object_id).unwrap().owner
                    })
                    .collect();
                assert_eq!(actual, dm.play_order);
                for player in [A, PlayerId(1), PlayerId(2)] {
                    assert!(dm.looks.contains(&(A, player)));
                    let found = find_in & (1 << player.0) != 0;
                    assert_eq!(
                        dm.cast_offers.contains(&player),
                        found,
                        "cast permission follows the cards actually found"
                    );
                    assert_eq!(
                        g.exile
                            .iter()
                            .filter(|&&id| g.object(id).unwrap().owner == player)
                            .count(),
                        usize::from(found && !cast_found)
                    );
                }
            }
        }
    }
}

#[test]
fn casting_multiple_spells_during_resolution_puts_them_on_the_stack_in_the_casters_order() {
    #[derive(Default)]
    struct CastOrder {
        expected: Vec<ObjectId>,
        prompts: usize,
    }
    impl DecisionMaker for CastOrder {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| {
                    c.legal
                        && game
                            .object(c.id)
                            .is_some_and(|card| card.zone == Zone::Exile)
                })
                .take(ctx.max.unwrap_or(usize::MAX))
                .map(|c| c.id)
                .collect()
        }

        fn decide_order(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            assert_eq!(ctx.player, A);
            self.prompts += 1;
            let cards: Vec<_> = ctx.items.iter().rev().map(|(id, _)| *id).collect();
            self.expected = cards
                .iter()
                .map(|id| game.object(*id).unwrap().stable_id.0)
                .collect();
            cards
        }
    }

    let creature = card("grizzly-bears");
    let mut board = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    for player in [A, PlayerId(1), PlayerId(2)] {
        for _ in 0..3 {
            board.create_object_from_definition(&creature, player, Zone::Library);
        }
    }
    for (slug, triggered, count) in [
        ("etali-primal-storm", true, 3),
        ("etali-primal-conqueror", true, 3),
        ("primeval-spawn", true, 3),
        ("fevered-suspicion", false, 2),
        ("villainous-wealth", false, 3),
    ] {
        let mut base = board.clone();
        let mut entry = if triggered {
            ability(&mut base, &card(slug))
        } else {
            let source = base.create_object_from_definition(&card(slug), A, Zone::Stack);
            StackEntry::new(source, A)
        };
        if slug == "villainous-wealth" {
            entry.targets = vec![Target::Player(PlayerId(1))];
            entry.x_value = Some(3);
        }
        for active in 0..3 {
            let mut g = base.clone();
            g.turn.active_player = PlayerId(active);
            g.push_to_stack(entry.clone());
            let mut dm = CastOrder::default();
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(dm.prompts, 1, "{slug}, active seat {active}");
            assert_eq!(dm.expected.len(), count, "{slug}");
            let actual: Vec<_> = g
                .stack
                .iter()
                .map(|spell| {
                    assert_eq!(spell.controller, A);
                    g.object(spell.object_id).unwrap().stable_id.0
                })
                .collect();
            assert_eq!(actual, dm.expected, "{slug}, active seat {active}");
        }
    }
    // A single linked permission can refer to several cards (CR 607.2a).
    // It must ask for their play order once, under the original caster.
    let mut g = board;
    let source = g.create_object_from_definition(&card("wastes"), A, Zone::Battlefield);
    let mut linked = Vec::new();
    for owner in [A, PlayerId(1), PlayerId(2)] {
        let id = g.create_object_from_definition(&creature, owner, Zone::Exile);
        linked.push(ironsmith::snapshot::ObjectSnapshot::from_object_id(&g, id).unwrap());
    }
    let mut dm = CastOrder::default();
    let mut ctx = ironsmith::EffectContext::new(source, A, &mut dm);
    ctx.set_tagged_objects(ironsmith::tag::SOURCE_EXILED_TAG, linked);
    let permission = ironsmith::effects::CastTaggedEffect::new(
        ironsmith::tag::SOURCE_EXILED_TAG,
        ironsmith::PlayerFilter::You,
    )
    .without_paying_mana_cost();
    ironsmith::EffectExecutor::execute(&permission, &mut g, &mut ctx).unwrap();
    assert_eq!(dm.prompts, 1);
    assert_eq!(g.stack.len(), 3);
    for (spell, expected) in g.stack.iter().zip(dm.expected) {
        assert_eq!(spell.controller, A);
        assert_eq!(g.object(spell.object_id).unwrap().stable_id.0, expected);
    }
}
