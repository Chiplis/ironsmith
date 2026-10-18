//! Claude package 18 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p18_find_effects<T: Clone + 'static>(effect: &Effect, found: &mut Vec<T>) {
    if let Some(hit) = effect.downcast_ref::<T>() {
        found.push(hit.clone());
    }
    effect.visit_child_effects(&mut |child| p18_find_effects(child, found));
}

fn p18_triggered_effects<T: Clone + 'static>(definition: &ironsmith::cards::CardDefinition) -> Vec<T> {
    let mut found = Vec::new();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                p18_find_effects(effect, &mut found);
            }
        }
    }
    found
}

#[test]
fn whispersteel_dagger_grants_one_any_color_cast_from_the_damaged_players_graveyard() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Whispersteel Dagger",
        "Type: Artifact — Equipment\nEquipped creature gets +2/+0.\nWhenever equipped creature deals combat damage to a player, you may cast a creature spell from that player's graveyard this turn, and you may spend mana as though it were mana of any color to cast that spell.\nEquip {3}", false).unwrap();
    let grants = p18_triggered_effects::<ironsmith::effects::GrantBySpecEffect>(&definition);
    assert_eq!(grants.len(), 1, "{definition:#?}");
    let grant = &grants[0];
    assert_eq!(grant.spec.zone, Zone::Graveyard);
    assert_eq!(grant.spec.max_plays, Some(1), "a creature spell is one use");
    assert_eq!(grant.spec.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    assert!(grant.spec.filter.card_types.contains(&CardType::Creature), "{:?}", grant.spec.filter);
    assert_ne!(grant.spec.filter.owner, Some(ironsmith::target::PlayerFilter::You), "{:?}", grant.spec.filter);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Graveyard creature").card_types(vec![CardType::Creature]).build();
    let bobs = game.create_object_from_card(&creature, bob, Zone::Graveyard);
    let alices = game.create_object_from_card(&creature, alice, Zone::Graveyard);
    let source = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::DamageEvent::with_cause(
            source, ironsmith::events::DamageTarget::Player(bob), 1, true,
            ironsmith::events::EventCause::from_effect(source, alice),
        ),
        ironsmith::provenance::ProvNodeId::default(),
    );
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice)
        .with_triggering_event(event);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(grant.clone()), &mut ctx).unwrap();
    // The permission is locked to Bob: it survives the resolution context.
    assert!(!game.effect_store.grant_registry.granted_play_from_for_card(&game, bobs, Zone::Graveyard, alice).is_empty());
    assert!(game.effect_store.grant_registry.granted_play_from_for_card(&game, alices, Zone::Graveyard, alice).is_empty());
}

#[test]
fn tavern_brawler_quoted_trigger_plays_the_exiled_card_after_a_source_pump() {
    let definition = compile_to_runtime_definition("Tavern Brawler",
        "Type: Legendary Enchantment — Background\nCommander creatures you own have \"At the beginning of your upkeep, exile the top card of your library. This creature gets +X/+0 until end of turn, where X is that card's mana value. You may play that card this turn.\"", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("GrantPlayTagged"), "{debug}");
    // The permission names the exiled card, never the pumped source.
    let start = debug.find("GrantPlayTagged").unwrap();
    let permission = &debug[start..(start + 400).min(debug.len())];
    assert!(!permission.contains("tag: TagKey(\"__it__\")") && !permission.contains("Source"), "{permission}");
}

#[test]
fn snort_damages_each_opponent_who_accepted_the_optional_wheel() {
    let definition = compile_to_runtime_definition("Snort",
        "Type: Sorcery\nEach player may discard their hand and draw five cards. Then Snort deals 5 damage to each opponent who discarded their hand this way.\nFlashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)", false).unwrap();
    let debug = format!("{:?}", definition.spell_effect);
    // The per-opponent gate is the accepted optional choice, not a tagged
    // discarded-card test (an empty hand still counts).
    assert!(debug.contains("Chosen"), "{debug}");
    assert!(!debug.contains("PlayerTaggedObjectMatches"), "{debug}");
}

#[test]
fn talent_of_the_telepath_spell_mastery_restates_the_collection_cast_with_two() {
    let definition = compile_to_runtime_definition("Talent of the Telepath",
        "Type: Sorcery\nTarget opponent reveals the top seven cards of their library. You may cast an instant or sorcery spell from among them without paying its mana cost. Then that player puts the rest into their graveyard.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may cast up to two instant and/or sorcery spells from among the revealed cards instead of one.", false).unwrap();
    let debug = format!("{:?}", definition.spell_effect);
    assert_eq!(debug.matches("ChooseObjectsEffect").count(), 2, "{debug}");
    assert_eq!(debug.matches("LookAtTopCardsEffect").count(), 1, "one reveal shared by both branches: {debug}");
}

