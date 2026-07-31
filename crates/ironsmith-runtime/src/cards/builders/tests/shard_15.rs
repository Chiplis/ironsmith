#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_16::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_standalone_choose_player_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Player Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a player.")
        .expect("standalone choose-player clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("chooseplayereffect"),
        "expected standalone choose-player effect, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn opportunistic_dragon_renders_one_leading_source_lifetime_bundle() {
    let def = parse_oracle_card_definition("Opportunistic Dragon");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "for as long as this creature remains on the battlefield, gain control of that permanent, it loses all abilities, and it can't attack or block"
        ),
        "expected one coordinated source-lifetime control bundle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn splinter_aging_champion_keeps_the_other_target_player_surface() {
    let def = parse_oracle_card_definition("Splinter, Aging Champion");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "When Splinter leaves the battlefield, you and another target player each draw a card."
        ),
        "expected the joint draw to retain its target declaration, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn master_of_the_wild_hunt_compiles_tagged_dynamic_damage_program() {
    let def = parse_oracle_card_definition("Master of the Wild Hunt");
    let rendered = compiled_text_lines(&def).join("\n");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        (rendered.contains(
            "Each Wolf tapped this way deals damage equal to its power to target creature"
        ) || rendered.contains(
            "For each Wolf tapped this way, that creature deals damage equal to its power to target creature"
        )) && (rendered.contains(
            "That creature deals damage equal to its power divided as its controller chooses among any number of those Wolves"
        ) || (rendered.contains(
            "A creature dealt damage this way deals X damage divided as its controller chooses among any number of those Wolves"
        ) && rendered.contains("where X is that creature's power"))),
        "expected Master of the Wild Hunt's reciprocal damage text, got {rendered}"
    );
    assert!(
        debug.contains("ForEachObject")
            && debug.contains("tapped_0")
            && debug.contains("DealDistributedDamageEffect")
            && debug.contains("ControllerOf(\n")
            && debug.contains("Source"),
        "expected tagged Wolves plus dynamic source/controller distributed damage, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn master_of_the_wild_hunt_damages_only_wolves_tapped_this_way() {
    use crate::card::{CardBuilder, PowerToughness};

    struct TargetControllerDistribution {
        chooser: PlayerId,
        source: ObjectId,
        first_wolf: ObjectId,
        second_wolf: ObjectId,
    }

    impl crate::decision::DecisionMaker for TargetControllerDistribution {
        fn decide_distribute(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::DistributeContext,
        ) -> Vec<(crate::game_state::Target, u32)> {
            assert_eq!(ctx.player, self.chooser);
            assert_eq!(ctx.source, Some(self.source));
            assert_eq!(ctx.total, 3);
            let candidates = ctx
                .targets
                .iter()
                .map(|entry| entry.target)
                .collect::<Vec<_>>();
            assert_eq!(candidates.len(), 2);
            assert!(candidates.contains(&crate::game_state::Target::Object(self.first_wolf)));
            assert!(candidates.contains(&crate::game_state::Target::Object(self.second_wolf)));
            vec![
                (crate::game_state::Target::Object(self.first_wolf), 1),
                (crate::game_state::Target::Object(self.second_wolf), 2),
            ]
        }
    }

    let def = parse_oracle_card_definition("Master of the Wild Hunt");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Master should have an activated ability");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let master = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let wolf = |id, name| {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wolf])
            .power_toughness(PowerToughness::fixed(2, 8))
            .build()
    };
    let first_wolf =
        game.create_object_from_card(&wolf(91_801, "First Wolf"), alice, Zone::Battlefield);
    let second_wolf =
        game.create_object_from_card(&wolf(91_802, "Second Wolf"), alice, Zone::Battlefield);
    let already_tapped_wolf = game.create_object_from_card(
        &wolf(91_803, "Already Tapped Wolf"),
        alice,
        Zone::Battlefield,
    );
    game.tap(already_tapped_wolf);
    let target_card = CardBuilder::new(CardId::from_raw(91_804), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 10))
        .build();
    let target = game.create_object_from_card(&target_card, bob, Zone::Battlefield);

    let mut dm = TargetControllerDistribution {
        chooser: bob,
        source: target,
        first_wolf,
        second_wolf,
    };
    let mut ctx = crate::effects::ExecutionContext::new(master, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|error| {
            panic!(
                "Master's activated ability effect should resolve: {error:?}; effect={effect:#?}"
            )
        });
    }

    assert!(game.is_tapped(first_wolf));
    assert!(game.is_tapped(second_wolf));
    assert_eq!(game.damage_on(target), 4);
    assert_eq!(game.damage_on(first_wolf), 1);
    assert_eq!(game.damage_on(second_wolf), 2);
    assert_eq!(game.damage_on(already_tapped_wolf), 0);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_creature_cant_be_blocked_by_creatures_with_flying() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gnat Alley Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by creatures with flying.")
        .expect("cant-be-blocked-by-flying clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction against fliers, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_creature_cant_be_blocked_except_by_black_creatures() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dread Warlock Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked except by black creatures.")
        .expect("cant-be-blocked-except-by-black clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction to nonblack blockers, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_elven_riders_strict_regression() {
    assert_oracle_card_parses_strict("Elven Riders");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn elven_riders_compiled_text_keeps_walls_and_flying_blocker_clause() {
    let def = parse_oracle_card_definition("Elven Riders");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("excluded_subtypes")
            && abilities_debug.contains("wall")
            && abilities_debug.contains("excluded_static_abilities")
            && abilities_debug.contains("flying"),
        "expected restriction to disallow non-Wall nonflying blockers, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("can't be blocked except by")
            && rendered.contains("wall")
            && rendered.contains("flying"),
        "expected rendered walls/flying blocker clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_phyrexian_colossus_strict_and_render_three_or_more_blockers_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phyrexian Colossus")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 8))
        .parse_text(
            "Trample\nPhyrexian Colossus doesn't untap during your untap step.\nPay 8 life: Untap Phyrexian Colossus.\nPhyrexian Colossus can't be blocked except by three or more creatures.",
        )
        .expect("Phyrexian Colossus should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("can't be blocked except by 3 or more creatures")
            || rendered.contains("can't be blocked except by three or more creatures"),
        "expected rendered min-blockers clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_creature_cant_be_blocked_by_walls() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bog Rats Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by Walls.")
        .expect("cant-be-blocked-by-walls clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction against Walls, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_creature_cant_block_creatures_with_power_two_or_greater() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brassclaw Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't block creatures with power 2 or greater.")
        .expect("cant-block-power-threshold clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("blockspecificattacker"),
        "expected blocker restriction by attacker power, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("can't block creatures with power 2 or greater"),
        "expected rendered blocker restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_creatures_without_flying_cant_block_this_turn() {
    let _def = CardDefinitionBuilder::new(CardId::from_raw(1), "Falter Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures without flying can't block this turn.")
        .expect("global cant-block-this-turn clause should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_cant_block_this_turn() {
    let _def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blindblast Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature can't block this turn.")
        .expect("target cant-block-this-turn clause should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_energy_count_binds_that_player_controls() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Energy Surveyor Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, choose target opponent. You get an amount of {E} equal to the number of nonbasic lands that player controls.",
        )
        .expect("target-player energy count clause should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected enters trigger");
    let energy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::EnergyCountersEffect>())
        .expect("expected energy counter effect");
    let Value::Count(filter) = &energy.count else {
        panic!(
            "expected energy count to be object-count based, got {:?}",
            energy.count
        );
    };
    assert_eq!(filter.card_types, vec![CardType::Land]);
    assert!(filter.excluded_supertypes.contains(&Supertype::Basic));
    assert!(matches!(
        &filter.controller,
        Some(PlayerFilter::Target(player) | PlayerFilter::AliasedTarget(player))
            if **player == PlayerFilter::Opponent
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_then_creatures_cant_block_splits_card_type_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Demolition Wave Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target nonbasic land defending player controls, and creatures that player controls without flying can't block this turn.",
        )
        .expect("destroy plus cant-block clause should parse");

    let spell_effect = def.spell_effect.as_ref().expect("expected spell effect");
    let effects = spell_effect.flattened_default_effects();
    fn collect_nested_effects<'a>(effect: &'a Effect, collected: &mut Vec<&'a Effect>) {
        collected.push(effect);
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            for nested in &sequence.effects {
                collect_nested_effects(nested, collected);
            }
        } else if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
            collect_nested_effects(&tagged.effect, collected);
        } else if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            collect_nested_effects(&tag_all.effect, collected);
        } else if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            collect_nested_effects(&with_id.effect, collected);
        }
    }
    let mut nested_effects = Vec::new();
    for effect in effects {
        collect_nested_effects(effect, &mut nested_effects);
    }
    let destroy = nested_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
        .expect("expected destroy effect");
    let ChooseSpec::Object(destroy_filter) = destroy.spec.base() else {
        panic!(
            "expected destroy target object filter, got {:?}",
            destroy.spec
        );
    };
    assert_eq!(destroy_filter.card_types, vec![CardType::Land]);
    assert!(!destroy_filter.card_types.contains(&CardType::Creature));
    assert_eq!(destroy_filter.controller, Some(PlayerFilter::Defending));
    assert!(
        destroy_filter
            .excluded_supertypes
            .contains(&Supertype::Basic)
    );

    let cant = nested_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::CantEffect>())
        .expect("expected cant-block effect");
    match &cant.restriction {
        crate::effect::Restriction::Block(filter) => {
            assert_eq!(filter.card_types, vec![CardType::Creature]);
            let controller_matches = match filter.controller.as_ref() {
                Some(PlayerFilter::Defending) => true,
                Some(PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))) => {
                    tag.as_str() == "destroyed_0"
                }
                Some(PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag))) => {
                    tag.as_str() == "destroyed_0"
                }
                _ => false,
            };
            assert!(
                controller_matches,
                "unexpected cant-block controller: {filter:?}"
            );
            assert!(
                filter
                    .excluded_static_abilities
                    .contains(&StaticAbilityId::Flying),
                "expected cant-block filter to exclude flying creatures, got {filter:?}"
            );
        }
        other => panic!("expected block restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_your_maximum_hand_size_reduced_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thought Devourer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying\nYour maximum hand size is reduced by four.")
        .expect("your maximum hand size reduction line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("maximum hand size is reduced by"),
        "expected maximum-hand-size reduction in rendered text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_opponents_maximum_hand_size_reduced_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ivory Tower Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("Each opponent's maximum hand size is reduced by one.")
        .expect("each-opponent maximum hand size reduction line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("maximum hand size is reduced by"),
        "expected maximum-hand-size reduction in rendered text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trusted_advisor_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Trusted Advisor");

    let has_increase_max_hand_size = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::IncreaseMaximumHandSize
        )
    });
    assert!(
        has_increase_max_hand_size,
        "Trusted Advisor should compile its maximum hand size increase as a static ability"
    );

    let rendered = unprocessed_compiled_lines(&def);
    assert_eq!(
        rendered,
        vec![
            "Your maximum hand size is increased by two.".to_string(),
            "At the beginning of your upkeep, return a blue creature you control to its owner's hand."
                .to_string(),
        ],
        "Trusted Advisor should render its full oracle text"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn twenty_toed_toad_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Twenty-Toed Toad");

    let has_set_max_hand_size = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::SetMaximumHandSize
        )
    });
    assert!(
        has_set_max_hand_size,
        "Twenty-Toed Toad should compile its exact maximum hand size as a static ability"
    );

    let win_trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => format!("{:?}", triggered.trigger)
                .contains("ThisAttacksTrigger")
                .then_some(triggered),
            _ => None,
        });
    let win_trigger = win_trigger.expect("Twenty-Toed Toad should have a this-attacks win trigger");
    assert!(
        win_trigger.intervening_if.is_none(),
        "Twenty-Toed Toad's trailing win condition should be checked on resolution, not as an intervening-if trigger gate"
    );
    let win_effects_debug = format!("{:?}", win_trigger.effects);
    assert!(
        win_effects_debug.contains("ConditionalEffect")
            && win_effects_debug.contains("WinTheGameEffect"),
        "Twenty-Toed Toad's win trigger should compile to a conditional win effect, got {win_effects_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("your maximum hand size is twenty."),
        "expected Twenty-Toed Toad compiled text to include exact maximum hand size, got {rendered}"
    );
    assert!(
        rendered.contains("whenever you attack with 2 or more creatures")
            || rendered.contains("whenever you attack with two or more creatures"),
        "expected Twenty-Toed Toad compiled text to include the attack threshold trigger, got {rendered}"
    );
    assert!(
        rendered.contains("twenty or more counters")
            || rendered.contains("20 or more counters")
            || rendered.contains("20 or more cards in hand"),
        "expected Twenty-Toed Toad compiled text to include its alternate-win condition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tide_skimmer_preserves_explicit_attack_with_threshold_surface() {
    let def = parse_oracle_card_definition("Tide Skimmer");
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![
            "Flying".to_string(),
            "Whenever you attack with two or more creatures with flying, draw a card.".to_string(),
        ]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn combustion_man_preserves_targeted_destroy_unless_source_power_damage() {
    let oracle = "Whenever Combustion Man attacks, destroy target permanent unless its controller has Combustion Man deal damage to them equal to his power.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Combustion Man")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(4, 6))
        .parse_text(oracle)
        .expect("Combustion Man text should parse");

    assert_eq!(compiled_text_lines(&def), vec![oracle.to_string()]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn go_shintai_hidden_cruelty_keeps_where_x_out_of_the_target_domain() {
    let oracle = "Deathtouch\nAt the beginning of your end step, you may pay {1}. When you do, destroy target creature with toughness X or less, where X is the number of Shrines you control.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Go-Shintai of Hidden Cruelty")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Shrine])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(oracle)
        .expect("Go-Shintai of Hidden Cruelty text should parse");

    assert_eq!(
        compiled_text_lines(&def),
        oracle.lines().map(str::to_string).collect::<Vec<_>>()
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn leave_no_trace_preserves_the_radiance_shared_color_fanout() {
    let oracle = "Radiance — Destroy target enchantment and each other enchantment that shares a color with it.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Leave No Trace")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("Leave No Trace text should parse");

    assert_eq!(compiled_text_lines(&def), vec![oracle.to_string()]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fold_into_aether_preserves_countered_spell_controller_hand_move() {
    let oracle = "Counter target spell. If that spell is countered this way, its controller may put a creature card from their hand onto the battlefield.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fold into Aether")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("Fold into Aether text should parse");

    assert_eq!(compiled_text_lines(&def), vec![oracle.to_string()]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn triassic_egg_preserves_modal_counter_activation_restriction() {
    let oracle = "{3}, {T}: Put a hatchling counter on this artifact.\nSacrifice this artifact: Choose one. Activate only if there are two or more hatchling counters on this artifact.\n• You may put a creature card from your hand onto the battlefield.\n• Return target creature card from your graveyard to the battlefield.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Triassic Egg")
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("Triassic Egg text should parse");

    let modal = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .nth(1)
        .expect("Triassic Egg should have a second activated ability");
    let restrictions = format!("{:#?}", modal.activation_restrictions);
    assert!(
        restrictions.contains("hatchling") && restrictions.contains("count: 2"),
        "expected the modal activation to retain its typed hatchling-counter threshold, got {restrictions}"
    );
    assert_eq!(compiled_text_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn authored_once_per_turn_activation_restriction_order_is_preserved() {
    for oracle in [
        "{1}{R}: Put a +1/+1 counter on this creature. Activate only if an opponent lost life this turn and only once each turn.",
        "{1}{R}: Put a +1/+1 counter on this creature. Activate only once each turn and only if an opponent lost life this turn.",
    ] {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Restriction Order")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .expect("ordered activation restriction text should parse");

        assert_eq!(compiled_text_lines(&def), vec![oracle.to_string()]);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn standalone_spell_draw_line_keeps_the_imperative_subject_surface() {
    let oracle = "Tap target artifact or creature.\nDraw a card.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Imperative Draw")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("separate imperative draw line should parse");

    assert_eq!(compiled_text_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shorecrasher_elemental_keeps_its_face_down_blink_sequence() {
    let oracle = "{U}: Exile this creature, then return it to the battlefield face down under its owner's control.\n{1}: This creature gets +1/-1 or -1/+1 until end of turn.\nMegamorph {4}{U}";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shorecrasher Elemental")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(oracle)
        .expect("Shorecrasher Elemental text should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("zone: Exile")
            && debug.contains("zone: Battlefield")
            && debug.contains("enters_face_down: true"),
        "expected a linked exile and face-down battlefield return, got {debug}"
    );
    assert_eq!(compiled_text_lines(&def).join("\n"), oracle, "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn comma_bearing_full_source_name_keeps_the_complete_etb_trigger() {
    let oracle = "When Malik, Grim Manipulator enters, you and target opponent each secretly choose a creature that player controls. Then those choices are revealed, and that player sacrifices those creatures.\nWhenever an opponent sacrifices a creature, you create a Treasure token.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Malik, Grim Manipulator")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("the comma-bearing named-source trigger should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ZoneChange") && debug.contains("CreateToken"),
        "expected the ETB and sacrifice triggers to remain distinct, got {debug}"
    );
    assert_eq!(compiled_text_lines(&def).join("\n"), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_top_x_until_end_of_your_next_turn_may_play_those_cards() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Commune with Lava Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile the top X cards of your library. Until the end of your next turn, you may play those cards.",
        )
        .expect("exile-top then until-next-turn play-those-cards should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("grantplaytaggedeffect"),
        "expected tagged play grant effect in spell text, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wrenns_resolve_exiles_top_two_cards() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wrenn's Resolve")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile the top two cards of your library. Until the end of your next turn, you may play those cards.",
        )
        .expect("wrenn's resolve style clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("top two cards") || rendered.contains("top 2 cards"),
        "expected top-two exile rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_top_card_you_may_play_that_card_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Impulse Draw Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile the top card of your library. You may play that card this turn.")
        .expect("exile-top then play-that-card-this-turn should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("grantplaytaggedeffect"),
        "expected end-of-turn tagged play grant, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("untilendofturn"),
        "expected end-of-turn duration on tagged play grant, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fallen_shinobi_uses_top_library_exile_and_plural_play_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fallen Shinobi")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Ninjutsu {2}{U}{B} ({2}{U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever this creature deals combat damage to a player, that player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs.",
        )
        .expect("fallen shinobi should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("exiletopoflibraryeffect"),
        "expected top-library exile effect in triggered ability, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("grantplaytaggedeffect"),
        "expected tagged play grant for exiled cards, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("granttaggedspellfreecastuntilendofturneffect"),
        "expected free-cast grant for the exiled spells, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("casttaggedeffect"),
        "expected play permission rather than immediate cast, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("thisdealscombatdamagetoplayer")
            || abilities_debug.contains("thisdealscombatdamagetoplayertrigger"),
        "expected the trigger to stay player-targeted, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("card in that player's library"),
        "expected the trigger to stay targeted at a player, got {rendered}"
    );
    assert!(
        (rendered.contains("play those cards") || rendered.contains("play that card"))
            && rendered.contains("without paying their mana costs"),
        "expected plural play-from-exile wording in compiled output, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged object") && !rendered.contains("tagged '"),
        "expected Fallen Shinobi output to avoid internal tagged markers, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_paladin_elizabeth_taggerdy_battalion_puts_hand_creature_tapped_and_attacking() {
    let def = parse_oracle_card_definition("Paladin Elizabeth Taggerdy");

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("thisattackswithnotherstrigger") && debug.contains("other_count: 2"),
        "expected battalion-style source-plus-two-others trigger, got {debug}"
    );
    assert!(
        debug.contains("lessthanorequalexpr")
            && debug.contains("wherexis")
            && debug.contains("paladin elizabeth taggerdy"),
        "expected mana-value X gate to keep the named source-power binding, got {debug}"
    );
    assert!(
        debug.contains("enters_attacking: true"),
        "expected battlefield move to enter attacking, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("paladin elizabeth taggerdy and at least")
            && rendered.contains("other creatures attack"),
        "expected named battalion trigger wording, got {rendered}"
    );
    assert!(
        rendered.contains(
            "mana value x or less from your hand onto the battlefield tapped and attacking"
        ) && rendered.contains("where x is paladin elizabeth taggerdy"),
        "expected tapped-and-attacking hand put with source-power X binding, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_paired_tactician_keeps_other_warrior_attack_subject() {
    let def = parse_oracle_card_definition("Paired Tactician");

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("thisattackswithnotherstrigger")
            && debug.contains("other_count: 1")
            && debug.contains("warrior"),
        "expected source-plus-one-other-Warrior trigger, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever this creature and at least one other warrior attack"),
        "expected Paired Tactician trigger subject to keep Warrior, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_necropotence_style_face_down_exile_with_delayed_return() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Necropotence Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.",
        )
        .expect("necropotence-style face-down exile should parse");

    let ability_debug = format!("{:#?}", def.abilities);
    assert!(
        ability_debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling in activated ability, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("BeginningOfEndStepTrigger"),
        "expected next-end-step trigger in activated ability, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("face down"),
        "expected face-down exile wording to remain explicit, got {rendered}"
    );
    assert!(
        rendered.contains("pay 1 life"),
        "expected necropotence-style activation to stay a life payment cost, got {rendered}"
    );
    assert!(
        rendered.contains("next end step"),
        "expected delayed return timing to remain explicit, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_veil_of_summer_full_spell() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Veil of Summer Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Draw a card if an opponent has cast a blue or black spell this turn. Spells you control can't be countered this turn. You and permanents you control gain hexproof from blue and from black until end of turn.",
        )
        .expect("veil of summer should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw a card if an opponent has cast a blue or black spell this turn")
            || rendered.contains(
                "if an opponent has cast a blue or black spell this turn, you draw a card"
            ),
        "expected draw condition to survive compilation, got {rendered}"
    );
    assert!(
        rendered.contains("can't be countered this turn")
            || rendered.contains("cant be countered this turn"),
        "expected anti-counter clause to survive compilation, got {rendered}"
    );
    assert!(
        rendered.contains("you and permanents you control gain hexproof from blue and from black")
            || (rendered.contains("you have hexproof from blue or black")
                && rendered.contains("permanents you control gain hexproof from blue or black")),
        "expected protection clause to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_finale_of_devastation_x_threshold_spell() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Finale of Devastation Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library and/or graveyard for a creature card with mana value X or less and put it onto the battlefield. If you search your library this way, shuffle. If X is 10 or more, creatures you control get +X/+X and gain haste until end of turn.",
        )
        .expect("finale of devastation should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("mana value x or less"),
        "expected search clause to survive compilation, got {rendered}"
    );
    assert!(
        rendered.contains("if x is 10 or more"),
        "expected x-threshold clause to survive compilation, got {rendered}"
    );
    assert!(
        rendered.contains("creatures you control get +x/+x")
            && rendered.contains("gain haste until end of turn"),
        "expected pump-and-haste clause to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_station_threshold_reminder_adds_creature_pt_support() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Station Probe")
        .card_types(vec![CardType::Artifact])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Station (Tap another creature you control: Put charge counters equal to its power on this artifact. Station only as a sorcery. It's an artifact creature at 4+.)\n4+ | Flying",
        )
        .expect("station artifact-creature threshold should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("station") && rendered.contains("4+ | flying"),
        "expected station threshold to compact to the station row surface, got {rendered}"
    );
}

#[test]
pub(super) fn station_threshold_rows_render_as_station_rows() {
    let sawship = CardDefinitionBuilder::new(CardId::new(), "Station Sawship Variant")
        .card_types(vec![CardType::Artifact])
        .power_toughness(PowerToughness::fixed(6, 5))
        .parse_text("Station\n3+ | Flying, haste")
        .expect("station keyword threshold should parse");
    assert_eq!(
        compiled_text_lines(&sawship),
        vec!["Station".to_string(), "3+ | Flying, haste".to_string()]
    );

    let frigate = CardDefinitionBuilder::new(CardId::new(), "Station Frigate Variant")
        .card_types(vec![CardType::Artifact])
        .power_toughness(PowerToughness::fixed(3, 5))
        .parse_text("Station\n2+ | Other creatures you control get +1/+1.\n12+ | Flying, lifelink")
        .expect("station mixed thresholds should parse");
    assert_eq!(
        compiled_text_lines(&frigate),
        vec![
            "Station".to_string(),
            "2+ | Other creatures you control get +1/+1".to_string(),
            "12+ | Flying, lifelink".to_string(),
        ]
    );

    let debug = format!("{:?}", frigate.abilities);
    assert!(
        debug.contains("CountersOnSource(Charge)")
            && debug.contains("GreaterThanOrEqual")
            && debug.contains("Fixed(12)"),
        "expected station keyword threshold to remain conditional, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_the_eternity_elevator_station_threshold_mana_regression() {
    assert_oracle_card_parses_strict("The Eternity Elevator");
    let oracle = oracle_text_by_name()
        .get("The Eternity Elevator")
        .expect("missing oracle text for The Eternity Elevator")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "The Eternity Elevator")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Spacecraft])
        .parse_text(oracle)
        .expect("The Eternity Elevator should parse with its type line");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Station")
            && rendered.contains(
                "20+ | {T}: Add X mana of any one color, where X is the number of charge counters on The Eternity Elevator"
            ),
        "expected The Eternity Elevator to preserve station threshold mana text, got {rendered}"
    );
    assert!(
        !rendered.contains("Activate only if"),
        "station threshold should render as a threshold prefix, got {rendered}"
    );

    let debug = format!("{def:?}");
    assert!(
        debug.contains("AddManaOfAnyOneColorEffect")
            && debug.contains("CountersOnSource(Charge)")
            && debug.contains("GreaterThanOrEqual")
            && debug.contains("Fixed(20)"),
        "expected threshold mana ability to count charge counters on source at 20+, got {debug}"
    );
}

