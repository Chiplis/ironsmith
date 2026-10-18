//! Claude package 34 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p34_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

#[test]
fn hedonists_trove_splits_land_and_once_each_turn_spell_permissions() {
    let definition = p34_compile("Hedonist's Trove",
        "Mana cost: {5}{B}{B}\nType: Enchantment\nWhen this enchantment enters, exile target opponent's graveyard.\nYou may play lands from among cards exiled with this enchantment.\nYou may cast spells from among cards exiled with this enchantment. You can't cast more than one spell this way each turn.");
    let grants: Vec<String> = definition
        .abilities
        .iter()
        .map(|ability| format!("{ability:?}"))
        .filter(|debug| debug.contains("PlayFrom"))
        .collect();
    assert_eq!(grants.len(), 2, "one land and one spell permission: {grants:#?}");
    let land = grants.iter().find(|debug| debug.contains(", card_types: [Land]"))
        .unwrap_or_else(|| panic!("land-only permission: {grants:#?}"));
    assert!(land.contains("usage_limit: None"), "{land}");
    let spells = grants.iter().find(|debug| debug.contains("OnceEachTurn"))
        .unwrap_or_else(|| panic!("budgeted spell permission: {grants:#?}"));
    assert!(!spells.contains(", card_types: [Land]"), "the spell permission is not the land one: {spells}");
    for grant in &grants {
        assert!(grant.contains("source_exiled"), "both read the source-linked pool: {grant}");
    }
}

#[test]
fn historians_boon_triggers_on_final_chapter_ability_only() {
    let definition = p34_compile("Historian's Boon",
        "Mana cost: {3}{W}\nType: Enchantment\nWhenever this enchantment or another nontoken enchantment you control enters, create a 1/1 white Soldier creature token.\nWhenever the final chapter ability of a Saga you control triggers, create a 4/4 white Angel creature token with flying and vigilance.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("FinalChapterAbilityTriggers"), "{debug}");
    assert!(!debug.contains("FinalChapterAbilityResolved"), "triggers, not resolves: {debug}");
    assert!(debug.contains("Saga"), "{debug}");
    assert!(debug.contains("Angel"), "{debug}");
}

#[test]
fn horn_of_plenty_delayed_draw_goes_to_the_casting_player() {
    let definition = p34_compile("Horn of Plenty",
        "Mana cost: {6}\nType: Artifact\nWhenever a player casts a spell, they may pay {1}. If the player does, they draw a card at the beginning of the next end step.");
    let debug = format!("{:?}", definition.abilities);
    let delayed = debug.find("ScheduleDelayedTriggerEffect").expect("delayed end-step draw");
    let draw = debug[delayed..].find("DrawCardsEffect").expect("delayed draw");
    let tail = &debug[delayed + draw..(delayed + draw + 120).min(debug.len())];
    assert!(tail.contains("IteratedPlayer"), "the casting player draws, not the controller: {tail}");
    assert!(debug.contains("PayManaEffect"), "{debug}");
}

#[test]
fn hushbringer_suppresses_entering_and_dying_triggers() {
    let definition = p34_compile("Hushbringer",
        "Mana cost: {1}{W}\nType: Creature — Faerie\nPower/Toughness: 1/2\nFlying, lifelink\nCreatures entering or dying don't cause abilities to trigger.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SuppressMatchingTriggeredAbilities"), "{debug}");
    let suppress = debug.find("SuppressMatchingTriggeredAbilities").unwrap();
    let tail = &debug[suppress..];
    assert!(tail.matches("ZoneChangeTrigger").count() >= 2, "entering and dying arms: {tail}");
    assert!(tail.contains("Graveyard"), "dying is suppressed: {tail}");
}

#[test]
fn hold_for_ransom_restricts_and_grants_ransom_ability() {
    let definition = p34_compile("Hold for Ransom",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't attack or block and has \"{7}: Hold for Ransom's controller sacrifices it and draws a card. Activate only as a sorcery.\"");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.to_ascii_lowercase().contains("attack"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("SorcerySpeed") || debug.contains("Sorcery"), "{debug}");
}