#[test]
fn ziatoras_envoy_plays_a_land_or_casts_a_cheap_spell_from_the_looked_top_card() {
    let definition = compile_to_runtime_definition("Ziatora's Envoy",
        "Type: Creature — Lizard Warrior\nPower/Toughness: 3/3\nTrample\nWhenever this creature deals combat damage to a player, look at the top card of your library. You may play a land from the top of your library or cast a spell with mana value less than or equal to the damage dealt from the top of your library without paying its mana cost. If you don't, put that card into your hand.\nBlitz {2}{B}{R}{G}", false).unwrap();
    let casts = p18_triggered_effects::<ironsmith::effects::CastTaggedEffect>(&definition);
    assert_eq!(casts.len(), 1, "{definition:#?}");
    assert!(casts[0].allow_land, "a land is played through the same instruction");
    assert!(casts[0].without_paying_mana_cost);
    let debug = format!("{definition:?}");
    assert!(debug.contains("Hand"), "the declined card goes to hand: {debug}");
}

#[test]
fn shamans_trance_swaps_graveyard_play_rights_for_the_turn() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Shaman's Trance",
        "Type: Instant\nOther players can't play lands or cast spells from their graveyards this turn. You may play lands and cast spells from other players' graveyards this turn as though those cards were in your graveyard.", false).unwrap();
    let mut grants = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p18_find_effects::<ironsmith::effects::GrantBySpecEffect>(effect, &mut grants);
    }
    assert_eq!(grants.len(), 1, "{definition:#?}");
    assert_eq!(grants[0].spec.filter.owner, Some(ironsmith::target::PlayerFilter::NotYou));
    assert_eq!(grants[0].spec.max_plays, None);
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("NotYou") && debug.contains("CastSpellsMatching") && debug.contains("PlayLandsMatching"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Graveyard creature").card_types(vec![CardType::Creature]).build();
    let bobs = game.create_object_from_card(&creature, bob, Zone::Graveyard);
    let source = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(grants[0].clone()), &mut ctx).unwrap();
    assert!(!game.effect_store.grant_registry.granted_play_from_for_card(&game, bobs, Zone::Graveyard, alice).is_empty());
    assert!(game.effect_store.grant_registry.granted_play_from_for_card(&game, bobs, Zone::Graveyard, bob).is_empty());
}

#[test]
fn skyway_robber_escapes_with_a_free_cast_trigger_over_its_linked_exile() {
    use ironsmith::Zone;
    let definition = compile_to_runtime_definition("Skyway Robber",
        "Type: Creature — Bird Rogue\nFlying\nEscape—{3}{U}, Exile five other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)\nThis creature escapes with \"Whenever this creature deals combat damage to a player, you may cast an artifact, instant, or sorcery spell from among cards exiled with this creature without paying its mana cost.\"", false).unwrap();
    let debug = format!("{definition:?}");
    // A self grant gated on the escape, carrying a free cast restricted to
    // the cards linked to this permanent (its escape-cost exiles).
    assert!(debug.contains("GrantObjectAbilityForFilter"), "{debug}");
    assert!(debug.contains("condition: Some(ThisSpellEscaped)"), "{debug}");
    assert!(debug.contains("ThisDealsCombatDamageToPlayerTrigger"), "{debug}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    let surface = ironsmith::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(surface.to_ascii_lowercase().contains("this creature escapes with \"whenever"), "{surface}");

    // Only the escaped permanent has the granted trigger.
    let alice = ironsmith::PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hard_cast = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let escaped = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.object_mut(escaped).unwrap().optional_costs_paid.mark_label_paid("Escape");
    game.refresh_continuous_state().unwrap();
    let triggered = |id| {
        game.current_abilities(id)
            .unwrap_or_default()
            .iter()
            .filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
            .count()
    };
    assert_eq!(triggered(hard_cast), 0);
    assert_eq!(triggered(escaped), 1);
}