#[test]
pub(super) fn render_charge_counter_condition_does_not_imply_station_threshold() {
    let mut mana_ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]);
    let AbilityKind::Activated(activated) = &mut mana_ability.kind else {
        panic!("mana ability should be activated");
    };
    activated.activation_condition = Some(Condition::ValueComparison {
        left: Value::CountersOnSource(CounterType::Charge),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(3),
    });

    let def = CardDefinitionBuilder::new(CardId::new(), "Charge Gate")
        .card_types(vec![CardType::Artifact])
        .with_ability(mana_ability)
        .build();
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Activate only if")
            && rendered.contains("charge counters")
            && (rendered.contains("greater than or equal to 3")
                || rendered.contains("is 3 or greater")),
        "ordinary charge-counter activation conditions should render as activation conditions, got {rendered}"
    );
    assert!(
        !rendered.contains("3+ |"),
        "only station threshold lines should render with a numeric threshold prefix, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_eldritch_evolution_sacrifice_scaled_where_x_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Eldritch Evolution")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nSearch your library for a creature card with mana value X or less, where X is 2 plus the sacrificed creature's mana value, put that card onto the battlefield, then shuffle.\nExile Eldritch Evolution.",
        )
        .expect("eldritch evolution should parse");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("lessthanorequalexpr")
            && raw.contains("add(")
            && raw.contains("fixed(")
            && raw.contains("manavalueof(")
            && raw.contains("tagkey(")
            && raw.contains("sacrificed_0")
            && raw.contains("sacrificedobject("),
        "expected eldritch evolution to preserve the sacrificed-creature mana-value bound, got {raw}"
    );
    assert!(
        {
            let rendered = unprocessed_compiled_lines(&def)
                .join(" ")
                .to_ascii_lowercase();
            rendered.contains("put it onto the battlefield, then shuffle")
                || rendered.contains("put that card onto the battlefield, then shuffle")
        },
        "expected eldritch evolution search destination to survive compilation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tymna_the_weaver_postcombat_where_x_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tymna the Weaver")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Lifelink\nAt the beginning of your postcombat main phase, you may pay X life, where X is the number of opponents that were dealt combat damage this turn. If you do, draw X cards.\nPartner",
        )
        .expect("tymna should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("postcombat main phase") || rendered.contains("second main phase"),
        "expected postcombat main phase wording, got {rendered}"
    );
    assert!(
        (rendered.contains("pay x life")
            || rendered.contains("lose x life")
            || rendered.contains("lose the number of opponents life"))
            && (rendered.contains("draw x cards") || rendered.contains("draw that many cards")),
        "expected tymna where-x draw clause to survive rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_until_end_of_turn_you_may_cast_that_card() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ragavan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.\nDash {1}{R}",
        )
        .expect("until-end-of-turn cast-that-card clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("grantplaytaggedeffect"),
        "expected end-of-turn tagged cast/play grant, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("granttaggedspellfreecastuntilendofturneffect"),
        "expected ordinary cast permission without free-cast helper, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Dash {1}{R}"),
        "expected Dash keyword line in compiled output, got {rendered}"
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("until end of turn, you may cast spells from among those cards")
            || rendered
                .to_ascii_lowercase()
                .contains("until end of turn, you may cast that card")
            || rendered
                .to_ascii_lowercase()
                .contains("you may cast that card this turn"),
        "expected cast permission in compiled output, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged 'exiled_"),
        "expected internal exile tag to stay out of compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rejects_unbound_that_player_hidden_zone_reference() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Broken Player Context")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile the top card of that player's library.")
        .expect_err("unbound 'that player' hidden-zone reference should fail validation");

    let err_text = err.to_string();
    assert!(
        err_text.contains("IteratedPlayer"),
        "expected validation error to mention IteratedPlayer, got {err_text}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_allows_that_player_when_trigger_binds_player_context() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Discard Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals combat damage to a player, that player discards a card.",
        )
        .expect("combat-damage trigger should bind 'that player'");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("DiscardEffect"),
        "expected discard effect in triggered ability, got {debug}"
    );
    assert!(
        debug.contains("ThisDealsCombatDamageToPlayer")
            || debug.contains("ThisDealsCombatDamageToPlayerTrigger"),
        "expected the trigger to stay player-targeted, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_allows_that_player_library_search_when_combat_damage_trigger_binds_player() {
    let cases = [
        (
            "Thada Search Probe",
            "Whenever this creature deals combat damage to a player, search that player's library for an artifact card and exile it. Then that player shuffles. Until end of turn, you may play that card.",
        ),
        (
            "Rootwater Search Probe",
            "Whenever this creature deals combat damage to a player, you may pay {2}. If you do, search that player's library for a card and exile it, then the player shuffles.",
        ),
    ];

    for (name, text) in cases {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Creature])
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));

        let debug = format!("{:#?}", def.abilities);
        let compact_debug = debug.split_whitespace().collect::<String>();
        assert!(
            ((compact_debug.contains("ChooseObjectsEffect")
                && compact_debug.contains("zone:Some("))
                || compact_debug.contains("SearchLibraryEffect"))
                && compact_debug.contains("Library")
                && compact_debug.contains("owner:Some(")
                && compact_debug.contains("DamagedPlayer"),
            "expected that player's library search to bind to the combat-damaged player, got {debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_until_end_of_turn_you_may_play_that_card_without_paying_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mind's Desire Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile the top card of your library. Until end of turn, you may play that card without paying its mana cost.",
        )
        .expect("until-end-of-turn free play clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("grantplaytaggedeffect"),
        "expected end-of-turn tagged play grant, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("granttaggedspellfreecastuntilendofturneffect"),
        "expected free-cast/play permission, got {spell_debug}"
    );
}

