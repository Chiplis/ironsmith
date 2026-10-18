//! Claude package 3 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::PlayerId;
use ironsmith::target::ChooseSpec;
use ironsmith::card::CardBuilder;
use ironsmith::ids::CardId;
use ironsmith::types::CardType;
use ironsmith::Zone;

/// Every effect reachable from a definition (spell, activated, triggered),
/// including nested children.
fn p3_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() { walk(effect, &mut out); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() { walk(effect, &mut out); }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() { walk(effect, &mut out); }
            }
            _ => {}
        }
    }
    out
}

#[test]
fn mana_alternative_with_for_each_count_stays_one_add_mana_action() {
    let definition = compile_to_runtime_definition("Muerra, Trash Tactician",
        "Mana cost: {1}{R}{G}\nType: Legendary Creature — Raccoon Warrior\nAt the beginning of your first main phase, add {R} or {G} for each Raccoon you control.\nWhenever you expend 4, you gain 3 life. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)\nWhenever you expend 8, exile the top two cards of your library. Until the end of your next turn, you may play those cards.", false)
        .expect("mana alternative with a for-each count should compile");
    let effects = p3_effects(&definition);
    let add = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::AddManaOfAnyColorEffect>())
        .expect("one add-mana effect over the listed colors");
    assert_eq!(add.available_colors, Some(vec![ironsmith::color::Color::Red, ironsmith::color::Color::Green]));
    assert!(!matches!(add.amount, ironsmith::effect::Value::Fixed(_)), "amount must count Raccoons: {:?}", add.amount);
    assert!(effects.iter().all(|effect| effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>().is_none()),
        "the second mana symbol is not an alternative action");
}

#[test]
fn additional_steps_and_beginning_phase_compile_and_schedule_this_turn() {
    use ironsmith::effects::{AdditionalStepAnchor, AdditionalStepKind, AdditionalStepsEffect};
    use ironsmith::game_state::{AddedStepPlacement, Phase, Step};
    let obeka = compile_to_runtime_definition("Obeka, Splitter of Seconds",
        "Type: Legendary Creature — Ogre Warlock\nMenace\nWhenever Obeka deals combat damage to a player, you get that many additional upkeep steps after this phase.", false)
        .expect("player-owned additional steps should compile");
    let steps = p3_effects(&obeka).into_iter()
        .find_map(|effect| effect.downcast_ref::<AdditionalStepsEffect>().cloned())
        .expect("additional steps effect");
    assert_eq!(steps.step, AdditionalStepKind::Upkeep);
    assert_eq!(steps.anchor, AdditionalStepAnchor::AfterThisPhase);
    assert!(!matches!(steps.count, ironsmith::effect::Value::Fixed(_)), "the count is the damage dealt: {:?}", steps.count);
    assert_eq!(steps.player, Some(ironsmith::target::PlayerFilter::You));

    // CR 500.10a: "you get" adds the steps only during your own turn.
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (active, expected) in [(alice, 2usize), (bob, 0)] {
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = active;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::CombatDamage);
        let effect = Effect::new(AdditionalStepsEffect { count: ironsmith::effect::Value::Fixed(2), ..steps.clone() });
        let source = game.new_object_id();
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.turn_store.added_steps.len(), expected);
        assert!(game.turn_store.added_steps.iter().all(|added|
            added.step == Step::Upkeep && added.placement == AddedStepPlacement::AfterPhase(Phase::Combat)));
    }

    let sphinx = compile_to_runtime_definition("Sphinx of the Second Sun",
        "Type: Creature — Sphinx\nFlying\nAt the beginning of each of your postcombat main phases, there is an additional beginning phase after this phase. (The beginning phase includes the untap, upkeep, and draw steps.)", false)
        .expect("additional beginning phase should compile");
    let phases = p3_effects(&sphinx).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AdditionalPhasesEffect>().cloned())
        .expect("additional phase effect");
    assert_eq!(phases.phases, vec![ironsmith::effects::AdditionalPhase::Beginning]);

    // "after this step" placement, exercised through the Paradox Haze shape.
    let haze = compile_to_runtime_definition("Paradox Haze",
        "Type: Enchantment — Aura\nEnchant player\nAt the beginning of enchanted player's first upkeep each turn, that player gets an additional upkeep step after this step.", false)
        .expect("after-this-step addition should compile");
    let steps = p3_effects(&haze).into_iter()
        .find_map(|effect| effect.downcast_ref::<AdditionalStepsEffect>().cloned()).unwrap();
    assert_eq!((steps.step, steps.anchor, steps.count.clone()), (AdditionalStepKind::Upkeep, AdditionalStepAnchor::AfterThisStep, ironsmith::effect::Value::Fixed(1)));
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Upkeep);
    let source = game.new_object_id();
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    let effect = Effect::new(AdditionalStepsEffect { player: Some(ironsmith::target::PlayerFilter::You), ..steps });
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert_eq!(game.turn_store.added_steps.len(), 1);
    assert_eq!(game.turn_store.added_steps[0].placement, AddedStepPlacement::AfterStep(Step::Upkeep));
}