#[test]
fn nezumi_graverobber_flips_only_when_the_exiled_cards_graveyard_is_empty() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    use ironsmith_core::effect_model::ValueComparisonOperator;
    use ironsmith_core::value_model::{Condition, Value};
    let front = compile_to_runtime_definition("Nezumi Graverobber",
        "Mana cost: {1}{B}\nType: Creature — Rat Rogue\n{1}{B}: Exile target card from an opponent's graveyard. If no cards are in that graveyard, flip this creature.", false).unwrap();
    let back = compile_to_runtime_definition("Nighteyes the Desecrator",
        "Type: Legendary Creature — Rat Wizard\n{4}{B}: Put target creature card from a graveyard onto the battlefield under your control.", false).unwrap();
    assert!(format!("{:?}", back.abilities).contains("Battlefield"), "{back:#?}");
    let mut conditionals = Vec::new();
    for ability in &front.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.all_effects() {
                p18_find_effects::<ironsmith::effects::ConditionalEffect>(effect, &mut conditionals);
            }
        }
    }
    assert_eq!(conditionals.len(), 1, "{front:#?}");
    let conditional = &conditionals[0];
    let Condition::ValueComparison { left: Value::CardsInGraveyard(player), operator, right: Value::Fixed(0) } = &conditional.condition else {
        panic!("expected an empty-graveyard count, got {:?}", conditional.condition);
    };
    assert_eq!(*player, ironsmith::target::PlayerFilter::OwnerOf(ironsmith::target::ObjectRef::Target));
    assert_eq!(*operator, ValueComparisonOperator::Equal);
    assert!(format!("{:?}", conditional.if_true).contains("FlipEffect"), "{:?}", conditional.if_true);

    // The target card has already left its graveyard; its owner is read
    // through the target snapshot, and the condition counts that graveyard.
    for (others, expect_true) in [(0usize, true), (1, false)] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Graveyard card").card_types(vec![CardType::Creature]).build();
        let target = game.create_object_from_card(&card, bob, Zone::Graveyard);
        for _ in 0..others {
            game.create_object_from_card(&card, bob, Zone::Graveyard);
        }
        game.create_object_from_card(&card, alice, Zone::Graveyard); // Another graveyard is irrelevant.
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        game.move_object_by_effect(target, Zone::Exile).unwrap();
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
        ctx.target_snapshots.insert(target, snapshot);
        let probe = Effect::conditional_only(conditional.condition.clone(), vec![Effect::gain_life(1)]);
        ironsmith::effects::execute_effect(&mut game, &probe, &mut ctx).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 20 + i32::from(expect_true), "others={others}");
    }
}

#[test]
fn monk_class_exiled_card_casts_from_exile_only_after_another_spell_this_turn() {
    use ironsmith::card::CardBuilder;
    use ironsmith::grant_registry::GrantSource;
    use ironsmith::ids::CardId;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Monk Class",
        "Type: Enchantment — Class\n(Gain the next level as a sorcery to add its ability.)\nThe second spell you cast each turn costs {1} less to cast.\n{W}{U}: Level 2\nWhen this Class becomes level 2, return up to one target nonland permanent to its owner's hand.\n{1}{W}{U}: Level 3\nAt the beginning of your upkeep, exile the top card of your library. For as long as it remains exiled, it has \"You may cast this card from exile as long as you've cast another spell this turn.\"", false).unwrap();
    let grants = p18_triggered_effects::<ironsmith::effects::GrantPlayTaggedEffect>(&definition);
    let grant = grants
        .iter()
        .find(|grant| grant.during_turns_cast_another_spell)
        .cloned()
        .unwrap_or_else(|| panic!("{definition:#?}"));
    assert_eq!(grant.duration, ironsmith::effects::GrantPlayTaggedDuration::ForAsLongAsExiled);
    assert!(!grant.allow_land, "the quoted ability only casts");
    assert_eq!(grant.player, ironsmith::target::PlayerFilter::You);

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Exiled creature").card_types(vec![CardType::Creature]).build();
    let exiled = game.create_object_from_card(&creature, alice, Zone::Exile);
    let source = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects(grant.tag.clone(), vec![ObjectSnapshot::from_object_id(&game, exiled).unwrap()]);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(grant.clone()), &mut ctx).unwrap();
    let registered = game
        .effect_store
        .grant_registry
        .grants
        .iter()
        .find(|grant| matches!(grant.source, GrantSource::EffectDuringTurnsCastAnotherSpell { .. }))
        .cloned()
        .expect("cast-history gated permission");
    assert!(!registered.source.is_valid(&game), "no spell cast yet this turn");
    let spell = game.create_object_from_card(&creature, alice, Zone::Stack);
    game.turn_store
        .turn_history
        .event_records
        .push(ironsmith::turn_history::TurnEventRecord {
            event: ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::SpellCastEvent::new(spell, alice, Zone::Hand),
                Default::default(),
            ),
            object_snapshot: None,
            source_snapshot: None,
        });
    assert!(registered.source.is_valid(&game), "another spell was cast this turn");
    game.next_turn();
    assert!(!registered.source.is_valid(&game), "a new turn resets the cast history");
}

