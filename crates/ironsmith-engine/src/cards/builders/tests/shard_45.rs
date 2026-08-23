use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn assert_named_inline_entry_counter(
    name: &str,
    counter_debug_name: &str,
    expected_rendered_fragment: &str,
) {
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    let counter_marker = format!("counter_type: {counter_debug_name}");
    let matching_entry_counters = debug
        .split("BattlefieldEntryCounterSpec")
        .skip(1)
        .filter(|entry| {
            entry
                .split_once('}')
                .is_some_and(|(spec, _)| spec.contains(&counter_marker))
        })
        .count();

    assert_eq!(
        matching_entry_counters, 1,
        "{name} should encode its authored {counter_debug_name} counter as one battlefield-entry modifier: {debug}"
    );
    assert!(
        debug.contains(&counter_marker) && debug.contains("surface: Inline"),
        "{name} should retain the counter kind and inline entry timing: {debug}"
    );
    assert!(
        !debug.contains("AdditionalEntryCounter"),
        "{name} says the permanent enters with a counter, not an additional counter: {debug}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(expected_rendered_fragment),
        "{name} should keep the counter inside the battlefield-entry instruction; expected {expected_rendered_fragment:?}, got {rendered:?}"
    );
}

#[test]
fn shellshock_preserves_the_demonstrative_target_set_in_compiled_text() {
    let rendered =
        crate::runtime_display::compiled_text_lines(&parse_oracle_card_definition("Shellshock"))
            .join("\n");

    assert!(
        rendered.contains("Shellshock deals X damage to each of those creatures."),
        "the prior target set must retain its authored demonstrative surface: {rendered}"
    );
    assert!(
        rendered.contains("You create a Mutagen token for each creature dealt damage this way."),
        "the damage-result count must retain its authored per-creature surface: {rendered}"
    );
}

#[test]
fn wargling_keeps_the_while_qualifier_on_the_attack_event() {
    let definition = parse_oracle_card_definition("Wargling");
    let debug = format!("{definition:#?}");
    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");

    assert!(
        debug.contains("ThisAttacksWhileYouControlTrigger"),
        "the ferocious predicate must be part of the attack matcher: {debug}"
    );
    assert!(
        debug.contains("intervening_if: None"),
        "authored 'attacks while' is not an intervening-if check: {debug}"
    );
    assert!(
        rendered.contains(
            "Whenever this creature attacks while you control a creature with power 4 or greater"
        ),
        "the event-time qualifier must round-trip on the trigger surface: {rendered}"
    );
    assert!(!rendered.contains("attacks, if you control"), "{rendered}");
}

#[test]
fn gollum_keeps_named_creature_combat_damage_history_on_each_opponent() {
    let definition = parse_oracle_card_definition("Gollum, Obsessed Stalker");
    let debug = format!("{definition:#?}");
    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");

    assert!(
        debug.contains("WasDealtCombatDamageBySourcesThisGame"),
        "the opponent iterator must retain its full-game combat history filter: {debug}"
    );
    assert!(
        debug.contains("name: Some") && debug.contains("\"gollum obsessed stalker\""),
        "the source filter must retain Gollum's normalized literal name: {debug}"
    );
    assert!(
        rendered.contains("each opponent dealt combat damage this game by a creature named gollum obsessed stalker loses life"),
        "the typed history qualifier must survive compiled text: {rendered}"
    );
}

#[test]
fn gideon_the_oathsworn_counters_the_exact_triggering_attack_group() {
    let definition = parse_oracle_card_definition("Gideon, the Oathsworn");
    let debug = format!("{definition:#?}");
    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");

    assert!(
        debug.contains("one_or_more: true")
            && debug.contains("min_total_attackers: 2")
            && debug.contains(ironsmith_core::ATTACKING_GROUP_TAG),
        "Gideon's trigger and counter target must share the captured attack group: {debug}"
    );
    assert!(
        rendered.contains(
            "Whenever you attack with two or more non-gideon creatures, put a +1/+1 counter on each of those creatures."
        ),
        "the captured group must retain Gideon's authored surface: {rendered}"
    );
    assert!(
        rendered.contains(
            "+2: Until end of turn, gideon becomes a 5/5 white Soldier creature that's still a planeswalker. Prevent all damage that would be dealt to him this turn."
        ),
        "Gideon's animation must retain color, subtype, and planeswalker status: {rendered}\n{debug}"
    );
    assert!(
        rendered.contains("−9: Exile Gideon and each creature your opponents control."),
        "Gideon's ultimate must retain the quantified opponent-controlled set: {rendered}\n{debug}"
    );
}

