#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_02::*;
use super::*;

#[cfg(ironsmith_runtime_removed_parser_helper_unit_tests)]
#[test]
fn parse_object_filter_rejects_controller_only_phrase() {
    let tokens = tokenize_line("you control", 0);
    let result = parse_object_filter(&tokens, false);
    assert!(
        result.is_err(),
        "controller-only phrase should not be treated as a valid object target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_set_life_total_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blessed Wind")
        .parse_text("Target player's life total becomes 20.")
        .expect("parse set life total");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<SetLifeTotalEffect>().is_some()),
        "should include set life total effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_discard_random_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Specter's Wail")
        .parse_text("Target player discards a card at random.")
        .expect("parse random discard");

    let effects = def.spell_effect.expect("spell effect");
    let discard = effects
        .iter()
        .find_map(|e| e.downcast_ref::<DiscardEffect>())
        .expect("should include discard effect");
    assert!(discard.random, "discard should be random");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_discard_it_after_reveal_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Faadiyah Variant")
        .parse_text("{T}: Draw a card and reveal it. If it isn't a land card, discard it.")
        .expect("discard-it clause should parse");

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("DiscardEffect")
            && debug.contains("card_filter: Some")
            && debug.contains("tagged_constraints: [TaggedObjectConstraint")
            && debug.contains("zone: Some(Hand)"),
        "expected discard-it lowering to a tagged hand-card discard filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_opponent_creature_that_was_dealt_damage_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Manticore Variant")
        .parse_text("Destroy target creature an opponent controls that was dealt damage this turn.")
        .expect("combat-history destroy filter should parse");

    let debug = format!("{:?}", def.spell_effect.expect("spell effect"));
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert!(
        debug.contains("was_dealt_damage_this_turn: true"),
        "expected dealt-damage-this-turn filter, got {debug}"
    );
    assert!(
        debug.contains("controller: Some(Opponent)"),
        "expected opponent-control filter on destroy target, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_damage_targets_that_were_dealt_damage_this_turn() {
    for (name, text) in [
        (
            "Crushing Pain",
            "Crushing Pain deals 6 damage to target creature that was dealt damage this turn.",
        ),
        (
            "Inflame",
            "Inflame deals 2 damage to each creature dealt damage this turn.",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .parse_text(text)
            .expect("damage-history target should parse");
        let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effect"));
        assert!(
            debug.contains("was_dealt_damage_this_turn: true"),
            "expected dealt-damage-this-turn filter for {name}, got {debug}"
        );
        let rendered = unprocessed_compiled_lines(&def).join("\n");
        assert!(
            rendered.contains("dealt damage this turn"),
            "expected duration-bearing damage-history surface for {name}, got {rendered}"
        );
        if name == "Inflame" {
            assert!(
                rendered.contains("each creature dealt damage this turn")
                    && !rendered.contains("each creature that was dealt damage this turn"),
                "expected compact mass-damage history surface for Inflame, got {rendered}"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_copy_duration_before_unmodeled_exception_tail() {
    for (name, text) in [
        (
            "Impossible Man",
            "{2}{U}: Impossible Man becomes a copy of another target permanent until end of turn, except his name is Impossible Man.",
        ),
        (
            "Hall of Mirrors",
            "Choose target creature you control. Each other creature you control becomes a copy of that creature until end of turn, except it isn't legendary.",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .parse_text(text)
            .expect("copy duration before exception should parse");
        let debug = format!("{def:?}");
        assert!(
            debug.contains("CopyOf"),
            "expected copy effect for {name}, got {debug}"
        );
        assert!(
            debug.contains("until: EndOfTurn"),
            "expected end-of-turn copy duration for {name}, got {debug}"
        );
        let rendered = unprocessed_compiled_lines(&def).join("\n");
        assert!(
            rendered.contains("until end of turn"),
            "expected duration-bearing copy surface for {name}, got {rendered}"
        );
        if name == "Hall of Mirrors" {
            assert!(debug.contains("RemoveSupertypes"), "{debug}");
            assert!(
                rendered.contains("except it isn't legendary") && !rendered.contains("For each"),
                "expected one plural copy clause with its nonlegendary exception, got {rendered}"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_composite_copy_exception_characteristics() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hulkling, Young Avenger")
        .parse_text(
            "Whenever you cast a noncreature spell, Hulkling becomes a copy of up to one other target creature until end of turn, except his name is Hulkling, Young Avenger, he's 4/4, and he has flying and this ability.",
        )
        .expect("composite copy exception should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CopyOf")
            && debug.contains("SetPowerToughness")
            && debug.contains("AddAbility"),
        "expected typed copy/name/PT/ability modifications, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("until end of turn")
            && rendered.contains("name is this")
            && rendered.contains("he's 4/4")
            && rendered.contains("he has flying and this ability"),
        "expected all composite copy-exception characteristics, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_copy_filter_excluding_the_chosen_creature() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chosen Copy Probe")
        .parse_text(
            "Choose target creature you control. Each creature you control other than the chosen creature becomes a copy of that creature until end of turn, except it isn't legendary.",
        )
        .expect("chosen-creature exclusion should parse");
    let debug = format!("{def:#?}");
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(
        compact_debug.contains("IsNotTaggedObject")
            && !compact_debug.contains("excluded_card_types:[Creature]"),
        "expected an identity exclusion rather than a creature-type negation, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("each creature you control other than the chosen creature")
            && rendered_lower.contains("except it isn't legendary")
            && rendered_lower.contains("becomes a copy of that creature until end of turn"),
        "expected chosen-exclusion copy semantics in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mindculling_draw_then_target_opponent_discards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mindculling Variant")
        .parse_text("You draw two cards and target opponent discards two cards.")
        .expect("parse mindculling-like text");

    let effects = def.spell_effect.expect("spell effect");
    fn find_draw(effect: &crate::effect::Effect) -> Option<DrawCardsEffect> {
        if let Some(draw) = effect.downcast_ref::<DrawCardsEffect>() {
            return Some(draw.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_draw(child);
            }
        });
        found
    }
    fn find_discard(effect: &crate::effect::Effect) -> Option<DiscardEffect> {
        if let Some(discard) = effect.downcast_ref::<DiscardEffect>() {
            return Some(discard.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_discard(child);
            }
        });
        found
    }

    let draw = effects
        .iter()
        .find_map(find_draw)
        .expect("should include draw effect");
    assert_eq!(draw.count, Value::Fixed(2));
    assert_eq!(draw.player, PlayerFilter::You);

    let discard = effects
        .iter()
        .find_map(find_discard)
        .expect("should include discard effect");
    assert_eq!(discard.count, Value::Fixed(2));
    assert_eq!(
        discard.player,
        PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_player_shuffles_library_activation() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Soldier of Fortune Variant")
        .parse_text("{R}, {T}: Target player shuffles their library.")
        .expect("parse shuffle-target-player activation");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ShuffleLibraryEffect"),
        "expected shuffle-library effect, got {debug}"
    );
    assert!(
        !debug.contains("TargetOnlyEffect"),
        "shuffle activation must not compile as target-only effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_draw_then_look_top_card_of_each_players_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Case the Joint Variant")
        .parse_text("Draw two cards. Look at the top card of each player's library.")
        .expect("each-player library look clause should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let for_players = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ForPlayersEffect>())
        .expect("expected ForPlayersEffect for each-player look clause");
    let debug = format!("{for_players:?}");
    assert!(
        debug.contains("LookAtTopCardsEffect"),
        "expected nested look-at-top effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_lantern_of_insight_public_top_library_static() {
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_its_owner_shuffles_it_into_their_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deglamer Variant")
        .parse_text(
            "Choose target artifact or enchantment. Its owner shuffles it into their library.",
        )
        .expect("deglamer-style shuffle clause should parse");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ShuffleObjectsIntoLibraryEffect"),
        "expected shuffle-objects-into-library effect, got {debug}"
    );
    assert!(
        debug.contains("OwnerOf("),
        "expected owner-of-target library shuffle, got {debug}"
    );
    assert!(
        !debug.contains("target: Source"),
        "expected shuffle clause to keep the chosen target instead of falling back to source, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_owner_of_target_shuffles_it_into_their_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cathartic Variant")
            .parse_text(
                "The owner of target artifact or enchantment an opponent controls shuffles it into their library.",
            )
            .expect("owner-of-target shuffle clause should parse");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ShuffleObjectsIntoLibraryEffect"),
        "expected shuffle-objects-into-library effect, got {debug}"
    );
    assert!(
        debug.contains("OwnerOf("),
        "expected owner-of-target library shuffle, got {debug}"
    );
    assert!(
        !debug.contains("MoveToZoneEffect"),
        "expected shuffle clause to avoid bottom-library move fallback, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_blink_keeps_targeted_shuffle_investigate_and_token_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blink")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .parse_text(
                "I, III — Choose target creature. Its owner shuffles it into their library, then investigates. (They create a Clue token.)\nII, IV — Create a 2/2 black Alien Angel artifact creature token with first strike, vigilance, and \"Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.\"",
            )
            .expect("Blink text should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("InvestigateEffect"),
        "expected Blink to keep the investigate follow-up, got {debug}"
    );
    assert!(
        debug.contains("ShuffleObjectsIntoLibraryEffect")
            && debug.contains("target: Tagged")
            && debug.contains("targeted_0")
            && debug.contains("player: OwnerOf"),
        "expected Blink shuffle clause to keep the chosen creature and its owner, got {debug}"
    );
    assert!(
        debug.contains("SpellCastTrigger")
            && debug.contains("caster: Opponent")
            && debug.contains("RemoveCardTypes")
            && debug.contains("until: EndOfTurn"),
        "expected Blink token to preserve the quoted trigger structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_put_counters_on_each_creature_you_control_compiles_foreach() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Saga Counter Variant")
        .parse_text("Put a +1/+1 counter on each creature you control.")
        .expect("parse put counter on each");

    let effects = def.spell_effect.expect("spell effect");
    let effects_debug = format!("{effects:#?}");
    if let Some(foreach) = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ForEachObject>())
    {
        assert_eq!(foreach.filter, ObjectFilter::creature().you_control());

        let put = foreach
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<PutCountersEffect>())
            .expect("expected nested PutCountersEffect");
        assert_eq!(put.target, ChooseSpec::Iterated);
    } else if let Some(put) = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<PutCountersEffect>())
    {
        assert_eq!(
            put.target,
            ChooseSpec::all(ObjectFilter::creature().you_control())
        );
    } else {
        assert!(
            effects_debug.contains("PutCountersEffect")
                && effects_debug.contains("counter_type: PlusOnePlusOne")
                && effects_debug.contains("card_types: [\n")
                && effects_debug.contains("Creature")
                && effects_debug.contains("controller: Some(\n")
                && effects_debug.contains("You"),
            "expected a +1/+1 counter effect over creatures you control, got {effects_debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_remove_counters_from_among_creatures_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tayam Cost Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{3}, Remove three counters from among creatures you control: Draw a card.")
        .expect("distributed remove-counters cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("CostEffect") && cost_debug.contains("RemoveAnyCountersAmongEffect"),
        "expected effect-backed distributed counter-removal cost, got {cost_debug}"
    );
    assert!(
        cost_debug.contains("count: 3"),
        "expected count 3 in distributed counter-removal cost effect, got {cost_debug}"
    );
    assert!(
        cost_debug.contains("card_types: [Creature]"),
        "expected creature filter in distributed counter-removal cost effect, got {cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_remove_typed_counter_from_controlled_creature_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quillspike Cost Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{B/G}, Remove a -1/-1 counter from a creature you control: This creature gets +3/+3 until end of turn.",
            )
            .expect("typed non-source remove-counter cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("CostEffect") && cost_debug.contains("RemoveAnyCountersAmongEffect"),
        "expected effect-backed distributed counter-removal cost, got {cost_debug}"
    );
    assert!(
        cost_debug.contains("counter_type: Some(MinusOneMinusOne)"),
        "expected typed distributed counter-removal cost effect, got {cost_debug}"
    );
    assert!(
        cost_debug.contains("card_types: [Creature]"),
        "expected creature filter in distributed counter-removal cost, got {cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_modal_activated_header_with_counter_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Power Conduit Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "{T}, Remove a counter from a permanent you control: Choose one —\n• Put a charge counter on target artifact.\n• Put a +1/+1 counter on target creature.",
            )
            .expect("modal activated header should parse as activated ability");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    assert!(
        !def.abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_))),
        "should not produce triggered abilities: {:?}",
        def.abilities
    );

    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("CostEffect") && cost_debug.contains("RemoveAnyCountersAmongEffect"),
        "expected effect-backed remove-counters-among activation cost, got {cost_debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    let line = lines.join(" ");
    assert!(
        line.contains("Remove a counter from a permanent you control"),
        "expected cost text in activated rendering, got {line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_modal_activated_header_x_clause_rewrites_mode_x_values() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gnostro Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{T}: Choose one. X is the number of spells you've cast this turn.\n• Scry X.\n• This creature deals X damage to target creature.\n• You gain X life.",
            )
            .expect("modal activated header with X clause should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let effect_debug = format!("{:?}", activated.effects);
    assert!(
            effect_debug.contains("SpellsCastThisTurn(\n                                                        You,\n                                                    )")
                || effect_debug.contains("SpellsCastThisTurn(You)"),
            "expected mode X values to resolve to spells-cast count, got {effect_debug}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_remove_charge_counter_from_this_artifact_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ox Cart Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}, Remove a charge counter from this artifact: Destroy target creature.",
        )
        .expect("source-specific remove-counter artifact cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("CostEffect")
            && cost_debug.contains("RemoveCountersEffect")
            && cost_debug.contains("counter_type: Charge")
            && cost_debug.contains("target: Source"),
        "expected source remove-counters effect-backed cost, got {cost_debug}"
    );
    assert!(
        !cost_debug.contains("RemoveAnyCountersAmongEffect"),
        "expected source-specific cost, got distributed remove cost: {cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exile_all_creatures_with_power_constraint() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Power Exile Variant")
        .parse_text("Exile all creatures with power 4 or greater.")
        .expect("parse exile all creatures with power filter");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{:#?}", effects);
    assert!(
        debug.contains("ExileEffect"),
        "expected exile effect, got {debug}"
    );
    assert!(
        debug.contains("power: Some(")
            && debug.contains("GreaterThanOrEqual")
            && debug.contains("4"),
        "expected power >= 4 filter on exile-all effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_each_nonland_permanent_compiles_as_destroy_all() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destroy Each Variant")
        .parse_text("Destroy each nonland permanent with mana value X or less.")
        .expect("parse destroy-each clause");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{:#?}", effects);
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert!(
        debug.contains("spec: All("),
        "expected non-targeted destroy-all spec, got {debug}"
    );
    assert!(
        debug.contains("mana value X or less") || debug.contains("mana_value"),
        "expected mana-value filter to remain on destroy-all spec, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_all_permanents_except_artifacts_and_lands() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scourglass Variant")
        .parse_text("Destroy all permanents except for artifacts and lands.")
        .expect("parse destroy-all except clause");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{:#?}", effects);
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert!(
        debug.contains("spec: All("),
        "expected non-targeted destroy-all spec, got {debug}"
    );
    assert!(
        debug.contains("excluded_card_types")
            && debug.contains("Artifact")
            && debug.contains("Land"),
        "expected artifact/land exclusions on destroy-all filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_target_creature_with_flying_keeps_keyword_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destroy Flying Variant")
        .parse_text("Destroy target creature with flying.")
        .expect("parse flying-qualified destroy");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("static_abilities: [Flying]"),
        "expected flying ability filter in runtime effect, got {debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Destroy target creature with flying"),
        "expected rendered destroy filter to include flying qualifier, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_target_creature_with_islandwalk_keeps_marker_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destroy Islandwalk Variant")
        .parse_text("Destroy target creature with islandwalk.")
        .expect("parse islandwalk-qualified destroy");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ability_markers: [\"islandwalk\"]"),
        "expected islandwalk marker filter in runtime effect, got {debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Destroy target creature with islandwalk"),
        "expected rendered destroy filter to include islandwalk qualifier, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_destroy_target_creature_without_flying_keeps_exclusion_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destroy NonFlying Variant")
        .parse_text("Destroy target creature without flying.")
        .expect("parse without-flying destroy");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("excluded_static_abilities: [Flying]"),
        "expected flying exclusion in runtime effect, got {debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Destroy target creature without flying"),
        "expected rendered destroy filter to include without-flying qualifier, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_target_player_exiles_flashback_cards_from_their_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tombfire Variant")
        .parse_text("Target player exiles all cards with flashback from their graveyard.")
        .expect("parse tombfire-like text");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{:#?}", effects);
    assert!(
        debug.contains("TargetOnlyEffect"),
        "expected explicit target-context effect for target player, got {debug}"
    );
    assert!(
        debug.contains("ExileEffect"),
        "expected exile effect, got {debug}"
    );
    assert!(
        debug.contains("zone: Some(") && debug.contains("Graveyard"),
        "expected graveyard zone filter on exile effect, got {debug}"
    );
    assert!(
        debug.contains("owner: Some(") && debug.contains("Target(") && debug.contains("Any"),
        "expected target-player owner filter on exile effect, got {debug}"
    );
    assert!(
        debug.contains("alternative_cast: Some(") && debug.contains("Flashback"),
        "expected flashback-qualified exile filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_each_opponent_sacrifices_creature_of_their_choice_renders_compactly() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Each Opponent Sacrifice Variant")
        .parse_text("Each opponent sacrifices a creature of their choice.")
        .expect("parse each-opponent sacrifice text");

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Each opponent sacrifices a creature of their choice"),
        "expected compact each-opponent sacrifice text, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_each_other_player_sacrifices_creature_of_their_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Each Other Player Sacrifice Variant")
        .parse_text("Each other player sacrifices a creature of their choice.")
        .expect("parse each-other-player sacrifice text");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("Sacrifice") && debug.contains("NotYou"),
        "expected NotYou sacrifice effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_myrkuls_edict_strict_d20_table_and_greatest_power_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Myrkul's Edict")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Roll a d20.\n\
                 1—9 | Choose an opponent. That player sacrifices a creature of their choice.\n\
                 10—19 | Each opponent sacrifices a creature of their choice.\n\
                 20 | Each opponent sacrifices a creature with the greatest power among creatures that player controls.",
            )
            .expect("Myrkul's Edict should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let model_debug = format!("{:#?}", def.spell_effect);
    assert!(
        rendered.contains("Roll a d20"),
        "expected Myrkul's Edict to render the d20 roll, got {rendered}"
    );
    assert!(
        rendered.contains("1—9 |") && rendered.contains("10—19 |") && rendered.contains("20 |"),
        "expected Myrkul's Edict to render all d20 result rows, got {rendered}"
    );
    assert!(
        model_debug.contains("GreatestPower")
            && model_debug.contains("Sacrifice")
            && model_debug.contains("20"),
        "expected Myrkul's Edict model to preserve the greatest-power sacrifice branch, got {model_debug}"
    );
    assert!(
        !rendered.contains("effect #") && !rendered.contains("dynamic value"),
        "Myrkul's Edict should not expose raw effect ids or dynamic-value wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_unless_controller_pays_life_keeps_unless_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Unless Life Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Tap target creature unless its controller pays 2 life.")
        .expect("parse unless-pays-life clause");

    let lines = unprocessed_compiled_lines(&def);
    let activated = lines.join(" ");
    assert!(
        activated.contains("unless"),
        "expected unless branch in render, got {activated}"
    );
    assert!(
        activated.contains("2 life"),
        "expected life-payment alternative in render, got {activated}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_damage_unless_controller_has_source_deal_damage() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blazing Salvo Variant")
            .parse_text(
                "This spell deals 3 damage to target creature unless that creature's controller has this spell deal 5 damage to them.",
            )
            .expect("parse damage-unless-controller alternative");

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("unless") && spell_line.contains("deal 5 damage"),
        "expected unless-controller alternative damage text, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_equip_keyword_displays_as_keyword_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Strider Harness Equip Variant")
        .parse_text(
            "Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)",
        )
        .expect("parse equip line");

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    assert!(matches!(&ability.kind, AbilityKind::Activated(_)));

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line == "Equip {1}"),
        "expected keyword ability line, got {:?}",
        lines
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_equip_keyword_with_subtype_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Veteran Powerblade Equip Variant")
        .subtypes(vec![Subtype::Equipment])
        .parse_text("Equip Soldier {W}\nEquip {2}")
        .expect("parse subtype-qualified equip lines");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("subtypes:") && abilities_debug.contains("Soldier"),
        "expected subtype-qualified equip target filter, got {abilities_debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    assert!(
        lines.iter().any(|line| line == "Equip Soldier {W}"),
        "expected subtype-qualified equip keyword line, got {:?}",
        lines
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_skip_turn_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Meditate")
        .parse_text("You skip your next turn.")
        .expect("parse skip turn");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<SkipTurnEffect>().is_some()),
        "should include skip turn effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_skip_draw_step_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fatigue")
        .parse_text("Target player skips their next draw step.")
        .expect("parse skip draw step");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<SkipDrawStepEffect>().is_some()),
        "should include skip draw step effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_skip_your_draw_step_inline_subject_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Null Profusion Variant")
        .parse_text("Skip your draw step.")
        .expect("parse inline-subject skip draw step");

    assert!(
        def.spell_effect.is_none(),
        "an active-source draw-step rule must not become a one-shot spell effect"
    );
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::PlayerSkipsDrawStep)
        }),
        "should include the typed draw-step-skipping static ability"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        ["Skip your draw step."],
        "the static rule should retain its non-next-step Oracle surface"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_skip_combat_phases_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "False Peace")
        .parse_text("Target player skips all combat phases of their next turn.")
        .expect("parse skip combat phases");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<SkipCombatPhasesEffect>().is_some()),
        "should include skip combat phases effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_skip_next_combat_phase_this_turn_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Moment of Silence")
        .parse_text("Target player skips their next combat phase this turn.")
        .expect("parse skip next combat phase this turn");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects.iter().any(|e| e
            .downcast_ref::<SkipNextCombatPhaseThisTurnEffect>()
            .is_some()),
        "should include skip-next-combat-phase-this-turn effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_cast_from_graveyard_trigger_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Secrets of the Dead Probe")
        .parse_text("Whenever you cast a spell from your graveyard, draw a card.")
        .expect("parse spell-cast-from-graveyard trigger");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("Whenever you cast a spell from your graveyard"),
        "expected graveyard origin qualifier in trigger text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_cast_another_during_your_turn_trigger_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Geralf Qualifier Probe")
            .parse_text(
                "Whenever you cast a spell during your turn other than your first spell that turn, draw a card.",
            )
            .expect("parse qualified spell-cast trigger");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("Whenever you cast another spell during your turn"),
        "expected spell-order + turn qualifier in trigger text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_cast_third_each_turn_trigger_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Third Spell Probe")
        .parse_text("Whenever you cast your third spell each turn, draw a card.")
        .expect("parse third-spell-each-turn trigger");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("Whenever you cast your third spell each turn"),
        "expected third-spell qualifier in trigger text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_pest_token_subtype_in_token_rendering() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pest Summoning Probe")
            .parse_text(
                "Create two 1/1 black and green Pest creature tokens with \"When this token dies, you gain 1 life.\"",
            )
            .expect("parse pest token creation with dies lifegain text");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("Pest creature token"),
        "expected Pest subtype to be retained in token rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_token_with_prowess_keyword_in_rendering() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Prowess Token Probe")
        .parse_text("Create a 4/4 red Dragon Elemental creature token with flying and prowess.")
        .expect("parse token creation with prowess");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.to_ascii_lowercase().contains("prowess"),
        "expected prowess keyword in token rendering, got: {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_named_source_damaged_by_trigger_as_this_creature() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rot Wolf Trigger Probe")
        .parse_text(
            "Whenever a creature dealt damage by Rot Wolf this turn dies, you may draw a card.",
        )
        .expect("parse named-source damaged-by trigger");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("dealt damage by this creature this turn dies"),
        "expected named source in damaged-by trigger to resolve to source creature, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enchanted_creature_damaged_by_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Enchanted Trigger Probe")
        .parse_text(
            "Whenever a creature dealt damage by enchanted creature this turn dies, draw a card.",
        )
        .expect("parse enchanted-creature damaged-by trigger");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("dealt damage by enchanted creature this turn dies"),
        "expected enchanted-creature damaged-by trigger rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_enters_as_copy_with_except_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Evil Twin Variant")
            .parse_text(
                "You may have this creature enter as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"",
            )
            .expect("enters-as-copy replacement with added ability should parse");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join(" ");
    assert!(
        joined.contains("copy of any creature on the battlefield, except it has"),
        "expected copy-as-enters text in render output, got {joined}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("added_abilities"),
        "expected copy-as-enters lowering to record added ability support, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_divided_damage_distribution_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fire at Will Variant").parse_text(
            "Fire at Will deals 3 damage divided as you choose among one, two, or three target attacking or blocking creatures.",
        )
        .expect("divided damage distribution should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DealDistributedDamageEffect"),
        "expected distributed damage effect, got {debug}"
    );
    assert!(
        debug.contains("ChoiceCount { min: 1, max: Some(3)"),
        "expected one-to-three distributed damage targets, got {debug}"
    );
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
            joined.contains(
                "deal 3 damage divided as you choose among one, two, or three target attacking or blocking creatures"
            ),
            "expected enumerated distributed damage rendering, got {joined}"
        );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_choose_land_of_each_basic_land_type_then_destroy() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sundering Titan Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters or leaves the battlefield, choose a land of each basic land type, then destroy those lands.",
            )
            .expect("Sundering Titan style choice should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let effects = &triggered.effects.segments[0].default_effects;
    let choices = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .collect::<Vec<_>>();
    assert_eq!(choices.len(), 5, "expected five basic-land-type choices");
    for subtype in [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ] {
        assert!(
            choices
                .iter()
                .any(|choose| choose.filter.subtypes == vec![subtype]
                    && choose.filter.controller == Some(PlayerFilter::Any)),
            "expected unrestricted land choice for {subtype:?}, got {choices:#?}"
        );
    }

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("choose a Plains")
            && joined.contains("choose an Island")
            && joined.contains("choose a Swamp")
            && joined.contains("choose a Mountain")
            && joined.contains("choose a Forest")
            && joined.contains("destroy those lands"),
        "expected all basic-land-type choices and the destroy follow-up, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_noncreature_graveyard_from_battlefield_trigger_keeps_controller() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ashiok's Reaper Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Whenever an enchantment you control is put into a graveyard from the battlefield, draw a card.",
            )
            .expect("enchantment-you-control graveyard trigger should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let trigger_debug = format!("{:?}", ability.trigger);
    assert!(
        trigger_debug.contains("controller: Some(You)"),
        "expected trigger subject to preserve controller, got {trigger_debug}"
    );

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "whenever an enchantment you control is put into a graveyard from the battlefield"
        ) && !joined.contains("enchantment you control dies"),
        "expected noncreature zone-change wording, got {joined}"
    );

    let yomiji = CardDefinitionBuilder::new(CardId::new(), "Yomiji Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Whenever a legendary permanent other than Yomiji is put into a graveyard from the battlefield, return that card to its owner's hand.",
            )
            .expect("other-than-source permanent graveyard trigger should parse");
    let yomiji_joined = crate::compiled_text::unprocessed_compiled_lines(&yomiji)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        yomiji_joined.contains(
            "whenever another legendary permanent is put into a graveyard from the battlefield"
        ),
        "expected the canonical other-than-source zone-change wording, got {yomiji_joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_verb_leading_line_keeps_all_typed_effects() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Nahiri Lithoforming Variant")
            .parse_text(
                "Sacrifice X lands. For each land sacrificed this way, draw a card. You may play X additional lands this turn. Lands you control enter tapped this turn.",
            );
    let def = result.expect("verb-leading spell text should parse structurally");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("SacrificePlayerEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("AdditionalLandPlaysEffect")
            && debug.contains("EnterTappedForFilter"),
        "expected every verb-leading effect to remain typed, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_choose_leading_line_does_not_fallback_to_static_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Rebuild City Variant").parse_text(
            "Choose target land. Create three tokens that are copies of it, except they're 3/3 creatures in addition to their other types and they have vigilance and menace.",
        );
    assert!(
        result.is_err(),
        "unsupported choose-leading spell text should fail parse instead of falling back to a partial static ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_supports_spent_to_cast_conditional_clause_chain() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Firespout Variant")
            .parse_text(
            "Firespout deals 3 damage to each creature without flying if {R} was spent to cast this spell and 3 damage to each creature with flying if {G} was spent to cast this spell.",
            )
            .expect("spent-to-cast conditional chain should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("if {r} was spent to cast this spell")
            && joined.contains("if {g} was spent to cast this spell"),
        "expected both spent-to-cast conditionals in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_would_enter_replacement_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Mistcaller Variant").parse_text(
            "Sacrifice this creature: Until end of turn, if a nontoken creature would enter and it wasn't cast, exile it instead.",
        );
    assert!(
        result.is_err(),
        "unsupported would-enter replacement clause should fail parse instead of collapsing to an immediate exile effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_different_mana_value_constraint_clause() {
    let result =
            CardDefinitionBuilder::new(CardId::new(), "Agadeem Awakening Variant").parse_text(
                "Return from your graveyard to the battlefield any number of target creature cards that each have a different mana value X or less.",
            );
    assert!(
        result.is_err(),
        "unsupported different-mana-value target constraint should fail parse instead of collapsing target restrictions"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_supports_most_common_color_constraint_clause() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Barrin Unmaking Variant")
            .parse_text(
                "Return target permanent to its owner's hand if that permanent shares a color with the most common color among all permanents or a color tied for most common.",
            )
            .expect("most-common-color conditional should parse structurally");
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(
        debug.contains("ConditionalEffect") && debug.contains("SharesMostCommonPermanentColor"),
        "expected most-common-color target condition in lowered effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_power_vs_count_conditional_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Unified Strike Variant")
            .parse_text(
                "Exile target attacking creature if its power is less than or equal to the number of Soldiers on the battlefield.",
            );
    let def = result.expect("power-vs-count conditional should parse structurally");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ConditionalEffect")
            && debug.contains("left: SourcePower")
            && debug.contains("right: Count")
            && debug.contains("Soldier"),
        "expected a typed power-vs-count condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_put_into_graveyards_from_battlefield_count_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Structural Assault Variant")
            .parse_text(
                "Destroy all artifacts, then this spell deals damage to each creature equal to the number of artifacts that were put into graveyards from the battlefield this turn.",
            );
    assert!(
        result.is_err(),
        "unsupported put-into-graveyards-from-battlefield count clause should fail parse instead of collapsing to a graveyard destroy effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_spell_with_it_has_token_trigger_stays_as_spell_effects() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Make Mischief Variant")
            .parse_text(
                "This spell deals 1 damage to any target. Create a 1/1 red Devil creature token. It has \"When this token dies, it deals 1 damage to any target.\"",
            )
            .expect("parse spell with token dies trigger rider");

    assert!(
        def.abilities.is_empty(),
        "spell line with token trigger rider should not collapse into a granted static ability"
    );
    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("DealDamageEffect") && spell_debug.contains("CreateTokenEffect"),
        "expected direct damage + token creation effects, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_standalone_token_reminder_sentence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sound the Call Variant")
            .parse_text(
                "Create a 1/1 green Wolf creature token. It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
            )
            .expect("standalone token reminder sentence should parse as token reminder text");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("named sound the call"),
        "expected token reminder text to keep named-card clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_cast_this_spell_only_declare_attackers_step_builds_typed_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Declare Attackers Restriction Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Cast this spell only during the declare attackers step and only if you've been attacked this step.\nDraw a card.",
            )
            .expect("declare attackers cast restriction should parse as typed static ability");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisSpellCastRestriction"),
        "expected typed this-spell cast restriction ability, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText")
            && !debug.contains("StaticAbilityId::KeywordFallbackText")
            && !debug.contains("StaticAbilityId::RuleFallbackText")
            && !debug.contains("StaticAbilityId::UnsupportedParserLine"),
        "cast restriction should not compile through placeholder/marker ids: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn this_spell_cast_restriction_runtime_requires_attacked_declare_attackers_step() {
    use crate::alternative_cast::CastingMethod;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::decision::can_cast_spell;
    use crate::game_state::{Phase, Step};
    use crate::ids::PlayerId;

    let def = CardDefinitionBuilder::new(CardId::new(), "Assassin's Blade Probe")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::new())
            .parse_text(
                "Cast this spell only during the declare attackers step and only if you've been attacked this step.\nDraw a card.",
            )
            .expect("declare attackers cast restriction should parse");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let attacker_card = CardBuilder::new(CardId::from_raw(70130), "Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker_id = game.create_object_from_card(&attacker_card, bob, Zone::Battlefield);

    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        !can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should fail outside declare attackers step"
    );

    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        }],
        ..CombatState::default()
    });

    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        !can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should fail when you were not attacked this step"
    );

    if let Some(combat) = game.combat.as_mut() {
        combat.attackers[0].target = AttackTarget::Player(alice);
    }
    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should pass when cast during declare attackers after being attacked"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn this_spell_cast_restriction_runtime_before_blockers_window() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::can_cast_spell;
    use crate::game_state::{Phase, Step};
    use crate::ids::PlayerId;

    let def = CardDefinitionBuilder::new(CardId::new(), "Panic Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new())
        .parse_text(
            "Cast this spell only during combat before blockers are declared.\nDraw a card.",
        )
        .expect("combat-before-blockers cast restriction should parse");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        !can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should fail outside combat"
    );

    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::BeginCombat);
    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should allow casting in begin combat step"
    );

    game.turn.step = Some(Step::DeclareAttackers);
    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should allow casting in declare attackers step"
    );

    game.turn.step = Some(Step::DeclareBlockers);
    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        !can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should fail once blockers are being declared"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn this_spell_cast_restriction_runtime_requires_another_spell_cast_this_turn() {
    use crate::alternative_cast::CastingMethod;
    use crate::card::CardBuilder;
    use crate::decision::can_cast_spell;
    use crate::ids::PlayerId;

    let def = CardDefinitionBuilder::new(CardId::new(), "Illusory Angel Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new())
        .parse_text("Cast this spell only if you've cast another spell this turn.\nDraw a card.")
        .expect("cast-another-spell restriction should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        !debug.contains("StaticAbilityId::RuleFallbackText")
            && !debug.contains("StaticAbilityId::KeywordMarker"),
        "cast-another-spell restriction should be typed, got {debug}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        !can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should fail before any prior spell is cast"
    );

    let prior_spell = CardBuilder::new(CardId::from_raw(70131), "Prior Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let prior_id = game.create_object_from_card(&prior_spell, alice, Zone::Graveyard);
    let prior_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(prior_id).expect("prior spell should exist"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(prior_snapshot.object_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);

    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should pass after another spell was cast this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn this_spell_cast_restriction_runtime_uses_doctor_subtype() {
    use crate::alternative_cast::CastingMethod;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::can_cast_spell;
    use crate::ids::PlayerId;
    use crate::types::Subtype;

    let def = CardDefinitionBuilder::new(CardId::new(), "Doctor Restriction Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new())
        .parse_text("Cast this spell only if you control two or more Doctors.\nDraw a card.")
        .expect("doctor subtype cast restriction should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouControlAtLeast")
            && debug.contains("subtypes: [Doctor]")
            && debug.contains("count: 2"),
        "expected typed Doctor subtype restriction, got {debug}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let spell_id = game.create_object_from_definition(&def, alice, Zone::Hand);

    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        !can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should fail with no Doctors"
    );

    for index in 0..2u32 {
        let doctor = CardBuilder::new(CardId::from_raw(74000 + index), "Doctor")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Doctor])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let _ = game.create_object_from_card(&doctor, alice, Zone::Battlefield);
    }

    let spell = game.object(spell_id).expect("spell should exist");
    assert!(
        can_cast_spell(&game, alice, spell, &CastingMethod::Normal),
        "restriction should pass with two Doctor creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_cumulative_upkeep_generic_line_builds_typed_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cumulative Upkeep Variant")
        .parse_text("Cumulative upkeep {1}")
        .expect("parse cumulative upkeep keyword line");

    assert!(
        def.spell_effect.is_none(),
        "cumulative upkeep line should compile as an ability, not a spell effect"
    );
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeginningOfUpkeepTrigger")
            && debug.contains("PutCountersEffect")
            && debug.contains("CumulativeUpkeepEffect"),
        "expected cumulative upkeep to compile into upkeep trigger primitives, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText")
            && !debug.contains("StaticAbilityId::KeywordFallbackText")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "cumulative upkeep {{1}} should not compile as fallback marker ability: {debug}"
    );
    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.to_ascii_lowercase().contains("cumulative upkeep"),
        "expected cumulative upkeep text in compiled abilities, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn cumulative_upkeep_generic_runtime_pays_then_sacrifices_when_unpaid() {
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::new(), "Cumulative Upkeep Runtime Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Cumulative upkeep {1}")
        .expect("parse cumulative upkeep keyword line");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected cumulative upkeep triggered ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected cumulative upkeep to compile as triggered ability");
    };

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source_card = crate::card::CardBuilder::new(CardId::from_raw(70110), "Upkeep Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let run_upkeep = |game: &mut crate::game_state::GameState| {
        let mut ctx = ExecutionContext::new_default(source, alice);
        for effect in &triggered.effects {
            execute_effect(game, effect, &mut ctx)
                .expect("cumulative upkeep trigger effect execution should succeed");
        }
    };

    run_upkeep(&mut game);
    let source_obj = game
        .object(source)
        .expect("source should remain after first upkeep");
    assert_eq!(
        source_obj
            .counters
            .get(&CounterType::Age)
            .copied()
            .unwrap_or(0),
        1,
        "first cumulative upkeep should add one age counter"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .mana_pool
            .total(),
        0,
        "first cumulative upkeep should spend available mana payment"
    );

    run_upkeep(&mut game);
    let source_obj = game.object(source);
    assert!(
        source_obj.is_none() || source_obj.is_some_and(|object| object.zone == Zone::Graveyard),
        "second cumulative upkeep without mana should sacrifice source, got {source_obj:?}"
    );
}

