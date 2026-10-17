//! Claude package 9 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p9_statics(definition: &ironsmith::cards::CardDefinition) -> Vec<String> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(ability) => Some(format!("{:?}", ability.compiled_model())),
        _ => None,
    }).collect()
}

fn p9_triggered(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).collect()
}

#[test]
fn mister_negative_draws_your_life_lost_in_exchange() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Mister Negative",
        "Mana cost: {5}{W}{B}\nType: Legendary Creature — Human Villain\nPower/Toughness: 5/5\nVigilance, lifelink\nDarkforce Inversion — When Mister Negative enters, you may exchange life totals with target opponent. If you lost life this way, draw that many cards.", false)
        .expect("life-exchange result should compile");
    let trigger = p9_triggered(&definition).into_iter().next().expect("ETB trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("ExchangeLifeTotals"), "{debug}");
    assert!(debug.contains("LifeLost"), "{debug}");
    assert!(debug.contains("player: Some(You)"), "the participant must be explicit: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let filler = CardBuilder::new(CardId::new(), "Filler").card_types(vec![CardType::Land]).build();
    for _ in 0..20 { game.create_object_from_card(&filler, alice, Zone::Library); }
    game.player_mut(alice).unwrap().life = 25;
    game.player_mut(bob).unwrap().life = 10;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand_before = game.player(alice).unwrap().hand.len();
    let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Player(bob)]);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 10, "the exchange executes");
    assert_eq!(game.player(bob).unwrap().life, 25);
    assert_eq!(game.player(alice).unwrap().hand.len() - hand_before, 15, "you lost 15 life this way");
    // Gaining life in the exchange is not losing life this way.
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for _ in 0..20 { game.create_object_from_card(&filler, alice, Zone::Library); }
    game.player_mut(alice).unwrap().life = 5;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Player(bob)]);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 20, "the controller gains life in this exchange");
    assert_eq!(game.player(bob).unwrap().life, 5);
    assert_eq!(game.player(alice).unwrap().hand.len(), 0, "a life gain draws nothing");
}

#[test]
fn cast_using_web_slinging_conditions() {
    let scarlet = compile_to_runtime_definition("Scarlet Spider, Ben Reilly",
        "Mana cost: {1}{R}{G}\nType: Legendary Creature — Spider Human Hero\nPower/Toughness: 3/3\nWeb-slinging {R}{G} (You may cast this spell for {R}{G} if you also return a tapped creature you control to its owner's hand.)\nTrample\nSensational Save — If Scarlet Spider was cast using web-slinging, he enters with X +1/+1 counters on him, where X is the mana value of the returned creature.", false)
        .expect("web-slinging entry counters should compile");
    assert!(scarlet.alternative_casts.iter().any(|method| method.name().eq_ignore_ascii_case("Web-slinging")));
    let statics = p9_statics(&scarlet);
    let entry = statics.iter().find(|text| text.contains("EntersWithCounters")).unwrap_or_else(|| panic!("{statics:?}"));
    assert!(entry.contains("Web-slinging") || entry.contains("web-slinging"), "{entry}");
    assert!(entry.contains("return_cost_0"), "X reads the returned cost object: {entry}");

    let spiders = compile_to_runtime_definition("Spiders-Man, Heroic Horde",
        "Mana cost: {1}{G}\nType: Legendary Creature — Spider Hero\nPower/Toughness: 2/2\nWeb-slinging {4}{G}{G} (You may cast this spell for {4}{G}{G} if you also return a tapped creature you control to its owner's hand.)\nWhen Spiders-Man enters, if they were cast using web-slinging, you gain 3 life and create two 2/1 green Spider creature tokens with reach.", false)
        .expect("web-slinging intervening-if should compile");
    let trigger = p9_triggered(&spiders).into_iter().next().expect("ETB trigger");
    let condition = format!("{:?}", trigger.intervening_if);
    assert!(condition.contains("AlternativeCast") && condition.contains("Web-slinging"), "{condition}");

    // A named query that lowering could not link still reads the named
    // alternative-cast receipt.
    let stored = ironsmith_core::OptionalCostKind::AlternativeCast(
        ironsmith_core::AlternativeCostReference::paid_marker("Web-slinging", None));
    assert!(ironsmith_core::OptionalCostKind::from_label("Web-slinging").is_query_for(&stored));
    assert!(!ironsmith_core::OptionalCostKind::from_label("Sneak").is_query_for(&stored));
}

