//! Player-relative play permission durations and shared play budgets.
//! Includes rendering, artifact round-trip, and multiplayer expiration regressions.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::{GrantPlayTaggedDuration, GrantPlayTaggedEffect};
use ironsmith::target::PlayerFilter;

const SUSPEND_AGGRESSION: &str = "Mana cost: {1}{R}{W}\nType: Instant\nExile target nonland permanent and the top card of your library. For each of those cards, its owner may play it until the end of their next turn.";
const EXPEDITED_INHERITANCE: &str = "Mana cost: {R}{R}\nType: Enchantment\nWhenever a creature is dealt damage, its controller may exile that many cards from the top of their library. They may play those cards until the end of their next turn.";
const MARCH_OF_RECKLESS_JOY: &str = "Mana cost: {X}{R}\nType: Instant\nAs an additional cost to cast this spell, you may exile any number of red cards from your hand. This spell costs {2} less to cast for each card exiled this way.\nExile the top X cards of your library. You may play up to two of those cards until the end of your next turn.";

fn grants(name: &str, text: &str) -> Vec<GrantPlayTaggedEffect> {
    support::definitions(name, text)
        .iter()
        .flat_map(support::find_all::<GrantPlayTaggedEffect>)
        .collect()
}

#[test]
fn suspend_aggression_owner_keeps_permission_through_their_next_turn() {
    let grants = grants("Suspend Aggression", SUSPEND_AGGRESSION);
    assert_eq!(grants.len(), 2, "one grant per route");
    for grant in grants {
        assert_eq!(
            grant.duration,
            GrantPlayTaggedDuration::UntilPlayerNextTurnEnd
        );
        assert!(grant.allow_land, "play, not just cast");
        assert_ne!(
            grant.player,
            PlayerFilter::You,
            "each card's owner holds the permission"
        );
        assert_eq!(grant.max_plays, None);
    }
}

#[test]
fn expedited_inheritance_damaged_controller_holds_their_next_turn_permission() {
    let grants = grants("Expedited Inheritance", EXPEDITED_INHERITANCE);
    assert_eq!(grants.len(), 2);
    for grant in grants {
        assert_eq!(
            grant.duration,
            GrantPlayTaggedDuration::UntilPlayerNextTurnEnd
        );
        assert_ne!(
            grant.player,
            PlayerFilter::You,
            "the exiling player, not the enchantment's controller"
        );
    }
}

#[test]
fn march_of_reckless_joy_shares_a_two_play_budget() {
    let grants = grants("March of Reckless Joy", MARCH_OF_RECKLESS_JOY);
    assert_eq!(grants.len(), 2);
    for grant in grants {
        assert_eq!(
            grant.duration,
            GrantPlayTaggedDuration::UntilYourNextTurnEnd
        );
        assert_eq!(grant.player, PlayerFilter::You);
        assert_eq!(grant.max_plays, Some(2));
    }
}

