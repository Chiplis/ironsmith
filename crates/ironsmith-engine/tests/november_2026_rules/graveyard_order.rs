use super::*;
use ironsmith::decisions::context::{OrderContext, PartitionContext};
use ironsmith::{StackEntry, Target};

const B: PlayerId = PlayerId(1);

#[derive(Default)]
struct Arrange {
    prompts: Vec<(PlayerId, Vec<String>)>,
    visible_graveyards: Vec<[usize; 2]>,
    pause_on: Option<PlayerId>,
    pending: bool,
    take_maximum: bool,
    vote: Option<usize>,
    middle_card: Option<&'static str>,
}

impl DecisionMaker for Arrange {
    fn decide_objects(
        &mut self,
        game: &GameState,
        choice: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if !self.take_maximum {
            return ironsmith::decision::SelectFirstDecisionMaker.decide_objects(game, choice);
        }
        choice
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .take(choice.max.unwrap_or(choice.candidates.len()))
            .collect()
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        choice: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(index) = self.vote
            && choice
                .options
                .iter()
                .any(|option| option.description.eq_ignore_ascii_case("taxes"))
        {
            return vec![index];
        }
        ironsmith::decision::SelectFirstDecisionMaker.decide_options(game, choice)
    }
    fn decide_partition(&mut self, _: &GameState, choice: &PartitionContext) -> Vec<ObjectId> {
        choice.cards.iter().map(|(id, _)| *id).collect()
    }

    fn decide_order(&mut self, game: &GameState, choice: &OrderContext) -> Vec<ObjectId> {
        self.visible_graveyards
            .push([A, B].map(|p| game.player(p).unwrap().graveyard.len()));
        let mut cards = choice.items.clone();
        cards.reverse();
        if let Some(name) = self.middle_card
            && let Some(index) = cards.iter().position(|(_, card)| card == name)
        {
            let card = cards.remove(index);
            cards.insert(1, card);
        }
        self.prompts.push((
            choice.player,
            cards.iter().map(|(_, name)| name.clone()).collect(),
        ));
        self.pending = self.pause_on == Some(choice.player);
        cards.into_iter().map(|(id, _)| id).collect()
    }

    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

fn graveyard(g: &GameState, player: PlayerId) -> Vec<String> {
    g.player(player)
        .unwrap()
        .graveyard
        .iter()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect()
}

fn put_creatures(g: &mut GameState, who: PlayerId, zone: Zone) {
    for name in ["grizzly-bears", "walking-corpse", "hill-giant"] {
        put(g, name, who, zone);
    }
}

#[test]
fn the_card_owner_orders_milled_surveilled_and_discarded_cards() {
    for (route, owner, from, targets) in [
        ("tome-scour", B, Zone::Library, vec![Target::Player(B)]),
        ("notion-rain", A, Zone::Library, vec![]),
        ("mind-rot", B, Zone::Hand, vec![Target::Player(B)]),
    ] {
        let mut g = game();
        put(&mut g, "plains", owner, Zone::Graveyard);
        put_creatures(&mut g, owner, from);
        let mut decider = Arrange::default();
        resolve_spell(&mut g, route, targets, &mut decider);
        assert_eq!(decider.prompts.len(), 1, "{route}");
        let (who, ordered) = &decider.prompts[0];
        assert_eq!(*who, owner);
        assert!(ordered.len() >= 2);
        let cards = graveyard(&g, owner);
        assert_eq!(cards[0], "Plains");
        assert_eq!(&cards[1..=ordered.len()], ordered.as_slice());
    }
}

#[test]
fn wheel_and_barter_collect_all_owners_orders_before_committing() {
    for (route, zone) in [
        ("wheel-of-fortune", Zone::Hand),
        ("barter-in-blood", Zone::Battlefield),
    ] {
        let mut g = game();
        g.turn.active_player = B;
        for p in [A, B] {
            put(&mut g, "plains", p, Zone::Graveyard);
            put_creatures(&mut g, p, zone);
        }
        let mut decider = Arrange::default();
        resolve_spell(&mut g, route, vec![], &mut decider);
        assert_eq!(
            decider
                .prompts
                .iter()
                .map(|(player, _)| *player)
                .collect::<Vec<_>>(),
            [B, A],
            "{route}"
        );
        assert_eq!(decider.visible_graveyards, [[1, 1], [1, 1]], "{route}");
        for (player, ordered) in decider.prompts {
            assert_eq!(&graveyard(&g, player)[1..=ordered.len()], ordered);
        }
    }
}

#[test]
fn lethal_damage_orders_the_cards_before_either_owners_graveyard_changes() {
    let mut g = game();
    g.turn.active_player = B;
    for p in [A, B] {
        put(&mut g, "plains", p, Zone::Graveyard);
        for name in ["grizzly-bears", "walking-corpse"] {
            let id = put(&mut g, name, p, Zone::Battlefield);
            g.mark_damage(id, 3);
        }
    }
    let mut pause = Arrange {
        pause_on: Some(A),
        ..Default::default()
    };
    assert!(
        !ironsmith::rules::state_based::apply_state_based_actions_with(&mut g, &mut pause).unwrap()
    );
    assert!(pause.pending);
    assert_eq!(g.battlefield.len(), 4);
    assert_eq!(graveyard(&g, A), ["Plains"]);
    assert_eq!(graveyard(&g, B), ["Plains"]);
    let mut decider = Arrange::default();
    assert!(
        ironsmith::rules::state_based::apply_state_based_actions_with(&mut g, &mut decider)
            .unwrap()
    );
    assert_eq!(decider.prompts, pause.prompts);
    assert_eq!(decider.visible_graveyards, [[1, 1], [1, 1]]);
    for (p, ordered) in decider.prompts {
        assert_eq!(&graveyard(&g, p)[1..], ordered);
    }
}

#[test]
fn shallow_grave_returns_the_creature_its_owner_placed_on_top_after_a_mill() {
    let mut g = game();
    put_creatures(&mut g, B, Zone::Library);
    let mut decider = Arrange::default();
    resolve_spell(&mut g, "tome-scour", vec![Target::Player(B)], &mut decider);
    let spell = put(&mut g, "shallow-grave", B, Zone::Stack);
    g.push_to_stack(StackEntry::new(spell, B));
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut decider).unwrap();
    let creatures: Vec<_> = g
        .battlefield
        .iter()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect();
    assert_eq!(creatures, ["Hill Giant"]);
}

