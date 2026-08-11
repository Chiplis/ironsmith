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
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
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
pub(super) fn render_daze_style_alternative_cost_clause_is_humanized() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Daze Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You may return an Island you control to its owner's hand rather than pay this spell's mana cost.\nCounter target spell unless its controller pays {1}.",
        )
        .expect("parse daze-style alternative cost");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains(
            "You may return an Island you control to its owner's hand rather than pay this spell's mana cost"
        ) || joined.contains(
            "You may return a Island you control to its owner's hand rather than pay this spell's mana cost"
        ),
        "expected normalized daze-style alternative cost wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_eldrazi_token_creation_drops_under_your_control_phrase() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scion Caller")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 colorless Eldrazi Scion creature token with \"Sacrifice this creature: Add {C}.\"",
        )
        .expect("parse eldrazi scion creation clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        !joined.contains("under your control"),
        "expected eldrazi token text without explicit control suffix, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conditional_create_token_with_quoted_comma_uses_first_comma_split() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Containment Breach")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target artifact or enchantment. If its mana value is 2 or less, create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\"",
        )
        .expect("conditional token clause with quoted comma should parse");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("if it matches permanent with mana value 2 or less")
            || lower.contains(
                "if the tagged object 'destroyed_0' matches permanent with mana value 2 or less"
            )
            || lower.contains("if its mana value is 2 or less"),
        "expected mana value predicate to stay on destroyed target, got {joined}"
    );
    assert!(
        lower.contains("create a 1/1 black and green pest creature token"),
        "expected pest token creation in conditional true branch, got {joined}"
    );
    assert!(
        lower.contains("when this token dies, you gain 1 life"),
        "expected pest dies trigger text to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fading_hope_uses_past_tense_mana_value_predicate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fading Hope")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return target creature to its owner's hand. If its mana value was 3 or less, scry 1.",
        )
        .expect("Fading Hope-style past-tense mana-value predicate should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return target creature to its owner's hand"),
        "expected bounce clause to survive rendering, got {rendered}"
    );
    assert!(
        rendered.contains("if its mana value was 3 or less, scry 1")
            || rendered.contains("if that creature's mana value was 3 or less, scry 1"),
        "expected oracle-like past-tense mana-value wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fatal_push_revolt_clause_keeps_permanent_left_gate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fatal Push Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
        )
        .expect("fatal push revolt clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PermanentLeftBattlefieldUnderYourControlThisTurn"),
        "expected revolt gate to compile into a permanent-left condition, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("mana value 4 or less") || rendered.contains("mana value is 4 or less"),
        "expected revolt branch to preserve the mana value 4 threshold, got {rendered}"
    );
    assert!(
        rendered.contains("mana value 2 or less") || rendered.contains("mana value is 2 or less"),
        "expected base branch to preserve the mana value 2 threshold, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_prohibit_kicked_counter_spell_mana_value_replacement() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prohibit Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {2}\nCounter target spell if its mana value is 2 or less. If this spell was kicked, counter that spell if its mana value is 4 or less instead.",
        )
        .expect("Prohibit-style kicked counter replacement should parse");

    fn tagged_mana_value_at_most(condition: &crate::effect::Condition, expected: i32) -> bool {
        matches!(
            condition,
            crate::effect::Condition::TaggedObjectMatches(_, filter)
                if matches!(
                    filter.mana_value.as_ref(),
                    Some(crate::target::Comparison::LessThanOrEqual(value)) if *value == expected
                )
        )
    }

    let program = def.spell_effect.as_ref().expect("spell effect");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one resolution segment, got {program:#?}");
    };
    let has_base_gate = segment.default_effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .is_some_and(|conditional| tagged_mana_value_at_most(&conditional.condition, 2))
    });
    let has_kicked_gate = segment.self_replacements.iter().any(|branch| {
        branch.condition == crate::effect::Condition::ThisSpellWasKicked
            && branch.replacement_effects.iter().any(|effect| {
                effect
                    .downcast_ref::<crate::effects::ConditionalEffect>()
                    .is_some_and(|conditional| tagged_mana_value_at_most(&conditional.condition, 4))
            })
    });

    assert!(
        has_base_gate && has_kicked_gate,
        "expected Prohibit-style kicked counter replacement to preserve base and kicked mana-value gates, got {program:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fatal_push_exposes_single_target_requirement() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fatal Push Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature if it has mana value 2 or less.\nRevolt — Destroy that creature if it has mana value 4 or less instead if a permanent left the battlefield under your control this turn.",
        )
        .expect("fatal push should parse");

    let Some(effects) = def.spell_effect.as_ref() else {
        panic!("fatal push should compile spell effects");
    };

    fn is_cast_time_target_prelude(effect: &crate::effect::Effect) -> bool {
        if effect.downcast_ref::<TargetOnlyEffect>().is_some() {
            return true;
        }
        if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
            return is_cast_time_target_prelude(&tagged.effect);
        }
        false
    }

    let targeting_requirements = effects
        .iter()
        .filter(|effect| is_cast_time_target_prelude(effect))
        .count();

    assert_eq!(
        targeting_requirements, 1,
        "Fatal Push should require exactly one declared target when casting",
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conditional_type_list_predicate_uses_rightmost_comma_split() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gate to the Aether")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of each player's upkeep, that player reveals the top card of their library. If it's an artifact, creature, enchantment, or land card, the player may put it onto the battlefield.",
        )
        .expect("type-list conditional predicate should parse");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("if it's an artifact, creature, enchantment, or land card")
            || lower.contains("matches artifact or creature or enchantment or land"),
        "expected full type-list predicate in conditional, got {joined}"
    );
    assert!(
        lower.contains("that player may put it onto the battlefield"),
        "expected true branch to keep put-it effect, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn matter_reshaper_parses_and_renders_conditional_may_otherwise() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Matter Reshaper")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Colorless],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "({C} represents colorless mana.)\n\
             When this creature dies, reveal the top card of your library. You may put that card onto the battlefield if it's a permanent card with mana value 3 or less. Otherwise, put that card into your hand.",
        )
        .expect("Matter Reshaper should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("when this creature dies, reveal the top card of your library"),
        "expected Matter Reshaper death trigger to render, got {rendered}"
    );
    assert!(
        lower.contains(
            "you may put that card onto the battlefield if it's a permanent card with mana value 3 or less"
        ) || lower.contains(
            "you may put it onto the battlefield if a permanent card with mana value 3 or less was revealed this way"
        ),
        "expected conditional may battlefield clause, got {rendered}"
    );
    assert!(
        lower.contains("otherwise, put that card into your hand")
            || lower.contains("otherwise, put it into your hand"),
        "expected otherwise hand clause, got {rendered}"
    );
    assert!(
        !lower.contains("you may if"),
        "Matter Reshaper should not render malformed conditional permission text: {rendered}"
    );
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("LessThanOrEqual") && debug.contains("mana_value: Some"),
        "Matter Reshaper should retain the mana-value bound structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_unless_where_x_fails_strictly() {
    let result = CardDefinitionBuilder::new(CardId::from_raw(1), "Rethink Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {X}, where X is its mana value.",
        );
    match result {
        Ok(def) => {
            let joined = unprocessed_compiled_lines(&def)
                .join(" ")
                .to_ascii_lowercase();
            assert!(
                joined.contains("counter target spell unless") && joined.contains("pays {x}"),
                "expected counter-unless rendering when parse succeeds, got {joined}"
            );
        }
        Err(err) => {
            let joined = format!("{err:?}");
            assert!(
                joined.contains("unsupported where-x clause")
                    || joined.contains("unsupported trailing counter-unless payment clause"),
                "expected where-x strict parse error, got {joined}"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cephalid_shrine_binds_same_name_graveyard_where_x_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cephalid Shrine")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a player casts a spell, counter that spell unless that player pays {X}, where X is the number of cards in all graveyards with the same name as the spell.",
        )
        .expect("Cephalid Shrine should bind its where-X payment clause");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter it unless that object's controller pays {x}, where x is the number of cards in all graveyards with the same name as that object")
            || rendered.contains("counter it unless that object's controller pays {x}, where x is the number of cards with the same name as that object in all graveyards")
            || rendered.contains("counter it unless they pay {x}, where x is the number of cards with the same name as that object in all graveyards")
            || rendered.contains("counter that spell unless that player pays {x}, where x is the number of cards in all graveyards with the same name as the spell")
            || rendered.contains("counter that spell unless that player pays {x}, where x is the number of cards with the same name as that spell in all graveyards"),
        "expected Cephalid Shrine to preserve the bound X same-name graveyard clause, got {rendered}"
    );
    assert!(
        !rendered.contains("plus an additional"),
        "Cephalid Shrine should bind X directly, not as an additive payment clause, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("spellcasttrigger")
            && debug.contains("unlesspayseffect")
            && debug.contains("x_value: some(")
            && debug.contains("count(")
            && debug.contains("samenameastagged")
            && debug.contains("graveyard")
            && !debug.contains("additional_generic: some"),
        "expected Cephalid Shrine lowering to keep a same-name graveyard count in x_value, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_x_plus_life_with_where_clause_binds_x_value() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "An-Havva Inn Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You gain X plus 1 life, where X is the number of green creatures on the battlefield.",
        )
        .expect("gain-x-plus-life with where clause should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("number of green creatures")
            && (joined.contains("plus 1 life")
                || joined.contains(
                    "life equal to the number of green creatures on the battlefield plus 1"
                )),
        "expected where-x binding to remain in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_colors_of_mana_spent_binds_spell_effect_values() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Painful Truths")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Converge — You draw X cards and lose X life, where X is the number of colors of mana spent to cast this spell.",
        )
        .expect("colors-of-mana where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("where x is the number of colors of mana spent to cast this spell"),
        "expected rendered where-X colors-of-mana clause, got {rendered}"
    );
    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("colorsofmanaspenttocastthisspell"),
        "expected X to bind to colors of mana spent, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_this_ability_resolved_count_binds_activated_effect_value() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bronze Cudgels")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "{2}: Until end of turn, equipped creature gets +X/+0, where X is the number of times this ability has resolved this turn.\nEquip {1}",
        )
        .expect("ability-resolution-count where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("equipped creature gets +x/+0")
            && rendered
                .contains("where x is the number of times this ability has resolved this turn"),
        "expected rendered ability-resolution-count where-X clause, got {rendered}"
    );
    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("thisabilityresolvedthisturncount"),
        "expected X to bind to this ability's resolution count, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_revealed_card_mana_value_uses_public_revealed_cost_tag() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Disaster Radius")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, reveal a creature card from your hand.\nDisaster Radius deals X damage to each creature your opponents control, where X is the revealed card's mana value.",
        )
        .expect("revealed-card where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("where x is the revealed card's mana value"),
        "expected rendered revealed-card where-X clause, got {rendered}"
    );
    let debug = format!("{:?}", def).to_ascii_lowercase();
    assert!(
        debug.contains("__public_revealed") && debug.contains("manavalueof"),
        "expected X to bind to public revealed-card mana value, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ancient_bronze_dragon_where_x_result_clause_parses_strictly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ancient Bronze Dragon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elder, Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "Flying\nWhenever Ancient Bronze Dragon deals combat damage to a player, roll a d20. When you do, put X +1/+1 counters on each of up to two target creatures, where X is the result.",
        )
        .expect("Ancient Bronze Dragon should parse with where-X result binding");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("roll a d20")
            && (rendered
                .contains("put that many +1/+1 counters on each of up to two target creatures")
                || (rendered
                    .contains("put x +1/+1 counters on each of up to two target creatures")
                    && rendered.contains("where x is the result")))
            && rendered.contains("when you do"),
        "expected Ancient Bronze Dragon where-X result clause in compiled text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("rolldieeffect")
            && debug.contains("putcounterseffect")
            && debug.contains("reflexivetriggereffect")
            && debug.contains("effectvalue(effectid(0))"),
        "expected Ancient Bronze Dragon trigger to bind X to die result, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_fixed_plus_counters_on_source() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lightning Storm Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Lightning Storm deals X damage to any target, where X is 3 plus the number of charge counters on this.",
        )
        .expect("fixed-plus-counters where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("3 plus") && rendered.contains("charge counters on this"),
        "expected rendered fixed-plus-counters where-X clause, got {rendered}"
    );
    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("add(fixed(3)")
            && debug.contains("countersonsource(charge)")
            && debug.contains("charge"),
        "expected X to bind to 3 plus charge counters on source, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_number_of_counters_on_that_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Parting Thoughts Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy target creature. You draw X cards and you lose X life, where X is the number of counters on that creature.",
        )
        .expect("where-X counters on that creature clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "destroy target creature. draw x cards and you lose x life, where x is the number of counters on it"
        ) && !rendered.contains("number of creatures"),
        "expected rendered where-X clause to count counters on that creature once, got {rendered}"
    );
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("CountersOn")
            && (debug.contains("destroyed_0") || debug.contains("__it__"))
            && !debug.contains("Count(\n                                        ObjectFilter"),
        "expected X to bind to counters on the destroyed creature, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_prior_effect_first_power_binds_to_affected_object_metric() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Astarion's Thirst")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target creature. Put X +1/+1 counters on a commander creature you control, where X is the power of the creature exiled this way.",
        )
        .expect("prior-effect first-power where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "exile target creature. put x +1/+1 counters on a commander creature you control, where x is the power of the creature exiled this way"
        ),
        "expected the typed sentence boundary and affected-creature reference, got {rendered}"
    );
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("AffectedObjects")
            && debug.contains("FirstPower")
            && !debug.contains("PendingEffectMetric"),
        "expected X to bind to resolved affected-object first-power metric, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_real_card_counter_followup_sentence_boundaries_survive_lowering() {
    let cases = [
        (
            "Aggressive Negotiations",
            "Target opponent reveals their hand. You choose a nonland card from it and exile that card. Put a +1/+1 counter on up to one target creature you control.",
            "exile that card. put a +1/+1 counter on up to one target creature you control",
        ),
        (
            "Applied Geometry",
            "Create a token that's a copy of target non-Aura permanent you control, except it's a 0/0 Fractal creature in addition to its other types. Put six +1/+1 counters on it.",
            "other types. put six +1/+1 counters on it",
        ),
        (
            "Miraculous Recovery",
            "Return target creature card from your graveyard to the battlefield. Put a +1/+1 counter on it.",
            "return target creature card from your graveyard to the battlefield. put a +1/+1 counter on it",
        ),
    ];

    for (name, text, expected) in cases {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
        let rendered = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            rendered.contains(expected),
            "{name} should preserve its typed counter-followup sentence boundary: {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_life_total_difference_between_target_players() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Profane Transfusion")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Two target players exchange life totals. You create an X/X colorless Horror artifact creature token, where X is the difference between those players' life totals.",
        )
        .expect("target-player life-total-difference where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("difference between those players' life totals"),
        "expected rendered target-player life-total difference, got {rendered}"
    );
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ExchangeLifeTotalsEffect")
            && debug.contains("LifeTotalDifference")
            && debug.contains("Target(Any)"),
        "expected X to bind to target-player life-total difference, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_commander_mana_value_creates_runtime_choice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Stinging Study")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You draw X cards and you lose X life, where X is the mana value of a commander you own on the battlefield or in the command zone.",
        )
        .expect("commander mana-value where-X clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("commander")
            && rendered.contains("mana value")
            && rendered.contains("draw")
            && rendered.contains("lose"),
        "expected rendered commander mana-value draw/life clause, got {rendered}"
    );
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("__where_x_commander_mana_value")
            && debug.contains("additional_zones: [Command]")
            && debug.contains("ManaValueOf"),
        "expected X to bind to a chosen commander mana value, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_unless_plus_additional_keeps_dynamic_payment_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spell Stutter Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {2} plus an additional {1} for each Faerie you control.",
        )
        .expect("parse counter-unless-plus-additional clause");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("pays {2}")
            && joined.contains("plus an additional {1} for each faerie you control"),
        "expected dynamic additional payment clause to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_destroy_all_artifacts_and_enchantments_combines_split_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Purify Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy all artifacts and enchantments.")
        .expect("parse destroy-all artifacts-and-enchantments clause");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Destroy all artifacts and enchantments"),
        "expected combined destroy-all wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_activation_typed_discard_cost_keeps_card_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tortured Existence Variant")
        .parse_text("{B}, Discard a creature card: Return target creature card from your graveyard to your hand.")
        .expect("typed discard activation cost should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("discard a creature card"),
        "expected typed discard activation cost wording, got {joined}"
    );
}