#[test]
fn muldrotha_offers_one_graveyard_budget_per_permanent_type_and_lets_the_player_choose() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::CardBuilder;
    use ironsmith::decision::LegalAction;
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Muldrotha, the Gravetide",
        "Type: Legendary Creature — Elemental Avatar\nDuring each of your turns, you may play a land and cast a permanent spell of each permanent type from your graveyard. (If a card has multiple permanent types, choose one as you play it.)", false).unwrap();
    let budgets = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Static(ability) => ability.grant_spec(), _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(budgets.len(), 6, "{definition:#?}");
    for (spec, card_type) in budgets.iter().zip([CardType::Land, CardType::Artifact, CardType::Creature,
        CardType::Enchantment, CardType::Planeswalker, CardType::Battle])
    {
        assert_eq!(spec.zone, Zone::Graveyard);
        assert_eq!(spec.filter.card_types, vec![card_type]);
        assert_eq!(spec.filter.excluded_card_types.contains(&CardType::Land), card_type != CardType::Land);
        assert_eq!(spec.usage_limit, Some(ironsmith::grant::GrantUsageLimit::OnceDuringEachOfYourTurns));
    }
    let surface = ironsmith::runtime_display::compiled_text_lines(&definition).join("\n");
    assert_eq!(surface.matches("of each permanent type from your graveyard").count(), 1, "{surface}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 6);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let card = |name: &str, types: Vec<CardType>| CardBuilder::new(CardId::new(), name).card_types(types)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])).build();
    let both = game.create_object_from_card(&card("Artifact creature", vec![CardType::Artifact, CardType::Creature]), alice, Zone::Graveyard);
    let artifact = game.create_object_from_card(&card("Plain artifact", vec![CardType::Artifact]), alice, Zone::Graveyard);
    let casts = |game: &ironsmith::GameState, id| ironsmith::decision::compute_legal_actions(game, alice).unwrap().into_iter()
        .filter_map(|action| match action {
            LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, casting_method } if spell_id == id => Some(casting_method),
            _ => None,
        }).collect::<Vec<_>>();
    // The two-type card is offered once through each matching budget: the
    // selected permission is the permanent type chosen for it.
    let both_methods = casts(&game, both);
    assert_eq!(both_methods.len(), 2, "{both_methods:#?}");
    assert!(both_methods.iter().all(|method| matches!(method, CastingMethod::ExactPermission { .. })), "{both_methods:#?}");
    let artifact_methods = casts(&game, artifact);
    assert_eq!(artifact_methods.len(), 1, "one artifact permission: {artifact_methods:#?}");
    // A single matching budget uses the ordinary PlayFrom route. Only an
    // ambiguous multi-type choice needs an ExactPermission wrapper.
    let artifact_grants = game.effect_store.grant_registry.get_grants_for_card(
        &game, artifact, Zone::Graveyard, alice,
    );
    let artifact_budget = artifact_grants.iter().find(|grant|
        matches!(grant.grantable, ironsmith::grant::Grantable::PlayFrom))
        .and_then(|grant| grant.permission_identity.clone()).expect("artifact budget identity");
    assert!(both_methods.iter().any(|method| matches!(method,
        CastingMethod::ExactPermission { permission, .. } if permission.identity == artifact_budget)));
    // Spending the artifact budget leaves the creature budget for the
    // two-type card but no route for a second plain artifact.
    game.turn_store.grant_cast_uses_this_turn.insert((alice, artifact_budget));
    assert!(casts(&game, artifact).is_empty());
    assert_eq!(casts(&game, both).len(), 1);
    // The permissions function only during Muldrotha's controller's turns.
    game.turn.active_player = bob;
    game.turn_store.grant_cast_uses_this_turn.clear();
    assert!(casts(&game, both).is_empty());
}