#[test]
fn zimone_infinite_analyst_reduces_only_the_first_x_spell_each_turn() {
    let definition = parse_oracle_card_definition("Zimone, Infinite Analyst");
    let debug = format!("{definition:#?}");
    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");

    assert!(
        debug.contains("CostReduction")
            && debug.contains("first_spell_cast_each_turn: true")
            && debug.contains("has_x_in_cost: true"),
        "Zimone's reduction must retain both first-spell and X-cost filters: {debug}"
    );
    assert!(
        rendered.contains(
            "The first spell you cast with {X} in its mana cost each turn costs {1} less to cast for each +1/+1 counter on Zimone."
        ),
        "Zimone's typed cost restriction must survive compiled text: {rendered}"
    );
}

#[test]
fn refreshed_restricted_mana_cards_with_supported_transactions_keep_typed_spending_rules() {
    let mut failures = Vec::new();
    for name in [
        "Automated Artificer",
        "Guidelight Optimizer",
        "Soldevi Machinist",
        "Purple Dragon Punks",
        "Sage of the Unknowable",
        "Smokebraider",
        "Vedalken Engineer",
        "Slobad, Iron Goblin",
        "Hargilde, Kindly Runechanter",
        "Dalakos, Crafter of Wonders",
        "Myr Reservoir",
        "Brotherhood Headquarters",
        "Castle Garenbrig",
        "Grand Architect",
        "Renowned Weaponsmith",
        "Oaken Siren",
        "Crucible of the Spirit Dragon",
        "Orb of Dragonkind",
        "Osgood, Operation Double",
        "Woodland Weavemaster",
        "Gallifrey Council Chamber",
        "Base Camp",
        "Fabrication Foundry",
        "Gwenna, Eyes of Gaea",
        "Sunken Citadel",
        "Lukka, Bound to Ruin",
        "Cargo Ship",
        "Power Depot",
        "Primal Beyond",
        "Avengers Tower",
        "Villainous Hideout",
    ] {
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
        let typed = definition.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Activated(activated) if !activated.mana_usage_restrictions.is_empty()
            )
        });
        let surfaced = rendered.contains("Spend this mana only");
        if !typed || !surfaced {
            failures.push(format!(
                "{name}: typed={typed}, surfaced={surfaced}\n{rendered}\n{debug}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn refreshed_cumulative_upkeep_cards_render_typed_payments() {
    for name in [
        "Phyrexian Soulgorger",
        "Sheltering Ancient",
        "Jötun Owl Keeper",
        "Polar Kraken",
        "Wall of Shards",
        "Arctic Nishoba",
        "Vexing Sphinx",
        "Earthen Goo",
        "Thought Lash",
        "Krovikan Whispers",
    ] {
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
        assert!(debug.contains("CumulativeUpkeepEffect"), "{name}: {debug}");
        assert!(
            !rendered.contains("unsupported effect") && rendered.contains("Cumulative upkeep"),
            "{name} dropped its cumulative payment surface: {rendered}\n{debug}"
        );
    }
}

#[test]
fn refreshed_as_though_cards_keep_authored_permission_surface() {
    let mut failures = Vec::new();
    for name in [
        "Rolling Stones",
        "Detection Tower",
        "Hungering Yeti",
        "Mirror Wall",
        "Krotiq Nestguard",
        "Returned Phalanx",
        "Cherished Hatchling",
        "Glaring Spotlight",
        "Kaya, Bane of the Dead",
        "Roving Keep",
        "Radagast of Rhosgobel",
        "Autumn Willow",
        "Dark Maze",
        "Glade Watcher",
        "Skyclave Squid",
        "Steelclad Spirit",
        "Wall of One Thousand Cuts",
        "Aetherflame Wall",
        "Vodalian War Machine",
        "Guardians of Oboro",
        "Wall of Wonder",
        "Prismari Pledgemage",
        "Assault Formation",
        "Hightide Hermit",
        "Nivix Cyclops",
        "Stalked Researcher",
        "High Alert",
        "Mobile Fort",
        "Walking Wall",
        "Serpent of the Pass",
        "Wakestone Gargoyle",
        "Swift Reckoning",
        "Nowhere to Run",
        "Arlinn, the Pack's Hope // Arlinn, the Moon's Fury",
        "Weathered Sentinels",
        "Territorial Witchstalker",
        "Aether Web",
        "Dragon Grip",
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
        if !rendered.to_ascii_lowercase().contains("as though") {
            failures.push(format!("{name}:\n{rendered}\n{definition:#?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn typed_as_though_families_keep_their_exact_permission_clauses() {
    for (name, expected) in [
        (
            "Weathered Sentinels",
            "This creature can attack players who attacked you during their last turn as though it didn't have defender.",
        ),
        (
            "Detection Tower",
            "Until end of turn, your opponents and creatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.",
        ),
        (
            "Glaring Spotlight",
            "Creatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.",
        ),
        (
            "Kaya, Bane of the Dead",
            "Your opponents and permanents your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.",
        ),
        (
            "Autumn Willow",
            "Until end of turn, Autumn Willow can be the target of spells and abilities controlled by target player as though it didn't have shroud.",
        ),
        (
            "Nowhere to Run",
            "Creatures your opponents control can be the targets of spells and abilities as though they didn't have hexproof.",
        ),
        (
            "Aetherflame Wall",
            "This creature can block creatures with shadow as though they didn't have shadow.",
        ),
        (
            "Aether Web",
            "Enchanted creature gets +1/+1, has reach, and can block creatures with shadow as though they didn't have shadow.",
        ),
        (
            "Hungering Yeti",
            "As long as you control a green or blue permanent, you may cast this spell as though it had flash.",
        ),
        (
            "Serpent of the Pass",
            "If there are three or more Lesson cards in your graveyard, you may cast this spell as though it had flash.",
        ),
        (
            "Swift Reckoning",
            "Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may cast this spell as though it had flash.",
        ),
        (
            "Dragon Grip",
            "Ferocious — If you control a creature with power 4 or greater, you may cast this spell as though it had flash.",
        ),
        (
            "Radagast of Rhosgobel",
            "The first creature spell you cast each turn costs {2} less to cast and can be cast as though it had flash.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered
                .lines()
                .any(|line| line == expected || line.ends_with(expected)),
            "{name} dropped or rewrote its exact typed permission:\nexpected: {expected}\nactual: {rendered}\n{definition:#?}"
        );
    }
}

#[test]
fn mirror_wall_temporary_permission_lowers_to_the_attack_override() {
    let definition = parse_oracle_card_definition("Mirror Wall");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("CanAttackAsThoughNoDefender"),
        "Mirror Wall must grant the executable defender override: {debug}"
    );
}

#[test]
fn destructive_revelry_uses_the_common_permanent_antecedent() {
    let definition = parse_oracle_card_definition("Destructive Revelry");
    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");

    assert!(
        rendered.contains("Destructive Revelry deals 2 damage to that permanent's controller."),
        "an artifact-or-enchantment target needs the common permanent antecedent: {rendered}"
    );
}

#[test]
fn voracious_fell_beast_renders_a_food_for_each_sacrifice() {
    let rendered = crate::runtime_display::compiled_text_lines(&parse_oracle_card_definition(
        "Voracious Fell Beast",
    ))
    .join("\n");

    assert!(
        rendered.contains("Create a Food token for each creature sacrificed this way."),
        "the typed prior-effect count should render as a per-creature token instruction: {rendered}"
    );
}

#[test]
pub(super) fn named_triggered_and_activated_entry_counter_cards_use_inline_entry_modifiers() {
    for (name, counter, rendered_fragment) in [
        (
            "Chaos Shrine's Black Crystal",
            "Finality",
            "onto the battlefield under your control with a finality counter on it",
        ),
        (
            "Emperor of Bones",
            "Finality",
            "onto the battlefield under your control with a finality counter on it",
        ),
        (
            "Excava, the Risen Past",
            "Finality",
            "from your graveyard to the battlefield with a finality counter on it",
        ),
        (
            "Ghost Vacuum",
            "Flying",
            "onto the battlefield under your control with a flying counter on it",
        ),
        (
            "Yuna, Hope of Spira",
            "Finality",
            "from your graveyard to the battlefield with a finality counter on it",
        ),
        (
            "Grim Reaper, Lethal Legionnaire",
            "Finality",
            "to the battlefield tapped and attacking with a finality counter on it",
        ),
    ] {
        assert_named_inline_entry_counter(name, counter, rendered_fragment);
    }
}

#[test]
pub(super) fn grim_reaper_keeps_tapped_attacking_and_finality_as_one_entry_event() {
    let definition = parse_oracle_card_definition("Grim Reaper, Lethal Legionnaire");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("enters_tapped: true"), "{debug}");
    assert!(debug.contains("enters_attacking: true"), "{debug}");
    assert_eq!(
        debug.matches("BattlefieldEntryCounterSpec").count(),
        1,
        "{debug}"
    );
    assert!(debug.contains("counter_type: Finality"), "{debug}");
    assert!(debug.contains("surface: Inline"), "{debug}");

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("to the battlefield tapped and attacking with a finality counter on it"),
        "{rendered}"
    );
}

#[test]
pub(super) fn charnel_serenade_keeps_the_already_fused_spell_entry_counter_path() {
    assert_named_inline_entry_counter(
        "Charnel Serenade",
        "Finality",
        "from your graveyard to the battlefield with a finality counter on it",
    );

    let definition = parse_oracle_card_definition("Charnel Serenade");
    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert_eq!(
        rendered,
        "Surveil 3, then return a creature card from your graveyard to the battlefield with a finality counter on it. Exile Charnel Serenade with three time counters on it.\nSuspend 3—{2}{B}",
        "the final scored surface should preserve Charnel Serenade's authored return action"
    );
    assert!(
        !rendered.contains("choose a creature card")
            && !rendered.contains("put a finality counter"),
        "the compiled surface should not expose the internal choose-and-followup implementation: {rendered}"
    );
}

#[test]
pub(super) fn source_linked_reanimation_and_returned_animation_keep_authored_surfaces() {
    for name in [
        "Chaos Shrine's Black Crystal",
        "Excava, the Risen Past",
        "Ghost Vacuum",
    ] {
        assert_oracle_card_parses_strict(name);
    }

    let chaos = crate::runtime_display::compiled_text_lines(&parse_oracle_card_definition(
        "Chaos Shrine's Black Crystal",
    ))
    .join("\n");
    assert!(
        chaos.contains(
            "a creature card exiled with this onto the battlefield under your control with a finality counter on it"
        ),
        "Chaos Shrine should retain its source-linked exile surface: {chaos}"
    );

    let excava = crate::runtime_display::compiled_text_lines(&parse_oracle_card_definition(
        "Excava, the Risen Past",
    ))
    .join("\n");
    assert_eq!(
        excava,
        "Flying, haste\nWhenever Excava attacks, return up to one target artifact, creature, or non-Aura enchantment card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it. It's a 1/1 Spirit creature with flying in addition to its other types.",
        "the final scored surface should preserve Excava's singular linked animation"
    );
    assert!(
        excava.contains(
            "target artifact, creature, or non-Aura enchantment card with mana value 3 or less"
        ),
        "Excava should preserve the branch-local non-Aura exclusion: {excava}"
    );
    assert!(
        excava.contains("It's a 1/1 Spirit creature with flying in addition to its other types"),
        "Excava returns at most one object, so its linked animation is singular: {excava}"
    );

    let ghost =
        crate::runtime_display::compiled_text_lines(&parse_oracle_card_definition("Ghost Vacuum"))
            .join("\n");
    assert_eq!(
        ghost,
        "{T}: Exile target card from a graveyard.\n{6}, {T}, Sacrifice this artifact: Put each creature card exiled with this artifact onto the battlefield under your control with a flying counter on it. Each of them is a 1/1 Spirit in addition to its other types. Activate only as a sorcery.",
        "the final scored surface should preserve Ghost Vacuum's linked collection animation"
    );
    assert!(
        ghost.contains(
            "Put each creature card exiled with this artifact onto the battlefield under your control with a flying counter on it"
        ),
        "Ghost Vacuum should retain its typed source-linked exile surface: {ghost}"
    );
    assert!(
        ghost.contains("Each of them is a 1/1 Spirit in addition to its other types"),
        "Ghost Vacuum's collection animation should use distributive singular agreement: {ghost}"
    );
    assert!(
        !ghost.contains("It has base power and toughness")
            && !ghost.contains("those creature cards in exile"),
        "Ghost Vacuum should not expose the old singular or generic-exile rendering: {ghost}"
    );
}

#[test]
pub(super) fn cinder_strike_replacement_reuses_the_spell_as_damage_source() {
    assert_oracle_card_parses_strict("Cinder Strike");
    let definition = parse_oracle_card_definition("Cinder Strike");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Cinder Strike should have a spell resolution");
    let branch = &program.segments[0].self_replacements[0];
    let replacement_debug = format!("{branch:#?}");

    assert!(
        replacement_debug.contains("source: Source"),
        "the replacement damage source should remain Cinder Strike itself: {replacement_debug}"
    );
    assert!(
        branch.condition_after_replacement,
        "the parsed trailing `instead if` condition should retain its authored order: {replacement_debug}"
    );
    assert!(
        !replacement_debug.contains("blight_cost_0")
            && !replacement_debug.contains("counters_0")
            && !replacement_debug.contains("ForEachObject"),
        "the replacement must not iterate or bind to a creature used to pay the Blight cost: {replacement_debug}"
    );

    let rendered = unprocessed_compiled_lines(&definition).join("\n");
    assert!(
        rendered.contains("deals 4 damage to that creature instead"),
        "Cinder Strike should retain its linked replacement-damage surface: {rendered}"
    );
    assert!(
        !rendered.contains("for each creature that had counters put on them this way"),
        "the blight payment must not turn the replacement into a per-cost-object effect: {rendered}"
    );
}

#[test]
pub(super) fn blight_additional_cost_cluster_compiles_to_canonical_oracle_surface() {
    for (name, expected) in [
        (
            "Bogslither's Embrace",
            "As an additional cost to cast this spell, blight 1 or pay {3}.\nExile target creature.",
        ),
        (
            "Wild Unraveling",
            "As an additional cost to cast this spell, blight 2 or pay {1}.\nCounter target spell.",
        ),
        (
            "Cinder Strike",
            "As an additional cost to cast this spell, you may blight 1.\nCinder Strike deals 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional cost was paid.",
        ),
        (
            "Requiting Hex",
            "As an additional cost to cast this spell, you may blight 1.\nDestroy target creature with mana value 2 or less. If this spell's additional cost was paid, you gain 2 life.",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("BlightKeywordAction"),
            "{name} should retain the typed Blight action instead of a generic counter-cost surface: {debug}"
        );

        let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
        assert_eq!(
            rendered, expected,
            "{name} should render the canonical Blight keyword without reminder expansion"
        );
    }
}

fn looked_choice_filters(definition: &CardDefinition) -> Vec<&ObjectFilter> {
    let mut filters = Vec::new();
    for ability in &definition.abilities {
        let effects = match &ability.kind {
            AbilityKind::Activated(activated) => activated.effects.flattened_default_effects(),
            AbilityKind::Triggered(triggered) => triggered.effects.flattened_default_effects(),
            _ => continue,
        };
        for effect in effects {
            if let Some(choose) = effect.downcast_ref::<ChooseObjectsEffect>()
                && (choose.zone == Some(Zone::Library) || choose.filter.zone == Some(Zone::Library))
            {
                filters.push(&choose.filter);
            }
        }
    }
    filters
}

#[test]
pub(super) fn named_looked_card_choices_preserve_the_permanent_card_filter() {
    let permanent_types = ObjectFilter::permanent_card().card_types;
    for name in [
        "Beastrider Vanguard",
        "Invasion of Ixalan",
        "Sandstalker Moloch",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let filters = looked_choice_filters(&definition);
        assert!(
            filters.iter().any(|filter| {
                filter.card_types == permanent_types
                    && filter.all_card_types.is_empty()
                    && filter.any_of.is_empty()
            }),
            "{name} should retain the full permanent-card union: {filters:#?}"
        );

        let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered.contains(
                "You may reveal a permanent card from among them and put it into your hand"
            ),
            "{name} should render its typed looked-card choice: {rendered}"
        );
        assert!(
            !rendered.contains("You may reveal up to one of them"),
            "{name} must not render a permanent-card filter as an unfiltered choice: {rendered}"
        );
    }
}

#[test]
pub(super) fn vessel_of_nascency_preserves_its_five_way_card_type_union() {
    assert_oracle_card_parses_strict("Vessel of Nascency");
    let definition = parse_oracle_card_definition("Vessel of Nascency");
    let filters = looked_choice_filters(&definition);
    let filter = filters
        .iter()
        .copied()
        .find(|filter| filter.any_of.len() == 5)
        .expect("Vessel should retain a five-branch looked-card choice");
    let card_types = filter
        .any_of
        .iter()
        .filter_map(|branch| match branch.card_types.as_slice() {
            [card_type] if branch.any_of.is_empty() => Some(*card_type),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(card_types.len(), 5, "unexpected Vessel filter: {filter:#?}");
    for card_type in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
    ] {
        assert!(
            card_types.contains(&card_type),
            "Vessel should retain {card_type:?} in its union: {filter:#?}"
        );
    }

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "an artifact, creature, enchantment, land, or planeswalker card from among them"
        ),
        "Vessel should render every branch of its card-type union: {rendered}"
    );
    assert!(
        !rendered.contains("a creature or land card"),
        "Vessel's larger union must not satisfy the creature-or-land shorthand: {rendered}"
    );
}

#[test]
pub(super) fn water_tribe_rallier_preserves_waterbend_and_the_power_bound() {
    assert_oracle_card_parses_strict("Water Tribe Rallier");
    let definition = parse_oracle_card_definition("Water Tribe Rallier");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Water Tribe Rallier should have a waterbend activation");
    let branches = activated
        .mana_cost
        .as_one_of()
        .expect("waterbend {5} should lower to alternative mana-or-tap payments");
    assert_eq!(
        branches.len(),
        6,
        "waterbend {{5}} needs 0..=5 tap branches"
    );
    assert_eq!(
        branches[0].mana_cost().map(ManaCost::to_oracle),
        Some("{5}".to_string())
    );
    for (branch, expected_taps) in [(&branches[1], 1), (&branches[5], 5)] {
        let choose = branch
            .costs()
            .iter()
            .filter_map(|cost| cost.effect_ref())
            .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
            .expect("each non-mana waterbend branch should choose permanents to tap");
        assert_eq!(choose.count, ChoiceCount::exactly(expected_taps));
        assert!(choose.filter.untapped);
        assert_eq!(choose.filter.controller, Some(PlayerFilter::You));
        assert!(
            choose
                .filter
                .any_of
                .iter()
                .any(|filter| filter.card_types == [CardType::Artifact])
                && choose
                    .filter
                    .any_of
                    .iter()
                    .any(|filter| filter.card_types == [CardType::Creature]),
            "waterbend should accept untapped artifacts and creatures: {choose:#?}"
        );
    }

    let filters = looked_choice_filters(&definition);
    assert!(
        filters.iter().any(|filter| {
            filter.card_types == [CardType::Creature]
                && filter.power == Some(crate::filter::Comparison::LessThanOrEqual(3))
        }),
        "Water Tribe Rallier should preserve its creature power bound: {filters:#?}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("Waterbend {5}: Look at the top four cards of your library"),
        "Water Tribe Rallier should retain its mechanic cost surface: {rendered}"
    );
    assert!(
        rendered.contains("a creature card with power 3 or less from among them"),
        "Water Tribe Rallier should retain the choice qualifier: {rendered}"
    );
}

#[test]
pub(super) fn grist_front_face_scopes_graveyard_origin_condition_on_the_trigger() {
    assert_oracle_card_parses_strict("Grist, Voracious Larva");
    let definition = parse_oracle_card_definition("Grist, Voracious Larva");
    let debug = format!("{:?}", definition.abilities);

    assert!(
        debug.contains("MovedFromOrCastFrom")
            && debug.contains("zone: Graveyard")
            && debug.contains("zone_owner: Some(You)")
            && debug.contains("caster: Some(You)"),
        "Grist's enters trigger must carry the graveyard origin condition (zone + owner + caster) on the trigger itself: {debug}"
    );
    assert!(
        !debug.contains("TaggedObjectMatches") && !debug.contains("ThisSpellWasCastFromZone"),
        "the origin qualifier must not be modeled as an intervening-if condition: {debug}"
    );

    let rendered = unprocessed_compiled_lines(&definition).join(" ");
    assert!(
        rendered.contains("if it entered from your graveyard or you cast it from your graveyard"),
        "Grist should round-trip its origin clause: {rendered}"
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("exile grist, then return it to the battlefield transformed"),
        "Grist should keep its authored exile-return surface: {rendered}"
    );
}

#[test]
pub(super) fn prized_amalgam_scopes_graveyard_origin_condition_on_the_trigger() {
    assert_oracle_card_parses_strict("Prized Amalgam");
    let definition = parse_oracle_card_definition("Prized Amalgam");
    let debug = format!("{:?}", definition.abilities);

    assert!(
        debug.contains("MovedFromOrCastFrom")
            && debug.contains("zone: Graveyard")
            && debug.contains("zone_owner: Some(You)")
            && debug.contains("caster: Some(You)"),
        "Prized Amalgam's enters trigger must carry the graveyard origin condition on the trigger itself: {debug}"
    );
    assert!(
        !debug.contains("TaggedObjectMatches") && !debug.contains("ThisSpellWasCastFromZone"),
        "the origin qualifier must not be modeled as an intervening-if condition: {debug}"
    );

    let rendered = unprocessed_compiled_lines(&definition).join(" ");
    assert!(
        rendered.contains("if it entered from your graveyard or you cast it from your graveyard"),
        "Prized Amalgam should round-trip its origin clause: {rendered}"
    );
}

#[test]
pub(super) fn gruul_spellbreaker_union_hexproof_covers_you_and_the_source_only() {
    assert_oracle_card_parses_strict("Gruul Spellbreaker");
    let definition = parse_oracle_card_definition("Gruul Spellbreaker");
    let debug = format!("{:?}", definition.abilities);

    assert!(
        debug.contains("BeTargetedPlayerFrom(You"),
        "the \"you\" half of the union must compile to a player targeting restriction: {debug}"
    );
    assert!(
        !debug.contains("card_types: [Creature]"),
        "no half of the union may grant hexproof to a battlefield-wide creature filter: {debug}"
    );

    let rendered = compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("During your turn, you and this creature have hexproof."),
        "Gruul Spellbreaker should round-trip its union subject line: {rendered}"
    );
    assert!(
        !rendered
            .to_ascii_lowercase()
            .contains("creatures have hexproof"),
        "opponents' creatures must not gain hexproof: {rendered}"
    );
}

