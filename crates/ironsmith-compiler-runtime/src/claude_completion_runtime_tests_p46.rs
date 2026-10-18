//! Claude package 46 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

/// Convert to Slime: the Delirium line is an independently lowered statement,
/// so the destroy line must export its result ID for "destroyed this way".
#[test]
fn convert_to_slime_followup_line_reads_destroyed_this_way_from_previous_line() {
    let definition = compile_to_runtime_definition(
        "Convert to Slime",
        "Mana cost: {3}{B}{G}\nType: Sorcery\nDestroy up to one target artifact, up to one target creature, and up to one target enchantment.\nDelirium — Then if there are four or more card types among cards in your graveyard, create an X/X green Ooze creature token, where X is the total mana value of permanents destroyed this way.",
        false,
    )
    .expect("Convert to Slime compiles");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("WithIdEffect"), "destroy line exports an ID: {debug}");
    assert!(debug.contains("PriorEffectMetric"), "metric binds to the exported ID");
    assert!(debug.contains("Destroyed"), "metric stays action-typed");
    assert!(!debug.contains("PendingPriorEffectMetric"), "no unbound metric remains");
}

/// Clay Golem: "Roll a d8" is an activation cost whose retained result is X.
#[test]
fn clay_golem_roll_cost_result_is_the_monstrosity_amount() {
    let definition = compile_to_runtime_definition(
        "Clay Golem",
        "Mana cost: {4}\nType: Artifact Creature — Golem\nPower/Toughness: 4/4\n{6}, Roll a d8: Monstrosity X, where X is the result. (If this creature isn't monstrous, put X +1/+1 counters on it and it becomes monstrous.)\nBerserk — When this creature becomes monstrous, destroy target permanent.",
        false,
    )
    .expect("Clay Golem compiles");
    let id = ironsmith::effect::EffectId::ACTIVATION_RESULT_COST.0;
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RollDieEffect"), "roll is part of the cost: {debug}");
    assert!(debug.contains(&format!("WithIdEffect {{ id: EffectId({id})")), "cost roll retains its result");
    assert!(debug.contains(&format!("EffectValue(EffectId({id}))")), "Monstrosity X reads the paid roll");
    assert!(debug.contains("MonstrosityEffect"));
}

/// Conundrum Sphinx: every player names a card first; each revealed card is
/// compared with the name its own owner chose.
#[test]
fn conundrum_sphinx_compares_each_reveal_with_its_owners_chosen_name() {
    let definition = compile_to_runtime_definition(
        "Conundrum Sphinx",
        "Mana cost: {2}{U}{U}\nType: Creature — Sphinx\nPower/Toughness: 4/4\nFlying\nWhenever this creature attacks, each player chooses a card name. Then each player reveals the top card of their library. If the card a player revealed has the name they chose, that player puts it into their hand. If it doesn't, that player puts it on the bottom of their library.",
        false,
    )
    .expect("Conundrum Sphinx compiles");
    let debug = format!("{:?}", definition.abilities);
    let choose = debug.find("ChooseCardNameEffect").expect("names are chosen");
    let reveal = debug.find("RevealTopEffect").expect("top cards are revealed");
    assert!(choose < reveal, "every choice precedes every reveal");
    assert!(debug.contains("SameNameAsTaggedChosenByOwner"), "{debug}");
    assert!(debug.contains("Hand") && debug.contains("Library"));
}

