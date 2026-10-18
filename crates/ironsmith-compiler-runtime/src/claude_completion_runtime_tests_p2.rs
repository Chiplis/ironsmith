//! Claude package 2 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn non_mana_flash_timing_payments_are_optional_flash_costs() {
    for (name, text, needle) in [
        ("Tegwyll's Scouring", "Mana cost: {4}{B}{B}\nType: Sorcery\nYou may cast this spell as though it had flash by tapping three untapped creatures you control with flying in addition to paying its other costs.\nDestroy all creatures. Create three 1/1 black Faerie Rogue creature tokens with flying.", "Tap"),
        ("Molten Exhale", "Mana cost: {1}{R}\nType: Sorcery\nYou may cast this spell as though it had flash if you behold a Dragon as an additional cost to cast it. (To behold a Dragon, choose a Dragon you control or reveal a Dragon card from your hand.)\nMolten Exhale deals 4 damage to target creature or planeswalker.", "Behold"),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        assert_eq!(definition.optional_costs.len(), 1, "{name}");
        let optional = &definition.optional_costs[0];
        assert_eq!(optional.kind, ironsmith_core::OptionalCostKind::FlashTiming, "{name}");
        assert!(optional.cost.has_non_mana_costs(), "{name}");
        assert!(optional.cost.mana_cost().is_none_or(|mana| mana.is_empty()), "{name}: no extra mana");
        let debug = format!("{:?}", optional.cost);
        assert!(debug.contains(needle), "{name}: {debug}");
        assert!(!definition.alternative_casts.iter().any(|method| matches!(method,
            ironsmith::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost { .. })));
    }
}

#[test]
fn source_exiled_return_at_random_is_a_random_bounded_pick() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Negative Zone Portal",
        "Type: Artifact\n{2}, {T}: Exile target card from an opponent's graveyard. If it's a creature card, draw a card.\nAt the beginning of your upkeep, if there are four or more creature cards exiled with this artifact, flip a coin. If you lose the flip, sacrifice this artifact and return a card exiled with it at random to its owner's hand.", false).unwrap();
    fn find(effect: &Effect, found: &mut Vec<ironsmith::effects::ReturnToHandEffect>) {
        if let Some(ret) = effect.downcast_ref::<ironsmith::effects::ReturnToHandEffect>() { found.push(ret.clone()); }
        effect.visit_child_effects(&mut |child| find(child, found));
    }
    let mut returns = Vec::new();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() { find(effect, &mut returns); }
        }
    }
    assert_eq!(returns.len(), 1, "{definition:#?}");
    assert!(returns[0].spec.count().is_random(), "{:?}", returns[0].spec);
    assert!(returns[0].spec.count().is_single());

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Exiled creature").card_types(vec![CardType::Creature]).build();
    let exiled: Vec<_> = (0..4).map(|_| game.create_object_from_card(&card, bob, Zone::Exile)).collect();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ctx.set_tagged_objects("random_pool", exiled.iter().map(|id| ObjectSnapshot::from_object_id(&game, *id).unwrap()).collect());
    let effect = Effect::new(ironsmith::effects::ReturnToHandEffect::with_spec(ChooseSpec::WithCount(
        Box::new(ChooseSpec::Tagged("random_pool".into())),
        ironsmith::effect::ChoiceCount::exactly(1).at_random(),
    )));
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(bob).unwrap().hand.len(), 1, "exactly one exiled card returns");
}

#[test]
fn trailing_condition_gates_source_exiled_play_permission() {
    let text = "Type: Enchantment\nWhenever a creature you control that entered this turn attacks, exile the top card of your library.\nYou may play cards exiled with this enchantment as long as you attacked this turn.";
    let trailing = compile_to_runtime_definition("Pick Up the Pace", text, false).unwrap();
    let leading = compile_to_runtime_definition("Pick Up the Pace", "Type: Enchantment\nWhenever a creature you control that entered this turn attacks, exile the top card of your library.\nAs long as you attacked this turn, you may play cards exiled with this enchantment.", false).unwrap();
    let statics = |definition: &ironsmith::CardDefinition| definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(ability) => Some(format!("{:?}", ability.compiled_model())),
        _ => None,
    }).collect::<Vec<_>>();
    let trailing_statics = statics(&trailing);
    assert_eq!(trailing_statics.len(), 1, "{trailing_statics:?}");
    assert!(trailing_statics[0].contains("Conditional") || trailing_statics[0].contains("condition"), "{}", trailing_statics[0]);
    assert!(trailing_statics[0].contains("Attack"), "{}", trailing_statics[0]);
    assert_eq!(trailing_statics, statics(&leading), "trailing and leading conditions share one permission");
}