#[test]
pub(super) fn temporal_aperture_oracle_parses_and_renders_top_library_permission() {
    let def = parse_oracle_card_definition("Temporal Aperture");
    let oracle = "{5}, {T}: Shuffle your library, then reveal the top card. Until end of turn, for as long as that card remains on top of your library, play with the top card of your library revealed and you may play that card without paying its mana cost.";

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("RevealTop"),
        "expected Temporal Aperture to reveal and tag the top card, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("AllPlayersLookAtYourTopLibraryCard"),
        "expected Temporal Aperture to grant temporary top-library reveal permission, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("GrantPlayTaggedEffect")
            || abilities_debug.contains("GrantPlayTaggedUntilEndOfTurn"),
        "expected Temporal Aperture to grant play permission for the revealed card, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect")
            || abilities_debug.contains("without_paying_mana_cost: true"),
        "expected Temporal Aperture to grant a free-cast permission for the revealed card, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("shuffle your library, then reveal the top card")
            && rendered_lower.contains("for as long as that card remains on top of your library")
            && rendered_lower.contains("play with the top card of your library revealed")
            && rendered_lower.contains("you may play that card without paying its mana cost"),
        "expected Temporal Aperture compiled text to preserve the revealed-top-card permission, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("unsupported") && !rendered_lower.contains("unimplemented"),
        "Temporal Aperture should compile without fallback markers, got {rendered}"
    );
    assert_eq!(unprocessed_compiled_lines(&def), vec![oracle.to_string()]);
}

#[test]
pub(super) fn temporal_aperture_runtime_grants_free_cast_only_while_revealed_card_is_top_library_card()
 {
    use crate::alternative_cast::CastingMethod;
    use crate::card::CardBuilder;
    use crate::decision::{LegalAction, compute_legal_actions};
    use crate::effects::{ExecutionContext, execute_effect};

    let def = parse_oracle_card_definition("Temporal Aperture");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Temporal Aperture should have an activated ability");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let temporal_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let lower_spell_a = CardBuilder::new(CardId::from_raw(77_003), "Lower Library Spell A")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let lower_spell_a_id = game.create_object_from_card(&lower_spell_a, alice, Zone::Library);
    let lower_spell_b = CardBuilder::new(CardId::from_raw(77_004), "Lower Library Spell B")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let lower_spell_b_id = game.create_object_from_card(&lower_spell_b, alice, Zone::Library);
    let expensive_spell = CardBuilder::new(CardId::from_raw(77_001), "Expensive Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_card(&expensive_spell, alice, Zone::Library);
    let pre_shuffle_order = game.player(alice).expect("alice exists").library.clone();
    game.queue_transcript_library_shuffle_order(
        alice,
        pre_shuffle_order.clone(),
        pre_shuffle_order,
    );

    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            spell_id,
            Zone::Library,
            alice
        ),
        "Temporal Aperture should not grant play permission before its ability resolves"
    );

    let mut ctx = ExecutionContext::new_default(temporal_id, alice);
    for effect in activated.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Temporal Aperture activated effect should resolve");
    }

    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            spell_id,
            Zone::Library,
            alice
        ),
        "Temporal Aperture should grant play-from-library permission to the revealed card"
    );
    assert!(
        !game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, spell_id, Zone::Library, alice)
            .is_empty(),
        "Temporal Aperture should grant a no-mana alternative cast from library"
    );
    assert!(
        game.current_has_static_ability_id(
            temporal_id,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
        ),
        "Temporal Aperture should reveal the top card while the tagged card remains on top"
    );

    assert!(
        game.set_player_library_order_with_audit(
            alice,
            vec![lower_spell_b_id, lower_spell_a_id, spell_id],
            "test reorder below Temporal Aperture top card",
        ),
        "test should be able to reorder lower library cards without moving the revealed top card"
    );
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            spell_id,
            Zone::Library,
            alice
        ),
        "Temporal Aperture's play permission should survive reorder below the revealed top card"
    );
    assert!(
        !game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, spell_id, Zone::Library, alice)
            .is_empty(),
        "Temporal Aperture's free-cast permission should survive reorder below the revealed top card"
    );
    assert!(
        game.current_has_static_ability_id(
            temporal_id,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
        ),
        "Temporal Aperture should keep revealing while the revealed card remains on top"
    );

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: action_spell_id,
                from_zone: Zone::Library,
                casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
            } if *action_spell_id == spell_id
        )),
        "revealed top spell should be castable from library without paying its mana cost, got {actions:?}"
    );

    let blocker = CardBuilder::new(CardId::from_raw(77_002), "New Top Card")
        .card_types(vec![CardType::Artifact])
        .build();
    let blocker_id = game.create_object_from_card(&blocker, alice, Zone::Library);
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            spell_id,
            Zone::Library,
            alice
        ),
        "Temporal Aperture's play-from-library grant should stop applying after the tagged card leaves the top"
    );
    assert!(
        game.effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, spell_id, Zone::Library, alice)
            .is_empty(),
        "Temporal Aperture's free-cast grant should stop applying after the tagged card leaves the top"
    );
    assert!(
        !game.current_has_static_ability_id(
            temporal_id,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
        ),
        "Temporal Aperture should stop revealing the top card after the tagged card leaves the top"
    );
    let _drawn_blocker = game.move_object_by_game_rule(blocker_id, Zone::Hand);
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            spell_id,
            Zone::Library,
            alice
        ),
        "Temporal Aperture's play grant should not resume if the tagged card later becomes top again"
    );
    assert!(
        !game.current_has_static_ability_id(
            temporal_id,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
        ),
        "Temporal Aperture's reveal permission should not resume if the tagged card later becomes top again"
    );
    let actions_after_top_changed = compute_legal_actions(&game, alice);
    assert!(
        !actions_after_top_changed.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: action_spell_id,
                from_zone: Zone::Library,
                ..
            } if *action_spell_id == spell_id
        )),
        "Temporal Aperture should not offer the revealed spell once it is no longer the top library card, got {actions_after_top_changed:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_your_opponents_cant_cast_spells_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Silence Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Your opponents can't cast spells this turn.")
        .expect("this-turn cast restriction should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("some(") && spell_debug.contains("canteffect"),
        "expected cant effect for cast restriction, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("restriction: castspellsmatching(")
            && spell_debug.contains("opponent"),
        "expected opponent cast restriction, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_your_opponents_cant_cast_creature_spells_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Creature Silence Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Your opponents can't cast creature spells this turn.")
        .expect("this-turn creature-spell restriction should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("some(") && spell_debug.contains("canteffect"),
        "expected cant effect for creature-spell restriction, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("creature"),
        "expected creature-spell cast restriction, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_render_silent_uses_controller_subject_for_cast_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Render Silent")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell. Its controller can't cast spells this turn.")
        .expect("render silent should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("ControllerOf"),
        "expected controller-of-target cast restriction, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dragonlord_dromoka_keeps_during_your_turn_static_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dragonlord Dromoka")
        .card_types(vec![CardType::Creature])
        .parse_text("Your opponents can't cast spells during your turn.")
        .expect("dragonlord dromoka restriction should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("DuringYourTurn"),
        "expected during-your-turn condition, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("OpponentsCantCastSpells")
            && abilities_debug.contains("DuringYourTurn"),
        "expected opponents-cant-cast static ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_abeyance_supports_instant_or_sorcery_cast_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Abeyance")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Until end of turn, target player can't cast instant or sorcery spells, and that player can't activate abilities that aren't mana abilities.\nDraw a card.",
        )
        .expect("abeyance should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("Instant") && spell_debug.contains("Sorcery"),
        "expected instant-or-sorcery restriction, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ActivateNonManaAbilities"),
        "expected non-mana ability restriction, got {spell_debug}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("until end of turn, target player can't cast instant or sorcery spells"),
        "expected an instant-or-sorcery spell restriction, got {rendered}"
    );
    assert!(!rendered.contains("spell matching"), "got {rendered}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_barals_expertise_renders_mana_value_free_cast_as_a_spell() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Baral's Expertise")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return up to three target artifacts and/or creatures to their owners' hands.\nYou may cast a spell with mana value 4 or less from your hand without paying its mana cost.",
        )
        .expect("Baral's Expertise should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "you may cast a spell with mana value 4 or less from your hand without paying its mana cost"
        ),
        "expected the free-cast filter to use the spell noun once, got {rendered}"
    );
    assert!(!rendered.contains("spell matching"), "got {rendered}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_west_coast_expansion_renders_hero_before_spell() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "West Coast Expansion")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Draw X cards. If X is 5 or more, you may cast a Hero spell from your hand without paying its mana cost.",
        )
        .expect("West Coast Expansion should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you may cast a hero spell from your hand without paying its mana cost"),
        "expected the subtype before the spell noun, got {rendered}"
    );
    assert!(!rendered.contains("spell matching"), "got {rendered}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_altar_of_the_lost_renders_flashback_spells_from_a_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Altar of the Lost")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "This artifact enters tapped.\n{T}: Add two mana in any combination of colors. Spend this mana only to cast spells with flashback from a graveyard.",
        )
        .expect("Altar of the Lost should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("spend this mana only to cast spells with flashback from a graveyard"),
        "expected the flashback filter and graveyard origin to render, got {rendered}"
    );
    assert!(!rendered.contains("spell matching"), "got {rendered}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conquerors_flail_condition_maps_attached_equipment_to_equipped() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conqueror's Flail")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "As long as this Equipment is attached to a creature, your opponents can't cast spells during your turn.",
        )
        .expect("conqueror's flail restriction should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("SourceIsEquipped"),
        "expected equipped condition, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("DuringYourTurn"),
        "expected during-your-turn condition, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_grand_abolisher_conditioned_or_restrictions_keep_opponent_subject() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Grand Abolisher")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments.",
        )
        .expect("grand abolisher restriction should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.matches("DuringYourTurn").count() >= 2,
        "expected both restrictions to keep the during-your-turn condition, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("RuleRestriction")
            && abilities_debug.contains("CastSpellsMatching")
            && abilities_debug.contains("Opponent")
            && abilities_debug.contains("ActivateAbilitiesOf"),
        "expected cast and activation restrictions for opponents, got {abilities_debug}"
    );
}