fn check_next_turn_reference(possessive: &str, owner_relative: bool) {
    use ironsmith::card::CardBuilder;
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{CardId, GameState, ObjectId, PlayerId, Zone};

    let text = format!(
        "Type: Instant\nExile target spell. Its owner may play it until the end of {possessive} next turn.\nExile the top card of your library. You may play it until the end of your next turn."
    );
    for definition in support::definitions("Duration reference probe", &text) {
        let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered.contains(&format!(
                "Its owner may play that card until the end of {possessive} next turn"
            )),
            "{rendered}"
        );
        assert!(
            rendered.contains("You may play it until the end of your next turn"),
            "{rendered}"
        );
        let grants = support::find_all::<GrantPlayTaggedEffect>(&definition);
        assert_eq!(grants.len(), 2);
        let alice = PlayerId(0);
        let bob = PlayerId(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.turn.active_player = alice;
        game.turn.turn_number = 10;
        let mut cards = Vec::new();
        let mut tags = std::collections::HashMap::new();
        for (index, (grant, owner)) in grants.iter().zip([bob, alice]).enumerate() {
            let card =
                CardBuilder::new(CardId::from_raw(index as u32 + 1), "Exiled witness").build();
            let id = game.create_object_from_card(&card, owner, Zone::Exile);
            tags.insert(
                grant.tag.clone(),
                vec![ObjectSnapshot::from_object(game.object(id).unwrap(), &game)],
            );
            cards.push(id);
        }
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx =
            EffectContext::new(ObjectId::from_raw(100), alice, &mut dm).with_tagged_objects(tags);
        for grant in &grants {
            grant.execute(&mut game, &mut ctx).unwrap();
        }
        let expected_player = if owner_relative { bob } else { alice };
        let expected_end = if owner_relative { 11 } else { 13 };
        match game.effect_store.grant_registry.grants[0].source {
            GrantSource::EffectUntilPlayerNextTurnEnd {
                duration_player,
                expires_end_of_turn,
                ..
            } => {
                assert_eq!(duration_player, expected_player);
                assert_eq!(expires_end_of_turn, expected_end);
            }
            ref other => panic!("unexpected grant source: {other:?}"),
        }
        let can_play = |game: &GameState, card, player| {
            game.effect_store.grant_registry.card_can_play_from_zone(
                game,
                card,
                Zone::Exile,
                player,
            )
        };
        assert!(can_play(&game, cards[0], bob));
        assert!(!can_play(&game, cards[0], alice));
        assert!(can_play(&game, cards[1], alice));
        assert!(!can_play(&game, cards[1], bob));
        game.turn.turn_number = 11;
        assert!(can_play(&game, cards[0], bob));
        game.turn.turn_number = 12;
        assert_eq!(can_play(&game, cards[0], bob), !owner_relative);
        assert!(can_play(&game, cards[1], alice));
        game.turn.turn_number = 13;
        assert_eq!(can_play(&game, cards[0], bob), !owner_relative);
        assert!(can_play(&game, cards[1], alice));
        game.turn.turn_number = 14;
        assert!(!can_play(&game, cards[0], bob));
        assert!(!can_play(&game, cards[1], alice));
    }
}

#[test]
fn their_next_turn_tracks_the_exiled_cards_owner() {
    check_next_turn_reference("their", true);
}

#[test]
fn your_next_turn_tracks_the_effect_controller_for_another_owner() {
    check_next_turn_reference("your", false);
}

fn check_other_duration_reference(boundary: &str, possessive: &str) {
    use ironsmith::card::CardBuilder;
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::game_state::Step;
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{CardId, GameState, ObjectId, Phase, PlayerId, Zone};
    let phrase = match boundary {
        "end_step" => format!("until {possessive} next end step"),
        "turn_start" => format!("until {possessive} next turn"),
        "during_turn" => format!("during {possessive} next turn"),
        "combat" => format!("until end of combat on {possessive} next turn"),
        _ => unreachable!(),
    };
    for leading in [false, true] {
        let permission = if leading {
            let capitalized = format!("{}{}", phrase[..1].to_uppercase(), &phrase[1..]);
            format!("{capitalized}, its owner may play it.")
        } else {
            format!("Its owner may play it {phrase}.")
        };
        let text = format!("Type: Instant\nExile target spell. {permission}");
        for definition in support::definitions("Other duration reference", &text) {
            let rendered = ironsmith_text::compiled_text_lines(&definition)
                .join(" ")
                .to_lowercase();
            assert!(rendered.contains(&phrase), "{rendered}");
            let grants = support::find_all::<GrantPlayTaggedEffect>(&definition);
            assert_eq!(grants.len(), 1);
            let alice = PlayerId(0);
            let bob = PlayerId(1);
            let holder_relative = possessive == "their";
            let duration_player = if holder_relative { bob } else { alice };
            let boundary_turn = if holder_relative {
                11
            } else if boundary == "end_step" {
                10
            } else {
                13
            };
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            game.turn.active_player = alice;
            game.turn.turn_number = 10;
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let card = CardBuilder::new(CardId::from_raw(1), "Other owner's card").build();
            let id = game.create_object_from_card(&card, bob, Zone::Exile);
            let tags = [(
                grants[0].tag.clone(),
                vec![ObjectSnapshot::from_object(game.object(id).unwrap(), &game)],
            )]
            .into();
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = EffectContext::new(ObjectId::from_raw(100), alice, &mut dm)
                .with_tagged_objects(tags);
            let mut grant = grants[0].clone();
            grant.mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::AnyColor;
            grant.execute(&mut game, &mut ctx).unwrap();
            let expected_mana_expiration = if boundary == "turn_start" {
                boundary_turn - 1
            } else {
                boundary_turn
            };
            assert!(matches!(
                game.effect_store.mana_spend_effects.permissions[0].source,
                ironsmith::game_state::ManaSpendPermissionSource::Effect { expires_end_of_turn, .. }
                    if expires_end_of_turn == expected_mana_expiration
            ));
            let source = &game.effect_store.grant_registry.grants[0].source;
            match source {
                GrantSource::EffectUntilPlayerNextTurnStart {
                    duration_player: actual,
                    ..
                }
                | GrantSource::EffectUntilPlayerNextTurnEnd {
                    duration_player: actual,
                    ..
                }
                | GrantSource::EffectUntilPlayerNextEndStep {
                    duration_player: actual,
                    ..
                } => assert_eq!(*actual, duration_player),
                other => panic!("unexpected duration: {other:?}"),
            }
            let can_play = |game: &GameState| {
                game.effect_store
                    .grant_registry
                    .card_can_play_from_zone(game, id, Zone::Exile, bob)
            };
            assert_eq!(can_play(&game), boundary != "during_turn");
            if boundary_turn > 10 {
                game.turn.turn_number = boundary_turn;
                game.turn.active_player = duration_player;
                game.effect_store
                    .grant_registry
                    .expire_at_turn_start(boundary_turn, &[duration_player]);
            }
            assert_eq!(can_play(&game), boundary != "turn_start");
            match boundary {
                "end_step" => {
                    game.turn.phase = Phase::Ending;
                    game.turn.step = Some(Step::End);
                }
                "combat" => {
                    game.turn.phase = Phase::NextMain;
                }
                _ => {
                    game.turn.turn_number += 1;
                    game.turn.active_player = PlayerId(2);
                }
            }
            assert!(!can_play(&game), "permission outlived {phrase}");
            assert!(!game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                id,
                Zone::Exile,
                alice
            ));
        }
    }
}