#[test]
pub(super) fn dion_union_flying_keeps_the_source_half_of_the_subject() {
    let name = "Dion, Bahamut's Dominant // Bahamut, Warden of Light";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{:?}", definition.abilities);

    assert!(
        debug.contains("source: true"),
        "the Dion half of the union must compile as a source-scoped flying grant: {debug}"
    );
    assert!(
        debug.contains("subtypes: [Knight]"),
        "the filter half must keep the other-Knights-you-control grant: {debug}"
    );

    // The test harness builds one joined definition for both faces, so the
    // source-reference noun follows the joined type line ("this Saga"); the
    // per-face compile renders "this creature". Assert the union shape
    // without pinning the noun.
    let rendered = compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(" and other Knights you control have flying.")
            && rendered.contains("During your turn, this"),
        "Dion should round-trip its union subject line: {rendered}"
    );
}

#[test]
pub(super) fn niv_mizzet_guildpact_counts_distinct_color_pairs() {
    let name = "Niv-Mizzet, Guildpact";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{:?}", definition.abilities);

    assert_eq!(
        debug.matches("ColorPairsAmong").count(),
        3,
        "each X basis (damage, draw, gain) must count distinct color pairs, \
         not degrade to a bare permanent count: {debug}"
    );
    assert!(
        debug.contains("exactly_two_colors: Some(true)"),
        "the color-pair basis filter must keep the exactly-two-colors qualifier: {debug}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "it deals X damage to any target, where X is the number of different \
             color pairs among permanents you control that are exactly two colors"
        ),
        "the trigger body should name the source \"it\" and reconstruct the authored \
         color-pair basis: {rendered}"
    );
    assert!(
        !rendered.contains("where X is the number of permanents you control,"),
        "the basis must not silently degrade to a bare permanent count: {rendered}"
    );
}

