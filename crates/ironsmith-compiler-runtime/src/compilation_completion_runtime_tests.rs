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

#[test]
fn shared_discard_types_compare_only_this_resolution_and_reveal_after_choices() {
    #[derive(Debug, Default)]
    struct DiscardChoices { chosen: usize, selected: Vec<ironsmith::ObjectId>, public_views: Vec<usize> }
    impl ironsmith::decision::DecisionMaker for DiscardChoices {
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            assert!(self.selected.iter().all(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Hand)),
                "all players must finish choosing before any selected card moves");
            let picked: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal).take(1).map(|candidate| candidate.id).collect();
            self.chosen += usize::from(!picked.is_empty());
            self.selected.extend(picked.iter().copied());
            picked
        }
        fn view_cards(&mut self, _: &ironsmith::GameState, _: PlayerId, _: &[ironsmith::ObjectId], ctx: &ironsmith::decisions::context::ViewCardsContext) {
            if ctx.public { self.public_views.push(self.chosen); }
        }
    }
    let definition = compile_to_runtime_definition("Creeping Dread",
        "Type: Enchantment\nAt the beginning of your upkeep, each player discards a card. Each opponent who discarded a card that shares a card type with the card you discarded loses 3 life. (Players reveal the discarded cards simultaneously.)", false).unwrap();
    let program = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(&trigger.effects), _ => None,
    }).unwrap();
    let cases = [
        (Some(vec![CardType::Creature]), Some(vec![CardType::Artifact, CardType::Creature]), Some(vec![CardType::Kindred, CardType::Instant]), [17, 20]),
        (None, Some(vec![CardType::Creature]), Some(vec![CardType::Creature]), [20, 20]),
        (Some(vec![CardType::Land]), Some(vec![CardType::Creature]), Some(vec![CardType::Instant]), [20, 20]),
        (Some(vec![CardType::Artifact, CardType::Creature]), Some(vec![CardType::Artifact, CardType::Creature]), Some(vec![CardType::Land]), [17, 20]),
        (Some(vec![CardType::Instant]), Some(vec![CardType::Instant]), Some(vec![CardType::Sorcery]), [17, 20]),
        (None, None, None, [20, 20]),
    ];
    for (your_types, bob_types, carol_types, expected) in cases {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = |types| CardBuilder::new(CardId::new(), "Discard witness")
            .card_types(types).subtypes(vec![Subtype::Elf])
            .supertypes(vec![ironsmith::types::Supertype::Legendary]).build();
        game.create_object_from_card(&card(vec![CardType::Creature]), alice, Zone::Graveyard);
        let hands = [your_types, bob_types, carol_types];
        let chosen_count = hands.iter().filter(|hand| hand.is_some()).count();
        for (index, types) in hands.into_iter().enumerate() {
            if let Some(types) = types { game.create_object_from_card(&card(types), PlayerId::from_index(index as u8), Zone::Hand); }
        }
        let mut dm = DiscardChoices::default();
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in program.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, expected[0]);
        assert_eq!(game.player(PlayerId::from_index(2)).unwrap().life, expected[1]);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(PlayerId::from_index(3)).unwrap().life, 20);
        for index in 0..4 { assert!(game.player(PlayerId::from_index(index)).unwrap().hand.is_empty()); }
        assert!(dm.public_views.iter().all(|choices| *choices == chosen_count), "discard openings must wait for every selection: {:?}", dm.public_views);
    }
}

#[test]
fn crafted_return_chooses_only_shared_exiled_material_types_before_entry() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::snapshot::ObjectSnapshot;
    let mut front = compile_to_runtime_definition("Eye of Ojer Taq", "Mana cost: {3}\nType: Artifact\n{T}: Add one mana of any color.\nCraft with two that share a card type {6}", false).unwrap();
    let mut back = compile_to_runtime_definition("Apex Observatory", "Type: Artifact\nThis artifact enters tapped. As it enters, choose a card type shared among two exiled cards used to craft it.\n{T}: The next spell you cast this turn of the chosen type can be cast without paying its mana cost.", false).unwrap();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    front.card.transforming_dfc = true;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.transforming_dfc = true;
    let craft = front.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(ability) if ability.effects.all_effects().iter().any(|effect|
            effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>().is_some()) => Some(ability),
        _ => None,
    }).expect("craft return");
    let material_choice = craft.mana_cost.costs().iter().find_map(|cost| cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>())).unwrap();
    assert!(material_choice.filter.shares_card_type);
    assert_eq!(material_choice.count.min, 2);
    assert_eq!(material_choice.count.max, Some(2));
    #[derive(Default)]
    struct Menu { types: Vec<String> }
    impl ironsmith::decision::DecisionMaker for Menu {
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            ironsmith::decision::DecisionMaker::decide_objects(&mut ironsmith::decision::SelectFirstDecisionMaker, game, ctx)
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.description == "Choose a card type" {
                self.types = ctx.options.iter().map(|option| option.description.to_ascii_lowercase()).collect();
                vec![ctx.options.last().unwrap().index]
            } else { vec![ctx.options.first().unwrap().index] }
        }
    }
    for (first, second, expected, chosen) in [
        (vec![CardType::Artifact, CardType::Creature], vec![CardType::Artifact, CardType::Creature], vec!["artifact", "creature"], Some(CardType::Creature)),
        (vec![CardType::Kindred, CardType::Instant], vec![CardType::Kindred, CardType::Sorcery], vec!["kindred"], Some(CardType::Kindred)),
        (vec![CardType::Instant], vec![CardType::Instant], vec!["instant"], Some(CardType::Instant)),
        (vec![CardType::Land], vec![CardType::Land], vec!["land"], Some(CardType::Land)),
        (vec![CardType::Artifact], vec![CardType::Creature], vec![], None),
    ] {
        let alice = PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.register_linked_face_definition(&back);
        // Resolution begins after the craft costs have exiled the source and materials.
        let source = game.create_object_from_definition(&front, alice, Zone::Exile);
        for types in [first, second] {
            let material = CardBuilder::new(CardId::new(), "Craft material").card_types(types).build();
            let id = game.create_object_from_card(&material, alice, Zone::Exile);
            game.add_exiled_with_source_link(source, id);
        }
        let unrelated = CardBuilder::new(CardId::new(), "Unrelated exiled card").card_types(vec![CardType::Land, CardType::Artifact]).build();
        let ignored = game.create_object_from_card(&unrelated, alice, Zone::Exile);
        let source_snapshot = ObjectSnapshot::from_object_id(&game, source).unwrap();
        let mut dm = Menu::default();
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(ironsmith::tag::SOURCE_COST_PUBLIC_ARRIVAL_TAG, vec![source_snapshot]);
        for effect in craft.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        let arrived = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Apex Observatory").unwrap();
        assert_eq!(dm.types, expected);
        assert_eq!(game.chosen_card_type(arrived), chosen);
        assert!(game.is_tapped(arrived));
        assert_eq!(game.object(ignored).unwrap().zone, Zone::Exile);
        assert_eq!(game.get_exiled_with_source_links(arrived).len(), 2);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let free_cast = back.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(&ability.effects), _ => None,
        }).unwrap();
        let source_memory = ObjectSnapshot::from_object_id(&game, arrived).unwrap();
        assert_eq!(source_memory.chosen_card_type, chosen);
        if chosen == Some(CardType::Kindred) {
            // The activated ability can resolve after its source is removed.
            let mut leave = ironsmith::effects::EffectContext::new_default(arrived, alice);
            ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::exile(ironsmith::target::ChooseSpec::Source), &mut leave).unwrap();
        }
        let mut grant_ctx = ironsmith::effects::EffectContext::new_default(arrived, alice);
        grant_ctx.source_snapshot = Some(source_memory);
        for effect in free_cast.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut grant_ctx).unwrap(); }
        drop(grant_ctx);
        let types = match chosen {
            Some(CardType::Kindred) => vec![CardType::Kindred, CardType::Instant],
            Some(kind) => vec![kind], None => vec![CardType::Creature],
        };
        let spell = CardBuilder::new(CardId::new(), "Matching spell").card_types(types)
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(9)]])).build();
        let matching = game.create_object_from_card(&spell, alice, Zone::Hand);
        let wrong_type = if chosen == Some(CardType::Creature) { CardType::Artifact } else { CardType::Creature };
        let wrong = CardBuilder::new(CardId::new(), "Wrong type").card_types(vec![wrong_type])
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(9)]])).build();
        let wrong = game.create_object_from_card(&wrong, alice, Zone::Hand);
        let has_free_cast = |game: &ironsmith::GameState, object| ironsmith::decision::compute_legal_actions(game, alice).unwrap()
            .iter().any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, casting_method, .. }
                if *spell_id == object && *casting_method != ironsmith::alternative_cast::CastingMethod::Normal));
        let expected_free = chosen.is_some() && chosen != Some(CardType::Land);
        assert_eq!(has_free_cast(&game, matching), expected_free, "the permission must retain the chosen card type");
        assert!(!has_free_cast(&game, wrong));
        if game.object(arrived).is_some() {
            let mut leave_ctx = ironsmith::effects::EffectContext::new_default(arrived, alice);
            ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::exile(ironsmith::target::ChooseSpec::Source), &mut leave_ctx).unwrap();
        }
        assert_eq!(has_free_cast(&game, matching), expected_free, "the resolved permission must survive its source leaving");
        assert!(!has_free_cast(&game, wrong));
    }

    // Also exercise the real activation/payment path, including source-ID changes.
    use ironsmith::game_loop::*;
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.register_linked_face_definition(&back);
    game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 6);
    let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
    let old_card = CardBuilder::new(CardId::new(), "Older linked artifact").card_types(vec![CardType::Artifact]).build();
    let old_card = game.create_object_from_card(&old_card, alice, Zone::Exile);
    game.add_exiled_with_source_link(source, old_card);
    for (types, zone) in [(vec![CardType::Creature], Zone::Battlefield), (vec![CardType::Artifact, CardType::Creature], Zone::Graveyard)] {
        let material = CardBuilder::new(CardId::new(), "Paid craft material").card_types(types).build();
        game.create_object_from_card(&material, alice, zone);
    }
    let ability_index = front.abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Activated(value) if std::ptr::eq(value, craft))).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = Menu::default();
    let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(ironsmith::decision::LegalAction::ActivateAbility { source, ability_index }), &mut dm).unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && !game.stack.is_empty() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let arrived = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Apex Observatory").unwrap();
    assert_eq!(dm.types, vec!["creature"]);
    assert_eq!(game.chosen_card_type(arrived), Some(CardType::Creature));
    assert!(game.is_tapped(arrived));
    assert_eq!(game.get_exiled_with_source_links(arrived).len(), 2);
    assert!(!game.get_exiled_with_source_links(arrived).contains(&old_card));
    assert_eq!(game.object(old_card).unwrap().zone, Zone::Exile);
}

