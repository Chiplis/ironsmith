//! Claude package 13 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p13_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p13_visit(child, f));
}

fn p13_triggered(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).collect()
}

#[test]
fn rhoda_tap_trigger_skips_attack_declaration_taps() {
    let definition = compile_to_runtime_definition("Rhoda, Geist Avenger",
        "Type: Legendary Creature — Human Soldier\nPartner with Timin, Youthful Geist (When this creature enters, target player may put Timin into their hand from their library, then shuffle.)\nVigilance\nWhenever a creature an opponent controls becomes tapped, if it isn't being declared as an attacker, put a +1/+1 counter on Rhoda.", false)
        .expect("Rhoda should compile");
    let condition = p13_triggered(&definition).into_iter()
        .find_map(|trigger| trigger.intervening_if.as_ref())
        .expect("intervening if");
    assert!(matches!(condition, ironsmith::effect::Condition::Not(inner)
        if matches!(inner.as_ref(), ironsmith::effect::Condition::TriggeringObjectTappedForAttack)), "{condition:?}");
}

#[test]
fn shadrix_may_choose_two_is_two_or_none() {
    let definition = compile_to_runtime_definition("Shadrix Silverquill",
        "Type: Legendary Creature — Elder Dragon\nFlying, double strike\nAt the beginning of combat on your turn, you may choose two. Each mode must target a different player.\n• Target player creates a 2/1 white and black Inkling creature token with flying.\n• Target player draws a card and loses 1 life.\n• Target player puts a +1/+1 counter on each creature they control.", false)
        .expect("optional modal trigger should compile");
    let trigger = p13_triggered(&definition).into_iter().next().expect("combat trigger");
    let mut found = None;
    for effect in trigger.effects.all_effects() {
        p13_visit(effect, &mut |effect| {
            if let Some(modal) = effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>() {
                found = Some(modal.clone());
            }
        });
    }
    let modal = found.expect("modal choice");
    assert!(modal.optional_choice, "{modal:?}");
    assert!(modal.distinct_player_targets_per_mode);
    assert_eq!(modal.choose_count, ironsmith::effect::Value::Fixed(2));
    assert_eq!(modal.min_choose_count, ironsmith::effect::Value::Fixed(2));
    assert_eq!(modal.modes.len(), 3);
    let spec = ironsmith::effects::EffectExecutor::get_modal_spec(&modal).expect("modal spec");
    assert!(spec.optional_choice);
}

#[test]
fn rosheen_keeps_x_in_mana_cost_filters() {
    let definition = compile_to_runtime_definition("Rosheen, Roaring Prophet",
        "Type: Legendary Creature — Giant Shaman\nWhen Rosheen enters, mill six cards. You may put a card with {X} in its mana cost from among them into your hand.\n{T}: Reveal any number of cards with {X} in their mana cost in your hand. Add {C}{C} for each card revealed this way. Spend this mana only on costs that contain {X}.", false)
        .expect("Rosheen should compile");
    let trigger = p13_triggered(&definition).into_iter().next().expect("ETB trigger");
    let trigger_debug = format!("{:?}", trigger.effects);
    assert!(trigger_debug.contains("has_x_in_cost: true"), "{trigger_debug}");
    let mana_debug = definition.abilities.iter()
        .filter(|ability| !matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
        .map(|ability| format!("{:?}", ability.kind))
        .find(|debug| debug.contains("Reveal"))
        .expect("reveal mana ability");
    assert!(mana_debug.contains("has_x_in_cost: true"), "{mana_debug}");
}

#[test]
fn retether_restricts_returned_auras_to_creatures() {
    let definition = compile_to_runtime_definition("Retether",
        "Type: Sorcery\nReturn each Aura card from your graveyard to the battlefield. Only creatures can be enchanted this way. (Aura cards that can't enchant a creature on the battlefield remain in your graveyard.)", false)
        .expect("Retether should compile");
    let mut found = None;
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p13_visit(effect, &mut |effect| {
            if let Some(return_all) = effect.downcast_ref::<ironsmith::effects::ReturnAllToBattlefieldEffect>() {
                found = Some(return_all.clone());
            }
        });
    }
    let return_all = found.expect("return-all effect");
    let attachment = return_all.aura_attachment_filter.expect("creature-only attachment restriction");
    assert_eq!(attachment.card_types, vec![ironsmith::types::CardType::Creature], "{attachment:?}");
    assert!(return_all.filter.subtypes.contains(&ironsmith::types::Subtype::Aura));
}

#[test]
fn seeker_of_slaanesh_forces_one_opponent_attack_per_combat() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Seeker of Slaanesh",
        "Type: Creature — Demon\nHaste\nAllure of Slaanesh — Each opponent must attack with at least one creature each combat if able.", false)
        .expect("Seeker should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpponentsAttackWithAtLeastOneCreature"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.remove_summoning_sickness(first);
    game.remove_summoning_sickness(second);
    let combat = ironsmith::combat_state::CombatState::default();
    let before = ironsmith::compute_legal_attackers(&game, &combat);
    assert!(before.iter().all(|option| !option.must_attack), "no requirement yet");
    game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let after = ironsmith::compute_legal_attackers(&game, &combat);
    assert_eq!(after.iter().filter(|option| option.must_attack).count(), 1,
        "exactly one creature is marked to satisfy the whole-declaration requirement: {after:?}");
}