pub(super) fn academic_probation_modal_effect(def: &CardDefinition) -> &ChooseModeEffect {
    def.spell_effect
        .as_ref()
        .expect("Academic Probation should compile to spell effects")
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        .expect("Academic Probation should compile to one modal choice effect")
}

#[test]
pub(super) fn academic_probation_strict_parser_text_and_structure_regression() {
    assert_oracle_card_parses_strict("Academic Probation");

    let def = parse_oracle_card_definition("Academic Probation");
    let modal = academic_probation_modal_effect(&def);
    let modal_debug = format!("{modal:#?}");
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    let oracle = oracle_text_by_name()
        .get("Academic Probation")
        .expect("Academic Probation oracle text");
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            oracle,
            &compiled,
            Some(crate::semantic_compare::EmbeddingConfig {
                dims: 384,
                mismatch_threshold: 0.99,
            }),
        );

    assert_eq!(modal.min_choose_count, Value::Fixed(1));
    assert_eq!(modal.choose_count, Value::Fixed(1));
    assert_eq!(modal.modes.len(), 2);
    assert!(
        modal_debug.contains("ChooseCardNameEffect")
            && modal_debug.contains("CastSpellsMatching")
            && modal_debug.contains("name: Some")
            && modal_debug.contains("{chosen name}")
            && modal_debug.contains("AttackOrBlock")
            && modal_debug.contains("ActivateAbilitiesOf"),
        "Academic Probation should structurally lower both modes, got {modal_debug}"
    );
    assert!(
        rendered_lower.contains("opponents can't cast spells with the chosen name")
            && rendered_lower.contains("until your next turn")
            && rendered_lower.contains("choose target nonland permanent")
            && rendered_lower.contains("it can't attack or block")
            && rendered_lower.contains("activated abilities can't be activated"),
        "expected Academic Probation compiled text to cover both restriction modes, got {rendered}"
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected Academic Probation semantic comparison to clear target, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
pub(super) fn alhammarret_high_arbiter_strict_parser_text_and_structure_regression() {
    assert_oracle_card_parses_strict("Alhammarret, High Arbiter");

    let def = parse_oracle_card_definition("Alhammarret, High Arbiter");
    let abilities_debug = format!("{:#?}", def.abilities);
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    let oracle = oracle_text_by_name()
        .get("Alhammarret, High Arbiter")
        .expect("Alhammarret oracle text");
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            oracle,
            &compiled,
            Some(crate::semantic_compare::EmbeddingConfig {
                dims: 384,
                mismatch_threshold: 0.99,
            }),
        );

    assert!(
        abilities_debug.contains("ChooseCardNameAsEnters")
            && abilities_debug.contains("reveal_opponents_hands: true")
            && abilities_debug.contains("require_nonland_from_revealed_opponents: true")
            && abilities_debug.contains("RuleRestriction")
            && abilities_debug.contains("CastSpellsMatching")
            && abilities_debug.contains("{chosen name}"),
        "Alhammarret should lower to an as-enters card-name choice and chosen-name cast restriction, got {abilities_debug}"
    );
    assert!(
        def.spell_effect.is_none(),
        "Alhammarret's as-enters choice should not lower into a resolving spell effect"
    );
    assert!(
        rendered_lower.contains("each opponent reveals their hand")
            && rendered_lower.contains("you choose the name of a nonland card revealed this way")
            && rendered_lower.contains("opponents can't cast spells with the chosen name"),
        "Alhammarret compiled text should preserve reveal-hand and chosen-name restriction clauses, got {rendered}"
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected Alhammarret semantic comparison to clear target, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
pub(super) fn archon_of_valors_reach_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Archon of Valor's Reach");
    let abilities_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert_eq!(def.name(), "Archon of Valor's Reach");
    assert!(
        abilities_debug.contains("ChooseNamedOptionAsEnters")
            && abilities_debug.contains("artifact")
            && abilities_debug.contains("enchantment")
            && abilities_debug.contains("instant")
            && abilities_debug.contains("sorcery")
            && abilities_debug.contains("planeswalker"),
        "Archon should parse its enters card-type choice into individual options, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CastSpellsMatching")
            && abilities_debug.contains("chosen_card_type: true"),
        "Archon should lower the chosen-type cast ban structurally, got {abilities_debug}"
    );
    assert!(
        rendered.contains("Players can't cast spells of the chosen type"),
        "Archon compiled text should preserve the chosen-type cast restriction, got {rendered}"
    );
}

#[test]
pub(super) fn academic_probation_chosen_name_mode_restricts_only_opponents_matching_spell_name() {
    struct ChooseLightningBolt;

    impl crate::decision::DecisionMaker for ChooseLightningBolt {
        fn decide_text(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::TextInputContext,
        ) -> String {
            "Lightning Bolt".to_string()
        }
    }

    let def = parse_oracle_card_definition("Academic Probation");
    let modal = academic_probation_modal_effect(&def).clone();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = ChooseLightningBolt;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_chosen_modes(Some(vec![0]));

    modal
        .execute(&mut game, &mut ctx)
        .expect("Academic Probation chosen-name mode should resolve");

    let bob_filters = game
        .effect_store
        .cant_effects
        .cast_filters_for_player(bob)
        .expect("Bob should receive a cast restriction");
    assert!(
        bob_filters
            .iter()
            .any(|restriction| restriction.filter.name.as_deref() == Some("Lightning Bolt")),
        "chosen-name mode should restrict Bob from casting Lightning Bolt, got {bob_filters:#?}"
    );
    assert!(
        bob_filters
            .iter()
            .all(|restriction| restriction.filter.name.as_deref() != Some("Opt")),
        "chosen-name mode should not restrict Bob from casting a different spell, got {bob_filters:#?}"
    );
    assert!(
        game.effect_store
            .cant_effects
            .cast_filters_for_player(alice)
            .is_none(),
        "Academic Probation should not restrict its controller from casting the chosen name"
    );

    let lightning = CardDefinitionBuilder::new(CardId::new(), "Lightning Bolt")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .build();
    let opt = CardDefinitionBuilder::new(CardId::new(), "Opt")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .build();
    let bob_lightning = game.create_object_from_definition(&lightning, bob, Zone::Hand);
    let bob_opt = game.create_object_from_definition(&opt, bob, Zone::Hand);
    let alice_lightning = game.create_object_from_definition(&lightning, alice, Zone::Hand);
    assert!(
        !crate::decision::can_cast_spell(
            &game,
            bob,
            game.object(bob_lightning)
                .expect("Bob's Lightning Bolt should be in hand"),
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Bob should not be able to cast the chosen Lightning Bolt"
    );
    assert!(
        crate::decision::can_cast_spell(
            &game,
            bob,
            game.object(bob_opt).expect("Bob's Opt should be in hand"),
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Bob should still be able to cast a spell with a different name"
    );
    assert!(
        crate::decision::can_cast_spell(
            &game,
            alice,
            game.object(alice_lightning)
                .expect("Alice's Lightning Bolt should be in hand"),
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Academic Probation should not stop its controller from casting the chosen name"
    );
}

#[test]
pub(super) fn academic_probation_target_permanent_mode_restricts_only_chosen_permanent() {
    let def = parse_oracle_card_definition("Academic Probation");
    let modal = academic_probation_modal_effect(&def).clone();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target = CardDefinitionBuilder::new(CardId::new(), "Target Grizzly")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other = CardDefinitionBuilder::new(CardId::new(), "Other Grizzly")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_definition(&target, bob, Zone::Battlefield);
    let other_id = game.create_object_from_definition(&other, bob, Zone::Battlefield);
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_chosen_modes(Some(vec![1]))
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target_permanent(),
            range: 0..1,
        }]);

    modal
        .execute(&mut game, &mut ctx)
        .expect("Academic Probation target-permanent mode should resolve");

    assert!(
        !game.can_attack(target_id),
        "target should not be able to attack"
    );
    assert!(
        !game.can_block(target_id),
        "target should not be able to block"
    );
    assert!(
        !game.can_activate_abilities_of(target_id),
        "target's activated abilities should not be activatable"
    );
    assert!(
        game.can_attack(other_id),
        "other creature should still be able to attack"
    );
    assert!(
        game.can_block(other_id),
        "other creature should still be able to block"
    );
    assert!(
        game.can_activate_abilities_of(other_id),
        "other creature's activated abilities should remain unrestricted"
    );
}