#[test]
fn damage_recipient_choice_keeps_the_prior_player_and_only_their_planeswalkers() {
    use ironsmith::ability::AbilityKind;
    #[derive(Default)]
    struct Pick { walker: bool, menus: Vec<Vec<String>> }
    impl ironsmith::decision::DecisionMaker for Pick {
        fn decide_options(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.menus.push(ctx.options.iter().map(|option| option.description.clone()).collect());
            let name = if self.walker { "Bob's walker" } else { "Bob" };
            vec![ctx.options.iter().find(|option| option.description == name).expect("chosen damage recipient must be offered").index]
        }
    }
    for (name, oracle, amount) in [
        ("Curse of the Pierced Heart", "Type: Enchantment — Aura\nEnchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.", 1),
        ("Vial Smasher the Fierce", "Mana cost: {1}{B}{R}\nType: Legendary Creature — Goblin Berserker\nPower/Toughness: 2/3\nWhenever you cast your first spell each turn, choose an opponent at random. Vial Smasher deals damage equal to that spell's mana value to that player or a planeswalker that player controls.\nPartner", 4),
    ] {
        let definition = compile_to_runtime_definition(name, oracle, false).unwrap();
        let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(ability) => Some(ability), _ => None,
        }).unwrap();
        for choose_walker in [false, true] {
            let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            if amount == 1 { assert!(game.attach_object_to_target(source, ironsmith::object::AttachmentTarget::Player(bob))); }
            let walker = |name| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Planeswalker]).build();
            let bob_walker = game.create_object_from_card(&walker("Bob's walker"), bob, Zone::Battlefield);
            let wrong_walker = game.create_object_from_card(&walker("Alice's walker"), alice, Zone::Battlefield);
            for id in [bob_walker, wrong_walker] { game.object_mut(id).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty, 8); }
            let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(&game, &trigger.effects, alice, Some(source), None);
            assert!(requirements.is_empty(), "the damage recipient is chosen on resolution, not targeted");
            let spell = CardBuilder::new(CardId::new(), "Triggering spell").card_types(vec![CardType::Instant])
                .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(4)]])).build();
            let spell = game.create_object_from_card(&spell, alice, Zone::Stack);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game, spell).unwrap();
            let mut dm = Pick { walker: choose_walker, ..Default::default() };
            let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
            if amount == 4 {
                ctx = ctx.with_triggering_event(ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::SpellCastEvent::new_with_snapshot(spell, alice, Zone::Hand, snapshot.clone()),
                    ironsmith::provenance::ProvNodeId::default(),
                ));
            }
            ctx.set_tagged_objects("triggering", vec![snapshot]);
            for effect in trigger.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            drop(ctx);
            assert_eq!(dm.menus.last().unwrap(), &vec!["Bob".to_string(), "Bob's walker".to_string()]);
            assert_eq!(game.player(bob).unwrap().life, if choose_walker { 20 } else { 20 - amount });
            assert_eq!(game.counter_count(bob_walker, ironsmith::object::CounterType::Loyalty), if choose_walker { 8 - amount as u32 } else { 8 });
            assert_eq!(game.counter_count(wrong_walker, ironsmith::object::CounterType::Loyalty), 8);
            assert_eq!(game.player(alice).unwrap().life, 20);
        }
    }
}

#[test]
fn returned_creature_controller_rewards_use_original_successful_moves_including_tokens() {
    let definition = compile_to_runtime_definition("Faerie Slumber Party", "Type: Sorcery\nReturn all creatures to their owners' hands. For each opponent who controlled a creature returned this way, you create two 1/1 blue Faerie creature tokens with flying and \"This token can block only creatures with flying.\"", false).unwrap();
    for case in 0..5 {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let dan = PlayerId::from_index(3);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let creature = CardBuilder::new(CardId::new(), "Returned creature").card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2,2)).build();
        game.create_object_from_card(&creature, bob, Zone::Hand); // Older unrelated return cannot count.
        if case <= 1 || case == 4 {
            let borrowed = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            game.set_current_controller(borrowed, bob).unwrap();
            if case == 0 { game.create_object_from_card(&creature, bob, Zone::Battlefield); }
            if case == 4 {
                game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
                    source, alice,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(borrowed), Some(Zone::Battlefield), Some(Zone::Hand)),
                    ironsmith::replacement::ReplacementAction::Instead(vec![Effect::new(ironsmith::effects::MoveToZoneEffect::to_exile(ChooseSpec::SpecificObject(borrowed)))]),
                ));
            }
        }
        if case == 0 || case == 2 {
            let token = game.create_object_from_card(&creature, dan, Zone::Battlefield);
            game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
        }
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let tokens = game.battlefield.iter().filter_map(|id| game.object(*id))
            .filter(|object| object.kind == ironsmith::object::ObjectKind::Token && object.subtypes.contains(&Subtype::Faerie)).collect::<Vec<_>>();
        assert_eq!(tokens.len(), match case {0 => 4, 1|2 => 2, _ => 0}, "case {case}");
        assert!(tokens.iter().all(|token| game.controller_of(token) == alice));
        for token in tokens {
            assert_eq!(game.current_power(token.id), Some(1));
            assert_eq!(game.current_toughness(token.id), Some(1));
            assert!(game.current_has_static_ability_id(token.id, ironsmith::static_abilities::StaticAbilityId::Flying));
            assert!(game.current_has_static_ability_id(token.id, ironsmith::static_abilities::StaticAbilityId::CanBlockOnlyFlying));
        }
    }
}

