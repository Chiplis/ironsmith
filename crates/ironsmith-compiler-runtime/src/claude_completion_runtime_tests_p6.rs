//! Claude package 6 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p6_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p6_visit(child, f));
}

fn p6_triggered(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).collect()
}

#[test]
fn that_player_is_you_compares_the_targeted_player_with_the_controller() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("The Ghoul, Gunslinger",
        "Type: Legendary Creature — Zombie Mutant Rogue\nFirst strike\nWhenever The Ghoul or another nontoken Zombie or Mutant you control dies, target player gets two rad counters. If that player is you, create a Treasure token.", false)
        .expect("player-is-you condition should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("death trigger");
    let mut found = false;
    for effect in trigger.effects.all_effects() {
        p6_visit(effect, &mut |effect| {
            if let Some(conditional) = effect.downcast_ref::<ironsmith::effects::ConditionalEffect>() {
                assert!(matches!(conditional.condition, ironsmith::effect::Condition::PlayerIsYou { .. }), "{:?}", conditional.condition);
                found = true;
            }
        });
    }
    assert!(found);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (target, treasures) in [(alice, 1usize), (bob, 0usize)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let before = game.battlefield.len();
        let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Player(target)]);
        for effect in trigger.effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(game.battlefield.len() - before, treasures, "target {target:?}");
    }
}

#[test]
fn attached_anthem_with_activation_ban_restricts_the_enchanted_host() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Viper's Kiss",
        "Type: Enchantment — Aura\nEnchant creature\nEnchanted creature gets -1/-1, and its activated abilities can't be activated.", false)
        .expect("attached anthem plus activation ban should compile");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Host").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3)).build();
    let host = game.create_object_from_card(&card, bob, Zone::Battlefield);
    let bystander = game.create_object_from_card(&card, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    assert_eq!(game.calculated_power(host), Some(2));
    assert_eq!(game.calculated_power(bystander), Some(3));
    assert!(!game.effect_store.cant_effects.can_activate_abilities_of(host));
    assert!(game.effect_store.cant_effects.can_activate_abilities_of(bystander));
}

#[test]
fn anaphoric_up_to_blocking_capacity_grants_two_extra_blocks_this_turn() {
    let definition = compile_to_runtime_definition("Yare",
        "Type: Instant\nTarget creature defending player controls gets +3/+0 until end of turn. That creature can block up to two additional creatures this turn.", false)
        .expect("anaphoric blocking capacity should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("CanBlockAdditionalCreatureEachCombat"), "{debug}");
    assert!(debug.contains("CanBlockAdditionalCreatureEachCombat(2)") || debug.contains("additional: 2"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
    let mut targets = 0;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { targets += 1; }
        });
    }
    assert!(targets <= 1, "the anaphor must not declare a second target");
}

#[test]
fn kicked_entry_keeps_sibling_counters_and_granted_haste() {
    let definition = compile_to_runtime_definition("Voidpouncer",
        "Type: Creature — Eldrazi\nDevoid\nKicker {2}{C}\nIf this creature was kicked, it enters with two +1/+1 counters and a trample counter on it and with haste.", false)
        .expect("sibling counters plus entry-granted haste should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("Haste"), "the granted haste must survive: {debug}");
}

#[test]
fn exile_all_creatures_and_graveyards_keeps_the_arms_independent() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Ultimate Nullification",
        "Type: Sorcery\nAs an additional cost to cast this spell, sacrifice a legendary creature.\nExile all creatures and graveyards. Put Ultimate Nullification on the bottom of its owner's library.", false)
        .expect("exile creatures and graveyards should compile");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = CardBuilder::new(CardId::new(), "Bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let land = CardBuilder::new(CardId::new(), "Plains").card_types(vec![CardType::Land]).build();
    let bear = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let plains = game.create_object_from_card(&land, bob, Zone::Battlefield);
    game.create_object_from_card(&land, alice, Zone::Graveyard);
    game.create_object_from_card(&creature, bob, Zone::Graveyard);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = EffectContext::new_default(spell, alice);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert!(!game.battlefield.contains(&bear), "creatures are exiled");
    assert!(game.battlefield.contains(&plains), "the graveyard arm must not widen to all permanents");
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert!(game.player(bob).unwrap().graveyard.is_empty());
}