#[test]
fn decision_dependent_flash_permissions() {
    let silver = compile_to_runtime_definition("Silver Scrutiny",
        "Mana cost: {X}{U}{U}\nType: Sorcery\nYou may cast this spell as though it had flash if X is 3 or less.\nDraw X cards.", false)
        .expect("X-dependent flash should compile");
    let statics = p9_statics(&silver);
    assert!(statics.iter().any(|text| text.contains("FlashIfCastChoice(XAtMost(3))")), "{statics:?}");

    let quantum = compile_to_runtime_definition("Quantum Reduction",
        "Mana cost: {1}{U}\nType: Enchantment — Aura\nTeamwork 2 (As an additional cost to cast this spell, you may tap any number of creatures you control with total power 2 or more.)\nYou may cast this spell as though it had flash if it's cast using teamwork.\nEnchant creature\nEnchanted creature gets -5/-0 and loses all abilities.", false)
        .expect("teamwork-dependent flash should compile");
    let statics = p9_statics(&quantum);
    assert!(statics.iter().any(|text| text.contains("FlashIfCastChoice(OptionalCostPaid") && text.contains("Teamwork")), "{statics:?}");

    let qasali = compile_to_runtime_definition("Qasali Ambusher",
        "Mana cost: {1}{G}{W}\nType: Creature — Cat Warrior\nPower/Toughness: 2/3\nReach\nIf a creature is attacking you and you control a Forest and a Plains, you may cast this creature without paying its mana cost and as though it had flash.", false)
        .expect("free flash cast should compile");
    let method = qasali.alternative_casts.iter().find(|method| method.grants_instant_timing())
        .unwrap_or_else(|| panic!("{:?}", qasali.alternative_casts));
    let condition = format!("{:?}", method.cast_condition());
    assert!(condition.contains("All") && condition.contains("CreatureIsAttackingYou"), "{condition}");
    assert!(condition.contains("Forest") && condition.contains("Plains"), "{condition}");
}

#[test]
fn watcher_of_hours_time_counter_removal_while_exiled() {
    let definition = compile_to_runtime_definition("Watcher of Hours",
        "Mana cost: {5}{U}\nType: Creature — Sphinx\nPower/Toughness: 4/6\nFlying, ward {3}\nWhenever you remove a time counter from this card while it's exiled, surveil 1.\nSuspend 6—{1}{U} (Rather than cast this card from your hand, you may pay {1}{U} and exile it with six time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost. It has haste.)", false)
        .expect("exiled time-counter removal trigger should compile");
    let found = p9_triggered(&definition).into_iter().any(|trigger| {
        let debug = format!("{:?}", trigger.trigger);
        debug.contains("CounterRemovedFrom") && debug.contains("Time") && debug.contains("Exile")
            && debug.contains("You")
            && format!("{:?}", trigger.effects).contains("Surveil")
    });
    assert!(found, "{:#?}", p9_triggered(&definition).iter().map(|t| format!("{:?}", t.trigger)).collect::<Vec<_>>());
}

#[test]
fn temporal_anchor_scry_bottom_trigger_exiles_from_bottom() {
    let definition = compile_to_runtime_definition("The Temporal Anchor",
        "Mana cost: {3}{U}{U}{U}\nType: Legendary Artifact\nAt the beginning of your upkeep, scry 2.\nWhenever you choose to put one or more cards on the bottom of your library while scrying, exile that many cards from the bottom of your library.\nDuring your turn, you may play cards exiled with The Temporal Anchor.", false)
        .expect("scry-bottom trigger should compile");
    let bottom = p9_triggered(&definition).into_iter()
        .find(|trigger| format!("{:?}", trigger.trigger).contains("ScryToBottom"))
        .unwrap_or_else(|| panic!("{:#?}", p9_triggered(&definition).iter().map(|t| format!("{:?}", t.trigger)).collect::<Vec<_>>()));
    let effects = format!("{:?}", bottom.effects);
    assert!(effects.contains("bottom_only: true"), "{effects}");
    assert!(!effects.contains("top_only: true"), "{effects}");
    assert!(effects.contains("count_value: Some"), "that many is dynamic: {effects}");
}

