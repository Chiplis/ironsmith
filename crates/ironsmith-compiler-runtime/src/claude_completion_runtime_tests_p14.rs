//! Claude package 14 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p14_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p14_visit(child, f));
}

fn p14_activated(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::ActivatedAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
        _ => None,
    }).collect()
}

fn p14_triggered(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).collect()
}

const P14_VIVIEN: &str = "Type: Legendary Planeswalker — Vivien\nLoyalty: 3\n+2: You may sacrifice a creature. If you do, search your library for a creature card with mana value equal to 1 plus the sacrificed creature's mana value, put it onto the battlefield, then shuffle.\n+1: Mill five cards, then put any number of creature cards milled this way into your hand.\n−1: Create a 4/4 green Rhino Warrior creature token.";

#[test]
fn comma_then_milled_this_way_binds_the_mill_result_in_an_activated_ability() {
    let definition = compile_to_runtime_definition("Vivien on the Hunt", P14_VIVIEN, false)
        .expect("Vivien on the Hunt should compile");
    let abilities = p14_activated(&definition);
    assert_eq!(abilities.len(), 3, "three loyalty abilities");
    let mut milled_tags = Vec::new();
    let mut moved_from_tag = Vec::new();
    for ability in &abilities {
        for effect in ability.effects.all_effects() {
            p14_visit(effect, &mut |effect| {
                if let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>()
                    && tagged.effect.downcast_ref::<ironsmith::effects::MillEffect>().is_some()
                {
                    milled_tags.push(tagged.tag.as_str().to_string());
                }
                if let Some(moved) = effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
                    && moved.zone == ironsmith::Zone::Hand
                {
                    moved_from_tag.push(format!("{:?}", moved.target));
                }
            });
        }
    }
    let [milled] = milled_tags.as_slice() else { panic!("one tagged mill: {milled_tags:?}") };
    assert!(
        moved_from_tag.iter().any(|target| target.contains(milled.as_str()) && target.contains("IsTaggedObject")),
        "the put-into-hand chooses among the milled cards ({milled}): {moved_from_tag:?}"
    );
}

#[test]
fn great_aurora_each_player_draws_as_many_as_they_shuffled() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("The Great Aurora",
        "Mana cost: {6}{G}{G}{G}\nType: Sorcery\nEach player shuffles all cards from their hand and all permanents they own into their library, then draws that many cards. Each player may put any number of land cards from their hand onto the battlefield. Exile The Great Aurora.", false)
        .expect("The Great Aurora should compile");
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = CardBuilder::new(CardId::new(), "Aurora filler").card_types(vec![CardType::Sorcery]).build();
    let bear = CardBuilder::new(CardId::new(), "Aurora bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    for _ in 0..10 {
        game.create_object_from_card(&spell, alice, Zone::Library);
        game.create_object_from_card(&spell, bob, Zone::Library);
    }
    game.create_object_from_card(&spell, alice, Zone::Hand);
    game.create_object_from_card(&spell, alice, Zone::Hand);
    game.create_object_from_card(&bear, alice, Zone::Battlefield);
    game.create_object_from_card(&spell, bob, Zone::Hand);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().hand.len(), 3, "two hand cards and one permanent");
    assert_eq!(game.player(bob).unwrap().hand.len(), 1, "one hand card");
    assert_eq!(game.player(alice).unwrap().library.len(), 10);
    assert_eq!(game.player(bob).unwrap().library.len(), 10);
}

#[test]
fn as_long_as_attached_restriction_and_quoted_grant_share_one_condition() {
    let definition = compile_to_runtime_definition("The Aetherspark",
        "Mana cost: {4}\nType: Legendary Artifact Planeswalker — Equipment\nLoyalty: 4\nAs long as The Aetherspark is attached to a creature, The Aetherspark can't be attacked and has \"Whenever equipped creature deals combat damage during your turn, put that many loyalty counters on The Aetherspark.\"\n+1: Attach The Aetherspark to up to one target creature you control. Put a +1/+1 counter on that creature.\n−5: Draw two cards.\n−10: Add ten mana of any one color.", false)
        .expect("The Aetherspark should compile");
    let statics: Vec<String> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(static_ability) => Some(format!("{static_ability:?}")),
        _ => None,
    }).collect();
    let restriction = statics.iter().find(|debug| debug.contains("AttackPermanents"))
        .unwrap_or_else(|| panic!("can't-be-attacked restriction: {statics:#?}"));
    let grant = statics.iter().find(|debug| debug.contains("Loyalty") && debug.contains("Whenever"))
        .or_else(|| statics.iter().find(|debug| debug.contains("Loyalty")))
        .unwrap_or_else(|| panic!("granted loyalty trigger: {statics:#?}"));
    for member in [restriction, grant] {
        assert!(member.contains("AttachmentCount"), "member is gated by the attached condition: {member}");
    }
    assert_eq!(p14_activated(&definition).len(), 3, "three loyalty abilities");
}