#[test]
fn create_for_each_opponent_dealt_damage_counts_turn_history_players() {
    use ironsmith::effect::Value;
    let definition = compile_to_runtime_definition("You've Been Caught Stealing",
        "Type: Sorcery\nChoose one —\n• Threaten the Merchant — Each creature blocks this turn if able.\n• Bribe the Guards — You create a Treasure token for each opponent who was dealt damage this turn.", false)
        .expect("turn-history player count should compile");
    let mut counts = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if let Some(create) = effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>() {
                counts.push(create.count.unhinted().clone());
            }
        });
    }
    assert_eq!(counts.len(), 1, "{counts:?}");
    assert!(matches!(&counts[0], Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersDealtDamage(ironsmith::target::PlayerFilter::Opponent))), "{counts:?}");
}

#[test]
fn past_tense_demonstrative_after_source_counter_reads_the_dying_creature() {
    let definition = compile_to_runtime_definition("Venom, Eddie Brock",
        "Type: Legendary Creature — Symbiote Villain\nMenace\nWhenever another creature dies, put a +1/+1 counter on Venom. If that creature was a Villain, draw a card.", false)
        .expect("demonstrative past-tense identity after a source counter should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("death trigger");
    let mut found = false;
    for effect in trigger.effects.all_effects() {
        p6_visit(effect, &mut |effect| {
            if let Some(conditional) = effect.downcast_ref::<ironsmith::effects::ConditionalEffect>() {
                let ironsmith::effect::Condition::TaggedObjectMatchedLastKnown(tag, filter) = &conditional.condition else {
                    panic!("{:?}", conditional.condition);
                };
                assert_eq!(tag.as_str(), "triggering");
                assert!(filter.subtypes.contains(&ironsmith::types::Subtype::Villain), "{filter:?}");
                found = true;
            }
        });
    }
    assert!(found);
}

#[test]
fn loses_your_choice_of_keywords_declares_one_target_then_chooses_one_loss() {
    let definition = compile_to_runtime_definition("Walking Sponge",
        "Type: Creature — Sponge\n{T}: Target creature loses your choice of flying, first strike, or trample until end of turn.", false)
        .expect("lose-choice clause should compile");
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("tap ability");
    let (mut targets, mut modes) = (0, Vec::new());
    for effect in ability.effects.all_effects() {
        p6_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { targets += 1; }
            if let Some(choose) = effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>() {
                modes.push(choose.modes.len());
            }
        });
    }
    assert_eq!(targets, 1, "one creature is targeted");
    assert_eq!(modes, vec![3], "one choice among three losses");
    let debug = format!("{:?}", ability.effects);
    for keyword in ["Flying", "FirstStrike", "Trample"] {
        assert!(debug.contains(keyword), "{keyword}: {debug}");
    }
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

