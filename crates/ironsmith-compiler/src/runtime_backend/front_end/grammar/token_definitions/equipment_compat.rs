use crate::runtime_backend::front_end::lexer::{TokenKind, lex_line, parser_token_word_refs};
use crate::runtime_backend::token_definition::{
    EquipmentRuleLineShape, EquipmentRulesShape, TokenEquipShape,
};
use winnow::prelude::*;

use super::super::primitives;
use super::{common, equipment, rules};

pub(crate) fn parse_equipment_rules_text(source_text: &str) -> Option<EquipmentRulesShape> {
    let tokens = lex_line(source_text, 0).ok()?;
    equipment::parse_equipment_rules_tokens(&tokens)
}

pub(crate) fn parse_equipment_damage_grant_text(
    ability_text: &str,
) -> Option<crate::runtime_backend::token_definition::EquipmentDamageGrantShape> {
    let tokens = lex_line(ability_text, 0).ok()?;
    equipment::parse_equipment_damage_grant_tokens(&tokens)
}

fn classify_equipment_line(line: &str) -> Option<EquipmentRuleLineShape> {
    let tokens = lex_line(line, 0).ok()?;
    let is_granted_line = primitives::parse_prefix(
        &tokens,
        primitives::phrase(&["equipped", "creature", "has"]).void(),
    )
    .is_some();
    if is_granted_line && tokens.iter().any(|token| token.kind == TokenKind::Colon) {
        let ability_text = rules::first_double_quoted_text(&tokens).unwrap_or_else(|| {
            rules::rendered_unquoted(tokens.get(3..).unwrap_or(tokens.as_slice()))
        });
        return Some(EquipmentRuleLineShape::GrantedDamage {
            display_text: line.to_string(),
            grant: parse_equipment_damage_grant_text(&ability_text)?,
        });
    }

    if primitives::parse_prefix(&tokens, primitives::kw("equip")).is_some() {
        return Some(EquipmentRuleLineShape::Equip(TokenEquipShape {
            amount: equipment::first_generic_mana(&tokens)?,
        }));
    }

    if is_granted_line {
        let words = parser_token_word_refs(&tokens);
        let power_toughness = common::phrase_present(&words, &["gets", "+1/+1"]).then_some((1, 1));
        let keywords = equipment::token_keywords(&words);
        if power_toughness.is_some() || !keywords.is_empty() {
            return Some(EquipmentRuleLineShape::StaticGrant {
                display_text: line.to_string(),
                power_toughness,
                scaled_power_toughness: None,
                keywords,
            });
        }
    }

    Some(EquipmentRuleLineShape::Other(line.to_string()))
}

pub(crate) fn parse_equipment_rule_lines_text(
    rules_text: &str,
) -> Option<Vec<EquipmentRuleLineShape>> {
    let mut shapes = Vec::new();
    for line in rules_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        shapes.push(classify_equipment_line(line)?);
    }
    (!shapes.is_empty()).then_some(shapes)
}

#[cfg(test)]
mod tests {
    use crate::object::CounterType;
    use crate::runtime_backend::token_definition::{
        EquipmentDamageGrantShape, EquipmentGrantCountShape, EquipmentScaledPowerToughnessShape,
    };

    use super::*;

    #[test]
    fn equipment_rules_preserve_quoted_grant_and_equip_cost() {
        let source = "Colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.";
        let shape = parse_equipment_rules_text(source).unwrap();
        assert!(
            shape
                .text
                .contains("This creature deals 2 damage to any target.")
        );
        assert!(shape.text.contains("Equip {1}"));
    }

    #[test]
    fn equipment_line_parser_returns_typed_damage_grant() {
        let lines = parse_equipment_rule_lines_text(
            "Equipped creature has \"{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target.\"\nEquip {1}",
        )
        .unwrap();
        assert!(matches!(
            &lines[0],
            EquipmentRuleLineShape::GrantedDamage {
                grant: EquipmentDamageGrantShape {
                    generic_amount: Some(1),
                    tap_cost: true,
                    sacrifice_equipment: true,
                    damage_amount: 2,
                },
                ..
            }
        ));
        assert_eq!(
            lines[1],
            EquipmentRuleLineShape::Equip(TokenEquipShape { amount: 1 })
        );
    }

    #[test]
    fn equipment_rules_preserve_counter_scaled_power_toughness() {
        let source = "Colorless Book Equipment artifact token named Guide with \"Equipped creature gets +1/+1 for each quest counter among permanents you control\" and equip {1}.";
        let shape = parse_equipment_rules_text(source).unwrap();
        assert!(matches!(
            shape.lines.as_slice(),
            [
                EquipmentRuleLineShape::StaticGrant {
                    power_toughness: None,
                    scaled_power_toughness: Some(EquipmentScaledPowerToughnessShape {
                        power: 1,
                        toughness: 1,
                        count: EquipmentGrantCountShape::CountersAmongPermanentsYouControl(
                            CounterType::Quest
                        ),
                    }),
                    ..
                },
                EquipmentRuleLineShape::Equip(TokenEquipShape { amount: 1 }),
            ]
        ));
        assert!(
            shape
                .text
                .contains("for each quest counter among permanents you control")
        );
    }
}