#[test]
fn lifetime_trigger_limit_keeps_tapped_condition_and_resets_only_for_new_incarnation() {
    let definition = compile_to_runtime_definition("Acrobatic Cheerleader", "Type: Creature — Human Survivor\nPower/Toughness: 2/2\nSurvival — At the beginning of your second main phase, if this creature is tapped, put a flying counter on it. This ability triggers only once.", false).unwrap();
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let event = |player, ordinal| ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent::new(player).with_main_phase_ordinal(ordinal),
        ironsmith::provenance::ProvNodeId::default(),
    );
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 2)).is_empty(), "untapped condition still applies");
    game.tap(source);
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 3)).is_empty(), "second main, not every later main");
    assert!(ironsmith::triggers::check_triggers(&game, &event(bob, 2)).is_empty(), "only the controller's main");
    let watcher = CardBuilder::new(CardId::new(), "Trigger watcher").card_types(vec![CardType::Enchantment]).build();
    let watcher = game.create_object_from_card(&watcher, alice, Zone::Battlefield);
    game.object_mut(watcher).unwrap().abilities = vec![ironsmith::ability::Ability::triggered(
        ironsmith::triggers::Trigger::another_ability_triggers(), vec![Effect::gain_life(1)],
    )].into();
    let matches = ironsmith::triggers::check_triggers(&game, &event(alice, 2));
    assert_eq!(matches.len(), 1);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    queue.add(matches[0].clone()); queue.add(matches[0].clone());
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 2, "one allowed occurrence and exactly one watcher notification");
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.player(alice).unwrap().life, 21, "suppressed triggers emit no extra notification");
    assert_eq!(game.counter_count(source, ironsmith::object::CounterType::Flying), 1, "the limit cannot stop its first registered trigger resolving");
    game.next_turn(); game.next_turn(); game.tap(source);
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 2)).is_empty(), "limit survives turns");
    game.set_current_controller(source, bob).unwrap();
    assert!(ironsmith::triggers::check_triggers(&game, &event(bob, 2)).is_empty(), "limit survives control change");
    let hand = game.move_object_by_effect(source, Zone::Hand).unwrap();
    let returned = game.move_object_by_effect(hand, Zone::Battlefield).unwrap();
    game.tap(returned);
    let matches = ironsmith::triggers::check_triggers(&game, &event(alice, 2));
    assert_eq!(matches.len(), 1, "a new incarnation may trigger again");
    queue.add(matches[0].clone());
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(returned, ironsmith::object::CounterType::Flying), 1);
    let fresh = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.tap(fresh);
    let matches = ironsmith::triggers::check_triggers(&game, &event(alice, 2));
    assert_eq!(matches.len(), 1);
    queue.add(matches[0].clone());
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    game.untap(fresh);
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(fresh, ironsmith::object::CounterType::Flying), 0, "tapped condition is checked again on resolution");
    game.tap(fresh);
    assert!(ironsmith::triggers::check_triggers(&game, &event(alice, 2)).is_empty(), "the lifetime allowance was consumed when it triggered");
}

#[test]
fn search_offer_rewards_wait_for_all_offers_and_shuffle_each_actual_searcher_once() {
    #[derive(Default)]
    struct Answers { accepted: Vec<PlayerId>, selections: Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.accepted.contains(&context.player)
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.selections.push(context.player);
            context.candidates.iter().filter(|candidate| candidate.legal).take(1).map(|candidate| candidate.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("Tempt with Discovery", "Type: Sorcery\nTempting offer — Search your library for a land card and put it onto the battlefield. Each opponent may search their library for a land card and put it onto the battlefield. For each opponent who searches a library this way, search your library for a land card and put it onto the battlefield. Then each player who searched a library this way shuffles.", false).unwrap();
    fn participant_tag(effect: &Effect, found: &mut Option<ironsmith::tag::TagKey>) {
        if let Some(choice) = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>() {
            if let Some(tag) = &choice.search_participant_tag { *found = Some(tag.clone()); }
        }
        effect.visit_child_effects(&mut |child| participant_tag(child, found));
    }
    let mut tag = None;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { participant_tag(effect, &mut tag); }
    let tag = tag.expect("exact participant capture on the native search instruction");
    for case in 0..4 {
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let dan = PlayerId::from_index(3);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let land = CardBuilder::new(CardId::new(), "Searchable land").card_types(vec![CardType::Land]).build();
        for _ in 0..4 { game.create_object_from_card(&land, alice, Zone::Library); }
        game.create_object_from_card(&land, bob, Zone::Library);
        let blocked = match case {2 => Some(bob), 3 => Some(alice), _ => None};
        if let Some(player) = blocked {
            let lock = CardBuilder::new(CardId::new(), "Search restriction").card_types(vec![CardType::Enchantment]).build();
            let lock = game.create_object_from_card(&lock, alice, Zone::Battlefield);
            game.object_mut(lock).unwrap().abilities = vec![ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::restriction(
                    ironsmith::effect::Restriction::SearchLibraries(ironsmith::target::PlayerFilter::Specific(player)), "search restriction".to_string(),
                ),
            )].into();
        }
        game.refresh_continuous_state().unwrap();
        let mut dm = Answers { accepted: if case == 0 {vec![bob,dan]} else if case == 1 {vec![]} else {vec![bob]}, ..Default::default() };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        ctx.set_tagged_players(tag.clone(), vec![bob,dan]); // Previous repetitions cannot leak participants.
        let mut searches = Vec::new(); let mut shuffles = Vec::new();
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            let result = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            for event in result.events {
                if let Some(search) = event.downcast::<ironsmith::events::SearchLibraryEvent>() { searches.push(search.player); }
                if let Some(shuffle) = event.downcast::<ironsmith::events::ShuffleLibraryEvent>() { shuffles.push(shuffle.player); }
            }
        }
        let expected = match case {0 => vec![alice,bob,dan,alice,alice], 1 => vec![alice], 2 => vec![alice], _ => vec![bob]};
        assert_eq!(searches, expected, "actual searches and instruction order, case {case}");
        shuffles.sort();
        let mut expected = match case {0 => vec![alice,bob,dan], 1|2 => vec![alice], _ => vec![bob]}; expected.sort();
        assert_eq!(shuffles, expected, "one final shuffle per actual searcher, case {case}");
        let alice_lands = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| game.controller_of(object) == alice && object.card_types.contains(&CardType::Land)).count();
        assert_eq!(alice_lands, match case {0 => 3, 3 => 0, _ => 1}, "reward searches belong to the controller");
    }
}

#[test]
fn bounded_filtered_exile_casts_recheck_the_pool_and_preserve_owner_and_face_up_limits() {
    #[derive(Default)] struct Answers { limit: usize, accepted: usize, labels: Vec<String> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            if self.accepted < self.limit { self.accepted += 1; true } else { false }
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.labels.extend(context.options.iter().map(|option| option.description.clone()));
            vec![0]
        }
    }
    let definition = compile_to_runtime_definition("Ashiok, Nightmare Muse", "Type: Legendary Planeswalker — Ashiok\nLoyalty: 5\n+1: Create a 2/3 blue and black Nightmare creature token with \"Whenever this token attacks or blocks, each opponent exiles the top two cards of their library.\"\n−3: Return target nonland permanent to its owner's hand, then that player exiles a card from their hand.\n−7: You may cast up to three spells from among face-up cards your opponents own from exile without paying their mana costs.", false).unwrap();
    let activated = definition.abilities.iter().filter_map(|ability| match &ability.kind { ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None }).collect::<Vec<_>>();
    assert_eq!(activated.len(), 3);
    let ultimate = activated[2];
    for limit in [0, 1, 3, 5] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell = |name| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Instant])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(9)])).build();
        let mut eligible = Vec::new();
        for (owner, name) in [(bob,"Bob A"),(bob,"Bob B"),(carol,"Carol A"),(carol,"Carol B")] {
            eligible.push(game.create_object_from_card(&spell(name), owner, Zone::Exile));
        }
        let own = game.create_object_from_card(&spell("Forbidden own spell"), alice, Zone::Exile);
        let hidden = game.create_object_from_card(&spell("Forbidden hidden spell"), bob, Zone::Exile); game.set_face_down(hidden);
        let land = CardBuilder::new(CardId::new(), "Forbidden land").card_types(vec![CardType::Land]).build();
        let land = game.create_object_from_card(&land, bob, Zone::Exile);
        let mut dm = Answers { limit, ..Default::default() };
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in ultimate.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        drop(ctx);
        assert_eq!(game.stack.len(), limit.min(3), "bounded immediate casts, requested {limit}");
        assert_eq!(eligible.iter().filter(|id| game.object(**id).is_some_and(|object| object.zone == Zone::Exile)).count(), 4-limit.min(3), "each cast must leave the pool before the next choice");
        for id in [own, hidden, land] { assert_eq!(game.object(id).unwrap().zone, Zone::Exile); }
        for entry in &game.stack { assert_eq!(entry.controller, alice); }
        assert!(dm.labels.iter().all(|label| !label.contains("Forbidden")), "illegal owners, face-down cards and lands must never be offered");
    }
}