#[test]
fn pump_before_color_change_keeps_one_target_and_three_children() {
    let definition = compile_to_runtime_definition("Traitor's Clutch",
        "Type: Instant\nTarget creature gets +1/+0, becomes black, and gains shadow until end of turn. (It can block or be blocked by only creatures with shadow.)\nFlashback {1}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)", false).unwrap();
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ModifyPowerToughness") || debug.contains("Pump"), "{debug}");
    fn has_black_modification(effect: &Effect) -> bool {
        let mut found = effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>().is_some_and(|effect| {
            effect.additional_modifications.iter().chain(effect.modification.iter()).any(|modification| matches!(modification, ironsmith::continuous::Modification::SetColors(colors) if *colors == ironsmith::color::ColorSet::BLACK))
        });
        effect.visit_child_effects(&mut |child| found |= has_black_modification(child));
        found
    }
    assert!(definition.spell_effect.as_ref().unwrap().all_effects().iter().any(|effect| has_black_modification(effect)), "{debug}");
    assert!(debug.contains("Shadow"), "{debug}");
    let spell = definition.spell_effect.as_ref().unwrap();
    let targets = spell.all_effects().iter().filter(|effect| effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some()).count();
    assert!(targets <= 1, "one announced target: {debug}");
    let alice = ironsmith::PlayerId::from_index(0);
    let bob = ironsmith::PlayerId::from_index(1);
    for owner in [alice, bob] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Color recipient")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .color_indicator(ironsmith::color::ColorSet::GREEN)
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3)).build();
        let target = game.create_object_from_card(&card, owner, ironsmith::Zone::Battlefield);
        let bystander = game.create_object_from_card(&card, owner, ironsmith::Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(source, alice).with_targets(vec![ironsmith::game_state::Target::Object(target)]));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.calculated_power(target), Some(3));
        assert_eq!(game.calculated_toughness(target), Some(3));
        assert_eq!(game.current_colors(target), Some(ironsmith::color::ColorSet::BLACK));
        assert!(game.current_has_static_ability_id(target, ironsmith::static_abilities::StaticAbilityId::Shadow));
        assert_eq!(game.calculated_power(bystander), Some(2));
        assert_eq!(game.current_colors(bystander), Some(ironsmith::color::ColorSet::GREEN));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.calculated_power(target), Some(2));
        assert_eq!(game.current_colors(target), Some(ironsmith::color::ColorSet::GREEN));
        assert!(!game.current_has_static_ability_id(target, ironsmith::static_abilities::StaticAbilityId::Shadow));
    }
}

