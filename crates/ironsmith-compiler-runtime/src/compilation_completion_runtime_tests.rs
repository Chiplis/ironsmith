//! Focused runtime checks for cards in the compilation-completion task.
use super::compile_to_runtime_definition;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::effect::Effect;
use ironsmith::target::ChooseSpec;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

#[test]
fn equipment_filtered_block_requirement_survives_recipient_ability_loss() {
    for (name, subtype, text) in [
        ("Ace's Baseball Bat", Subtype::Dalek, "Type: Legendary Artifact — Equipment\nEquipped creature gets +3/+0.\nAs long as equipped creature is attacking, it has first strike and must be blocked by a Dalek if able.\nEquip legendary creature {1}\nEquip {3}"),
        ("Slayer's Cleaver", Subtype::Eldrazi, "Type: Artifact — Equipment\nEquipped creature gets +3/+1 and must be blocked by an Eldrazi if able.\nEquip {4}"),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        let creature = CardBuilder::new(CardId::new(), "Equipped attacker")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
        let matching = CardBuilder::new(CardId::new(), "Eligible blocker")
            .card_types(vec![CardType::Creature]).subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let blocker = game.create_object_from_card(&matching, bob, Zone::Battlefield);
        let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(equipment, ironsmith::object::AttachmentTarget::Object(attacker)));
        game.remove_summoning_sickness(attacker);
        game.refresh_continuous_state().unwrap();
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::combat_state::declare_attackers(&mut game, &mut combat,
            vec![(attacker, ironsmith::combat_state::AttackTarget::Player(bob))]).unwrap();
        let removal = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Object(ironsmith::target::ObjectFilter::specific(attacker)),
            ironsmith::continuous::Modification::RemoveAllAbilities,
            ironsmith::effect::Until::EndOfTurn,
        ));
        let mut ctx = ironsmith::effects::EffectContext::new_default(attacker, alice);
        ironsmith::effects::execute_effect(&mut game, &removal, &mut ctx).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![]).is_err(), "{name}");
        ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![(blocker, attacker)]).unwrap();
        assert!(game.detach_object_from_current_target(equipment));
        game.refresh_continuous_state().unwrap();
        ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![]).unwrap();
    }
}

#[test]
fn entry_life_payment_and_token_stats_share_numeric_acquisition() {
    use ironsmith::effect::Value;
    let definition = compile_to_runtime_definition("Phyrexian Processor",
        "Type: Artifact\nAs this artifact enters, pay any amount of life.\n{4}, {T}: Create an X/X black Phyrexian Minion creature token, where X is the life paid as this artifact entered.", false)
        .expect("entry life payment and later token reader should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("token activation");
    let pair = activated.effects.source_number_pair.expect("linked numeric reader");
    let create = activated.effects.all_effects().into_iter().find_map(|effect|
        effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>()).expect("create token");
    let (power, toughness) = create.dynamic_power_toughness.as_ref().expect("dynamic token stats");
    for value in [power, toughness] {
        assert!(matches!(value.unhinted(), Value::SourceChosenNumber { if_unset: Some(0), pair: Some(binding) } if *binding == pair));
    }
    let debug = format!("{definition:#?}");
    assert!(debug.contains("source_owned: true"), "payment must retain its completed result: {debug}");
}

#[test]
fn repeated_returns_announce_one_player_and_preserve_four_choices() {
    let definition = compile_to_runtime_definition("Linessa, Zephyr Mage",
        "Type: Creature\n{T}: Target player returns a creature they control to its owner's hand, then repeats this process for an artifact, an enchantment, and a land.", false)
        .expect("repeated return should compile");
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
    }).unwrap();
    fn inspect(effect: &Effect, targets: &mut usize, choices: &mut Vec<ironsmith::effects::ChooseObjectsEffect>) {
        if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { *targets += 1; }
        if let Some(choice) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() { choices.push(choice.clone()); }
        effect.visit_child_effects(&mut |child| inspect(child, targets, choices));
    }
    let mut targets = 0;
    let mut choices = Vec::new();
    for effect in ability.effects.all_effects() { inspect(effect, &mut targets, &mut choices); }
    assert_eq!(targets, 1);
    assert_eq!(ability.choices.len(), 1);
    assert_eq!(choices.len(), 4);
    for (choice, kind) in choices.iter().zip([CardType::Creature, CardType::Artifact, CardType::Enchantment, CardType::Land]) {
        assert_eq!(choice.filter.card_types, vec![kind]);
        assert_eq!(choice.count.min, 1);
        assert_eq!(choice.count.max, Some(1));
        assert!(matches!(choice.chooser, ironsmith::target::PlayerFilter::Target(_) | ironsmith::target::PlayerFilter::AliasedTarget(_)));
        assert!(matches!(&choice.filter.controller,
            Some(ironsmith::target::PlayerFilter::Target(player) | ironsmith::target::PlayerFilter::AliasedTarget(player))
                if **player == ironsmith::target::PlayerFilter::Any));
    }
}