#[test]
fn addendum_on_a_permanent_reads_its_main_phase_cast_receipt() {
    let definition = compile_to_runtime_definition("Sentinel's Mark",
        "Type: Enchantment — Aura\nFlash\nEnchant creature\nEnchanted creature gets +1/+2 and has vigilance.\nAddendum — When this Aura enters, if you cast it during your main phase, enchanted creature gains lifelink until end of turn.", false)
        .expect("permanent addendum should compile");
    let condition = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.clone(),
        _ => None,
    }).expect("intervening main-phase cast condition");
    assert!(matches!(&condition, ironsmith::effect::Condition::ThisSpellPaidLabel(label)
        if label.kind == ironsmith::cost::OptionalCostKind::CastDuringYourMainPhase), "{condition:?}");
}

#[test]
fn first_upkeep_and_first_end_step_gates_count_steps_this_turn() {
    use ironsmith::game_state::{Phase, Step};
    let haze = compile_to_runtime_definition("Paradox Haze",
        "Type: Enchantment — Aura\nEnchant player\nAt the beginning of enchanted player's first upkeep each turn, that player gets an additional upkeep step after this step.", false)
        .expect("first-upkeep trigger should compile");
    let trigger = haze.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("upkeep trigger");
    let qualified = trigger.trigger.downcast_ref::<ironsmith::triggers::ConditionQualifiedTrigger>()
        .expect("additional upkeeps must not retrigger the ability");
    assert_eq!(qualified.condition, ironsmith::effect::Condition::FirstUpkeepStepOfTurn);
    let steps = p3_effects(&haze).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AdditionalStepsEffect>().cloned())
        .expect("that player gets an additional upkeep");
    assert!(steps.player.is_some());

    let yshtola = compile_to_runtime_definition("Y'shtola Rhul",
        "Type: Legendary Creature — Cat Druid\nAt the beginning of your end step, exile target creature you control, then return it to the battlefield under its owner's control. Then if it's the first end step of the turn, there is an additional end step after this step.", false)
        .expect("first-end-step addition should compile");
    let conditional = p3_effects(&yshtola).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().cloned())
        .expect("first end step conditional");
    assert_eq!(conditional.condition, ironsmith::effect::Condition::FirstEndStepOfTurn);
    let added = conditional.if_true.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AdditionalStepsEffect>())
        .expect("impersonal end step addition");
    assert_eq!((added.step, added.anchor, added.player.clone()),
        (ironsmith::effects::AdditionalStepKind::End, ironsmith::effects::AdditionalStepAnchor::AfterThisStep, None));

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::End);
    let source = game.new_object_id();
    let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for (started, first) in [(1, true), (2, false)] {
        game.turn_store.end_steps_started_this_turn = started;
        assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &conditional.condition, &ctx).unwrap(), first);
    }
}

#[test]
fn single_object_phase_out_can_last_until_the_source_leaves() {
    let definition = compile_to_runtime_definition("The Moment",
        "Type: Legendary Artifact\nAt the beginning of your upkeep, put a time counter on The Moment.\n{2}, {T}: Untap target creature you control. It phases out until The Moment leaves the battlefield.\n{3}, {T}: Destroy each nonland permanent with mana value less than or equal to the number of time counters on The Moment. Then sacrifice The Moment. Activate only as a sorcery.", false)
        .expect("source-linked single phase-out should compile");
    let phase = p3_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PhaseOutEffect>().cloned())
        .expect("phase out effect");
    assert_eq!(phase.duration, ironsmith::effects::PhaseOutDuration::UntilSourceLeaves);
    assert!(!matches!(phase.spec, ChooseSpec::All(_)), "only the untapped target phases out: {:?}", phase.spec);
}