/// Codie: the next-cast delayed trigger owns the whole three-sentence
/// exile-until procedure, comparing with the triggering spell's mana value.
#[test]
fn codie_next_cast_trigger_owns_its_multi_sentence_consult_procedure() {
    let definition = compile_to_runtime_definition(
        "Codie, Vociferous Codex",
        "Mana cost: {3}\nType: Legendary Artifact Creature — Book Construct\nPower/Toughness: 1/4\nYou can't cast permanent spells.\n{4}, {T}: Add {W}{U}{B}{R}{G}. When you next cast a spell this turn, exile cards from the top of your library until you exile an instant or sorcery card with lesser mana value. Until end of turn, you may cast that card without paying its mana cost. Put each other card exiled this way on the bottom of your library in a random order.",
        false,
    )
    .expect("Codie compiles");
    let debug = format!("{:?}", definition.abilities);
    let delayed = debug.find("ScheduleDelayedTriggerEffect").expect("delayed trigger");
    let body = &debug[delayed..];
    assert!(body.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(body.contains("ManaValueLtTagged"), "lesser than the triggering spell");
}

/// Cruel Entertainment: each of the two target players is controlled by the
/// other during their own next turn.
#[test]
fn cruel_entertainment_targets_two_players_who_control_each_other() {
    let definition = compile_to_runtime_definition(
        "Cruel Entertainment",
        "Mana cost: {6}{B}\nType: Sorcery\nChoose target player and another target player. The first player controls the second player during the second player's next turn, and the second player controls the first player during the first player's next turn.",
        false,
    )
    .expect("Cruel Entertainment compiles");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("ControlPlayerEffect") && debug.contains("controller"), "{debug}");
    assert!(debug.contains("IteratedPlayer"));

    // The explicit controller, not the effect's controller, gains control.
    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    let carol = ironsmith::PlayerId::from_index(2);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let source = game.new_object_id();
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    let control = ironsmith::effects::ControlPlayerEffect::during_next_turn(
        ironsmith::target::PlayerFilter::Specific(bob),
    )
    .controlled_by(ironsmith::target::PlayerFilter::Specific(carol));
    ironsmith::effects::execute_effect(&mut game, &Effect::new(control), &mut ctx).unwrap();
    drop(ctx);
    game.next_turn();
    assert_eq!(game.turn.active_player, bob);
    assert_eq!(game.controlling_player_for(bob), carol);
}

/// Comet, Stellar Pup: em-dash die rows key the branches; bracketed loyalty
/// symbols inside a branch add/remove loyalty; the 6 row grants two more
/// loyalty activations this turn.
#[test]
fn comet_die_rows_carry_inline_loyalty_changes_and_extra_activations() {
    let definition = compile_to_runtime_definition(
        "Comet, Stellar Pup",
        "Mana cost: {2}{R}{W}\nType: Legendary Planeswalker — Comet\nLoyalty: 5\n0: Roll a six-sided die.\n1 or 2 — [+2], then create two 1/1 green Squirrel creature tokens. They gain haste until end of turn.\n3 — [−1], then return a card with mana value 2 or less from your graveyard to your hand.\n4 or 5 — Comet deals damage equal to the number of loyalty counters on him to a creature or player, then [−2].\n6 — [+1], and you may activate Comet's loyalty ability two more times this turn.",
        false,
    )
    .expect("Comet compiles");
    let debug = format!("{:?}", definition.abilities);
    assert_eq!(definition.abilities.len(), 1, "one loyalty ability owns every row: {debug}");
    assert!(debug.contains("RollDieEffect"));
    assert!(debug.contains("BetweenInclusive(1, 2)") && debug.contains("BetweenInclusive(4, 5)"), "{debug}");
    assert!(debug.contains("Loyalty"), "inline loyalty symbols change loyalty counters");
    assert_eq!(debug.matches("GrantLoyaltyActivationAllowanceEffect").count(), 2, "two more times");
}