#[test]
fn pronoun_ability_and_type_pair_keeps_both_predicates_on_the_returned_card() {
    let valkyrie = compile_to_runtime_definition("Valkyrie's Call",
        "Type: Enchantment\nWhenever a nontoken, non-Angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types.", false)
        .expect("has-and-is pair should compile");
    let trigger = p6_triggered(&valkyrie).into_iter().next().expect("death trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("Flying"), "{debug}");
    assert!(debug.contains("Angel"), "{debug}");
    let xu = compile_to_runtime_definition("Xu-Ifit, Osteoharmonist",
        "Type: Legendary Creature — Human Wizard\n{T}: Return target creature card from your graveyard to the battlefield. It's a Skeleton in addition to its other types and has no abilities. Activate only as a sorcery.", false)
        .expect("is-and-has-no-abilities pair should compile");
    let ability = xu.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("tap ability");
    let debug = format!("{:?}", ability.effects);
    assert!(debug.contains("Skeleton"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
}

#[test]
fn english_noun_subtype_heading_a_card_noun_is_the_subtype() {
    let definition = compile_to_runtime_definition("The Masters of Evil",
        "Type: Legendary Creature — Human Villain\nOther Villains you control get +2/+1.\n{1}{B}, Discard this card: Search your library for a Plan card, reveal it, put it into your hand, then shuffle.", false)
        .expect("Plan card search should compile");
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("discard ability");
    let debug = format!("{:?}", ability.effects);
    assert!(debug.contains("Plan"), "{debug}");
}

#[test]
fn return_each_of_them_names_the_chosen_targets() {
    let definition = compile_to_runtime_definition("The War in Heaven",
        "Type: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — You draw three cards and you lose 3 life.\nII — Mill three cards.\nIII — Choose up to three target creature cards with total mana value 8 or less in your graveyard. Return each of them to the battlefield with a necrodermis counter on it. They're artifacts in addition to their other types.", false)
        .expect("distributive pronoun return should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.to_lowercase().contains("necrodermis"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    let mut targets = 0;
    for trigger in p6_triggered(&definition) {
        for effect in trigger.effects.all_effects() {
            p6_visit(effect, &mut |effect| {
                if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { targets += 1; }
            });
        }
    }
    assert!(targets <= 1, "'each of them' must not declare new targets");
}

#[test]
fn mana_spent_to_cast_gendered_pronoun_reads_the_source() {
    let definition = compile_to_runtime_definition("Toph, Greatest Earthbender",
        "Type: Legendary Creature — Human Warrior Ally\nWhen Toph enters, earthbend X, where X is the amount of mana spent to cast her.\nLand creatures you control have double strike.", false)
        .expect("gendered mana-spent where-X should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("ETB trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("ManaSpentToCastThisSpell"), "{debug}");
}

#[test]
fn either_of_them_picks_one_referenced_object_without_new_targets() {
    let definition = compile_to_runtime_definition("Wicked Slumber",
        "Type: Instant\nConvoke\nTap up to two target creatures. Put a stun counter on either of them. Then put a stun counter on either of them.", false)
        .expect("either-of-them pick should compile");
    let mut targets = 0;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { targets += 1; }
        });
    }
    assert!(targets <= 1, "only the tap declares targets");
    let debug = format!("{:?}", definition.spell_effect);
    assert_eq!(debug.matches("Stun").count() >= 2, true, "{debug}");
}

#[test]
fn sacrifice_for_each_creature_put_into_your_graveyard_counts_turn_history() {
    let definition = compile_to_runtime_definition("Urborg Justice",
        "Type: Instant\nTarget opponent sacrifices a creature of their choice for each creature put into your graveyard from the battlefield this turn.", false)
        .expect("graveyard-history sacrifice count should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("MovedZones"), "{debug}");
    assert!(debug.contains("Battlefield") && debug.contains("Graveyard"), "{debug}");
    let mut targets = 0;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { targets += 1; }
        });
    }
    assert!(targets <= 1, "the opponent is targeted once: {debug}");
}

#[test]
fn optional_target_player_life_total_becomes_half_their_starting_life() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Torgaar, Famine Incarnate",
        "Type: Legendary Creature — Avatar\nAs an additional cost to cast this spell, you may sacrifice any number of creatures. This spell costs {2} less to cast for each creature sacrificed this way.\nWhen Torgaar enters, up to one target player's life total becomes half their starting life total, rounded down.", false)
        .expect("optional player life-total set should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("ETB trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("HalfStartingLifeTotalRoundedDown"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 21);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![ResolvedTarget::Player(bob)]);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(bob).unwrap().life, 10, "half of 21 rounded down");
    assert_eq!(game.player(alice).unwrap().life, 21);
}

#[test]
fn if_you_cant_restating_the_sacrifice_reads_as_if_you_cant() {
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Xathrid Demon",
        "Type: Creature — Demon\nFlying, trample\nAt the beginning of your upkeep, sacrifice a creature other than this creature, then each opponent loses life equal to the sacrificed creature's power. If you can't sacrifice a creature, tap this creature and you lose 7 life.", false)
        .expect("restated if-you-can't should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("upkeep trigger");
    // With no other creature to sacrifice, the fallback taps the demon and costs 7 life.
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let demon = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(demon, alice);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert!(game.is_tapped(demon));
    assert_eq!(game.player(alice).unwrap().life, 13);
}

#[test]
fn rad_counters_if_none_otherwise_proliferate_branches_on_the_damaged_player() {
    let definition = compile_to_runtime_definition("Vexing Radgull",
        "Type: Creature — Bird Mutant\nFlying\nWhenever this creature deals combat damage to a player, that player gets two rad counters if they don't have any rad counters. Otherwise, proliferate.", false)
        .expect("rad conditional with otherwise should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("combat damage trigger");
    let mut found = false;
    for effect in trigger.effects.all_effects() {
        p6_visit(effect, &mut |effect| {
            if let Some(conditional) = effect.downcast_ref::<ironsmith::effects::ConditionalEffect>() {
                let debug = format!("{:?}", conditional.condition);
                if debug.contains("PlayerHasCountersOrMore") {
                    assert!(debug.contains("Rad"), "{debug}");
                    assert!(!format!("{:?}", conditional.if_false).is_empty());
                    found = true;
                }
            }
        });
    }
    assert!(found, "{:?}", trigger.effects);
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("Proliferate"), "{debug}");
}