#[test]
fn clash_winner_binds_the_opponent_as_the_untap_step_owner() {
    let definition = compile_to_runtime_definition("Pollen Lullaby",
        "Mana cost: {1}{W}\nType: Instant\nPrevent all combat damage that would be dealt this turn. Clash with an opponent. If you win, creatures that player controls don't untap during the player's next untap step. (Each clashing player reveals the top card of their library, then puts that card on their choice of the top or bottom. A player wins if their card had a greater mana value.)", false)
        .expect("clash opponent untap restriction should compile");
    let cant = p3_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CantEffect>().cloned())
        .expect("untap restriction");
    let ironsmith::effect::Until::PlayersNextUntapStep { player } = &cant.duration else {
        panic!("named untap step duration: {:?}", cant.duration);
    };
    assert!(matches!(player, ironsmith::target::PlayerFilter::TaggedPlayer(_)), "the clashed opponent owns the step: {player:?}");
    let ironsmith::effect::Restriction::Untap(filter) = &cant.restriction else { panic!("{:?}", cant.restriction) };
    assert_eq!(filter.controller.as_ref(), Some(player));
}

#[test]
fn untap_during_your_untap_step_qualifies_the_untap_event() {
    let definition = compile_to_runtime_definition("The Ninth Doctor",
        "Mana cost: {3}{R}{W}\nType: Legendary Creature — Time Lord Doctor\nHaste\nInto the TARDIS — Whenever The Ninth Doctor becomes untapped during your untap step, you get an additional upkeep step after this step.", false)
        .expect("untap-step qualified untap trigger should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("untap trigger");
    let qualified = trigger.trigger.downcast_ref::<ironsmith::triggers::ConditionQualifiedTrigger>()
        .expect("untapping outside the untap step must not trigger");
    assert_eq!(qualified.condition, ironsmith::effect::Condition::SourceControllersUntapStep);
    let steps = p3_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AdditionalStepsEffect>().cloned())
        .expect("additional upkeep");
    assert_eq!(steps.anchor, ironsmith::effects::AdditionalStepAnchor::AfterThisStep);
    assert_eq!(steps.player, Some(ironsmith::target::PlayerFilter::You));

    use ironsmith::game_state::{Phase, Step};
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.new_object_id();
    let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for (active, step, expected) in [(0u8, Step::Untap, true), (1, Step::Untap, false), (0, Step::Upkeep, false)] {
        game.turn.active_player = PlayerId::from_index(active);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(step);
        assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &qualified.condition, &ctx).unwrap(), expected);
    }
}

#[test]
fn skipped_untap_step_is_bound_to_the_created_extra_turn() {
    let definition = compile_to_runtime_definition("Savor the Moment",
        "Mana cost: {1}{U}{U}\nType: Sorcery\nTake an extra turn after this one. Skip the untap step of that turn.", false)
        .expect("extra turn with skipped untap should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("UntapStepOfCreatedExtraTurn"), "{debug}");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.new_object_id();
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.turn_store.extra_turns, vec![alice]);
    assert_eq!(game.turn_store.extra_turn_untap_skips, vec![0], "the skip waits for that extra turn");
    assert!(game.turn_store.skipped_steps.is_empty(), "the player's ordinary next untap step is unaffected");
}

#[test]
fn entry_counters_for_each_mana_spent_read_the_cast_receipt() {
    let definition = compile_to_runtime_definition("Verazol, the Split Current",
        "Mana cost: {X}{G}{U}\nType: Legendary Creature — Serpent\nVerazol enters with a +1/+1 counter on it for each mana spent to cast it.\nWhenever you cast a kicked spell, you may remove two +1/+1 counters from Verazol. If you do, copy that spell. You may choose new targets for the copy. (A copy of a permanent spell becomes a token.)", false)
        .expect("for-each-mana entry counters should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ManaSpentToCastThisSpell"), "{debug}");
    assert!(!debug.contains("ColorsOfManaSpentToCastThisSpell"), "total mana, not colors: {debug}");
}