#[test]
fn mavinda_turn_long_graveyard_permission_waives_surcharge_for_own_creature_targets() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Mavinda, Students' Advocate",
        "Type: Legendary Creature — Bird Advisor\nFlying\n{0}: You may cast target instant or sorcery card from your graveyard this turn. If that spell doesn't target a creature you control, it costs {8} more to cast this way. If that spell would be put into your graveyard, exile it instead. Activate only once each turn. (You still pay the spell's costs. Timing rules for the spell still apply.)", false).unwrap();
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("activated ability");
    let debug = format!("{:?}", activated.effects);
    // A permission lasting the turn (normal timing rules), never a cast
    // during the ability's resolution.
    assert!(!debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");
    let mut grants = Vec::new();
    for effect in activated.effects.all_effects() {
        p18_find_effects::<ironsmith::effects::GrantPlayTaggedEffect>(effect, &mut grants);
    }
    assert_eq!(grants.len(), 1, "{debug}");
    let grant = &grants[0];
    assert_eq!(grant.duration, ironsmith::effects::GrantPlayTaggedDuration::UntilEndOfTurn);
    assert_eq!(grant.spell_cost_increase.as_ref().map(|cost| cost.mana_value()), Some(8), "{grant:?}");
    let waiver = grant.spell_cost_increase_unless_targets.as_ref().expect("target-conditional surcharge");
    assert!(waiver.card_types.contains(&CardType::Creature), "{waiver:?}");
    assert_eq!(waiver.controller, Some(ironsmith::target::PlayerFilter::You), "{waiver:?}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let instant = CardBuilder::new(CardId::new(), "Graveyard instant").card_types(vec![CardType::Instant]).build();
    let bird = CardBuilder::new(CardId::new(), "Bird").card_types(vec![CardType::Creature]).build();
    let card = game.create_object_from_card(&instant, alice, Zone::Graveyard);
    let source = game.create_object_from_card(&bird, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Object(card)]);
    for effect in activated.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    // The permission stays on the card in the graveyard for the turn and
    // carries the conditional surcharge onto the selected cast.
    assert!(!game.effect_store.grant_registry.granted_play_from_for_card(&game, card, Zone::Graveyard, alice).is_empty());
    let constraints = game.effect_store.grant_registry.play_from_constraints_for_card(&game, card, Zone::Graveyard, alice, source);
    assert_eq!(constraints.spell_cost_increase.as_ref().map(|cost| cost.mana_value()), Some(8));
    assert!(constraints.spell_cost_increase_unless_targets.is_some(), "{constraints:?}");
}

#[test]
fn triple_triad_frees_your_exiled_card_and_other_exiled_cards_with_lesser_mana_value() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Triple Triad",
        "Mana cost: {3}{R}{R}{R}\nType: Enchantment\nAt the beginning of your upkeep, each player exiles the top card of their library. Until end of turn, you may play the card you own exiled this way and each other card exiled this way with lesser mana value than it without paying their mana costs.", false)
        .expect("Triple Triad should compile");
    let grants = p18_triggered_effects::<ironsmith::effects::GrantPlayTaggedEffect>(&definition);
    assert_eq!(grants.len(), 2, "play permission plus canonical free-cast permission: {definition:#?}");
    assert!(grants.iter().all(|grant| grant.duration == ironsmith::effects::GrantPlayTaggedDuration::UntilEndOfTurn));
    let play = grants.iter().find(|grant| grant.allow_land).expect("play includes lands");
    let free = grants.iter().find(|grant| grant.alternative_cost.as_ref()
        .is_some_and(|cost| cost.as_all().is_some_and(|components| components.is_empty())))
        .expect("canonical free-cast permission");
    assert_eq!(free.tag, play.tag, "the free price covers exactly the playable pool");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ForPlayersEffect") && debug.contains("GreatestManaValue"), "{debug}");
    assert!(!debug.contains("\"__it__\""), "every pool reference is bound: {debug}");

    let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.clone()),
        _ => None,
    }).expect("upkeep trigger");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let card = |name: &str, generic: u8| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(generic)]])).build();
    // Alice's card has mana value 3: Bob's 2 qualifies, Carol's 4 does not.
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.create_object_from_card(&card("Alice top", 3), alice, Zone::Library);
    game.create_object_from_card(&card("Bob top", 2), bob, Zone::Library);
    game.create_object_from_card(&card("Carol top", 4), carol, Zone::Library);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    for effect in triggered.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let exiled: Vec<_> = game.exile.iter().copied().collect();
    assert_eq!(exiled.len(), 3, "each player exiled their top card");
    for id in exiled {
        let owner = game.object(id).unwrap().owner;
        let playable = !game.effect_store.grant_registry
            .granted_play_from_for_card(&game, id, Zone::Exile, alice).is_empty();
        assert_eq!(playable, owner != carol, "owner {owner:?}");
        assert!(game.effect_store.grant_registry
            .granted_play_from_for_card(&game, id, Zone::Exile, bob).is_empty(), "only you may play them");
    }

    // With an empty library you exile nothing, so no card has lesser mana
    // value than "it" and nothing is playable.
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_card(&card("Bob top", 0), bob, Zone::Library);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    for effect in triggered.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let exiled: Vec<_> = game.exile.iter().copied().collect();
    assert_eq!(exiled.len(), 1);
    assert!(game.effect_store.grant_registry
        .granted_play_from_for_card(&game, exiled[0], Zone::Exile, alice).is_empty());
}