#[test]
fn milling_a_single_card_never_reorders_older_graveyard_cards() {
    let mut g = game();
    put_creatures(&mut g, B, Zone::Graveyard);
    put(&mut g, "plains", B, Zone::Library);
    let mut decider = Arrange::default();
    resolve_spell(&mut g, "tome-scour", vec![Target::Player(B)], &mut decider);
    assert!(decider.prompts.is_empty());
    assert_eq!(
        graveyard(&g, B),
        ["Grizzly Bears", "Walking Corpse", "Hill Giant", "Plains"]
    );
}

#[test]
fn the_owner_orders_cards_selected_from_the_library_and_the_rest_of_a_pile() {
    for route in [
        "buried-alive",
        "intuition",
        "gifts-ungiven",
        "realms-uncharted",
    ] {
        let mut g = game();
        put_creatures(&mut g, A, Zone::Library);
        for name in ["plains", "island", "forest", "mountain"] {
            put(&mut g, name, A, Zone::Library);
        }
        let mut decider = Arrange {
            take_maximum: true,
            ..Default::default()
        };
        let targets = if matches!(route, "intuition" | "gifts-ungiven") {
            vec![Target::Player(B)]
        } else {
            vec![]
        };
        resolve_spell(&mut g, route, targets, &mut decider);
        assert_eq!(decider.prompts.len(), 1, "{route}");
        let (owner, order) = &decider.prompts[0];
        assert_eq!(*owner, A);
        assert_eq!(
            order.len(),
            if route == "buried-alive" { 3 } else { 2 },
            "{route}"
        );
        assert_eq!(&graveyard(&g, A)[..order.len()], order);
    }
}