#[test]
pub(super) fn breathkeeper_seraph_grants_verb_first_optional_delayed_return() {
    let name = "Breathkeeper Seraph";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let rendered = compiled_text_lines(&definition).join("\n");

    assert!(
        rendered.contains(
            "each of those creatures has \"When this creature dies, you may return it \
             to the battlefield under its owner's control at the beginning of your next upkeep.\""
        ),
        "the quoted granted dies-trigger must keep \"you may\" and read verb-first with \
         the delayed timing as a trailing modifier: {rendered}"
    );
    assert!(
        !rendered.contains("may At the beginning"),
        "the may-wrapper must not prepend the delayed-timing clause mid-sentence: {rendered}"
    );

    assert!(
        rendered
            .lines()
            .any(|line| line.trim_end_matches('.') == "Flying, soulbond"),
        "the keyword list must stay on one line as \"Flying, soulbond\": {rendered}"
    );
    assert!(
        !rendered
            .lines()
            .any(|line| line.trim_end_matches('.') == "Soulbond"),
        "soulbond must merge into the keyword list instead of claiming its own line: {rendered}"
    );
}

#[test]
pub(super) fn circle_of_the_moon_druid_scopes_bear_form_to_the_source() {
    let name = "Circle of the Moon Druid";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("source: true"),
        "the Bear Form animation must compile against the source, not every creature: {debug}"
    );
    assert!(
        debug.contains("condition: ActivationTiming(DuringYourTurn)"),
        "the leading \"During your turn,\" clause must survive as a typed condition: {debug}"
    );
    assert!(
        debug.contains("subtypes: [Bear]") && debug.contains("power: Fixed(4)"),
        "the Bear subtype replacement and 4/2 base stats must both compile: {debug}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "During your turn, this creature is a Bear with base power and toughness 4/2."
        ),
        "the animation bundle should render as the oracle's single clause: {rendered}"
    );
    assert!(
        !rendered.contains("All creatures"),
        "the animation must not scope to every creature on the battlefield: {rendered}"
    );
}

