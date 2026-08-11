use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_spell_cost_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    compiled_text_lines(&parse_oracle_card_definition(name))
        .join("\n")
        .to_ascii_lowercase()
}

#[test]
fn conditional_spell_cost_modifiers_keep_their_turn_prefix() {
    for name in ["Geyser Drake", "Naiad of Hidden Coves"] {
        let compiled = compiled_spell_cost_text(name);
        assert!(
            compiled
                .contains("during turns other than yours, spells you cast cost {1} less to cast"),
            "{name} must retain its turn condition:\n{compiled}"
        );
    }
}

#[test]
fn spell_cost_filters_keep_reusable_qualifiers_and_origin_unions() {
    for (name, expected) in [
        (
            "Kethis, the Hidden Hand",
            "legendary spells you cast cost {1} less to cast",
        ),
        (
            "Gonti, Canny Acquisitor",
            "spells you cast but don't own cost {1} less to cast",
        ),
        ("Urza's Filter", "multicolored spells cost {2} less to cast"),
        (
            "Cunning Nightbonder",
            "spells with flash you cast cost {1} less to cast",
        ),
    ] {
        let compiled = compiled_spell_cost_text(name);
        assert!(
            compiled.contains(expected),
            "{name} must retain its spell filter:\n{compiled}"
        );
    }

    for name in [
        "Savvy Trader",
        "Advanced Reconstruction",
        "Sage of the Beyond",
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            compiled.contains("spells you cast from anywhere other than your hand cost"),
            "{name} must retain the non-hand origin union:\n{compiled}"
        );
        let filter = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => static_ability
                    .cost_reduction()
                    .map(|reduction| &reduction.filter),
                _ => None,
            })
            .expect("non-hand spell reduction should compile");
        assert!(
            filter.any_of.iter().any(|branch| {
                branch.zone == Some(Zone::Hand) && branch.owner == Some(PlayerFilter::NotYou)
            }),
            "{name}'s non-hand complement must include cards in another player's hand: {filter:#?}"
        );
    }
}

#[test]
fn cunning_nightbonder_shares_its_flash_spell_filter_across_both_clauses() {
    let definition = parse_oracle_card_definition("Cunning Nightbonder");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Flash\nSpells with flash you cast cost {1} less to cast and can't be countered."
    );

    let mut reduction_filter = None;
    let mut protected_filter = None;
    for ability in &definition.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(model) = static_ability.compiled_model() else {
            continue;
        };
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => {
                reduction_filter = Some(reduction.filter.clone());
            }
            ironsmith_core::StaticAbilityPayload::RuleRestriction {
                restriction: ironsmith_core::Restriction::BeCountered(filter),
                additional_restrictions,
                ..
            } if additional_restrictions.is_empty() => {
                protected_filter = Some(filter.clone());
            }
            _ => {}
        }
    }
    let reduction_filter = reduction_filter.expect("flash-spell reduction");
    let protected_filter = protected_filter.expect("flash-spell counter restriction");
    assert_eq!(protected_filter, reduction_filter);
    assert_eq!(reduction_filter.cast_by, Some(PlayerFilter::You));
    assert_eq!(
        reduction_filter.static_abilities,
        vec![crate::static_abilities::StaticAbilityId::Flash]
    );
}

#[test]
fn hinata_cost_modifiers_scale_for_each_exact_target() {
    let compiled = compiled_spell_cost_text("Hinata, Dawn-Crowned");
    assert!(
        compiled.contains("spells you cast cost {1} less to cast for each target"),
        "Hinata's reduction must retain per-target scaling:\n{compiled}"
    );
    assert!(
        compiled.contains("spells your opponents cast cost {1} more to cast for each target"),
        "Hinata's tax must retain per-target scaling:\n{compiled}"
    );
}

#[test]
fn self_only_spell_increases_keep_source_scope_targets_and_turn_conditions() {
    for (name, expected) in [
        (
            "Dragon's Prey",
            "this spell costs {2} more to cast if it targets a dragon",
        ),
        (
            "Vanish into Eternity",
            "this spell costs {3} more to cast if it targets a creature",
        ),
        (
            "Hurkyl's Final Meditation",
            "during turns other than yours, this spell costs {3} more to cast",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();
        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(
            compiled.contains(expected),
            "{name} must retain its self-only increase surface:\n{compiled}"
        );
        assert!(
            debug.contains("costincrease") && debug.contains("source: true"),
            "{name} must compile its increase as a source-only filter:\n{debug}"
        );
    }
}