#[test]
fn mana_value_guess_is_made_by_the_named_player_with_optional_fallback() {
    fn modes(definition: &ironsmith::CardDefinition) -> Vec<ironsmith::effects::ChooseModeEffect> {
        fn visit(effect: &Effect, found: &mut Vec<ironsmith::effects::ChooseModeEffect>) {
            if let Some(choose) = effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>() { found.push(choose.clone()); }
            effect.visit_child_effects(&mut |child| visit(child, found));
        }
        let mut found = Vec::new();
        for ability in &definition.abilities {
            if let ironsmith::ability::AbilityKind::Triggered(trigger) = &ability.kind {
                for effect in trigger.effects.all_effects() { visit(effect, &mut found); }
            }
        }
        found
    }
    let master = compile_to_runtime_definition("Master of Predicaments",
        "Type: Creature — Sphinx\nFlying\nWhenever this creature deals combat damage to a player, choose a card in your hand. That player guesses whether the card's mana value is greater than 4. If the player guessed wrong, you may cast the card without paying its mana cost.", false).unwrap();
    let doctor = compile_to_runtime_definition("The Seventh Doctor",
        "Type: Legendary Creature — Time Lord Doctor\nWhenever The Seventh Doctor attacks, choose a card in your hand. Defending player guesses whether that card's mana value is greater than the number of artifacts you control. If they guessed wrong, you may cast it without paying its mana cost. If you don't cast a spell this way, investigate.", false).unwrap();
    for (definition, chooser, investigates) in [
        (&master, ironsmith::target::PlayerFilter::DamagedPlayer, false),
        (&doctor, ironsmith::target::PlayerFilter::Defending, true),
    ] {
        let found = modes(definition);
        assert_eq!(found.len(), 1, "{definition:#?}");
        assert_eq!(found[0].chooser.as_ref(), Some(&chooser));
        assert_eq!(found[0].modes.len(), 2);
        let debug = format!("{definition:?}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        assert_eq!(debug.contains("Investigate"), investigates);
    }
}

#[test]
fn exact_color_count_spell_filter_reads_with_link() {
    let definition = compile_to_runtime_definition("Meeting of the Five",
        "Mana cost: {2}{W}{U}{B}{R}{G}\nType: Sorcery\nExile the top ten cards of your library. You may cast spells with exactly three colors from among them this turn. Add {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}. Spend this mana only to cast spells with exactly three colors.", false).unwrap();
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.matches("color_count: Some(Equal(3))").count() >= 2, "permission and mana restriction: {debug}");
}

#[test]
fn targeted_damaged_player_graveyard_cast_keeps_target_and_any_type_mana() {
    let definition = compile_to_runtime_definition("Tinybones, the Pickpocket",
        "Type: Legendary Creature — Skeleton Rogue\nDeathtouch\nWhenever Tinybones deals combat damage to a player, you may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("mana_spend_mode: AnyType"), "{debug}");
    assert!(debug.contains("DamagedPlayer") || debug.contains("IteratedPlayer"), "graveyard owner is the damaged player: {debug}");
}

#[test]
fn self_graveyard_cast_accepts_general_gerund_additional_payment() {
    let definition = compile_to_runtime_definition("Quilled Greatwurm",
        "Mana cost: {4}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 7/7\nTrample\nWhenever a creature you control deals combat damage during your turn, put that many +1/+1 counters on it. (It must survive to get the counters.)\nYou may cast this card from your graveyard by removing six counters from among creatures you control in addition to paying its other costs.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("RemoveAnyCountersAmong") || debug.contains("RemoveCountersAmong") || debug.contains("RemoveCounters"), "{debug}");
}

#[test]
fn as_long_as_otherwise_static_pair_uses_condition_and_negation() {
    let definition = compile_to_runtime_definition("Mishra's Domination",
        "Mana cost: {R}\nType: Enchantment — Aura\nEnchant creature\nAs long as you control enchanted creature, it gets +2/+2. Otherwise, it can't block.", false).unwrap();
    let mut anthem_condition = None;
    let mut restriction_condition = None;
    for ability in &definition.abilities {
        let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else { continue };
        let Some(model) = ability.compiled_model() else { continue };
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition, .. }
                if matches!(&ability.payload, ironsmith_core::StaticAbilityPayload::Anthem(_)) => anthem_condition = Some(condition),
            ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(grant) => {
                let ironsmith_core::AbilityKind::Static(ability) = &grant.ability.kind else { continue };
                if matches!(&ability.payload, ironsmith_core::StaticAbilityPayload::RuleRestriction { restriction: ironsmith::effect::Restriction::Block(_), .. }) {
                    restriction_condition = grant.condition.as_ref();
                }
            }
            _ => {}
        }
    }
    let anthem = anthem_condition.expect("conditional +2/+2");
    let restriction = restriction_condition.expect("conditional blocking restriction");
    assert!(matches!(restriction, ironsmith::effect::Condition::Not(inner) if inner.as_ref() == anthem), "otherwise must negate exactly the anthem condition");
}