#[test]
fn uncalled_coin_face_branches_counter_or_copy_the_same_target() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Odds",
        "Mana cost: {U}{R}\nType: Instant\nFlip a coin. If it comes up heads, counter target instant or sorcery spell. If it comes up tails, copy that spell and you may choose new targets for the copy.", false)
        .expect("heads/tails coin branches should compile");
    let debug = format!("{definition:#?}");
    for needle in ["FaceOnly", "CoinHeads", "CoinTails", "CounterEffect", "CopySpellEffect"] {
        assert!(debug.contains(needle), "missing {needle}: {debug}");
    }
    assert!(!debug.contains("CoinFlipsWon"), "an uncalled flip has no winner: {debug}");
    fn visit(effect: &ironsmith::effect::Effect, declarations: &mut Vec<ironsmith::tag::TagKey>, copies: &mut Vec<ChooseSpec>) {
        if let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
            let mut inner = &*tagged.effect;
            while let Some(with_id) = inner.downcast_ref::<ironsmith::effects::WithIdEffect>() { inner = &with_id.effect; }
            if inner.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() {
                declarations.push(tagged.tag.clone());
            }
        }
        if let Some(copy) = effect.downcast_ref::<ironsmith::effects::CopySpellEffect>() { copies.push(copy.target.clone()); }
        effect.visit_child_effects(&mut |child| visit(child, declarations, copies));
    }
    let mut declarations = Vec::new();
    let mut copies = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { visit(effect, &mut declarations, &mut copies); }
    assert_eq!(declarations.len(), 1, "one captured target before the flip");
    assert_eq!(copies.len(), 1);
    assert!(matches!(copies[0].base(), ChooseSpec::Tagged(tag) if tag == &declarations[0]), "copy must read the same declared target: {copies:?}");
    let mut heads = 0;
    let mut tails = 0;
    for seed in 0..32 {
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.set_random_seed(seed);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let original = CardBuilder::new(CardId::new(), "Original spell").card_types(vec![CardType::Instant]).build();
        let target = game.create_object_from_card(&original, bob, Zone::Stack);
        let stable = game.object(target).unwrap().stable_id;
        game.stack.push(ironsmith::game_state::StackEntry::new(target, bob));
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Object(target)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        let current = game.find_object_by_stable_id(stable).unwrap();
        if game.object(current).unwrap().zone == Zone::Graveyard {
            heads += 1;
            assert!(game.stack.is_empty(), "heads counters the original without making a copy");
        } else {
            tails += 1;
            assert_eq!(current, target, "the original stack incarnation remains on tails");
            assert_eq!(game.object(current).unwrap().zone, Zone::Stack);
            assert_eq!(game.stack.len(), 2, "tails copies the declared original even though the counter branch never ran");
            let copied = game.object(game.stack.last().unwrap().object_id).unwrap();
            assert_ne!(copied.id, target);
            assert_eq!(copied.name, "Original spell");
            assert_eq!(game.controller_of(copied), alice);
        }
    }
    assert!(heads > 0 && tails > 0, "both faces exercised: {heads} heads, {tails} tails");
}

#[test]
fn first_noncast_from_hand_spell_keyword_grant_has_one_typed_owner() {
    let definition = compile_to_runtime_definition("The Twelfth Doctor",
        "Mana cost: {2}{U}{R}\nType: Legendary Creature — Time Lord Doctor\nThe first spell you cast from anywhere other than your hand each turn has demonstrate. (When you cast that spell, you may copy it. If you do, choose an opponent to also copy it. A copy of a permanent spell becomes a token.)\nWhenever you copy a spell, put a +1/+1 counter on The Twelfth Doctor.", false)
        .expect("spell keyword grant should compile");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Demonstrate"), "{debug}");
    assert!(debug.contains("first_spell_cast_each_turn: true"), "only the first such spell each turn: {debug}");
    assert!(debug.contains("excluded_cast_origin_zone: Some("), "cast from anywhere other than the hand: {debug}");
}

#[test]
fn oath_allows_a_second_loyalty_activation_each_turn() {
    let definition = compile_to_runtime_definition("Oath of Teferi",
        "Mana cost: {3}{W}{U}\nType: Legendary Enchantment\nWhen Oath of Teferi enters, exile another target permanent you control. Return it to the battlefield under its owner's control at the beginning of the next end step.\nYou may activate the loyalty abilities of planeswalkers you control twice each turn rather than only once.", false)
        .expect("loyalty twice static should compile");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::LoyaltyAbilitiesTwiceEachTurn)));
}