#[test]
pub(super) fn archon_of_valors_reach_blocks_only_spells_of_the_chosen_card_type() {
    use crate::card::CardBuilder;
    use crate::decision::{LegalAction, compute_legal_actions};

    let archon = parse_oracle_card_definition("Archon of Valor's Reach");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(bob)
        .expect("Bob should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let archon_in_hand = game.create_object_from_definition(&archon, alice, Zone::Hand);
    let archon_id = game
        .move_object_with_etb_processing(archon_in_hand, Zone::Battlefield)
        .expect("Archon should enter with its card-type choice")
        .new_id;
    assert_eq!(
        game.chosen_card_type(archon_id),
        Some(CardType::Artifact),
        "Archon's enters choice should record the selected card type"
    );

    let instant = CardBuilder::new(CardId::from_raw(91_001), "Bob Instant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Instant])
        .build();
    let instant_id = game.create_object_from_card(&instant, bob, Zone::Hand);
    let sorcery = CardBuilder::new(CardId::from_raw(91_002), "Bob Sorcery")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, bob, Zone::Hand);

    let has_cast_action = |actions: &[LegalAction], spell_id| {
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: action_spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *action_spell_id == spell_id
            )
        })
    };

    game.set_chosen_card_type(archon_id, CardType::Instant);
    game.refresh_continuous_state();
    let instant_banned_actions = compute_legal_actions(&game, bob);
    assert!(
        !has_cast_action(&instant_banned_actions, instant_id),
        "Archon choosing instant should remove instant spell cast actions, got {instant_banned_actions:?}"
    );
    assert!(
        has_cast_action(&instant_banned_actions, sorcery_id),
        "Archon choosing instant should still allow nonchosen sorcery spells, got {instant_banned_actions:?}"
    );

    game.set_chosen_card_type(archon_id, CardType::Sorcery);
    game.refresh_continuous_state();
    let sorcery_banned_actions = compute_legal_actions(&game, bob);
    assert!(
        has_cast_action(&sorcery_banned_actions, instant_id),
        "Archon choosing sorcery should still allow nonchosen instant spells, got {sorcery_banned_actions:?}"
    );
    assert!(
        !has_cast_action(&sorcery_banned_actions, sorcery_id),
        "Archon choosing sorcery should remove sorcery spell cast actions, got {sorcery_banned_actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_your_opponents_cant_cast_noncreature_spells_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Noncreature Silence Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Your opponents can't cast noncreature spells this turn.")
        .expect("this-turn noncreature-spell restriction should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("some(") && spell_debug.contains("canteffect"),
        "expected cant effect for noncreature-spell restriction, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("excluded_card_types") && spell_debug.contains("creature"),
        "expected noncreature spell filter, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_may_cast_tagged_card_without_paying_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cast Tagged Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile the top card of target player's library. That player may cast that card without paying its mana cost.",
        )
        .expect("target-player may-cast-tagged clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("casttaggedeffect"),
        "expected cast-tagged effect in spell text, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exiled_cards_owner_may_cast_tagged_card_without_paying_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spell Queller Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, exile target spell with mana value 4 or less.\n\
             When this creature leaves the battlefield, the exiled card's owner may cast that card without paying its mana cost.",
        )
        .expect("exiled-card-owner may-cast-tagged clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("mayeffect")
            && abilities_debug.contains("casttaggedeffect")
            && abilities_debug.contains("ownerof")
            && abilities_debug.contains("__source_exiled"),
        "expected exiled-card owner free-cast effect, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_stolen_goods_uses_consult_target_opponent_path() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Stolen Goods")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent exiles cards from the top of their library until they exile a nonland card. Until end of turn, you may cast that card without paying its mana cost.",
        )
        .expect("stolen goods should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("ConsultTopOfLibraryEffect"),
        "expected consult-top-of-library lowering, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("GrantPlayTaggedEffect")
            && spell_debug.contains("GrantTaggedSpellFreeCastUntilEndOfTurnEffect"),
        "expected until-end-of-turn free-cast grant, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("Target(\n                                Opponent")
            || spell_debug.contains("Target(Opponent)"),
        "expected targeted-opponent player binding, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("ChooseObjectsEffect"),
        "expected consult lowering instead of raw library chooser, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_chaos_wand_uses_consult_cast_bottom_path() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Chaos Wand")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{4}, {T}: Target opponent exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order.",
        )
        .expect("chaos wand should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ConsultTopOfLibraryEffect"),
        "expected consult-top-of-library lowering, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CastTaggedEffect")
            || abilities_debug.contains("GrantPlayTaggedEffect"),
        "expected consult cast permission, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected consult remainder bottom effect, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("ChooseObjectsEffect"),
        "expected consult lowering instead of raw library chooser, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_ryan_sinclair_uses_dynamic_consult_gate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ryan Sinclair")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever Ryan attacks, exile cards from the top of your library until you exile a nonland card. You may cast the exiled card without paying its mana cost if it's a spell with mana value less than or equal to Ryan's power. Put the exiled cards not cast this way on the bottom of your library in a random order.",
        )
        .expect("ryan sinclair should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("CastTaggedEffect"),
        "expected immediate tagged cast in consult follow-up, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected exiled remainder to return to library bottom, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_house_cartographer_uses_postcombat_consult_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "House Cartographer")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Survival — At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        )
        .expect("house cartographer should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("phase_type: Postcombat"),
        "expected second-main trigger lowering, got {abilities_debug}"
    );
    assert!(
        def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                triggered.intervening_if == Some(crate::ConditionExpr::SourceIsTapped)
            }
            _ => false,
        }),
        "expected tapped intervening-if predicate, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ConsultTopOfLibraryEffect"),
        "expected consult-top-of-library effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected consult remainder-bottom effect, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("RevealCardsEffect"),
        "expected consult lowering instead of generic reveal effect, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("put that card into your hand")
            && (rendered.contains("put the rest on the bottom of your library in a random order")
                || rendered
                    .contains("and the rest on the bottom of your library in a random order")),
        "expected consult render cleanup for hand-and-bottom wording, got {rendered}"
    );
    assert!(
        !rendered.contains("return it to its owner's hand")
            && !rendered.contains("remaining tagged cards"),
        "expected no internal consult phrasing leaks, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_solstice_revelations_uses_dynamic_consult_gate_with_hand_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Solstice Revelations")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is less than the number of Mountains you control. If you don't cast that card this way, put it into your hand.\nFlashback {6}{R}",
        )
        .expect("solstice revelations should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("Condition::ValueComparison")
            || spell_debug.contains("ValueComparison"),
        "expected mana-value gate in consult follow-up, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("MoveToZoneEffect") && spell_debug.contains("zone: Hand"),
        "expected declined-cast hand fallback, got {spell_debug}"
    );
}

#[test]
pub(super) fn parse_oracle_illuna_apex_of_wishes_strictly_parses_mutate_trigger() {
    let def = parse_oracle_card_definition("Illuna, Apex of Wishes");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Mutate {3}{R/G}{U}{U}")
            && (rendered.contains("When this creature mutates")
                || rendered.contains("Whenever this creature mutates")),
        "expected Illuna mutate keyword and mutate trigger in compiled text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_illuna_apex_of_wishes_compiles_battlefield_or_hand_clause() {
    let def = parse_oracle_card_definition("Illuna, Apex of Wishes");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("you may put it onto the battlefield")
            && (rendered_lower.contains("if you don't, return it to its owner's hand")
                || rendered_lower.contains("if you dont, return it to its owner's hand")
                || rendered_lower.contains("if you don't, put it into its owner's hand")
                || rendered_lower.contains("if you dont, put it into its owner's hand"))
            && !rendered_lower.contains("exile the top card of your library"),
        "expected consult branch over nonland permanent and no top-card fallback text, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ConsultTopOfLibrary")
            && abilities_debug.contains("mode: Exile")
            && abilities_debug.contains("MayEffect")
            && abilities_debug.contains("DidNot")
            && abilities_debug.contains("zone: Battlefield")
            && abilities_debug.contains("zone: Hand"),
        "expected consult + battlefield-or-hand fallback lowering, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_synthesis_pod_accepts_that_exiled_card_cast_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Synthesis Pod")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "({U/P} can be paid with either {U} or 2 life.)\n{1}{U/P}, {T}, Exile a spell you control: Target opponent reveals cards from the top of their library until they reveal a card with mana value equal to 1 plus the exiled spell's mana value. Exile that card, then that player shuffles. You may cast that exiled card without paying its mana cost.",
        )
        .expect("synthesis pod should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("CastTaggedEffect"),
        "expected tagged cast effect from exiled-card clause, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_the_rest_on_bottom_with_previous_put_into_hand() {
    let _def = CardDefinitionBuilder::new(CardId::from_raw(1), "Put Rest Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Look at the top three cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
        )
        .expect("put-the-rest-on-bottom follow-up should parse as part of put clause");
}

#[test]
pub(super) fn parse_oracle_quandrix_apprentice_uses_looked_land_choice_and_bottom_remainder() {
    let def = parse_oracle_card_definition("Quandrix Apprentice");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("you may reveal a land card from among them")
            && (rendered_lower.contains("put that card into your hand")
                || rendered_lower.contains("put it into your hand")),
        "expected looked-card land choice wording, got {rendered}"
    );
    assert!(
        rendered_lower.contains("put the rest on the bottom of your library in any order"),
        "expected looked-card remainder ordering to stay intact, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("SpellCastTrigger")
            && abilities_debug.contains("SpellCopiedTrigger"),
        "expected both magecraft trigger branches, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected filtered looked-card choice plus explicit library-bottom remainder handling, got {abilities_debug}"
    );
    assert!(
        !rendered.contains("__sentence_helper"),
        "expected Quandrix Apprentice compiled text to avoid leaking helper tags, got {rendered}"
    );
    assert!(
        abilities_debug.contains("LookAtTopCardsEffect")
            && !abilities_debug.contains("RevealTopEffect")
            && !abilities_debug.contains("ForEachObject"),
        "expected Quandrix Apprentice to keep looked-card lowering instead of reveal fallback helpers, got {abilities_debug}"
    );
}

#[test]
pub(super) fn parse_oracle_expressive_iteration_splits_looked_cards_by_destination() {
    let def = parse_oracle_card_definition("Expressive Iteration");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("put one of them into your hand")
            && rendered_lower.contains("put one of them on the bottom of your library")
            && rendered_lower.contains("exile one of them")
            && rendered_lower.contains("you may play the exiled card this turn"),
        "expected looked-card split destinations and exiled-card permission, got {rendered}"
    );

    let spell_debug = format!("{:#?}", def.spell_effect);
    let choose_count = spell_debug.matches("ChooseObjectsEffect").count();
    assert!(
        choose_count >= 3
            && spell_debug.contains("GrantPlayTaggedEffect")
            && spell_debug.contains("IsNotTaggedObject"),
        "expected distinct looked-card choices before destination moves, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_when_this_creature_becomes_blocked_may_untap_and_remove_from_combat() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gustcloak Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying\nWhenever this creature becomes blocked, you may untap it and remove it from combat.")
        .expect("becomes-blocked untap-and-remove-from-combat trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("removefromcombateffect"),
        "expected remove-from-combat effect in triggered ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_you_gain_protection_from_everything_until_your_next_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "The One Ring")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "When The One Ring enters, if you cast it, you gain protection from everything until your next turn.",
        )
        .expect("player protection-from-everything trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("betargetedplayer"),
        "expected temporary cant-target-player restriction, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("preventalldamagetotargeteffect"),
        "expected temporary prevent-all-damage-to-player effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("sourcewascast"),
        "expected intervening-if 'you cast it' condition, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_the_stasis_coffin_gain_protection_from_everything_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "The Stasis Coffin")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .parse_text("{2}, {T}, Exile The Stasis Coffin: You gain protection from everything until your next turn.")
        .expect("The Stasis Coffin text should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("exileeffect"),
        "expected self-exile activation cost, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("betargetedplayer"),
        "expected temporary cant-target-player restriction, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("preventalldamagetotargeteffect"),
        "expected temporary prevent-all-damage-to-player effect, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you gain protection from everything until your next turn"),
        "expected Stasis Coffin protection wording, got {rendered}"
    );
    assert!(
        !rendered.contains("you can't be targeted until your next turn")
            && !rendered
                .contains("prevent all damage that would be dealt to you until your next turn"),
        "expected the expanded protection wording to normalize away, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lose_half_your_life_rounded_up_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cruel Bargain")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw four cards. You lose half your life, rounded up.")
        .expect("half-life loss clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("halflifetotalroundedup"),
        "expected half-life rounded-up value in lose-life effect, got {spell_debug}"
    );
}

#[test]
pub(super) fn parse_oracle_liquimetal_coating_type_addition_render_regression() {
    let def = parse_oracle_card_definition("Liquimetal Coating");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("addcardtypes") && raw.contains("artifact"),
        "expected raw compiled definition to keep artifact type addition, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "target permanent becomes an artifact in addition to its other types until end of turn"
        ),
        "expected Liquimetal Coating type-addition wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Liquimetal Coating to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_repel_the_abominable_prevention_regression() {
    let def = parse_oracle_card_definition("Repel the Abominable");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("preventalldamageeffect")
            && raw.contains("from_source")
            && raw.contains("excluded_subtypes")
            && raw.contains("human"),
        "expected Repel the Abominable to keep a non-Human source damage filter, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent all damage")
            && rendered.contains("that would be dealt this turn")
            && rendered.contains("non-human")
            && rendered.contains("sources"),
        "expected Repel the Abominable prevention wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported"),
        "expected Repel the Abominable to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_encroaching_mycosynth_type_addition_regression() {
    let def = parse_oracle_card_definition("Encroaching Mycosynth");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.matches("addcardtypes").count() >= 3 && raw.contains("artifact"),
        "expected battlefield, stack, and off-battlefield artifact type addition, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "nonland permanents you control are artifacts in addition to their other types"
        ),
        "expected battlefield clause to render, got {rendered}"
    );
    assert!(
        rendered.contains("the same is true for permanent spells you control")
            || rendered.contains(
                "permanent spells you control are artifacts in addition to their other types"
            ),
        "expected stack clause to render, got {rendered}"
    );
    assert!(
        (rendered.contains("nonland permanent cards in your hand")
            && rendered.contains("nonland permanent cards in your library")
            && rendered.contains("nonland permanent cards in your graveyard")
            && rendered.contains("are artifacts in addition to their other types"))
            || (rendered
                .contains("nonland permanent cards you own that aren't on the battlefield")
                && rendered.contains("are artifacts in addition to their other types")),
        "expected off-battlefield clause to render, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Encroaching Mycosynth to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_roshan_hidden_magister_regression() {
    let def = parse_oracle_card_definition("Roshan, Hidden Magister");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.matches("addsubtypes").count() >= 3 && raw.contains("assassin"),
        "expected Roshan to add Assassin subtype on battlefield, stack, and off-battlefield zones, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("other creatures you control are assassins in addition to their other types"),
        "expected Roshan battlefield clause to render, got {rendered}"
    );
    assert!(
        rendered
            .contains("creature spells you control are assassins in addition to their other types")
            || rendered.contains("the same is true for creature spells you control"),
        "expected Roshan stack clause to render, got {rendered}"
    );
    assert!(
        (rendered.contains("creature cards in your hand")
            && rendered.contains("creature cards in your library")
            && rendered.contains("creature cards in your graveyard")
            && rendered.contains("creature cards in your exile")
            && rendered.contains("creature cards in your command zone")
            && rendered.contains("are assassins in addition to their other types"))
            || (rendered.contains("creature cards you own that aren't on the battlefield")
                && rendered.contains("are assassins in addition to their other types")),
        "expected Roshan off-battlefield clause to render, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Roshan to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_dune_chanter_keeps_control_and_ownership_domains_independent() {
    let def = parse_oracle_card_definition("Dune Chanter");
    let rendered = unprocessed_compiled_lines(&def);

    assert_eq!(
        rendered,
        vec![
            "Reach".to_string(),
            "Lands you control and land cards you own that aren't on the battlefield are Deserts in addition to their other types.".to_string(),
            "Lands you control have \"{T}: Add one mana of any color.\"".to_string(),
            "{T}: Mill two cards. You gain 1 life for each land card milled this way.".to_string(),
        ]
    );
    let desert_filter = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            match &model.payload {
                ironsmith_core::StaticAbilityPayload::AddSubtypes { filter, subtypes }
                    if subtypes.contains(&Subtype::Desert) =>
                {
                    Some(filter)
                }
                _ => None,
            }
        })
        .expect("Dune Chanter should have its Desert type-addition filter");
    assert!(
        desert_filter.has_conjunctive_set_surface()
            && desert_filter.any_of.len() == 6
            && desert_filter.any_of.iter().any(|branch| {
                branch.zone == Some(Zone::Battlefield)
                    && branch.controller == Some(PlayerFilter::You)
                    && branch.owner.is_none()
                    && branch.card_types == [CardType::Land]
            })
            && [
                Zone::Hand,
                Zone::Library,
                Zone::Graveyard,
                Zone::Exile,
                Zone::Command,
            ]
            .into_iter()
            .all(|zone| desert_filter.any_of.iter().any(|branch| {
                branch.zone == Some(zone)
                    && branch.owner == Some(PlayerFilter::You)
                    && branch.controller.is_none()
                    && branch.card_types == [CardType::Land]
            })),
        "expected separately scoped battlefield/controller and nonbattlefield/owner branches: {desert_filter:#?}"
    );
}