#[test]
fn unless_damage_follow_up_reads_damage_dealt_to_you() {
    let definition = compile_to_runtime_definition("Minion of Leshrac",
        "Mana cost: {4}{B}\nType: Creature — Demon Minion\nPower/Toughness: 5/5\nProtection from black\nAt the beginning of your upkeep, this creature deals 5 damage to you unless you sacrifice a creature other than this creature. If this creature deals damage to you this way, tap it.\n{T}: Destroy target creature or land.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("DealtDamageToPlayer"), "{debug}");
    assert!(debug.contains("TapEffect"), "{debug}");
}

#[test]
fn creature_type_choice_in_addition_keeps_other_types() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Mistform Sliver",
        "Mana cost: {1}{U}\nType: Creature — Illusion Sliver\nPower/Toughness: 1/1\nAll Slivers have \"{1}: This permanent becomes the creature type of your choice in addition to its other types until end of turn.\"", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("in_addition_to_other_types: true"), "{debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Sliver fixture").card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sliver]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let sliver = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let effect = Effect::new(ironsmith::effects::BecomeCreatureTypeChoiceEffect::new(
        ironsmith::target::ChooseSpec::SpecificObject(sliver), ironsmith::effect::Until::EndOfTurn, Vec::new(),
    ).in_addition_to_other_types(true));
    let mut ctx = ironsmith::effects::EffectContext::new_default(sliver, alice);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    drop(ctx);
    game.refresh_continuous_state().unwrap();
    let subtypes = game.current_subtypes(sliver).unwrap_or_default();
    assert!(subtypes.contains(&Subtype::Sliver), "keeps its other creature types: {subtypes:?}");
    assert!(subtypes.len() >= 2, "gains the chosen type: {subtypes:?}");
}

#[test]
fn delayed_return_unless_the_exiled_card_was_cast() {
    for (name, text) in [
        ("Psychic Theft", "Mana cost: {1}{U}\nType: Sorcery\nTarget player reveals their hand. You choose an instant or sorcery card from it and exile that card. You may cast that card for as long as it remains exiled. At the beginning of the next end step, if you haven't cast the card, return it to its owner's hand."),
        ("Planeswalker's Mischief", "Mana cost: {2}{U}\nType: Enchantment\n{3}{U}: Target opponent reveals a card at random from their hand. If it's an instant or sorcery card, exile it. You may cast it without paying its mana cost for as long as it remains exiled. At the beginning of the next end step, if you haven't cast it, return it to its owner's hand. Activate only as a sorcery."),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        let debug = format!("{definition:?}");
        assert!(debug.contains("TaggedObjectWasCast"), "{name}: {debug}");
        assert!(debug.contains("ReturnToHandEffect") || debug.contains("MoveToZoneEffect"), "{name}: {debug}");
    }
}

#[test]
fn twice_that_much_damage_is_a_doubling_replacement() {
    let definition = compile_to_runtime_definition("Neriv, Heart of the Storm",
        "Mana cost: {1}{R}{W}{B}\nType: Legendary Creature — Spirit Dragon\nPower/Toughness: 4/5\nFlying\nIf a creature you control that entered this turn would deal damage, it deals twice that much damage instead.", false).unwrap();
    let statics = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(ability) => Some(format!("{:?}", ability.compiled_model())),
        _ => None,
    }).collect::<Vec<_>>();
    let multiplier = statics.iter().find(|debug| debug.contains("factor: 2")).unwrap_or_else(|| panic!("{statics:?}"));
    assert!(multiplier.contains("entered_battlefield_this_turn: true"), "{multiplier}");
}

#[test]
fn coordinated_ability_counter_target_keeps_controller_tail() {
    let definition = compile_to_runtime_definition("Nimble Obstructionist",
        "Mana cost: {2}{U}\nType: Creature — Bird Wizard\nPower/Toughness: 3/1\nFlash\nFlying\nCycling {2}{U} ({2}{U}, Discard this card: Draw a card.)\nWhen you cycle this card, counter target activated or triggered ability you don't control.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ActivatedAbility") && debug.contains("TriggeredAbility"), "{debug}");
    assert!(debug.contains("controller: Some(\n") || debug.contains("NotYou"), "{debug}");
}