/// Close Encounter: the additional cost chooses (never exiles) a creature
/// you control or a warped creature card you own in exile.
#[test]
fn close_encounter_additional_cost_chooses_a_creature_or_warped_card() {
    let definition = compile_to_runtime_definition(
        "Close Encounter",
        "Mana cost: {1}{G}\nType: Instant\nAs an additional cost to cast this spell, choose a creature you control or a warped creature card you own in exile.\nClose Encounter deals damage equal to the power of the chosen creature or card to target creature.",
        false,
    )
    .expect("Close Encounter compiles");
    let debug = format!("{definition:?}");
    assert!(debug.contains("warped: true"), "{debug}");
    let cost = format!("{:?}", definition.additional_cost);
    assert!(cost.contains("tag: TagKey(\"__it__\")")
        && debug.contains("PowerOf(Tagged(TagKey(\"__it__\")))"),
        "both object domains export the same choice read by the damage: {debug}");
    assert!(!debug.contains("ExileEffect"), "choosing is not exiling");

    let alice = ironsmith::PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Warp witness")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3))
        .build();
    let exiled = game.create_object_from_card(&card, alice, ironsmith::Zone::Exile);
    assert!(!game.is_warped(exiled));
    game.set_warped(exiled);
    assert!(game.is_warped(exiled));
}

/// Cover of Winter: X is one budget per attacking creature, shared by all of
/// that creature's simultaneous recipients among you and your creatures.
#[test]
fn cover_of_winter_prevention_budget_is_shared_per_source() {
    let definition = compile_to_runtime_definition(
        "Cover of Winter",
        "Mana cost: {2}{W}\nType: Snow Enchantment\nCumulative upkeep {S} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it. {S} can be paid with one mana from a snow source.)\nIf a creature would deal combat damage to you and/or one or more creatures you control, prevent X of that damage, where X is the number of age counters on this enchantment.\n{S}: Put an age counter on this enchantment.",
        false,
    )
    .expect("Cover of Winter compiles");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SharedPerSource"), "{debug}");
    assert!(debug.contains("combat_only: true"));
    assert!(debug.contains("target_player_filter: Some(You)"), "{debug}");
}

/// Circling Vultures: its owner may discard it from hand as a special action
/// whenever they have priority, without using the stack.
#[test]
fn circling_vultures_discards_from_hand_as_a_special_action() {
    let definition = compile_to_runtime_definition(
        "Circling Vultures",
        "Mana cost: {B}\nType: Creature — Bird\nPower/Toughness: 3/2\nFlying\nYou may discard this card any time you could cast an instant.\nAt the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.",
        false,
    )
    .expect("Circling Vultures compiles");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("MayDiscardSourceAnyTimeYouCouldCastInstant"), "{debug}");
    assert!(debug.contains("top_only: true"), "the top creature card of the graveyard");

    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Hand);
    let action = ironsmith::special_actions::SpecialAction::DiscardFromHand { card_id: card };
    game.turn.priority_player = Some(bob);
    assert!(ironsmith::special_actions::can_perform_check(&action, &game, alice).is_err());
    game.turn.priority_player = Some(alice);
    ironsmith::special_actions::can_perform_check(&action, &game, alice).expect("legal with priority");
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::special_actions::perform(action, &mut game, alice, &mut dm).expect("discard");
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    assert!(game.stack.is_empty(), "no stack object");
}

/// Cogwork Tracker: the passing player noted during the draft becomes an
/// attack requirement for this creature.
#[test]
fn cogwork_tracker_must_attack_the_player_noted_while_drafting_it() {
    let definition = compile_to_runtime_definition(
        "Cogwork Tracker",
        "Mana cost: {4}\nType: Artifact Creature — Dog Construct\nPower/Toughness: 4/2\nReveal this card as you draft it and note the player who passed it to you.\nThis creature attacks each combat if able.\nThis creature attacks a player you noted for cards named Cogwork Tracker each combat if able.",
        false,
    )
    .expect("Cogwork Tracker compiles");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("MustAttackDraftNotedPlayers"), "{debug}");

    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    let carol = ironsmith::PlayerId::from_index(2);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let tracker = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
    assert!(game.required_attack_players_this_turn(tracker).next().is_none(), "nothing noted");
    game.record_draft_noted_player(alice, "Cogwork Tracker", carol);
    let required = game.required_attack_players_this_turn(tracker).collect::<Vec<_>>();
    assert_eq!(required, vec![carol]);
    assert!(!required.contains(&bob));
}