#[test]
fn creature_payment_prohibition_applies_to_casts_and_activations_but_not_resolution_costs() {
    use ironsmith::costs::{Cost, CostContext, PaymentReason};
    #[derive(Default)] struct Picks;
    impl ironsmith::decision::DecisionMaker for Picks {
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let choices = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect::<Vec<_>>();
            assert!(choices.iter().all(|id| !game.current_has_card_type(*id, CardType::Creature)), "prohibited creatures must not reach the payment choice");
            choices.into_iter().take(context.min.max(1)).collect()
        }
    }
    let definition = compile_to_runtime_definition("Angel of Jubilation", "Mana cost: {1}{W}{W}{W}\nType: Creature — Angel\nPower/Toughness: 3/3\nFlying\nOther nonblack creatures you control get +1/+1.\nPlayers can't pay life or sacrifice creatures to cast spells or activate abilities.", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
    let angel = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let card = |types| CardBuilder::new(CardId::new(), "Payment subject").card_types(types).power_toughness(PowerToughness::fixed(2,2)).build();
    let creature = game.create_object_from_card(&card(vec![CardType::Creature]),bob,Zone::Battlefield);
    let land_creature = game.create_object_from_card(&card(vec![CardType::Land,CardType::Creature]),bob,Zone::Battlefield);
    let artifact = game.create_object_from_card(&card(vec![CardType::Artifact]),bob,Zone::Battlefield);
    let land = game.create_object_from_card(&card(vec![CardType::Land]),bob,Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(game.current_has_static_ability_id(angel, ironsmith::static_abilities::StaticAbilityId::Flying));
    for payer in [alice,bob] {
        for reason in [PaymentReason::CastSpell,PaymentReason::ActivateAbility,PaymentReason::ActivateManaAbility] {
            let mut dm = Picks;
            let ctx = CostContext::new(creature,payer,&mut dm).with_reason(reason);
            assert!(Cost::life(2).can_pay(&game,&ctx).is_err());
            assert!(Cost::life(0).can_pay(&game,&ctx).is_ok(), "zero-life payments remain legal");
        }
    }
    for reason in [PaymentReason::CastSpell,PaymentReason::ActivateAbility,PaymentReason::ActivateManaAbility] {
        for subject in [creature,land_creature] {
            let mut dm = Picks; let ctx = CostContext::new(subject,bob,&mut dm).with_reason(reason);
            assert!(Cost::sacrifice_self().can_pay(&game,&ctx).is_err());
        }
        for subject in [artifact,land] {
            let mut dm = Picks; let ctx = CostContext::new(subject,bob,&mut dm).with_reason(reason);
            assert!(Cost::sacrifice_self().can_pay(&game,&ctx).is_ok());
        }
        let mut paid = game.clone(); let mut dm = Picks;
        let mut ctx = CostContext::new(creature,bob,&mut dm).with_reason(reason);
        Cost::sacrifice(ironsmith::target::ObjectFilter::default()).pay(&mut paid,&mut ctx).unwrap();
        assert!(paid.object(creature).is_some()); assert!(paid.object(land_creature).is_some());
    }
    let mut paid = game.clone(); let mut dm = ironsmith::decision::AutoPassDecisionMaker;
    let mut ctx = CostContext::new(creature,bob,&mut dm).with_reason(PaymentReason::Effect);
    Cost::life(2).pay(&mut paid,&mut ctx).unwrap();
    Cost::sacrifice_self().pay(&mut paid,&mut ctx).unwrap();
    assert_eq!(paid.player(bob).unwrap().life,18);
    assert!(paid.object(creature).is_none(), "resolution costs may sacrifice creatures");
    game.move_object_by_effect(angel,Zone::Graveyard).unwrap();
    let mut dm = Picks; let ctx = CostContext::new(land_creature,bob,&mut dm).with_reason(PaymentReason::CastSpell);
    assert!(Cost::sacrifice_self().can_pay(&game,&ctx).is_ok(), "source leaving removes its prohibition");
    assert!(Cost::life(2).can_pay(&game,&ctx).is_ok());
}

#[test]
fn excess_damage_trigger_uses_historical_typed_sources_and_exact_recipient() {
    let definition = compile_to_runtime_definition("Aegar, the Freezing Flame", "Mana cost: {1}{U}{R}\nType: Legendary Creature — Giant Wizard\nPower/Toughness: 3/3\nWhenever a creature or planeswalker an opponent controls is dealt excess damage, if a Giant, Wizard, or spell you controlled dealt damage to it this turn, draw a card.", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for (label, types, subtypes, zone, owner, prior, current, expected) in [
        ("earlier Giant", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 1),
        ("earlier Wizard", vec![CardType::Creature], vec![Subtype::Wizard], Zone::Battlefield, alice, true, false, 1),
        ("earlier spell", vec![CardType::Instant], vec![], Zone::Stack, alice, true, false, 1),
        ("current Giant", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 1),
        ("current spell", vec![CardType::Instant], vec![], Zone::Stack, alice, false, true, 1),
        ("opponent spell", vec![CardType::Instant], vec![], Zone::Stack, bob, true, false, 0),
        ("opponent Giant", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, bob, true, false, 0),
        ("unqualified source", vec![CardType::Artifact], vec![], Zone::Battlefield, alice, true, false, 0),
        ("Giant permanent", vec![CardType::Kindred,CardType::Enchantment], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 1),
        ("Giant card", vec![CardType::Creature], vec![Subtype::Giant], Zone::Graveyard, alice, true, false, 0),
        ("different recipient", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("new incarnation", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("previous turn", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("prevented earlier damage", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 0),
        ("exact lethal", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 0),
        ("own recipient", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 0),
        ("planeswalker recipient", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 1),
        ("combat damage", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, false, true, 1),
        ("lost Giant subtype", vec![CardType::Creature], vec![Subtype::Giant], Zone::Battlefield, alice, true, false, 1),
        ("gained Giant subtype", vec![CardType::Creature], vec![], Zone::Battlefield, alice, true, false, 0),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let victim_types = if label == "planeswalker recipient" { vec![CardType::Planeswalker] } else { vec![CardType::Creature] };
        let toughness = if label == "exact lethal" { 5 } else { 4 };
        let victim_card = CardBuilder::new(CardId::new(),"Recipient").card_types(victim_types).power_toughness(PowerToughness::fixed(4,toughness)).build();
        let mut victim = game.create_object_from_card(&victim_card,if label == "own recipient" { alice } else { bob },Zone::Battlefield);
        if label == "planeswalker recipient" { game.object_mut(victim).unwrap().counters.insert(ironsmith::object::CounterType::Loyalty,4); }
        let source_card = CardBuilder::new(CardId::new(),"Historical source").card_types(types).subtypes(subtypes).power_toughness(PowerToughness::fixed(1,1)).build();
        let source = game.create_object_from_card(&source_card,owner,zone);
        let other_card = CardBuilder::new(CardId::new(),"Other damage source").card_types(vec![CardType::Artifact]).build();
        let other = game.create_object_from_card(&other_card,bob,Zone::Battlefield);
        let library_card = CardBuilder::new(CardId::new(),"Drawn card").card_types(vec![CardType::Land]).build();
        game.create_object_from_card(&library_card,alice,Zone::Library);
        if prior {
            let mut ctx = ironsmith::effects::EffectContext::new_default(source,owner);
            if label == "prevented earlier damage" {
                ironsmith::effects::execute_effect(&mut game,&Effect::prevent_damage(1,ChooseSpec::SpecificObject(victim),ironsmith::effect::Until::EndOfTurn),&mut ctx).unwrap();
            }
            let prior_recipient = if label == "different recipient" { game.create_object_from_card(&victim_card,bob,Zone::Battlefield) } else { victim };
            ironsmith::effects::execute_effect(&mut game,&Effect::deal_damage(1,ChooseSpec::SpecificObject(prior_recipient)),&mut ctx).unwrap();
            // Historical controller and subtype survive later changes and a spell leaving the stack.
            if zone == Zone::Stack { game.move_object_by_effect(source,Zone::Graveyard).unwrap(); }
            else if !matches!(label,"lost Giant subtype" | "gained Giant subtype") { game.set_current_controller(source,if owner==alice {bob} else {alice}).unwrap(); }
            if label == "lost Giant subtype" { game.object_mut(source).unwrap().subtypes.clear(); }
            if label == "gained Giant subtype" { game.object_mut(source).unwrap().subtypes = vec![Subtype::Giant].into(); }
            if label == "new incarnation" {
                let graveyard = game.move_object_by_effect(victim,Zone::Graveyard).unwrap();
                victim = game.move_object_by_effect(graveyard,Zone::Battlefield).unwrap();
            }
            if label == "previous turn" { game.next_turn(); }
        }
        let damager = if current {source} else {other};
        let controller = if current {owner} else {bob};
        let mut ctx = ironsmith::effects::EffectContext::new_default(damager,controller);
        let mut damage = ironsmith::effects::DealDamageEffect::new(5,ChooseSpec::SpecificObject(victim));
        damage.source_is_combat = label == "combat damage";
        ironsmith::effects::execute_effect(&mut game,&Effect::new(damage),&mut ctx).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
        assert_eq!(queue.entries.len(),expected,"{label}: trigger matching");
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        if game.object(victim).is_some() { game.move_object_by_effect(victim,Zone::Graveyard).unwrap(); }
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        assert_eq!(game.player(alice).unwrap().hand.len(),expected,"{label}: resolution uses the damaged object's captured identity");
    }
}

#[test]
fn draft_noted_name_search_keeps_optional_zones_cast_and_shuffle_correlated() {
    struct Answers { zone: Zone, search: bool, cast: bool, fail: bool, zones: Vec<Zone>, booleans: usize }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.booleans += 1;
            if self.booleans == 1 { self.search } else { self.cast }
        }
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            if self.fail { return vec![]; }
            context.candidates.iter().filter(|candidate| candidate.legal && game.object(candidate.id).is_some_and(|object| object.zone == self.zone && object.name == "Noted spell")).take(1).map(|candidate| candidate.id).collect()
        }
        fn decide_options(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if context.description.to_lowercase().contains("zones to search") {
                return context.options.iter().enumerate().filter_map(|(index, option)| self.zones.iter().any(|zone| option.description.eq_ignore_ascii_case(&zone.to_string())).then_some(index)).collect();
            }
            vec![0]
        }
    }
    let definition = compile_to_runtime_definition("Aether Searcher", "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 6/4\nReveal this card as you draft it. Reveal the next card you draft and note its name.\nWhen this creature enters, you may search your hand and/or library for a card with a name noted as you drafted cards named Aether Searcher. You may cast it without paying its mana cost. If you search your library this way, shuffle.", false).unwrap();
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {ironsmith::ability::AbilityKind::Triggered(ability) => Some(ability), _ => None}).unwrap();
    fn optional_zones(effect: &Effect) -> bool {
        let mut found = effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().is_some_and(|choice| choice.optional_search_zones);
        effect.visit_child_effects(&mut |child| found |= optional_zones(child));
        found
    }
    assert!(ability.effects.all_effects().into_iter().any(optional_zones), "authored and/or search zones must survive lowering");
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["hand", "library", "both", "decline search", "decline cast", "no note", "opponent note", "fail to find", "land", "blocked library"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(),"Noted spell").card_types(vec![if case == "land" {CardType::Land} else {CardType::Instant}]);
        let spell = if case == "land" { spell } else { spell.mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(9)])) }.build();
        let hand = game.create_object_from_card(&spell,alice,Zone::Hand);
        let library = game.create_object_from_card(&spell,alice,Zone::Library);
        let forbidden = CardBuilder::new(CardId::new(),"Unnoted spell").card_types(vec![CardType::Instant]).build();
        game.create_object_from_card(&forbidden,alice,Zone::Library);
        game.create_object_from_card(&spell,bob,Zone::Library);
        if case != "no note" { game.record_draft_noted_name(if case == "opponent note" { bob } else { alice },"Aether Searcher","Noted spell"); }
        if case == "blocked library" {
            let lock_card = CardBuilder::new(CardId::new(),"Search restriction").card_types(vec![CardType::Enchantment]).build();
            let lock = game.create_object_from_card(&lock_card,alice,Zone::Battlefield);
            game.object_mut(lock).unwrap().abilities = vec![ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::restriction(ironsmith::effect::Restriction::SearchLibraries(ironsmith::target::PlayerFilter::You), "Library search restriction".to_string()),
            )].into();
            game.refresh_continuous_state().unwrap();
        }
        let source_snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,source).unwrap();
        game.move_object_by_effect(source,Zone::Graveyard).unwrap(); // Trigger survives its source leaving.
        let zone = if matches!(case,"library" | "both" | "fail to find") {Zone::Library} else {Zone::Hand};
        let zones = if case == "both" {vec![Zone::Hand,Zone::Library]} else {vec![zone]};
        let mut dm = Answers {zone, search:case != "decline search",cast:case != "decline cast",fail:case == "fail to find",zones,booleans:0};
        let mut ctx = ironsmith::effects::EffectContext::new(source,alice,&mut dm);
        ctx.source_snapshot = Some(source_snapshot);
        let mut shuffles = 0;
        for effect in ability.effects.all_effects() {
            let result = ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();
            shuffles += result.events.iter().filter(|event| event.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count();
        }
        let cast = !matches!(case,"decline search" | "decline cast" | "no note" | "opponent note" | "fail to find" | "land");
        assert_eq!(game.stack.len(),usize::from(cast),"{case}: immediate free cast");
        assert_eq!(shuffles,usize::from(matches!(case,"library" | "both" | "fail to find")),"{case}: shuffle only an actually searched library");
        if cast { assert!(game.object(if zone == Zone::Hand {hand} else {library}).is_none_or(|object| object.zone == Zone::Stack)); }
    }
}