#[test]
fn started_the_turn_untapped_reads_the_turn_start_receipt() {
    let definition = compile_to_runtime_definition("Rasputin Dreamweaver",
        "Mana cost: {4}{W}{U}\nType: Legendary Creature — Human Wizard\nRasputin enters with seven dream counters on it.\nRemove a dream counter from Rasputin: Add {C}.\nRemove a dream counter from Rasputin: Prevent the next 1 damage that would be dealt to Rasputin this turn.\nAt the beginning of your upkeep, if Rasputin started the turn untapped, put a dream counter on it.\nRasputin can't have more than seven dream counters on it.", false)
        .expect("started-the-turn-untapped condition should compile");
    let condition = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.clone(),
        _ => None,
    }).expect("upkeep intervening condition");
    assert_eq!(condition, ironsmith::effect::Condition::SourceStartedTurnUntapped);
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Dreamer").card_types(vec![CardType::Creature]).build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, &condition, &ctx).unwrap());
    game.turn_store.untapped_at_turn_start.insert(source);
    assert!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &condition, &ctx).unwrap());
}

#[test]
fn snow_symbol_count_reads_mana_from_snow_sources_spent_to_cast() {
    let definition = compile_to_runtime_definition("Tundra Fumarole",
        "Mana cost: {1}{R}\nType: Snow Sorcery\nTundra Fumarole deals 4 damage to target creature or planeswalker. Add {C} for each {S} spent to cast this spell. Until end of turn, you don't lose this mana as steps and phases end. ({S} is mana from a snow source.)", false)
        .expect("snow-spent mana count should compile");
    let add = p3_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AddScaledManaEffect>().cloned())
        .expect("scaled colorless mana");
    let ironsmith::effect::Value::ManaFromSourceSpentToCastThisSpell { source_filter, .. } = add.amount.unhinted() else {
        panic!("{:?}", add.amount);
    };
    assert_eq!(source_filter.supertypes, vec![ironsmith::types::Supertype::Snow]);
}

#[test]
fn prevented_damage_to_you_triggers_with_the_prevented_amount() {
    let definition = compile_to_runtime_definition("Selfless Squire",
        "Mana cost: {3}{W}\nType: Creature — Human Soldier\nFlash\nWhen this creature enters, prevent all damage that would be dealt to you this turn.\nWhenever damage that would be dealt to you is prevented, put that many +1/+1 counters on this creature.", false)
        .expect("prevented-damage trigger should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger)
            if trigger.trigger.downcast_ref::<ironsmith::triggers::DamageToPlayerPreventedTrigger>().is_some() => Some(trigger),
        _ => None,
    }).expect("damage-prevented trigger");
    let matcher = trigger.trigger.downcast_ref::<ironsmith::triggers::DamageToPlayerPreventedTrigger>().unwrap();
    assert_eq!(matcher.player, ironsmith::target::PlayerFilter::You);
    let debug = format!("{:#?}", trigger.effects);
    assert!(debug.contains("EventValue"), "that many = prevented amount: {debug}");
}

#[test]
fn that_many_after_had_counters_reads_the_triggering_counter_count() {
    let definition = compile_to_runtime_definition("Oft-Nabbed Goat",
        "Mana cost: {B}\nType: Creature — Goat\n{1}: Draw a card. Gain control of this creature and put a -1/-1 counter on it. Only your opponents may activate this ability and only as a sorcery.\nWhen this creature dies, if it had one or more -1/-1 counters on it, its owner draws that many cards and each other player loses that much life.", false)
        .expect("counter-count antecedent should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("dies trigger");
    let debug = format!("{:#?}", trigger.effects);
    assert!(debug.contains("CountersOn"), "that many = -1/-1 counters it had: {debug}");
    assert!(debug.contains("MinusOneMinusOne"), "{debug}");
    // "that much life" names the same counter count, not the cards drawn.
    let life = p3_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::LoseLifeEffect>().cloned())
        .expect("life loss");
    assert!(matches!(life.amount.unhinted(), ironsmith::effect::Value::CountersOn(..)), "{:?}", life.amount);
}

#[test]
fn port_razer_cannot_attack_a_player_it_already_attacked_this_turn() {
    let definition = compile_to_runtime_definition("Port Razer",
        "Mana cost: {3}{R}{R}\nType: Creature — Orc Pirate\nWhenever this creature deals combat damage to a player, untap each creature you control. After this phase, there is an additional combat phase.\nThis creature can't attack a player it has already attacked this turn.", false)
        .expect("attack-history restriction should compile");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::CantAttackPlayerAlreadyAttackedThisTurn)));
}