pub(super) fn niambi_draw_activated_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| effect.downcast_ref::<DrawCardsEffect>().is_some()) =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Niambi, Esteemed Speaker should have a draw activated ability")
}

pub(super) fn niambi_enter_triggered_ability(
    def: &CardDefinition,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| {
                        effect.downcast_ref::<IfEffect>().is_some_and(|if_effect| {
                            if_effect
                                .then
                                .iter()
                                .any(|effect| effect.downcast_ref::<GainLifeEffect>().is_some())
                        })
                    }) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Niambi, Esteemed Speaker should have an enters triggered ability")
}

#[test]
pub(super) fn niambi_esteemed_speaker_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Niambi, Esteemed Speaker");

    let def = parse_oracle_card_definition("Niambi, Esteemed Speaker");
    let activated = niambi_draw_activated_ability(&def);
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let activated_debug = format!("{activated:#?}");

    assert!(
        rendered.contains("Discard a legendary card: Draw two cards"),
        "Niambi compiled text should preserve the legendary discard activation cost, got {rendered}"
    );
    assert!(
        rendered.contains(
            "When Niambi enters, you may return another target creature you control to its owner's hand. If you do, you gain life equal to that creature's mana value"
        ) || rendered.contains(
            "When Niambi enters, you may return another target creature you control to its owner's hand. If you do, you gain life equal to its mana value"
        ),
        "Niambi compiled text should preserve the named ETB and returned-creature mana-value reference, got {rendered}"
    );
    assert!(
        activated_debug.contains("DiscardEffect")
            && activated_debug.contains("supertypes: [")
            && activated_debug.contains("Legendary")
            && activated_debug.contains("DrawCardsEffect"),
        "Niambi activation should structurally require discarding a legendary card and draw two cards, got {activated_debug}"
    );
}

#[test]
pub(super) fn niambi_esteemed_speaker_activation_cost_requires_and_discards_legendary_card() {
    let def = parse_oracle_card_definition("Niambi, Esteemed Speaker");
    let activated = niambi_draw_activated_ability(&def);
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source);

    let nonlegendary_card = CardBuilder::new(CardId::new(), "Nonlegendary Test Card")
        .card_types(vec![CardType::Creature])
        .build();
    let nonlegendary_id = game.create_object_from_card(&nonlegendary_card, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::White, 2);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    assert!(
        crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost).is_err(),
        "Niambi activation should not be payable with only a nonlegendary card in hand"
    );

    let legendary_card = CardBuilder::new(CardId::new(), "Legendary Test Card")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .build();
    let legendary_id = game.create_object_from_card(&legendary_card, alice, Zone::Hand);
    let draw_one = CardBuilder::new(CardId::new(), "Niambi Draw One").build();
    let draw_two = CardBuilder::new(CardId::new(), "Niambi Draw Two").build();
    game.create_object_from_card(&draw_one, alice, Zone::Library);
    game.create_object_from_card(&draw_two, alice, Zone::Library);

    crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost)
        .expect("Niambi activation should be payable with a legendary card in hand");
    let mut dm = crate::decision::AutoPassDecisionMaker::default();
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("Niambi activation cost should select and discard the legendary card");

    let player = game.player(alice).expect("Alice should exist");
    assert!(game.is_tapped(source), "Niambi should tap to pay the cost");
    assert!(
        player.hand.contains(&nonlegendary_id),
        "nonlegendary hand card should not satisfy Niambi's legendary discard cost"
    );
    assert!(
        !player.hand.contains(&legendary_id),
        "legendary hand card should be discarded to pay Niambi's activation cost"
    );
    assert_eq!(player.graveyard.len(), 1, "one card should be discarded");

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Niambi activated ability should draw two cards after costs are paid");
    }
    let player = game.player(alice).expect("Alice should exist");
    assert_eq!(
        player.hand.len(),
        3,
        "after discarding one legendary card, Niambi should draw two cards"
    );
    assert_eq!(
        player.library.len(),
        0,
        "Niambi should draw both library cards"
    );
}