#[test]
fn harness_the_storm_targets_same_name_graveyard_card_and_offers_cast() {
    let definition = p34_compile("Harness the Storm",
        "Mana cost: {2}{R}\nType: Enchantment\nWhenever you cast an instant or sorcery spell from your hand, you may cast target card with the same name as that spell from your graveyard. (You still pay its costs.)");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SameNameAsTagged"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("MayEffect") || debug.contains("May"), "{debug}");
    assert!(debug.contains("CastTagged") || debug.contains("CastTaggedEffect"), "{debug}");
    use ironsmith::{PlayerId, Zone};
    use ironsmith::game_state::Target;
    struct Answers { target: ironsmith::ObjectId, accept: bool, legal: Vec<Target> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(ctx.requirements.len(), 1);
            self.legal = ctx.requirements[0].legal_targets.clone();
            assert_eq!(self.legal, vec![Target::Object(self.target)], "same name and your graveyard only");
            vec![Target::Object(self.target)]
        }
        fn decide_boolean(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
            assert_eq!(ctx.player, PlayerId::from_index(0)); self.accept
        }
        fn decide_options(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Confirm mana payment for") && ctx.options.iter().any(|option| option.index == 1 && option.legal) { vec![1] }
            else { ironsmith::decision::DecisionMaker::decide_options(&mut ironsmith::decision::SelectFirstDecisionMaker, game, ctx) }
        }
    }
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in 0..6 {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Matching cast witness")
            .card_types(vec![if case == 5 { ironsmith::types::CardType::Creature } else { ironsmith::types::CardType::Instant }])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Red]))
            .with_spell_effect(vec![Effect::gain_life(1)]).build();
        let target = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
        game.create_object_from_definition(&spell, bob, Zone::Graveyard);
        let other = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Different name witness")
            .card_types(vec![ironsmith::types::CardType::Instant]).build();
        game.create_object_from_definition(&other, alice, Zone::Graveyard);
        if case != 2 { game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 1); }
        let caster = if case == 3 { bob } else { alice };
        let origin = if case == 4 { Zone::Graveyard } else { Zone::Hand };
        let original = game.create_object_from_definition(&spell, caster, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(original, caster));
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, original).unwrap();
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new_with_snapshot(original, caster, origin, snapshot), Default::default());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
        let mut dm = Answers { target, accept: case != 1, legal: Vec::new() };
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        let triggers = case < 3;
        assert_eq!(game.stack.len(), if triggers { 2 } else { 1 }, "trigger eligibility case {case}");
        if triggers { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap_or_else(|error| panic!("cast case {case}: {error:?}")); }
        let cast = case == 0;
        assert_eq!(game.stack.len(), if cast { 2 } else { 1 }, "optional normal-cost cast case {case}");
        assert_eq!(game.player(alice).unwrap().mana_pool.red, if cast || case == 2 { 0 } else { 1 });
        assert_eq!(game.player(alice).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Matching cast witness"), !cast);
        assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(alice).unwrap().life, 20, "the original spell must not resolve during a canceled nested cast");
    }
}