#[test]
pub(super) fn parse_oracle_word_of_undoing_preserves_the_shared_owner_destination() {
    let def = parse_oracle_card_definition("Word of Undoing");

    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![
            "Return target creature and all white Auras you own attached to it to their owners' hands."
                .to_string(),
        ]
    );
}

#[test]
pub(super) fn parse_oracle_eaten_by_spiders_keeps_the_target_and_its_attachments_linked() {
    let def = parse_oracle_card_definition("Eaten by Spiders");

    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![
            "Destroy target creature with flying and all Equipment attached to that creature."
                .to_string(),
        ]
    );
    let raw = format!("{def:#?}");
    assert!(
        raw.contains("AttachedToTaggedObject") && raw.contains("DestroyEffect"),
        "expected separately executable target and attachment destruction linked by tag: {raw}"
    );
}

#[test]
pub(super) fn eaten_by_spiders_destroys_the_target_and_attached_equipment_at_runtime() {
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};

    let def = parse_oracle_card_definition("Eaten by Spiders");
    let effects = def.spell_effect.as_ref().expect("spell effects");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let flier = CardDefinitionBuilder::new(CardId::from_raw(72_010), "Target Flier")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .flying()
        .build();
    let flier_id = game.create_object_from_definition(&flier, bob, Zone::Battlefield);
    create_attached_test_equipment(&mut game, alice, flier_id);
    let unattached_equipment =
        CardDefinitionBuilder::new(CardId::from_raw(72_011), "Unattached Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
    let unattached_id =
        game.create_object_from_definition(&unattached_equipment, alice, Zone::Battlefield);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Object(flier_id)]);
    for effect in effects {
        execute_effect(&mut game, effect, &mut ctx)
            .unwrap_or_else(|error| panic!("Eaten by Spiders should resolve: {error:?}"));
    }

    let alice_graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice")
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect();
    let bob_graveyard_names: Vec<_> = game
        .player(bob)
        .expect("bob")
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect();
    assert!(
        alice_graveyard_names.contains(&"Vibranium Shield"),
        "the attached Equipment should be destroyed, got {alice_graveyard_names:?}"
    );
    assert!(
        bob_graveyard_names.contains(&"Target Flier"),
        "the target creature should be destroyed, got {bob_graveyard_names:?}"
    );
    assert!(
        game.object(unattached_id).is_some(),
        "unattached Equipment should remain on the battlefield"
    );
}