#[test]
pub(super) fn niambi_esteemed_speaker_enter_trigger_returns_another_creature_and_gains_life() {
    struct DeclineMay;
    impl crate::decision::DecisionMaker for DeclineMay {}

    let def = parse_oracle_card_definition("Niambi, Esteemed Speaker");
    let triggered = niambi_enter_triggered_ability(&def);
    let target_spec = triggered
        .choices
        .first()
        .expect("Niambi enters trigger should target another creature")
        .clone();
    let ChooseSpec::Target(inner) = target_spec.unhinted() else {
        panic!("Niambi enters trigger should use a target spec, got {target_spec:#?}");
    };
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        panic!("Niambi enters trigger should target an object, got {inner:#?}");
    };
    assert!(
        filter.other,
        "Niambi enters trigger must target another creature, not Niambi itself"
    );
    assert_eq!(
        filter.controller,
        Some(PlayerFilter::You),
        "Niambi enters trigger should only target a creature you control"
    );
    assert_eq!(
        filter.card_types,
        vec![CardType::Creature],
        "Niambi enters trigger should target a creature"
    );

    let alice = PlayerId::from_index(0);
    let returned_def = CardDefinitionBuilder::new(CardId::new(), "Niambi Returned Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let returned = game.create_object_from_definition(&returned_def, alice, Zone::Battlefield);
    let returned_stable_id = game
        .object(returned)
        .expect("returned creature should exist")
        .stable_id;
    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source).expect("Niambi should exist"),
        &game,
    );
    let entry_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(source_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(returned)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec.clone(),
            range: 0..1,
        }])
        .with_triggering_event(entry_event);
    ctx.snapshot_targets(&game);

    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Niambi enters trigger should return target and gain life");
    }

    let returned_hand_id = game
        .find_object_by_stable_id(returned_stable_id)
        .expect("returned creature should still exist after changing zones");
    let player = game.player(alice).expect("Alice should exist");
    assert!(
        player.hand.contains(&returned_hand_id),
        "accepted Niambi trigger should return the targeted creature to hand"
    );
    assert_eq!(
        game.life_total(alice),
        24,
        "Niambi should gain life equal to the returned creature's mana value"
    );

    let mut declined_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let declined_source =
        declined_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let declined_returned =
        declined_game.create_object_from_definition(&returned_def, alice, Zone::Battlefield);
    let declined_source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        declined_game
            .object(declined_source)
            .expect("Niambi should exist"),
        &declined_game,
    );
    let declined_entry_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            declined_source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(declined_source_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = DeclineMay;
    let mut declined_ctx = crate::effects::ExecutionContext::new_default(declined_source, alice)
        .with_decision_maker(&mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(
            declined_returned,
        )])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }])
        .with_triggering_event(declined_entry_event);
    declined_ctx.snapshot_targets(&declined_game);

    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut declined_game, effect, &mut declined_ctx)
            .expect("declined Niambi enters trigger should resolve without doing anything");
    }

    let declined_player = declined_game.player(alice).expect("Alice should exist");
    assert!(
        !declined_player.hand.contains(&declined_returned),
        "declining Niambi trigger should not return the targeted creature"
    );
    assert_eq!(
        declined_game.life_total(alice),
        20,
        "declining Niambi trigger should skip the conditional life gain"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_activation_discard_hand_cost_keeps_full_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Null Brooch Variant")
        .parse_text("{2}, {T}, Discard your hand: Counter target noncreature spell.")
        .expect("discard-hand activation cost should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("discard your hand"),
        "expected discard-your-hand activation cost wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_activation_random_discard_cost_keeps_random_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mage il-Vec Variant")
        .parse_text("{T}, Discard a card at random: This creature deals 1 damage to any target.")
        .expect("random discard activation cost should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("discard a card at random"),
        "expected random discard activation cost wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_activation_return_cost_preserves_numeric_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flooded Shoreline Variant")
        .parse_text("{U}{U}, Return two Islands you control to their owner's hand: Return target creature to its owner's hand.")
        .expect("counted return cost should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("return two islands you control to their owners' hands")
            || joined.contains("return two islands you control to their owner's hand"),
        "expected counted return activation cost wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shard_style_activation_cost_preserves_complete_alternative_branches() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pearl Shard Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{3}, {T} or {W}, {T}: Prevent the next 2 damage that would be dealt to any target this turn.",
        )
        .expect("shard-style mana-or-tap activation cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let branches = activated
        .mana_cost
        .as_one_of()
        .expect("shard-style cost should be a choice between complete costs");
    assert_eq!(branches.len(), 2);
    for (branch, expected_mana) in branches.iter().zip([
        vec![vec![ManaSymbol::Generic(3)]],
        vec![vec![ManaSymbol::White]],
    ]) {
        let costs = branch
            .as_all()
            .expect("each alternative should be conjunctive");
        let mana = branch
            .mana_cost()
            .expect("each shard branch should include a mana component");
        assert_eq!(mana.pips(), expected_mana);
        assert_eq!(
            costs.iter().filter(|cost| cost.requires_tap()).count(),
            1,
            "each complete branch should retain its own tap cost"
        );
    }
    assert!(
        activated.choices.iter().any(ChooseSpec::is_target),
        "the prevention recipient must remain a target, not a payment choice"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{3}, {T} or {W}, {T}: Prevent the next 2 damage"),
        "expected rendered shard-style cost to preserve complete alternatives, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_return_at_end_of_combat_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kaijin Variant")
        .parse_text("Return target creature to its owner's hand at end of combat.")
        .expect("delayed end-of-combat return should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("EndOfCombatTrigger"),
        "expected end-of-combat delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("ReturnToHandEffect"),
        "expected delayed return-to-hand payload, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("At this turn's next end of combat"),
        "expected rendered delayed end-of-combat timing, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conquerors_galleon_attack_trigger_delays_return_and_transform_at_end_of_combat()
{
    let def = CardDefinitionBuilder::new(
        CardId::from_raw(1),
        "Conqueror's Galleon // Conqueror's Foothold",
    )
    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![Subtype::Vehicle])
    .power_toughness(PowerToughness::fixed(2, 10))
    .parse_text(
        "When this Vehicle attacks, exile it at end of combat, then return it to the battlefield transformed under your control.\nCrew 4 (Tap any number of creatures you control with total power 4 or more: This Vehicle becomes an artifact creature until end of turn.)",
    )
    .expect("Conqueror's Galleon should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CrewCostEffect"),
        "expected crew ability to survive parsing, got {debug}"
    );
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("EndOfCombatTrigger"),
        "expected end-of-combat delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected delayed exile/return zone movement, got {debug}"
    );
    assert!(
        debug.contains("enters_transformed: true"),
        "expected transformed return payload, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_turns_next_end_of_combat_prefix_wraps_payload() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Triton Tactics Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("At this turn's next end of combat, tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step.")
        .expect("this-turn next-end-of-combat prefix should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect") && debug.contains("EndOfCombatTrigger"),
        "expected scheduled end-of-combat payload, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("At this turn's next end of combat"),
        "expected rendered next end-of-combat prefix, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_triton_tactics_combat_pronouns() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Triton Tactics")
        .card_types(vec![CardType::Instant])
        .parse_text("Up to two target creatures each get +0/+3 until end of turn. Untap those creatures. At this turn's next end of combat, tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step.")
        .expect("Triton Tactics should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Up to two target creatures each get +0/+3 until end of turn")
            && rendered.contains("Untap those creatures")
            && rendered.contains("tap each creature that was blocked by one of those creatures this turn and it doesn't untap"),
        "expected Triton Tactics combat pronouns to render cleanly, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_return_at_next_end_step_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flicker Variant")
        .parse_text(
            "Exile target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .expect("next-end-step return should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfEndStepTrigger"),
        "expected next-end-step delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect"),
        "expected delayed return-to-battlefield payload, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_source_pronoun_transformed_return_uses_object_motion_not_player_return() {
    let def = CardDefinitionBuilder::new(
        CardId::from_raw(1),
        "Sorin of House Markov // Sorin, Ravenous Neonate",
    )
    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Black]]))
    .card_types(vec![CardType::Creature])
    .subtypes(vec![Subtype::Human, Subtype::Noble])
    .power_toughness(PowerToughness::fixed(1, 4))
    .parse_text(
        "Lifelink\nExtort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)\nAt the beginning of each of your postcombat main phases, if you gained 3 or more life this turn, exile Sorin, then return him to the battlefield transformed under his owner's control.",
    )
    .expect("sorin transform line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "At the beginning of each of your postcombat main phases, if you gained 3 or more life this turn, exile Sorin, then return him to the battlefield transformed under his owner's control"
        ),
        "expected the typed phase and source-pronoun surfaces, got {rendered}"
    );
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Exile")
            && debug.contains("zone: Battlefield")
            && debug.contains("enters_transformed: true"),
        "expected blink-style transformed return payload, got {debug}"
    );
    assert!(
        !debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            && !debug.contains("target: Player(IteratedPlayer)"),
        "expected source pronoun to avoid player/graveyard lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_return_at_your_next_upkeep_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Upkeep Return Variant")
        .parse_text(
            "Return target creature to its owner's hand at the beginning of your next upkeep.",
        )
        .expect("next-upkeep return should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfUpkeepTrigger"),
        "expected beginning-of-upkeep delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("start_next_turn: true"),
        "expected next-turn gate for next-upkeep trigger, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_destroy_at_end_of_combat_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Basilisk Variant")
        .parse_text("Destroy target creature at end of combat.")
        .expect("delayed destroy at end of combat should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("EndOfCombatTrigger"),
        "expected end-of-combat delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected delayed destroy payload, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_control_at_end_of_combat_schedules_delayed_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tolarian Entrancer Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature becomes blocked by a creature, gain control of that creature at end of combat.",
        )
        .expect("delayed gain-control trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect") && debug.contains("EndOfCombatTrigger"),
        "expected delayed end-of-combat control payload, got {debug}"
    );
    assert!(
        debug.contains("ChangeControllerToEffectController"),
        "expected delayed payload to gain control of the blocker, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gain control of that creature at end of combat"),
        "expected rendered delayed gain-control text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_destroy_at_next_end_step_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bearer Variant")
        .parse_text("Destroy all permanents at the beginning of the next end step.")
        .expect("delayed destroy at next end step should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfEndStepTrigger"),
        "expected next-end-step delayed trigger, got {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected delayed destroy payload, got {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec!["Destroy all permanents at the beginning of the next end step.".to_string()]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exchange_control_keeps_first_target_before_missing_target_prelude() {
    let oracle = "Exchange control of target artifact or creature and another target permanent that shares one of those types with it.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Legerdemain Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("parse heterogeneous exchange control");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("ExchangeControlEffect"),
        "expected exchange control effect, got {debug}"
    );
    assert!(
        !debug.contains("TargetOnlyEffect"),
        "expected heterogeneous exchange control to expose its own ordered target requirements without target-only preludes, got {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![oracle.to_string()],
        "the typed relative shared-type exchange should retain its exact singular relation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn phyrexian_rebirth_keeps_its_destroyed_result_set_in_the_dynamic_token() {
    let oracle = "Destroy all creatures, then create an X/X colorless Phyrexian Horror artifact creature token, where X is the number of creatures destroyed this way.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rebirth Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("parse destroyed-result dynamic token");
    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));

    assert!(
        debug.contains("DestroyEffect")
            && debug.contains("CreateTokenEffect")
            && debug.contains("SetBasePowerToughnessEffect")
            && debug.contains("PriorEffectMetric")
            && debug.contains("source: AffectedObjects")
            && debug.contains("action: Some(Destroyed)"),
        "the token size must consume the exact destroyed-object result set: {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![oracle.to_string()],
        "the typed result-set bundle should preserve the comma-then X/X token surface"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lifestreams_blessing_preserves_cast_time_power_and_x_backreference() {
    let oracle = "Draw X cards, where X is the greatest power among creatures you controlled as you cast this spell. If this spell was cast from exile, you gain twice X life.\nForetell {4}{G}";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lifestream Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("parse cast-time aggregate and later X backreference");
    let debug = format!("{:#?}", def.spell_effect.as_ref().expect("spell effects"));

    assert!(
        debug.contains(ironsmith_core::CAST_CONTROLLED_OBJECTS_TAG)
            && debug.contains("GreatestPower")
            && debug.contains("ThisSpellWasCastFromZone"),
        "the aggregate must use a cast-time snapshot and remain linked to the exile condition: {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        oracle.lines().map(str::to_string).collect::<Vec<_>>(),
        "the typed aggregate and scaled backreference should retain their exact surfaces"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shadows_verdict_preserves_battlefield_and_graveyard_domains() {
    let oracle = "Exile all creatures and planeswalkers with mana value 3 or less from the battlefield and all creature and planeswalker cards with mana value 3 or less from all graveyards.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Verdict Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("parse shared-characteristic dual-zone exile");
    let debug = format!("{:#?}", def.spell_effect.as_ref().expect("spell effects"));

    assert!(
        debug.contains("ExileEffect")
            && debug.contains("Battlefield")
            && debug.contains("Graveyard")
            && debug.contains("LessThanOrEqual"),
        "both zone branches and the shared mana-value restriction must survive: {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![oracle.to_string()],
        "the semantically shared filter should retain both explicit domains"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_arcbond_delayed_trigger_without_unsupported_fallback_in_allow_unsupported_mode()
{
    let parsed = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcbond Variant")
        .card_types(vec![CardType::Instant])
        .parse_text_allow_unsupported(
            "Choose target creature. Whenever that creature is dealt damage this turn, it deals that much damage to each other creature and each player.",
        );

    let def = parsed.expect("arcbond delayed trigger should parse");
    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        !abilities_debug.contains("UnsupportedParserLine"),
        "arcbond parse should not rely on unsupported fallback marker: {abilities_debug}"
    );

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("target_tag: Some"),
        "expected delayed trigger to track a tagged watched object, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("IsDealtDamageTrigger { target: Source"),
        "expected delayed trigger to watch damage dealt to the tagged object source, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn arcbond_delayed_trigger_deals_damage_to_each_other_creature_and_each_player() {
    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = crate::card::CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let obj =
            crate::object::Object::from_card(id, &card, controller, crate::zone::Zone::Battlefield);
        game.add_object(obj);
        id
    }

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcbond Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose target creature. Whenever that creature is dealt damage this turn, it deals that much damage to each other creature and each player.",
        )
        .expect("arcbond delayed trigger should parse");
    let spell_effects = def.spell_effect.clone().expect("spell effects");

    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let chosen_creature = create_creature(&mut game, "Chosen", alice);
    let other_creature_one = create_creature(&mut game, "Other One", bob);
    let other_creature_two = create_creature(&mut game, "Other Two", charlie);

    let spell_source = game.new_object_id();
    let mut ctx =
        crate::effects::ExecutionContext::new_default(spell_source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(chosen_creature),
        ]);
    for effect in &spell_effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("spell effect execution should succeed");
    }

    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "expected one delayed trigger"
    );
    assert_eq!(
        game.effect_store.delayed_triggers[0].target_objects,
        vec![chosen_creature],
        "expected delayed trigger watcher to be the chosen creature"
    );

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            other_creature_one,
            crate::events::DamageTarget::Object(chosen_creature),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let delayed_entries = crate::triggers::check_delayed_triggers(&mut game, &damage_event);
    assert_eq!(
        delayed_entries.len(),
        1,
        "expected arcbond delayed trigger to fire once"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in delayed_entries {
        trigger_queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put delayed trigger on stack");
    assert_eq!(game.stack.len(), 1, "expected delayed trigger on stack");

    crate::game_loop::resolve_stack_entry(&mut game).expect("resolve delayed trigger");

    assert_eq!(
        game.damage_on(other_creature_one),
        3,
        "first other creature should be dealt matching damage"
    );
    assert_eq!(
        game.damage_on(other_creature_two),
        3,
        "second other creature should be dealt matching damage"
    );
    assert_eq!(
        game.damage_on(chosen_creature),
        0,
        "chosen creature should not be in the 'each other creature' fanout"
    );

    assert_eq!(game.player(alice).expect("alice should exist").life, 17);
    assert_eq!(game.player(bob).expect("bob should exist").life, 17);
    assert_eq!(game.player(charlie).expect("charlie should exist").life, 17);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_unless_or_mana_choice_uses_total_cost_one_of() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thrull Wizard Variant")
        .parse_text("Counter target black spell unless that spell's controller pays {B} or {3}.")
        .expect("alternative mana unless-payment should parse as TotalCost::OneOf");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("pays {b} or {3}") || rendered.contains("pays {b} or pays {3}"),
        "expected rendered alternative payment clause, got {rendered}"
    );
    let debug = format!("{def:?}");
    assert!(
        debug.contains("OneOf"),
        "expected counter-unless payment to carry TotalCost::OneOf, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_it_unless_discard_creature_card_as_unless_action() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Body Snatcher Variant")
        .parse_text("When this creature enters, exile it unless you discard a creature card.")
        .expect("triggered unless-discard clause should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("UnlessActionEffect"),
        "expected unless-action lowering, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("DiscardEffect"),
        "expected discard alternative action, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("tag: TagKey(\"triggering\")"),
        "expected triggering-object tag for 'it', got {abilities_debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("unless you discard a creature card"),
        "expected unless-discard wording to render, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_named_count_filter_keeps_named_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Powerstone Shard Variant")
        .parse_text("{T}: Add {C} for each artifact you control named Powerstone Shard.")
        .expect("named count filter should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("named powerstone shard"),
        "expected named count filter wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_named_filter_preserves_articles_in_card_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cleric of the Forward Order Variant")
        .parse_text(
            "When this creature enters, you gain 2 life for each creature you control named Cleric of the Forward Order.",
        )
        .expect("named filter with article should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("named cleric of the forward order"),
        "expected named filter to keep articles in card name, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_nonsnow_filter_keeps_non_supertype() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hallowed Ground Variant")
        .parse_text("{W}{W}: Return target nonsnow land you control to its owner's hand.")
        .expect("nonsnow target filter should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("nonsnow land you control"),
        "expected nonsnow target filter wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_semantic_guard_is_disabled_by_default() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Semantic Guard Baseline Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying")
        .expect("semantic guard should be opt-in by env var");

    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains("Flying"),
        "expected parsed output while semantic guard is disabled, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shared_color_prevent_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Prevent Variant")
        .parse_text(
            "Prevent the next 1 damage that would be dealt to target creature and each other creature that shares a color with it this turn.",
        )
        .expect("shared-color prevent fanout should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent the next 1 damage") && rendered.contains("target creature"),
        "expected primary prevent target clause, got {rendered}"
    );
    assert!(
        rendered.contains("shares a color with that object")
            || !rendered.contains("unsupported parser line fallback"),
        "expected shared-color clause to avoid fallback rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shared_color_gain_ability_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Gain Variant")
        .parse_text(
            "Radiance — Target creature and each other creature that shares a color with it gain haste until end of turn.",
        )
        .expect("shared-color gain fanout should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("shares a color with it"),
        "expected shared-color fanout filter, got {rendered}"
    );
    assert!(
        rendered.contains("until end of turn") && rendered.contains("gain haste"),
        "expected haste grant to fanout targets, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shared_color_pump_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Pump Variant")
        .parse_text(
            "Radiance — Target creature and each other creature that shares a color with it get +1/+1 until end of turn.",
        )
        .expect("shared-color pump fanout should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("shares a color with it"),
        "expected shared-color fanout filter, got {rendered}"
    );
    assert!(
        rendered.contains("+1/+1"),
        "expected +1/+1 pump to be preserved, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shared_color_damage_with_named_subject_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiance Damage Variant")
        .parse_text(
            "Radiance — Cleansing Beam deals 2 damage to target creature and each other creature that shares a color with it.",
        )
        .expect("named-subject shared-color damage fanout should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("deal 2 damage to target creature"),
        "expected primary target damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("shares a color with that object")
            || rendered.contains("shares a color with it"),
        "expected shared-color fanout damage clause, got {rendered}"
    );
}