#[test]
fn imskir_draws_and_loses_half_artifacts_rounded_down() {
    let definition = p34_compile("Imskir Iron-Eater",
        "Mana cost: {6}{B}{R}\nType: Legendary Creature — Demon\nPower/Toughness: 5/5\nAffinity for artifacts (This spell costs {1} less to cast for each artifact you control.)\nWhen Imskir enters, you draw X cards and you lose X life, where X is half the number of artifacts you control, rounded down.\n{3}{R}, Sacrifice an artifact: Imskir deals damage equal to the sacrificed artifact's mana value to any target.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DrawCardsEffect") && debug.contains("LoseLifeEffect"), "{debug}");
    assert!(debug.contains("BindXValueEffect") && debug.contains("HalfRoundedDown"), "one shared half-artifact value: {debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    for artifacts in 0..6 {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let artifact = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Artifact count witness")
            .card_types(vec![ironsmith::types::CardType::Artifact]).build();
        for _ in 0..artifacts { game.create_object_from_card(&artifact, alice, ironsmith::Zone::Battlefield); }
        for _ in 0..3 { game.create_object_from_card(&artifact, bob, ironsmith::Zone::Battlefield); }
        let card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Draw witness")
            .card_types(vec![ironsmith::types::CardType::Instant]).build();
        for _ in 0..4 { game.create_object_from_card(&card, alice, ironsmith::Zone::Library); }
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Hand);
        game.move_object(source, ironsmith::Zone::Battlefield, ironsmith::events::EventCause::effect());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let half = artifacts / 2;
        assert_eq!(game.player(alice).unwrap().hand.len(), half as usize, "draw count for {artifacts} artifacts");
        assert_eq!(game.player(alice).unwrap().life, 20 - half);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
    // A draw replacement changes the artifact count midway through resolution.
    // Both actions must use the single X calculated before drawing.
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let artifact = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Changing count witness")
        .card_types(vec![ironsmith::types::CardType::Artifact]).build();
    let returning = game.create_object_from_card(&artifact, alice, ironsmith::Zone::Battlefield);
    let owner = game.create_object_from_card(&artifact, alice, ironsmith::Zone::Battlefield);
    let card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Draw witness")
        .card_types(vec![ironsmith::types::CardType::Instant]).build();
    game.create_object_from_card(&card, alice, ironsmith::Zone::Library);
    game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
        owner, alice, ironsmith::events::cards::matchers::WouldDrawCardMatcher::you(),
        ironsmith::replacement::ReplacementAction::Instead(vec![
            Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(ironsmith::target::ChooseSpec::SpecificObject(returning))), Effect::draw(1),
        ]),
    ));
    let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Hand);
    game.move_object(source, ironsmith::Zone::Battlefield, ironsmith::events::EventCause::effect());
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 2, "returned artifact plus drawn card");
    assert_eq!(game.player(alice).unwrap().life, 19, "X remains one after the draw changes the artifact count");

}

#[test]
fn henry_wu_exploited_power_gate_reads_exploited_creature_lki() {
    let definition = p34_compile("Henry Wu, InGen Geneticist",
        "Mana cost: {B}{G}{U}\nType: Legendary Creature — Human Scientist\nPower/Toughness: 1/4\nHenry Wu and other Human creatures you control have exploit. (When a creature with exploit enters, you may sacrifice a creature.)\nWhenever a creature you control exploits a non-Human creature, draw a card. If the exploited creature had power 3 or greater, create a Treasure token.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Exploit"), "{debug}");
    assert!(debug.contains("exploited"), "the gate reads the exploited creature: {debug}");
    assert!(debug.contains("GreaterThanOrEqual(3)"), "{debug}");
    assert!(debug.contains("Treasure"), "{debug}");
    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut no_human_type = definition.clone();
    no_human_type.card.subtypes = vec![ironsmith::types::Subtype::Scientist];
    let source = game.create_object_from_definition(&no_human_type, alice, ironsmith::Zone::Battlefield);
    let creature = |subtype| ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Grant recipient witness")
        .card_types(vec![ironsmith::types::CardType::Creature]).subtypes(vec![subtype])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
    let human = game.create_object_from_card(&creature(ironsmith::types::Subtype::Human), alice, ironsmith::Zone::Battlefield);
    let elf = game.create_object_from_card(&creature(ironsmith::types::Subtype::Elf), alice, ironsmith::Zone::Battlefield);
    let opposing = game.create_object_from_card(&creature(ironsmith::types::Subtype::Human), bob, ironsmith::Zone::Battlefield);
    fn sacrifices(effect: &Effect) -> bool {
        let mut found = effect.downcast_ref::<ironsmith::effects::SacrificeEffect>().is_some();
        effect.visit_child_effects(&mut |child| found |= sacrifices(child));
        found
    }
    let has_exploit = |game: &ironsmith::GameState, id| game.current_abilities(id).unwrap().iter().any(|ability| {
        matches!(&ability.kind, ironsmith::ability::AbilityKind::Triggered(trigger)
            if trigger.effects.all_effects().iter().any(|effect| sacrifices(effect)))
    });
    assert!(has_exploit(&game, source), "the explicit source gets exploit even if it is no longer Human");
    assert!(has_exploit(&game, human));
    assert!(!has_exploit(&game, elf));
    assert!(!has_exploit(&game, opposing));
    game.object_mut(human).unwrap().subtypes = vec![ironsmith::types::Subtype::Elf].into();
    game.object_mut(elf).unwrap().subtypes = vec![ironsmith::types::Subtype::Human].into();
    assert!(!has_exploit(&game, human));
    assert!(has_exploit(&game, elf));
    game.move_object(source, ironsmith::Zone::Graveyard, ironsmith::events::EventCause::effect());
    assert!(!has_exploit(&game, elf));
}

