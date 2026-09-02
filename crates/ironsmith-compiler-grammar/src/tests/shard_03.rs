#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::*;
use crate::target::PlayerFilter;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

#[test]
pub(super) fn rewrite_keyword_craft_line_uses_supported_activated_keyword_lowering() {
    let tokens = lex_line("Craft with artifact {3}{W}{W}", 0)
        .expect("rewrite lexer should classify craft line");

    let parsed = super::super::activation_and_restrictions::parse_craft_line_lexed(&tokens)
        .expect("craft line should parse")
        .expect("craft line should produce an activated ability");
    let crate::model::CompilerAbilityKindCore::Activated(activated) = &parsed.ability.kind else {
        panic!("craft line should lower to an activated ability: {parsed:#?}");
    };
    let [
        crate::model::CompilerCost::Mana(mana),
        crate::model::CompilerCost::ExileChosen { filter, .. },
        crate::model::CompilerCost::EmitKeywordAction {
            kind: crate::events::KeywordActionKind::Craft,
            amount: 1,
        },
        crate::model::CompilerCost::ExileSelf {
            from_graveyard: false,
        },
    ] = activated
        .mana_cost
        .as_all()
        .expect("craft should lower to a typed sequential cost")
    else {
        panic!("craft should keep typed mana, material, action, and self-exile costs: {parsed:#?}");
    };
    assert_eq!(mana.to_oracle(), "{3}{W}{W}");
    assert!(
        filter
            .any_of
            .iter()
            .all(|arm| arm.card_types == [CardType::Artifact])
    );
}

#[test]
pub(super) fn rewrite_keyword_craft_line_supports_creature_material_clause() {
    let tokens = lex_line("Craft with creature {5}{G}{G}", 0)
        .expect("rewrite lexer should classify craft line");

    let parsed = super::super::activation_and_restrictions::parse_craft_line_lexed(&tokens)
        .expect("craft line should parse")
        .expect("craft line should produce an activated ability");
    let crate::model::CompilerAbilityKindCore::Activated(activated) = &parsed.ability.kind else {
        panic!("craft line should lower to an activated ability: {parsed:#?}");
    };
    let [
        crate::model::CompilerCost::Mana(mana),
        crate::model::CompilerCost::ExileChosen { filter, .. },
        crate::model::CompilerCost::EmitKeywordAction {
            kind: crate::events::KeywordActionKind::Craft,
            amount: 1,
        },
        crate::model::CompilerCost::ExileSelf {
            from_graveyard: false,
        },
    ] = activated
        .mana_cost
        .as_all()
        .expect("craft should lower to a typed sequential cost")
    else {
        panic!("craft should keep typed mana, material, action, and self-exile costs: {parsed:#?}");
    };
    assert_eq!(mana.to_oracle(), "{5}{G}{G}");
    assert!(
        filter
            .any_of
            .iter()
            .all(|arm| arm.card_types == [CardType::Creature])
    );
}

#[test]
pub(super) fn rewrite_keyword_static_as_enters_choice_parsers_share_subject_tables() {
    let color_tokens = lex_line("as this aura enters, choose a color.", 0)
        .expect("rewrite lexer should classify choose-color static line");
    let player_tokens = lex_line("as this artifact enters, choose a player.", 0)
        .expect("rewrite lexer should classify choose-player static line");
    let opponent_tokens = lex_line("as this artifact enters, choose an opponent.", 0)
        .expect("rewrite lexer should classify choose-opponent static line");

    let color = super::super::keyword_static::parse_choose_color_as_enters_line(&color_tokens)
        .expect("choose-color static line should parse");
    let player = super::super::keyword_static::parse_choose_player_as_enters_line(&player_tokens)
        .expect("choose-player static line should parse");
    let opponent =
        super::super::keyword_static::parse_choose_player_as_enters_line(&opponent_tokens)
            .expect("choose-opponent static line should parse")
            .expect("choose-opponent static line should produce an ability");

    assert!(matches!(
        color,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::ChooseColorAsEnters
    ));
    assert!(matches!(
        player,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::ChoosePlayerAsEnters
    ));
    assert!(matches!(
        opponent.payload,
        ironsmith_core::StaticAbilityPayload::ChoosePlayerAsEnters {
            filter: PlayerFilter::Opponent,
            ..
        }
    ));
}

#[test]
pub(super) fn rewrite_as_enters_combined_color_and_creature_type_choice_is_typed() {
    let tokens = lex_line(
        "As this artifact enters, choose a color and a creature type.",
        0,
    )
    .expect("combined choice should lex");
    let abilities =
        super::super::keyword_static::parse_choose_color_and_creature_type_as_enters_line(&tokens)
            .expect("combined choice parser should not error")
            .expect("combined choice should parse");

    assert_eq!(abilities.len(), 2, "{abilities:#?}");
    assert_eq!(
        abilities[0].id(),
        crate::static_abilities::StaticAbilityId::ChooseColorAsEnters
    );
    assert_eq!(
        abilities[1].id(),
        crate::static_abilities::StaticAbilityId::ChooseCreatureTypeAsEnters
    );
}

#[test]
pub(super) fn rewrite_as_enters_color_creature_pairs_keep_correlated_options() {
    let tokens = lex_line(
        "As this artifact enters, choose white Citizen, blue Camarid, black Thrull, red Goblin, or green Saproling.",
        0,
    )
    .expect("paired choices should lex");
    let ability =
        super::super::keyword_static::parse_choose_color_creature_type_pairs_as_enters_line(
            &tokens,
        )
        .expect("paired choice parser should not error")
        .expect("paired choices should parse");

    assert!(matches!(
        ability.payload,
        crate::model::CompilerStaticAbilityPayloadCore::ChooseNamedOptionAsEnters {
            ref options,
            ..
        } if options
            == &[
                "white Citizen",
                "blue Camarid",
                "black Thrull",
                "red Goblin",
                "green Saproling",
            ]
    ));
}

#[test]
pub(super) fn create_token_keeps_source_chosen_characteristics_outside_compact_blueprint() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Chosen Token Characteristics")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create a 2/2 creature token of the chosen color and type.")
        .expect("chosen token characteristics should compile");
    let debug = format!("{:#?}", definition.spell_effect);

    assert!(debug.contains("use_source_chosen_color: true"), "{debug}");
    assert!(
        debug.contains("use_source_chosen_creature_type: true"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_keyword_static_as_enters_revealed_hand_card_name_choice() {
    let tokens = lex_line(
        "as this creature enters, each opponent reveals their hand. you choose the name of a nonland card revealed this way.",
        0,
    )
    .expect("rewrite lexer should classify revealed-hand card-name choice");

    let ability =
        super::super::keyword_static::parse_revealed_hand_choose_nonland_card_name_as_enters_line(
            &tokens,
        )
        .expect("revealed-hand card-name choice should parse");

    assert!(matches!(
        ability,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::ChooseCardNameAsEnters
                && ability.display().contains("each opponent reveals their hand")
                && ability.display().contains("nonland card revealed this way")
                && matches!(
                    &ability.payload,
                    crate::model::CompilerStaticAbilityPayloadCore::ChooseCardNameAsEnters {
                        reveal_opponents_hands: true,
                        require_nonland_from_revealed_opponents: true,
                        ..
                    }
                )
    ));
}

#[test]
pub(super) fn rewrite_keyword_static_as_enters_reveal_from_hand_counted_cards() {
    let tokens = lex_line(
        "as this creature enters, you may reveal any number of other artifact cards from your hand.",
        0,
    )
    .expect("rewrite lexer should classify as-enters reveal line");

    let ability = super::super::keyword_static::parse_as_enters_reveal_from_hand_line(&tokens)
        .expect("as-enters reveal line should parse")
        .expect("as-enters reveal line should produce a static ability");

    assert_eq!(ability.id(), StaticAbilityId::RevealFromHandAsEnters);
    assert!(matches!(
        &ability.payload,
        crate::model::CompilerStaticAbilityPayloadCore::RevealFromHandAsEnters {
            filter,
            count,
            optional: true,
            ..
        } if filter.zone == Some(Zone::Hand)
            && filter.other
            && filter.card_types == [CardType::Artifact]
            && *count == ChoiceCount::any_number()
    ));
}

#[test]
pub(super) fn rewrite_keyword_static_pt_color_type_addition_bundle() {
    let tokens = lex_line(
        "All Forests and all Saprolings are 1/1 green Saproling creatures and Forest lands in addition to their other types.",
        0,
    )
    .expect("rewrite lexer should classify Life and Limb style static line");

    let abilities =
        super::super::keyword_static::parse_all_are_pt_color_type_addition_line(&tokens)
            .expect("pt/color/type addition line should parse")
            .expect("pt/color/type addition line should produce static abilities");
    let ids = abilities
        .iter()
        .map(|ability| ability.id())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            StaticAbilityId::SetColors,
            StaticAbilityId::AddCardTypes,
            StaticAbilityId::AddSubtypes,
            StaticAbilityId::SetBasePowerToughnessForFilter,
        ]
    );
    assert!(matches!(
        &abilities[0].payload,
        crate::model::CompilerStaticAbilityPayloadCore::SetColors { colors, .. }
            if *colors == ColorSet::GREEN
    ));
    let debug = format!("{abilities:#?}");
    assert!(
        debug.contains("any_of")
            && debug.contains("Forest")
            && debug.contains("Saproling")
            && debug.contains("Creature")
            && debug.contains("Land"),
        "expected typed union subject and characteristic additions, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lose_all_transform_name_uses_parser_word_coordinates() {
    for (text, expected_name) in [
        (
            "Enchanted creature loses all abilities and is a Citizen with base power and toughness 1/1 and \"{T}: Add {C}\" named Humble Merchant.",
            "Humble Merchant",
        ),
        (
            "Enchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named Legitimate Businessperson.",
            "Legitimate Businessperson",
        ),
    ] {
        let tokens = lex_line(text, 0).expect("transform line should lex");
        let abilities =
            super::super::keyword_static::parse_lose_all_abilities_and_transform_base_pt_line(
                &tokens,
            )
            .expect("transform line should parse")
            .expect("transform line should produce static abilities");

        assert!(abilities.iter().any(|ability| matches!(
            &ability.payload,
            crate::model::CompilerStaticAbilityPayloadCore::SetName { name, .. }
                if name == expected_name
        )));
    }
}

#[test]
pub(super) fn rewrite_compound_lose_all_base_pt_dispatch_keeps_both_modifications() {
    let tokens = lex_line(
        "Non-Horror creatures with slime counters on them lose all abilities and have base power and toughness 2/2.",
        0,
    )
    .expect("compound lose-all/base-pt line should lex");
    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("compound lose-all/base-pt line should parse")
        .expect("compound lose-all/base-pt line should produce static abilities");
    let ids = parsed
        .iter()
        .filter_map(|ability| match ability {
            crate::cards::builders::StaticAbilityAst::Static(ability) => Some(ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter,
            crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter,
        ]
    );
    for ability in &parsed {
        let crate::cards::builders::StaticAbilityAst::Static(ability) = ability else {
            panic!("expected static ability, got {ability:#?}");
        };
        let filter = match &ability.payload {
            crate::model::CompilerStaticAbilityPayloadCore::RemoveAllAbilities(filter)
            | crate::model::CompilerStaticAbilityPayloadCore::SetBasePowerToughness {
                filter,
                ..
            } => filter,
            payload => panic!("unexpected payload: {payload:#?}"),
        };
        assert!(filter.card_types.contains(&CardType::Creature));
        assert!(filter.excluded_subtypes.contains(&Subtype::Horror));
        assert!(filter.with_counter.is_some());
    }
}

#[test]
pub(super) fn rewrite_dynamic_lose_all_becomes_shape_bypasses_fixed_early_route() {
    let tokens = lex_line(
        "Each noncreature artifact loses all abilities and becomes an artifact creature with power and toughness each equal to its mana value.",
        0,
    )
    .expect("dynamic lose-all animation should lex");
    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("dynamic lose-all animation should continue past fixed early routing")
        .expect("dynamic lose-all animation should produce static abilities");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
}

#[test]
pub(super) fn filtered_per_object_animation_cards_keep_dynamic_characteristics() {
    let animate = CardDefinitionBuilder::new(CardId::new(), "Animate Artifact")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant artifact\nAs long as enchanted artifact isn't a creature, it's an artifact creature with power and toughness each equal to its mana value.",
        )
        .expect("Animate Artifact should parse");
    let animate_debug = format!("{:#?}", animate.abilities);
    assert!(animate_debug.contains("SetCardTypes"), "{animate_debug}");
    assert!(
        animate_debug.contains("SetBasePowerToughnessValue"),
        "{animate_debug}"
    );
    assert!(animate_debug.contains("ManaValueOf"), "{animate_debug}");
    assert!(animate_debug.contains("Iterated"), "{animate_debug}");
    assert!(animate_debug.contains("Not"), "{animate_debug}");
    assert!(animate_debug.contains("enchanted"), "{animate_debug}");

    let march = CardDefinitionBuilder::new(CardId::new(), "March of the Machines")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Each noncreature artifact is an artifact creature with power and toughness each equal to its mana value. (Equipment that's a creature can't equip a creature.)",
        )
        .expect("March of the Machines should parse");
    let march_debug = format!("{:#?}", march.abilities);
    assert!(march_debug.contains("SetCardTypes"), "{march_debug}");
    assert!(march_debug.contains("ManaValueOf"), "{march_debug}");
    assert!(march_debug.contains("Iterated"), "{march_debug}");
    assert!(
        march_debug.contains("excluded_card_types") && march_debug.contains("Creature"),
        "{march_debug}"
    );

    let spark = CardDefinitionBuilder::new(CardId::new(), "Spark Rupture")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, draw a card.\nEach planeswalker with one or more loyalty counters on it loses all abilities and is a creature with power and toughness each equal to the number of loyalty counters on it.",
        )
        .expect("Spark Rupture should parse");
    let spark_debug = format!("{:#?}", spark.abilities);
    assert!(spark_debug.contains("RemoveAllAbilities"), "{spark_debug}");
    assert!(spark_debug.contains("SetCardTypes"), "{spark_debug}");
    assert!(spark_debug.contains("CountersOn"), "{spark_debug}");
    assert!(spark_debug.contains("Loyalty"), "{spark_debug}");
    assert!(spark_debug.contains("Iterated"), "{spark_debug}");
}

#[test]
pub(super) fn rewrite_grammar_exile_to_countered_exile_instead_of_graveyard_splitter_matches_static_shape()
 {
    let tokens = lex_line(
        "If a creature would be put into an opponent's graveyard from anywhere, exile it instead with a stun counter on it.",
        0,
    )
    .expect("rewrite lexer should classify exile-replacement static line");

    let spec =
        super::super::grammar::abilities::parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(
            &tokens,
        )
        .expect("grammar-owned exile-replacement splitter should match");
    assert_eq!(spec.player, crate::target::PlayerFilter::Opponent);
    assert_eq!(spec.counter_type, crate::object::CounterType::Stun);

    let parsed =
        super::super::keyword_static::parse_exile_to_countered_exile_instead_of_graveyard_line(
            &tokens,
        )
        .expect("exile-replacement static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
    ));
}

