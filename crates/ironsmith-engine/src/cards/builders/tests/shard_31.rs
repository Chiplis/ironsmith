use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

#[test]
pub(super) fn kiora_master_of_the_depths_keeps_the_action_after_its_quoted_emblem() {
    let name = "Kiora, Master of the Depths";
    assert_oracle_card_parses_strict(name);
    let definition = parse_oracle_card_definition(name);
    let compiled = compiled_text_lines(&definition).join("\n");
    let mut found_ultimate = false;
    for ability in &definition.abilities {
        let AbilityKind::Activated(activated) = &ability.kind else {
            continue;
        };
        let effects = activated.effects.flattened_default_effects();
        if effects
            .first()
            .and_then(|effect| effect.downcast_ref::<crate::effects::CreateEmblemEffect>())
            .is_none()
        {
            continue;
        }
        found_ultimate = true;
        let [emblem_effect, token_effect] = effects else {
            panic!(
                "Kiora's ultimate should contain exactly the emblem and token actions: {effects:#?}"
            );
        };
        assert!(
            emblem_effect
                .downcast_ref::<crate::effects::CreateEmblemEffect>()
                .is_some()
        );
        let create = token_effect
            .downcast_ref::<CreateTokenEffect>()
            .expect("Kiora's second ultimate action should create tokens");
        assert_eq!(create.count, Value::Fixed(3));
        assert!(create.token.card.subtypes.contains(&Subtype::Octopus));
    }

    assert!(
        found_ultimate
            && compiled.contains(
                "You get an emblem with \"Whenever a creature you control enters, you may have it fight target creature.\""
            )
            && compiled.contains("three 8/8 blue Octopus creature tokens"),
        "the ultimate must preserve and render its emblem and trailing token creation as two actions:\n{compiled}"
    );
}

#[test]
pub(super) fn infernal_vessel_compacts_returned_object_counters_and_type_followup() {
    let name = "Infernal Vessel";
    assert_oracle_card_parses_strict(name);
    let compiled = compiled_text_lines(&parse_oracle_card_definition(name)).join("\n");

    assert!(
        (compiled.contains(
            "return that card to the battlefield under its owner's control with two +1/+1 counters on it"
        ) || compiled.contains(
            "return it to the battlefield under its owner's control with two +1/+1 counters on it"
        )) && (compiled.contains(". It becomes a Demon in addition to its other types")
            || compiled.contains(". It is a Demon in addition to its other types")
            || compiled.contains(". It's a Demon in addition to its other types"))
            && !compiled.contains("and put two +1/+1 counters"),
        "the returned-object result must carry both the counters and Demon followup:\n{compiled}"
    );
}

#[test]
pub(super) fn shadow_prophecy_renders_its_bounded_looked_card_partition() {
    let name = "Shadow Prophecy";
    assert_oracle_card_parses_strict(name);
    let compiled = compiled_text_lines(&parse_oracle_card_definition(name)).join("\n");

    assert!(
        compiled.contains(
            "Put up to two of them into your hand and the rest into your graveyard. You lose 2 life"
        ),
        "the exact looked-card complement must render with the bounded up-to count:\n{compiled}"
    );
}

#[test]
pub(super) fn ordinary_quoted_emblem_payloads_survive_statement_grouping() {
    for name in [
        "Ajani Steadfast",
        "Mordenkainen",
        "Narset Transcendent",
        "Nissa, Who Shakes the World",
        "Sorin, Lord of Innistrad",
        "Tamiyo, Field Researcher",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition).join("\n");
        assert!(
            definition.abilities.iter().any(|ability| {
                let AbilityKind::Activated(activated) = &ability.kind else {
                    return false;
                };
                activated
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .any(|effect| {
                        effect
                            .downcast_ref::<crate::effects::CreateEmblemEffect>()
                            .is_some()
                    })
            }),
            "{name} should retain a typed emblem effect:\n{compiled}"
        );
        assert!(
            compiled.contains("get an emblem with"),
            "{name} should render its quoted emblem payload:\n{compiled}"
        );
    }
}