#[test]
fn ogre_enforcer_needs_lethal_damage_from_a_single_source() {
    let definition = compile_to_runtime_definition("Ogre Enforcer",
        "Mana cost: {3}{R}{R}\nType: Creature — Ogre\nThis creature can't be destroyed by lethal damage unless lethal damage dealt by a single source is marked on it.", false)
        .expect("single-source lethal damage static should compile");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::LethalDamageMustBeFromSingleSource)));
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let ogre = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let card = CardBuilder::new(CardId::new(), "Pinger").card_types(vec![CardType::Creature]).build();
    let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.set_damage_marked(ogre, 4);
    game.mark_damage_from_source(ogre, first, 2);
    game.mark_damage_from_source(ogre, second, 2);
    assert_eq!(game.max_damage_marked_from_single_source(ogre), 2);
    game.set_damage_marked(ogre, 1);
    assert_eq!(game.max_damage_marked_from_single_source(ogre), 1, "removed damage caps each source");
}

#[test]
fn exactly_one_blocker_requirement_is_must_be_blocked_plus_one_blocker_cap() {
    let definition = compile_to_runtime_definition("Nacatl War-Pride",
        "Mana cost: {3}{G}{G}{G}\nType: Creature — Cat Warrior\nThis creature must be blocked by exactly one creature if able.\nWhenever this creature attacks, create X tokens that are copies of it and that are tapped and attacking, where X is the number of creatures defending player controls. Exile the tokens at the beginning of the next end step.", false)
        .expect("exactly-one blocker requirement should compile");
    let ids = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(static_ability) => Some(static_ability.id()),
        _ => None,
    }).collect::<Vec<_>>();
    assert!(ids.contains(&ironsmith::static_abilities::StaticAbilityId::CantBeBlockedByMoreThan), "{ids:?}");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("MustBeBlocked"), "{debug}");
}

#[test]
fn oubliette_holds_the_creature_and_taps_it_as_it_phases_in() {
    let definition = compile_to_runtime_definition("Oubliette",
        "Mana cost: {1}{B}{B}\nType: Enchantment\nWhen this enchantment enters, target creature phases out until this enchantment leaves the battlefield. Tap that creature as it phases in this way. (Auras and Equipment phase out with it. While permanents are phased out, they're treated as though they don't exist.)", false)
        .expect("tapped phase-in rider should compile");
    let effects = p3_effects(&definition);
    let phase = effects.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PhaseOutEffect>().cloned())
        .expect("phase out");
    assert_eq!(phase.duration, ironsmith::effects::PhaseOutDuration::UntilSourceLeavesThenTapped);
    assert!(effects.iter().all(|effect| effect.downcast_ref::<ironsmith::effects::TapEffect>().is_none()),
        "the creature is not tapped now");
}

#[test]
fn prevention_follow_ups_reuse_the_proposed_amount() {
    for (name, text) in [
        ("Panther Habit", "Mana cost: {1}\nType: Artifact — Equipment\nIf equipped creature would be dealt damage, prevent that damage and put that many +1/+1 counters on it.\nEquip {2}"),
        ("Sekki, Seasons' Guide", "Mana cost: {5}{G}{G}{G}\nType: Legendary Creature — Spirit\nSekki enters with eight +1/+1 counters on it.\nIf damage would be dealt to Sekki, prevent that damage, remove that many +1/+1 counters from Sekki, and create that many 1/1 colorless Spirit creature tokens.\nSacrifice eight Spirits: Return this card from your graveyard to the battlefield."),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let debug = format!("{definition:#?}");
        assert!(debug.contains("PreventMatchingDamageWithFollowUp") || debug.contains("PreventDamageThenFromProposedAmount") || debug.contains("Proposed"), "{name}: {debug}");
        assert!(debug.contains("EventValue"), "{name}: follow-up reads that many: {debug}");
    }
}