#[test]
pub(super) fn rewrite_grammar_exile_to_countered_exile_splitter_accepts_instead_lead_word_order() {
    let tokens = lex_line(
        "If a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.",
        0,
    )
    .expect("rewrite lexer should classify exile-replacement static line");

    let spec =
        super::super::grammar::abilities::parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(
            &tokens,
        )
        .expect("grammar-owned exile-replacement splitter should match");
    assert_eq!(spec.player, crate::target::PlayerFilter::Opponent);
    assert_eq!(spec.counter_type, crate::object::CounterType::Void);
}

#[test]
pub(super) fn parse_named_source_exile_instead_of_graveyard_from_anywhere() {
    let tokens = lex_line(
        "if hook-haunt drifter would be put into a graveyard from anywhere, exile it instead.",
        0,
    )
    .expect("named source exile-replacement line should lex");

    let parsed =
        super::super::keyword_static::parse_exile_to_exile_instead_of_graveyard_line(&tokens)
            .expect("named source exile-replacement line should parse");
    let words = crate::lexer::token_word_refs(&tokens);
    assert!(
        matches!(
            parsed,
            Some(ref ability)
                if ability.id()
                    == crate::static_abilities::StaticAbilityId::ExileToExileInsteadOfGraveyard
        ),
        "parsed={parsed:?} words={words:?}"
    );
}

#[test]
pub(super) fn parse_card_or_token_exile_instead_keeps_cards_in_filter() {
    let tokens = lex_line(
        "If a card or token would be put into a graveyard from anywhere, exile it instead.",
        0,
    )
    .expect("card-or-token exile-replacement line should lex");
    let parsed =
        super::super::keyword_static::parse_exile_to_exile_instead_of_graveyard_line(&tokens)
            .expect("card-or-token exile-replacement line should parse");
    let Some(ability) = parsed else {
        panic!("expected exile-replacement static ability");
    };
    let debug = format!("{ability:?}");

    assert_eq!(
        ability.id(),
        crate::static_abilities::StaticAbilityId::ExileToExileInsteadOfGraveyard
    );
    assert!(
        !debug.contains("token: true"),
        "card-or-token replacement must not be restricted to tokens only: {debug}"
    );
}

#[test]
pub(super) fn parse_cycling_card_exile_instead_unless_cycled() {
    let line = "If a card that has a cycling ability would be put into your graveyard from anywhere and it wasn't cycled, exile it instead.";
    let tokens = lex_line(line, 0).expect("cycling exile-replacement line should lex");
    let parsed =
        super::super::keyword_static::parse_exile_to_exile_instead_of_graveyard_line(&tokens)
            .expect("cycling exile-replacement line should parse");
    let Some(ability) = parsed else {
        panic!("expected exile-replacement static ability");
    };
    let debug = format!("{ability:?}").to_ascii_lowercase();

    assert_eq!(
        ability.id(),
        crate::static_abilities::StaticAbilityId::ExileToExileInsteadOfGraveyard
    );
    assert!(
        debug.contains("cycling") && debug.contains("exclude_cycled: true"),
        "debug={debug}"
    );
    let crate::model::CompilerStaticAbilityPayloadCore::ExileToExileInsteadOfGraveyard {
        filter,
        ..
    } = &ability.payload
    else {
        panic!("expected typed exile replacement: {ability:#?}");
    };
    assert!(filter.has_explicit_card_noun(), "{filter:#?}");
}

#[test]
pub(super) fn graveyard_play_grants_render_filtered_spell_subjects() {
    let cycling_line = "You may cast spells that have a cycling ability from your graveyard.";
    let cycling_tokens = lex_line(cycling_line, 0).expect("cycling permission should lex");
    let cycling_abilities =
        super::super::keyword_static::parse_you_may_static_grant_line(&cycling_tokens)
            .expect("cycling permission parser should not error")
            .expect("cycling permission should parse");
    let [cycling_ability] = cycling_abilities.as_slice() else {
        panic!("expected one cycling grant: {cycling_abilities:#?}");
    };
    let crate::model::CompilerStaticAbilityPayloadCore::Grants(cycling_grant) =
        &cycling_ability.payload
    else {
        panic!("expected a typed cycling grant: {cycling_ability:#?}");
    };
    assert_eq!(cycling_grant.display(), cycling_line.trim_end_matches('.'));

    let zombie_filter = crate::filter::ObjectFilter::default()
        .with_subtype(Subtype::Zombie)
        .without_type(CardType::Land);
    let zombie_grant = crate::grant::GrantSpec::new(
        crate::grant::Grantable::play_from(),
        zombie_filter,
        Zone::Graveyard,
    );
    assert_eq!(
        zombie_grant.display(),
        "You may cast Zombie spells from your graveyard"
    );
}

#[test]
pub(super) fn rewrite_grammar_draw_replace_exile_top_face_down_probe_matches_static_shape() {
    let tokens = lex_line(
        "If you would draw a card, exile the top card of your library face down instead.",
        0,
    )
    .expect("rewrite lexer should classify draw-replacement static line");

    assert!(
        super::super::grammar::abilities::is_draw_replace_exile_top_face_down_line_lexed(&tokens),
        "grammar-owned draw-replacement probe should match"
    );

    let parsed = super::super::keyword_static::parse_draw_replace_exile_top_face_down_line(&tokens)
        .expect("draw-replacement static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::DrawReplacementExileTopFaceDown
    ));
}

#[test]
pub(super) fn rewrite_grammar_draw_replacement_exile_top_and_play_probe_matches_static_shape() {
    let tokens = lex_line(
        "If you would draw a card, exile the top two cards of your library instead. You may play those cards this turn.",
        0,
    )
    .expect("rewrite lexer should classify draw-replacement exile/play static line");

    let parsed =
        super::super::keyword_static::parse_draw_replacement_exile_top_and_play_line(&tokens)
            .expect("draw-replacement exile/play static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::DrawReplacementExileTopAndPlay
    ));
}

#[test]
pub(super) fn rewrite_grammar_branching_evolution_counter_line_matches_static_shape() {
    let tokens = lex_line(
        "If one or more +1/+1 counters would be put on a creature you control, twice that many +1/+1 counters are put on it instead.",
        0,
    )
    .expect("rewrite lexer should classify counter-doubling static line");

    let parsed = super::super::keyword_static::parse_double_counters_replacement_line(&tokens)
        .expect("counter-doubling static line should parse");
    assert!(
        matches!(
            parsed,
            Some(ref ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::DoubleCountersReplacement
        ),
        "got {parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_typed_replacement_predicate_regression_shapes_preserve_semantics() {
    let explore = lex_line(
        "If a creature you control would explore, instead it explores, then it explores again.",
        0,
    )
    .expect("explore replacement should lex");
    let explore_ability =
        super::super::keyword_static::parse_keyword_action_replacement_line(&explore)
            .expect("explore replacement should parse")
            .expect("explore replacement should be recognized");
    assert_eq!(
        explore_ability.id(),
        StaticAbilityId::KeywordActionReplacement
    );
    let explore_debug = format!("{explore_ability:#?}");
    assert!(
        explore_debug.matches("action: Explore").count() >= 2,
        "double-explore replacement must retain both replacement actions: {explore_debug}"
    );

    let counters = lex_line(
        "If one or more +1/+1 counters would be put on a permanent you control, that many plus one +1/+1 counters are put on that permanent instead.",
        0,
    )
    .expect("counter replacement should lex");
    let counter_ability =
        super::super::keyword_static::parse_double_counters_replacement_line(&counters)
            .expect("counter replacement should parse")
            .expect("counter replacement should be recognized");
    assert_eq!(
        counter_ability.id(),
        StaticAbilityId::AddCountersPlacementReplacement
    );
    let crate::model::CompilerStaticAbilityPayloadCore::AddCountersPlacementReplacement {
        filter,
        player_filter,
        counter_type,
        additional,
        ..
    } = &counter_ability.payload
    else {
        panic!("expected additive counter replacement payload: {counter_ability:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(crate::filter::PlayerFilter::You));
    assert_eq!(
        filter.card_types,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ]
    );
    assert_eq!(*player_filter, None);
    assert_eq!(*counter_type, Some(CounterType::PlusOnePlusOne));
    assert_eq!(*additional, 1);

    let heart = lex_line(
        "Prevent all damage that would be dealt to and dealt by enchanted creature.",
        0,
    )
    .expect("combined attached prevention should lex");
    let heart_ability =
        super::super::keyword_static::parse_attached_prevent_all_damage_dealt_to_and_by_attached_line(
            &heart,
        )
        .expect("combined attached prevention should parse")
        .expect("combined attached prevention should be recognized");
    let heart_debug = format!("{heart_ability:#?}");
    assert!(
        heart_debug.contains("AttachedStaticAbilityGrant")
            && heart_debug.contains("PreventAllDamageDealtToAndByThisPermanent"),
        "combined prevention must remain one attached event-layer ability: {heart_debug}"
    );

    let ocelot = lex_line(
        "Then if you have the city's blessing, for each token you control that entered this turn, create a token that's a copy of it.",
        0,
    )
    .expect("conditional for-each copy should lex");
    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&ocelot)
        .expect("conditional for-each copy should parse");
    let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one conditional token-copy clause: {parsed:#?}");
    };
    assert!(
        matches!(
            predicate,
            crate::cards::builders::PredicateAst::PlayerHasCitysBlessing { .. }
        ),
        "conditional token-copy clause must retain its city-blessing predicate: {parsed:#?}"
    );
    assert!(if_false.is_empty());
    let [EffectAst::ForEachObject { filter, effects }] = if_true.as_slice() else {
        panic!("expected object iteration under city-blessing predicate: {parsed:#?}");
    };
    assert!(filter.token);
    assert!(filter.entered_battlefield_this_turn);
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(subject_verb)]
            if matches!(
                &subject_verb.action,
                SubjectVerbActionAst::CreateTokenCopy { .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
            )
    ));
}

