use super::*;
use crate::cards::CardDefinitionRuntimeExt;

const TEXT: &str = "As an additional cost to cast this spell, sacrifice an artifact or creature.\nDraw cards equal to the mana value of the sacrificed permanent.";

fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Morbid Curiosity")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap()
}

#[test]
fn additional_sacrifice_draw_counts_the_chosen_artifact_or_creatures_mana_value() {
    let definition = definition();
    for kind in [CardType::Artifact, CardType::Creature] {
        for mana_value in [0, 3, 6] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let card =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Sacrifice Candidate")
                    .card_types(vec![kind])
                    .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                    .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                        crate::mana::ManaSymbol::Generic(mana_value),
                    ]]))
                    .build();
            let victim = game.create_object_from_card(&card, alice, Zone::Battlefield);
            for _ in 0..10 {
                game.create_object_from_card(&card, alice, Zone::Library);
            }
            let mut decisions = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_decision_maker(&mut decisions);
            for cost in definition.additional_non_mana_costs() {
                crate::effects::execute_effect(
                    &mut game,
                    cost.effect_ref().expect("effect-backed sacrifice cost"),
                    &mut ctx,
                )
                .unwrap();
            }
            assert!(!game.battlefield.contains(&victim));
            for effect in definition.spell_effect.as_ref().unwrap() {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(
                game.player(alice).unwrap().hand.len(),
                usize::from(mana_value)
            );
        }
    }
}

#[test]
fn additional_sacrifice_mana_value_keeps_the_authored_object_kind() {
    let definition = definition();
    let draw = definition
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()[0]
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .unwrap();
    assert!(
        draw.count
            .has_surface_hint(ValueSurfaceHint::SacrificedObject(
                ironsmith_core::SacrificedObjectKind::Permanent
            ))
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.replace(
            "the mana value of the sacrificed permanent",
            "the sacrificed permanent's mana value"
        )
    );
}

#[test]
fn sacrificed_draw_characteristics_preserve_each_explicit_noun() {
    use ironsmith_core::SacrificedObjectKind;
    for (noun, kind) in [
        ("artifact", SacrificedObjectKind::Artifact),
        ("creature", SacrificedObjectKind::Creature),
        ("enchantment", SacrificedObjectKind::Enchantment),
        ("permanent", SacrificedObjectKind::Permanent),
    ] {
        for characteristic in ["power", "toughness", "mana value"] {
            let text = format!(
                "As an additional cost to cast this spell, sacrifice a permanent.\nDraw cards equal to the {characteristic} of the sacrificed {noun}."
            );
            let definition = crate::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Sacrificed Characteristic Probe",
            )
            .card_types(vec![CardType::Sorcery])
            .parse_text(&text)
            .unwrap();
            let draw = definition
                .spell_effect
                .as_ref()
                .unwrap()
                .flattened_default_effects()[0]
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .unwrap();
            assert!(
                draw.count
                    .has_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
                "{text}: {:?}",
                draw.count
            );
            assert!(
                crate::compiled_text::compiled_text_lines(&definition)
                    .last()
                    .unwrap()
                    .contains(&format!("sacrificed {noun}'s {characteristic}")),
                "{text}: {:?}",
                crate::compiled_text::compiled_text_lines(&definition)
            );
        }
    }
}