#[test]
fn hive_mind_each_other_player_copies_the_cast_spell() {
    let definition = p34_compile("Hive Mind",
        "Mana cost: {5}{U}\nType: Enchantment\nWhenever a player casts an instant or sorcery spell, each other player copies that spell. Each of those players may choose new targets for their copy.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    assert!(debug.contains("Excluding"), "the caster does not copy: {debug}");
    assert!(debug.contains("IteratedPlayer"), "each other player is the copier: {debug}");
    use ironsmith::{PlayerId, Zone};
    let alice = PlayerId::from_index(0);
    for caster_index in 0..4 { for kind in [ironsmith::types::CardType::Instant, ironsmith::types::CardType::Sorcery, ironsmith::types::CardType::Creature] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let caster = PlayerId::from_index(caster_index);
        let spell = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Copied spell witness")
            .card_types(vec![kind]).with_spell_effect(vec![Effect::gain_life(1)]).build();
        let original = game.create_object_from_definition(&spell, caster, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(original, caster));
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, original).unwrap();
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new_with_snapshot(original, caster, Zone::Graveyard, snapshot), Default::default());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let triggers = kind != ironsmith::types::CardType::Creature;
        assert_eq!(game.stack.len(), if triggers { 2 } else { 1 });
        if triggers { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        assert_eq!(game.stack.len(), if triggers { 4 } else { 1 });
        if triggers {
            let mut controllers: Vec<_> = game.stack.iter().map(|entry| entry.controller).collect(); controllers.sort();
            assert_eq!(controllers, (0..4).map(PlayerId::from_index).collect::<Vec<_>>(), "each opponent of caster owns exactly one copy");
            assert_eq!(game.stack.iter().filter(|entry| entry.object_id == original).count(), 1);
            // Copies aren't cast and must not recursively trigger this enchantment.
            ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 4);
            while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            for index in 0..4 { assert_eq!(game.player(PlayerId::from_index(index)).unwrap().life, 21); }
        }
    }}
    struct Retarget { accept: bool, choosers: Vec<PlayerId>, target_choices: usize }
    impl ironsmith::decision::DecisionMaker for Retarget {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.choosers.push(ctx.player); self.accept
        }
        fn decide_targets(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            let target = ironsmith::game_state::Target::Player(ctx.player);
            assert!(ctx.requirements[0].legal_targets.contains(&target));
            self.target_choices += 1; vec![target]
        }
    }
    for accept in [false, true] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let bob = PlayerId::from_index(1);
        let spell = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Targeted copy witness")
            .card_types(vec![ironsmith::types::CardType::Instant]).with_spell_effect(vec![Effect::new(ironsmith::effects::TargetOnlyEffect::new(ironsmith::target::ChooseSpec::target_player())), Effect::target_draws(1, ironsmith::target::PlayerFilter::Target(Box::new(ironsmith::target::PlayerFilter::Any)))]).build();
        let original = game.create_object_from_definition(&spell, bob, Zone::Stack);
        let mut entry = ironsmith::game_state::StackEntry::new(original, bob);
        entry.targets = vec![ironsmith::game_state::Target::Player(bob)]; game.push_to_stack(entry);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, original).unwrap();
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new_with_snapshot(original, bob, Zone::Hand, snapshot), Default::default());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for trigger in ironsmith::triggers::check_triggers(&game, &event) { queue.add(trigger); }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let mut dm = Retarget { accept, choosers: Vec::new(), target_choices: 0 };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        dm.choosers.sort();
        assert_eq!(dm.choosers, vec![alice, PlayerId::from_index(2), PlayerId::from_index(3)], "each copier decides for their own copy");
        assert_eq!(dm.target_choices, if accept { 3 } else { 0 });
        assert_eq!(game.stack.len(), 4);
        for entry in game.stack.iter() {
            let expected = if accept && entry.object_id != original { entry.controller } else { bob };
            assert_eq!(entry.targets, vec![ironsmith::game_state::Target::Player(expected)]);
        }
    }
}