#[test]
pub(super) fn rewrite_grammar_hardened_scales_counter_line_matches_static_shape() {
    let tokens = lex_line(
        "If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on it instead.",
        0,
    )
    .expect("rewrite lexer should classify additive counter replacement line");

    let parsed = super::super::keyword_static::parse_double_counters_replacement_line(&tokens)
        .expect("additive counter replacement line should parse");
    assert!(
        matches!(
            parsed,
            Some(ref ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::AddCountersPlacementReplacement
        ),
        "got {parsed:?}"
    );
}

#[test]
pub(super) fn party_size_value_parses_from_equal_to_clause() {
    let lexed = lex_line(
        "You gain life equal to the number of creatures in your party.",
        0,
    )
    .expect("lex should succeed");
    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("gain-life party sentence should parse");
    let debug = format!("{parsed:?}");
    assert!(debug.contains("PartySize"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_doubling_season_token_line_matches_static_shape() {
    let tokens = lex_line(
        "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.",
        0,
    )
    .expect("rewrite lexer should classify token-doubling static line");

    let parsed =
        super::super::keyword_static::parse_double_token_creation_replacement_line(&tokens)
            .expect("token-doubling static line should parse");
    assert!(
        matches!(
            parsed,
            Some(ref ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::DoubleTokenCreationReplacement
        ),
        "got {parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_grammar_token_creation_replacement_probe_matches_static_shape() {
    let tokens = lex_line(
        "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.",
        0,
    )
    .expect("rewrite lexer should classify token-creation replacement static line");

    let parsed =
        super::super::keyword_static::parse_double_token_creation_replacement_line(&tokens)
            .expect("token-creation replacement static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::AddTokenCreationReplacement
    ));
}

#[test]
pub(super) fn rewrite_grammar_named_source_characteristic_pt_probe_matches_static_shape() {
    let tokens = lex_line(
        "Power Tester's power is equal to the number of creatures you control.",
        0,
    )
    .expect("rewrite lexer should classify named-source characteristic P/T static line");

    let ability = super::super::keyword_static::parse_characteristic_defining_pt_line(&tokens)
        .expect("named-source characteristic P/T static line should parse")
        .expect("named-source characteristic P/T should be typed");
    assert_eq!(
        ability.id(),
        crate::static_abilities::StaticAbilityId::CharacteristicDefiningPT
    );
    let crate::model::CompilerStaticAbilityPayloadCore::CharacteristicDefiningPt {
        power,
        toughness,
    } = &ability.payload
    else {
        panic!("expected named characteristic P/T payload: {ability:#?}");
    };
    assert!(power.has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject));
    assert!(toughness.has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject));

    let ordinary = lex_line(
        "Equipped creature's power and toughness are each equal to the number of creatures you control.",
        0,
    )
    .expect("ordinary possessive characteristic fixture should lex");
    let ordinary = super::super::keyword_static::parse_characteristic_defining_pt_line(&ordinary)
        .expect("ordinary possessive characteristic line should parse")
        .expect("ordinary possessive characteristic line should be typed");
    let crate::model::CompilerStaticAbilityPayloadCore::CharacteristicDefiningPt {
        power,
        toughness,
    } = &ordinary.payload
    else {
        panic!("expected ordinary characteristic P/T payload: {ordinary:#?}");
    };
    assert!(!power.has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject));
    assert!(!toughness.has_surface_hint(ironsmith_core::ValueSurfaceHint::SourceNameSubject));
}

#[test]
pub(super) fn characteristic_pt_count_binds_its_controller_graveyard_to_the_affected_object() {
    let tokens = lex_line(
        "This token's power and toughness are each equal to the number of creature cards in its controller's graveyard.",
        0,
    )
    .expect("controller-relative characteristic fixture should lex");
    let ability = super::super::keyword_static::parse_characteristic_defining_pt_line(&tokens)
        .expect("controller-relative characteristic P/T should parse")
        .expect("controller-relative characteristic P/T should be typed");
    let crate::model::CompilerStaticAbilityPayloadCore::CharacteristicDefiningPt {
        power,
        toughness,
    } = &ability.payload
    else {
        panic!("expected a characteristic-defining P/T payload: {ability:#?}");
    };
    assert_eq!(power.unhinted(), toughness.unhinted());
    let Value::Count(filter) = power.unhinted() else {
        panic!("expected a controller-relative creature count: {power:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
    );
}

#[test]
pub(super) fn rewrite_grammar_living_conundrum_empty_library_draw_skip_matches_static_shape() {
    let tokens = lex_line(
        "If you would draw a card while your library has no cards in it, skip that draw instead.",
        0,
    )
    .expect("rewrite lexer should classify Living Conundrum draw-skip replacement line");

    assert!(
        super::super::grammar::abilities::is_draw_replacement_skip_empty_library_line_lexed(
            &tokens
        ),
        "grammar-owned empty-library draw-skip replacement probe should match"
    );

    let parsed =
        super::super::keyword_static::parse_draw_replacement_skip_empty_library_line(&tokens)
            .expect("empty-library draw-skip replacement static line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::DrawReplacementSkipEmptyLibrary
    ));
}

#[test]
pub(super) fn rewrite_grammar_empty_library_draw_win_lowers_to_conditional_replacement() {
    let text =
        "If you would draw a card while your library has no cards in it, you win the game instead.";
    let tokens = lex_line(text, 0).expect("empty-library draw-win replacement should lex");

    assert!(
        super::super::grammar::abilities::is_draw_replacement_win_empty_library_line_lexed(&tokens),
        "grammar-owned empty-library draw-win replacement probe should match"
    );

    let ability = super::super::keyword_static::parse_conditional_draw_replacement_line(&tokens)
        .expect("empty-library draw-win replacement should parse")
        .expect("empty-library draw-win replacement should be recognized");
    assert_eq!(ability.id(), StaticAbilityId::ConditionalDrawReplacement);

    let crate::model::CompilerStaticAbilityPayloadCore::ConditionalDrawReplacement {
        condition,
        replacement_effects,
        display,
        ..
    } = &ability.payload
    else {
        panic!("expected conditional draw replacement payload: {ability:#?}");
    };
    assert!(matches!(
        condition,
        crate::cards::builders::PredicateAst::ValueComparison {
            left: crate::effect::Value::CardsInLibrary(crate::target::PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: crate::effect::Value::Fixed(0),
        }
    ));
    assert_eq!(replacement_effects.len(), 1);
    assert!(matches!(
        &replacement_effects[0],
        crate::cards::builders::EffectAst::SubjectVerb(
            crate::cards::builders::SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::WinGame,
                ..
            }
        )
    ));
    assert_eq!(display, text);
}

#[test]
pub(super) fn aether_refinery_energy_doubling_static_line_parses_as_replacement() {
    let tokens = lex_line(
        "if you would get one or more {e} (energy counters), you get twice that many {e} instead.",
        0,
    )
    .expect("Aether Refinery energy replacement line should lex");

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Aether Refinery energy replacement static parser should not error");
    let direct = super::super::keyword_static::parse_double_counters_replacement_line(&tokens)
        .expect("Aether Refinery direct energy replacement parser should not error");
    assert!(
        matches!(
            parsed.as_deref(),
            Some([crate::cards::builders::StaticAbilityAst::Static(ability)])
                if ability.id() == crate::static_abilities::StaticAbilityId::DoubleCountersReplacement
        ),
        "expected player energy double-counters replacement, words={:?}, direct={direct:?}, got {parsed:?}",
        super::super::token_word_refs(&tokens)
    );
}

#[test]
pub(super) fn aether_refinery_oracle_dispatches_energy_doubling_line_as_static() {
    let card =
        CardBuilder::new(CardId::new(), "Aether Refinery").card_types(vec![CardType::Artifact]);
    let preprocessed = super::super::preprocess::preprocess_document(
        card,
        "If you would get one or more {E} (energy counters), you get twice that many {E} instead.\n\
         {T}: You get {E}, then you may pay one or more {E}. If you do, create an X/X black Aetherborn creature token, where X is the amount of {E} paid this way.",
    )
    .expect("Aether Refinery oracle should preprocess");
    let cst = super::super::document_parser::recognize_document(&preprocessed, false)
        .expect("Aether Refinery oracle should dispatch");

    assert!(
        matches!(
            cst.lines.first(),
            Some(super::super::recognized_document::RecognizedLine::Static(_))
        ),
        "expected Aether Refinery energy replacement to dispatch as static, got {cst:?}"
    );
}

#[test]
pub(super) fn parse_sages_of_the_anima_draw_replacement_static_line() {
    let tokens = lex_line(
        "If you would draw a card, instead reveal the top three cards of your library. Put all creature cards revealed this way into your hand and the rest on the bottom of your library in any order.",
        0,
    )
    .expect("Sages of the Anima draw replacement line should lex");

    let direct =
        super::super::keyword_static::parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(
            &tokens,
        )
        .expect("direct Sages draw replacement parser should not error");
    assert!(
        matches!(
            direct,
            Some(ref ability)
                if ability.id()
                    == crate::static_abilities::StaticAbilityId::DrawReplacementRevealTopMatchingToHandRestBottom
        ),
        "expected direct draw replacement static ability, got {direct:?}"
    );

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Sages of the Anima draw replacement line should parse");
    assert!(
        matches!(
            parsed.as_deref(),
            Some([
                crate::cards::builders::StaticAbilityAst::Static(ability)
            ]) if ability.id() == crate::static_abilities::StaticAbilityId::DrawReplacementRevealTopMatchingToHandRestBottom
        ),
        "expected draw replacement static ability, got {parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_grammar_replacement_static_probes_match_keyword_static_shapes() {
    let library_tokens = lex_line(
        "If an effect causes you to discard a card, you may discard it to the top of your library instead of into your graveyard.",
        0,
    )
    .expect("rewrite lexer should classify Library of Leng replacement line");
    assert!(
        super::super::grammar::abilities::is_effect_discard_to_library_replacement_line_lexed(
            &library_tokens
        ),
        "grammar-owned effect discard replacement probe should match"
    );
    assert!(
        super::super::keyword_static::parse_effect_discard_to_library_replacement_line(
            &library_tokens
        )
        .expect("effect discard replacement line should parse")
        .is_some()
    );

    let shuffle_tokens = lex_line(
        "If Darksteel Colossus would be put into a graveyard from anywhere, reveal Darksteel Colossus and shuffle it into its owner's library instead.",
        0,
    )
    .expect("rewrite lexer should classify shuffle-into-library replacement line");
    assert!(
        super::super::grammar::abilities::is_shuffle_into_library_from_graveyard_line_lexed(
            &shuffle_tokens
        ),
        "grammar-owned shuffle-into-library probe should match"
    );
    assert!(
        super::super::keyword_static::parse_shuffle_into_library_from_graveyard_line(
            &shuffle_tokens
        )
        .expect("shuffle-into-library replacement line should parse")
        .is_some()
    );

    let artifact_land_tokens = lex_line(
        "Nontoken artifacts you control are lands in addition to their other types.",
        0,
    )
    .expect("rewrite lexer should classify type-addition static line");
    let parsed_artifact_land =
        super::super::keyword_static::parse_subject_are_card_types_in_addition_to_their_other_types_line(
            &artifact_land_tokens,
        )
        .expect("type-addition static line should parse");
    assert!(matches!(
        parsed_artifact_land,
        Some(abilities) if abilities.iter().any(|ability| {
            ability.id() == crate::static_abilities::StaticAbilityId::AddCardTypes
        })
    ));

    let discard_tokens = lex_line(
        "If Mox Diamond would enter the battlefield, you may discard a land card instead. If you don't, put it into its owner's graveyard.",
        0,
    )
    .expect("rewrite lexer should classify discard-or-redirect replacement line");
    assert!(
        super::super::keyword_static::parse_discard_or_redirect_replacement_line(&discard_tokens)
            .expect("discard-or-redirect replacement line should parse")
            .is_some()
    );
}

#[test]
pub(super) fn rewrite_discard_or_redirect_replacement_stays_static_through_document_dispatch() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Mox Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "If this artifact would enter the battlefield, you may discard a land card instead. If you do, put this artifact onto the battlefield. If you don't, put it into its owner's graveyard.",
        )
        .expect("discard-or-redirect replacement should bypass effect-statement dispatch");

    assert!(
        definition.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::DiscardOrRedirectReplacement
            )
        }),
        "expected a typed discard-or-redirect static ability, got {:#?}",
        definition.abilities
    );
}

#[test]
pub(super) fn rewrite_cast_restriction_keeps_multiword_no_permanents_named_card_name() {
    let tokens = lex_line(
        "Cast this spell only if no permanents named Tidal Influence are on the battlefield.",
        0,
    )
    .expect("rewrite lexer should classify cast restriction line");

    let parsed = super::super::util::parse_cast_this_spell_only_line(&tokens)
        .expect("cast restriction should parse")
        .expect("cast restriction should produce a static ability");

    assert_eq!(parsed.id(), StaticAbilityId::ThisSpellCastRestriction);
    assert_eq!(
        parsed.display(),
        "Cast this spell only if no permanents named Tidal Influence are on the battlefield."
    );
    let debug = format!("{parsed:#?}");
    assert!(
        debug.contains("if no permanents named Tidal Influence"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_grammar_krrik_life_payment_probe_matches_static_line() {
    let tokens = lex_line(
        "For each {B} in a cost, you may pay 2 life rather than pay that mana.",
        0,
    )
    .expect("rewrite lexer should classify Krrik static line");

    assert!(
        super::super::grammar::abilities::is_krrik_black_mana_life_payment_line_lexed(&tokens),
        "grammar-owned Krrik probe should match the static line"
    );

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Krrik static line should parse")
        .expect("Krrik static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::BlackManaMayBePaidWithLife
    ));
}

#[test]
pub(super) fn rewrite_grammar_once_each_turn_enchantment_life_cost_grant_is_typed() {
    let tokens = lex_line(
        "Once during each of your turns, you may cast an enchantment spell by paying life equal to its mana value rather than paying its mana cost.",
        0,
    )
    .expect("life-equal-mana-value grant line should lex");

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("life-equal-mana-value grant line should parse")
        .expect("life-equal-mana-value grant line should produce an ability");

    let [crate::cards::builders::StaticAbilityAst::Static(ability)] = parsed.as_slice() else {
        panic!("expected one static life-cost grant, got {parsed:?}");
    };
    let crate::model::CompilerStaticAbilityPayloadCore::Grants(spec) = &ability.payload else {
        panic!("life-equal-mana-value ability should expose a grant spec: {ability:?}");
    };
    assert_eq!(spec.zone, Zone::Hand);
    assert_eq!(spec.filter.card_types, [CardType::Enchantment]);
    assert!(matches!(
        spec.grantable,
        crate::model::CompilerGrantableCore::DerivedAlternativeCast(
            ironsmith_core::DerivedAlternativeCast::LifeEqualManaValueFromHand {
                usage_limit: Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns)
            }
        )
    ));
}

#[test]
pub(super) fn rewrite_demon_sacrifice_cost_binds_mana_value_pump_to_cost_object() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Demon Probe")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .parse_text(
            "Once during each of your turns, you may cast an enchantment spell by paying life equal to its mana value rather than paying its mana cost.\n\
             {2}{B}, Sacrifice another enchantment: This creature gets +X/+0 until end of turn, where X is the sacrificed enchantment's mana value.",
        )
        .expect("Demon-style life-cost grant and sacrifice pump should parse");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("sacrifice_cost_0")
            && debug.contains("ManaValueOf")
            && debug.contains("WhereXIs")
            && debug.contains("Tagged(")
            && debug.contains("Enchantment")
            && !debug.contains("the sacrificed enchantment'"),
        "sacrificed enchantment should remain the typed basis of the pump: {debug}"
    );
}

#[test]
pub(super) fn rewrite_grammar_untap_each_other_players_step_probe_splits_subject_tokens() {
    let tokens = lex_line(
        "Untap all permanents you control during each other player's untap step.",
        0,
    )
    .expect("rewrite lexer should classify untap-step static line");

    let spec =
        super::super::grammar::abilities::split_untap_each_other_players_untap_step_line_lexed(
            &tokens,
        )
        .expect("grammar-owned untap-step probe should match the static line");

    assert_eq!(
        render_token_slice(spec.subject_tokens),
        "permanents you control",
        "subject tokens should stop before the untap-step suffix"
    );
}

#[test]
pub(super) fn rewrite_grammar_players_cant_pay_life_or_sacrifice_probe_matches_static_line() {
    let tokens = lex_line(
        "Players can't pay life or sacrifice nonland permanents to cast spells or activate abilities.",
        0,
    )
    .expect("rewrite lexer should classify anti-life-payment static line");

    assert!(
        super::super::grammar::abilities::is_players_cant_pay_life_or_sacrifice_line_lexed(&tokens),
        "grammar-owned anti-life-payment probe should match the static line"
    );

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("anti-life-payment static line should parse")
        .expect("anti-life-payment static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CantPayLifeOrSacrificeNonlandForCastOrActivate
    ));
}

#[test]
pub(super) fn rewrite_grammar_minimum_spell_total_mana_probe_matches_static_line() {
    let tokens = lex_line(
        "As long as Trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast.",
        0,
    )
    .expect("rewrite lexer should classify minimum-spell-total-mana static line");

    assert!(
        super::super::grammar::abilities::is_minimum_spell_total_mana_three_line_lexed(&tokens),
        "grammar-owned minimum-spell-total-mana probe should match the static line"
    );

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("minimum-spell-total-mana static line should parse")
        .expect("minimum-spell-total-mana static line should produce abilities");

    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::StaticAbilityAst::Static(ability)]
            if ability.id() == crate::static_abilities::StaticAbilityId::MinimumSpellTotalMana
    ));
}

#[test]
pub(super) fn cultist_of_the_absolute_static_line_parses_as_static_abilities() {
    let tokens = lex_line(
        "Commander creatures you own get +3/+3 and have flying, deathtouch, \"Ward—Pay 3 life,\" and \"At the beginning of your upkeep, sacrifice a creature.\"",
        0,
    )
    .expect("Cultist of the Absolute static line should lex");

    let trailing = super::super::keyword_static::parse_anthem_with_trailing_segments_line(&tokens)
        .expect("trailing anthem rule should not error");
    let trailing_debug = format!("{trailing:#?}");
    assert!(
        trailing.is_some(),
        "expected trailing anthem rule to parse Cultist, got {trailing_debug}"
    );

    let parsed = super::super::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
        .expect("Cultist of the Absolute static line should not error");
    let debug = format!("{parsed:#?}");

    assert!(parsed.is_some(), "expected static abilities, got {debug}");
    assert!(
        super::super::grammar::effects::gain_ability_shapes::parse_source_gain_ability_shape(
            &tokens
        )
        .is_none(),
        "a quantified commander anthem must not be claimed as a source gain-ability statement"
    );

    let card = CardBuilder::new(CardId::new(), "Cultist of the Absolute")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Background]);
    let preprocessed = super::super::preprocess::preprocess_document(
        card,
        "Commander creatures you own get +3/+3 and have flying, deathtouch, \"Ward—Pay 3 life,\" and \"At the beginning of your upkeep, sacrifice a creature.\"",
    )
    .expect("Cultist document should preprocess");
    let (cst, trace) = crate::parse_trace::capture(|| {
        super::super::document_parser::recognize_document(&preprocessed, false)
    });
    let cst = cst.expect("Cultist document should parse to CST");
    assert!(
        cst.lines.iter().any(|line| matches!(
            line,
            super::super::recognized_document::RecognizedLine::Static(_)
        )),
        "expected Cultist line to classify as static, got {cst:?}\n{}",
        trace.render()
    );
}

#[test]
pub(super) fn rewrite_grammar_permanents_enter_tapped_probe_matches_static_line() {
    let tokens = lex_line("Permanents enter tapped.", 0)
        .expect("rewrite lexer should classify permanents-enter-tapped static line");

    assert!(
        super::super::grammar::abilities::is_permanents_enter_tapped_line_lexed(&tokens),
        "grammar-owned permanents-enter-tapped probe should match the static line"
    );

    let parsed = super::super::keyword_static::parse_permanents_enter_tapped_line(&tokens)
        .expect("permanents-enter-tapped static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::AllPermanentsEnterTapped
    ));
}

#[test]
pub(super) fn rewrite_grammar_creatures_entering_dont_trigger_probe_matches_static_line() {
    let tokens = lex_line("Creatures entering don't cause abilities to trigger.", 0)
        .expect("rewrite lexer should classify anti-trigger static line");

    assert!(
        super::super::grammar::abilities::is_creatures_entering_dont_cause_abilities_to_trigger_line_lexed(
            &tokens
        ),
        "grammar-owned anti-trigger probe should match the static line"
    );

    let parsed =
        super::super::keyword_static::parse_creatures_entering_dont_cause_abilities_to_trigger_line(
            &tokens,
        )
        .expect("anti-trigger static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger
    ));
}