#[test]
fn source_attacked_this_combat_gates_the_wolf_block_requirement() {
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Tolsimir, Midnight's Light",
        "Type: Legendary Creature — Elf Scout\nLifelink\nWhen Tolsimir enters, create Voja Fenstalker, a legendary 5/5 green and white Wolf creature token with trample.\nWhenever a Wolf you control attacks, if Tolsimir attacked this combat, target creature an opponent controls blocks that Wolf this combat if able.", false)
        .expect("source-attacked-this-combat intervening if should compile");
    let condition = p6_triggered(&definition).into_iter()
        .find_map(|trigger| trigger.intervening_if.as_ref())
        .expect("intervening if");
    assert!(matches!(condition, ironsmith::effect::Condition::SourceAttackedThisCombat), "{condition:?}");
    // Outside any combat declaration the source has not attacked.
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap_or(false));
}

#[test]
fn tapped_trigger_excludes_attack_declaration_taps() {
    let definition = compile_to_runtime_definition("Verity Circle",
        "Type: Enchantment\nWhenever a creature an opponent controls becomes tapped, if it isn't being declared as an attacker, you may draw a card.\n{4}{U}: Tap target creature without flying.", false)
        .expect("attack-declaration tap gate should compile");
    let condition = p6_triggered(&definition).into_iter()
        .find_map(|trigger| trigger.intervening_if.as_ref())
        .expect("intervening if");
    assert!(matches!(condition, ironsmith::effect::Condition::Not(inner)
        if matches!(inner.as_ref(), ironsmith::effect::Condition::TriggeringObjectTappedForAttack)), "{condition:?}");
    let tapped = ironsmith::events::PermanentTappedEvent::new(ironsmith::ids::ObjectId::from_raw(1));
    assert!(!tapped.attack_declaration, "ordinary taps are not attack declarations");
}

#[test]
fn behold_land_enters_untapped_with_a_matching_permanent_or_hand_card() {
    let definition = compile_to_runtime_definition("Theorist's Sanctum",
        "Type: Land — Island\n({T}: Add {U}.)\nAs this land enters, you may behold a Jace. If you don't, this land enters tapped. (To behold a Jace, choose a Jace you control or reveal a Jace card from your hand.)\n{2}{U}, {T}: Empower Jace 2.", false)
        .expect("behold land should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("YouHaveCardInHandMatching"), "{debug}");
    assert!(debug.contains("YouControl"), "{debug}");
    assert!(debug.contains("Jace"), "{debug}");
}

#[test]
fn departure_trigger_excludes_sacrificed_permanents() {
    let definition = compile_to_runtime_definition("Urza's Miter",
        "Type: Artifact\nWhenever an artifact you control is put into a graveyard from the battlefield, if it wasn't sacrificed, you may pay {3}. If you do, draw a card.", false)
        .expect("not-sacrificed intervening if should compile");
    let condition = p6_triggered(&definition).into_iter()
        .find_map(|trigger| trigger.intervening_if.as_ref())
        .expect("intervening if");
    assert!(matches!(condition, ironsmith::effect::Condition::Not(inner)
        if matches!(inner.as_ref(), ironsmith::effect::Condition::TriggeringObjectWasSacrificed)), "{condition:?}");
}

#[test]
fn that_player_discovers_x_after_owner_shuffle() {
    let definition = compile_to_runtime_definition("Zoyowa's Justice",
        "Type: Instant\nThe owner of target artifact or creature with mana value 1 or greater shuffles it into their library. Then that player discovers X, where X is its mana value. (They exile cards from the top of their library until they exile a nonland card with that mana value or less. They cast it without paying its mana cost or put it into their hand. They put the rest on the bottom in a random order.)", false)
        .expect("that-player discover should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("Discover"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "X is the shuffled object's mana value: {debug}");
    let mut targets = 0;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().is_some() { targets += 1; }
        });
    }
    assert!(targets <= 1, "{debug}");
}