#[test]
fn musician_grants_its_upkeep_trigger_only_once_across_repeated_activations() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::object::CounterType;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Musician",
        "Mana cost: {2}{U}\nType: Creature — Human Wizard\nPower/Toughness: 1/3\nCumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\n{T}: Put a music counter on target creature. If it doesn't have \"At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it,\" it gains that ability.", false).unwrap();
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("tap ability");
    let effects = activated.effects.all_effects().into_iter().cloned().collect::<Vec<_>>();
    let mut grants = Vec::new();
    for effect in &effects {
        p18_find_effects::<ironsmith::effects::ApplyContinuousEffect>(effect, &mut grants);
    }
    assert_eq!(grants.len(), 1, "{definition:#?}");
    assert!(grants[0].grant_only_if_absent, "{:?}", grants[0]);
    assert_eq!(grants[0].until, ironsmith::effect::Until::Forever);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Music recipient")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let recipient = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let upkeep_triggers = |game: &ironsmith::GameState| game.current_abilities(recipient).unwrap()
        .iter()
        .filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
        .count();
    assert_eq!(upkeep_triggers(&game), 0);
    for _ in 0..2 {
        let mut ctx = EffectContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(recipient)]);
        for effect in &effects {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    assert_eq!(game.counter_count(recipient, CounterType::Music), 2);
    // The second activation adds a counter but no second copy of the trigger.
    assert_eq!(upkeep_triggers(&game), 1);
}

#[test]
fn portent_of_calamity_exiles_one_per_card_type_then_threshold_casts_and_hands_the_rest() {
    use ironsmith::types::CardType;
    use ironsmith::Zone;
    let definition = compile_to_runtime_definition("Portent of Calamity",
        "Type: Sorcery\nReveal the top X cards of your library. For each card type, you may exile a card of that type from among them. Put the rest into your graveyard. You may cast a spell from among the exiled cards without paying its mana cost if you exiled four or more cards this way. Then put the rest of the exiled cards into your hand.", false).unwrap();
    let mut chooses = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p18_find_effects::<ironsmith::effects::ChooseObjectsEffect>(effect, &mut chooses);
    }
    // One simultaneous choice: any number of revealed cards, each filling a
    // distinct card-type slot.
    let per_type = chooses
        .iter()
        .find(|choose| choose.filter.one_per_card_type)
        .unwrap_or_else(|| panic!("{definition:#?}"));
    assert_eq!(per_type.count, ironsmith::effect::ChoiceCount::any_number());
    assert_eq!(per_type.filter.zone, Some(Zone::Library));
    assert!(per_type.filter.card_types.is_empty(), "every card type is a slot");
    // The free cast offers at most one nonland card that is still exiled.
    let cast = chooses
        .iter()
        .find(|choose| choose.filter.zone == Some(Zone::Exile))
        .unwrap_or_else(|| panic!("{definition:#?}"));
    assert_eq!(cast.count, ironsmith::effect::ChoiceCount::up_to(1));
    assert!(cast.filter.excluded_card_types.contains(&CardType::Land));
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ExileEffect"), "{debug}");
    assert!(debug.contains("Fixed(4)"), "the free cast is gated on four exiled cards: {debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(debug.contains("Graveyard") && debug.contains("Hand"), "{debug}");
}