#[test]
pub(super) fn rewrite_grammar_combat_damage_using_toughness_probe_tracks_subject_variant() {
    let tokens = lex_line(
        "Each creature you control assigns combat damage equal to its toughness rather than its power.",
        0,
    )
    .expect("rewrite lexer should classify toughness-damage static line");

    assert_eq!(
        super::super::grammar::abilities::parse_creatures_assign_combat_damage_using_toughness_line_lexed(
            &tokens
        ),
        Some(super::super::grammar::abilities::CombatDamageUsingToughnessSubject::EachCreatureYouControl),
        "grammar-owned toughness-damage probe should preserve the subject variant"
    );

    let parsed =
        super::super::keyword_static::parse_creatures_assign_combat_damage_using_toughness_line(
            &tokens,
        )
        .expect("toughness-damage static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness
    ));
}

#[test]
pub(super) fn rewrite_grammar_defending_player_combat_damage_assignment_probe_matches_static_line()
{
    let tokens = lex_line(
        "Rather than the attacking player, you assign the combat damage of each creature attacking you. You can divide that creature's combat damage as you choose among any of the creatures blocking it.",
        0,
    )
    .expect("rewrite lexer should classify defending-player combat-damage assignment static line");

    assert!(
        super::super::grammar::abilities::is_you_assign_combat_damage_of_creatures_attacking_you_line_lexed(
            &tokens
        ),
        "grammar-owned defending-player combat-damage assignment probe should match Defensive Formation's static line"
    );

    let parsed =
        super::super::keyword_static::parse_you_assign_combat_damage_of_creatures_attacking_you_line(
            &tokens,
        )
        .expect("defending-player combat-damage assignment static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::YouAssignCombatDamageOfCreaturesAttackingYou
    ));
}

#[test]
pub(super) fn rewrite_grammar_zilortha_lethal_damage_power_static_line() {
    let tokens = lex_line(
        "Lethal damage dealt to creatures you control is determined by their power rather than their toughness.",
        0,
    )
    .expect("rewrite lexer should classify Zilortha lethal-damage static line");

    assert!(
        super::super::grammar::abilities::is_lethal_damage_to_creatures_you_control_uses_power_line_lexed(
            &tokens
        ),
        "grammar-owned lethal-damage power probe should match Zilortha's static line"
    );

    let parsed =
        super::super::keyword_static::parse_lethal_damage_to_creatures_you_control_uses_power_line(
            &tokens,
        )
        .expect("Zilortha lethal-damage static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower
    ));
}

#[test]
pub(super) fn rewrite_grammar_players_cant_cycle_probe_matches_static_line() {
    let tokens = lex_line("Players can't cycle cards.", 0)
        .expect("rewrite lexer should classify anti-cycle static line");

    assert!(
        super::super::grammar::abilities::is_players_cant_cycle_line_lexed(&tokens),
        "grammar-owned anti-cycle probe should match the static line"
    );

    let parsed = super::super::keyword_static::parse_players_cant_cycle_line(&tokens)
        .expect("anti-cycle static line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::PlayersCantCycle
    ));
}

#[test]
pub(super) fn rewrite_grammar_exact_static_line_probes_match_simple_keyword_static_shapes() {
    type Probe = fn(&[crate::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::lexer::OwnedLexToken],
    ) -> Result<Option<crate::model::CompilerStaticAbilityCore>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "Players skip their upkeep steps.",
            super::super::grammar::abilities::is_players_skip_upkeep_line_lexed as Probe,
            super::super::keyword_static::parse_players_skip_upkeep_line as Parser,
            crate::static_abilities::StaticAbilityId::PlayersSkipUpkeep,
        ),
        (
            "Skip your draw step.",
            super::super::grammar::abilities::is_skip_your_draw_step_line_lexed as Probe,
            super::super::keyword_static::parse_skip_your_draw_step_static_line as Parser,
            crate::static_abilities::StaticAbilityId::PlayerSkipsDrawStep,
        ),
        (
            "All permanents are colorless.",
            super::super::grammar::abilities::is_all_permanents_colorless_line_lexed as Probe,
            super::super::keyword_static::parse_all_permanents_colorless_line as Parser,
            crate::static_abilities::StaticAbilityId::MakeColorless,
        ),
        (
            "Nonbasic lands are Mountains.",
            (|tokens| {
                super::super::keyword_static::parse_nonbasic_lands_are_basic_land_type_line(tokens)
                    .ok()
                    .flatten()
                    .is_some()
            }) as Probe,
            super::super::keyword_static::parse_nonbasic_lands_are_basic_land_type_line as Parser,
            crate::static_abilities::StaticAbilityId::SetLandSubtypes,
        ),
        (
            "Enchanted land is an Island.",
            (|tokens| {
                super::super::keyword_static::parse_nonbasic_lands_are_basic_land_type_line(tokens)
                    .ok()
                    .flatten()
                    .is_some()
            }) as Probe,
            super::super::keyword_static::parse_nonbasic_lands_are_basic_land_type_line as Parser,
            crate::static_abilities::StaticAbilityId::SetLandSubtypes,
        ),
        (
            "All lands are no longer snow.",
            super::super::grammar::abilities::is_remove_snow_line_lexed as Probe,
            super::super::keyword_static::parse_remove_snow_line as Parser,
            crate::static_abilities::StaticAbilityId::RemoveSupertypes,
        ),
        (
            "You have no maximum hand size.",
            super::super::grammar::abilities::is_no_maximum_hand_size_line_lexed as Probe,
            super::super::keyword_static::parse_no_maximum_hand_size_line as Parser,
            crate::static_abilities::StaticAbilityId::NoMaximumHandSize,
        ),
        (
            "This can be your commander.",
            super::super::grammar::abilities::is_can_be_your_commander_line_lexed as Probe,
            super::super::keyword_static::parse_can_be_your_commander_line as Parser,
            crate::static_abilities::StaticAbilityId::CanBeCommander,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify simple static line");

        assert!(
            probe(&tokens),
            "{text}: grammar-owned exact-shape probe should match"
        );

        let parsed = parser(&tokens).expect("simple static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }
}

#[test]
pub(super) fn rewrite_grammar_creatures_cant_block_probe_matches_static_line() {
    let tokens = lex_line("Creatures can't block.", 0)
        .expect("rewrite lexer should classify cant-block static line");

    assert!(
        super::super::grammar::abilities::is_creatures_cant_block_line_lexed(&tokens),
        "grammar-owned cant-block probe should match the static line"
    );

    let parsed = super::super::keyword_static::parse_creatures_cant_block_line(&tokens)
        .expect("cant-block static line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::GrantStaticAbility { filter, ability, .. })
            if filter == crate::filter::ObjectFilter::creature()
                && matches!(
                    ability.as_ref(),
                    crate::cards::builders::StaticAbilityAst::Static(ability)
                        if ability.id() == crate::static_abilities::StaticAbilityId::CantBlock
                )
    ));
}

#[test]
pub(super) fn rewrite_grammar_prevention_static_line_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::lexer::OwnedLexToken],
    ) -> Result<Option<crate::model::CompilerStaticAbilityCore>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "Prevent all damage that would be dealt to creatures.",
            super::super::grammar::abilities::is_prevent_all_damage_dealt_to_creatures_line_lexed as Probe,
            super::super::keyword_static::parse_prevent_all_damage_dealt_to_creatures_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllDamageDealtToCreatures,
        ),
        (
            "If damage would be dealt to another creature you control, prevent that damage. Put a +1/+1 counter on that creature for each 1 damage prevented this way.",
            super::super::grammar::abilities::is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed as Probe,
            super::super::keyword_static::parse_prevent_damage_to_other_creature_you_control_put_counters_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventDamageToOtherCreatureYouControlPutCountersInstead,
        ),
        (
            "During your turn, prevent all damage that would be dealt to this creature.",
            super::super::grammar::abilities::is_during_your_turn_prevent_all_damage_to_source_line_lexed
                as Probe,
            super::super::keyword_static::parse_during_your_turn_prevent_all_damage_to_source_line
                as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllDamageToSelf,
        ),
        (
            "Prevent all combat damage that would be dealt to this creature.",
            super::super::grammar::abilities::is_prevent_all_combat_damage_to_source_line_lexed as Probe,
            super::super::keyword_static::parse_prevent_all_combat_damage_to_source_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllCombatDamageToSelf,
        ),
        (
            "Prevent all noncombat damage that would be dealt to other creatures you control.",
            super::super::grammar::abilities::is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed as Probe,
            super::super::keyword_static::parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllNoncombatDamageToOtherCreaturesYouControl,
        ),
        (
            "Prevent all noncombat damage that would be dealt to creatures you control.",
            super::super::grammar::abilities::is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed as Probe,
            super::super::keyword_static::parse_prevent_all_noncombat_damage_to_matching_permanents_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllNoncombatDamageToPermanentsMatching,
        ),
        (
            "Prevent all damage that would be dealt to this permanent by creatures.",
            super::super::grammar::abilities::is_prevent_all_damage_to_source_by_creatures_line_lexed as Probe,
            super::super::keyword_static::parse_prevent_all_damage_to_source_by_creatures_line as Parser,
            crate::static_abilities::StaticAbilityId::PreventAllDamageToSelfByCreatures,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify prevention static line");

        assert!(probe(&tokens), "{text}: grammar-owned prevention probe should match");

        let parsed = parser(&tokens).expect("prevention static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }
}

#[test]
pub(super) fn parse_prevent_all_damage_by_opponents_creatures_effect_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thwart Effect Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Prevent all damage that would be dealt this turn by creatures your opponents control.",
        )
        .expect("prevent-all damage clause with opponent creature source filter should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("preventalldamageeffect")
            && spell_debug.contains("from_source")
            && spell_debug.contains("creature")
            && spell_debug.contains("opponent"),
        "expected source-filter prevent-all-damage effect, got {spell_debug}"
    );
}

#[test]
pub(super) fn parse_reverberation_target_sorcery_damage_redirect_effect_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reverberation")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead.",
        )
        .expect("Reverberation target-sorcery redirect clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("redirectnexttimedamagetosourceeffect")
            && spell_debug.contains("source: target")
            && spell_debug.contains("target: none")
            && spell_debug.contains("destination: sourcecontroller")
            && spell_debug.contains("all_this_turn: true")
            && spell_debug.contains("target")
            && spell_debug.contains("sorcery"),
        "expected targeted sorcery source redirected to its controller, got {spell_debug}"
    );
}

#[test]
pub(super) fn rewrite_grammar_attached_prevent_all_damage_to_enchanted_creature_line() {
    let tokens = lex_line(
        "Prevent all damage that would be dealt to enchanted creature.",
        0,
    )
    .expect("rewrite lexer should classify attached prevention static line");

    let parsed =
        super::super::keyword_static::parse_attached_prevent_all_damage_dealt_to_attached_line(
            &tokens,
        )
        .expect("attached prevention line should parse");

    assert!(matches!(
        parsed,
        Some(crate::cards::builders::StaticAbilityAst::AttachedStaticAbilityGrant { ability, .. })
            if matches!(
                ability.as_ref(),
                crate::cards::builders::StaticAbilityAst::Static(ability)
                    if ability.id() == crate::static_abilities::StaticAbilityId::PreventAllDamageToSelf
            )
    ));
}

#[test]
pub(super) fn rewrite_grammar_skulk_rules_text_probe_matches_static_line() {
    let tokens = lex_line(
        "Creatures with power less than this creature's power can't block this creature.",
        0,
    )
    .expect("rewrite lexer should classify skulk rules text");

    assert!(
        super::super::grammar::abilities::is_skulk_rules_text_line_lexed(&tokens),
        "grammar-owned skulk probe should match"
    );

    let parsed = super::super::keyword_static::parse_skulk_rules_text_line(&tokens)
        .expect("skulk line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::CantBeBlockedByLowerPowerThanSource
    ));
}