#[test]
fn any_target_dealt_damage_this_turn_restricts_players_and_permanents() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::target::ChooseSpec;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Needle Drop",
        "Mana cost: {R}\nType: Instant\nNeedle Drop deals 1 damage to any target that was dealt damage this turn.\nDraw a card.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("WasDealtDamage"), "player domain carries the damage history: {debug}");
    assert!(debug.contains("was_dealt_damage_this_turn: true"), "object domain carries the damage history: {debug}");
    fn damage_spec(effect: &Effect) -> Option<ChooseSpec> {
        if let Some(damage) = effect.downcast_ref::<ironsmith::effects::DealDamageEffect>() {
            return Some(damage.target.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() { found = damage_spec(child); }
        });
        found
    }
    let spec = definition.spell_effect.as_ref().unwrap().all_effects().into_iter()
        .find_map(damage_spec).expect("damage target specification");
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let creature = CardBuilder::new(CardId::new(), "Damage recipient")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4)).build();
    let mut eligible = Vec::new();
    let mut ineligible = Vec::new();
    for kind in [CardType::Creature, CardType::Planeswalker, CardType::Battle, CardType::Artifact, CardType::Enchantment, CardType::Land] {
        for damaged in [false, true] {
            let id = game.create_object_from_card(&creature, bob, Zone::Battlefield);
            if damaged {
                let mut ctx = EffectContext::new_default(source, alice);
                execute_effect(&mut game, &Effect::deal_damage(1, ChooseSpec::SpecificObject(id)), &mut ctx).unwrap();
            }
            // A damaged creature can stop being a creature during the turn;
            // damage history must not make its new artifact/land type legal.
            game.object_mut(id).unwrap().card_types = vec![kind].into();
            if damaged && matches!(kind, CardType::Creature | CardType::Planeswalker | CardType::Battle) {
                eligible.push(id);
            } else {
                ineligible.push(id);
            }
        }
    }
    let mut ctx = EffectContext::new_default(source, alice);
    execute_effect(&mut game, &Effect::deal_damage(1, ChooseSpec::SpecificPlayer(bob)), &mut ctx).unwrap();
    let legal = ironsmith::targeting::compute_legal_targets(&game, &spec, alice, Some(source));
    for id in &eligible { assert!(legal.contains(&ironsmith::game_state::Target::Object(*id)), "damaged damageable object {id:?}"); }
    for id in ineligible { assert!(!legal.contains(&ironsmith::game_state::Target::Object(id)), "ineligible object {id:?}"); }
    assert!(legal.contains(&ironsmith::game_state::Target::Player(bob)));
    assert!(!legal.contains(&ironsmith::game_state::Target::Player(alice)));
    let drawn = CardBuilder::new(CardId::new(), "Draw fixture").build();
    game.create_object_from_card(&drawn, alice, Zone::Library);
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(eligible[0])]);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.damage_on(eligible[0]), 2);
    assert_eq!(game.player(alice).unwrap().hand_size(), 1);
}