#[test]
fn auratouched_mage_moves_the_searched_aura_and_attaches_to_the_source() {
    let definition = compile_to_runtime_definition("Auratouched Mage",
        "Type: Creature — Human Wizard\nWhen this creature enters, search your library for an Aura card that could enchant it. If this creature is still on the battlefield, put that Aura card onto the battlefield attached to it. Otherwise, reveal the Aura card and put it into your hand. Then shuffle.", false).unwrap();
    fn inspect(effect: &Effect, moved: &mut bool, attached: &mut bool) {
        if let Some(movement) = effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
            && movement.zone == Zone::Battlefield
        {
            match movement.target.base() {
                ChooseSpec::Object(filter) => {
                    assert_eq!(filter.zone, None, "the searched card is in the library");
                    assert!(filter.tagged_constraints.iter().any(|tag| tag.tag.as_str() == "searched"));
                }
                ChooseSpec::Tagged(tag) => assert_eq!(tag.as_str(), "searched"),
                other => panic!("unexpected Aura origin: {other:?}"),
            }
            *moved = true;
        }
        if let Some(attachment) = effect.downcast_ref::<ironsmith::effects::AttachObjectsEffect>() {
            assert!(matches!(attachment.target.base(), ChooseSpec::Source), "host must be the entering creature: {:?}", attachment.target);
            *attached = true;
        }
        effect.visit_child_effects(&mut |child| inspect(child, moved, attached));
    }
    let mut moved = false;
    let mut attached = false;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() { inspect(effect, &mut moved, &mut attached); }
        }
    }
    assert!(moved && attached);
}

#[test]
fn targeted_reveal_checks_the_underlying_card_before_offering_face_up() {
    use ironsmith::snapshot::ObjectSnapshot;
    #[derive(Debug, Default)]
    struct Accept { offers: usize }
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.offers += 1;
            true
        }
    }
    let definition = compile_to_runtime_definition("Hauntwoods Shrieker",
        "Type: Creature — Beast Mutant\nWhenever this creature attacks, manifest dread.\n{1}{G}: Reveal target face-down permanent. If it's a creature card, you may turn it face up.", false).unwrap();
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
    }).unwrap();
    fn inspect(effect: &Effect, targets: &mut usize, reveal: &mut Option<Effect>, condition: &mut Option<Effect>) {
        if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { *targets += 1; }
        if effect.downcast_ref::<ironsmith::effects::RevealTaggedEffect>().is_some() { *reveal = Some(effect.clone()); }
        if effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some() { *condition = Some(effect.clone()); }
        effect.visit_child_effects(&mut |child| inspect(child, targets, reveal, condition));
    }
    let (mut targets, mut reveal, mut condition) = (0, None, None);
    for effect in ability.effects.all_effects() { inspect(effect, &mut targets, &mut reveal, &mut condition); }
    assert_eq!(targets, 1);
    assert_eq!(ability.choices.len(), 1);
    let reveal = reveal.unwrap();
    let tag = reveal.downcast_ref::<ironsmith::effects::RevealTaggedEffect>().unwrap().tag.clone();
    let condition = condition.unwrap();
    let alice = PlayerId::from_index(0);
    for (kind, token) in [(CardType::Creature, false), (CardType::Land, false), (CardType::Creature, true)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Revealed identity").card_types(vec![kind]).build();
        let target = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if token { game.object_mut(target).unwrap().kind = ironsmith::object::ObjectKind::Token; }
        assert!(game.set_face_down(target));
        game.refresh_continuous_state().unwrap();
        assert!(game.current_card_types(target).unwrap().contains(&CardType::Creature));
        let mut dm = Accept::default();
        let mut ctx = ironsmith::effects::EffectContext::new(target, alice, &mut dm);
        ctx.set_tagged_objects(tag.clone(), vec![ObjectSnapshot::from_object_id(&game, target).unwrap()]);
        ironsmith::effects::execute_effect(&mut game, &reveal, &mut ctx).unwrap();
        assert!(game.is_face_down(target), "revealing alone must preserve the overlay");
        ironsmith::effects::execute_effect(&mut game, &condition, &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(dm.offers, usize::from(kind == CardType::Creature && !token));
        assert_eq!(game.is_face_down(target), kind != CardType::Creature || token);
    }
}