#[test]
pub(super) fn rewrite_grammar_tap_status_and_max_cards_helpers_match_keyword_static_shapes() {
    for (text, expected) in [
        (
            "this creature is tapped",
            crate::cards::builders::PredicateAst::SourceIsTapped,
        ),
        (
            "this permanent is untapped",
            crate::cards::builders::PredicateAst::SourceIsUntapped,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify tap-status condition");
        assert_eq!(
            super::super::grammar::abilities::parse_source_tap_status_condition_lexed(&tokens),
            Some(expected),
            "{text}: grammar-owned tap-status helper should match"
        );
    }

    let tokens = lex_line(
        "cards in the hand of the opponent with the most cards in hand",
        0,
    )
    .expect("rewrite lexer should classify max-cards-in-hand value");
    assert_eq!(
        super::super::grammar::values::parse_max_cards_in_hand_value_lexed(&tokens),
        Some(crate::effect::Value::MaxCardsInHand(
            crate::target::PlayerFilter::Opponent,
        )),
        "grammar-owned max-cards helper should match Adamaro-style wording"
    );
}

#[test]
pub(super) fn rewrite_grammar_flying_block_probes_match_keyword_static_shapes() {
    let flying_only = lex_line(
        "This creature can't be blocked except by creatures with flying.",
        0,
    )
    .expect("rewrite lexer should classify flying-only restriction");
    assert_eq!(
        super::super::grammar::abilities::parse_flying_block_restriction_line_lexed(&flying_only),
        Some(super::super::grammar::abilities::FlyingBlockRestrictionKind::FlyingOnly),
        "grammar-owned flying-only probe should match"
    );
    let parsed_flying_only =
        super::super::keyword_static::parse_flying_restriction_line(&flying_only)
            .expect("flying-only restriction should parse");
    assert!(parsed_flying_only.is_some(), "{parsed_flying_only:?}");

    let flying_or_reach = lex_line(
        "This can't be blocked except by creatures with flying or reach.",
        0,
    )
    .expect("rewrite lexer should classify flying-or-reach restriction");
    assert_eq!(
        super::super::grammar::abilities::parse_flying_block_restriction_line_lexed(
            &flying_or_reach
        ),
        Some(super::super::grammar::abilities::FlyingBlockRestrictionKind::FlyingOrReach),
        "grammar-owned flying-or-reach probe should match"
    );
    let parsed_flying_or_reach =
        super::super::keyword_static::parse_flying_restriction_line(&flying_or_reach)
            .expect("flying-or-reach restriction should parse");
    assert!(
        parsed_flying_or_reach.is_some(),
        "{parsed_flying_or_reach:?}"
    );

    let only_flying = lex_line("Can block only creatures with flying.", 0)
        .expect("rewrite lexer should classify can-block-only-flying restriction");
    assert!(
        super::super::grammar::abilities::is_can_block_only_flying_line_lexed(&only_flying),
        "grammar-owned can-block-only-flying probe should match"
    );
    let parsed_only_flying =
        super::super::keyword_static::parse_can_block_only_flying_line(&only_flying)
            .expect("can-block-only-flying restriction should parse");
    assert!(parsed_only_flying.is_some(), "{parsed_only_flying:?}");

    let subtype_reach = lex_line("This creature can block Dragons as though it had reach.", 0)
        .expect("rewrite lexer should classify subtype reach blocking clause");
    assert_eq!(
        super::super::grammar::abilities::parse_can_block_subtype_as_though_reach_line_lexed(
            &subtype_reach,
        ),
        Some(crate::types::Subtype::Dragon),
        "grammar-owned subtype reach blocking probe should match"
    );
    let parsed_subtype_reach =
        super::super::keyword_static::parse_can_block_subtype_as_though_reach_line(&subtype_reach)
            .expect("subtype reach blocking clause should parse")
            .expect("subtype reach blocking clause should produce an ability");
    assert_eq!(
        parsed_subtype_reach.can_block_as_though_reach_subtype(),
        Some(crate::types::Subtype::Dragon)
    );
}

#[test]
pub(super) fn rewrite_grammar_static_marker_exact_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::lexer::OwnedLexToken]) -> bool;

    for (text, probe, expected_id) in [
        (
            "You have shroud.",
            super::super::grammar::abilities::is_you_have_shroud_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
        (
            "Creatures without flying can't attack.",
            super::super::grammar::abilities::is_creatures_without_flying_cant_attack_line_lexed
                as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
        (
            "This creature can't attack alone.",
            super::super::grammar::abilities::is_this_creature_cant_attack_alone_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
        (
            "This creature can't attack its owner.",
            super::super::grammar::abilities::is_this_creature_cant_attack_its_owner_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::CantAttackItsOwner,
        ),
        (
            "Lands don't untap during their controller's untap steps.",
            super::super::grammar::abilities::is_lands_dont_untap_during_their_controllers_untap_steps_line_lexed as Probe,
            crate::static_abilities::StaticAbilityId::RuleRestriction,
        ),
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify static-marker exact line");
        assert!(probe(&tokens), "{text}: grammar-owned static-marker probe should match");

        let parsed =
            super::super::keyword_static::parse_static_text_marker_line(&tokens).expect("line should parse");
        if expected_id == crate::static_abilities::StaticAbilityId::CantAttackItsOwner {
            assert_eq!(parsed.id(), expected_id, "{text}: {parsed:?}");
        } else {
            assert!(
                parsed.id() == expected_id
                    || parsed.id() == crate::static_abilities::StaticAbilityId::RuleRestriction
                    || parsed.id() == crate::static_abilities::StaticAbilityId::Grants
                    || format!("{parsed:?}").contains("restriction"),
                "{text}: {parsed:?}"
            );
        }
    }
}

#[test]
pub(super) fn rewrite_grammar_assign_damage_as_unblocked_probe_matches_keyword_static_shape() {
    let tokens = lex_line(
        "You may have this creature assign its combat damage as though it weren't blocked.",
        0,
    )
    .expect("rewrite lexer should classify assign-damage-as-unblocked text");
    assert!(
        super::super::grammar::abilities::is_may_assign_damage_as_unblocked_line_lexed(&tokens),
        "grammar-owned assign-damage-as-unblocked probe should match"
    );

    let parsed = super::super::keyword_static::parse_assign_damage_as_unblocked_line(&tokens)
        .expect("assign-damage-as-unblocked line should parse");
    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id()
                == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
    ));

    let definition = ironsmith_compiler_lowering::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Assign Damage Grant Probe",
    )
    .card_types(vec![crate::types::CardType::Enchantment])
    .parse_text(
        "Enchant creature\nEnchanted creature's controller may have it assign its combat damage as though it weren't blocked.",
    )
    .expect("public document route should lower the attached static grant directly");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("AttachedAbilityGrant"), "{debug}");
    assert!(debug.contains("MayAssignDamageAsUnblocked"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_exact_permission_static_line_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::lexer::OwnedLexToken],
    ) -> Result<Option<crate::model::CompilerStaticAbilityCore>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "You may look at the top card of your library any time.",
            super::super::grammar::abilities::is_you_may_look_top_card_any_time_line_lexed as Probe,
            super::super::keyword_static::parse_you_may_look_top_card_any_time_line as Parser,
            crate::static_abilities::StaticAbilityId::LookAtTopCardOfLibrary,
        ),
        (
            "You may look at face-down creatures you don't control any time.",
            super::super::grammar::abilities::is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed
                as Probe,
            super::super::keyword_static::parse_you_may_look_face_down_creatures_you_dont_control_any_time_line
                as Parser,
            crate::static_abilities::StaticAbilityId::LookAtFaceDownCreaturesYouDontControl,
        ),
        (
            "Players play with the top card of their libraries revealed.",
            super::super::grammar::abilities::is_players_play_top_card_libraries_revealed_line_lexed
                as Probe,
            super::super::keyword_static::parse_players_play_top_card_libraries_revealed_line as Parser,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries,
        ),
        (
            "Play with the top card of your library revealed.",
            super::super::grammar::abilities::is_play_top_card_your_library_revealed_line_lexed as Probe,
            super::super::keyword_static::parse_play_top_card_your_library_revealed_line as Parser,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
        ),
        (
            "Your opponents play with their hands revealed.",
            super::super::grammar::abilities::is_your_opponents_play_with_hands_revealed_line_lexed
                as Probe,
            super::super::keyword_static::parse_your_opponents_play_with_hands_revealed_line as Parser,
            crate::static_abilities::StaticAbilityId::OpponentsPlayWithHandsRevealed,
        ),
        (
            "You may cast this spell as though it had flash.",
            super::super::grammar::abilities::is_cast_this_spell_as_though_it_had_flash_line_lexed
                as Probe,
            super::super::keyword_static::parse_cast_this_spell_as_though_it_had_flash_line as Parser,
            crate::static_abilities::StaticAbilityId::Flash,
        ),
        (
            "You may play lands from your graveyard.",
            super::super::grammar::abilities::is_play_lands_from_graveyard_line_lexed as Probe,
            super::super::keyword_static::parse_play_lands_from_graveyard_line as Parser,
            crate::static_abilities::StaticAbilityId::Grants,
        ),
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify exact permission static line");

        assert!(
            probe(&tokens),
            "{text}: grammar-owned exact-shape probe should match"
        );

        let parsed = parser(&tokens).expect("exact permission static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }
}

#[test]
pub(super) fn parse_lens_of_clarity_split_look_permissions() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lens of Clarity Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "You may look at the top card of your library and at face-down creatures you don't control any time.",
        )
        .expect("Lens of Clarity text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LookAtTopCardOfLibrary"),
        "expected top-card look permission, got {debug}"
    );
    assert!(
        debug.contains("LookAtFaceDownCreaturesYouDontControl"),
        "expected face-down look permission, got {debug}"
    );
}

#[test]
pub(super) fn parse_radha_split_top_look_and_top_land_play_permissions() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Radha Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "You may look at the top card of your library any time, and you may play lands from the top of your library.",
        )
        .expect("Radha static line should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LookAtTopCardOfLibrary"),
        "expected top-card look permission, got {debug}"
    );
    assert!(
        debug.contains("PlayFromZone")
            || debug.contains("lands from the top of your library")
            || debug.contains("GrantSpec"),
        "expected top-library land-play grant, got {debug}"
    );
}

#[test]
pub(super) fn parse_lantern_of_insight_public_top_library_static_and_shuffle() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lantern of Insight Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Players play with the top card of their libraries revealed.\n{T}, Sacrifice this artifact: Target player shuffles.",
        )
        .expect("Lantern of Insight text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AllPlayersLookAtTopCardsOfLibraries"),
        "expected public top-library static ability, got {debug}"
    );
    assert!(
        debug.contains("ShuffleLibraryEffect"),
        "expected target-player shuffle activation, got {debug}"
    );
}

#[test]
pub(super) fn parse_telepathy_opponents_play_with_hands_revealed() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Telepathy Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Your opponents play with their hands revealed.")
        .expect("Telepathy text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("OpponentsPlayWithHandsRevealed"),
        "expected opponents' revealed-hand static ability, got {debug}"
    );
}

#[test]
pub(super) fn parse_courser_of_kruphix_public_top_library_and_land_grant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Courser of Kruphix Variant")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .parse_text(
            "Play with the top card of your library revealed.\nYou may play lands from the top of your library.\nLandfall — Whenever a land you control enters, you gain 1 life.",
        )
        .expect("Courser of Kruphix text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AllPlayersLookAtYourTopLibraryCard"),
        "expected public own-top-library static ability, got {debug}"
    );
    assert!(
        debug.contains("PlayFromZone")
            || debug.contains("lands from the top of your library")
            || debug.contains("GrantSpec"),
        "expected top-library land-play grant, got {debug}"
    );
    assert!(
        debug.contains("GainLifeEffect"),
        "expected landfall life trigger, got {debug}"
    );
}

#[test]
pub(super) fn parse_searching_library_static_permissions() {
    let opposition = CardDefinitionBuilder::new(CardId::new(), "Opposition Agent Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flash\nYou control your opponents while they're searching their libraries.\nWhile an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.",
        )
        .expect("Opposition Agent search-control text should parse");
    let opposition_debug = format!("{:?}", opposition.abilities);
    assert!(
        opposition_debug.contains("ControlOpponentsWhileSearchingLibraries"),
        "expected opponent-search control static ability, got {opposition_debug}"
    );
    assert!(
        opposition_debug.contains("OpponentSearchExileFoundCards"),
        "expected opponent-search exile/play static ability, got {opposition_debug}"
    );

    let panglacial = CardDefinitionBuilder::new(CardId::new(), "Panglacial Wurm Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Trample\nWhile you're searching your library, you may cast this card from your library.",
        )
        .expect("Panglacial Wurm library-search cast text should parse");
    let panglacial_debug = format!("{:?}", panglacial.abilities);
    assert!(
        panglacial_debug.contains("CastThisCardFromLibraryWhileSearching"),
        "expected library-search cast static ability, got {panglacial_debug}"
    );
    assert!(
        panglacial_debug.contains("functional_zones: [Library]"),
        "expected Panglacial Wurm permission to function from library, got {panglacial_debug}"
    );
}

#[test]
pub(super) fn parse_final_parting_search_two_split_destinations() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Final Parting Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle.",
        )
        .expect("Final Parting search split should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("is_search: true"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
}

#[test]
pub(super) fn rewrite_grammar_chosen_type_static_line_probes_match_keyword_static_shapes() {
    type Probe = fn(&[crate::lexer::OwnedLexToken]) -> bool;
    type Parser = fn(
        &[crate::lexer::OwnedLexToken],
    ) -> Result<Option<crate::model::CompilerStaticAbilityCore>, CardTextError>;

    for (text, probe, parser, expected_id) in [
        (
            "Enchanted land is the chosen type.",
            super::super::grammar::abilities::is_enchanted_land_is_chosen_type_line_lexed as Probe,
            super::super::keyword_static::parse_enchanted_land_is_chosen_type_line as Parser,
            crate::static_abilities::StaticAbilityId::EnchantedLandIsChosenType,
        ),
        (
            "Double all damage that sources you control of the chosen type would deal.",
            super::super::grammar::abilities::is_double_damage_from_sources_you_control_of_chosen_type_line_lexed as Probe,
            super::super::keyword_static::parse_double_damage_from_sources_you_control_of_chosen_type_line as Parser,
            crate::static_abilities::StaticAbilityId::DoubleDamageFromSourcesYouControlOfChosenType,
        ),
    ] {
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify chosen-type static line");

        assert!(probe(&tokens), "{text}: grammar-owned chosen-type probe should match");

        let parsed = parser(&tokens).expect("chosen-type static line should parse");
        assert!(
            matches!(parsed, Some(ref ability) if ability.id() == expected_id),
            "{text}: {parsed:?}"
        );
    }

    let source_tokens = lex_line(
        "This creature is the chosen type in addition to its other types.",
        0,
    )
    .expect("rewrite lexer should classify chosen-type addition line");

    assert_eq!(
        super::super::grammar::abilities::parse_source_is_chosen_type_in_addition_line_lexed(
            &source_tokens
        ),
        Some("This creature is the chosen type in addition to its other types."),
        "grammar-owned chosen-type addition probe should preserve the display wording"
    );

    let parsed =
        super::super::keyword_static::parse_source_is_chosen_type_in_addition_line(&source_tokens)
            .expect("chosen-type addition line should parse");

    assert!(matches!(
        parsed,
        Some(ability)
            if ability.id() == crate::static_abilities::StaticAbilityId::AddChosenCreatureType
    ));
}

#[test]
pub(super) fn rewrite_static_line_supports_painters_servant_chosen_color_clause() {
    let tokens = lex_line(
        "All cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors.",
        0,
    )
    .expect("rewrite lexer should classify Painter's Servant chosen-color line");

    assert_eq!(
        super::super::lexer::parser_token_word_refs(&tokens),
        vec![
            "all",
            "cards",
            "that",
            "arent",
            "on",
            "the",
            "battlefield",
            "spells",
            "and",
            "permanents",
            "are",
            "the",
            "chosen",
            "color",
            "in",
            "addition",
            "to",
            "their",
            "other",
            "colors",
        ]
    );

    let parsed =
        super::super::keyword_static::parse_all_cards_spells_permanents_add_chosen_color_line(
            &tokens,
        )
        .expect("Painter's Servant chosen-color line should parse");

    assert!(matches!(
        parsed,
        Some(ability) if ability.id() == crate::static_abilities::StaticAbilityId::AddChosenColor
    ));
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_supports_tivit_vote_trigger_body() {
    let triggered_tokens = lex_line(
        "Whenever this creature enters the battlefield or deals combat damage to a player, starting with you, each player votes for evidence or bribery. For each evidence vote, investigate. For each bribery vote, create a Treasure token. You may vote an additional time.",
        0,
    )
    .expect("rewrite lexer should classify tivit trigger probe");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&triggered_tokens);
    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_document_normalizes_labeled_named_tivit_vote_trigger() {
    let text = "Mana cost: {3}{W}{U}{B}\n\
         Type: Legendary Creature — Sphinx Rogue\n\
         Power/Toughness: 6/6\n\
         Flying, ward {3}\n\
         Council's dilemma — Whenever Tivit enters the battlefield or deals combat \
         damage to a player, starting with you, each player votes for evidence or \
         bribery. For each evidence vote, investigate. For each bribery vote, create \
         a Treasure token. You may vote an additional time.";
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), "Tivit, Seller of Secrets")
        .card_types(vec![CardType::Creature]);
    let (semantic, _) =
        parse_text_to_semantic_document(builder.card_builder.clone(), text.to_string(), false)
            .expect("Tivit semantic document should parse");
    assert!(
        semantic.items.iter().any(rewrite_item_is_triggered),
        "expected labeled named-source Tivit vote trigger semantic item, got {:?}",
        semantic.items
    );

    let def = builder
        .parse_text(text)
        .expect("Tivit definition text should parse");

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, crate::ability::AbilityKind::Triggered(_))),
        "expected labeled named-source Tivit vote trigger to parse, got {:?}",
        def.abilities
    );
}

#[test]
pub(super) fn rewrite_lexed_vote_start_sentence_supports_object_votes() {
    let tokens = lex_line(
        "Starting with you, each player votes for a nonland permanent you don't control.",
        0,
    )
    .expect("rewrite lexer should classify council vote probe");

    let parsed = parse_effect_sentence_lexed(&tokens);
    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
pub(super) fn rewrite_lexed_vote_followups_support_vote_conditions_and_winner_filters() {
    for text in [
        "If death gets more votes, each opponent sacrifices a creature of their choice.",
        "If torture gets more votes or the vote is tied, each opponent loses 4 life.",
        "Exile each permanent with the most votes or tied for most votes.",
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify vote follow-up probe");
        let parsed = parse_effect_sentence_lexed(&tokens);
        assert!(parsed.is_ok(), "{text}: {parsed:?}");
    }
}

#[test]
pub(super) fn rewrite_lexed_vote_sentence_sequences_support_representative_line_families() {
    for text in [
        "Starting with you, each player votes for death or torture. If death gets more votes, each opponent sacrifices a creature of their choice. If torture gets more votes or the vote is tied, each opponent loses 4 life.",
        "Each player secretly votes for truth or consequences, then those votes are revealed. For each truth vote, draw a card. Then choose an opponent at random. For each consequences vote, Truth or Consequences deals 3 damage to that player.",
        "Starting with you, each player votes for death or taxes. For each death vote, each opponent sacrifices a creature of their choice. For each taxes vote, Each opponent discards a card.",
    ] {
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify vote sequence probe");
        let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens);
        assert!(parsed.is_ok(), "{text}: {parsed:?}");
    }
}