#[test]
pub(super) fn score_card_text_shadow_urchin_preserves_blight_and_counter_death_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shadow Urchin")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, blight 1.\n\
             Whenever a creature you control with one or more counters on it dies, exile that many cards from the top of your library. Until your next end step, you may play those cards.",
        )
        .expect("parse Shadow Urchin text");

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever this creature attacks, blight 1."),
        "expected blight keyword action to render, got {rendered}"
    );
    assert!(
        rendered.contains("Whenever a creature you control with a counter on it dies, exile that many cards from the top of your library")
            || (rendered.contains("Whenever a creature you control with counters on it dies, exile the top X cards of your library")
                && rendered.contains("where X is the number of counters on it")),
        "expected counter-death trigger to keep the counter count reference, got {rendered}"
    );
    assert!(
        rendered.contains("Until your next end step, you may play those cards")
            || rendered.contains("You may play those cards until your next end step"),
        "expected exile/play follow-up to compact around those cards, got {rendered}"
    );
}

#[test]
pub(super) fn score_card_text_incite_hysteria_compacts_radiance_quoted_ability_grant() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Incite Hysteria")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Radiance — Until end of turn, target creature and each other creature that shares a color with it gain \"This creature can't block.\"",
        )
        .expect("parse Incite Hysteria text");

    let rendered = compiled_text_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Radiance — Until end of turn, target creature and each other creature that shares a color with it gain \"This creature can't block.\""
    );
}