#[test]
fn renowned_conditions_follow_the_target_or_triggering_creature() {
    use ironsmith::snapshot::ObjectSnapshot;
    let mist = compile_to_runtime_definition("Enshrouding Mist",
        "Type: Instant\nTarget creature gets +1/+1 until end of turn. Prevent all damage that would be dealt to it this turn. If it's renowned, untap it.", false).unwrap();
    fn conditional(effect: &Effect, found: &mut Option<Effect>) {
        if effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().is_some() { *found = Some(effect.clone()); }
        effect.visit_child_effects(&mut |child| conditional(child, found));
    }
    let mut found = None;
    for effect in mist.spell_effect.as_ref().unwrap().all_effects() { conditional(effect, &mut found); }
    let effect = found.unwrap();
    let condition = &effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().unwrap().condition;
    let ironsmith::effect::Condition::TaggedObjectMatches(tag, filter) = condition else { panic!("{condition:?}"); };
    assert!(filter.renowned);
    assert_eq!(tag.as_str(), "pumped_0");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Renown witness").card_types(vec![CardType::Creature]).build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let target = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.tap(target);
    game.set_renowned(source);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects(tag.clone(), vec![ObjectSnapshot::from_object_id(&game, target).unwrap()]);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert!(game.is_tapped(target), "the renowned spell source cannot substitute for its target");
    game.set_renowned(target);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert!(!game.is_tapped(target));

    let consul = compile_to_runtime_definition("Consul's Lieutenant",
        "Type: Creature — Human Soldier\nFirst strike\nRenown 1\nWhenever this creature attacks, if it's renowned, other attacking creatures you control get +1/+1 until end of turn.", false).unwrap();
    let condition = consul.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.as_ref()
            .filter(|condition| matches!(condition, ironsmith::effect::Condition::TaggedObjectMatches(_, filter) if filter.renowned)), _ => None,
    }).unwrap();
    let fresh = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(fresh, alice);
    ctx.set_tagged_objects("triggering", vec![ObjectSnapshot::from_object_id(&game, fresh).unwrap()]);
    assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
    game.set_renowned(fresh);
    assert!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
}

#[test]
fn exile_counter_upkeep_requires_the_source_zone_and_its_remaining_counter() {
    let definition = compile_to_runtime_definition("Darigaaz Reincarnated",
        "Type: Legendary Creature — Dragon\nFlying, trample, haste\nIf Darigaaz would die, instead exile it with three egg counters on it.\nAt the beginning of your upkeep, if this card is exiled with an egg counter on it, remove an egg counter from it. Then if this card has no egg counters on it, return it to the battlefield.", false).unwrap();
    let (condition, program, zones) = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.as_ref()
            .map(|condition| (condition, &trigger.effects, &ability.functional_zones)), _ => None,
    }).unwrap();
    assert!(zones.contains(&Zone::Exile));
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Exile, Zone::Battlefield] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, zone);
        let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
        game.add_counters(source, ironsmith::CounterType::Egg, 1);
        assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap(), zone == Zone::Exile);
        if zone == Zone::Exile {
            game.add_counters(source, ironsmith::CounterType::Egg, 2);
            let stable = game.object(source).unwrap().stable_id;
            let mut ctx = ctx;
            for remaining in [2, 1, 0] {
                for effect in program.all_effects() {
                    ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                let current = game.find_object_by_stable_id(stable).unwrap();
                let object = game.object(current).unwrap();
                assert_eq!(object.zone, if remaining == 0 { Zone::Battlefield } else { Zone::Exile });
                assert_eq!(object.counters.get(&ironsmith::CounterType::Egg).copied().unwrap_or(0), remaining);
            }
        }
    }
}