#[test]
fn targeted_prevention_can_choose_its_source_on_resolution() {
    let definition = compile_to_runtime_definition("Shieldmage Advocate",
        "Mana cost: {2}{W}\nType: Creature — Human Cleric\n{T}: Return target card from an opponent's graveyard to their hand. Prevent all damage that would be dealt to any target this turn by a source of your choice.", false)
        .expect("targeted source-choice prevention should compile");
    let prevent = p3_effects(&definition).into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PreventAllDamageToTargetEffect>().cloned())
        .expect("targeted prevention");
    assert!(prevent.source_of_your_choice);
    assert!(matches!(prevent.target.base(), ChooseSpec::AnyTarget) || prevent.target.is_target(), "{:?}", prevent.target);
}

#[test]
fn pandorica_untaps_then_holds_the_target_while_it_stays_tapped() {
    let definition = compile_to_runtime_definition("The Pandorica",
        "Mana cost: {5}\nType: Legendary Artifact\nYou may choose not to untap The Pandorica during your untap step.\n{1}{W}, {T}: Untap another target nonland permanent, then it phases out. It can't phase in for as long as The Pandorica remains tapped. When The Pandorica becomes untapped or leaves the battlefield, that permanent phases in. Activate only as a sorcery.", false)
        .expect("tapped-source phasing hold should compile");
    let effects = p3_effects(&definition);
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::UntapEffect>().is_some()), "the target is untapped first");
    let phase = effects.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::PhaseOutEffect>().cloned())
        .expect("phase out");
    assert_eq!(phase.duration, ironsmith::effects::PhaseOutDuration::UntilSourceUntapsOrLeaves);
}

#[test]
fn world_at_war_inserts_after_the_second_main_phase() {
    let definition = compile_to_runtime_definition("World at War",
        "Mana cost: {3}{R}{R}\nType: Sorcery\nAfter the second main phase this turn, there's an additional combat phase followed by an additional main phase. At the beginning of that combat, untap all creatures that attacked this turn.\nRebound (If you cast this spell from your hand, exile it as it resolves. At the beginning of your next upkeep, you may cast this card from exile without paying its mana cost.)", false)
        .expect("deferred additional phases should compile");
    let effects = p3_effects(&definition);
    let conditional = effects.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().cloned())
        .expect("second-main-phase branch");
    assert_eq!(conditional.condition, ironsmith::effect::Condition::SecondMainPhaseOfTurn);
    let debug = format!("{definition:#?}");
    assert!(debug.contains("BeginningOfMainPhaseTrigger") && debug.contains("Postcombat"), "{debug}");
    assert!(debug.contains("BeginningOfCombat"), "that combat's untap rider: {debug}");
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::AdditionalPhasesEffect>()
        .is_some_and(|phases| phases.phases == vec![ironsmith::effects::AdditionalPhase::Combat, ironsmith::effects::AdditionalPhase::Main])));
}

#[test]
fn plural_named_self_reference_reads_they_and_them_as_the_source() {
    let definition = compile_to_runtime_definition("The Notary Hobbits",
        "Mana cost: {3}{W}{U}\nType: Legendary Creature — Halfling Advisor\nWhen The Notary Hobbits enter, if they're not a token, create two tokens that are copies of them, except the tokens aren't legendary.\n{T}: Add {C} for each Halfling you control.", false)
        .expect("plural self references should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).expect("enters trigger");
    assert!(trigger.intervening_if.is_some(), "if they're not a token");
    let debug = format!("{:#?}", trigger.effects);
    assert!(debug.contains("Legendary"), "the copies drop legendary: {debug}");
}

#[test]
fn vindicator_prevents_and_reflects_through_its_own_prevention_trigger() {
    let definition = compile_to_runtime_definition("Phyrexian Vindicator",
        "Mana cost: {3}{W}{W}\nType: Creature — Phyrexian Horror\nFlying\nIf damage would be dealt to this creature, prevent that damage. When damage is prevented this way, this creature deals that much damage to any other target.", false)
        .expect("static prevention with reflexive trigger should compile");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(static_ability)
            if static_ability.id() == ironsmith::static_abilities::StaticAbilityId::PreventAllDamageToSelf)));
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger)
            if trigger.trigger.downcast_ref::<ironsmith::triggers::DamagePreventedBySourceTrigger>().is_some() => Some(trigger),
        _ => None,
    }).expect("prevented-this-way trigger");
    assert!(!trigger.choices.is_empty(), "any other target is chosen as the trigger goes on the stack");
    assert!(format!("{:#?}", trigger.effects).contains("EventValue"), "that much = prevented damage");
}