#[test]
pub(super) fn parse_oracle_leyline_of_transformation_regression() {
    let def = parse_oracle_card_definition("Leyline of Transformation");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::PregameAction),
        "expected Leyline opening-hand pregame ability, got {ids:?}"
    );
    assert!(
        ids.contains(&StaticAbilityId::ChooseCreatureTypeAsEnters),
        "expected Leyline to choose a creature type as it enters, got {ids:?}"
    );
    assert_eq!(
        ids.iter()
            .filter(|id| **id == StaticAbilityId::AddChosenCreatureType)
            .count(),
        3,
        "expected battlefield, stack, and off-battlefield chosen-type static abilities, got {ids:?}"
    );

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.matches("addchosencreaturetype").count() >= 3,
        "expected Leyline to lower chosen-type additions structurally, got {raw}"
    );

    let compiled_lines = unprocessed_compiled_lines(&def);
    assert_eq!(
        compiled_lines,
        vec![
            "If this card is in your opening hand, you may begin the game with it on the battlefield.".to_string(),
            "As this enchantment enters, choose a creature type.".to_string(),
            "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.".to_string(),
        ],
        "expected Leyline compiled text to match oracle wording"
    );

    let rendered = compiled_lines.join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("as this enchantment enters, choose a creature type"),
        "expected Leyline choose-as-enters wording to keep the enchantment self-reference, got {rendered}"
    );
    assert!(
        rendered
            .contains("creatures you control are the chosen type in addition to their other types"),
        "expected Leyline battlefield chosen-type clause to render, got {rendered}"
    );
    assert!(
        rendered.contains(
            "the same is true for creature spells you control and creature cards you own that aren't on the battlefield"
        ),
        "expected Leyline chosen-type clauses to merge through same-is-true wording, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported"),
        "expected Leyline to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_leyline_of_the_guildpact_static_characteristics_regression() {
    let def = parse_oracle_card_definition("Leyline of the Guildpact");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("setcolors")
            && raw.contains("addsubtypes")
            && raw.contains("plains")
            && raw.contains("island")
            && raw.contains("swamp")
            && raw.contains("mountain")
            && raw.contains("forest")
            && !raw.contains("desert")
            && !raw.contains("gate"),
        "expected all-colors and exact basic land subtype static abilities, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "if this card is in your opening hand, you may begin the game with it on the battlefield"
        ),
        "expected simple pregame battlefield text to use pronoun surface, got {rendered}"
    );
    assert!(
        rendered.contains("each nonland permanent you control is all colors"),
        "expected all-colors static text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "lands you control are every basic land type in addition to their other types"
        ),
        "expected every-basic-land-type static text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported effect"),
        "expected Leyline of the Guildpact to avoid unsupported markers, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_dispossess_typed_card_name_regression() {
    let def = parse_oracle_card_definition("Dispossess");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("choosecardnameeffect")
            && raw.contains("filter: some")
            && raw.contains("artifact"),
        "expected raw compiled definition to retain typed card-name choice, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose an artifact card name"),
        "expected Dispossess to preserve the typed name-choice clause, got {rendered}"
    );
    assert!(
        rendered.contains("search target opponent's graveyard, hand, and library")
            && (rendered.contains("with that name")
                || rendered.contains("with the same name as that object"))
            && rendered.contains("exile"),
        "expected Dispossess search-and-exile wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifact you control in the battlefield")
            && !rendered.contains("exile you"),
        "expected Dispossess to avoid battlefield-target fallback text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_infinite_obliteration_typed_card_name_regression() {
    let def = parse_oracle_card_definition("Infinite Obliteration");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("choosecardnameeffect")
            && raw.contains("filter: some")
            && raw.contains("creature"),
        "expected raw compiled definition to retain typed card-name choice, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose a creature card name"),
        "expected Infinite Obliteration to preserve the typed name-choice clause, got {rendered}"
    );
    assert!(
        rendered.contains("search target opponent's graveyard, hand, and library")
            && (rendered.contains("with that name")
                || rendered.contains("with the same name as that object"))
            && rendered.contains("exile"),
        "expected Infinite Obliteration search-and-exile wording, got {rendered}"
    );
    assert!(
        !rendered.contains("creature you control in the battlefield")
            && !rendered.contains("exile you"),
        "expected Infinite Obliteration to avoid battlefield-target fallback text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_human_frailty_targeted_subtype_regression() {
    let def = parse_oracle_card_definition("Human Frailty");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("destroyeffect") && raw.contains("human") && raw.contains("creature"),
        "expected raw compiled definition to keep Human-target destroy effect, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target")
            && rendered.contains("human")
            && rendered.contains("creature"),
        "expected Human Frailty to preserve Human-target destroy wording, got {rendered}"
    );
    assert!(
        !rendered.contains("destroy this spell"),
        "expected Human Frailty to avoid self-destroy fallback text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_barkweave_crusher_enlist_render_regression() {
    let def = parse_oracle_card_definition("Barkweave Crusher");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("enlistattack")
            && raw.contains("enlisted_creature")
            && raw.contains("powerof"),
        "expected a typed enlist attack cost with its linked power trigger, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enlist"),
        "expected Barkweave Crusher to keep the enlist marker, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_enlist_action_and_combat_history_regressions() {
    let guardian = parse_oracle_card_definition("Guardian of New Benalia");
    let guardian_raw = format!("{guardian:#?}");
    assert!(
        guardian_raw.contains("KeywordActionTrigger")
            && guardian_raw.contains("Enlist")
            && guardian_raw.contains("ScryEffect"),
        "expected Guardian's enlist-action trigger and scry effect to remain typed, got {guardian_raw}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&guardian).join("\n"),
        "Enlist\nWhenever this creature enlists a creature, scry 2.\nDiscard a card: This creature gains indestructible until end of turn. Tap it."
    );

    let aradesh = parse_oracle_card_definition("Aradesh, the Founder");
    let aradesh_raw = format!("{aradesh:#?}");
    let aradesh_compact = aradesh_raw.split_whitespace().collect::<String>();
    assert!(
        aradesh_raw.contains("TriggeringObjectEnlistedThisCombat")
            && aradesh_compact.contains("target_spec:Some(Tagged(TagKey(\"triggering\",),),)")
            && aradesh_raw.contains("DoubleStrike"),
        "expected Aradesh to gate and affect the triggering enlisted attacker, got {aradesh_raw}"
    );
    let rendered = unprocessed_compiled_lines(&aradesh).join(" ");
    assert!(
        rendered.contains("if it enlisted a creature this combat")
            && rendered.contains("it gains double strike until end of turn")
            && rendered.contains("its power is 4 or greater"),
        "expected Aradesh to preserve enlist history, attacker reference, and power threshold, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_war_report_summed_battlefield_count_regression() {
    let def = parse_oracle_card_definition("War Report");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("add(") && raw.contains("creature") && raw.contains("artifact"),
        "expected raw compiled definition to keep the summed creature-plus-artifact value, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gain life equal to")
            && rendered.contains("number of creatures")
            && rendered.contains("number of artifacts")
            && rendered.contains("plus"),
        "expected War Report to preserve the summed battlefield-count wording, got {rendered}"
    );
    assert!(
        !rendered.contains("creature artifact"),
        "expected War Report to avoid collapsed creature-artifact count wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_descent_into_avernus_scaling_trigger_regression() {
    let def = parse_oracle_card_definition("Descent into Avernus");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("named(")
            && raw.contains("descent")
            && (raw.contains("countersonsource(") || raw.contains("counterson("))
            && raw.contains("treasure"),
        "expected raw compiled definition to keep descent counters and treasure scaling, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("put two descent counters"),
        "expected Descent into Avernus to keep named counter wording, got {rendered}"
    );
    assert!(
        rendered.contains("treasure")
            && rendered.contains("descent counter")
            && rendered.contains("each player"),
        "expected Descent into Avernus to keep treasure scaling text, got {rendered}"
    );
    assert!(
        rendered.contains("damage")
            && rendered.contains("descent counter")
            && (rendered.contains("that player") || rendered.contains("each player")),
        "expected Descent into Avernus to keep damage scaling text, got {rendered}"
    );
    assert!(
        !rendered.contains("put two this counters")
            && !rendered.contains("for each enchantment under that player's control"),
        "expected Descent into Avernus to avoid the old fallback wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_ugins_insight_where_x_tail_regression() {
    let def = parse_oracle_card_definition("Ugin's Insight");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("greatestmanavalue")
            && raw.contains("artifact")
            && raw.contains("creature")
            && raw.contains("enchantment")
            && raw.contains("planeswalker")
            && raw.contains("drawcardseffect"),
        "expected raw compiled definition to keep greatest-mana-value scry and draw, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "scry the greatest mana value among permanents you control, then draw 3 cards"
        ) || rendered.contains(
            "scry the greatest mana value among permanents you control, then draw three cards"
        ) || rendered.contains(
            "scry x, where x is the greatest mana value among permanents you control, then draw three cards"
        ),
        "expected Ugin's Insight to preserve both scry and draw clauses, got {rendered}"
    );
    assert!(
        !rendered.eq("spell effects: scry the greatest mana value among permanents you control."),
        "expected Ugin's Insight to avoid dropping the draw clause, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_wan_shi_tong_half_x_draw_regression() {
    let def = parse_oracle_card_definition("Wan Shi Tong, Librarian");

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("HalfRoundedDown") && raw.contains("X"),
        "expected Wan Shi Tong to keep half-X draw semantics, got {raw}"
    );
    assert!(
        raw.contains("PlayerSearchesLibraryTrigger") && raw.contains("player: Opponent"),
        "expected Wan Shi Tong to keep the opponent-search trigger, got {raw}"
    );
    assert!(
        raw.contains("target: Source") || raw.contains("spec: Source"),
        "expected Wan Shi Tong to put counters on itself, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("half x") && rendered_lower.contains("rounded down"),
        "expected Wan Shi Tong compiled text to preserve half-X draw wording, got {rendered}"
    );
    assert!(
        rendered_lower.contains("when wan shi tong enters, put x +1/+1 counters on him")
            && rendered_lower.contains("draw half x")
            && rendered_lower.contains("rounded down")
            && rendered_lower.contains("cards"),
        "expected Wan Shi Tong ETB text to normalize the rounded-down draw clause, got {rendered}"
    );
    assert!(
        rendered_lower.contains("whenever an opponent searches their library")
            && (rendered_lower.contains("put a +1/+1 counter on this creature")
                || rendered_lower.contains("put a +1/+1 counter on wan shi tong"))
            && rendered_lower.contains("draw a card"),
        "expected Wan Shi Tong trigger text to read like oracle text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_smothering_tithe_player_doesnt_regression() {
    let def = parse_oracle_card_definition("Smothering Tithe");

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("DidNot"),
        "expected Smothering Tithe to preserve the negative pay result gate, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("that player may pay {2}") || lower.contains("player may pay {2}"),
        "expected Smothering Tithe to keep the payment choice, got {rendered}"
    );
    assert!(
        lower.contains("doesn't") || lower.contains("dont"),
        "expected Smothering Tithe to render the negative follow-up, got {rendered}"
    );
    assert!(
        rendered.contains("If that player doesn't, you create a Treasure token")
            || rendered.contains("If that player doesn't, create a Treasure token"),
        "expected Smothering Tithe to keep the Treasure creation clause, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_shuffle_trigger_regressions() {
    let cosi = parse_oracle_card_definition("Cosi's Trickster");
    let cosi_raw = format!("{cosi:#?}");
    assert!(
        cosi_raw.contains("PlayerShufflesLibraryTrigger")
            && cosi_raw.contains("player: Opponent")
            && cosi_raw.contains("caused_by_effect: false"),
        "expected Cosi's Trickster to compile as an opponent-shuffle trigger, got {cosi_raw}"
    );

    let probe = parse_oracle_card_definition("Psychogenic Probe");
    let probe_raw = format!("{probe:#?}");
    assert!(
        probe_raw.contains("PlayerShufflesLibraryTrigger")
            && probe_raw.contains("player: Any")
            && probe_raw.contains("caused_by_effect: true")
            && probe_raw.contains("source_controller_shuffles: false"),
        "expected Psychogenic Probe to compile as an effect-caused shuffle trigger, got {probe_raw}"
    );

    let panic = parse_oracle_card_definition("Widespread Panic");
    let panic_raw = format!("{panic:#?}");
    assert!(
        panic_raw.contains("PlayerShufflesLibraryTrigger")
            && panic_raw.contains("caused_by_effect: true")
            && panic_raw.contains("source_controller_shuffles: true"),
        "expected Widespread Panic to compile as a controller-shuffles trigger, got {panic_raw}"
    );

    let rendered = unprocessed_compiled_lines(&panic).join(" ");
    assert!(
        rendered
            .contains("Whenever a spell or ability causes its controller to shuffle their library"),
        "expected Widespread Panic compiled text to preserve its shuffle-trigger wording, got {rendered}"
    );
    assert!(
        rendered.contains("that player puts a card from their hand on top of their library")
            || rendered.contains(
                "that player puts a card from their hand on top of that player's library"
            ),
        "expected Widespread Panic to use a pronoun for the shuffling player's hand, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_cyber_controller_milled_collection_regression() {
    let def = parse_oracle_card_definition("The Cyber-Controller");
    let raw = format!("{def:#?}");
    assert!(
        raw.contains("ReturnAllToBattlefieldEffect")
            && raw.contains("Graveyard")
            && raw.contains("face_down: true"),
        "expected The Cyber-Controller to return its tagged milled cards from the graveyard face down, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains(
            "each opponent mills x cards. for each opponent, put all creature cards milled this way onto the battlefield face down under your control. they're 2/2 cyberman artifact creatures"
        ),
        "expected The Cyber-Controller to preserve its linked mill/return/animation sequence, got {rendered}"
    );
    assert!(
        rendered.contains("Other artifact creatures you control get +1/+1"),
        "expected The Cyber-Controller's anthem subject to pluralize creature, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_into_the_flood_maw_gift_regression() {
    let def = parse_oracle_card_definition("Into the Flood Maw");

    assert_eq!(
        def.optional_costs.len(),
        1,
        "expected Into the Flood Maw to lower Gift as one optional cost"
    );
    assert!(
        def.optional_costs[0]
            .source_label
            .to_ascii_lowercase()
            .starts_with("gift a tapped fish"),
        "expected Into the Flood Maw gift label to preserve the gift descriptor, got {:?}",
        def.optional_costs[0].source_label
    );

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("ChoosePlayerEffect")
            && raw.contains("remember_as_chosen_player: true")
            && raw.contains("ChosenPlayer"),
        "expected Into the Flood Maw gift lowering to record and reuse the chosen opponent, got {raw}"
    );
    assert!(
        raw.contains("ThisSpellPaidLabel") && raw.contains("Gift"),
        "expected Into the Flood Maw to preserve the gift-was-promised condition, got {raw}"
    );
    assert!(
        raw.contains("EmitGiftGivenEffect"),
        "expected Into the Flood Maw gift lowering to emit a gift-given event when the gift resolves, got {raw}"
    );
    assert!(
        !raw.contains("YouCastThisSpell"),
        "expected Into the Flood Maw Gift to resolve with the spell instead of a cast trigger, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("Gift a tapped Fish"),
        "expected Into the Flood Maw compiled text to preserve the gift line, got {rendered}"
    );
    assert!(
        rendered.contains("Return target creature an opponent controls")
            && rendered.contains("If the gift was promised")
            && rendered.contains("nonland permanent")
            && rendered.contains("owner"),
        "expected Into the Flood Maw compiled text to normalize the gift branch into oracle order, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("when you cast this spell, if the gift was promised"),
        "expected Into the Flood Maw compiled text to omit the duplicated Gift cast trigger, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("chosen player creates"),
        "expected Into the Flood Maw compiled text to hide the synthetic Gift followup line, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_scrapshooter_gift_etb_regression() {
    let def = parse_oracle_card_definition("Scrapshooter");

    assert_eq!(
        def.optional_costs.len(),
        1,
        "expected Scrapshooter to lower Gift as one optional cost"
    );
    assert!(
        def.optional_costs[0]
            .source_label
            .to_ascii_lowercase()
            .starts_with("gift a card"),
        "expected Scrapshooter gift label to preserve the gift descriptor, got {:?}",
        def.optional_costs[0].source_label
    );

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("ZoneChangeTrigger")
            && raw.contains("Battlefield")
            && raw.contains("this_object: true"),
        "expected Scrapshooter Gift to become an ETB-triggered ability, got {raw}"
    );
    assert!(
        raw.contains("ThisSpellPaidLabel") && raw.contains("Gift") && raw.contains("ChosenPlayer"),
        "expected Scrapshooter Gift ETB trigger to depend on Gift and the chosen player, got {raw}"
    );
    assert!(
        raw.contains("EmitGiftGivenEffect"),
        "expected Scrapshooter Gift ETB trigger to emit a gift-given event when the gift resolves, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("Gift a card"),
        "expected Scrapshooter compiled text to preserve the gift line, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("chosen player draws a card"),
        "expected Scrapshooter compiled text to hide the synthetic Gift ETB line, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_longstalk_brawl_gift_spell_line_keeps_main_effects() {
    let def = parse_oracle_card_definition("Longstalk Brawl");

    assert_eq!(
        def.optional_costs.len(),
        1,
        "expected Longstalk Brawl to lower Gift as one optional cost"
    );
    assert!(
        def.optional_costs[0]
            .source_label
            .to_ascii_lowercase()
            .starts_with("gift a tapped fish"),
        "expected Longstalk Brawl gift label to preserve the gift descriptor, got {:?}",
        def.optional_costs[0].source_label
    );

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("ThisSpellPaidLabel") && raw.contains("Gift"),
        "expected Longstalk Brawl to preserve the gift-was-promised condition, got {raw}"
    );
    assert!(
        raw.contains("PutCountersEffect") && raw.contains("FightEffect"),
        "expected Longstalk Brawl to keep its counter-plus-fight spell effects, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("Gift a tapped Fish"),
        "expected Longstalk Brawl compiled text to preserve the gift line, got {rendered}"
    );
    assert!(
        rendered
            .contains("Choose target creature you control and target creature you don't control")
            && rendered.contains(
                "Put a +1/+1 counter on the creature you control if the gift was promised"
            )
            && rendered.contains("Then those creatures fight each other"),
        "expected Longstalk Brawl compiled text to preserve the choose/counter/fight spell line, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("chosen player creates")
            && !rendered_lower.contains(
                "create a 1/1 blue fish creature token under the chosen player's control"
            ),
        "expected Longstalk Brawl compiled text to hide the synthetic Gift payload line, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_druids_deliverance_prevention_surface_regression() {
    let def = parse_oracle_card_definition("Druid's Deliverance");

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("PreventAllCombatDamageEffect")
            || (raw.contains("PreventAllDamageEffect") && raw.contains("combat_only: true")),
        "expected Druid's Deliverance to compile into combat-damage prevention, got {raw}"
    );
    assert!(
        raw.contains("PopulateEffect"),
        "expected Druid's Deliverance to preserve populate, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Prevent all combat damage that would be dealt to you this turn."),
        "expected Druid's Deliverance compiled text to use oracle-style combat prevention wording, got {rendered}"
    );
    assert!(
        rendered.contains("Populate."),
        "expected Druid's Deliverance compiled text to preserve populate, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_sarkhan_dragon_ascendant_behold_regression() {
    let def = parse_oracle_card_definition("Sarkhan, Dragon Ascendant");
    let raw = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        raw.contains("BeholdEffect"),
        "expected Sarkhan to lower behold into a runtime behold effect, got {raw}"
    );
    assert!(
        rendered.contains("behold a Dragon"),
        "expected Sarkhan compiled text to preserve the behold clause, got {rendered}"
    );
    assert!(
        rendered.contains("When Sarkhan enters"),
        "expected Sarkhan's named ETB self-reference to survive trigger rendering, got {rendered}"
    );
    assert!(
        rendered_lower.contains("if you do, create a treasure token"),
        "expected Sarkhan to preserve the behold follow-up, got {rendered}"
    );
    assert!(
        rendered.contains("Sarkhan becomes a Dragon in addition to its other types"),
        "expected Sarkhan's named self-reference to survive continuous-effect rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn desmond_miles_test_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Desmond Miles")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(1, 3))
        .parse_text(
            "Menace\nDesmond Miles gets +1/+0 for each other Assassin you control and each Assassin card in your graveyard.\nWhenever Desmond Miles deals combat damage to a player, surveil X, where X is the amount of damage it dealt to that player.",
        )
        .expect("Desmond Miles should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_desmond_miles_strict_and_renders_compound_count_and_surveil_x() {
    let def = desmond_miles_test_definition();
    let debug = format!("{def:#?}");
    let compact_debug = debug.split_whitespace().collect::<String>();
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        debug.contains("any_of")
            && debug.contains("other: true")
            && debug.contains("zone: Some(Graveyard)")
            && compact_debug.contains("EventValue(Amount"),
        "expected Desmond Miles to lower compound count and surveil event amount structurally, got {debug}"
    );
    assert!(
        rendered.contains(
            "Desmond Miles gets +1/+0 for each other Assassin you control and each Assassin card in your graveyard."
        ),
        "expected Desmond Miles compound count wording, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever Desmond Miles deals combat damage to a player, surveil X, where X is the amount of damage it dealt to that player."
        ),
        "expected Desmond Miles combat-damage surveil-X wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_nadaar_enters_or_attacks_surface_regression() {
    let def = parse_oracle_card_definition("Nadaar, Selfless Paladin");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("Whenever Nadaar enters or attacks, venture into the dungeon."),
        "expected Nadaar's source-name trigger branches to render as one oracle-style subject, got {rendered}"
    );
    assert!(
        !rendered.contains("or this creature attacks"),
        "expected Nadaar not to mix named and generic source-reference surfaces, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_timeless_lotus_enters_tapped_surface_regression() {
    let def = parse_oracle_card_definition("Timeless Lotus");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("This artifact enters tapped."),
        "expected Timeless Lotus to render source ETB tapped with the card subject, got {rendered}"
    );
    assert!(
        !rendered.contains("source enter tapped"),
        "expected Timeless Lotus not to compile named source as a filtered ETB-tapped ability, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_hyphenated_name_source_reference_regressions() {
    let spider = parse_oracle_card_definition("Spider-Man, Miles Morales");
    let spider_rendered = unprocessed_compiled_lines(&spider).join(" ");
    assert!(
        spider_rendered.contains("Whenever Spider-Man enters or attacks"),
        "expected Spider-Man short-name source trigger to avoid Spider subtype parsing, got {spider_rendered}"
    );
    assert!(
        !spider_rendered.contains("Whenever a Spider enters"),
        "expected Spider-Man not to compile as a Spider subtype ETB trigger, got {spider_rendered}"
    );

    let commander = parse_oracle_card_definition("Commander Greven il-Vec");
    let commander_rendered = unprocessed_compiled_lines(&commander).join(" ");
    assert!(
        commander_rendered.contains("When Commander Greven il-Vec enters"),
        "expected full Commander Greven source name to avoid commander-filter parsing, got {commander_rendered}"
    );
    assert!(
        !commander_rendered.contains("commander permanent enters"),
        "expected Commander Greven not to compile as a commander-permanent ETB trigger, got {commander_rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_possessive_named_source_enters_tapped_regression() {
    let def = parse_oracle_card_definition("Teferi's Isle");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("This land enters tapped."),
        "expected Teferi's Isle to render as a source ETB-tapped ability, got {rendered}"
    );
    assert!(
        !rendered.contains("Teferi enter tapped"),
        "expected Teferi's Isle not to leak a name fragment into the ETB-tapped filter, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_named_source_metadata_surface_regressions() {
    let hall = parse_oracle_card_definition("Hall of Triumph");
    let hall_rendered = unprocessed_compiled_lines(&hall).join(" ");
    assert!(
        hall_rendered.contains("As this artifact enters, choose a color."),
        "expected Hall of Triumph to keep artifact metadata through named-source parsing, got {hall_rendered}"
    );
    assert!(
        !hall_rendered.contains("As this permanent enters"),
        "expected Hall of Triumph not to fall back to oracle-only metadata loss, got {hall_rendered}"
    );

    let shaun = parse_oracle_card_definition("Shaun & Rebecca, Agents");
    let shaun_rendered = unprocessed_compiled_lines(&shaun).join(" ");
    assert!(
        shaun_rendered.contains("When Shaun & Rebecca enters"),
        "expected ampersand source names to preserve creature metadata, got {shaun_rendered}"
    );
    assert!(
        !shaun_rendered.contains("When this permanent enters"),
        "expected Shaun & Rebecca not to fall back to oracle-only metadata loss, got {shaun_rendered}"
    );

    let splinter = parse_oracle_card_definition("Splinter & Leo, Father & Son");
    let splinter_rendered = unprocessed_compiled_lines(&splinter).join(" ");
    assert!(
        splinter_rendered.contains("When Splinter & Leo enters")
            || splinter_rendered.contains("When Splinter & Leo enter"),
        "expected Splinter & Leo to preserve creature metadata despite ampersand source name, got {splinter_rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_polukranos_preserves_named_damage_recipient_regression() {
    let def = parse_oracle_card_definition("Polukranos, World Eater");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains("deals damage equal to its power to Polukranos"),
        "expected Polukranos reciprocal damage clause to keep the named recipient, got {rendered}"
    );
    assert!(
        !rendered.contains("deals damage equal to its power to this creature"),
        "expected named-trigger fallback not to rewrite the effect body recipient, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_osseous_exhale_behold_paid_regression() {
    let def = parse_oracle_card_definition("Osseous Exhale");
    let raw = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        def.optional_costs.len(),
        1,
        "expected Osseous Exhale to lower optional behold as one optional cost"
    );
    assert!(
        def.optional_costs[0]
            .source_label
            .to_ascii_lowercase()
            .starts_with("as an additional cost to cast this spell, you may behold a dragon"),
        "expected Osseous Exhale to preserve the optional behold line, got {:?}",
        def.optional_costs[0].source_label
    );
    assert!(
        raw.contains("ThisSpellPaidLabel") && raw.contains("Behold"),
        "expected Osseous Exhale to preserve the 'was beheld' condition, got {raw}"
    );
    assert!(
        rendered.contains("If a Dragon was beheld, you gain 2 life"),
        "expected Osseous Exhale compiled text to keep the behold payoff, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_caustic_exhale_behold_or_pay_regression() {
    let def = parse_oracle_card_definition("Caustic Exhale");
    let raw = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        raw.contains("ChooseModeEffect") || raw.contains("ChooseMode"),
        "expected Caustic Exhale to lower the behold-or-pay additional cost as a modal cost, got {raw}"
    );
    assert!(
        rendered_lower.contains("behold a dragon or pay {1}")
            || rendered_lower.contains("behold a dragon, or pay {1}"),
        "expected Caustic Exhale compiled text to preserve the behold-or-pay choice, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_cinder_strike_blight_additional_cost_regression() {
    let def = parse_oracle_card_definition("Cinder Strike");
    let raw = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        raw.contains("ThisSpellPaidLabel")
            && raw.contains("Additional")
            && raw.contains("MinusOneMinusOne"),
        "expected Cinder Strike to preserve additional-cost-paid gating and blight counters, got {raw}"
    );
    assert!(
        rendered.contains("As an additional cost to cast this spell")
            && rendered.contains("you may blight 1")
            && rendered.contains("additional cost was paid"),
        "expected Cinder Strike compiled text to keep blight cost and payoff wiring, got {rendered}"
    );
}

#[test]
pub(super) fn mandatory_minus_counter_additional_costs_do_not_render_as_optional_blight() {
    for card_name in ["Lethal Sting", "Scarscale Ritual"] {
        let def = parse_oracle_card_definition(card_name);
        let rendered = unprocessed_compiled_lines(&def).join(" ");
        let rendered_lower = rendered.to_ascii_lowercase();

        assert!(
            rendered_lower.contains(
                "as an additional cost to cast this spell, put a -1/-1 counter on a creature you control"
            ),
            "expected {card_name} to keep its mandatory counter cost, got {rendered}"
        );
        assert!(
            !rendered_lower.contains("you may blight"),
            "mandatory counter cost must not become optional Blight: {rendered}"
        );
    }
}

#[test]
pub(super) fn diabolic_servitude_preserves_linked_creature_trigger_surface() {
    let def = parse_oracle_card_definition("Diabolic Servitude");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "When the creature put onto the battlefield with this enchantment dies, exile it and return this enchantment to its owner's hand"
        ),
        "expected the linked death trigger to keep its definite subject and joined cleanup, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_perch_protection_gift_extra_turn_regression() {
    let def = parse_oracle_card_definition("Perch Protection");

    let raw = format!("{def:#?}");
    assert!(
        raw.contains("ExtraTurnEffect") && raw.contains("ChosenPlayer"),
        "expected Perch Protection Gift to grant the chosen player an extra turn, got {raw}"
    );
    assert!(
        raw.contains("ThisSpellPaidLabel") && raw.contains("Gift"),
        "expected Perch Protection Gift to preserve the gift-was-promised condition, got {raw}"
    );
}

