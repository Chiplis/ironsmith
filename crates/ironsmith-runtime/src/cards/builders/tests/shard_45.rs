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

    assert_eq!(
        debug.matches("BattlefieldEntryCounterSpec").count(),
        1,
        "{name} should encode its authored counter as one battlefield-entry modifier: {debug}"
    );
    assert!(
        debug.contains(&format!("counter_type: {counter_debug_name}"))
            && debug.contains("surface: Inline"),
        "{name} should retain the counter kind and inline entry timing: {debug}"
    );
    assert!(
        !debug.contains("AdditionalEntryCounter"),
        "{name} says the permanent enters with a counter, not an additional counter: {debug}"
    );

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(expected_rendered_fragment),
        "{name} should keep the counter inside the battlefield-entry instruction; expected {expected_rendered_fragment:?}, got {rendered:?}"
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

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
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
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
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

    let chaos = crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(
        "Chaos Shrine's Black Crystal",
    ))
    .join("\n");
    assert!(
        chaos.contains(
            "a creature card exiled with this onto the battlefield under your control with a finality counter on it"
        ),
        "Chaos Shrine should retain its source-linked exile surface: {chaos}"
    );

    let excava = crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(
        "Excava, the Risen Past",
    ))
    .join("\n");
    assert_eq!(
        excava,
        "Flying, haste\nWhenever Excava attacks, return up to one target artifact, creature, or non-Aura enchantment card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it. It's 1/1 Spirit creature with flying in addition to its other types.",
        "the final scored surface should preserve Excava's singular linked animation"
    );
    assert!(
        excava.contains(
            "target artifact, creature, or non-Aura enchantment card with mana value 3 or less"
        ),
        "Excava should preserve the branch-local non-Aura exclusion: {excava}"
    );
    assert!(
        excava.contains("It's 1/1 Spirit creature with flying in addition to its other types"),
        "Excava returns at most one object, so its linked animation is singular: {excava}"
    );

    let ghost =
        crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition("Ghost Vacuum"))
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

        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
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

        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
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

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
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

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("Waterbend {5}: Look at the top four cards of your library"),
        "Water Tribe Rallier should retain its mechanic cost surface: {rendered}"
    );
    assert!(
        rendered.contains("a creature card with power 3 or less from among them"),
        "Water Tribe Rallier should retain the choice qualifier: {rendered}"
    );
}