#[test]
pub(super) fn act_of_heroism_keeps_the_untap_and_pump_before_the_block_grant() {
    let name = "Act of Heroism";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("UntapEffect"),
        "the leading \"Untap target creature.\" sentence must compile: {debug}"
    );
    assert!(
        debug.contains("CanBlockAdditionalCreatureEachCombat(1)"),
        "the conjoined block permission must compile as a granted ability: {debug}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "Untap target creature. It gets +2/+2 until end of turn and can block an additional creature this turn."
        ),
        "the untap, pump, and block permission should round-trip as one authored line: {rendered}"
    );
    assert!(
        !rendered.contains("Each creature"),
        "the block permission must stay on the untapped creature, not every creature: {rendered}"
    );
}

#[test]
pub(super) fn second_guess_keeps_the_ordinal_spell_cast_qualifier() {
    let name = "Second Guess";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("TargetSpellCastOrderThisTurn"),
        "the \"second spell cast this turn\" qualifier must compile as a cast-order condition, \
         not be swallowed by short-name self-reference rewriting: {debug}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("Counter target spell that's the second spell cast this turn."),
        "the ordinal qualifier should round-trip: {rendered}"
    );
    assert!(
        !rendered.contains("permanent spell"),
        "the ordinal word \"second\" must not be rewritten as a self reference: {rendered}"
    );
}