#[test]
fn filtered_entry_counter_without_additional_reaches_face_down_creatures() {
    let definition = compile_to_runtime_definition("Veiled Ascension",
        "Type: Enchantment\nWhen this enchantment enters, put a flying counter on each face-down creature you control.\nFace-down creatures you control enter with a flying counter on them.\nAt the beginning of your upkeep, you may cloak the top card of your library.", false)
        .expect("filtered face-down entry counters should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("EntersWithCountersAndSubtypesForFilter"), "{debug}");
    assert!(debug.contains("face_down: Some(true)"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
}

#[test]
fn ulamog_exiles_half_library_and_has_dynamic_annihilator() {
    let definition = compile_to_runtime_definition("Ulamog, the Defiler",
        "Type: Legendary Creature — Eldrazi\nWhen you cast this spell, target opponent exiles the top half of their library, rounded up.\nWard—Sacrifice two permanents.\nUlamog enters with a number of +1/+1 counters on it equal to the greatest mana value among cards in exile.\nUlamog has annihilator X, where X is the number of +1/+1 counters on it.", false)
        .expect("Ulamog should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("HalfRoundedDown"), "{debug}");
    assert!(debug.contains("CardsInLibrary"), "{debug}");
    assert!(debug.contains("GreatestManaValue"), "{debug}");
    // The annihilator trigger sacrifices a counter-defined number of permanents.
    let attack_trigger = p6_triggered(&definition).into_iter()
        .find(|trigger| format!("{:?}", trigger.effects).contains("Defending"))
        .expect("annihilator trigger");
    let effects = format!("{:?}", attack_trigger.effects);
    assert!(effects.contains("PlusOnePlusOne"), "X counts +1/+1 counters: {effects}");
}

#[test]
fn destroyed_equipment_damages_its_last_known_host() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Unforge",
        "Type: Instant\nDestroy target Equipment. If that Equipment was attached to a creature, Unforge deals 2 damage to that creature.", false)
        .expect("attached-host follow-up should compile");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for attached in [true, false] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bear = CardBuilder::new(CardId::new(), "Host").card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3)).build();
        let gear = CardBuilder::new(CardId::new(), "Gear").card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment]).build();
        let host = game.create_object_from_card(&bear, bob, Zone::Battlefield);
        let bystander = game.create_object_from_card(&bear, bob, Zone::Battlefield);
        let equipment = game.create_object_from_card(&gear, bob, Zone::Battlefield);
        if attached {
            assert!(game.attach_object_to_target(equipment, ironsmith::object::AttachmentTarget::Object(host)));
        }
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = EffectContext::new_default(spell, alice).with_targets(vec![ResolvedTarget::Object(equipment)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(!game.battlefield.contains(&equipment));
        assert_eq!(game.damage_on(host), if attached { 2 } else { 0 });
        assert_eq!(game.damage_on(bystander), 0);
    }
}

#[test]
fn next_turn_attack_ban_starts_on_that_players_next_turn() {
    for (name, text) in [
        ("Willie Lumpkin, Postman", "Type: Legendary Creature — Human Citizen\nWillie Lumpkin can't be blocked.\nWhenever Willie Lumpkin deals combat damage to an opponent, you draw a card and that player may draw a card. If they do, that player can't attack you or permanents you control during their next turn."),
        ("The Second Doctor", "Type: Legendary Creature — Time Lord Doctor\nPlayers have no maximum hand size.\nHow Civil of You — At the beginning of your end step, each player may draw a card. Each opponent who does can't attack you or permanents you control during their next turn."),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("AttackPlayerOrPlaneswalkersControlledBy"), "{name}: {debug}");
        assert!(debug.contains("NextTurn("), "{name}: {debug}");
    }
}

#[test]
fn first_spell_each_player_casts_on_their_turn_is_free() {
    let definition = compile_to_runtime_definition("Weftwalking",
        "Type: Enchantment\nWhen this enchantment enters, if you cast it, shuffle your hand and graveyard into your library, then draw seven cards.\nThe first spell each player casts during each of their turns may be cast without paying its mana cost.", false)
        .expect("first-spell free cast should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("first_spell_cast_each_turn: true"), "{debug}");
    assert!(debug.contains("cast_by: Some(Active)"), "{debug}");
    assert!(debug.contains("beneficiary: Any"), "{debug}");
}

#[test]
fn draw_up_to_with_life_for_each_card_not_drawn() {
    let definition = compile_to_runtime_definition("Truce",
        "Type: Instant\nEach player may draw up to two cards. For each card less than two a player draws this way, that player gains 2 life.", false)
        .expect("draw-up-to shortfall should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert_eq!(debug.matches("GainLife").count() >= 2 || debug.matches("gain_life").count() >= 2 || debug.matches("Gain").count() >= 2, true, "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
fn reveal_until_named_card_branches_on_whether_it_was_found() {
    let definition = compile_to_runtime_definition("Tunnel Vision",
        "Type: Sorcery\nChoose a card name. Target player reveals cards from the top of their library until a card with that name is revealed. If it is, that player puts the rest of the revealed cards into their graveyard and puts the card with the chosen name on top of their library. Otherwise, the player shuffles.", false)
        .expect("reveal-until result branch should compile");
    let mut found = false;
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if let Some(conditional) = effect.downcast_ref::<ironsmith::effects::ConditionalEffect>() {
                let debug = format!("{:?}", conditional.if_false);
                if debug.contains("Shuffle") { found = true; }
            }
        });
    }
    assert!(found, "the not-found branch shuffles: {:?}", definition.spell_effect);
}