#[test]
fn tapped_to_pay_keyword_cost_triggers_only_for_actual_teamwork_contributions() {
    use ironsmith::costs::{Cost, CostContext, PaymentReason};
    struct Picks(Vec<ironsmith::ObjectId>);
    impl ironsmith::decision::DecisionMaker for Picks {
        fn decide_objects(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> { self.0.clone() }
    }
    let definition = compile_to_runtime_definition("Agent Maria Hill", "Mana cost: {W}\nType: Legendary Creature — Human Spy Hero\nPower/Toughness: 2/1\nWhenever Agent Maria Hill becomes tapped to pay a teamwork cost, put a +1/+1 counter on her and draw a card.", false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["teamwork", "crew", "effect tap", "tap ability cost", "insufficient power", "multiple contributors", "other contributor", "untapped before resolution", "new incarnation"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let maria = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        let other_maria = game.create_object_from_definition(&definition,bob,Zone::Battlefield);
        let support = CardBuilder::new(CardId::new(),"Support creature").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3,3)).build();
        let support = game.create_object_from_card(&support,alice,Zone::Battlefield);
        if case == "insufficient power" { game.move_object_by_effect(support,Zone::Hand).unwrap(); }
        let spell_card = CardBuilder::new(CardId::new(),"Payment source").card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Vehicle]).build();
        let spell = game.create_object_from_card(&spell_card,alice,Zone::Battlefield);
        let drawn = CardBuilder::new(CardId::new(),"Drawn card").card_types(vec![CardType::Land]).build();
        let drawn = game.create_object_from_card(&drawn,alice,Zone::Library);
        if case == "effect tap" {
            let mut ctx = ironsmith::effects::EffectContext::new_default(spell,alice);
            ironsmith::effects::execute_effect(&mut game,&Effect::tap(ChooseSpec::SpecificObject(maria)),&mut ctx).unwrap();
        } else if case == "tap ability cost" {
            let mut dm = Picks(vec![maria]);
            Cost::tap().pay(&mut game,&mut CostContext::new(maria,alice,&mut dm).with_reason(PaymentReason::ActivateAbility)).unwrap();
        } else {
            let cost = Cost::effect(if case == "crew" { ironsmith::effects::CrewCostEffect::new(2) } else { ironsmith::effects::CrewCostEffect::teamwork(if case == "insufficient power" {3} else if case == "multiple contributors" {5} else {2}) });
            let mut dm = Picks(if case == "multiple contributors" {vec![maria,support]} else if case == "other contributor" {vec![support]} else {vec![maria]});
            let mut ctx = CostContext::new(spell,alice,&mut dm).with_reason(PaymentReason::CastSpell);
            if case == "insufficient power" {
                assert!(cost.can_pay(&game,&ctx).is_err()); assert!(cost.pay(&mut game,&mut ctx).is_err());
            } else { cost.pay(&mut game,&mut ctx).unwrap(); }
        }
        let triggers = !matches!(case,"crew" | "effect tap" | "tap ability cost" | "insufficient power" | "other contributor");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
        assert_eq!(queue.entries.len(),usize::from(triggers),"{case}: exact source and typed cost");
        assert_eq!(game.counter_count(maria,ironsmith::object::CounterType::PlusOnePlusOne),0,"trigger can't increase power while costs are still being paid");
        assert!(game.player(alice).unwrap().library.contains(&drawn));
        ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
        let mut returned = None;
        if case == "untapped before resolution" { game.untap(maria); }
        if case == "new incarnation" {
            let hand = game.move_object_by_effect(maria,Zone::Hand).unwrap();
            returned = Some(game.move_object_by_effect(hand,Zone::Battlefield).unwrap());
        }
        while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
        assert_eq!(game.player(alice).unwrap().hand.iter().filter(|id| game.object(**id).is_some_and(|object| object.name == "Drawn card")).count(),usize::from(triggers),"{case}: draw");
        assert_eq!(game.counter_count(returned.unwrap_or(maria),ironsmith::object::CounterType::PlusOnePlusOne),u32::from(triggers && case != "new incarnation"),"{case}: source counter");
        assert_eq!(game.counter_count(other_maria,ironsmith::object::CounterType::PlusOnePlusOne),0);
    }
}