#[test]
pub(super) fn parse_oracle_octomancer_gift_octopus_regression() {
    let def = parse_oracle_card_definition("Octomancer");

    let raw = format!("{def:#?}");
    let raw_lower = raw.to_ascii_lowercase();
    assert!(
        raw.contains("CreateTokenEffect")
            && raw.contains("Octopus")
            && raw.contains("ChosenPlayer"),
        "expected Octomancer Gift to create an Octopus for the chosen player, got {raw}"
    );
    assert!(
        raw_lower.contains("zonechangetrigger")
            && raw_lower.contains("specific")
            && raw_lower.contains("battlefield")
            && raw_lower.contains("thisspellpaidlabel"),
        "expected Octomancer Gift to be an ETB trigger gated by Gift, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("when this creature enters, if the gift was promised, create a 8/8 blue octopus creature token under the chosen player's control"),
        "expected Octomancer compiled text to hide the synthetic Gift ETB line, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_curious_herd_targeted_artifact_count_regression() {
    let def = parse_oracle_card_definition("Curious Herd");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("targetonlyeffect")
            && raw.contains("createtokeneffect")
            && raw.contains("controller: some")
            && raw.contains("opponent")
            && raw.contains("artifact"),
        "expected raw compiled definition to target an opponent and count their artifacts, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose target opponent")
            && rendered.contains("create x 3/3 green beast creature tokens")
            && (rendered.contains("number of artifacts that player controls")
                || rendered.contains("number of artifacts target opponent controls")),
        "expected Curious Herd to preserve the targeted artifact-count token creation, got {rendered}"
    );
    assert!(
        !rendered.contains(
            "create a 3/3 green beast creature token for each artifact target opponent controls"
        ),
        "expected Curious Herd to avoid the collapsed for-each token wording, got {rendered}"
    );
    assert!(
        !rendered.contains("number of tokens you control"),
        "expected Curious Herd to avoid stale x-token fallback wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_drag_to_the_bottom_domain_value_regression() {
    let def = parse_oracle_card_definition("Drag to the Bottom");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("modifypowertoughness")
            && raw.contains("scaled")
            && raw.contains("basiclandtypesamong"),
        "expected raw compiled definition to keep signed domain scaling, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("all creatures get -x/-x until end of turn")
            || rendered.contains("each creature gets -x/-x until end of turn"))
            && rendered.contains(
                "where x is 1 plus the number of basic land types among lands you control"
            ),
        "expected Drag to the Bottom to preserve domain-based -X/-X wording, got {rendered}"
    );
    assert!(
        !rendered.contains("+x/+x")
            && !rendered.contains("where x is -x")
            && !rendered.contains("basic lands you control"),
        "expected Drag to the Bottom to avoid the old signed-x fallback wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_over_the_top_dynamic_reveal_and_distribution_regression() {
    let def = parse_oracle_card_definition("Over the Top");

    let raw = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        raw.contains("forplayerseffect")
            && raw.contains("lookattopcardseffect")
            && raw.contains("excluded_card_types")
            && raw.contains("land")
            && raw.contains("movetozoneeffect"),
        "expected raw compiled definition to reveal top cards per player and distribute them, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player reveals")
            && rendered.contains("number of nonland permanents they control")
            && rendered
                .contains("puts all permanent cards they revealed this way onto the battlefield")
            && rendered.contains("rest into their graveyard"),
        "expected Over the Top to preserve dynamic reveal and battlefield/graveyard distribution, got {rendered}"
    );
    assert!(
        !rendered.contains("reveals the top card of their library")
            && !rendered.contains("return all permanent revealed this way"),
        "expected Over the Top to avoid the single-card reveal fallback wording, got {rendered}"
    );
}

#[test]
pub(super) fn over_the_top_moves_nonpermanents_to_their_owners_graveyards_at_runtime() {
    use crate::card::CardBuilder;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::zone::Zone;

    let def = parse_oracle_card_definition("Over the Top");
    let effects = def.spell_effect.as_ref().expect("spell effects");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let bear = CardBuilder::new(CardId::from_raw(30_001), "Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let rock = CardBuilder::new(CardId::from_raw(30_002), "Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&bear, alice, Zone::Battlefield);
    game.create_object_from_card(&rock, alice, Zone::Battlefield);

    let goblin = CardBuilder::new(CardId::from_raw(30_003), "Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&goblin, bob, Zone::Battlefield);

    let alice_spell = CardBuilder::new(CardId::from_raw(30_004), "Alice Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let alice_perm = CardBuilder::new(CardId::from_raw(30_005), "Alice Permanent")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&alice_spell, alice, Zone::Library);
    game.create_object_from_card(&alice_perm, alice, Zone::Library);

    let bob_spell = CardBuilder::new(CardId::from_raw(30_006), "Bob Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&bob_spell, bob, Zone::Library);

    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in effects {
        execute_effect(&mut game, effect, &mut ctx).expect("execute Over the Top effect");
    }

    let alice_graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        alice_graveyard_names
            .iter()
            .any(|name| name == "Alice Spell"),
        "expected Alice Spell in Alice's graveyard after Over the Top, got {alice_graveyard_names:?}"
    );

    let bob_graveyard_names: Vec<_> = game
        .player(bob)
        .expect("bob")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        bob_graveyard_names.iter().any(|name| name == "Bob Spell"),
        "expected Bob Spell in Bob's graveyard after Over the Top, got {bob_graveyard_names:?}"
    );

    let battlefield_names: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        battlefield_names
            .iter()
            .any(|name| name == "Alice Permanent"),
        "expected Alice Permanent on the battlefield after Over the Top, got {battlefield_names:?}"
    );
}