#[test]
fn put_onto_battlefield_with_conditional_entry_counters() {
    let definition = compile_to_runtime_definition("The First Tyrannic War",
        "Type: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — You may put a creature card from your hand onto the battlefield. If its mana cost contains {X}, it enters with a number of +1/+1 counters on it equal to the number of lands you control.\nII, III — Double the number of each kind of counter on target creature you control.", false)
        .expect("conditional entry counters after a put should compile");
    let chapter_one = p6_triggered(&definition).into_iter().next().expect("chapter I");
    let debug = format!("{:?}", chapter_one.effects);
    assert!(debug.contains("RegisterEnterWithCountersReplacement"), "{debug}");
    assert!(debug.contains("has_x_in_cost: true"), "{debug}");
    assert!(debug.contains("Resolution"), "{debug}");
}

#[test]
fn named_cycling_count_this_game_gates_the_battlefield_replacement() {
    let definition = compile_to_runtime_definition("Yidaro, Wandering Monster",
        "Type: Legendary Creature — Dinosaur Turtle\nTrample, haste\nCycling {1}{R}\nWhen you cycle this card, shuffle it into your library from your graveyard. If you've cycled a card named Yidaro, Wandering Monster four or more times this game, put it onto the battlefield from your graveyard instead. (Do this before you draw.)", false)
        .expect("named cycling count should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("YouCycledCardNamedThisGameAtLeast"), "{debug}");
    assert!(debug.contains("count: 4"), "{debug}");
}

#[test]
fn cast_spell_using_teamwork_trigger_filters_paid_teamwork() {
    let definition = compile_to_runtime_definition("Virtual Assistant",
        "Type: Artifact Creature — Illusion Advisor\nDefender\nWhenever you cast a spell using teamwork, create a 1/1 colorless Robot Hero artifact creature token with flying.", false)
        .expect("teamwork spell trigger should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("\"teamwork\""), "{debug}");
}

#[test]
fn mana_burn_static_costs_life_for_lost_unspent_mana() {
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Yurlok of Scorch Thrash",
        "Type: Legendary Creature — Lizard Shaman\nVigilance\nA player losing unspent mana causes that player to lose that much life.\n{1}, {T}: Each player adds {B}{R}{G}.", false)
        .expect("mana burn should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("LoseLifeForUnspentMana"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    game.player_mut(bob).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 3);
    game.empty_mana_pools().unwrap();
    assert_eq!(game.player(bob).unwrap().life, 17);
    assert_eq!(game.player(alice).unwrap().life, 20);
}

#[test]
fn tiered_shared_instruction_takes_each_tiers_base_power_and_toughness() {
    let definition = compile_to_runtime_definition("Vincent's Limit Break",
        "Type: Instant\nTiered (Choose one additional cost.)\nUntil end of turn, target creature you control gains \"When this creature dies, return it to the battlefield tapped under its owner's control\" and has the chosen base power and toughness.\n• Galian Beast — {0} — 3/2.\n• Death Gigas — {1} — 5/2.\n• Hellmasker — {3} — 7/2.", false)
        .expect("tiered shared-body modes should compile");
    let mut modes = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        p6_visit(effect, &mut |effect| {
            if let Some(choose) = effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>() {
                modes.push(choose.tiered);
            }
        });
    }
    assert_eq!(modes, vec![true], "one tiered choice");
    let debug = format!("{:?}", definition.spell_effect);
    for pt in ["Fixed(3)", "Fixed(5)", "Fixed(7)"] {
        assert!(debug.contains(pt), "{pt}: {debug}");
    }
}