#[test]
fn molten_sentry_coin_flip_entry_characteristics() {
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Molten Sentry",
        "Mana cost: {3}{R}\nType: Creature — Elemental\nPower/Toughness: 0/0\nAs this creature enters, flip a coin. If the coin comes up heads, this creature enters as a 5/2 creature with haste. If it comes up tails, this creature enters as a 2/5 creature with defender.", false)
        .expect("coin-flip entry characteristics should compile");
    let statics = p9_statics(&definition);
    let entry = statics.iter().find(|text| text.contains("ChoosePowerToughnessAsEntersOrTurnsFaceUp"))
        .unwrap_or_else(|| panic!("{statics:?}"));
    assert!(entry.contains("at_random: true"), "{entry}");
    assert!(entry.contains("power: 5") && entry.contains("toughness: 5"), "{entry}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let result = game.move_object_with_etb_processing_with_dm(card, Zone::Battlefield, &mut dm)
        .expect("entry should execute");
    assert!(!result.pending, "entry should complete without a continuation");
    assert!(result.programs.is_empty(), "entry should complete all replacement programs");
    let result = result.original.into_result().expect("enters");
    let pt = (game.calculated_power(result.new_id), game.calculated_toughness(result.new_id));
    assert!(pt == (Some(5), Some(2)) || pt == (Some(2), Some(5)), "5/2 or 2/5: {pt:?}");
}

#[test]
fn multiple_choice_do_all_of_the_above_repeats_paragraphs() {
    let definition = compile_to_runtime_definition("Multiple Choice",
        "Mana cost: {X}{U}\nType: Sorcery\nIf X is 1, scry 1, then draw a card.\nIf X is 2, you may choose a player. They return a creature they control to its owner's hand.\nIf X is 3, create a 4/4 blue and red Elemental creature token.\nIf X is 4 or more, do all of the above.", false)
        .expect("do all of the above should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.matches("ScryEffect").count() >= 2, "the X>=4 paragraph repeats scry: {debug}");
    assert!(debug.matches("CreateTokenEffect").count() >= 2, "the X>=4 paragraph repeats the token: {debug}");
    assert!(debug.contains("GreaterThanOrEqual") || debug.contains("AtLeast"), "{debug}");
}

#[test]
fn nephalia_academy_opponent_discard_to_library_top() {
    let definition = compile_to_runtime_definition("Nephalia Academy",
        "Type: Land\nIf a spell or ability an opponent controls causes you to discard a card, you may reveal that card and put it on top of your library instead of putting it anywhere else.\n{T}: Add {C}.", false)
        .expect("opponent-caused discard replacement should compile");
    let found = definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(ability)
            if ability.id() == ironsmith::static_abilities::StaticAbilityId::OpponentEffectDiscardToLibraryReplacement));
    assert!(found, "{:?}", p9_statics(&definition));
}

#[test]
fn riku_modal_spell_trigger_counts_chosen_modes() {
    let definition = compile_to_runtime_definition("Riku of Many Paths",
        "Mana cost: {G}{U}{R}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 3/3\nWhenever you cast a modal spell, choose up to X, where X is the number of times you chose a mode for that spell —\n• Exile the top card of your library. Until the end of your next turn, you may play it.\n• Put a +1/+1 counter on Riku. It gains trample until end of turn.\n• Create a 1/1 blue Bird creature token with flying.", false)
        .expect("modal-spell trigger should compile");
    let trigger = p9_triggered(&definition).into_iter().next().expect("cast trigger");
    let trigger_debug = format!("{:?}", trigger.trigger);
    assert!(trigger_debug.contains("\"modal\""), "{trigger_debug}");
    let effects = format!("{:?}", trigger.effects);
    assert!(effects.contains("ModeSelectionsOf"), "{effects}");
    assert!(effects.contains("triggering"), "that spell is the triggering spell: {effects}");
}