#[test]
fn unblockable_while_defending_player_has_rad_counter() {
    let definition = compile_to_runtime_definition("Nightkin Ambusher",
        "Mana cost: {2}{U}\nType: Creature — Mutant Warrior\nPower/Toughness: 3/3\nWard {2}\nWhen this creature enters, target player gets four rad counters.\nThis creature can't be blocked as long as defending player has a rad counter.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("PlayerHasCountersOrMore"), "{debug}");
    assert!(debug.contains("Defending"), "{debug}");
    assert!(debug.contains("Rad"), "{debug}");
}

#[test]
fn land_quality_union_search_and_distinct_name_gate() {
    let definition = compile_to_runtime_definition("Monument to Perfection",
        "Mana cost: {2}\nType: Artifact\n{3}, {T}: Search your library for a basic, Sphere, or Locus land card, reveal it, put it into your hand, then shuffle.\n{3}: This artifact becomes a 9/9 Phyrexian Construct artifact creature, loses all abilities, and gains indestructible and toxic 9. Activate only if there are nine or more lands with different names among the basic, Sphere, and Locus lands you control.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("Sphere") && debug.contains("Locus") && debug.contains("Basic"), "{debug}");
    assert!(debug.contains("DistinctNames"), "{debug}");
}

#[test]
fn delayed_graveyard_put_of_unplayed_exiled_cards() {
    let definition = compile_to_runtime_definition("Three Wishes",
        "Mana cost: {1}{U}{U}\nType: Instant\nExile the top three cards of your library face down. You may look at those cards for as long as they remain exiled. Until your next turn, you may play those cards. At the beginning of your next upkeep, put any of those cards you didn't play into your graveyard.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("Delayed") || debug.contains("delayed"), "{debug}");
}

#[test]
fn graveyard_landfall_grants_self_cast_this_turn() {
    let definition = compile_to_runtime_definition("Skyclave Shade",
        "Mana cost: {1}{B}\nType: Creature — Shade\nPower/Toughness: 3/1\nKicker {2}{B}\nThis creature can't block.\nIf this creature was kicked, it enters with two +1/+1 counters on it.\nLandfall — Whenever a land you control enters, if this card is in your graveyard and it's your turn, you may cast it from your graveyard this turn.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("functional_zones: [\n                Graveyard") || debug.contains("functional_zones: [Graveyard"), "{debug}");
    assert!(debug.contains("GrantBySpecEffect") && debug.contains("source: true"), "{debug}");
    assert!(debug.contains("SourceIsInZone"), "{debug}");
}

#[test]
fn escape_entry_trigger_is_gated_on_escaping() {
    let definition = compile_to_runtime_definition("Pharika's Spawn",
        "Mana cost: {3}{B}\nType: Creature — Gorgon\nPower/Toughness: 3/3\nEscape—{5}{B}, Exile three other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)\nThis creature escapes with two +1/+1 counters on it. When it enters this way, each opponent sacrifices a non-Gorgon creature of their choice.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ThisSpellEscaped"), "{debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    let triggers = definition.abilities.iter().filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))).count();
    assert_eq!(triggers, 1, "{debug}");
}

#[test]
fn escape_counter_choice_is_conditioned_on_escaping() {
    let definition = compile_to_runtime_definition("Tizerus Charger",
        "Mana cost: {2}{B}\nType: Creature — Pegasus\nPower/Toughness: 2/2\nEscape—{4}{B}, Exile five other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)\nThis creature escapes with your choice of a +1/+1 counter or a flying counter on it.", false).unwrap();
    let choice = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(ability) => match ability.compiled_model().map(|model| &model.payload) {
            Some(ironsmith_core::StaticAbilityPayload::EntersWithCounterChoice { counter_types, condition, .. }) => Some((counter_types.clone(), condition.clone())),
            _ => None,
        },
        _ => None,
    }).expect("counter choice static");
    assert_eq!(choice.0.len(), 2);
    assert!(matches!(choice.1, Some(ironsmith::effect::Condition::ThisSpellEscaped)), "{:?}", choice.1);
}

#[test]
fn cast_this_way_exile_rider_is_bound_to_the_permitted_spell() {
    let definition = compile_to_runtime_definition("Radiant Scrollwielder",
        "Mana cost: {2}{R}{W}\nType: Creature — Dwarf Cleric\nPower/Toughness: 2/4\nInstant and sorcery spells you control have lifelink.\nAt the beginning of your upkeep, exile an instant or sorcery card at random from your graveyard. You may cast it this turn. If a spell cast this way would be put into your graveyard, exile it instead.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("RegisterFutureZoneReplacementEffect"), "{debug}");
    assert!(debug.contains("cast_this_way_surface: true"), "{debug}");
}

#[test]
fn cast_trigger_replaces_resolution_destination_and_links_exile() {
    let definition = compile_to_runtime_definition("River Song's Diary",
        "Mana cost: {3}\nType: Artifact — Book\nImprint — Whenever a player casts an instant or sorcery spell from their hand, exile it instead of putting it into a graveyard as it resolves.\nAt the beginning of your upkeep, if there are four or more cards exiled with this artifact, choose one of them at random. You may cast it without paying its mana cost.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("RegisterFutureZoneReplacementEffect"), "{debug}");
    assert!(!debug.contains("ExileEffect"), "the spell is not exiled on cast: {debug}");
}

#[test]
fn revealed_free_cast_requires_matching_mana_value() {
    let definition = compile_to_runtime_definition("Powerbalance",
        "Mana cost: {2}{R}\nType: Enchantment\nWhenever an opponent casts a spell, you may reveal the top card of your library. If you do, you may cast that card without paying its mana cost if the two spells have the same mana value.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
}

#[test]
fn counter_when_no_milled_card_reaches_hand() {
    let definition = compile_to_runtime_definition("Nashi, Searcher in the Dark",
        "Mana cost: {U}{B}\nType: Legendary Creature — Rat Ninja Wizard\nPower/Toughness: 2/2\nMenace\nWhenever Nashi deals combat damage to a player, you mill that many cards. You may put any number of legendary and/or enchantment cards from among them into your hand. If you put no cards into your hand this way, put a +1/+1 counter on Nashi.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("PutIntoHand"), "{debug}");
    assert!(!debug.contains("name: Some("), "no literal card name filter: {debug}");
}

#[test]
fn dies_trigger_grants_adventure_face_cast_until_next_turn() {
    let definition = compile_to_runtime_definition("Mosswood Dreadknight",
        "Mana cost: {1}{G}\nType: Creature — Human Knight\nPower/Toughness: 3/2\nTrample\nWhen this creature dies, you may cast it from your graveyard as an Adventure until the end of your next turn.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.contains("UntilYourNextTurnEnd"), "{debug}");
    assert!(debug.contains("Adventure"), "{debug}");
}

#[test]
fn another_token_for_each_poisoned_opponent() {
    let definition = compile_to_runtime_definition("Wurmquake",
        "Mana cost: {4}{G}{G}\nType: Sorcery\nCorrupted — Create an X/X green Phyrexian Wurm creature token with trample and toxic 1, where X is the amount of mana spent to cast this spell. Then for each opponent with three or more poison counters, you create another one of those tokens.\nFlashback {8}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)", false).unwrap();
    let debug = format!("{:?}", definition.spell_effect);
    assert_eq!(debug.matches("CreateTokenEffect").count(), 2, "{debug}");
    assert!(debug.contains("PlayerHasPoisonCountersOrMore"), "{debug}");
    assert!(debug.contains("ForEachOpponent") || debug.contains("ForPlayersEffect") || debug.contains("ForEach"), "{debug}");
}

#[test]
fn commander_cost_reduction_scales_with_command_zone_casts() {
    let definition = compile_to_runtime_definition("Myth Unbound",
        "Mana cost: {2}{G}\nType: Enchantment\nYour commander costs {1} less to cast for each time it's been cast from the command zone this game.\nWhenever your commander is put into the command zone from anywhere, draw a card.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("CommanderCastCount"), "{debug}");
    assert!(debug.contains("is_commander: true"), "{debug}");
}

#[test]
fn threshold_permission_over_the_etb_exiled_card() {
    let definition = compile_to_runtime_definition("Null Summoner",
        "Mana cost: {2}{B}{B}\nType: Creature — Human Warlock\nPower/Toughness: 3/3\nWhen this creature enters, if you cast it, target opponent reveals their hand. You choose a nonland card from it. Exile that card.\nThreshold — As long as there are seven or more cards in your graveyard, you may cast the exiled card, and mana of any type can be spent to cast that spell.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("PlayFrom"), "{debug}");
    assert!(debug.contains("AnyType"), "{debug}");
    assert!(debug.contains("__source_exiled__") || debug.contains("SourceExiled"), "{debug}");
}

#[test]
fn filtered_graveyard_cast_with_life_cost_and_finality_rider() {
    let definition = compile_to_runtime_definition("Noctis, Prince of Lucis",
        "Mana cost: {1}{W}{U}{B}\nType: Legendary Creature — Human Noble\nPower/Toughness: 4/3\nLifelink\nYou may cast artifact spells from your graveyard by paying 3 life in addition to paying their other costs. If you cast a spell this way, that artifact enters with a finality counter on it.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("GraveyardCastFromCardManaCost"), "{debug}");
    assert!(debug.contains("PayLifeEffect { amount: Fixed(3)"), "{debug}");
    assert!(debug.contains("Finality"), "{debug}");
}

#[test]
fn villainous_choice_draw_then_chaos_ensues() {
    let definition = compile_to_runtime_definition("Missy",
        "Mana cost: {2}{U}{B}\nType: Legendary Creature — Time Lord Rogue\nPower/Toughness: 4/4\nWhenever another nonartifact creature dies, return it to the battlefield under your control face down and tapped. It's a 2/2 Cyberman artifact creature.\nAt the beginning of your end step, each opponent faces a villainous choice — Each artifact creature you control deals 1 damage to that opponent, or you draw a card and chaos ensues.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ChaosEnsues"), "{debug}");
    assert!(debug.contains("VillainousChoice"), "{debug}");
}