#[test]
fn reckless_blaze_watches_each_creature_it_damaged() {
    let definition = compile_to_runtime_definition("Reckless Blaze",
        "Type: Sorcery — Lesson\nReckless Blaze deals 5 damage to each creature. Whenever a creature you control dealt damage this way dies this turn, add {R}.", false)
        .expect("Reckless Blaze should compile");
    let mut damage_tag = None;
    let mut watcher = None;
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p13_visit(effect, &mut |effect| {
            if let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>()
                && format!("{:?}", tagged.effect).contains("DealDamageEachEffect")
            {
                damage_tag = Some(tagged.tag.clone());
            }
            if let Some(delayed) = effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>() {
                watcher = Some(delayed.clone());
            }
        });
    }
    let damage_tag = damage_tag.expect("damage-each result is tagged");
    let watcher = watcher.expect("delayed dies watcher");
    assert_eq!(watcher.target_tag.as_ref(), Some(&damage_tag), "{watcher:?}");
    let filter = watcher.target_filter.expect("controller restriction");
    assert_eq!(filter.controller, Some(ironsmith::target::PlayerFilter::You), "{filter:?}");
}

#[test]
fn ruxa_grants_unblocked_assignment_to_abilityless_creatures() {
    let definition = compile_to_runtime_definition("Ruxa, Patient Professor",
        "Type: Legendary Creature — Bear Druid\nWhenever Ruxa enters or attacks, return target creature card with no abilities from your graveyard to your hand.\nCreatures you control with no abilities get +1/+1.\nFor each creature you control with no abilities, you may have that creature assign its combat damage as though it weren't blocked.", false)
        .expect("Ruxa should compile");
    let grant = definition.abilities.iter()
        .map(|ability| format!("{:?}", ability.kind))
        .find(|debug| debug.contains("MayAssignDamageAsUnblocked"))
        .expect("unblocked-assignment grant");
    assert!(grant.contains("no_abilities: true"), "{grant}");
}

#[test]
fn reed_richards_replaces_first_non_draw_step_draw_each_turn() {
    let definition = compile_to_runtime_definition("Reed Richards, Smartest Man",
        "Type: Legendary Creature — Human Scientist Hero\nReach\nYou have no maximum hand size.\nThe first time you would draw a card each turn except the first card you draw during each of your draw steps, you draw four cards instead.", false)
        .expect("Reed Richards should compile");
    let debug = definition.abilities.iter()
        .map(|ability| format!("{:?}", ability.kind))
        .find(|debug| debug.contains("ConditionalDrawReplacement"))
        .expect("first-draw replacement");
    assert!(debug.contains("except_first_of_draw_step: true"), "{debug}");
    assert!(debug.contains("MaxCardsDrawnThisTurn"), "{debug}");
    assert!(debug.contains("YourTurn"), "{debug}");
}

#[test]
fn scion_of_halaster_grants_first_draw_look_two_replacement() {
    let definition = compile_to_runtime_definition("Scion of Halaster",
        "Type: Legendary Enchantment — Background\nCommander creatures you own have \"The first time you would draw a card each turn, instead look at the top two cards of your library. Put one of them into your graveyard and the other back on top of your library. Then draw a card.\"", false)
        .expect("Scion of Halaster should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ConditionalDrawReplacement"), "{debug}");
    assert!(debug.contains("MaxCardsDrawnThisTurn"), "{debug}");
    assert!(debug.contains("LookAtTopCardsEffect"), "{debug}");
    assert!(debug.contains("Graveyard"), "the chosen card goes to the graveyard: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("is_commander: true"), "{debug}");
}

#[test]
fn seize_the_spotlight_branches_on_each_opponents_choice() {
    let definition = compile_to_runtime_definition("Seize the Spotlight",
        "Type: Sorcery\nEach opponent chooses fame or fortune. For each player who chose fame, gain control of a creature that player controls until end of turn. Untap those creatures and they gain haste until end of turn. For each player who chose fortune, you draw a card and create a Treasure token.", false)
        .expect("Seize the Spotlight should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("ChoosePlayerOption"), "{debug}");
    assert!(debug.contains("fame") && debug.contains("fortune"), "{debug}");
    assert!(debug.contains("Haste"), "{debug}");
    assert!(debug.contains("Treasure"), "{debug}");
}

#[test]
fn river_song_triggers_on_each_listed_library_action() {
    let definition = compile_to_runtime_definition("River Song",
        "Type: Legendary Creature — Human Time Lord Rogue\nMeet in Reverse — You draw cards from the bottom of your library rather than the top.\nSpoilers — Whenever an opponent scries, surveils, or searches their library, put a +1/+1 counter on River Song. Then River Song deals damage to that player equal to its power.", false)
        .expect("River Song should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DrawFromBottom"), "{debug}");
    let trigger = p13_triggered(&definition).into_iter().next().expect("Spoilers trigger");
    let trigger_debug = format!("{:?}", trigger.trigger);
    for action in ["Scry", "Surveil", "Search"] {
        assert!(trigger_debug.to_lowercase().contains(&action.to_lowercase()), "{action}: {trigger_debug}");
    }
}

#[test]
fn scythe_specter_drains_the_greatest_discarded_mana_value() {
    let definition = compile_to_runtime_definition("Scythe Specter",
        "Type: Creature — Specter\nFlying\nWhenever this creature deals combat damage to a player, each opponent discards a card. Each player who discarded a card with the greatest mana value among cards discarded this way loses life equal to that mana value.", false)
        .expect("Scythe Specter should compile");
    let trigger = p13_triggered(&definition).into_iter().next().expect("combat damage trigger");
    let debug = format!("{:?}", trigger.effects);
    assert!(debug.contains("GreatestManaValue"), "{debug}");
    assert!(debug.contains("EqualExpr"), "{debug}");
    assert!(debug.contains("LoseLife"), "{debug}");
}
