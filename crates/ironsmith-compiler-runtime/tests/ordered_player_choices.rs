//! cf8 p05: "Starting with you / the next opponent in turn order, each
//! player/opponent chooses ..." — the participants choose one at a time in
//! the stated order, each seeing earlier choices (CR 101.4), and every
//! choice joins one chosen set that the following instruction consumes.
//! Choice pools: "from among them" (the revealed group), "from among
//! permanents your opponents control", "from among the permanents controlled
//! by the player to their left" (PlayerToLeftOf(IteratedPlayer)), and the
//! running exclusions "a different" / "that hasn't been chosen".
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

use ironsmith::effects::{ChooseObjectsEffect, ForPlayersEffect};
use ironsmith::target::PlayerFilter;

const CLUSTER: &str = "ordered_player_choices";

fn ordered_loops(definition: &ironsmith::cards::CardDefinition) -> Vec<ForPlayersEffect> {
    support::effects_of::<ForPlayersEffect>(definition)
        .into_iter()
        .filter(|loop_| loop_.starting_with_controller)
        .collect()
}

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 6);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn horus_heresy_destroys_every_creature_chosen_in_order() {
    for definition in support::definitions(&support::row(CLUSTER, "The Horus Heresy")) {
        let loops = ordered_loops(&definition);
        assert_eq!(loops.len(), 1, "{loops:#?}");
        assert_eq!(loops[0].filter, PlayerFilter::Any);
        let debug = support::debug(&definition);
        assert!(debug.contains("chosen_objects") || debug.contains("ChosenObjects"), "{debug}");
    }
}

#[test]
fn yangchen_exiles_the_union_of_choices_among_opponents_permanents() {
    for definition in
        support::definitions(&support::row(CLUSTER, "The Legend of Yangchen // Avatar Yangchen"))
    {
        assert_eq!(ordered_loops(&definition).len(), 1);
        let choices = support::effects_of::<ChooseObjectsEffect>(&definition);
        assert!(
            choices.iter().any(|choice| {
                choice.count.max == Some(1)
                    && choice.count.min == 0
                    && choice.filter.controller == Some(PlayerFilter::Opponent)
                    && choice.filter.mana_value.is_some()
            }),
            "{choices:#?}"
        );
    }
}

#[test]
fn grenzos_rebuttal_chooses_three_type_slots_from_the_player_to_their_left() {
    for definition in support::definitions(&support::row(CLUSTER, "Grenzo's Rebuttal")) {
        assert_eq!(ordered_loops(&definition).len(), 1);
        let choices = support::effects_of::<ChooseObjectsEffect>(&definition);
        let left = PlayerFilter::PlayerToLeftOf(Box::new(PlayerFilter::IteratedPlayer));
        assert_eq!(
            choices
                .iter()
                .filter(|choice| choice.filter.controller.as_ref() == Some(&left))
                .count(),
            3,
            "an artifact, a creature and a land: {choices:#?}"
        );
    }
}

#[test]
fn rejoin_the_fight_opponents_choose_distinct_creature_cards_in_turn_order() {
    for definition in support::definitions(&support::row(CLUSTER, "Rejoin the Fight")) {
        let loops = ordered_loops(&definition);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].filter, PlayerFilter::Opponent);
        let debug = support::debug(&definition);
        assert!(debug.contains("IsNotTaggedObject"), "the running exclusion: {debug}");
    }
}

#[test]
fn manifold_insights_opponents_choose_different_revealed_cards() {
    for definition in support::definitions(&support::row(CLUSTER, "Manifold Insights")) {
        let loops = ordered_loops(&definition);
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].filter, PlayerFilter::Opponent);
        let debug = support::debug(&definition);
        assert!(debug.contains("IsNotTaggedObject"), "a different card: {debug}");
    }
}

#[test]
fn manifold_insights_opponents_choose_distinct_cards_in_turn_order_at_runtime() {
    use ironsmith::card::CardBuilder;
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::effects::{execute_effect, EffectContext};
    use ironsmith::ids::{CardId, ObjectId};
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};

    struct OrderedPicks { players: Vec<PlayerId>, cards: Vec<ObjectId> }
    impl DecisionMaker for OrderedPicks {
        fn decide_objects(&mut self, _: &ironsmith::GameState, choice: &SelectObjectsContext) -> Vec<ObjectId> {
            let selected = choice.candidates.iter().find(|candidate| candidate.legal)
                .map(|candidate| candidate.id).into_iter().collect::<Vec<_>>();
            self.players.push(choice.player);
            self.cards.extend(selected.iter().copied());
            selected
        }
    }

    let definition = support::definitions(&support::row(CLUSTER, "Manifold Insights"))
        .into_iter().next().expect("compiled spell");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    for (index, card_type) in [CardType::Creature, CardType::Artifact, CardType::Instant, CardType::Land]
        .into_iter().enumerate() {
        let card = CardBuilder::new(CardId::new(), format!("Library card {index}"))
            .card_types(vec![card_type]).build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = OrderedPicks { players: Vec::new(), cards: Vec::new() };
    {
        let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).expect("ordered opponent resolution");
        }
    }
    assert_eq!(decisions.players, vec![bob, carol], "the next opponent chooses first");
    assert_eq!(decisions.cards.len(), 2);
    assert_ne!(decisions.cards[0], decisions.cards[1], "the running chosen tag excludes the first card");
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().library.len(), 2, "unpicked cards remain in the library");
}

#[test]
fn thieves_auction_repeats_rounds_until_every_exiled_card_was_chosen_once() {
    for definition in support::definitions(&support::row(CLUSTER, "Thieves' Auction")) {
        assert_eq!(ordered_loops(&definition).len(), 1);
        let debug = support::debug(&definition);
        assert!(debug.contains("RepeatProcessEffect"), "{debug}");
        // A pick is never offered again, so an unplayable card can't loop.
        assert!(debug.contains("auction_chosen"), "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        let choices = support::effects_of::<ChooseObjectsEffect>(&definition);
        assert!(choices.iter().all(|choice| choice.count.max == Some(1)), "{choices:#?}");
    }
}