#[test]
fn xanathar_locks_out_the_chosen_opponent_and_opens_their_library_top() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Xanathar, Guild Kingpin",
        "Mana cost: {4}{U}{B}\nType: Legendary Creature — Beholder\nPower/Toughness: 5/6\nAt the beginning of your upkeep, choose target opponent. Until end of turn, that player can't cast spells, you may look at the top card of their library any time, you may play the top card of their library, and you may spend mana as though it were mana of any color to cast spells this way.", false).unwrap();
    let cants = p18_triggered_effects::<ironsmith::effects::CantEffect>(&definition);
    assert_eq!(cants.len(), 1, "{definition:#?}");
    let restriction = format!("{:?}", cants[0].restriction);
    assert!(restriction.contains("CastSpells") && restriction.contains("Target"), "{restriction}");
    let grants = p18_triggered_effects::<ironsmith::effects::GrantBySpecEffect>(&definition);
    assert_eq!(grants.len(), 1, "{definition:#?}");
    let grant = &grants[0];
    assert_eq!(grant.spec.zone, Zone::Library);
    assert!(grant.spec.top_card_only && grant.spec.may_look_at_top);
    assert_eq!(grant.spec.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    assert!(matches!(grant.spec.filter.owner, Some(ironsmith::target::PlayerFilter::Target(_) | ironsmith::target::PlayerFilter::AliasedTarget(_))), "{:?}", grant.spec.filter);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Library card").card_types(vec![CardType::Creature]).build();
    let bobs_deep = game.create_object_from_card(&card, bob, Zone::Library);
    let bobs_top = game.create_object_from_card(&card, bob, Zone::Library);
    let alices_top = game.create_object_from_card(&card, alice, Zone::Library);
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    // Execute the permission with "their library" bound to Bob.
    let mut bound = grant.clone();
    bound.spec.filter.owner = Some(ironsmith::target::PlayerFilter::IteratedPlayer);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.iteration.iterated_player = Some(bob);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(bound), &mut ctx).unwrap();
    let registry = &game.effect_store.grant_registry;
    assert!(!registry.granted_play_from_for_card(&game, bobs_top, Zone::Library, alice).is_empty());
    assert!(registry.granted_play_from_for_card(&game, bobs_deep, Zone::Library, alice).is_empty());
    assert!(registry.granted_play_from_for_card(&game, alices_top, Zone::Library, alice).is_empty());
    assert!(registry.granted_play_from_for_card(&game, bobs_top, Zone::Library, bob).is_empty());
    // The private view is of Bob's top, never Alice's own.
    assert!(registry.grants_private_library_top_view_of(&game, alice, bob));
    assert!(!registry.grants_private_library_top_view(&game, alice));
}

#[test]
fn unpredictable_cyclone_replaces_only_cycling_draws_with_a_shared_type_free_cast() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let text = "Type: Enchantment\nIf a cycling ability of another nonland card would cause you to draw a card, instead exile cards from the top of your library until you exile a card that shares a card type with the cycled card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of your library in a random order.\nCycling {2} ({2}, Discard this card: Draw a card.)";
    let definition = compile_to_runtime_definition("Unpredictable Cyclone", text, false).unwrap();
    let (cycling_source, replacement_effects) = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(s) => match &s.compiled_model()?.payload {
                ironsmith_core::StaticAbilityPayload::DrawReplacementWithEffects {
                    cycling_source: Some(filter),
                    replacement_effects,
                    ..
                } => Some((filter.clone(), replacement_effects.clone())),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("cycling-scoped draw replacement: {definition:#?}"));
    assert!(cycling_source.other, "{cycling_source:?}");
    assert!(cycling_source.excluded_card_types.contains(&CardType::Land), "{cycling_source:?}");
    let program = format!("{replacement_effects:?}");
    assert!(program.contains("TagTriggeringSource"), "{program}");
    assert!(program.contains("triggering_source") && program.contains("SharesCardType"), "{program}");
    assert!(program.contains("ConsultTopOfLibrary"), "{program}");
    let cycling_index = definition
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind,
            ironsmith::ability::AbilityKind::Activated(activated)
                if activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Cycling)))
        .expect("cycling ability");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let land = CardBuilder::new(CardId::new(), "Library land").card_types(vec![CardType::Land]).build();
    let enchantment = CardBuilder::new(CardId::new(), "Library enchantment")
        .card_types(vec![CardType::Enchantment]).build();
    game.create_object_from_card(&land, alice, Zone::Library);
    game.create_object_from_card(&enchantment, alice, Zone::Library);
    game.create_object_from_card(&land, alice, Zone::Library);
    let _cyclone = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    // Another nonland card's cycling ability resolving: its draw is replaced.
    let cycled = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(cycled).unwrap(), &game);
    game.update_replacement_effects().unwrap();
    let library_before = game.player(alice).unwrap().library.len();
    let draw = Effect::draw(ironsmith::effect::Value::Fixed(1));
    {
        let mut ctx = ironsmith::effects::EffectContext::new_default(cycled, alice)
            .with_ability_index(cycling_index)
            .with_source_snapshot(snapshot);
        ironsmith::effects::execute_effect(&mut game, &draw, &mut ctx).unwrap();
    }
    let hand_after_cycle = game.player(alice).unwrap().hand.len();
    assert_eq!(hand_after_cycle, 1, "only the cycled card itself stays in hand; nothing was drawn");
    assert!(game.exile.is_empty(), "every exiled card was cast or put on the bottom");
    let library_after = game.player(alice).unwrap().library.len();
    assert!(library_after == library_before || library_after + 1 == library_before,
        "the land stays in the library; only the shared-type card may leave: {library_before} -> {library_after}");
    // A draw no cycling ability caused is not replaced.
    {
        let mut ctx = ironsmith::effects::EffectContext::new_default(cycled, alice);
        ironsmith::effects::execute_effect(&mut game, &draw, &mut ctx).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().hand.len(), hand_after_cycle + 1);
}