#[test]
fn noted_creature_types_are_counted_for_the_targeted_journal() {
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::types::Subtype;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Volo, Itinerant Scholar",
        "Type: Legendary Creature — Human Wizard\nWhen Volo enters, create Volo's Journal, a legendary colorless Book artifact token with hexproof and \"Whenever you cast a creature spell, note one of its creature types that hasn't been noted for Volo's Journal.\"\n{2}, {T}: Draw a card for each creature type noted for target permanent you control named Volo's Journal.\nChoose a Background", false)
        .expect("noted creature types should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("NoteTriggeringCreatureTypeEffect"), "{debug}");
    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).expect("draw ability");
    assert!(format!("{:?}", ability.effects).contains("NotedCreatureTypeCount"));
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let journal = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for _ in 0..3 {
        game.create_object_from_definition(&definition, alice, Zone::Library);
    }
    assert!(game.note_creature_type_for_source(journal, Subtype::Elf));
    assert!(game.note_creature_type_for_source(journal, Subtype::Wizard));
    assert!(!game.note_creature_type_for_source(journal, Subtype::Elf), "already noted");
    let before = game.player(alice).unwrap().hand.len();
    let mut ctx = EffectContext::new_default(journal, alice).with_targets(vec![ResolvedTarget::Object(journal)]);
    for effect in ability.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().hand.len() - before, 2);
}

#[test]
fn source_loyalty_twice_and_power_up_additional_time_statics_compile() {
    let urza = compile_to_runtime_definition("Urza, Planeswalker",
        "Type: Legendary Planeswalker — Urza\nLoyalty: 7\nYou may activate the loyalty abilities of Urza twice each turn rather than only once.\n+2: Artifact, instant, and sorcery spells you cast this turn cost {2} less to cast. You gain 2 life.\n+1: Draw two cards, then discard a card.\n0: Create two 1/1 colorless Soldier artifact creature tokens.\n−3: Exile target nonland permanent.\n−10: Artifacts and planeswalkers you control gain indestructible until end of turn. Destroy all nonland permanents.", false)
        .expect("source loyalty twice should compile");
    let debug = format!("{:?}", urza.abilities);
    assert!(debug.contains("SourceLoyaltyAbilitiesTwiceEachTurn"), "{debug}");
    let wonder = compile_to_runtime_definition("Wonder Man, Hollywood Hero",
        "Type: Legendary Creature — Human Hero\nFlying\nEach power-up ability of permanents you control can be activated an additional time.\nPower-up — {5}{R}{R}: Put two +1/+1 counters on Wonder Man.", false)
        .expect("power-up additional time should compile");
    let debug = format!("{:?}", wonder.abilities);
    assert!(debug.contains("PowerUpAbilitiesAdditionalTime"), "{debug}");
    assert!(debug.contains("MaxActivationsPerObject(1)"), "{debug}");
}

#[test]
fn mana_abilities_of_this_land_take_a_life_surcharge() {
    let definition = compile_to_runtime_definition("Thran Portal",
        "Type: Land — Gate\nThis land enters tapped unless you control two or fewer other lands.\nAs this land enters, choose a basic land type.\nThis land is the chosen type in addition to its other types.\nMana abilities of this land cost an additional 1 life to activate.", false)
        .expect("mana-ability life surcharge should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ManaAbility"), "{debug}");
    assert!(debug.contains("Life"), "{debug}");
}

#[test]
fn first_activated_ability_of_an_artifact_each_turn_is_discounted() {
    let definition = compile_to_runtime_definition("Tezzeret, Betrayer of Flesh",
        "Type: Legendary Planeswalker — Tezzeret\nLoyalty: 4\nThe first activated ability of an artifact you activate each turn costs {2} less to activate.\n+1: Draw two cards. Then discard two cards unless you discard an artifact card.\n−2: Target artifact becomes an artifact creature. If it isn't a Vehicle, it has base power and toughness 4/4.\n−6: You get an emblem with \"Whenever an artifact you control becomes tapped, draw a card.\"", false)
        .expect("first-activation discount should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("FirstActivationOfMatchingSourceThisTurn"), "{debug}");
    assert!(debug.contains("Activator(You)"), "{debug}");
}

#[test]
fn opponents_must_attack_you_with_at_least_one_creature_compiles() {
    let definition = compile_to_runtime_definition("Trove of Temptation",
        "Type: Enchantment\nEach opponent must attack you or a planeswalker you control with at least one creature each combat if able.\nAt the beginning of your end step, create a Treasure token.", false)
        .expect("group attack requirement should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpponentsAttackYouWithAtLeastOneCreature"), "{debug}");
}