#[test]
fn you_and_those_players_each_draw_then_discard_reaches_only_the_attacked_players() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, AttackerInfo};
    use ironsmith::events::combat::CreatureAttackedEvent;
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::triggers::{AttackEventTarget, TriggerEvent};
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Zurzoth, Chaos Rider",
        "Mana cost: {2}{R}\nType: Legendary Creature — Devil\nPower/Toughness: 2/3\nWhenever an opponent draws their first card each turn, if it's not their turn, you create a 1/1 red Devil creature token with \"When this token dies, it deals 1 damage to any target.\"\nWhenever one or more Devils you control attack one or more players, you and those players each draw a card, then discard a card at random.", false)
        .expect("Zurzoth should compile");
    let triggers = p14_triggered(&definition);
    assert_eq!(triggers.len(), 2);
    let attack = triggers[1];
    let debug = format!("{:?}", attack.effects);
    assert!(debug.contains(ironsmith_core::tag::ATTACKING_GROUP_DEFENDERS_TAG), "those players: {debug}");
    assert!(debug.contains("random: true"), "discard at random: {debug}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let players = [PlayerId::from_index(0), PlayerId::from_index(1), PlayerId::from_index(2)];
    let filler = CardBuilder::new(CardId::new(), "Zurzoth filler").card_types(vec![CardType::Sorcery]).build();
    for player in players {
        for _ in 0..5 { game.create_object_from_card(&filler, player, Zone::Library); }
        game.create_object_from_card(&filler, player, Zone::Hand);
    }
    let devil_card = CardBuilder::new(CardId::new(), "Zurzoth devil").card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Devil]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let devil = game.create_object_from_card(&devil_card, players[0], Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, players[0], Zone::Battlefield);
    let declared: std::sync::Arc<[AttackerInfo]> =
        vec![AttackerInfo { creature: devil, target: AttackTarget::Player(players[1]) }].into();
    let event = TriggerEvent::new_with_provenance(
        CreatureAttackedEvent::with_total_attackers(devil, AttackEventTarget::Player(players[1]), 1)
            .with_declared_attackers(declared),
        Default::default(),
    );
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, players[0]).with_triggering_event(event);
    ctx.set_tagged_objects(ironsmith_core::ATTACKING_GROUP_TAG,
        vec![ObjectSnapshot::from_object_id(&game, devil).unwrap()]);
    for effect in attack.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    for (player, drew) in [(players[0], true), (players[1], true), (players[2], false)] {
        let state = game.player(player).unwrap();
        assert_eq!(state.library.len(), if drew { 4 } else { 5 }, "{player:?} library");
        assert_eq!(state.hand.len(), 1, "{player:?} hand");
        assert_eq!(state.graveyard.len(), usize::from(drew), "{player:?} graveyard");
    }
}

#[test]
fn enchanted_players_graveyard_replacement_reveals_and_bottoms_the_card() {
    let definition = compile_to_runtime_definition("Wheel of Sun and Moon",
        "Mana cost: {G/W}{G/W}\nType: Enchantment — Aura\nEnchant player\nIf a card would be put into enchanted player's graveyard from anywhere, instead that card is revealed and put on the bottom of that player's library.", false)
        .expect("Wheel of Sun and Moon should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ZoneChange"), "zone-change replacement: {debug}");
    assert!(debug.contains("\"enchanted\""), "only the enchanted player's cards: {debug}");
    assert!(debug.contains("Reveal"), "the card is revealed: {debug}");
    assert!(debug.contains("Library") && !debug.contains("to_top: true"), "bottom of the library: {debug}");
}