#[test]
fn vault_112_chapter_three_exiles_as_many_cards_as_energy_paid_after_shuffling() {
    let definition = compile_to_runtime_definition("Vault 112: Sadistic Simulation",
        "Type: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI, II — Tap up to one target creature and put a stun counter on it. You get {E}{E} (two energy counters).\nIII — Pay any amount of {E}. If you paid one or more {E} this way, shuffle your library, then exile that many cards from the top. You may play one of those cards without paying its mana cost.", false).unwrap();
    let pays = p18_triggered_effects::<ironsmith::effects::PayAnyEnergyEffect>(&definition);
    assert_eq!(pays.len(), 1, "{definition:#?}");
    assert_eq!(pays[0].min_amount, 0, "any amount includes zero");
    let exiles = p18_triggered_effects::<ironsmith::effects::ExileTopOfLibraryEffect>(&definition);
    assert_eq!(exiles.len(), 1, "{definition:#?}");
    // "that many" is the energy payment's amount, not the chapter trigger's
    // lore-counter event amount.
    assert!(
        matches!(exiles[0].count.unhinted(), ironsmith::effect::Value::EffectValue(_)),
        "{:?}",
        exiles[0].count
    );
    assert_eq!(exiles[0].player, ironsmith::target::PlayerFilter::You, "{:?}", exiles[0].player);
    let debug = format!("{definition:?}");
    assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
    // Gate: one or more energy actually paid.
    assert!(debug.contains("IfEffect") && debug.contains("GreaterThan(0)"), "{debug}");
    // The free play admits a land (play, not cast) and is limited to one card.
    let casts = p18_triggered_effects::<ironsmith::effects::CastTaggedEffect>(&definition);
    assert_eq!(casts.len(), 1, "{definition:#?}");
    assert!(casts[0].allow_land, "{:?}", casts[0]);
    assert!(casts[0].without_paying_mana_cost);
    assert!(debug.contains("up_to") || debug.contains("UpTo") || debug.contains("max: Some(1)"), "{debug}");
}

#[test]
fn wheel_of_potential_reads_the_energy_payment_through_the_per_player_wheel() {
    let definition = compile_to_runtime_definition("Wheel of Potential",
        "Type: Sorcery\nYou get {E}{E}{E} (three energy counters), then you may pay any amount of {E}.\nEach player may exile their hand and draw a number of cards equal to the amount of {E} paid this way. If seven or more {E} was paid this way, you may play cards you own exiled this way until the end of your next turn.", false).unwrap();
    let mut draws = Vec::new();
    let mut grants = Vec::new();
    let mut pays = Vec::new();
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p18_find_effects::<ironsmith::effects::DrawCardsEffect>(effect, &mut draws);
        p18_find_effects::<ironsmith::effects::GrantPlayTaggedEffect>(effect, &mut grants);
        p18_find_effects::<ironsmith::effects::PayAnyEnergyEffect>(effect, &mut pays);
    }
    assert_eq!(pays.len(), 1, "{definition:#?}");
    assert_eq!(draws.len(), 1, "{definition:#?}");
    // The draw count is the energy payment's removed-energy result, not the
    // number of cards the same player just exiled from their hand.
    assert!(
        matches!(
            draws[0].count.unhinted(),
            ironsmith::effect::Value::PriorEffectMetric { query, .. }
                if query.action == Some(ironsmith_core::PriorEffectAction::Removed)
                    && query.counter_type == Some(ironsmith::object::CounterType::Energy)
        ),
        "{:?}",
        draws[0].count
    );
    // Seven-or-more gate over the same payment, then a play permission over
    // your own exiled cards (lands included) through your next turn.
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("GreaterThanOrEqual") && debug.contains("Fixed(7)"), "{debug}");
    assert_eq!(grants.len(), 1, "{definition:#?}");
    let grant = &grants[0];
    assert!(grant.allow_land, "{grant:?}");
    assert_eq!(grant.duration, ironsmith::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd);
    let owner = grant.spell_filter.as_ref().and_then(|filter| filter.owner.clone());
    assert_eq!(owner, Some(ironsmith::target::PlayerFilter::You), "{grant:?}");
}