#[test]
fn hit_the_mother_lode_creates_treasures_for_the_mana_value_gap() {
    let definition = p34_compile("Hit the Mother Lode",
        "Mana cost: {4}{R}{R}{R}\nType: Sorcery\nDiscover 10. If the discovered card's mana value is less than 10, create a number of tapped Treasure tokens equal to the difference. (To discover 10, exile cards from the top of your library until you exile a nonland card with mana value 10 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("DiscoverEffect"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "the gate and count read the discovered card: {debug}");
    assert!(debug.contains("Treasure"), "{debug}");
}

#[test]
fn honest_work_turns_enchanted_creature_into_humble_merchant() {
    let definition = p34_compile("Honest Work",
        "Mana cost: {U}\nType: Enchantment — Aura\nEnchant creature an opponent controls\nWhen this Aura enters, tap enchanted creature and remove all counters from it.\nEnchanted creature loses all abilities and is a Citizen with base power and toughness 1/1 and \"{T}: Add {C}\" named Humble Merchant. (It loses all other creature types and names.)");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Humble Merchant"), "{debug}");
    assert!(debug.contains("Citizen"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(debug.contains("Colorless") || debug.contains("AddMana"), "the granted mana ability survives: {debug}");
}

#[test]
fn hot_pursuit_goads_while_on_battlefield_and_steals_after_two_losses() {
    let definition = p34_compile("Hot Pursuit",
        "Mana cost: {1}{R}\nType: Enchantment\nWhen this enchantment enters, suspect target creature an opponent controls. As long as this enchantment remains on the battlefield, that creature is also goaded.\nAt the beginning of combat on your turn, if two or more players have lost the game, gain control of all goaded and/or suspected creatures until end of turn. Untap them. They gain haste until end of turn.");
    let debug = format!("{:?}", definition.abilities);
    let goad = debug.find("GoadEffect").expect("goad while the enchantment remains");
    let tail = &debug[goad..(goad + 600).min(debug.len())];
    assert!(tail.contains("ObjectOnBattlefield") && tail.contains("Source"),
        "goaded for as long as the source remains on the battlefield: {tail}");
    assert!(!tail.contains("YourNextTurn"), "{tail}");
    assert!(debug.contains("PlayersLostGame"), "{debug}");
    assert!(debug.contains("goaded: true") && debug.contains("suspected: true"), "{debug}");
    assert!(debug.contains("ChangeControllerToEffectController"), "{debug}");
}

#[test]
fn immortal_obligation_duty_counter_scopes_goad_and_restrictions() {
    let definition = p34_compile("Immortal Obligation",
        "Mana cost: {1}{W}\nType: Instant\nReturn target creature card from an opponent's graveyard to the battlefield under their control with a duty counter on it. For as long as that creature has a duty counter on it, it is goaded, can't attack you or a permanent you control, and can't block creatures you control.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("GoadEffect"), "{debug}");
    assert!(debug.contains("AttackPlayerOrPlaneswalkersControlledBy"), "{debug}");
    assert!(debug.contains("BlockSpecificAttacker") || debug.contains("Block"), "{debug}");
    assert!(debug.matches("ObjectHasCounter").count() >= 2,
        "goad and restrictions last while the duty counter remains: {debug}");
    assert!(!debug.contains("YouStopControllingThis"), "{debug}");
}