#[test]
pub(super) fn obeka_offers_the_end_of_turn_choice_to_the_active_player() {
    let name = "Obeka, Brute Chronologist";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("MayEffect"),
        "\"may end the turn\" must stay an optional choice, not a forced end-turn: {debug}"
    );
    assert!(
        debug.contains("decider: Some(\n                                            Active,")
            || debug.contains("Active"),
        "the may decision must belong to the active player (the player whose turn it is): {debug}"
    );
    let end_turn_tail = debug
        .split("EndTurnEffect")
        .nth(1)
        .expect("the granted action must compile to the end-the-turn effect");
    let end_turn_player = end_turn_tail
        .split("player:")
        .nth(1)
        .map(|rest| rest.trim_start())
        .expect("EndTurnEffect should record its player");
    assert!(
        end_turn_player.starts_with("Active"),
        "the end-turn action must belong to the active player (EndTurnEffect no-ops for \
         anyone else), got: {end_turn_player:.40}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("{T}: The player whose turn it is may end the turn."),
        "the activated ability should round-trip the oracle wording: {rendered}"
    );
}

#[test]
pub(super) fn bonus_round_registers_a_temporary_copy_trigger_for_each_caster() {
    let name = "Bonus Round";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "\"Until end of turn, whenever ...\" must register a temporary delayed trigger, \
         not resolve as a one-shot copy: {debug}"
    );
    assert!(
        debug.contains("until_end_of_turn: true"),
        "the delayed trigger must expire at end of turn: {debug}"
    );
    assert!(
        debug.contains("leading_duration_surface: true"),
        "the authored leading duration must survive lowering: {debug}"
    );
    assert!(
        debug.contains("caster: Any"),
        "the trigger must watch every player's instant and sorcery spells: {debug}"
    );
    assert!(
        debug.contains("copier: IteratedPlayer"),
        "the copy must be created by the spell's caster, not this spell's controller: {debug}"
    );
    assert!(
        debug.contains("ChooseNewTargetsEffect"),
        "the caster must be offered new targets for the copy: {debug}"
    );

    let rendered = crate::runtime_display::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "Until end of turn, whenever a player casts an instant or sorcery spell, that player copies it and may choose new targets for the copy."
        ),
        "the temporary trigger should render with the caster as the copying subject: {rendered}"
    );
    assert!(
        !rendered.contains("Copy this spell"),
        "the line must not degrade into copying Bonus Round itself: {rendered}"
    );
}