#[test]
fn conditional_landwalk_covers_all_basic_types_and_tracks_current_lands_and_controller() {
    let definition = compile_to_runtime_definition("Magnigoth Treefolk", "Mana cost: {4}{G}\nType: Creature — Treefolk\nPower/Toughness: 2/6\nDomain — For each basic land type among lands you control, this creature has landwalk of that type. (It can't be blocked as long as defending player controls a land of that type.)", false).unwrap();
    let types = [Subtype::Plains,Subtype::Island,Subtype::Swamp,Subtype::Mountain,Subtype::Forest];
    assert_eq!(definition.abilities.len(),6);
    assert!(matches!(&definition.abilities[0].kind,ironsmith::ability::AbilityKind::Static(ability) if matches!(&ability.compiled_model().unwrap().payload, ironsmith_core::StaticAbilityPayload::SourceLineStaticGroup {member_count:5})));
    for (ability, land_type) in definition.abilities[1..].iter().zip(types) {
        let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else {panic!("static conditional required")};
        let model = ability.compiled_model().unwrap();
        let ironsmith_core::static_ability_model::StaticAbilityPayload::Conditional {ability,condition,implied_words} = &model.payload else {panic!("typed conditional required")};
        assert!(*implied_words, "generated type cases must retain word provenance through lowering and transport");
        assert!(matches!(&ability.payload,ironsmith_core::static_ability_model::StaticAbilityPayload::Landwalk(ironsmith_core::LandwalkKind::Subtype{subtype,snow:false}) if *subtype == land_type));
        assert_eq!(*condition,ironsmith_core::Condition::YouControl(ironsmith_core::ObjectFilter::land().with_subtype(land_type)));
    }
    let surface = ironsmith::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(surface.contains("For each basic land type among lands you control"), "{surface}");
    for bad in ["partial", "duplicate", "snow", "authored", "extra qualifier", "different zone"] {
        let mut probe = definition.clone();
        probe.canonical_text = "unrelated cached text".into();
        if bad == "partial" { probe.abilities.pop(); }
        else if bad == "different zone" { probe.abilities[1].functional_zones = vec![Zone::Hand]; }
        else {
            let ironsmith::ability::AbilityKind::Static(ability) = &mut probe.abilities[1].kind else {unreachable!()};
            let mut model = ability.compiled_model().unwrap().clone();
            let ironsmith_core::StaticAbilityPayload::Conditional {ability: inner,condition,implied_words} = &mut model.payload else {unreachable!()};
            match bad {
                "duplicate" => {
                    inner.payload = ironsmith_core::StaticAbilityPayload::Landwalk(ironsmith_core::LandwalkKind::Subtype{subtype:Subtype::Island,snow:false});
                    *condition = ironsmith_core::Condition::YouControl(ironsmith_core::ObjectFilter::land().with_subtype(Subtype::Island));
                }
                "snow" => { inner.payload = ironsmith_core::StaticAbilityPayload::Landwalk(ironsmith_core::LandwalkKind::Subtype{subtype:Subtype::Plains,snow:true}); }
                "authored" => { *implied_words = false; }
                "extra qualifier" => {
                    let ironsmith_core::Condition::YouControl(filter) = condition else {unreachable!()};
                    filter.supertypes.push(ironsmith::types::Supertype::Basic);
                }
                _ => unreachable!(),
            }
            *ability = ironsmith::static_abilities::StaticAbility::from_model(model);
        }
        let text = ironsmith::runtime_display::compiled_text_lines(&probe).join("\n");
        assert!(!text.contains("For each basic land type"), "{bad} must not claim the complete generated family: {text}");
    }
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    let land = |subtypes| CardBuilder::new(CardId::new(),"Typed nonbasic land").card_types(vec![CardType::Land]).subtypes(subtypes).build();
    let blocker_card = CardBuilder::new(CardId::new(),"Blocker").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1,1)).build();
    let can_block = |game: &ironsmith::GameState, attacker, blocker| ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),game);
    for mask in 0..32 {
        for (index, defender_type) in types.iter().enumerate() {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
            let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
            let blocker = game.create_object_from_card(&blocker_card,bob,Zone::Battlefield);
            for (bit, land_type) in types.iter().enumerate() {
                if mask & (1 << bit) != 0 {game.create_object_from_card(&land(vec![*land_type]),alice,Zone::Battlefield);}
            }
            game.create_object_from_card(&land(vec![*defender_type]),bob,Zone::Battlefield);
            game.create_object_from_card(&land(types.to_vec()),carol,Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            assert_eq!(can_block(&game,source,blocker),mask & (1 << index) == 0,"own types {mask:05b}, defender {defender_type:?}; another player's lands never grant abilities");
        }
    }
    let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let blocker = game.create_object_from_card(&blocker_card,bob,Zone::Battlefield);
    let own = game.create_object_from_card(&land(vec![Subtype::Forest]),alice,Zone::Battlefield);
    let defending = game.create_object_from_card(&land(vec![Subtype::Forest]),bob,Zone::Battlefield);
    game.create_object_from_card(&land(vec![Subtype::Swamp]),carol,Zone::Battlefield);
    assert!(!can_block(&game,source,blocker));
    game.object_mut(own).unwrap().subtypes = vec![Subtype::Island].into();
    assert!(can_block(&game,source,blocker),"lost a land type");
    game.object_mut(defending).unwrap().subtypes = vec![Subtype::Island].into();
    assert!(!can_block(&game,source,blocker),"current defending land type");
    game.set_current_controller(source,carol).unwrap();
    assert!(can_block(&game,source,blocker),"new controller has only Swamp");
    game.set_current_controller(source,alice).unwrap();
    assert!(!can_block(&game,source,blocker));
    game.move_object_by_effect(own,Zone::Graveyard).unwrap();
    assert!(can_block(&game,source,blocker),"land leaves battlefield");
    game.create_object_from_card(&land(vec![Subtype::Island,Subtype::Forest]),alice,Zone::Battlefield);
    assert!(!can_block(&game,source,blocker),"a nonbasic dual land supplies both basic land types");
    game.object_mut(defending).unwrap().subtypes = vec![Subtype::Forest].into();
    assert!(!can_block(&game,source,blocker));
    let rewrite = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::SpecificObject(source),
        ironsmith::continuous::Modification::RewriteText(ironsmith_core::TextChange::basic_land_type(Subtype::Forest,Subtype::Mountain).unwrap()),
        ironsmith::effect::Until::EndOfTurn,
    ));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source,alice);
    ironsmith::effects::execute_effect(&mut game,&rewrite,&mut ctx).unwrap();
    assert!(!can_block(&game,source,blocker),"a text change cannot change unwritten land-type cases generated by the generic domain rule");
}