#[test]
fn parse_jotun_grunt_cumulative_upkeep_strict_regression() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Jötun Grunt")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![
                vec![crate::mana::ManaSymbol::Generic(1)],
                vec![crate::mana::ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Giant, Subtype::Soldier])
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .parse_text("Cumulative upkeep—Put two cards from a single graveyard on the bottom of their owner's library. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)")
            .expect("Jötun Grunt should parse strictly");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CumulativeUpkeepEffect") && debug.contains("MoveToZoneEffect"),
        "expected Jötun Grunt cumulative upkeep move effect, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("KeywordFallbackText")
            && !debug.contains("RuleFallbackText"),
        "Jötun Grunt should not compile as fallback marker text: {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("cumulative upkeep")
            && joined.contains("put two cards from a single graveyard"),
        "expected compiled text to include Jötun Grunt upkeep clause, got {joined}"
    );
}

#[test]
fn jotun_grunt_cumulative_upkeep_runtime_branches() {
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::{CardId as RuntimeCardId, PlayerId};
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::new(), "Jötun Grunt")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .parse_text("Cumulative upkeep—Put two cards from a single graveyard on the bottom of their owner's library. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)")
            .expect("Jötun Grunt should parse");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected Jötun Grunt cumulative upkeep triggered ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected triggered ability");
    };

    let run_upkeep = |game: &mut crate::game_state::GameState,
                      source: crate::ids::ObjectId,
                      controller: PlayerId| {
        let mut ctx = ExecutionContext::new_default(source, controller);
        for effect in &triggered.effects {
            execute_effect(game, effect, &mut ctx)
                .expect("Jötun Grunt upkeep trigger should execute");
        }
    };

    let mut game_paid =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_card = crate::card::CardBuilder::new(RuntimeCardId::from_raw(70300), "Jötun Grunt")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .build();
    let source_paid = game_paid.create_object_from_card(&source_card, alice, Zone::Battlefield);

    for idx in 0..2 {
        let card =
            crate::card::CardBuilder::new(RuntimeCardId::from_raw(70310 + idx), "Graveyard Fodder")
                .card_types(vec![CardType::Creature])
                .build();
        game_paid.create_object_from_card(&card, bob, Zone::Graveyard);
    }

    run_upkeep(&mut game_paid, source_paid, alice);
    let source_obj_paid = game_paid
        .object(source_paid)
        .expect("source should remain when upkeep cost is payable");
    assert_eq!(
        source_obj_paid
            .counters
            .get(&CounterType::Age)
            .copied()
            .unwrap_or(0),
        1,
        "upkeep should add one age counter"
    );
    assert_eq!(
        game_paid
            .player(bob)
            .expect("bob should exist")
            .graveyard
            .len(),
        0,
        "paying upkeep should move two cards out of the chosen graveyard"
    );

    let mut game_unpaid =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source_unpaid = game_unpaid.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let one_card = crate::card::CardBuilder::new(RuntimeCardId::from_raw(70400), "Only Card")
        .card_types(vec![CardType::Creature])
        .build();
    game_unpaid.create_object_from_card(&one_card, bob, Zone::Graveyard);

    run_upkeep(&mut game_unpaid, source_unpaid, alice);
    let source_obj_unpaid = game_unpaid
        .object(source_unpaid)
        .expect("source should still exist for insufficient graveyard branch");
    assert_eq!(
        source_obj_unpaid
            .counters
            .get(&CounterType::Age)
            .copied()
            .unwrap_or(0),
        1,
        "insufficient graveyard branch should still add an age counter"
    );
    assert_eq!(
        game_unpaid
            .player(bob)
            .expect("bob should exist")
            .graveyard
            .len(),
        0,
        "single-card graveyard branch should still move available cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_filter_granted_cumulative_upkeep_compiles_as_granted_triggered_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Breath of Dreams Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Green creatures have \"Cumulative upkeep {1}.\"")
        .expect("filter granted cumulative upkeep should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("GrantObjectAbilityForFilter")
            && debug.contains("BeginningOfUpkeepTrigger")
            && debug.contains("PutCountersEffect")
            && (debug.contains("UnlessPaysEffect") || debug.contains("CumulativeUpkeepEffect")),
        "expected granted cumulative upkeep to compile as granted triggered ability, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::KeywordFallbackText")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "filter granted cumulative upkeep should not fallback to marker/static placeholder: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_attached_granted_cumulative_upkeep_compiles_as_attached_triggered_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mana Chains Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nEnchanted creature has \"Cumulative upkeep {1}.\"")
        .expect("attached granted cumulative upkeep should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttachedAbilityGrant")
            && debug.contains("BeginningOfUpkeepTrigger")
            && debug.contains("PutCountersEffect")
            && (debug.contains("UnlessPaysEffect") || debug.contains("CumulativeUpkeepEffect")),
        "expected attached granted cumulative upkeep to compile as attached triggered ability, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::KeywordFallbackText")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "attached granted cumulative upkeep should not fallback to marker/static placeholder: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_non_cost_cumulative_upkeep_line_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Varchild's War-Riders Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Cumulative upkeep—Have an opponent create a 1/1 red Survivor creature token. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\nTrample; rampage 1",
            )
            .expect_err("non-cost cumulative upkeep payment should fail loudly");

    assert!(
        format!("{err:?}")
            .to_ascii_lowercase()
            .contains("cumulative")
            || format!("{err:?}")
                .to_ascii_lowercase()
                .contains("cost-executable"),
        "expected loud cumulative upkeep cost error, got {err:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_skulk_keyword_line_builds_skulk_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skulk Probe")
        .parse_text("Skulk")
        .expect("parse skulk keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Skulk"),
        "expected skulk ability in debug output, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "skulk should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_relative_power_blocking_rules_text_line_builds_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wandering Wolf Rules Text Probe")
        .parse_text("Creatures with power less than this creature's power can't block it.")
        .expect("parse wandering wolf rules text line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CantBeBlockedByLowerPowerThanSource"),
        "expected relative-power blocking ability in debug output, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText")
            && !debug.contains("StaticAbilityId::UnsupportedParserLine"),
        "relative-power blocking rules text should not compile as placeholder ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn relative_power_blocking_rules_text_runtime_restricts_lower_power_blocks() {
    use crate::card::PowerToughness;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    let attacker_def =
        CardDefinitionBuilder::new(CardId::from_raw(70101), "Wandering Wolf Rules Text")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("Creatures with power less than this creature's power can't block it.")
            .expect("parse wandering wolf rules text line");

    let equal_blocker_def = CardDefinitionBuilder::new(CardId::from_raw(70102), "Equal Blocker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let smaller_blocker_def =
        CardDefinitionBuilder::new(CardId::from_raw(70103), "Smaller Blocker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
    let larger_blocker_def = CardDefinitionBuilder::new(CardId::from_raw(70104), "Larger Blocker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let attacker_id = game.create_object_from_definition(&attacker_def, alice, Zone::Battlefield);
    let equal_blocker_id =
        game.create_object_from_definition(&equal_blocker_def, bob, Zone::Battlefield);
    let smaller_blocker_id =
        game.create_object_from_definition(&smaller_blocker_def, bob, Zone::Battlefield);
    let larger_blocker_id =
        game.create_object_from_definition(&larger_blocker_def, bob, Zone::Battlefield);

    let attacker = game
        .object(attacker_id)
        .expect("attacker should exist")
        .clone();
    let equal_blocker = game
        .object(equal_blocker_id)
        .expect("equal blocker should exist")
        .clone();
    let smaller_blocker = game
        .object(smaller_blocker_id)
        .expect("smaller blocker should exist")
        .clone();
    let larger_blocker = game
        .object(larger_blocker_id)
        .expect("larger blocker should exist")
        .clone();

    assert!(
        crate::rules::combat::can_block(&attacker, &equal_blocker, &game),
        "equal-power creature should be allowed to block relative-power attacker"
    );
    assert!(
        !crate::rules::combat::can_block(&attacker, &smaller_blocker, &game),
        "lower-power creature should not block relative-power attacker"
    );
    assert!(
        crate::rules::combat::can_block(&attacker, &larger_blocker, &game),
        "greater-power creature should be allowed to block relative-power attacker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_ingest_keyword_line_builds_triggered_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ingest Probe")
        .parse_text("Ingest")
        .expect("parse ingest keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisDealsCombatDamageToPlayerTrigger"),
        "expected ingest combat-damage trigger, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "ingest should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_battle_cry_keyword_line_builds_triggered_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Battle Cry Probe")
        .parse_text("Battle cry")
        .expect("parse battle cry keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisAttacksTrigger"),
        "expected battle cry attack trigger, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "battle cry should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_dethrone_keyword_line_builds_most_life_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dethrone Probe")
        .parse_text("Dethrone")
        .expect("parse dethrone keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisAttacksPlayerWithMostLifeTrigger"),
        "expected dethrone most-life attack trigger, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "dethrone should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_evolve_keyword_line_builds_etb_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Evolve Probe")
        .parse_text("Evolve")
        .expect("parse evolve keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ZoneChangeTrigger") && debug.contains("Specific(Battlefield)"),
        "expected evolve ETB zone-change trigger, got {debug}"
    );
    assert!(
        debug.contains("EvolveEffect"),
        "expected evolve resolution effect, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "evolve should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_mentor_keyword_line_builds_attack_target_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mentor Probe")
        .parse_text("Mentor")
        .expect("parse mentor keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisAttacksTrigger"),
        "expected mentor attack trigger matcher, got {debug}"
    );
    assert!(
        debug.contains("power_relative_to_source: Some(LessThanSource)"),
        "expected mentor lesser-power target constraint, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "mentor should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_training_keyword_line_builds_greater_power_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Training Probe")
        .parse_text("Training")
        .expect("parse training keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisAttacksWithGreaterPowerTrigger"),
        "expected training trigger matcher, got {debug}"
    );
    assert!(
        debug.contains("PutCountersEffect")
            && debug.contains("EmitKeywordActionEffect")
            && debug.contains("Train"),
        "expected training to resolve via primitive counter + keyword-action emission, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "training should not compile as placeholder marker ability: {debug}"
    );
}

#[test]
fn training_trigger_execution_adds_counter_and_emits_train_action() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::events::{KeywordActionEvent, KeywordActionKind};
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::new(), "Training Probe")
        .card_types(vec![CardType::Creature])
        .training()
        .build();

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("expected Training ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected Training to add a triggered ability");
    };

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source_card = CardBuilder::new(CardId::from_raw(9001), "Training Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let mut ctx = ExecutionContext::new_default(source, alice);

    let mut saw_train_keyword_action = false;
    for effect in &triggered.effects {
        let outcome = execute_effect(&mut game, effect, &mut ctx)
            .expect("training trigger effect execution should succeed");
        for event in outcome.events {
            if let Some(action) = event.downcast::<KeywordActionEvent>()
                && action.action == KeywordActionKind::Train
                && action.player == alice
                && action.source == source
            {
                saw_train_keyword_action = true;
            }
        }
    }

    let source_obj = game.object(source).expect("source object should exist");
    assert_eq!(
        source_obj
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        1,
        "training trigger should place one +1/+1 counter on source"
    );
    assert!(
        saw_train_keyword_action,
        "training trigger should emit a train keyword-action event"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_renown_keyword_line_builds_combat_damage_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Renown Probe")
        .parse_text("Renown 1")
        .expect("parse renown keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisDealsCombatDamageToPlayerTrigger"),
        "expected renown combat-damage trigger matcher, got {debug}"
    );
    assert!(
        debug.contains("RenownEffect"),
        "expected renown resolution effect, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "renown should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_afterlife_keyword_line_builds_dies_token_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Afterlife Probe")
        .parse_text("Afterlife 2")
        .expect("parse afterlife keyword line");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ZoneChangeTrigger")
            && debug.contains("from: Specific(Battlefield)")
            && debug.contains("to: Specific(Graveyard)"),
        "expected afterlife dies zone-change trigger, got {debug}"
    );
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected afterlife token creation effect, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "afterlife should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_fabricate_keyword_line_builds_etb_modal_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fabricate Probe")
        .parse_text("Fabricate 1")
        .expect("parse fabricate keyword line");

    let rendered = unprocessed_compiled_lines(&def).join(" | ");
    assert!(
        rendered.contains("Fabricate 1"),
        "expected raw compiled output to keep keyword-only fabricate text, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("choose one"),
        "keyword-only fabricate should not render the expanded modal scaffold, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ZoneChangeTrigger") && debug.contains("Specific(Battlefield)"),
        "expected fabricate ETB zone-change trigger, got {debug}"
    );
    assert!(
        debug.contains("ChooseModeEffect"),
        "expected fabricate modal choice effect, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            && !debug.contains("StaticAbilityId::RuleFallbackText"),
        "fabricate should not compile as placeholder marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_this_creature_becomes_renowned_trigger_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Renowned Trigger Probe")
        .parse_text("Whenever this creature becomes renowned, draw a card.")
        .expect("parse source renowned trigger");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("KeywordActionTrigger")
            && debug.contains("action: Renown")
            && debug.contains("source_must_match: true"),
        "expected keyword-action trigger for becoming renowned, got {debug}"
    );
    assert!(
        debug.contains("DrawCardsEffect"),
        "expected draw effect on renowned trigger, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_investigate_for_each_clause_uses_prior_effect_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Declaration Variant")
            .parse_text(
                "Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.",
            )
            .expect("investigate-for-each clause should parse");

    let effects = def.spell_effect.expect("spell effect");
    let investigate = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::InvestigateEffect>()
                .cloned()
        })
        .expect("should include investigate effect");

    let Value::Count(filter) = &investigate.count else {
        panic!(
            "investigate count should be derived from the prior exile, got {:?}",
            investigate.count
        );
    };
    assert_eq!(filter.zone, Some(crate::zone::Zone::Exile));
    assert!(
        filter
            .card_types
            .contains(&crate::types::CardType::Creature)
    );
    assert!(filter.nontoken);
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }),
        "investigate count should follow the tagged nontoken creatures exiled by the prior effect: {filter:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_same_name_exile_until_source_leaves_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deputy Variant")
            .parse_text(
                "Exile target nonland permanent an opponent controls and all other nonland permanents that player controls with the same name as that permanent until this creature leaves the battlefield.",
            )
            .expect("same-name exile-until clause should parse");

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("until this permanent leaves the battlefield"),
        "compiled text should preserve exile duration, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_exile_target_until_source_leaves_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Static Prison Variant")
            .parse_text(
                "Exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
            )
            .expect("target exile-until clause should parse");

    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("until this permanent leaves the battlefield"),
        "compiled text should preserve exile-until duration, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_phase_out_until_leaves_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Oubliette Variant").parse_text(
            "When this enchantment enters, target creature phases out until this enchantment leaves the battlefield.",
        );
    assert!(
        result.is_err(),
        "unsupported phase-out-until-leaves clause should fail parse instead of mis-targeting objects"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_untap_then_phase_out_until_source_leaves_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Out of Time Variant")
        .parse_text(
            "When this enchantment enters, untap all creatures, then those creatures phase out until this enchantment leaves the battlefield. Put a time counter on this enchantment for each creature that phased out this way.",
        )
        .expect("linked phase-out duration should parse");

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains(
            "untap all creatures, then those creatures phase out until this enchantment leaves the battlefield"
        ),
        "compiled text should preserve the linked duration: {compiled}"
    );
    assert!(
        compiled.contains("for each creature that phased out this way"),
        "follow-up count should bind to phased-out object memory: {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_same_name_as_another_in_hand_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Hint Insanity Variant").parse_text(
            "Target player reveals their hand. That player discards all nonland cards with the same name as another card in their hand.",
        );
    assert!(
        result.is_err(),
        "unsupported same-name-as-another-in-hand discard clause should fail parse instead of discarding entire hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_rejects_for_each_mana_from_spent_clause() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Cataclysmic Prospecting Variant")
        .parse_text(
            "For each mana from a Desert spent to cast this spell, create a tapped Treasure token.",
        );
    assert!(
        result.is_err(),
        "unsupported for-each-mana-from-spent clause should fail parse instead of iterating over spells"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_labeled_trigger_line_as_triggered_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Heroic Label Variant")
            .parse_text(
                "Heroic — Whenever you cast a spell that targets this creature, put a +1/+1 counter on this creature, then scry 1.",
            )
            .expect("parse heroic labeled trigger");

    assert!(
        def.spell_effect.is_none(),
        "labeled trigger should not collapse into spell-effect text"
    );
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability from labeled trigger line");
    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("PutCountersEffect") && effects_debug.contains("ScryEffect"),
        "expected +1/+1 counter and scry effects in heroic trigger, got {effects_debug}"
    );
    assert_eq!(
        triggered
            .presentation_label
            .as_ref()
            .and_then(ability::PresentationLabel::display_prefix)
            .as_deref(),
        Some("Heroic"),
        "expected labeled trigger provenance to be stored on the trigger"
    );
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def);
    assert!(
        rendered
            .iter()
            .any(|line| line.starts_with("Heroic — Whenever you cast a spell")),
        "expected labeled trigger to render from structured provenance, got {rendered:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn keyword_list_followed_by_trigger_does_not_gain_label_dash() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Atraxa Shape Variant")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Phyrexian, Subtype::Angel])
            .power_toughness(PowerToughness::fixed(4, 4))
            .parse_text("Flying, vigilance, deathtouch, lifelink\nAt the beginning of your end step, proliferate.")
            .expect("parse keyword list plus trigger");

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def);
    assert!(
        rendered.iter().any(|line| {
            line == "Flying, vigilance, deathtouch, lifelink"
                || line == "Flying, deathtouch, lifelink, vigilance"
        }),
        "expected keyword list to remain its own line, got {rendered:?}"
    );
    assert!(
        !rendered.iter().any(|line| line.contains("lifelink — At")),
        "keyword list should not be reinterpreted as a presentation label, got {rendered:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_heroic_trigger_with_short_source_name_preserves_targets() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Anthousa, Setessan Hero")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Heroic — Whenever you cast a spell that targets Anthousa, up to three target lands you control each become 2/2 Warrior creatures until end of turn. They're still lands.",
            )
            .expect("parse heroic trigger with short source name");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability from heroic line");

    let spell_cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::spell_ability::SpellCastTrigger>()
        .expect("expected heroic to compile as a spell-cast trigger");
    let spell_filter = spell_cast
        .filter
        .as_ref()
        .expect("heroic trigger should filter spells that target the source");
    let target_filter = spell_filter
        .targets_object
        .as_deref()
        .expect("spell filter should target an object");
    assert!(
        target_filter.source,
        "heroic trigger should target the source, got {spell_filter:?}"
    );

    let animate = triggered.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("expected land-animation continuous effect");
    assert_eq!(
        animate
            .target_spec
            .as_ref()
            .map(crate::target::ChooseSpec::count),
        Some(ChoiceCount::up_to(3)),
        "land-animation effect should target up to three lands"
    );
    let target_spec = animate
        .target_spec
        .as_ref()
        .map(crate::target::ChooseSpec::base);
    assert!(
        matches!(target_spec, Some(crate::target::ChooseSpec::Object(filter))
                if filter.card_types == vec![CardType::Land] && filter.controller == Some(PlayerFilter::You)),
        "land-animation effect should target lands you control, got {:?}",
        animate.target_spec
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn anthousa_animation_effect_turns_selected_lands_into_warrior_creatures() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Anthousa, Setessan Hero")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Heroic — Whenever you cast a spell that targets Anthousa, up to three target lands you control each become 2/2 Warrior creatures until end of turn. They're still lands.",
            )
            .expect("parse heroic trigger with short source name");
    let animate_effect = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                Some(triggered.effects.segments[0].default_effects[0].clone())
            }
            _ => None,
        })
        .expect("expected Anthousa animation effect");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = crate::ids::PlayerId::from_index(0);
    let anthousa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let first_land = game.create_object_from_definition(
        &crate::cards::definitions::basic_forest(),
        alice,
        Zone::Battlefield,
    );
    let second_land = game.create_object_from_definition(
        &crate::cards::definitions::basic_mountain(),
        alice,
        Zone::Battlefield,
    );
    let third_land = game.create_object_from_definition(
        &crate::cards::definitions::basic_island(),
        alice,
        Zone::Battlefield,
    );
    let unselected_land = game.create_object_from_definition(
        &crate::cards::definitions::basic_plains(),
        alice,
        Zone::Battlefield,
    );

    let mut ctx =
        crate::effects::ExecutionContext::new_default(anthousa_id, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(first_land),
            crate::effects::ResolvedTarget::Object(second_land),
            crate::effects::ResolvedTarget::Object(third_land),
        ]);
    crate::effects::execute_effect(&mut game, &animate_effect, &mut ctx)
        .expect("Anthousa animation effect should resolve");

    for land in [first_land, second_land, third_land] {
        assert!(
            game.current_has_card_type(land, CardType::Land),
            "animated land should remain a land"
        );
        assert!(
            game.current_has_card_type(land, CardType::Creature),
            "selected land should become a creature"
        );
        assert!(
            game.current_has_subtype(land, Subtype::Warrior),
            "selected land should become a Warrior"
        );
        assert_eq!(game.current_power(land), Some(2));
        assert_eq!(game.current_toughness(land), Some(2));
    }
    assert!(
        !game.current_has_card_type(unselected_land, CardType::Creature),
        "unselected lands should not be animated"
    );
    assert!(
        !game.current_has_subtype(unselected_land, Subtype::Warrior),
        "unselected lands should not gain Warrior"
    );
    assert_eq!(game.current_power(unselected_land), None);
    assert_eq!(game.current_toughness(unselected_land), None);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_labeled_trigger_line_preserves_once_each_turn_suffix() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reach Label Variant")
            .parse_text(
                "Reach\nThe Allagan Eye — Whenever another creature you control dies, draw a card. This ability triggers only once each turn.",
            )
            .expect("parse reach line plus labeled once-each-turn trigger");

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Static(_))),
        "expected the standalone Reach line to compile to a static ability"
    );
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability from labeled trigger line");
    assert!(
        matches!(
            triggered.intervening_if.as_ref(),
            Some(crate::ConditionExpr::MaxTimesEachTurn(1))
        ),
        "expected 'This ability triggers only once each turn' suffix to set an intervening-if cap"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_labeled_trigger_line_preserves_twice_each_turn_suffix() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nadu Label Variant")
            .parse_text(
                "The Allagan Eye — Whenever another creature you control dies, draw a card. This ability triggers only twice each turn.",
            )
            .expect("parse reach line plus labeled twice-each-turn trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability from labeled trigger line");
    assert!(
        !matches!(
            triggered.intervening_if.as_ref(),
            Some(crate::ConditionExpr::MaxTimesEachTurn(1))
        ),
        "expected 'This ability triggers only twice each turn' suffix not to set once-each-triggers"
    );
    assert!(
        matches!(
            triggered.intervening_if.as_ref(),
            Some(crate::ConditionExpr::MaxTimesEachTurn(2))
        ),
        "expected 'This ability triggers only twice each turn' to set a per-turn cap of 2"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_gain_control_clause_structurally() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exert Influence Variant")
            .parse_text(
                "Gain control of target creature if its power is less than or equal to the number of colors of mana spent to cast this spell.",
            )
            .expect("conditional gain-control clause should parse structurally");
    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("ConditionalEffect")
            && debug.contains("ChangeControllerToEffectController")
            && debug.contains("ColorsOfManaSpentToCastThisSpell"),
        "expected conditional gain-control lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_commander_creatures_have_granted_cost_reduction() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Acolyte of Bahamut Variant")
            .parse_text(
                "Commander creatures you own have \"The first Dragon spell you cast each turn costs {2} less to cast.\"",
            )
            .expect_err("unsupported first-spell-each-turn granted cost reduction should fail");
    let joined = format!("{err:?}").to_ascii_lowercase();
    assert!(
        joined.contains("unsupported first-spell-each-turn cost modifier"),
        "expected strict first-spell-each-turn rejection, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_reveal_targets_hand_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Peek Variant")
        .parse_text("Target player reveals their hand.")
        .expect("parse reveal hand");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<LookAtHandEffect>().is_some()),
        "should include look-at-hand effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_surveil_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Surveil Card")
        .parse_text("Surveil 1.")
        .expect("parse surveil");

    let effects = def.spell_effect.expect("spell effect");
    assert!(
        effects
            .iter()
            .any(|e| e.downcast_ref::<SurveilEffect>().is_some()),
        "should include surveil effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_transform_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Werewolf Shift")
        .parse_text("Transform this creature.")
        .expect("parse transform");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("TransformEffect"),
        "should include transform effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_convert_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Autobot Shift")
        .parse_text("Convert this creature.")
        .expect("parse convert");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ConvertEffect"),
        "should include convert effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_meld_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Meld Variant")
            .parse_text(
                "If you both own and control this creature and a creature named Midnight Scavengers, exile them, then meld them into Chittering Host. It enters tapped and attacking.",
            )
            .expect("parse meld");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("MeldEffect"),
        "should include meld effect, got {debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("meld them into chittering host")
            && rendered.contains("it enters tapped and attacking"),
        "expected meld rendering with combat followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_populate_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wake Variant")
        .parse_text("Populate.")
        .expect("parse populate");

    assert!(
        crate::compiled_text::unprocessed_compiled_lines(&def)
            .join(" ")
            .contains("Populate"),
        "expected compiled text to retain populate"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_populate_x_times_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flowering Variant")
        .parse_text("Populate X times.")
        .expect("parse populate x times");

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Populate X times"),
        "expected populate x rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_monstrosity_static_designation_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fleecemane Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{3}{G}{W}: Monstrosity 1.\nAs long as this creature is monstrous, it has hexproof and indestructible.",
            )
            .expect("parse monstrous static designation");

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("monstrosity 1")
            && rendered.contains("as long as this creature is monstrous"),
        "expected compiled text to preserve monstrous condition, got {rendered}"
    );

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("SourceIsMonstrous")
            && abilities_debug.contains("Hexproof")
            && abilities_debug.contains("Indestructible"),
        "expected monstrous-conditioned hexproof and indestructible grants, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_monstrosity_trigger_with_up_to_x_targets_from_text() {
    use crate::ability::AbilityKind;

    let def = CardDefinitionBuilder::new(CardId::new(), "Vitality Variant")
            .parse_text(
                "{X}{W}{W}: Monstrosity X.\nWhen this creature becomes monstrous, put a lifelink counter on each of up to X target creatures.",
            )
            .expect("parse monstrosity x trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected becomes monstrous trigger");

    assert!(
        triggered.trigger.display().contains("becomes monstrous"),
        "expected becomes monstrous trigger, got {}",
        triggered.trigger.display()
    );
    assert!(
        triggered
            .choices
            .first()
            .is_some_and(|choice| choice.count().is_up_to_dynamic_x()),
        "expected trigger to carry an up-to-X target choice, got {:?}",
        triggered.choices
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_populate_created_this_way_followups_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Iteration Variant")
            .parse_text(
                "At the beginning of combat on your turn, populate. The token created this way gains haste. Sacrifice it at the beginning of the next end step.",
            )
            .expect("parse populate followups");

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("populate")
            && rendered.contains("gains haste")
            && rendered.contains("sacrifice it at the beginning of the next end step"),
        "expected populate followup rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_populate_enters_tapped_and_attacking_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghired Variant")
        .parse_text(
            "Whenever this creature attacks, populate. The token enters tapped and attacking.",
        )
        .expect("parse populate enters attacking followup");

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("populate") && rendered.contains("enters tapped and attacking"),
        "expected populate combat followup rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_detain_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Azorius Arrest Variant")
        .parse_text("Detain target creature an opponent controls.")
        .expect("parse detain");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("DetainEffect"),
        "should include detain effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_detain_each_nonland_permanent_from_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lavinia Variant")
        .parse_text(
            "Detain each nonland permanent your opponents control with mana value 4 or less.",
        )
        .expect("parse detain each");

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("DetainEffect"),
        "should include detain effect, got {debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Detain all nonland permanents with mana value 4 or less your opponents control"
        ),
        "expected detain each rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_activated_gets_dynamic_minus_x_plus_x() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Belbe's Armor Variant")
        .parse_text("{X}, {T}: Target creature gets -X/+X until end of turn.")
        .expect("activated dynamic gets should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("X"),
        "expected dynamic X modifier in rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_targeted_gets_where_x_is_number_of_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Where X Gets Variant")
            .parse_text("Target creature gets +X/+X until end of turn, where X is the number of creatures you control.")
            .expect("where-X targeted gets should parse");

    let effects = def.spell_effect.expect("spell effect");
    let debug = format!("{effects:?}").to_ascii_lowercase();
    assert!(
        debug.contains("applycontinuouseffect"),
        "expected targeted pump effect, got {debug}"
    );
    assert!(
        debug.contains("modifypowertoughness")
            && debug.contains("count(objectfilter")
            && debug.contains("controller: some(you)")
            && debug.contains("card_types: [creature]"),
        "expected where-X to compile into a creature-count value, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_gets_where_x_supports_signed_dynamic_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Signed Where X Variant")
            .parse_text(
                "Each non-Vampire creature gets -X/-X until end of turn, where X is the number of Vampires you control.",
            )
            .expect("signed dynamic where-X should parse with signed runtime replacement");
    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("scaled(")
            && debug.contains("count(objectfilter")
            && debug.contains("vampire"),
        "expected signed where-X replacement in parsed effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_metalcraft_self_buff_preserves_condition_and_subject() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ardent Recruit Variant")
        .parse_text(
            "Metalcraft — This creature gets +2/+2 as long as you control three or more artifacts.",
        )
        .expect("parse metalcraft static buff");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display.contains("this creature gets +2/+2"),
        "expected source-scoped anthem display, got: {display}"
    );
    assert!(
        display.contains("as long as you control three or more artifacts"),
        "expected condition to be preserved, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_domain_self_buff_preserves_for_each_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kavu Scout Variant")
        .parse_text(
            "Domain — This creature gets +1/+0 for each basic land type among lands you control.",
        )
        .expect("parse domain static buff");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display
            .contains("this creature gets +1/+0 for each basic land type among lands you control"),
        "expected dynamic domain display, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_descend_condition_keeps_permanent_cards_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Basking Capybara Variant")
            .parse_text(
                "Descend 4 — This creature gets +3/+0 as long as there are four or more permanent cards in your graveyard.",
            )
            .expect("parse descend static buff");

    assert_eq!(def.abilities.len(), 1, "expected one static ability");
    let display = match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        other => panic!("expected static ability, got {other:?}"),
    };
    assert!(
        display.contains("as long as there are four or more permanent cards in your graveyard"),
        "expected descend condition text to be preserved, got: {display}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_anthem_and_keyword_applies_condition_to_both() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Grant Variant")
        .parse_text(
            "As long as you control an artifact, this creature gets +1/+1 and has deathtouch.",
        )
        .expect("parse conditional anthem and keyword");

    assert_eq!(def.abilities.len(), 2, "expected two static abilities");
    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays
            .iter()
            .any(|display| display.contains("this creature gets +1/+1")
                && display.contains("as long as you control an artifact")),
        "expected conditional self buff ability, got: {displays:?}"
    );
    assert!(
        displays
            .iter()
            .any(|display| display.contains("has deathtouch")
                && display.contains("as long as you control an artifact")),
        "expected conditional grant ability, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_anthem_and_haste_keeps_pump_and_keyword() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Haste Variant")
            .parse_text(
                "As long as you control another multicolored permanent, this creature gets +1/+1 and has haste.",
            )
            .expect("parse conditional anthem and haste");

    assert_eq!(def.abilities.len(), 2, "expected two static abilities");
    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    assert!(
        displays.iter().any(|display| {
            display.contains("this creature gets +1/+1")
                && display.contains("as long as you control another multicolored permanent")
        }),
        "expected conditional self buff ability, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| {
            display.contains("has haste")
                && display.contains("as long as you control another multicolored permanent")
        }),
        "expected conditional haste ability, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_natures_embrace_creature_or_land_static_conditions() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nature's Embrace")
            .parse_text(
                "Enchant creature or land\nAs long as enchanted permanent is a creature, it gets +2/+2.\nAs long as enchanted permanent is a land, it has \"{T}: Add two mana of any one color.\"",
            )
            .expect("Nature's Embrace should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();

    assert!(
        displays.iter().any(|display| {
            display.contains("enchanted permanent gets +2/+2")
                && display.contains("as long as enchanted permanent is a creature")
        }),
        "expected creature branch with +2/+2, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| {
            display.contains("as long as enchanted permanent is a land")
                && (display.to_ascii_lowercase().contains("add two mana")
                    || display.contains("{T}: Add"))
        }),
        "expected land branch granting mana ability, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_attached_anthem_keyword_and_activated_grant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Careful Cultivation Variant")
            .parse_text(
                "Enchant artifact or creature.\nAs long as enchanted permanent is a creature, it gets +1/+3 and has reach and \"{T}: Add {G}{G}.\"",
            )
            .expect("conditional attached anthem + keyword + activated grant should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();

    assert!(
        displays.iter().any(|display| {
            display.contains("enchanted permanent gets +1/+3")
                && display.contains("as long as enchanted permanent is a creature")
        }),
        "expected conditional attached anthem, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| {
            display.contains("has reach")
                && display.contains("as long as enchanted permanent is a creature")
        }),
        "expected conditional attached reach grant, got: {displays:?}"
    );
    assert!(
        displays
            .iter()
            .any(|display| { display.contains("t add g g") || display.contains("add {G}{G}") }),
        "expected conditional attached activated mana grant, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_clawing_torment_attached_pronoun_as_attached_permanent() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Clawing Torment Variant")
            .parse_text(
                "Enchant artifact or creature\nAs long as enchanted permanent is a creature, it gets -1/-1 and can't block.\nEnchanted permanent has \"At the beginning of your upkeep, you lose 1 life.\"",
            )
            .expect("clawing torment should parse");

    let displays: Vec<String> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();

    assert!(
        displays.iter().any(|display| {
            display.contains("enchanted permanent gets -1/-1")
                && display.contains("as long as enchanted permanent is a creature")
        }),
        "expected attached permanent anthem text, got: {displays:?}"
    );
    assert!(
        displays.iter().any(|display| display
            .to_ascii_lowercase()
            .contains("enchanted permanent can't block")),
        "expected attached permanent cant-block text, got: {displays:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_conditional_attached_anthem_and_loses_keyword() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Short Circuit Variant")
            .parse_text(
                "Enchant artifact or creature\nFlash\nAs long as enchanted permanent is a creature, it gets -3/-0 and loses flying.",
            )
            .expect("conditional attached anthem + loses keyword should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("GrantAbility")
            && abilities_debug.contains("RemoveAbilityForFilter")
            && abilities_debug.contains("Flying"),
        "expected conditional lose-flying static effect, got: {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("EnchantedPermanentIsCreature"),
        "expected conditional gating on enchanted permanent creature type, got: {abilities_debug}"
    );
}