#[test]
pub(super) fn score_card_text_dark_supplicant_compacts_multi_zone_search_put_and_shuffle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dark Supplicant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Cleric])
        .parse_text(
            "{T}, Sacrifice three Clerics: Search your graveyard, hand, and/or library for a card named Scion of Darkness and put it onto the battlefield. If you search your library this way, shuffle.",
        )
        .expect("parse Dark Supplicant text");

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Search your graveyard, hand, and/or library for a card named Scion of Darkness and put it onto the battlefield"
        ),
        "expected multi-zone search and put to compact, got {rendered}"
    );
    assert!(
        rendered.contains("If you search your library this way, shuffle"),
        "expected searched-library conditional shuffle, got {rendered}"
    );
    assert!(
        !rendered.contains("For each card searched")
            && !rendered.contains("effect #")
            && !rendered.contains("You search your graveyard"),
        "expected no fallback search wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_unless_then_counter_that_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter That Spell Variant")
        .parse_text(
            "Counter target noncreature spell unless its controller pays {1}. If you control a creature with power 4 or greater, counter that spell instead.",
        )
        .expect("counter-that-spell follow-up should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target noncreature spell"),
        "expected base counter clause, got {rendered}"
    );
    assert!(
        rendered.contains("if you control a creature with power 4 or greater")
            && rendered.contains("instead counter that spell")
            && rendered.contains("unless its controller pays"),
        "expected conditional replacement to keep shared target semantics, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_cost_sacrificed_power_reference_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fling")
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nFling deals damage equal to the sacrificed creature's power to any target.",
        )
        .expect("sacrificed-power follow-up should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("as an additional cost to cast this spell, sacrifice a creature")
            && rendered.contains("deals damage equal to")
            && rendered.contains("power"),
        "expected additional-cost sacrificed-power linkage, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tormented_thoughts_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Tormented Thoughts");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("as an additional cost to cast this spell, sacrifice a creature"),
        "expected Tormented Thoughts sacrifice additional cost, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Target player discards a number of cards equal to the sacrificed creature's power"
        ),
        "expected Tormented Thoughts to render dynamic sacrificed-power discard count, got {rendered}"
    );

    let discard = def
        .spell_effect
        .as_ref()
        .expect("Tormented Thoughts should have spell effects")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::DiscardEffect>()
                .cloned()
        })
        .expect("Tormented Thoughts should lower to a discard effect");
    assert!(
        matches!(
            discard.count.unhinted(),
            crate::effect::Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == "sacrificed_0")
        ),
        "expected Tormented Thoughts discard count to reference the sacrificed creature cost tag, got {:?}",
        discard.count
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soulblast_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Soulblast");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains(
            "as an additional cost to cast this spell, sacrifice all creatures you control"
        ),
        "expected Soulblast all-creatures additional cost, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Soulblast deals damage to any target equal to the total power of the sacrificed creatures"
        ),
        "expected Soulblast damage to use sacrificed-creatures total power, got {rendered}"
    );

    let debug = format!("{:#?}", def);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("SacrificePlayerEffect")
            && debug.contains("EffectMetric")
            && debug.contains("TotalPower"),
        "expected Soulblast cost sacrifice to feed total-power damage metric, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn corpse_lunge_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Corpse Lunge");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains(
            "as an additional cost to cast this spell, exile a creature card from your graveyard"
        ),
        "expected Corpse Lunge additional exile cost, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Corpse Lunge deals damage equal to the exiled card's power to target creature"
        ),
        "expected Corpse Lunge to render exiled-card power damage, got {rendered}"
    );

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("DealDamageEffect")
            && spell_debug.contains(crate::tag::SOURCE_EXILED_TAG),
        "expected Corpse Lunge damage amount to be based on the exiled cost card, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_cost_discard_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Discard Cost Variant")
        .parse_text("As an additional cost to cast this spell, discard a card.\nDraw a card.")
        .expect("discard additional cost should parse through checked payment conversion");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("as an additional cost to cast this spell")
            && rendered.contains("discard a card")
            && rendered.contains("draw a card"),
        "expected discard additional cost and spell effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_cost_mixed_life_and_sacrifice_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mixed Cost Variant")
        .parse_text(
            "As an additional cost to cast this spell, pay 2 life and sacrifice a creature.\nDraw a card.",
        )
        .expect("mixed additional cost should parse through checked payment conversion");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("pay 2 life")
            && rendered.contains("sacrifice a creature")
            && rendered.contains("draw a card"),
        "expected mixed additional cost and spell effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_fumes_parses_with_exile_creature_additional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Necrotic Fumes")
        .parse_text(
            "As an additional cost to cast this spell, exile a creature you control.\nExile target creature or planeswalker.",
        )
        .expect("Necrotic Fumes should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("as an additional cost to cast this spell, exile a creature you control"),
        "expected Necrotic Fumes additional cost clause, got {rendered}"
    );
    assert!(
        lower.contains("exile target creature or planeswalker"),
        "expected Necrotic Fumes exile target clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_cost_with_non_cost_effect_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Impossible Additional Cost")
        .parse_text("As an additional cost to cast this spell, draw a card.")
        .expect_err("non-cost additional payment should fail loudly");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("draw") || message.contains("cost"),
        "expected loud non-cost additional-cost error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_named_spell_exile_self_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Burning Wish")
        .parse_text("Exile Burning Wish.")
        .expect("named self-exile clause should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile"),
        "expected exile-self clause to remain present, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_next_end_step_sentence_schedules_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Planebound Variant")
        .parse_text("{R}: You may put a planeswalker card from your hand onto the battlefield. Sacrifice it at the beginning of the next end step.")
        .expect("next-end-step delayed sacrifice should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("beginning of the next end step"),
        "expected delayed next-end-step wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_next_end_step_sentence_with_named_creature_keeps_delay() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sneak Attack Variant")
        .parse_text(
            "{R}: You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .expect("named delayed next-end-step sacrifice should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("beginning of the next end step"),
        "expected delayed next-end-step wording, got {rendered}"
    );
    assert!(
        !rendered.contains("sacrifice a creature"),
        "expected delayed clause not to collapse to generic immediate sacrifice, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_delayed_next_end_step_sentence_with_this_creature_keeps_source_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pyric Variant")
        .parse_text(
            "{R}: This creature gets +1/+0 until end of turn. Sacrifice this creature at the beginning of the next end step.",
        )
        .expect("self-referential delayed next-end-step sacrifice should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("beginning of the next end step"),
        "expected delayed next-end-step wording, got {rendered}"
    );
    assert!(
        !rendered.contains("sacrifice a creature"),
        "expected self-referential sacrifice not to collapse to generic creature, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_goblin_kites_strictly_and_renders_delayed_coin_flip_clause() {
    let def = parse_oracle_card_definition("Goblin Kites");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("At the beginning of the next end step, flip a coin"),
        "expected delayed coin-flip timing in rendered text, got {rendered}"
    );
    assert!(
        rendered.contains("If you lose the flip, sacrifice it"),
        "expected lose-the-flip sacrifice branch in rendered text, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("FlipCoinEffect"), "{debug}");
    assert!(debug.contains("DidNotHappen"), "{debug}");
    assert!(
        debug.contains("LessThanOrEqual") && debug.contains("2"),
        "{debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn tide_of_war_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Tide of War");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.abilities);

    assert_eq!(
        rendered,
        "Whenever one or more creatures block, flip a coin. If you win the flip, each blocking creature is sacrificed by its controller. If you lose the flip, each blocked creature is sacrificed by its controller."
    );
    assert!(
        debug.contains("BlocksTrigger")
            && debug.contains("one_or_more: true")
            && debug.contains("FlipCoinEffect")
            && debug.contains("Happened")
            && debug.contains("DidNotHappen")
            && debug.contains("ForEachObject")
            && debug.contains("blocking: true")
            && debug.contains("blocked: true")
            && debug.contains("SacrificeTargetEffect"),
        "expected Tide of War to structurally model its one-or-more block trigger and both coin-flip sacrifice branches, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_object_filter_with_entered_since_last_turn_ended_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Premature Burial Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target nonblack creature that entered since your last turn ended.")
        .expect("entered-since-last-turn qualifier should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("entered since your last turn ended"),
        "expected entered-since-last-turn qualifier in rendered output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_when_you_do_followup_clause_as_reflexive_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Invasion Variant")
        .card_types(vec![CardType::Battle])
        .parse_text(
            "When this permanent enters, you may sacrifice an artifact or creature. When you do, exile target artifact or creature an opponent controls.",
        )
        .expect("when-you-do followup clause should parse as a reflexive trigger");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("when you do"),
        "expected reflexive followup to keep when-you-do linkage, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ReflexiveTriggerEffect"),
        "expected reflexive followup lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_forage_when_you_do_followup_as_reflexive_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Curious Forager Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may forage. When you do, return target permanent card from your graveyard to your hand.",
        )
        .expect("forage when-you-do followup should parse as a reflexive trigger");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("you may forage"),
        "expected visible forage action, got {rendered}"
    );
    assert!(
        rendered_lower.contains("when you do"),
        "expected reflexive followup to keep when-you-do linkage, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("EmitKeywordActionEffect") && debug.contains("ReflexiveTriggerEffect"),
        "expected forage emission and reflexive followup lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_combustible_gearhulk_declined_optional_branch_gates_full_followup() {
    let def = parse_oracle_card_definition("Combustible Gearhulk");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("target opponent may have you draw three cards"),
        "expected opponent optional draw wording, got {rendered}"
    );
    assert!(
        (rendered.contains("If the player doesn't, you mill three cards")
            || rendered.contains("If they don't, mill three cards"))
            && (rendered.contains(
                "this creature deals damage to that player equal to the total mana value of those cards"
            ) || rendered.contains(
                "it deals damage to that player equal to the total mana value of those cards"
            )),
        "expected declined optional branch to include both mill and damage follow-up, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("IfEffect")
            && debug.contains("DidNotHappen")
            && debug.contains("MillEffect")
            && debug.contains("DealDamageEffect")
            && debug.contains("TotalManaValue")
            && debug.contains("milled_0"),
        "expected declined branch to gate the mill and tagged damage together, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_modal_trigger_header_keeps_prefix_effect_and_result_gate() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Immard Variant")
        .parse_text(
            "Whenever this creature enters or attacks, put a charge counter on it or remove one from it. When you remove a counter this way, choose one —\n• This creature deals 4 damage to any target.\n• This creature gains lifelink and indestructible until end of turn.",
        )
        .expect_err("specific this-way result gating should fail until modeled precisely");
    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("this way")
            || rendered.contains("unsupported predicate")
            || rendered.contains("unsupported target phrase"),
        "expected strict this-way modal gating rejection, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_aura_barbs_attached_target_contraction_keeps_second_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Aura Barbs Variant")
        .parse_text(
            "Each enchantment deals 2 damage to its controller, then each Aura attached to a creature deals 2 damage to the creature it's attached to.",
        )
        .expect("attached-target contraction should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.matches("ForEachObject").count() >= 2,
        "expected both for-each damage clauses, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("AttachedToTaggedObject"),
        "expected second clause target to stay linked to attached object, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("for each aura"),
        "expected rendered text to keep aura damage clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sage_of_hours_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Sage of Hours");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("Heroic")
            && rendered.contains("Whenever you cast a spell that targets this creature"),
        "Sage of Hours should preserve its heroic trigger in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "For each five counters removed this way, you take an extra turn after this one"
        ),
        "Sage of Hours should render the counter-group extra-turn clause, got {rendered}"
    );
    assert!(
        abilities_debug.contains("RemoveAnyCountersFromSourceEffect")
            && abilities_debug.contains("RepeatEffectsEffect")
            && abilities_debug.contains("DividedRoundedDown")
            && abilities_debug.contains("ExtraTurnEffect"),
        "Sage of Hours should lower removed-counter groups into repeated extra turns, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_for_each_of_x_target_permanents_builds_choose_then_for_each_tagged() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Doppelgang Variant")
        .parse_text(
            "For each of X target permanents, create X tokens that are copies of that permanent.",
        )
        .expect("for-each of X target permanents should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ChooseObjectsEffect"),
        "expected explicit target choice effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("dynamic_x: true"),
        "expected dynamic X target count, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ForEachTaggedEffect"),
        "expected per-target iteration over chosen objects, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("CreateTokenCopyEffect"),
        "expected token copy follow-up effect, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("for each tagged")
            || rendered
                .to_ascii_lowercase()
                .contains("for each of x target permanents"),
        "expected rendered text to keep 'for each of X target permanents', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_modal_choose_up_to_x_header_preserves_dynamic_bounds() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Modes Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose up to X —\n• Counter target spell.\n• Draw a card.\n• Create a Treasure token.",
        )
        .expect("choose-up-to-X modal header should parse");

    let modal = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("expected choose-mode effect");
    assert!(matches!(modal.choose_count, Value::X));
    assert!(
        matches!(modal.min_choose_count, Value::Fixed(0)),
        "expected zero minimum for choose-up-to-X header, got {modal:?}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("choose up to x"),
        "expected rendered text to keep choose-up-to-X header, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_nonhistoric_filter_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Historic Filter Variant")
        .parse_text("Return each nonland permanent that's not historic to its owner's hand.")
        .expect("nonhistoric filter should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("not historic"),
        "expected nonhistoric clause wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_historic_permanent_enters_becomes_dinosaur_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Displaced Dinosaurs Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dinosaur])
        .power_toughness(crate::card::PowerToughness::fixed(7, 7))
        .parse_text(
            "As a historic permanent you control enters, it becomes a 7/7 Dinosaur creature in addition to its other types.",
        )
        .expect("historic as-enters characteristic replacement should parse");

    assert!(
        def.spell_effect.is_none(),
        "as-enters replacement should not lower to a spell effect"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "As a historic permanent you control enters, it becomes a 7/7 Dinosaur creature in addition to its other types"
        ),
        "expected as-enters characteristic replacement rendering, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("EnterWithCharacteristicsForFilter"),
        "expected structured ETB characteristic replacement, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_same_name_damage_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Homing Lightning")
        .parse_text(
            "Homing Lightning deals 4 damage to target creature and each other creature with the same name as that creature.",
        )
        .expect("same-name damage fanout should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("deal 4 damage to target creature"),
        "expected primary targeted damage clause, got {rendered}"
    );
    assert!(
        rendered.contains("with the same name as that object")
            || rendered.contains("with the same name as that creature"),
        "expected same-name fanout wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_same_name_return_from_graveyard_fanout_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echoing Return")
        .parse_text(
            "Return target creature card and all other cards with the same name as that card from your graveyard to your hand.",
        )
        .expect("same-name return fanout from graveyard should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("from your graveyard to your hand"),
        "expected graveyard-to-hand return destination, got {rendered}"
    );
    assert!(
        !rendered.contains("to its owner's hand"),
        "expected graveyard return wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_of_up_to_target_damage_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wrap in Flames")
        .parse_text(
            "Wrap in Flames deals 1 damage to each of up to three target creatures. Those creatures can't block this turn.",
        )
        .expect("each-of-up-to-target damage clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("up to three target creatures"),
        "expected targeted damage count wording, got {rendered}"
    );
    assert!(
        !rendered.contains("for each creature"),
        "expected targeted (not global each-creature) damage wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spell_delayed_trigger_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Song of Blood Variant")
        .parse_text(
            "Mill four cards. Whenever a creature attacks this turn, it gets +1/+0 until end of turn for each creature card put into your graveyard this way.",
        )
        .expect("spell delayed trigger clause should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect"),
        "expected delayed trigger scheduling effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("until_end_of_turn: true"),
        "expected delayed trigger to expire at end of turn, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever creature attacks this turn"),
        "expected rendered delayed trigger wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_sacrifices_artifact_and_land_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Structural Collapse")
        .parse_text(
            "Target player sacrifices an artifact and a land of their choice. Structural Collapse deals 2 damage to that player.",
        )
        .expect("artifact-and-land sacrifice clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("artifact") && rendered.contains("land"),
        "expected both artifact and land sacrifice wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifact or land"),
        "expected split sacrifice effects rather than artifact-or-land, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_create_uses_each_player_controller() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Each Player Cat Variant")
        .parse_text("Each player creates a Food token.")
        .expect("each-player token creation should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("each player creates"),
        "expected each-player create phrasing, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("under your control"),
        "expected iterated-player token controller, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_create_for_each_tail_does_not_pollute_token_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tail Token Name Variant")
        .parse_text("Create a Food token for each untapped artifact you control.")
        .expect("create-for-each tail should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("food untapped artifact"),
        "expected token name to remain Food, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_pay_life_line_as_static_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Life Variant")
        .parse_text("Ward—Pay 3 life.")
        .expect("ward-pay-life line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Ward—Pay 3 life"),
        "expected ward-pay-life marker text, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("you lose 3 life"),
        "ward-pay-life should not lower as a standalone lose-life spell effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_mana_and_life_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Hybrid Cost Variant")
        .parse_text("Ward—{2}, Pay 2 life.")
        .expect("ward mixed-cost line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ward"),
        "expected ward text in compiled output, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("you lose 2 life"),
        "ward mixed cost should not lower as standalone lose-life spell effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_sacrifice_permanent_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Sacrifice Variant")
        .parse_text("Ward—Sacrifice a permanent.")
        .expect("ward sacrifice line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ward") && rendered_lower.contains("sacrifice"),
        "expected ward sacrifice wording in compiled output, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Ward(TotalCost") && debug.contains("SacrificeEffect"),
        "expected real ward sacrifice static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_sacrifice_mana_value_or_greater_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Mana Value Sacrifice Variant")
        .parse_text("Ward—Sacrifice a permanent with mana value 1 or greater.")
        .expect("ward sacrifice mana-value line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ward")
            && rendered_lower.contains("sacrifice")
            && rendered_lower.contains("mana value 1 or greater"),
        "expected ward sacrifice mana-value wording in compiled output, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Ward(TotalCost")
            && debug.contains("SacrificeEffect")
            && debug.contains("GreaterThanOrEqual(1)"),
        "expected real ward sacrifice cost with mana-value comparison, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ulamogs_dreadsire_oracle_text_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ulamog's Dreadsire")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .parse_text(
            "Vigilance\nWard—Sacrifice a permanent with mana value 1 or greater.\n{T}: Create a 10/10 colorless Eldrazi creature token.",
        )
        .expect("Ulamog's Dreadsire oracle text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("vigilance")
            && rendered_lower.contains("ward")
            && rendered_lower.contains("mana value 1 or greater")
            && rendered_lower.contains("10/10 colorless eldrazi"),
        "expected Ulamog's Dreadsire compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_tap_creature_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Tap Variant")
        .parse_text("Ward—Tap an untapped creature you control.")
        .expect("ward tap line should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Ward(TotalCost")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("TapEffect"),
        "expected real ward tap cost effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_exile_graveyard_card_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Graveyard Exile Variant")
        .parse_text("Ward—Exile a card from your graveyard.")
        .expect("ward exile line should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Ward(TotalCost") && debug.contains("ExileEffect"),
        "expected real ward exile static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_mana_and_sacrifice_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Mixed Sacrifice Variant")
        .parse_text("Ward—{2}, Sacrifice a creature.")
        .expect("ward mixed sacrifice line should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Ward(TotalCost")
            && debug.contains("Mana")
            && debug.contains("SacrificeEffect"),
        "expected real ward mixed mana and sacrifice cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_draw_card_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Draw Bad Variant")
        .parse_text("Ward—Draw a card.")
        .expect_err("non-payment ward clause should fail");

    let lower = format!("{err:?}").to_ascii_lowercase();
    assert!(
        lower.contains("ward"),
        "expected ward parse error, got {err:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cultist_of_the_absolute_stays_static_and_grants_commander_abilities() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cultist of the Absolute")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Background])
        .parse_text(
            "Commander creatures you own get +3/+3 and have flying, deathtouch, \"Ward—Pay 3 life,\" and \"At the beginning of your upkeep, sacrifice a creature.\"",
        )
        .expect("Cultist of the Absolute should parse as a static grant line");

    assert!(
        def.spell_effect.is_none(),
        "Cultist of the Absolute should not compile as a spell effect: {:?}",
        def.spell_effect
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        def.abilities.len() >= 5
            && abilities_debug.contains("is_commander: true")
            && abilities_debug.contains("Ward")
            && abilities_debug.contains("BeginningOfUpkeepTrigger")
            && abilities_debug.contains("Sacrifice"),
        "expected Cultist of the Absolute to grant its pump, ward, and upkeep trigger statically, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn guild_artisan_stays_static_and_grants_the_treasure_trigger_to_commanders() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Guild Artisan")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Background])
        .parse_text(
            "Commander creatures you own have \"Whenever this creature attacks a player, if no opponent has more life than that player, you create two Treasure tokens.\"",
        )
        .expect("Guild Artisan should parse as a static grant line");

    assert!(
        def.spell_effect.is_none(),
        "Guild Artisan should not compile as a spell effect: {:?}",
        def.spell_effect
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("GrantObjectAbilityForFilter")
            && abilities_debug.contains("PlayerHasNoOpponentWithMoreLifeThan")
            && abilities_debug.contains("ThisAttacksTrigger")
            && abilities_debug.contains("CreateTokenEffect"),
        "expected Guild Artisan to grant an attack trigger to commander creatures, got {abilities_debug}"
    );

    assert!(
        abilities_debug.contains("intervening_if: Some")
            && abilities_debug.contains("PlayerHasNoOpponentWithMoreLifeThan"),
        "expected Guild Artisan's granted trigger to keep its intervening-if gate, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dungeon_delver_strictly_grants_room_trigger_duplication_to_commanders() {
    assert_oracle_card_parses_strict("Dungeon Delver");

    let def = parse_oracle_card_definition("Dungeon Delver");
    assert!(
        def.spell_effect.is_none(),
        "Dungeon Delver should not compile as a spell effect: {:?}",
        def.spell_effect
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("GrantAbility")
            && abilities_debug.contains("is_commander: true")
            && abilities_debug.contains("DungeonRoomTriggerDuplication"),
        "expected Dungeon Delver to grant room trigger duplication to commander creatures, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dungeon_delver_compiled_text_preserves_room_trigger_duplication_clause() {
    let def = parse_oracle_card_definition("Dungeon Delver");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Commander creatures you own have \"Room abilities of dungeons you own trigger an additional time.\""
        ),
        "expected Dungeon Delver compiled text to render its granted room ability, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ward_discard_multiple_card_types_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ward Typed Discard Variant")
        .parse_text("Ward—Discard an enchantment, instant, or sorcery card.")
        .expect("ward typed-discard line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ward")
            && rendered_lower.contains("discard")
            && rendered_lower.contains("enchantment")
            && rendered_lower.contains("instant")
            && rendered_lower.contains("sorcery"),
        "expected ward typed-discard wording in compiled output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("keyword_marker") && !debug.contains("staticabilityid::custom"),
        "ward typed-discard should lower to a real ward static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_if_they_dont_uses_negative_may_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Umbilicus Variant")
        .parse_text(
            "At the beginning of each player's upkeep, that player may pay 2 life. If they don't, they return a permanent they control to its owner's hand.",
        )
        .expect("if-they-dont sentence should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains(
            "At the beginning of each player's upkeep, that player may pay 2 life. If they don't, they return a permanent they control to its owner's hand"
        ),
        "expected typed life payment and contextualized negative branch, got {rendered}"
    );
    assert!(
        !lower.contains("if that player does,"),
        "did-not branch should not be rendered as affirmative branch, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(debug.contains("PayLifeEffect"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_smothering_tithe_if_the_player_doesnt_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Smothering Tithe Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever an opponent draws a card, that player may pay {2}. If the player doesn't, you create a Treasure token.",
        )
        .expect("smothering tithe trigger should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("whenever an opponent draws a card")
            && lower.contains("may pay {2}")
            && lower.contains("treasure token"),
        "expected Smothering Tithe trigger to render its payment and Treasure branch, got {rendered}"
    );
    assert!(
        lower.contains("if they don't") || lower.contains("if that player doesn't"),
        "expected negative may branch wording, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("PayManaEffect") && debug.contains("CreateTokenEffect"),
        "expected trigger to include mana payment and Treasure creation, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ranger_captain_of_eos_search_and_silence_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ranger-Captain of Eos Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may search your library for a creature card with mana value 1 or less, reveal it, put it into your hand, then shuffle.\nSacrifice this creature: Your opponents can't cast noncreature spells this turn.",
        )
        .expect("ranger-captain should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("search your library for a creature card with mana value 1 or less")
            && lower.contains("reveal it")
            && lower.contains("put it into your hand"),
        "expected search trigger wording, got {rendered}"
    );
    assert!(
        lower.contains("sacrifice this creature")
            && lower.contains("opponents can't cast noncreature spells this turn"),
        "expected sacrifice silence ability wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_iterative_library_loop_for_tainted_pact() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tainted Pact")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
        )
        .expect("iterative library loop should parse");

    assert_eq!(def.name(), "Tainted Pact");
    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("repeatprocess")
            && spell_debug.contains("distinctnames")
            && spell_debug.contains("maymovetozone"),
        "expected Tainted Pact runtime loop support, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_iterative_library_repeat_process_uses_oracle_loop_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tainted Pact")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
        )
        .expect("iterative library loop should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you may put that card into your hand unless it has the same name as another card exiled this way")
            && rendered.contains("repeat this process until you put a card into your hand or you exile two cards with the same name")
            && !rendered.contains("iterative_library_current")
            && !rendered.contains("tagged object"),
        "expected Tainted Pact compiled text to use oracle loop wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_cost_prefixed_each_player_draw_discard_compacts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lore Broker Variant")
        .parse_text("{T}: Each player draws a card, then discards a card.")
        .expect("draw-then-discard should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("{T}: Each player draws a card, then discards a card."),
        "expected compact each-player draw/discard wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_attack_skip_untap_uses_controller_next_untap_step() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Apes Variant")
        .parse_text(
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step.",
        )
        .expect("attack untap-skip line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("doesn't untap during its controller's next untap step"),
        "expected controller-next-untap-step wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_combat_damage_tap_then_doesnt_untap_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kashi Variant")
        .parse_text(
            "Whenever this creature deals combat damage to a creature, tap that creature and it doesn't untap during its controller's next untap step.",
        )
        .expect("combat-damage tap+untap-skip trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("tap that creature") || rendered.contains("tap it"),
        "expected tap follow-up wording, got {rendered}"
    );
    assert!(
        rendered.contains("doesn't untap during its controller's next untap step")
            || rendered.contains("doesnt untap during its controller's next untap step")
            || rendered.contains("can't untap during its controller's next untap step")
            || rendered.contains("cant untap during its controller's next untap step"),
        "expected controller-next-untap-step wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rejects_three_dog_aura_copy_attachment_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Three Dog Variant")
        .parse_text(
            "Whenever you attack, you may pay {2} and sacrifice an Aura attached to this creature. When you sacrifice an Aura this way, for each other attacking creature you control, create a token that's a copy of that Aura attached to that creature.",
        )
        .expect_err("unsupported aura-copy attachment fanout should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern")
            || message.contains("unsupported aura-copy attachment fanout clause"),
        "expected explicit unsupported rejection, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_defending_player_suffix_subject_keeps_player_binding() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper Variant")
        .parse_text(
            "Whenever this creature attacks and isn't blocked, defending player loses 2 life.",
        )
        .expect("parse defending-player suffix subject");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("defending player loses 2 life"),
        "expected defending-player life-loss wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_compound_assigns_no_combat_damage_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper of Tresserhorn Probe")
        .parse_text(
            "Whenever this creature attacks and isn't blocked, it assigns no combat damage this turn and defending player loses 2 life.",
        )
        .expect("assigns-no-combat-damage should compose with a following action");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("assigns no combat damage this turn")
            && rendered.contains("defending player loses 2 life"),
        "expected both compound actions, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rejects_defending_players_choice_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Erithizon Reject Variant")
        .parse_text(
            "Whenever this creature attacks, put a +1/+1 counter on target creature of defending player's choice.",
        )
        .expect_err("defending player's choice clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("defending-players-choice")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected defending player's choice rejection, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rejects_creature_token_player_planeswalker_target_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Coalborn Reject Variant")
        .parse_text("{2}{R}: This creature deals 1 damage to target creature token, player, or planeswalker.")
        .expect_err("creature-token/player/planeswalker target clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("creature-token/player/planeswalker")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected creature-token/player/planeswalker rejection, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rejects_if_you_sacrifice_an_island_this_way_clause() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Serendib Reject Variant")
        .parse_text(
            "At the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you.",
        )
        .expect_err("if-you-sacrifice-an-island-this-way clause should not partially parse");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("if-you-sacrifice-an-island-this-way")
            || message.contains("if you sacrifice an island this way")
            || message.contains("unsupported triggered line")
            || message.contains("unsupported parser line")
            || message.contains("unsupported known partial parse pattern"),
        "expected island-this-way rejection, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_rain_of_daggers_uses_destroyed_this_way_life_loss_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rain of Daggers Variant")
        .parse_text(
            "Destroy all creatures target opponent controls. You lose 2 life for each creature destroyed this way.",
        )
        .expect("rain-of-daggers style text should parse");

    let lose_life = def
        .spell_effect
        .as_ref()
        .expect("Rain of Daggers variant should have spell effects")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::LoseLifeEffect>()
                .cloned()
        })
        .expect("Rain of Daggers variant should lower to life loss");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("destroy all target opponent's creatures")
            || lower.contains("destroy all creatures target opponent controls"),
        "expected opponent-controls destroy-all clause, got {rendered}"
    );
    assert!(
        lower.contains("lose 2 life for each creature destroyed this way"),
        "expected destroyed-this-way life-loss clause, got {rendered}"
    );
    assert!(
        lose_life
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        "authored for-each life loss should retain its surface hint, got {:?}",
        lose_life.amount
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fabrication_module_parses_and_renders_one_or_more_energy_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fabrication Module")
        .parse_text(
            "Whenever you get one or more {E} (energy counters), put a +1/+1 counter on target creature you control.\n{4}, {T}: You get {E}.",
        )
        .expect("Fabrication Module should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever you get one or more {e}")
            && rendered.contains("put a +1/+1 counter on target creature you control"),
        "expected one-or-more energy trigger with target creature clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fabrication_module_uses_player_gets_counters_trigger_model() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fabrication Module")
        .parse_text(
            "Whenever you get one or more {E}, put a +1/+1 counter on target creature you control.",
        )
        .expect("Fabrication Module trigger line should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");

    let debug = format!("{:?}", triggered.trigger);
    assert!(
        debug.contains("PlayerGetsCountersTrigger")
            && debug.contains("OneOrMore")
            && debug.contains("Energy"),
        "expected one-or-more energy player-counters trigger, got {debug}"
    );

    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("PutCountersEffect")
            && effects_debug.contains("card_types: [Creature]")
            && effects_debug.contains("controller: Some(You)"),
        "expected targeted +1/+1 counter effect, got {effects_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_terastodon_keeps_destroy_and_graveyard_loop() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Terastodon Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elephant])
        .power_toughness(PowerToughness::fixed(9, 9))
        .parse_text(
            "When this creature enters, you may destroy up to three target noncreature permanents. For each permanent put into a graveyard this way, its controller creates a 3/3 green Elephant creature token.",
        )
        .expect("Terastodon should parse");

    let ability_debug = format!("{:#?}", def.abilities);
    let ability_debug_compact = ability_debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    assert!(
        ability_debug.contains("Destroy"),
        "expected Terastodon to keep the destroy effect, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("TaggedEffect"),
        "expected Terastodon destroy clause to keep tagged follow-up linkage, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("ForEachTaggedEffect"),
        "expected Terastodon to lower the graveyard follow-up to a tagged loop, got {ability_debug}"
    );
    assert!(
        !ability_debug_compact.contains("ForEachTaggedEffect{tag:TagKey(\"__it__\")"),
        "expected Terastodon follow-up loop to bind to the destroy tag instead of raw __it__, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("CreateTokenEffect"),
        "expected Terastodon to create Elephant tokens in the loop, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy up to three target noncreature permanents"),
        "expected Terastodon destroy clause to render, got {rendered}"
    );
    assert!(
        rendered.contains("for each object destroyed this way"),
        "expected Terastodon graveyard follow-up to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_artifact_or_tapped_creature_does_not_require_tapped_artifacts() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Radiant Strike Variant")
        .parse_text("Destroy target artifact or tapped creature. You gain 3 life.")
        .expect("artifact-or-tapped-creature line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        !lower.contains("tapped artifact or creature"),
        "expected tapped to apply only to creature side of disjunction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_named_angel_token_keeps_explicit_pt_and_keywords() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Battle at the Helvault Variant")
        .parse_text(
            "Create Avacyn, a legendary 8/8 white Angel creature token with flying, vigilance, and indestructible.",
        )
        .expect("named angel token line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("8/8 white angel creature token"),
        "expected explicit 8/8 angel token, got {rendered}"
    );
    assert!(
        rendered.contains("vigilance") && rendered.contains("indestructible"),
        "expected explicit vigilance and indestructible keywords, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_exile_until_clause_keeps_target_filter_without_until_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Liminal Hold Variant")
        .parse_text(
            "When this enchantment enters, exile up to one target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
        )
        .expect("exile-until line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target nonland permanent an opponent controls")
            || rendered.contains("target opponent's nonland permanent"),
        "expected nonland-permanent target filter, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_look_at_target_players_hand_keeps_targeting() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Glasses Variant")
        .parse_text("{T}: Look at target player's hand.")
        .expect("target-hand look clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Look at target player's hand."),
        "expected explicit target player hand wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_who_controls_condition_wraps_conditional() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shatter Variant")
        .parse_text(
            "Each player who controls a creature with power 4 or greater draws a card. Then destroy all creatures.",
        )
        .expect("each-player conditional clause should parse");

    // The per-player condition must be a real ConditionalEffect inside the
    // player loop — a surface like "each player who controls ..." could also
    // come from a (wrong) filter-based lowering.
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("ConditionalEffect")
            && debug.contains("PlayerControls"),
        "expected the control condition to wrap a per-player conditional, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("for each player, if that player controls")
            || rendered.contains("each player who controls"))
            && rendered.contains("power 4 or greater"),
        "expected per-player control condition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_may_exile_then_return_same_object_keeps_followup_return() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conjurer's Closet Variant")
        .parse_text(
            "At the beginning of your end step, you may exile target creature you control, then return that card to the battlefield under your control.",
        )
        .expect("may exile-then-return line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile target creature you control"),
        "expected exile clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("return")
            && rendered.contains("battlefield")
            && rendered.contains("under your control"),
        "expected return-to-battlefield followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hazorets_favor_keeps_delayed_sacrifice_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hazoret's Favor Variant")
        .parse_text(
            "At the beginning of combat on your turn, you may have target creature you control get +2/+0 and gain haste until end of turn. If you do, sacrifice it at the beginning of the next end step.",
        )
        .expect("hazorets favor line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("next end step") && rendered.contains("sacrifice"),
        "expected delayed sacrifice followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_earthbend_then_earthbend_chain_keeps_both_and_life_gain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cracked Earth Technique Variant")
        .parse_text("Earthbend 3, then earthbend 3. You gain 3 life.")
        .expect("earthbend then earthbend line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.matches("earthbend 3").count() >= 2,
        "expected both earthbend clauses, got {rendered}"
    );
    assert!(
        rendered.contains("gain 3 life"),
        "expected trailing life gain clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn strict_parse_dai_li_indoctrination_regression() {
    assert_oracle_card_parses_strict("Dai Li Indoctrination");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dai_li_indoctrination_compiled_text_keeps_earthbend_mode() {
    let def = parse_oracle_card_definition("Dai Li Indoctrination");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose one"),
        "expected modal text in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("earthbend 2"),
        "expected earthbend mode to keep count, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dai_li_indoctrination_compiled_text_keeps_discard_mode_targets() {
    let def = parse_oracle_card_definition("Dai Li Indoctrination");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target opponent reveals their hand"),
        "expected reveal-hand mode text, got {rendered}"
    );
    assert!(
        rendered.contains("nonland permanent card") && rendered.contains("discards"),
        "expected nonland-permanent discard clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn follow_the_lumarets_keeps_optional_one_or_two_partition_and_shared_remainder() {
    let def = parse_oracle_card_definition("Follow the Lumarets");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Infusion — Look at the top four cards of your library. You may reveal a creature or land card from among them and put it into your hand. If you gained life this turn, you may instead reveal two creature and/or land cards from among them and put them into your hand. Put the rest on the bottom of your library in a random order."
    );

    let program = def.spell_effect.as_ref().expect("spell effect");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one count-replacement segment: {program:#?}");
    };
    let [replacement] = segment.self_replacements.as_slice() else {
        panic!("expected one count-replacement branch: {program:#?}");
    };
    assert!(matches!(
        &replacement.condition,
        crate::effect::Condition::PlayerGainedLifeThisTurnOrMore {
            player: PlayerFilter::You,
            count: 1,
        }
    ));
    assert!(replacement.leading_instead_surface);

    let assert_partition = |effects: &[crate::effect::Effect], count| {
        let [look_effect, may_effect, remainder_effect] = effects else {
            panic!("expected look/may/remainder partition: {effects:#?}");
        };
        let look = look_effect
            .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            .expect("looked-card producer");
        let may = may_effect
            .downcast_ref::<crate::effects::MayEffect>()
            .expect("optional exact-size reveal");
        let [choose_effect, reveal_effect, move_effect] = may.effects.as_slice() else {
            panic!("expected choose/reveal/move optional body: {may:#?}");
        };
        let choose = choose_effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .expect("typed selected subset");
        assert_eq!(choose.count, ChoiceCount::exactly(count));
        assert!(
            reveal_effect
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                .is_some_and(|for_each| for_each.tag == choose.tag)
        );
        assert!(
            move_effect
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                .is_some_and(|for_each| for_each.tag == choose.tag)
        );
        let remainder = remainder_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
            .expect("typed looked-minus-selected complement");
        assert_eq!(remainder.tag, look.tag);
        assert_eq!(remainder.keep_tagged.as_ref(), Some(&choose.tag));
        assert_eq!(
            remainder.order,
            crate::effects::consult_helpers::LibraryBottomOrder::Random
        );
    };
    assert_partition(&segment.default_effects, 1);
    assert_partition(&replacement.replacement_effects, 2);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kill_suit_cultist_registers_typed_one_shot_damage_replacement() {
    let def = parse_oracle_card_definition("Kill-Suit Cultist");
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![
            "This creature attacks each combat if able.".to_string(),
            "{B}, Sacrifice this creature: The next time damage would be dealt to target creature this turn, destroy that creature instead.".to_string(),
        ]
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Kill-Suit Cultist activated ability");
    let [effect] = activated.effects.flattened_default_effects() else {
        panic!("expected one replacement-registration effect: {activated:#?}");
    };
    let replacement = effect
        .downcast_ref::<crate::effects::ReplaceNextDamageToTargetEffect>()
        .expect("typed next-damage replacement");
    assert!(matches!(
        replacement.target.base(),
        ChooseSpec::Object(filter) if filter.card_types.as_slice() == [CardType::Creature]
    ));
    let [tag_effect, destroy_effect] = replacement.replacement_effects.as_slice() else {
        panic!("expected damaged-target tag and destroy action: {replacement:#?}");
    };
    let tag = &tag_effect
        .downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()
        .expect("triggering damage target tag")
        .tag;
    let destroy = destroy_effect
        .downcast_ref::<crate::effects::DestroyEffect>()
        .expect("destroy replacement action");
    assert!(matches!(destroy.spec.base(), ChooseSpec::Tagged(found) if found == tag));
}