#[test]
fn capital_punishment_counts_votes_before_sacrificing_or_discarding() {
    for vote in [0, 1] {
        let mut g = game();
        put_creatures(&mut g, B, Zone::Hand);
        put_creatures(&mut g, B, Zone::Battlefield);
        let mut decider = Arrange {
            vote: Some(vote),
            ..Default::default()
        };
        resolve_spell(&mut g, "capital-punishment", vec![], &mut decider);
        assert_eq!(decider.prompts.len(), 1, "vote {vote}");
        assert_eq!(decider.prompts[0].0, B);
        assert_eq!(decider.prompts[0].1.len(), 2);
        assert_eq!(graveyard(&g, B), decider.prompts[0].1);
        assert_eq!(
            g.player(B).unwrap().hand.len(),
            if vote == 1 { 1 } else { 3 }
        );
        assert_eq!(g.battlefield.len(), if vote == 0 { 1 } else { 3 });
    }
}

#[test]
fn sacrifices_by_two_controllers_give_the_cards_single_owner_one_choice() {
    let mut g = game();
    for name in ["grizzly-bears", "hill-giant"] {
        put(&mut g, name, A, Zone::Battlefield);
    }
    for name in ["walking-corpse", "grizzly-bears"] {
        let id = put(&mut g, name, A, Zone::Battlefield);
        g.set_current_controller(id, B).unwrap();
    }
    let mut decider = Arrange::default();
    resolve_spell(&mut g, "barter-in-blood", vec![], &mut decider);
    assert_eq!(decider.prompts.len(), 1);
    assert_eq!(decider.prompts[0].0, A);
    assert_eq!(decider.prompts[0].1.len(), 4);
    assert_eq!(&graveyard(&g, A)[..4], decider.prompts[0].1);
    assert!(graveyard(&g, B).is_empty());
}

#[test]
fn a_discard_followed_by_a_mill_stays_in_instruction_order() {
    let mut g = game();
    put(&mut g, "walking-corpse", B, Zone::Library);
    put(&mut g, "hill-giant", B, Zone::Hand);
    let mut decider = Arrange::default();
    resolve_spell(
        &mut g,
        "horrifying-revelation",
        vec![Target::Player(B)],
        &mut decider,
    );
    assert!(decider.prompts.is_empty());
    assert_eq!(graveyard(&g, B), ["Hill Giant", "Walking Corpse"]);
}

#[test]
fn cleanup_discards_wait_for_the_owners_order_and_resume_without_duplicate_moves() {
    let mut g = game();
    for _ in 0..7 {
        put(&mut g, "plains", A, Zone::Hand);
    }
    let bear = put(&mut g, "grizzly-bears", A, Zone::Hand);
    let giant = put(&mut g, "hill-giant", A, Zone::Hand);
    let mut suspended = Arrange {
        pause_on: Some(A),
        ..Default::default()
    };
    ironsmith::turn::apply_cleanup_discard(&mut g, &[bear, giant], &mut suspended).unwrap();
    assert!(suspended.pending);
    assert_eq!(g.player(A).unwrap().hand.len(), 9);
    assert!(g.player(A).unwrap().graveyard.is_empty());
    let mut decider = Arrange::default();
    ironsmith::turn::apply_cleanup_discard(&mut g, &[bear, giant], &mut decider).unwrap();
    assert_eq!(suspended.prompts, decider.prompts);
    assert_eq!(g.player(A).unwrap().hand.len(), 7);
    assert_eq!(graveyard(&g, A), ["Hill Giant", "Grizzly Bears"]);
}