#[test]
fn tasha_casts_from_page_countered_exile() {
    let definition = compile_to_runtime_definition("Tasha, the Witch Queen",
        "Mana cost: {3}{U}{B}\nType: Legendary Planeswalker — Tasha\nLoyalty: 5\nWhenever you cast a spell you don't own, create a 3/3 black Demon creature token.\n+1: Draw a card. For each opponent, exile up to one target instant or sorcery card from that player's graveyard and put a page counter on it.\n−3: You may cast a spell from among cards in exile with page counters on them without paying its mana cost.\nTasha, the Witch Queen can be your commander.", false)
        .expect("page-counter exile cast should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Page"), "{debug}");
    let minus = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) if format!("{:?}", activated.effects).contains("CastTagged")
            || format!("{:?}", activated.effects).contains("Cast") => Some(format!("{:?}", activated.effects)),
        _ => None,
    }).expect("-3 cast ability");
    assert!(minus.contains("Exile") && minus.contains("Page"), "{minus}");
    assert!(!minus.contains("owner: Some(You)"), "any owner's exiled cards: {minus}");
}

#[test]
fn primeval_spawn_unpaid_entry_exile_and_total_mana_value_cast() {
    let definition = compile_to_runtime_definition("Primeval Spawn",
        "Mana cost: {5}{W}{U}{B}{R}{G}\nType: Creature — Avatar\nPower/Toughness: 10/10\nIf this creature would enter and it wasn't cast or no mana was spent to cast it, exile it instead.\nVigilance, trample, lifelink\nWhen this creature leaves the battlefield, exile the top ten cards of your library. You may cast any number of spells with total mana value 10 or less from among them without paying their mana costs.", false)
        .expect("unpaid-entry exile and total mana value cast should compile");
    let statics = p9_statics(&definition);
    assert!(statics.iter().any(|text| text.contains("RedirectWouldEnter") && text.contains("or_no_mana_spent: true")), "{statics:?}");
    let trigger = p9_triggered(&definition).into_iter().next().expect("leaves trigger");
    let effects = format!("{:?}", trigger.effects);
    assert!(effects.contains("aggregate_constraint: Some"), "{effects}");
    assert!(effects.contains("Fixed(10)"), "{effects}");
}

#[test]
fn rod_of_absorption_total_mana_value_collection_cast() {
    let definition = compile_to_runtime_definition("Rod of Absorption",
        "Mana cost: {2}{U}\nType: Artifact\nWhenever a player casts an instant or sorcery spell, exile it instead of putting it into a graveyard as it resolves.\n{X}, {T}, Sacrifice this artifact: You may cast any number of spells from among cards exiled with this artifact with total mana value X or less without paying their mana costs.", false)
        .expect("total mana value collection cast should compile");
    let trigger = p9_triggered(&definition).into_iter().next().expect("cast trigger");
    let trigger_effects = format!("{:?}", trigger.effects);
    assert!(trigger_effects.contains("Graveyard") && trigger_effects.contains("Exile"), "resolution destination replaced: {trigger_effects}");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(format!("{:?}", activated.effects)),
        _ => None,
    }).expect("sacrifice ability");
    assert!(activated.contains("aggregate_constraint: Some"), "{activated}");
    assert!(activated.contains("ManaValue"), "{activated}");
}

#[test]
fn ur_sphinx_casts_one_card_each_player_milled() {
    let definition = compile_to_runtime_definition("The Ur-Sphinx",
        "Mana cost: {6}{W}{U}{B}\nType: Legendary Creature — Sphinx Avatar\nPower/Toughness: 7/7\nEminence — As long as The Ur-Sphinx is in the command zone or on the battlefield, other Sphinx spells you cast cost {1} less to cast.\nFlying\nWhenever one or more Sphinxes you control attack, each player mills that many cards. For each player, you may cast a card that player milled this way without paying its mana cost.", false)
        .expect("per-player milled-card cast should compile");
    let attack = p9_triggered(&definition).into_iter()
        .find(|trigger| format!("{:?}", trigger.effects).contains("Mill"))
        .expect("attack trigger");
    let effects = format!("{:?}", attack.effects);
    assert!(effects.contains("IteratedPlayer"), "{effects}");
    assert!(effects.contains("CastTagged") || effects.contains("Cast"), "{effects}");
}