#[test]
fn draft_notes_creature_types_apply_one_bonus_to_current_matching_other_creatures() {
    let definition = compile_to_runtime_definition("Paliano Vanguard", "Mana cost: {1}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/2\nDraft this card face up.\nAs you draft a creature card, you may reveal it, note its creature types, then turn this card face down.\nOther creatures you control of a type you noted for cards named Paliano Vanguard get +1/+1.", false).unwrap();
    let surface = ironsmith::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(surface.contains("type you noted for cards named Paliano Vanguard"), "{surface}");
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
    let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    let creature = |name, types| CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).subtypes(types).power_toughness(PowerToughness::fixed(2,2)).build();
    let matching = game.create_object_from_card(&creature("Two noted types",vec![Subtype::Human,Subtype::Soldier]),alice,Zone::Battlefield);
    let other = game.create_object_from_card(&creature("Unnoted type",vec![Subtype::Elf]),alice,Zone::Battlefield);
    let opposing = game.create_object_from_card(&creature("Opponent",vec![Subtype::Human]),bob,Zone::Battlefield);
    let typeless = game.create_object_from_card(&creature("Typeless",vec![]),alice,Zone::Battlefield);
    let artifact = CardBuilder::new(CardId::new(),"Noncreature").card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Human]).build();
    let artifact = game.create_object_from_card(&artifact,alice,Zone::Battlefield);
    let changeling = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(),"All creature types").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::changeling())).build();
    let changeling = game.create_object_from_definition(&changeling,alice,Zone::Battlefield);
    for id in [source,matching,other,opposing,typeless,changeling] {assert_eq!(game.calculated_power(id),Some(2),"empty notes");}
    game.record_draft_noted_creature_types(bob,"Paliano Vanguard",[Subtype::Elf]);
    game.record_draft_noted_creature_types(alice,"Different group",[Subtype::Elf]);
    assert_eq!(game.calculated_power(other),Some(2),"player and group isolation");
    game.record_draft_noted_creature_types(alice,"PALIANO VANGUARD",[Subtype::Human,Subtype::Soldier,Subtype::Human,Subtype::Equipment]);
    assert_eq!(game.draft_noted_creature_types(alice,"paliano vanguard"),&[Subtype::Human,Subtype::Soldier]);
    for id in [matching,changeling] {assert_eq!(game.calculated_power(id),Some(3),"one bonus even with multiple matching types");assert_eq!(game.calculated_toughness(id),Some(3));}
    for id in [source,other,opposing,typeless] {assert_eq!(game.calculated_power(id),Some(2),"source, unnoted, opponent, typeless excluded");}
    assert_eq!(game.calculated_power(artifact),None);
    game.object_mut(matching).unwrap().subtypes = vec![Subtype::Elf].into();
    assert_eq!(game.calculated_power(matching),Some(2),"current creature types");
    let change = Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::SpecificObject(matching),
        ironsmith::continuous::Modification::AddSubtypes(vec![Subtype::Human]),
        ironsmith::effect::Until::EndOfTurn,
    ));
    let mut ctx = ironsmith::effects::EffectContext::new_default(source,alice);
    ironsmith::effects::execute_effect(&mut game,&change,&mut ctx).unwrap();
    assert_eq!(game.object(matching).unwrap().subtypes.as_slice(),&[Subtype::Elf]);
    assert_eq!(game.calculated_power(matching),Some(3),"layered type change must affect the note filter");
    game.set_current_controller(source,bob).unwrap();
    assert_eq!(game.calculated_power(other),Some(2));
    let bob_elf = game.create_object_from_card(&creature("Bob's noted Elf",vec![Subtype::Elf]),bob,Zone::Battlefield);
    assert_eq!(game.calculated_power(bob_elf),Some(3),"new controller uses own notes");
    game.set_current_controller(source,alice).unwrap();
    let second = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
    assert_eq!(game.calculated_power(source),Some(3),"another same-named source can boost this one");
    assert_eq!(game.calculated_power(second),Some(3));
    game.move_object_by_effect(second,Zone::Graveyard).unwrap();
    game.record_draft_noted_creature_types(alice,"Paliano Vanguard",Subtype::all_creature_types().iter().copied());
    assert_eq!(game.calculated_power(other),Some(3),"noting all creature types");
    assert_eq!(game.calculated_power(typeless),Some(2),"no type still cannot share a noted type");
    game.move_object_by_effect(source,Zone::Graveyard).unwrap();
    assert_eq!(game.calculated_power(other),Some(2),"bonus ends when source leaves");
}

#[test]
fn draft_notes_name_search_reveals_one_noted_card_and_shuffles_only_when_accepted() {
    struct Answers { accept: bool, fail: bool, legal: Vec<String> }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.accept }
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.legal = context.candidates.iter().filter(|c|c.legal).map(|c|game.object(c.id).unwrap().name.to_string()).collect();
            if self.fail {vec![]} else {context.candidates.iter().filter(|c|c.legal).take(1).map(|c|c.id).collect()}
        }
    }
    let definition = compile_to_runtime_definition("Smuggler Captain", "Mana cost: {3}{B}\nType: Creature — Human Pirate\nPower/Toughness: 2/2\nDraft this card face up.\nAs you draft a card, you may reveal it, note its name, then turn this card face down.\nWhen this creature enters, you may search your library for a card with a name you noted for cards named Smuggler Captain, reveal it, put it into your hand, then shuffle.",false).unwrap();
    let ability = definition.abilities.iter().find_map(|a| match &a.kind {ironsmith::ability::AbilityKind::Triggered(a)=>Some(a), _=>None}).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for case in ["accept","decline","fail to find","no notes","source left"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Battlefield);
        for name in ["First noted","Second noted","Other player's note","Other group's note","Unnoted"] {
            let card = CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Land]).build();
            game.create_object_from_card(&card,alice,Zone::Library);
            game.create_object_from_card(&card,bob,Zone::Library);
        }
        if case != "no notes" {
            game.record_draft_noted_name(alice,"SMUGGLER CAPTAIN","First noted");
            game.record_draft_noted_name(alice,"Smuggler Captain","Second noted");
        }
        game.record_draft_noted_name(bob,"Smuggler Captain","Other player's note");
        game.record_draft_noted_name(alice,"Other group","Other group's note");
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,source).unwrap();
        if case == "source left" {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
        let mut dm = Answers{accept:case != "decline",fail:case == "fail to find",legal:vec![]};
        let mut ctx = ironsmith::effects::EffectContext::new(source,alice,&mut dm);
        ctx.source_snapshot = Some(snapshot);
        let mut shuffles = 0; let mut reveals = 0; let mut searches = 0;
        for effect in ability.effects.all_effects() {
            let result = ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap();
            shuffles += result.events.iter().filter(|e|e.downcast::<ironsmith::events::ShuffleLibraryEvent>().is_some()).count();
            reveals += result.events.iter().filter(|e|e.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).count();
            searches += result.events.iter().filter(|e|e.downcast::<ironsmith::events::SearchLibraryEvent>().is_some()).count();
        }
        drop(ctx);
        let found = matches!(case,"accept"|"source left");
        assert_eq!(game.player(alice).unwrap().hand.len(),usize::from(found),"{case}: one card total");
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert_eq!(shuffles,usize::from(case != "decline"),"{case}");
        assert_eq!(searches,usize::from(case != "decline"),"{case}");
        assert_eq!(reveals,usize::from(found),"{case}");
        if case != "decline" && case != "no notes" {dm.legal.sort();assert_eq!(dm.legal,vec!["First noted","Second noted"],"only this player's named group");}
        if case == "no notes" {assert!(dm.legal.is_empty());}
    }
}