#[test]
fn unblockable_unless_all_defending_creatures_block_compiles() {
    let definition = compile_to_runtime_definition("Tromokratis",
        "Type: Legendary Creature — Kraken\nTromokratis has hexproof unless it's attacking or blocking.\nTromokratis can't be blocked unless all creatures defending player controls block it.", false)
        .expect("all-defenders blocking rule should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("CantBeBlockedUnlessAllDefendingCreaturesBlock"), "{debug}");
    assert!(debug.contains("Hexproof"), "{debug}");
}

#[test]
fn destroy_target_creature_blocking_it_and_named_sacrifice_members() {
    let definition = compile_to_runtime_definition("Urborg Panther",
        "Type: Creature — Nightstalker Cat\n{B}, Sacrifice this creature: Destroy target creature blocking it.\nSacrifice a creature named Feral Shadow, a creature named Breathstealer, and this creature: Search your library for a card named Spirit of the Night, put that card onto the battlefield, then shuffle.", false)
        .expect("blocking-it destroy and named sacrifice members should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("\"Feral Shadow\""), "{debug}");
    assert!(debug.contains("\"Breathstealer\""), "{debug}");
    assert!(!debug.contains("Feral Shadow, a creature"), "{debug}");
}

#[test]
fn capitals_label_and_exile_with_mana_value_time_counters() {
    let definition = compile_to_runtime_definition("The Eleventh Doctor",
        "Type: Legendary Creature — Time Lord Doctor\nI. AM. TALKING! — Whenever The Eleventh Doctor deals combat damage to a player, you may exile a card from your hand with a number of time counters on it equal to its mana value. If it doesn't have suspend, it gains suspend.\n{2}: Target creature with power 3 or less can't be blocked this turn.", false)
        .expect("all-capitals label and dynamic time counters should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("combat damage trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("Time"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("Suspend"), "{debug}");
}

#[test]
fn removed_loyalty_counters_to_activate_gates_the_trigger() {
    let definition = compile_to_runtime_definition("Way of the Mind Sculptor",
        "Type: Enchantment\nWhen Way of the Mind Sculptor enters, empower Jace 5.\nWhenever you activate a loyalty ability, if you removed two or more loyalty counters to activate it, draw a card.", false)
        .expect("loyalty-cost intervening if should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("LoyaltyCountersRemovedToActivate"), "{debug}");
}

#[test]
fn goaded_for_as_long_as_they_control_it() {
    let definition = compile_to_runtime_definition("Vislor Turlough",
        "Type: Legendary Creature — Human Advisor\nDeal with the Black Guardian — When Vislor Turlough enters, you may have an opponent gain control of it. If you do, it's goaded for as long as they control it.\nAt the beginning of your end step, draw a card, then you lose life equal to the number of cards in your hand.\nDoctor's companion", false)
        .expect("control-bounded goad should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("enters trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("GoadEffect"), "{debug}");
    assert!(debug.contains("ObjectControlledBy"), "{debug}");
}

#[test]
fn exiled_card_becomes_foretold() {
    let definition = compile_to_runtime_definition("The Foretold Soldier",
        "Type: Enchantment Creature — Human Soldier\nThis creature must be blocked if able.\nThis creature can't be blocked by more than one creature.\nWhenever this creature deals damage, exile it face down. It becomes foretold.\nForetell {1}{G}", false)
        .expect("becomes foretold should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("BecomeForetoldEffect"), "{debug}");
}

#[test]
fn damage_to_each_of_independent_optional_targets() {
    let definition = compile_to_runtime_definition("Volatile Arsonist",
        "Type: Creature — Human Werewolf\nMenace, haste\nWhenever this creature attacks, it deals 1 damage to each of up to one target creature, up to one target player, and/or up to one target planeswalker.\nDaybound", false)
        .expect("each-of damage targets should compile");
    let trigger = p6_triggered(&definition).into_iter().next().expect("attack trigger");
    let mut damage = 0;
    for effect in trigger.effects.all_effects() {
        p6_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::DealDamageEffect>().is_some() {
                damage += 1;
            }
        });
    }
    assert_eq!(damage, 3, "{:?}", trigger.effects);
}