#[test]
fn search_filter_mana_value_bound_reads_the_preceding_sacrifice_count() {
    let definition = compile_to_runtime_definition("The Hunger Tide Rises",
        "Mana cost: {2}{G}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\nI, II, III — Create a 1/1 black and green Insect creature token.\nIV — Sacrifice any number of creatures. Search your library and/or graveyard for a creature card with mana value less than or equal to the number of creatures sacrificed this way and put it onto the battlefield. If you search your library this way, shuffle.", false)
        .expect("The Hunger Tide Rises should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("LessThanOrEqualExpr"), "mana value bound: {debug}");
    assert!(debug.contains("PriorEffectMetric") && debug.contains("Sacrificed"),
        "bound reads the sacrificed-this-way count: {debug}");
    assert!(debug.contains("Graveyard"), "library and/or graveyard: {debug}");
}

#[test]
fn card_types_among_them_bounds_the_whole_exiled_selection() {
    let definition = compile_to_runtime_definition("Winter, Cynical Opportunist",
        "Mana cost: {2}{B}{G}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 2/5\nDeathtouch\nWhenever Winter attacks, mill three cards.\nDelirium — At the beginning of your end step, you may exile any number of cards from your graveyard with four or more card types among them. If you do, put a permanent card from among them onto the battlefield with a finality counter on it.", false)
        .expect("Winter should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DistinctCardTypes"), "whole-set card-type bound: {debug}");
    let mut filter = ironsmith::target::ObjectFilter::default();
    filter.target_set_aggregate_constraint = Some(Box::new(ironsmith_core::ChoiceAggregateConstraint::at_least(
        ironsmith_core::ChoiceAggregateMetric::DistinctCardTypes, 4)));
    assert!(filter.description().contains("4 or more card types among them"), "{}", filter.description());
}

#[test]
fn power_toughness_or_mana_value_spell_filter_is_a_three_way_disjunction() {
    let definition = compile_to_runtime_definition("The Fantastic Four",
        "Mana cost: {R}{G}{W}{U}\nType: Legendary Creature — Human Hero\nPower/Toughness: 4/4\nWhen The Fantastic Four enter and whenever you cast a spell with power, toughness, or mana value 4, choose one that hasn't been chosen this turn.\n• Create a 0/4 colorless Wall creature token with defender.\n• The Fantastic Four deal 3 damage to each opponent.\n• Put two +1/+1 counters on target creature.\n• Draw a card.", false)
        .expect("The Fantastic Four should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("power: Some(Equal(4))") && debug.contains("toughness: Some(Equal(4))")
        && debug.contains("mana_value: Some(Equal(4))"), "power, toughness, or mana value 4: {debug}");
}

#[test]
fn for_each_permanent_choose_a_counter_then_may_add_one_of_that_kind() {
    let definition = compile_to_runtime_definition("The Caves of Androzani",
        "Mana cost: {3}{W}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\nI — Put two stun counters on each of up to two target tapped creatures.\nII, III — For each non-Saga permanent, choose a counter on it. You may put an additional counter of that kind on that permanent.\nIV — Search your library for a Doctor card, reveal it, put it into your hand, then shuffle.", false)
        .expect("The Caves of Androzani should compile");
    let mut found = 0;
    for trigger in p14_triggered(&definition) {
        for effect in trigger.effects.all_effects() {
            p14_visit(effect, &mut |effect| {
                if let Some(for_each) = effect.downcast_ref::<ironsmith::effects::ForEachObject>() {
                    let debug = format!("{for_each:?}");
                    if debug.contains("PutCounterOfKindChosenFromEffect") {
                        assert!(debug.contains("MayEffect"), "the extra counter is optional: {debug}");
                        assert!(debug.contains("Saga"), "non-Saga permanents: {debug}");
                        assert_eq!(trigger.trigger.downcast_ref::<ironsmith::triggers::SagaChapterTrigger>()
                            .expect("chapter trigger").chapters, vec![2, 3]);
                        found += 1;
                    }
                }
            });
        }
    }
    assert_eq!(found, 1, "chapters II and III share one grouped program");
}

#[test]
fn chosen_players_creatures_attack_the_other_chosen_player() {
    let definition = compile_to_runtime_definition("The Brothers' War",
        "Mana cost: {3}{R}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI — Create two tapped Powerstone tokens.\nII — Choose two target players. Until your next turn, each creature they control attacks the other chosen player each combat if able.\nIII — This Saga deals X damage to any target and X damage to any other target, where X is the number of artifacts you control.", false)
        .expect("The Brothers' War should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("MustAttackPlayer"), "per-player attack requirement: {debug}");
    assert!(debug.contains("ForPlayersEffect") && debug.contains("AliasedTarget"),
        "one requirement for each chosen player: {debug}");
    assert!(debug.contains("YourNextTurn"), "until your next turn: {debug}");
}