#[test]
fn destroying_a_melded_creature_with_another_creature_allows_interleaving_the_cards() {
    let mut g = game();
    let melded = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
    let mut components = Vec::new();
    for route in ["walking-corpse", "hill-giant"] {
        let definition = card(route);
        g.register_linked_face_definition(&definition);
        components.push(ironsmith::game_state::MeldComponentState {
            stable_id: ironsmith::ids::StableId::from_object_id(g.new_object_id()),
            owner: B,
            name: definition.card.name.to_string(),
        });
    }
    g.set_melded_permanent(melded, components);
    put(&mut g, "grizzly-bears", B, Zone::Battlefield);
    let mut decider = Arrange {
        middle_card: Some("Grizzly Bears"),
        ..Default::default()
    };
    resolve_spell(&mut g, "wrath-of-god", vec![], &mut decider);
    assert_eq!(decider.prompts.len(), 1);
    assert_eq!(decider.prompts[0].0, B);
    assert_eq!(decider.prompts[0].1.len(), 3);
    assert_eq!(graveyard(&g, B), decider.prompts[0].1);
    assert_eq!(graveyard(&g, B)[1], "Grizzly Bears");
}

#[test]
fn a_token_dying_with_one_card_does_not_ask_the_owner_to_order_cards() {
    let mut g = game();
    put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    g.create_token_object_from_definition(&card("walking-corpse"), A, Zone::Battlefield);
    let mut decider = Arrange::default();
    resolve_spell(&mut g, "wrath-of-god", vec![], &mut decider);
    assert!(decider.prompts.is_empty());
    assert!(graveyard(&g, A).iter().any(|name| name == "Grizzly Bears"));
}

#[test]
fn mass_reanimation_excludes_the_creatures_just_sacrificed_by_any_player() {
    let mut g = game();
    let mut old_creatures = Vec::new();
    for who in [A, B] {
        let old = put(&mut g, "grizzly-bears", who, Zone::Graveyard);
        old_creatures.push(g.object(old).unwrap().stable_id);
        put(&mut g, "grizzly-bears", who, Zone::Battlefield);
        put(&mut g, "hill-giant", who, Zone::Battlefield);
    }
    let stolen = put(&mut g, "walking-corpse", A, Zone::Battlefield);
    g.set_current_controller(stolen, B).unwrap();
    let source = put(&mut g, "bringer-of-the-last-gift", A, Zone::Hand);
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::Black, 8);
    assert_eq!(
        super::expanded_interactions::spend(&mut g, source, ironsmith::CastingMethod::Normal),
        8
    );
    let mut decider = Arrange::default();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut decider).unwrap();
    let mut queue = ironsmith::TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(&mut g, &mut queue).unwrap();
    assert_eq!(g.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut decider).unwrap();
    assert_eq!(g.battlefield.len(), 3);
    for stable in old_creatures {
        let id = g.find_object_by_stable_id(stable).unwrap();
        assert!(g.battlefield.contains(&id));
    }
    assert_eq!(decider.prompts.len(), 2);
    for (owner, order) in decider.prompts {
        assert_eq!(order.len(), if owner == A { 3 } else { 2 });
        assert_eq!(graveyard(&g, owner), order);
    }
}

#[test]
fn exile_replacements_leave_only_the_cards_reaching_the_graveyard_in_the_choice() {
    for creatures in [1, 2] {
        let mut g = game();
        put(&mut g, "dryad-militant", A, Zone::Battlefield);
        put(&mut g, "plains", B, Zone::Graveyard);
        put(&mut g, "opt", B, Zone::Library);
        for route in ["grizzly-bears", "hill-giant"].into_iter().take(creatures) {
            put(&mut g, route, B, Zone::Library);
        }
        let mut decider = Arrange::default();
        resolve_spell(&mut g, "tome-scour", vec![Target::Player(B)], &mut decider);
        assert_eq!(decider.prompts.len(), usize::from(creatures == 2));
        let cards = graveyard(&g, B);
        assert_eq!(cards.len(), creatures + 1);
        assert_eq!(cards[0], "Plains");
        if creatures == 2 {
            assert_eq!(&cards[1..], decider.prompts[0].1);
        }
        assert!(
            g.exile
                .iter()
                .any(|id| g.object(*id).unwrap().name.as_str() == "Opt")
        );
    }
}