#[test]
fn sacrifice_combat_history_union_uses_current_opponent_and_conditional_count() {
    struct Picks { offered: Vec<String>, players: Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Picks {
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.players.push(context.player);
            self.offered.extend(context.candidates.iter().filter(|c|c.legal).map(|c|game.object(c.id).unwrap().name.to_string()));
            context.candidates.iter().filter(|c|c.legal).take(context.max.unwrap_or(context.candidates.len())).map(|c|c.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("Gideon's Triumph", "Mana cost: {1}{W}\nType: Instant\nTarget opponent sacrifices a creature of their choice that attacked or blocked this turn. If you control a Gideon planeswalker, that player sacrifices two of those creatures of their choice instead.",false).unwrap();
    let program = definition.spell_effect.as_ref().unwrap();
    assert_eq!(program.segments.len(),1);
    assert_eq!(program.segments[0].self_replacements.len(),1,"one replacement, not additive sacrifices");
    let choice = program.segments[0].default_effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()).unwrap();
    let replacement_choice = program.segments[0].self_replacements[0].replacement_effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()).unwrap();
    assert_eq!(replacement_choice.filter,choice.filter,"count replacement must preserve the full original selection domain");
    let compact = ironsmith_core::filter_model::describe_shared_combat_history_union(&choice.filter).unwrap();
    assert!(compact.contains("attacked or blocked this turn"),"{compact}");
    for bad in ["partial","extra","qualified"] {
        let mut filter = choice.filter.clone();
        match bad {
            "partial" => {filter.any_of.pop();}
            "extra" => {filter.any_of.push(ironsmith_core::ObjectFilter::default());}
            "qualified" => {filter.any_of[0].subtypes.push(Subtype::Elf);}
            _=>unreachable!(),
        }
        assert!(ironsmith_core::filter_model::describe_shared_combat_history_union(&filter).is_none(),"{bad}: different domains cannot be collapsed");
    }
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    for case in ["ordinary","own Gideon","opponent Gideon","not planeswalker","no eligible","one eligible","previous turn","new incarnation","Gideon left"] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
        let source = game.create_object_from_definition(&definition,alice,Zone::Stack);
        let creature = |name| CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build();
        let attacked = game.create_object_from_card(&creature("Attacked"),bob,Zone::Battlefield);
        let blocked = game.create_object_from_card(&creature("Blocked"),bob,Zone::Battlefield);
        let idle = game.create_object_from_card(&creature("Idle"),bob,Zone::Battlefield);
        let other_player = game.create_object_from_card(&creature("Other opponent"),carol,Zone::Battlefield);
        let own = game.create_object_from_card(&creature("Caster creature"),alice,Zone::Battlefield);
        if case != "no eligible" {
            for id in [attacked,other_player,own] {game.mark_creature_attacked_this_turn(id);}
            if case != "one eligible" {
                let event = ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::CreatureBlockedEvent::new(blocked,own),ironsmith::ProvNodeId::default());
                game.turn_store.turn_history.record_event(&event,None,None);
            }
        }
        if case == "previous turn" {game.turn_store.turn_history.clear_for_new_turn();}
        if case == "new incarnation" {
            let hand = game.move_object_by_effect(attacked,Zone::Hand).unwrap();
            game.move_object_by_effect(hand,Zone::Battlefield).unwrap();
        }
        if matches!(case,"own Gideon"|"opponent Gideon"|"not planeswalker"|"one eligible"|"Gideon left") {
            let gideon = CardBuilder::new(CardId::new(),"Planeswalker fixture").card_types(vec![if case == "not planeswalker" {CardType::Creature} else {CardType::Planeswalker}]).subtypes(vec![Subtype::Gideon]).build();
            let gideon = game.create_object_from_card(&gideon,if case == "opponent Gideon" {bob} else {alice},Zone::Battlefield);
            if case == "Gideon left" {game.move_object_by_effect(gideon,Zone::Graveyard).unwrap();}
        }
        let mut dm = Picks{offered:vec![],players:vec![]};
        game.push_to_stack(ironsmith::game_state::StackEntry::new(source,alice).with_targets(vec![ironsmith::game_state::Target::Player(bob)]));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let count = if matches!(case,"no eligible"|"previous turn") {0} else if case == "own Gideon" {2} else {1};
        assert_eq!(game.player(bob).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|o|matches!(o.name.as_ref(),"Attacked"|"Blocked"|"Idle"))).count(),count,"{case}");
        assert!(game.object(idle).is_some(),"idle creature never eligible");
        assert!(game.object(other_player).is_some() && game.object(own).is_some(),"only targeted opponent sacrifices");
        assert!(dm.players.iter().all(|p|*p==bob),"sacrificing player chooses: {case}");
        assert!(!dm.offered.iter().any(|name|matches!(name.as_str(),"Idle"|"Other opponent"|"Caster creature")),"{case}: {:?}",dm.offered);
        if case == "ordinary" {dm.offered.sort();assert_eq!(dm.offered,vec!["Attacked","Blocked"],"either earlier combat role must qualify");}
    }
}

#[test]
fn wrecking_crew_x_entry_and_optional_modal_trigger_cover_every_mode_subset() {
    struct Answers { modes: Vec<usize>, bounds: Vec<(usize,usize)>, bob: PlayerId, token: ironsmith::ObjectId }
    impl ironsmith::decision::DecisionMaker for Answers {
        fn decide_options(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if context.description.to_lowercase().contains("mode") {
                self.bounds.push((context.min,context.max));
                assert_eq!(context.options.len(),4);
                assert!(context.options.iter().all(|o|o.legal));
                return self.modes.clone();
            }
            context.options.iter().filter(|o|o.legal).take(context.min).map(|o|o.index).collect()
        }
        fn decide_targets(&mut self, _: &ironsmith::GameState, context: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::game_state::Target> {
            context.requirements.iter().flat_map(|requirement| {
                let preferred = if requirement.legal_targets.contains(&ironsmith::game_state::Target::Object(self.token)) {ironsmith::game_state::Target::Object(self.token)} else {ironsmith::game_state::Target::Player(self.bob)};
                assert!(requirement.legal_targets.contains(&preferred));
                vec![preferred; requirement.min_targets]
            }).collect()
        }
        fn decide_objects(&mut self, game: &ironsmith::GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let mut legal = context.candidates.iter().filter(|c|c.legal).collect::<Vec<_>>();
            legal.sort_by_key(|c| usize::from(!game.object(c.id).unwrap().name.ends_with("fodder")));
            legal.into_iter().take(context.min).map(|c|c.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("The Ruinous Wrecking Crew", "Mana cost: {X}{B}{R}\nType: Legendary Creature — Human Villain\nPower/Toughness: 2/2\nThe Ruinous Wrecking Crew enters with X +1/+1 counters on it.\nWhen The Ruinous Wrecking Crew enters, choose up to X —\n• Discard a card, then draw a card.\n• Target opponent loses 2 life.\n• Destroy target token.\n• Each player sacrifices a creature of their choice.",false).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1); let carol = PlayerId::from_index(2);
    let creature = |name| CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build();
    for x in 0..=5u32 {
        for mask in 0..16u32 {
            if mask.count_ones() > x {continue;}
            let mut game = ironsmith::GameState::new(vec!["Alice".into(),"Bob".into(),"Carol".into()],20);
            let mut fodder = Vec::new();
            for (player,name) in [(alice,"Alice fodder"),(bob,"Bob fodder"),(carol,"Carol fodder")] {fodder.push(game.create_object_from_card(&creature(name),player,Zone::Battlefield));}
            let token = CardBuilder::new(CardId::new(),"Token fixture").card_types(vec![CardType::Artifact]).build();
            let token = game.create_object_from_card(&token,bob,Zone::Battlefield);
            game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
            let discarded = CardBuilder::new(CardId::new(),"Discard fixture").card_types(vec![CardType::Land]).build();
            let drawn = CardBuilder::new(CardId::new(),"Draw fixture").card_types(vec![CardType::Land]).build();
            game.create_object_from_card(&discarded,alice,Zone::Hand);
            game.create_object_from_card(&drawn,alice,Zone::Library);
            let spell = game.create_object_from_definition(&definition,alice,Zone::Stack);
            game.object_mut(spell).unwrap().x_value = Some(x);
            game.push_to_stack(ironsmith::game_state::StackEntry::new(spell,alice).with_x(x));
            let modes = (0..4usize).filter(|mode| mask & (1 << mode) != 0).collect::<Vec<_>>();
            let mut dm = Answers{modes:modes.clone(),bounds:vec![],bob,token};
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            let source = *game.battlefield.iter().find(|id|game.object(**id).unwrap().name == "The Ruinous Wrecking Crew").unwrap();
            assert_eq!(game.counter_count(source,ironsmith::object::CounterType::PlusOnePlusOne),x,"X={x}, modes={mask:04b}: entry counters");
            assert_eq!(game.calculated_power(source),Some(2+x as i32));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events(&mut game,&mut queue);
            assert_eq!(queue.entries.len(),1,"exactly one ETB trigger");
            assert_eq!(queue.entries[0].x_value,Some(x),"cast X must reach trigger choices");
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
            if mask == 0 {
                assert!(game.stack.is_empty(),"X={x}: CR 603.3c removes a modal trigger with no chosen modes");
            } else {
                assert_eq!(game.stack.len(),1,"X={x}, modes={mask:04b}");
                assert_eq!(game.stack[0].chosen_modes,Some(modes));
                ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
            }
            assert!(dm.bounds.iter().all(|(min,max)|*min==0 && *max<=x as usize && *max>=mask.count_ones() as usize),"X={x}, bounds={:?}",dm.bounds);
            if x > 0 {assert_eq!(dm.bounds.len(),1,"modes are selected as the trigger goes onto the stack");}
            let selected = |mode: usize| -> bool { mask & (1u32 << mode) != 0u32 };
            assert_eq!(game.player(bob).unwrap().life,if selected(1) {18} else {20});
            assert_eq!(game.player(alice).unwrap().life,20); assert_eq!(game.player(carol).unwrap().life,20);
            assert_eq!(game.object(token).is_none_or(|o|o.zone!=Zone::Battlefield),selected(2));
            for id in fodder {assert_eq!(game.object(id).is_none_or(|o|o.zone!=Zone::Battlefield),selected(3),"each player sacrifices their chosen creature");}
            let hand_names = game.player(alice).unwrap().hand.iter().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
            assert_eq!(hand_names,vec![if selected(0) {"Draw fixture"} else {"Discard fixture"}],"discard then draw mode");
            assert!(game.object(source).is_some(),"sacrifice choice can preserve the source");
            if x == 3 && mask == 0 {
                let exile = game.move_object_by_effect(source,Zone::Exile).unwrap();
                let returned = game.move_object_by_effect(exile,Zone::Battlefield).unwrap();
                assert_eq!(game.counter_count(returned,ironsmith::object::CounterType::PlusOnePlusOne),0,"new noncast entry does not reuse cast X");
            }
        }
    }
}