#[test]
fn hand_or_command_choice_keeps_owner_and_lesser_mana_value_constraints() {
    use ironsmith::snapshot::ObjectSnapshot;
    #[derive(Debug)]
    struct SelectCommand { preferred: ironsmith::ObjectId, legal: Vec<ironsmith::ObjectId> }
    impl ironsmith::decision::DecisionMaker for SelectCommand {
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.legal = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
            assert!(self.legal.contains(&self.preferred));
            vec![self.preferred]
        }
    }
    let definition = compile_to_runtime_definition("Next of Kin",
        "Type: Enchantment — Aura\nEnchant creature\nWhen enchanted creature dies, you may put a creature card you own with lesser mana value from your hand or from the command zone onto the battlefield. If you do, return this card to the battlefield attached to that creature at the beginning of the next end step.", false).unwrap();
    fn find(effect: &Effect, choice: &mut Option<ironsmith::effects::ChooseObjectsEffect>) {
        if let Some(found) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() { *choice = Some(found.clone()); }
        effect.visit_child_effects(&mut |child| find(child, choice));
    }
    let mut choice = None;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
            for effect in trigger.effects.all_effects() { find(effect, &mut choice); }
        }
    }
    let choice = choice.unwrap();
    assert_eq!(choice.zone, Some(Zone::Hand));
    assert_eq!(choice.additional_zones, vec![Zone::Command]);
    assert_eq!(choice.filter.zone, None);
    assert_eq!(choice.filter.owner, Some(ironsmith::target::PlayerFilter::You));
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = |value| CardBuilder::new(CardId::new(), "Choice witness")
        .card_types(vec![CardType::Creature])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(value)])).build();
    let dead = game.create_object_from_card(&creature(4), alice, Zone::Graveyard);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand = game.create_object_from_card(&creature(2), alice, Zone::Hand);
    let command = game.create_object_from_card(&creature(3), alice, Zone::Command);
    let foreign = game.create_object_from_card(&creature(1), bob, Zone::Command);
    let expensive = game.create_object_from_card(&creature(4), alice, Zone::Command);
    let battlefield = game.create_object_from_card(&creature(1), alice, Zone::Battlefield);
    let mut dm = SelectCommand { preferred: command, legal: Vec::new() };
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    ctx.set_tagged_objects("triggering", vec![ObjectSnapshot::from_object_id(&game, dead).unwrap()]);
    let tag = choice.tag.clone();
    ironsmith::effects::execute_effect(&mut game, &Effect::new(choice), &mut ctx).unwrap();
    assert_eq!(ctx.get_tagged_all(&tag).unwrap()[0].object_id, command);
    drop(ctx);
    assert!(dm.legal.contains(&hand));
    for forbidden in [foreign, expensive, battlefield] { assert!(!dm.legal.contains(&forbidden)); }
}

#[test]
fn source_zone_alternatives_allow_the_graveyard_and_battlefield_upkeep() {
    let definition = compile_to_runtime_definition("Firemane Angel",
        "Type: Creature — Angel\nFlying, first strike\nAt the beginning of your upkeep, if Firemane Angel is in your graveyard or on the battlefield, you may gain 1 life.\n{6}{R}{R}{W}{W}: Return this card from your graveyard to the battlefield. Activate only during your upkeep.", false).unwrap();
    let (condition, zones) = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => trigger.intervening_if.as_ref().map(|condition| (condition, &ability.functional_zones)), _ => None,
    }).unwrap();
    assert!(zones.contains(&Zone::Graveyard));
    assert!(zones.contains(&Zone::Battlefield));
    assert_eq!(zones.len(), 2);
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Graveyard, Zone::Battlefield, Zone::Hand, Zone::Exile] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, zone);
        let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap(), matches!(zone, Zone::Graveyard | Zone::Battlefield));
    }
}

#[test]
fn commander_mana_value_choice_drives_both_draw_and_life_loss() {
    #[derive(Debug)]
    struct Select { preferred: Option<ironsmith::ObjectId>, forbidden: ironsmith::ObjectId }
    impl ironsmith::decision::DecisionMaker for Select {
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            assert!(!ctx.candidates.iter().any(|candidate| candidate.id == self.forbidden && candidate.legal));
            self.preferred.filter(|id| ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal)).into_iter().collect()
        }
    }
    let definition = compile_to_runtime_definition("Stinging Study",
        "Type: Instant\nYou draw X cards and you lose X life, where X is the mana value of a commander you own on the battlefield or in the command zone.", false).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for chosen_zone in [None, Some(Zone::Battlefield), Some(Zone::Command)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let commander_card = CardBuilder::new(CardId::new(), "Commander choice witness")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(3)])).build();
        let foreign = game.create_object_from_card(&commander_card, bob, Zone::Command);
        game.set_as_commander(foreign, bob);
        let preferred = chosen_zone.map(|zone| {
            let commander = game.create_object_from_card(&commander_card, alice, zone);
            game.set_as_commander(commander, alice);
            commander
        });
        let library_card = CardBuilder::new(CardId::new(), "Draw witness").card_types(vec![CardType::Land]).build();
        for _ in 0..4 { game.create_object_from_card(&library_card, alice, Zone::Library); }
        let mut dm = Select { preferred, forbidden: foreign };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let amount = if chosen_zone.is_some() { 3 } else { 0 };
        assert_eq!(game.player(alice).unwrap().life, 20 - amount);
        assert_eq!(game.player(alice).unwrap().hand.len(), amount as usize);
    }
}