#[test]
pub(super) fn rewrite_lexed_vote_sentence_sequences_keep_elrond_vote_branches() {
    let text = "Each player secretly votes for fellowship or aid, then those votes are revealed. For each fellowship vote, the voter chooses a creature they control. You gain control of each creature chosen this way, and they gain \"This creature can't attack its owner.\" Then for each aid vote, put a +1/+1 counter on each creature you control.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify elrond vote probe");
    let (parsed, trace) = crate::parse_trace::capture(|| {
        super::super::clause_support::parse_effect_sentences_lexed(&tokens)
    });
    let parsed = parsed.unwrap_or_else(|error| {
        panic!(
            "elrond vote sequence should parse: {error}\n{}",
            trace.render()
        )
    });
    let debug = format!("{parsed:#?}");
    assert!(debug.contains("VoteStart"), "{debug}");
    assert!(debug.contains("VoteOption"), "{debug}");
    assert!(debug.contains("fellowship"), "{debug}");
    assert!(debug.contains("aid"), "{debug}");
    assert!(debug.contains("CantAttackItsOwner"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_parses_common_native_shapes() {
    let dies_tokens = lex_line("another creature dies", 0)
        .expect("rewrite lexer should classify dies trigger probe");
    let dies_during_turn_tokens = lex_line("a Mutant you control dies during your turn", 0)
        .expect("rewrite lexer should classify dies-during-turn trigger probe");
    let upkeep_tokens = lex_line("the beginning of your upkeep", 0)
        .expect("rewrite lexer should classify upkeep trigger probe");
    let etb_tokens = lex_line(
        "one or more goblins enter the battlefield under your control",
        0,
    )
    .expect("rewrite lexer should classify etb trigger probe");
    let spell_tokens = lex_line("you cast an aura, equipment, or vehicle spell", 0)
        .expect("rewrite lexer should classify spell-cast trigger probe");
    let counter_tokens = lex_line("you put one or more -1/-1 counters on a creature", 0)
        .expect("rewrite lexer should classify counter trigger probe");
    let graveyard_tokens = lex_line(
        "a nontoken creature is put into your graveyard from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify graveyard trigger probe");
    let combat_tokens = lex_line("the beginning of each combat", 0)
        .expect("rewrite lexer should classify combat trigger probe");
    let second_main_tokens = lex_line("the beginning of your second main phase", 0)
        .expect("rewrite lexer should classify second-main trigger probe");
    let gift_tokens = lex_line("an opponent gives a gift", 0)
        .expect("rewrite lexer should classify gift-given trigger probe");
    let chaos_tokens = lex_line("chaos ensues", 0)
        .expect("rewrite lexer should classify chaos-ensues trigger probe");
    let enchanted_upkeep_tokens = lex_line("the beginning of enchanted player's upkeep", 0)
        .expect("rewrite lexer should classify enchanted player's upkeep trigger probe");
    let exile_tokens = lex_line(
        "one or more cards are put into exile from graveyards and/or the battlefield during your turn",
        0,
    )
    .expect("rewrite lexer should classify exile zone-change trigger probe");
    let exile_from_hand_tokens = lex_line("one or more cards are put into exile from your hand", 0)
        .expect("rewrite lexer should classify hand-to-exile zone-change trigger probe");
    let spell_or_ability_exile_tokens = lex_line(
        "a spell or ability you control exiles one or more permanents from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify spell-or-ability exile trigger probe");
    let hand_or_spell_exile_tokens = lex_line(
        "one or more cards are put into exile from your hand or a spell or ability you control exiles one or more permanents from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify hand-or-spell exile trigger probe");
    let dealt_combat_damage_tokens = lex_line("this creature is dealt combat damage", 0)
        .expect("rewrite lexer should classify dealt-combat-damage trigger probe");
    let enters_or_transforms_tokens = lex_line(
        "this creature enters or transforms into Trystan, Callous Cultivator",
        0,
    )
    .expect("rewrite lexer should classify enter-or-transform trigger probe");
    let enters_or_graveyard_tokens = lex_line(
        "this artifact enters or is put into a graveyard from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify enter-or-graveyard trigger probe");
    let graveyard_or_exile_from_battlefield_tokens = lex_line(
        "this artifact or another nontoken artifact you control is put into a graveyard from the battlefield or is put into exile from the battlefield",
        0,
    )
    .expect("rewrite lexer should classify graveyard-or-exile-from-battlefield trigger probe");
    let transforms_tokens = lex_line("this creature transforms into Trystan, Penitent Culler", 0)
        .expect("rewrite lexer should classify standalone transforms trigger probe");
    let this_case_enters_tokens = lex_line("this Case enters", 0)
        .expect("rewrite lexer should classify case ETB trigger probe");

    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &dies_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::Dies(_))
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &dies_during_turn_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::DiesDuringTurn {
            during_turn: crate::target::PlayerFilter::You,
            ..
        })
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &upkeep_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfUpkeep(
            crate::target::PlayerFilter::You
        ))
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &enchanted_upkeep_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfUpkeep(
            crate::target::PlayerFilter::TaggedPlayer(tag)
        )) if tag.as_str() == "enchanted"
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &chaos_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ChaosEnsues,
            player: crate::target::PlayerFilter::Any,
            source_filter: None,
            during_your_turn: false,
        })
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &etb_tokens,
        ),
        Ok(
            crate::cards::builders::TriggerSpec::EntersBattlefieldOneOrMore { .. }
                | crate::cards::builders::TriggerSpec::EntersBattlefield { .. }
        )
    ));
    let this_case_enters =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &this_case_enters_tokens,
        );
    assert!(
        matches!(
            this_case_enters,
            Ok(crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface {
                surface: crate::target::SourceReferenceSurface::ThisPermanentType(ref surface),
                ..
            }) if surface == "this Case"
        ),
        "expected this Case ETB to parse as a source ETB trigger, got {this_case_enters:?}"
    );
    let enters_or_transforms =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &enters_or_transforms_tokens,
        );
    assert!(
        matches!(
            enters_or_transforms,
            Ok(crate::cards::builders::TriggerSpec::Either(ref left, ref right))
                if matches!(
                    left.as_ref(),
                    crate::cards::builders::TriggerSpec::ThisEntersBattlefield { .. }
                        | crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface { .. }
                )
                    && matches!(
                        right.as_ref(),
                        crate::cards::builders::TriggerSpec::ThisTransforms { destination_name }
                            | crate::cards::builders::TriggerSpec::ThisTransformsWithSurface { destination_name, .. }
                            if destination_name.as_deref() == Some("Trystan, Callous Cultivator")
                    )
        ),
        "expected enter-or-transform trigger pair, got {enters_or_transforms:?}"
    );
    let enters_or_graveyard =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &enters_or_graveyard_tokens,
        );
    assert!(
        matches!(
            enters_or_graveyard,
            Ok(crate::cards::builders::TriggerSpec::Either(ref left, ref right))
                if matches!(
                    left.as_ref(),
                    crate::cards::builders::TriggerSpec::ThisEntersBattlefield { .. }
                        | crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface { .. }
                )
                    && matches!(
                        right.as_ref(),
                        crate::cards::builders::TriggerSpec::PutIntoGraveyardFromZone {
                            from: crate::zone::Zone::Battlefield,
                            ..
                        }
                    )
        ),
        "expected enter-or-graveyard trigger pair, got {enters_or_graveyard:?}"
    );
    let transforms =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &transforms_tokens,
        );
    assert!(
        matches!(
            transforms,
            Ok(crate::cards::builders::TriggerSpec::ThisTransforms { ref destination_name })
                | Ok(crate::cards::builders::TriggerSpec::ThisTransformsWithSurface { ref destination_name, .. })
                    if destination_name.as_deref() == Some("Trystan, Penitent Culler")
        ),
        "expected standalone transform trigger, got {transforms:?}"
    );
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &spell_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::SpellCast { .. })
    ));
    let counter =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &counter_tokens,
        );
    assert!(
        matches!(
            counter,
            Ok(crate::cards::builders::TriggerSpec::CounterPutOn {
                one_or_more: true,
                ..
            })
        ),
        "{counter:?}"
    );
    let graveyard =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &graveyard_tokens,
        );
    assert!(
        matches!(
            graveyard,
            Ok(crate::cards::builders::TriggerSpec::PutIntoGraveyardFromZone { .. })
        ),
        "{graveyard:?}"
    );
    let graveyard_or_exile_from_battlefield =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &graveyard_or_exile_from_battlefield_tokens,
        );
    assert!(
        matches!(
            graveyard_or_exile_from_battlefield,
            Ok(crate::cards::builders::TriggerSpec::Either(ref left, ref right))
                if matches!(
                    left.as_ref(),
                    crate::cards::builders::TriggerSpec::PutIntoGraveyardFromZone {
                        from: crate::zone::Zone::Battlefield,
                        filter,
                        ..
                    } if filter.source
                        && filter.card_types == vec![crate::types::CardType::Artifact]
                        && filter.nontoken
                        && filter.controller == Some(crate::target::PlayerFilter::You)
                )
                    && matches!(
                        right.as_ref(),
                        crate::cards::builders::TriggerSpec::PutIntoExileFromZones {
                            from,
                            filter,
                            ..
                        } if *from == vec![crate::zone::Zone::Battlefield]
                            && filter.source
                            && filter.card_types == vec![crate::types::CardType::Artifact]
                            && filter.nontoken
                            && filter.controller == Some(crate::target::PlayerFilter::You)
                    )
        ),
        "expected graveyard-or-exile battlefield trigger pair, got {graveyard_or_exile_from_battlefield:?}"
    );
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &combat_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::BeginningOfCombat(
            crate::target::PlayerFilter::Any
        ))
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &second_main_tokens,
        ),
        Ok(
            crate::cards::builders::TriggerSpec::BeginningOfPostcombatMain {
                player: crate::target::PlayerFilter::You,
                ..
            }
        )
    ));
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &gift_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::PlayerGivesGift(
            crate::target::PlayerFilter::Opponent
        ))
    ));
    let exile =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &exile_tokens,
        );
    assert!(
        matches!(
            exile,
            Ok(crate::cards::builders::TriggerSpec::PutIntoExileFromZones {
                one_or_more: true,
                during_turn: Some(crate::target::PlayerFilter::You),
                ..
            })
        ),
        "{exile:?}"
    );
    let exile_from_hand =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &exile_from_hand_tokens,
        );
    assert!(
        matches!(
            exile_from_hand,
            Ok(crate::cards::builders::TriggerSpec::PutIntoExileFromZones {
                one_or_more: true,
                ref from,
                ref filter,
                ..
            }) if *from == vec![crate::zone::Zone::Hand]
                && filter.owner == Some(crate::target::PlayerFilter::You)
        ),
        "{exile_from_hand:?}"
    );
    let spell_or_ability_exile =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &spell_or_ability_exile_tokens,
        );
    assert!(
        matches!(
            spell_or_ability_exile,
            Ok(crate::cards::builders::TriggerSpec::PutIntoExileFromZones {
                one_or_more: true,
                ref from,
                ref filter,
                cause_filter: Some(ref cause_filter),
                ..
            }) if *from == vec![crate::zone::Zone::Battlefield]
                && filter.card_types == crate::target::ObjectFilter::permanent_card().card_types
                && matches!(
                    &cause_filter.cause_type,
                    Some(crate::events::cause::CauseTypeFilter::EffectLike)
                )
                && matches!(
                    &cause_filter.controller_filter,
                    Some(crate::events::cause::ControllerFilter::ContextController)
                )
        ),
        "{spell_or_ability_exile:?}"
    );
    let hand_or_spell_exile =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &hand_or_spell_exile_tokens,
        );
    assert!(
        matches!(
            hand_or_spell_exile,
            Ok(crate::cards::builders::TriggerSpec::Either(ref left, ref right))
                if matches!(
                    left.as_ref(),
                    crate::cards::builders::TriggerSpec::PutIntoExileFromZones {
                        one_or_more: true,
                        from,
                        filter,
                        cause_filter: None,
                        ..
                    } if *from == vec![crate::zone::Zone::Hand]
                        && filter.owner == Some(crate::target::PlayerFilter::You)
                )
                    && matches!(
                        right.as_ref(),
                        crate::cards::builders::TriggerSpec::PutIntoExileFromZones {
                            one_or_more: true,
                            from,
                            filter,
                            cause_filter: Some(cause_filter),
                            ..
                        } if *from == vec![crate::zone::Zone::Battlefield]
                            && filter.card_types == crate::target::ObjectFilter::permanent_card().card_types
                            && matches!(
                                &cause_filter.controller_filter,
                                Some(crate::events::cause::ControllerFilter::ContextController)
                            )
                    )
        ),
        "{hand_or_spell_exile:?}"
    );
    assert!(matches!(
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &dealt_combat_damage_tokens,
        ),
        Ok(crate::cards::builders::TriggerSpec::ThisIsDealtCombatDamage)
    ));
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_resolves_double_slash_source_name_etb() {
    let tokens = lex_line("When SP//dr enters", 0)
        .expect("rewrite lexer should classify double-slash source trigger");

    let context = crate::parse_context::ParseContext::for_fragment(
        "SP//dr, Piloted by Peni",
        vec![CardType::Artifact, CardType::Creature],
        vec![],
        "When SP//dr enters",
    );
    let parsed = super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
        context.view(),
        &tokens,
    );

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::TriggerSpec::WithIntro {
                intro: crate::model::ast::TriggerIntroSurfaceAst::When,
                ref trigger,
            }) if matches!(
                trigger.as_ref(),
                crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface {
                    surface: crate::target::SourceReferenceSurface::ShortName(surface),
                    ..
                } if surface == "SP//dr"
            )
        ),
        "expected SP//dr source ETB trigger with surface, got {parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_resolves_double_slash_source_name_etb() {
    let tokens = lex_line(
        "When SP//dr enters, put a +1/+1 counter on target creature.",
        0,
    )
    .expect("rewrite lexer should classify double-slash source triggered line");

    let context = crate::parse_context::ParseContext::for_fragment(
        "SP//dr, Piloted by Peni",
        vec![CardType::Artifact, CardType::Creature],
        vec![],
        "When SP//dr enters, put a +1/+1 counter on target creature.",
    );
    let parsed = super::super::clause_support::parse_triggered_line_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("SP//dr triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("ThisEntersBattlefieldWithSurface") && debug.contains("SP//dr"),
        "expected SP//dr source ETB trigger with surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_document_lowering_resolves_double_slash_source_name_etb()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "SP//dr, Piloted by Peni")
        .card_types(vec![CardType::Artifact, CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "When SP//dr enters, put a +1/+1 counter on target creature.".to_string(),
        false,
    )?;
    let debug = format!("{definition:#?}");

    assert!(
        debug.contains("this_surface") && debug.contains("SP//dr"),
        "expected document lowering to preserve SP//dr source ETB surface, got {debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_exile_multi_target_preserves_short_source_surface()
-> Result<(), CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::new(), "Mangara of Corondor")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature]);
    let (definition, _) = parse_text_with_annotations_lowered(
        builder,
        "{T}: Exile Mangara and target permanent.".to_string(),
        false,
    )?;
    let debug = format!("{definition:?}");

    assert!(
        debug.contains("SourceReference(ShortName(\"Mangara\"))"),
        "expected short-name source surface on source exile, got {debug}"
    );
    Ok(())
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_player_contraction_dealt_damage_trigger() {
    let text = "Whenever you're dealt damage, put that many vitality counters on this Aura.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify player dealt-damage trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("player contraction dealt-damage triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DealsDamageToPlayer"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("vitality"), "{debug}");
    assert!(
        debug.contains("EventValue") && debug.contains("Amount"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_generic_damage_to_object_trigger() {
    let text = "Whenever equipped creature deals damage to a blocking creature, draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify damage-to-object trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("damage-to-object triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DealsDamageTo"), "{debug}");
    assert!(debug.contains("source"), "{debug}");
    assert!(debug.contains("target"), "{debug}");
    assert!(debug.contains("blocking: true"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_damage_to_object_trigger_preserves_generic_source_surface() {
    let text = "Whenever a source deals damage to this creature, draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify source damage trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("generic-source damage-to-object line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DealsDamageTo"), "{debug}");
    assert!(debug.contains("source_surface: Source"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_damage_to_player_trigger_preserves_generic_source_surface() {
    let text = "Whenever a source an opponent controls deals damage to you, draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify source damage trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("generic-source damage-to-player line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DealsDamageToPlayer"), "{debug}");
    assert!(debug.contains("source_surface: Source"), "{debug}");
    assert!(debug.contains("zone: None"), "{debug}");
    assert!(
        debug.contains("controller: Some(\n                Opponent"),
        "{debug}"
    );
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_recipientless_damage_trigger_preserves_generic_source_surface() {
    let text = "Whenever a noncreature source you control deals damage, you gain that much life.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify source damage trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("recipientless generic-source damage line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("DealsDamage"), "{debug}");
    assert!(debug.contains("source_surface: Source"), "{debug}");
    assert!(debug.contains("zone: None"), "{debug}");
    assert!(
        debug.contains("controller: Some(\n                You"),
        "{debug}"
    );
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("GainLife"), "{debug}");
}

#[test]
pub(super) fn rewrite_result_gated_consult_preserves_sacrificed_card_type_relation() {
    let text = "If the player does, they reveal cards from the top of their library until they reveal a permanent card that shares a card type with the sacrificed permanent, put that card onto the battlefield, then shuffle.";
    let tokens = lex_line(text, 0).expect("result-gated consult should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("result-gated consult should parse through typed bundle grammar");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("SharesCardType"), "{debug}");
    assert!(debug.contains("sacrificed_0"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_spell_countered_trigger() {
    let text = "Whenever a spell you've cast is countered, draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify spell-countered trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("spell-countered triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("SpellCountered"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
    assert!(!debug.contains("SpellCast"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_ketramose_exile_trigger() {
    let text = "Whenever one or more cards are put into exile from graveyards and/or the battlefield during your turn, you draw a card and lose 1 life.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify exile zone-change triggered line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("exile zone-change triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("PutIntoExileFromZones"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("Battlefield"), "{debug}");
    assert!(debug.contains("during_turn: Some"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("LoseLife"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_stonebinders_familiar_trigger() {
    let text = "Whenever one or more cards are put into exile during your turn, put a +1/+1 counter on this creature. This ability triggers only once each turn.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify Stonebinder's Familiar triggered line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("Stonebinder's Familiar triggered line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("PutIntoExileFromZones"), "{debug}");
    assert!(debug.contains("one_or_more: true"), "{debug}");
    assert!(debug.contains("during_turn: Some"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
    assert!(debug.contains("nontoken: true"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert!(debug.contains("max_triggers_per_turn: Some"), "{debug}");
}

#[test]
pub(super) fn parse_trigger_clause_supports_you_discard_one_or_more_cards() {
    let tokens =
        lex_line("you discard one or more cards", 0).expect("discard trigger clause should lex");
    let parsed =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("discard one-or-more trigger clause should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("PlayerDiscardsCard"), "{debug}");
    assert!(debug.contains("one_or_more: true"), "{debug}");
}

#[test]
pub(super) fn parse_effect_sentence_supports_target_pump_for_each_card_discarded_this_way() {
    let tokens = lex_line(
        "target creature gets +2/+0 until end of turn for each card discarded this way",
        0,
    )
    .expect("discarded-this-way pump clause should lex");
    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("discarded-this-way pump clause should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("PumpForEach"), "{debug}");
    assert!(
        debug.contains("EventValue") && debug.contains("Amount"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_handles_punctuation_before_enter_verb() {
    let text = "Whenever one or more noncreature, nonland permanents you control enter, put a +1/+1 counter on target creature you control.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify comma-separated enter trigger line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_state_trigger_condition() {
    let text = "When you control no Swamps, sacrifice this creature.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify state trigger line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("state-triggered line should parse");

    match parsed {
        crate::cards::builders::LineAst::Triggered { trigger, .. } => {
            assert!(
                matches!(
                    trigger,
                    crate::cards::builders::TriggerSpec::StateBased { .. }
                ),
                "expected state trigger, got {trigger:?}"
            );
        }
        other => panic!("expected triggered line, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_named_counter_threshold_state_trigger() {
    let text = "Whenever there are four or more tide counters on this enchantment, remove all tide counters from it.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify counter state trigger line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("named-counter state-triggered line should parse");

    match parsed {
        crate::cards::builders::LineAst::Triggered {
            trigger, effects, ..
        } => {
            let trigger_debug = format!("{trigger:?}");
            let effects_debug = format!("{effects:?}");
            assert!(
                matches!(
                    trigger,
                    crate::cards::builders::TriggerSpec::StateBased { .. }
                ),
                "expected named-counter state trigger, got {trigger_debug}"
            );
            assert!(
                trigger_debug.contains("SourceHasCounterAtLeast")
                    && trigger_debug.contains("tide")
                    && trigger_debug.contains("count: 4"),
                "expected four-or-more tide counter predicate, got {trigger_debug}"
            );
            assert!(
                effects_debug.contains("RemoveUpToAnyCounters") && effects_debug.contains("tide"),
                "expected remove-tide-counters effect, got {effects_debug}"
            );
        }
        other => panic!("expected triggered line, got {other:?}"),
    }
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_parses_remove_all_named_counters_from_target() {
    let text = "remove all charge counters from target artifact";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify remove-all-counters effect");

    let parsed = parse_effect_sentence_lexed(&tokens)
        .expect("remove-all-counters from target effect should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("RemoveUpToAnyCounters")
            && debug.contains("Charge")
            && debug.contains("CountersOn(")
            && debug.contains("card_types: [Artifact]")
            && debug.contains("Some(TextSpan"),
        "expected generic remove-all named counters from target, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_entrypoint_matches_wrapper_comma_then_chain() {
    let text = "Discard your hand, then draw four cards.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma-then effect");
    let compat = crate::util::tokenize_line(text, 0);

    let wrapper = super::super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper effect sentence parser should succeed");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed effect sentence parser should succeed");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_matches_wrapper_conditional_dispatch() {
    let text = "If you control an artifact, draw a card.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify conditional sentence");
    let compat = crate::util::tokenize_line(text, 0);

    let wrapper = super::super::clause_support::parse_effect_sentences_lexed(&compat)
        .expect("wrapper conditional sentence should parse");
    let native = super::super::clause_support::parse_effect_sentences_lexed(&lexed)
        .expect("lexed conditional sentence should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_matches_wrapper_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");
    let compat = crate::util::tokenize_line(text, 0);

    let native = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
        .expect("lexed predicate should parse");
    let wrapper = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&compat)
        .expect("wrapper predicate should parse");

    assert_eq!(format!("{native:?}"), format!("{wrapper:?}"));
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_matches_wrapper_pre_diagnostic_clause_helpers() {
    for text in [
        "The next time a red source of your choice would deal damage to you this turn, prevent that damage.",
        "Double target creature's power until end of turn.",
    ] {
        let lexed = lex_line(text, 0).expect("rewrite lexer should classify clause helper probe");
        let compat = crate::util::tokenize_line(text, 0);
        let native =
            parse_effect_sentence_lexed(&lexed).expect("lexed clause helper sentence should parse");
        let wrapper = parse_effect_sentence_lexed(&compat)
            .expect("wrapper clause helper sentence should parse");

        assert_eq!(format!("{native:?}"), format!("{wrapper:?}"), "{text}");
    }
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_lifts_intervening_if_simple_clause_after_structure_cutover()
 {
    let text = "At the beginning of your upkeep, if you control an artifact, draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify upkeep intervening-if");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("triggered intervening-if line should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_source_crew_count_intervening_if() {
    let text = "Whenever this Vehicle becomes crewed for the first time each turn, if it was crewed by exactly two creatures, it gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify crew-count trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("crew-count intervening-if line should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("KeywordActionTaggedObject") && debug.contains("Crew"),
        "{debug}"
    );
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("SourceCrewedByExactly"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("TagKey(\"__it__\")"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_source_or_another_turned_face_up_subject() {
    let text = "Whenever this creature or another creature you control is turned face up, put +1/+1 counters on that creature equal to its power.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify turned-face-up trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("source-or-another turned-face-up trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("TurnedFaceUp"), "{debug}");
    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("source: true"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
}

#[test]
pub(super) fn rewrite_effect_sentences_parse_pronoun_quoted_trigger_grant() {
    let text = "it gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify quoted trigger grant");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("quoted trigger grant effect should parse from sentence dispatch");
    let debug = format!("{effects:?}");

    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("TagKey(\"__it__\")"), "{debug}");
    assert!(debug.contains("ThisDealsCombatDamageToPlayer"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_lifts_intervening_if_with_multisentence_body() {
    let text = "At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify postcombat intervening-if trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("intervening-if trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("BeginningOfPostcombatMain"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_keeps_source_exiled_move_then_damage_body() {
    let text = "At the beginning of your end step, if there are cards exiled with this enchantment, put them into their owner's graveyard, then this enchantment deals that much damage to each opponent.";
    let tokens =
        lex_line(text, 0).expect("rewrite lexer should classify source-exiled end-step trigger");

    let (parsed, trace) = crate::parse_trace::capture(|| {
        super::super::clause_support::parse_triggered_line_lexed(&tokens)
            .expect("source-exiled move-then-damage trigger should parse")
    });
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("ValueComparison"),
        "{debug}\n{}",
        trace.render()
    );
    assert!(debug.contains("MoveToZone"), "{debug}\n{}", trace.render());
    assert!(debug.contains("DealDamage"), "{debug}\n{}", trace.render());
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_parses_double_sweep_body() {
    let tokens = lex_line(
        "At the beginning of each combat, double the power and toughness of each creature you control until end of turn.",
        0,
    )
    .expect("rewrite lexer should classify double sweep trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("double sweep trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_parses_double_sweep_body() {
    let tokens = lex_line(
        "double the power and toughness of each creature you control until end of turn",
        0,
    )
    .expect("rewrite lexer should classify double sweep effect");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("double sweep effect should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ScalePowerToughnessAll"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_parses_triple_target_pt_body() {
    let tokens = lex_line(
        "triple target creature's power and toughness until end of turn",
        0,
    )
    .expect("rewrite lexer should classify triple target pt effect");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("triple target pt effect should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("Scaled"), "{debug}");
    assert!(debug.contains("PowerOf"), "{debug}");
    assert!(debug.contains("ToughnessOf"), "{debug}");
}

#[test]
pub(super) fn rewrite_effect_entrypoint_parses_cast_from_zone_copy_sequence_generically() {
    let tokens = lex_line(
        "If this spell was cast from a graveyard, copy this spell and you may choose a new target for the copy.",
        0,
    )
    .expect("rewrite lexer should classify cast-from-zone copy effect");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("cast-from-zone copy effect should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("ThisSpellWasCastFromZone"), "{debug}");
    assert!(debug.contains("CopySpell"), "{debug}");
}

#[test]
pub(super) fn rewrite_effect_entrypoint_parses_devotion_library_threshold_sequence_generically() {
    let tokens = lex_line(
        "Look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random order. If X is greater than or equal to the number of cards in your library, you win the game.",
        0,
    )
    .expect("rewrite lexer should classify the devotion library-threshold sequence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&tokens)
        .expect("devotion library-threshold sequence should parse");
    let debug = format!("{parsed:?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("Devotion"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(debug.contains("ValueComparison"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual"), "{debug}");
    assert!(debug.contains("WinGame"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_matches_grammar_entrypoint_output() {
    let text = "it's your turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify predicate text");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed predicate should parse");
    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");

    assert_eq!(format!("{parser_root:?}"), format!("{grammar:?}"));
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_handles_color_contraction() {
    let text = "it's blue";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify color predicate text");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed predicate should parse");
    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");
    let debug = format!("{parser_root:?}");

    assert_eq!(debug, format!("{grammar:?}"));
    assert!(
        debug.contains("ItMatches"),
        "expected object-match predicate, got {debug}"
    );
    assert!(
        debug.contains("colors: Some("),
        "expected blue color constraint in predicate, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_splits_both_spell_cast_conditions() {
    let text = "you've cast both a creature spell and a noncreature spell this turn";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify spell-cast predicate");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed predicate should parse");
    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");
    let debug = format!("{parser_root:?}");

    assert_eq!(debug, format!("{grammar:?}"));
    assert!(
        debug.contains("And("),
        "expected conjoined spell-cast predicates, got {debug}"
    );
    assert!(
        debug.contains("card_types: [Creature]")
            && debug.contains("excluded_card_types: [Creature]"),
        "expected separate creature and noncreature spell filters, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_matches_grammar_entrypoint_for_this_spell_cast_from_zone()
 {
    let text = "this spell was cast from a graveyard";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify graveyard-cast predicate");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed predicate should parse");
    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");

    assert_eq!(format!("{parser_root:?}"), format!("{grammar:?}"));
    assert!(
        format!("{parser_root:?}").contains("ThisSpellWasCastFromZone(Graveyard)"),
        "expected graveyard-cast predicate AST, got {parser_root:?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_handles_exiled_source_with_named_counter() {
    let text = "this card is exiled with a scream counter on it";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify exiled counter predicate");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed predicate should parse");
    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse");
    let debug = format!("{parser_root:?}");

    assert_eq!(debug, format!("{grammar:?}"));
    assert!(
        debug.contains("And("),
        "expected conjoined predicate, got {debug}"
    );
    assert!(
        debug.contains("SourceIsInZone(Exile)"),
        "expected exile zone predicate, got {debug}"
    );
    assert!(
        debug.contains("SourceHasCounterAtLeast") && debug.contains("scream"),
        "expected named scream counter threshold, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_handles_no_more_named_counters_on_it() {
    let text = "there are no more scream counters on it";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify no-counter predicate");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed predicate should parse");
    let debug = format!("{parser_root:?}");

    assert!(
        debug.contains("SourceHasNoCounter") && debug.contains("scream"),
        "expected named scream no-counter predicate, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_predicate_parser_handles_named_counter_threshold_on_source() {
    let text = "there are four or more tide counters on this enchantment";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify named-counter threshold predicate");

    let parser_root =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("lexed named-counter threshold predicate should parse");
    let grammar =
        super::super::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&lexed)
            .expect("grammar predicate entrypoint should parse named-counter threshold");
    let debug = format!("{parser_root:?}");

    assert_eq!(debug, format!("{grammar:?}"));
    assert!(
        debug.contains("SourceHasCounterAtLeast")
            && debug.contains("tide")
            && debug.contains("count: 4"),
        "expected four-or-more tide counter predicate, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_keeps_where_x_trailing_clause_after_dispatch_inner_cutover()
 {
    let text = "Target creature gets +X/+0 until end of turn, where X is its power; target creature gains flying until end of turn.";
    let lexed = lex_line(text, 0)
        .expect("rewrite lexer should classify where-x sentence with trailing clause");

    let parsed =
        parse_effect_sentence_lexed(&lexed).expect("lexed where-x trailing sentence should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("PowerOf") && debug.contains("WhereXIs"),
        "{debug}"
    );
    assert!(
        !debug.contains("SourcePower"),
        "the possessive refers to the targeted creature, not the spell source: {debug}"
    );
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
}

#[test]
pub(super) fn rewrite_dispatch_inner_split_choose_list_routes_separator_helpers() {
    let tokens = lex_line("an artifact, creature, and enchantment", 0)
        .expect("rewrite lexer should classify choose-list separators");

    let segments: Vec<Vec<String>> = crate::effect_sentences::split_choose_list(&tokens)
        .into_iter()
        .map(|segment| {
            super::super::token_word_refs(&segment)
                .into_iter()
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    assert_eq!(
        segments,
        vec![
            vec!["an".to_string(), "artifact".to_string()],
            vec!["creature".to_string()],
            vec!["enchantment".to_string()],
        ]
    );
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_supports_this_creature_leaves_battlefield() {
    let tokens = lex_line("this creature leaves the battlefield", 0)
        .expect("rewrite lexer should classify leaves-the-battlefield trigger");

    let parsed =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("lexed leaves-the-battlefield trigger should parse");

    let debug = format!("{parsed:?}");
    assert!(
        debug.contains("ThisLeavesBattlefieldWithSurface") && debug.contains("this creature"),
        "expected this-creature leaves trigger to preserve surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_preserves_this_aura_leaves_surface() {
    let tokens = lex_line("this Aura leaves the battlefield", 0)
        .expect("rewrite lexer should classify aura leaves-the-battlefield trigger");

    let parsed =
        super::super::activation_and_restrictions::trigger_clause_core::parse_trigger_clause_lexed(
            &tokens,
        )
        .expect("aura leaves-the-battlefield trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("ThisLeavesBattlefieldWithSurface") && debug.contains("this aura"),
        "expected this-aura leaves trigger to preserve surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_trigger_clause_preserves_named_source_leaves_surface() {
    let text = "emrakul leaves the battlefield";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify named leaves-the-battlefield trigger");
    let context = crate::parse_context::ParseContext::for_fragment(
        "Emrakul, the World Anew",
        vec![CardType::Creature],
        vec![],
        text,
    );

    let parsed =
        super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
            context.view(),
            &tokens,
        )
        .expect("named source leaves-the-battlefield trigger should parse");
    let debug = format!("{parsed:?}");

    assert!(
        debug.contains("ThisLeavesBattlefieldWithSurface")
            && debug.contains("ShortName(\"Emrakul\")"),
        "expected named source leaves trigger to preserve surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_supports_leave_battlefield_sacrifice_land() {
    let tokens = lex_line("When this leaves the battlefield, sacrifice a land.", 0)
        .expect("rewrite lexer should classify leave-battlefield sacrifice line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_preserves_named_source_leaves_surface() {
    let text = "When emrakul leaves the battlefield, sacrifice all creatures you control.";
    let tokens = lex_line(text, 0)
        .expect("rewrite lexer should classify named leave-battlefield sacrifice line");
    let context = crate::parse_context::ParseContext::for_fragment(
        "Emrakul, the World Anew",
        vec![CardType::Creature],
        vec![],
        text,
    );

    let parsed = super::super::clause_support::parse_triggered_line_lexed_with_context(
        context.view(),
        &tokens,
    )
    .expect("named source leaves trigger line should parse");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("ThisLeavesBattlefieldWithSurface")
            && debug.contains("ShortName")
            && debug.contains("\"Emrakul\""),
        "expected named source leaves trigger line to preserve surface, got {debug}"
    );
}

#[test]
pub(super) fn compile_named_source_leaves_trigger_preserves_surface() {
    let (semantic, _) = parse_text_to_semantic_document(
        CardBuilder::new(CardId::from_raw(1), "Emrakul, the World Anew")
            .card_types(vec![CardType::Creature]),
        "When Emrakul leaves the battlefield, sacrifice all creatures you control.".to_string(),
        false,
    )
    .expect("named source leaves trigger should parse semantically");
    let semantic_debug = format!("{semantic:#?}");
    assert!(
        semantic_debug.contains("ThisLeavesBattlefieldWithSurface")
            && semantic_debug.contains("ShortName")
            && semantic_debug.contains("\"Emrakul\""),
        "expected semantic document to carry the typed named-source trigger, got {semantic_debug}"
    );

    let compiled = super::super::compile_card_text(
        CardDefinitionBuilder::new(CardId::from_raw(1), "Emrakul, the World Anew")
            .card_types(vec![CardType::Creature]),
        "When Emrakul leaves the battlefield, sacrifice all creatures you control.",
        false,
    )
    .expect("named source leaves trigger should compile");
    let debug = format!("{:#?}", compiled.definition.abilities);

    assert!(
        debug.contains("this_surface") && debug.contains("\"Emrakul\""),
        "expected compiled trigger model to preserve named source surface, got {debug}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_supports_leave_battlefield_sacrifice_all_non_ogres() {
    let tokens = lex_line(
        "When this creature leaves the battlefield, sacrifice all non-Ogre creatures you control.",
        0,
    )
    .expect("rewrite lexer should classify leave-battlefield sacrifice-all line");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens);

    assert!(
        matches!(
            parsed,
            Ok(crate::cards::builders::LineAst::Triggered { .. })
        ),
        "{parsed:?}"
    );
}

#[test]
pub(super) fn rewrite_lexed_triggered_line_supports_this_or_another_leaves_battlefield() {
    let tokens = lex_line(
        "Whenever this creature or another creature you control leaves the battlefield, each opponent loses 1 life.",
        0,
    )
    .expect("rewrite lexer should classify source-or-another leaves trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("source-or-another leaves trigger should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ThisLeavesBattlefield"), "{debug}");
    assert!(debug.contains("LeavesBattlefield"), "{debug}");
    assert!(debug.contains("LoseLife"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_swindlers_scheme_trigger_keeps_opponent_hand_reveal_and_followup_cast()
{
    let text = "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If it shares a card type with that spell, counter that spell and that opponent may cast the revealed card without paying its mana cost.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Swindler's Scheme");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("Swindler's Scheme trigger line should parse");
    let crate::cards::builders::LineAst::Triggered { trigger, .. } = &parsed else {
        panic!("expected Swindler's Scheme to remain a triggered line: {parsed:#?}");
    };
    let crate::cards::builders::TriggerSpec::SpellCast {
        filter: Some(filter),
        caster,
        ..
    } = trigger
    else {
        panic!("expected a typed spell-cast trigger: {trigger:#?}");
    };
    assert_eq!(caster, &PlayerFilter::Opponent);
    assert_eq!(filter.zone, Some(Zone::Hand), "{filter:#?}");
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::Opponent),
        "the cast-origin owner should mirror the typed opponent caster: {filter:#?}"
    );
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("SpellCast"), "{debug}");
    assert!(debug.contains("caster: Opponent"), "{debug}");
    assert!(debug.contains("from_not_hand: false"), "{debug}");
    assert!(debug.contains("RevealTop"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("SharesCardType"), "{debug}");
    assert!(debug.contains("Counter"), "{debug}");
    assert!(debug.contains("MayByPlayer"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_gandalf_trigger_keeps_each_opponent_reveal_in_effect_body() {
    let text = "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify Gandalf trigger");

    let parsed = super::super::clause_support::parse_triggered_line_lexed(&tokens)
        .expect("Gandalf trigger line should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("SpellCast"), "{debug}");
    assert!(debug.contains("ForEachOpponent"), "{debug}");
    assert!(debug.contains("RevealTop"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
    assert!(debug.contains("SharesCardType"), "{debug}");
    assert!(debug.contains("CopySpell"), "{debug}");
    assert!(debug.contains("if_false"), "{debug}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_labeled_spent_to_cast_conditional() {
    let text =
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify labeled spent-to-cast sentence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
pub(super) fn rewrite_grammar_conditional_family_head_parser_strips_labeled_and_then_if_prefixes() {
    let labeled = lex_line(
        "Adamant — If at least three blue mana was spent to cast this spell, create a Food token.",
        0,
    )
    .expect("rewrite lexer should classify labeled conditional sentence");
    let labeled_stripped =
        super::super::grammar::effects::split_conditional_sentence_family_head_lexed(&labeled)
            .expect("labeled conditional family head should strip to the if clause");

    assert_eq!(
        super::super::token_word_refs(labeled_stripped)
            .first()
            .map(|word| word.to_ascii_lowercase())
            .as_deref(),
        Some("if")
    );

    let then_if = lex_line("Then if it's blue, create a Treasure token.", 0)
        .expect("rewrite lexer should classify then-if conditional sentence");
    let then_if_stripped =
        super::super::grammar::effects::split_conditional_sentence_family_head_lexed(&then_if)
            .expect("then-if conditional family head should strip to the if clause");

    assert_eq!(
        super::super::token_word_refs(then_if_stripped)
            .into_iter()
            .map(|word| word.to_ascii_lowercase())
            .collect::<Vec<_>>(),
        vec!["if", "it's", "blue", "create", "a", "treasure", "token"]
    );
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_supports_unlabeled_spent_to_cast_conditional() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify unlabeled spent-to-cast sentence");

    let parsed = super::super::clause_support::parse_effect_sentences_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
pub(super) fn rewrite_lexed_effect_sentence_routes_then_if_conditional_through_grammar_family() {
    let text = "Then if it's blue, create a Treasure token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify then-if sentence");

    let grammar = super::super::grammar::effects::parse_conditional_sentence_family_lexed(
        &lexed,
        super::super::effect_sentences::parse_effect_chain_lexed,
    )
    .expect("grammar conditional family parser should succeed")
    .expect("then-if conditional family should be recognized");
    let parsed = super::super::effect_sentences::parse_effect_sentence_lexed(&lexed)
        .expect("effect sentence parser should route then-if through grammar family");

    assert_eq!(format!("{parsed:?}"), format!("{grammar:?}"));
    assert!(matches!(
        parsed.as_slice(),
        [crate::cards::builders::EffectAst::Conditional { .. }]
    ));
}

#[test]
pub(super) fn rewrite_lexed_conditional_parser_supports_spent_to_cast_conditional_directly() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed =
        lex_line(text, 0).expect("rewrite lexer should classify unlabeled spent-to-cast sentence");

    let parsed = super::super::effect_sentences::parse_conditional_sentence_lexed(&lexed);

    assert!(parsed.is_ok(), "{parsed:?}");
}

#[test]
pub(super) fn rewrite_lexed_conditional_parser_routes_comma_clause_through_structure_splitter() {
    let text = "If at least three blue mana was spent to cast this spell, create a Food token.";
    let lexed = lex_line(text, 0).expect("rewrite lexer should classify comma if clause");

    let parsed = super::super::effect_sentences::parse_conditional_sentence_lexed(&lexed)
        .expect("comma if clause should parse");

    match parsed.as_slice() {
        [
            crate::cards::builders::EffectAst::Conditional {
                predicate: _,
                if_true,
                if_false,
            },
        ] => {
            assert!(if_false.is_empty());
            assert!(matches!(
                if_true.as_slice(),
                [crate::cards::builders::EffectAst::SubjectVerb(
                    crate::cards::builders::SubjectVerbEffectAst {
                        action: crate::cards::builders::SubjectVerbActionAst::CreateTokenWithMods { .. },
                        ..
                    }
                )]
            ));
        }
        other => panic!("expected conditional comma if clause, got {other:?}"),
    }
}

#[test]
pub(super) fn activated_cost_objects_seed_typed_resolution_references() {
    let tapped = CardDefinitionBuilder::new(CardId::new(), "Gateway Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Tap two untapped creatures you control: You may put a creature card from your hand that shares a creature type with each creature tapped this way onto the battlefield.",
        )
        .expect("tap-cost relation should parse");
    let tapped_debug = format!("{tapped:#?}");
    assert!(tapped_debug.contains("tap_cost_0"), "{tapped_debug}");
    assert!(
        tapped_debug.contains("SharesSubtypeWithEachTagged"),
        "{tapped_debug}"
    );
    let activated = tapped
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Gateway Probe should have an activated ability");
    let effects = activated.effects.flattened_default_effects();
    let choose_hand_card = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::ChooseObjectsEffect>(effect))
        .expect("Gateway Probe should choose the matching hand card");
    let filter = &choose_hand_card.filter;
    assert_eq!(filter.zone, Some(Zone::Hand), "{filter:#?}");
    assert_eq!(filter.owner, Some(crate::PlayerFilter::You), "{filter:#?}");
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SharesSubtypeWithEachTagged
        }),
        "{filter:#?}"
    );
    assert!(
        filter.tagged_constraints.iter().all(|constraint| {
            constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }),
        "the hand card must not also be one of the creatures tapped for the cost: {filter:#?}"
    );

    let returned = CardDefinitionBuilder::new(CardId::new(), "Sage Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}, Return a land you control to its owner's hand: Draw a card. Then discard a card unless that land had a nonbasic land type.",
        )
        .expect("return-cost LKI condition should parse");
    let returned_debug = format!("{returned:#?}");
    assert!(returned_debug.contains("return_cost_0"), "{returned_debug}");
    assert!(
        returned_debug.contains("TaggedObjectMatchedLastKnown"),
        "{returned_debug}"
    );
    assert!(
        returned_debug.contains("has_nonbasic_land_type: true"),
        "{returned_debug}"
    );

    let self_cost = CardDefinitionBuilder::new(CardId::new(), "Self Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{2}, {T}, Sacrifice this creature: You gain life equal to its power.")
        .expect("self-sacrifice LKI value should parse");
    let self_debug = format!("{self_cost:#?}");
    assert!(self_debug.contains("PowerOf"), "{self_debug}");
    assert!(!self_debug.contains("zone: Some(Hand)"), "{self_debug}");

    let discarded = CardDefinitionBuilder::new(CardId::new(), "Discard Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{B}, Discard a creature card: This creature gets +X/+X until end of turn, where X is the discarded card's mana value.",
        )
        .expect("discard-cost mana value should parse");
    let discarded_debug = format!("{discarded:#?}");
    assert!(
        discarded_debug.contains("discarded_cost"),
        "{discarded_debug}"
    );
    assert!(discarded_debug.contains("ManaValueOf"), "{discarded_debug}");
}