#[test]
fn their_next_end_step_keeps_its_player_reference() {
    check_other_duration_reference("end_step", "their");
}
#[test]
fn your_next_end_step_keeps_its_player_reference() {
    check_other_duration_reference("end_step", "your");
}
#[test]
fn their_next_turn_start_keeps_its_player_reference() {
    check_other_duration_reference("turn_start", "their");
}
#[test]
fn your_next_turn_start_keeps_its_player_reference() {
    check_other_duration_reference("turn_start", "your");
}
#[test]
fn during_their_next_turn_keeps_its_player_reference() {
    check_other_duration_reference("during_turn", "their");
}
#[test]
fn during_your_next_turn_keeps_its_player_reference() {
    check_other_duration_reference("during_turn", "your");
}
#[test]
fn combat_on_their_next_turn_keeps_its_player_reference() {
    check_other_duration_reference("combat", "their");
}
#[test]
fn combat_on_your_next_turn_keeps_its_player_reference() {
    check_other_duration_reference("combat", "your");
}

fn check_non_tagged_grant_duration(exile_first: bool) {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{
        EffectContext, EffectExecutor, ExileThenGrantPlayEffect, GrantBySpecEffect, ResolvedTarget,
    };
    use ironsmith::grant::{GrantDuration, GrantSpec, Grantable};
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::target::{ChooseSpec, ObjectFilter};
    use ironsmith::{CardId, GameState, ObjectId, PlayerId, Zone};
    for duration in [
        GrantDuration::UntilYourNextTurn,
        GrantDuration::UntilYourNextTurnEnd,
    ] {
        let alice = PlayerId(0);
        let bob = PlayerId(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.turn.active_player = alice;
        game.turn.turn_number = 10;
        let card = CardBuilder::new(CardId::from_raw(1), "Bob's card").build();
        let zone = if exile_first { Zone::Hand } else { Zone::Exile };
        let id = game.create_object_from_card(&card, bob, zone);
        let mut ctx = EffectContext::new_default(ObjectId::from_raw(100), alice);
        if exile_first {
            ctx.targets = vec![ResolvedTarget::Object(id)];
            ExileThenGrantPlayEffect::new(
                ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
                PlayerFilter::Specific(bob),
                duration,
            )
            .execute(&mut game, &mut ctx)
            .unwrap();
        } else {
            GrantBySpecEffect::new(
                GrantSpec::new(Grantable::PlayFrom, ObjectFilter::default(), Zone::Exile),
                PlayerFilter::Specific(bob),
                duration,
            )
            .execute(&mut game, &mut ctx)
            .unwrap();
        }
        let exiled = *game.exile.last().unwrap();
        let source = game.effect_store.grant_registry.grants[0].source.clone();
        match source {
            GrantSource::EffectUntilPlayerNextTurnStart {
                duration_player, ..
            }
            | GrantSource::EffectUntilPlayerNextTurnEnd {
                duration_player, ..
            } => assert_eq!(duration_player, alice),
            ref other => panic!("unexpected source: {other:?}"),
        }
        let can_play = |game: &GameState, player| {
            game.effect_store.grant_registry.card_can_play_from_zone(
                game,
                exiled,
                Zone::Exile,
                player,
            )
        };
        assert!(can_play(&game, bob));
        assert!(!can_play(&game, alice));
        game.turn.turn_number = 11;
        game.turn.active_player = bob;
        assert!(can_play(&game, bob));
        game.turn.turn_number = 13;
        game.turn.active_player = alice;
        assert_eq!(
            can_play(&game, bob),
            duration == GrantDuration::UntilYourNextTurnEnd
        );
        game.turn.turn_number = 14;
        assert!(!can_play(&game, bob));
    }
}

#[test]
fn filter_grant_your_duration_uses_effect_controller() {
    check_non_tagged_grant_duration(false);
}
#[test]
fn exile_then_grant_your_duration_uses_effect_controller() {
    check_non_tagged_grant_duration(true);
}

#[test]
fn leading_during_and_combat_permissions_keep_their_boundaries() {
    for (name, text, expected) in [
        (
            "Galvanic Relay",
            "Type: Sorcery\nExile the top card of your library. During your next turn, you may play that card.\nStorm",
            GrantPlayTaggedDuration::DuringYourNextTurn,
        ),
        (
            "Combat permission",
            "Type: Instant\nExile the top card of your library. Until end of combat on your next turn, you may play that card.",
            GrantPlayTaggedDuration::UntilEndOfCombatOnYourNextTurn,
        ),
    ] {
        for definition in support::definitions(name, text) {
            let grants = support::find_all::<GrantPlayTaggedEffect>(&definition);
            assert_eq!(grants.len(), 1);
            assert_eq!(grants[0].duration, expected);
        }
    }
}

#[test]
fn source_control_duration_uses_effect_controller_not_permission_holder() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{CardId, GameState, PlayerId, Zone};
    let text = "Type: Creature\nPower/Toughness: 2/2\nWhen this creature enters, exile target spell. Its owner may play it for as long as you control this creature.";
    for definition in support::definitions("Control duration probe", text) {
        let grants = support::find_all::<GrantPlayTaggedEffect>(&definition);
        assert_eq!(grants.len(), 1);
        assert_eq!(
            grants[0].duration,
            GrantPlayTaggedDuration::ForAsLongAsYouControlSource
        );
        let alice = PlayerId(0);
        let bob = PlayerId(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = CardBuilder::new(CardId::from_raw(99), "Bob's card").build();
        let id = game.create_object_from_card(&card, bob, Zone::Exile);
        let tags = [(
            grants[0].tag.clone(),
            vec![ObjectSnapshot::from_object(game.object(id).unwrap(), &game)],
        )]
        .into();
        grants[0]
            .execute(
                &mut game,
                &mut EffectContext::new_default(source, alice).with_tagged_objects(tags),
            )
            .unwrap();
        let can_play = |game: &GameState| {
            game.effect_store
                .grant_registry
                .card_can_play_from_zone(game, id, Zone::Exile, bob)
        };
        assert!(
            can_play(&game),
            "Alice controls the source even though Bob holds the permission: effect={:?}; grants={:?}; controller={:?}",
            grants[0],
            game.effect_store.grant_registry.grants,
            game.controller_of(game.object(source).unwrap())
        );
        game.set_current_controller(source, bob).unwrap();
        assert!(
            !can_play(&game),
            "Bob taking the source ends Alice's control duration"
        );
    }
}

#[test]
fn explicit_that_players_end_step_still_compiles() {
    let text = "Type: Instant\nTarget player exiles the top card of their library. Until that player's next end step, they may play that card without paying its mana cost.";
    for definition in support::definitions("End step reference", text) {
        let grants = support::find_all::<GrantPlayTaggedEffect>(&definition);
        assert!(!grants.is_empty());
        assert!(
            grants
                .iter()
                .all(|grant| grant.duration == GrantPlayTaggedDuration::UntilPlayerNextEndStep)
        );
    }
}
