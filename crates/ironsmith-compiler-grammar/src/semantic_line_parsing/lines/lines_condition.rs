use super::*;

pub fn parse_gift_keyword_line(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    let spec =
        semantic_grammar::parse_standard_gift_spec_tokens(&line.parse_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "rewrite keyword lowering could not parse gift line '{}'",
                line.info.raw_line
            ))
        })?;
    let cost = OptionalCost::custom(
        line.info.raw_line.trim(),
        ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Effect(Box::new(
            EffectAst::subject_verb_choose_player(
                PlayerAst::You,
                PlayerFilter::Opponent,
                crate::tag::CompilerReferenceTag::GiftedPlayer.bind(),
                false,
                0,
            ),
        ))),
    );

    Ok(LineAst::GiftKeyword {
        cost,
        effects: standard_gift_effects(spec.variant),
        timing: spec.timing,
    })
}

pub(super) fn standard_gift_effects(
    variant: semantic_grammar::StandardGiftVariant,
) -> Vec<EffectAst> {
    match variant {
        semantic_grammar::StandardGiftVariant::Card => vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Chosen,
            SubjectVerbActionAst::Draw {
                count: crate::effect::Value::Fixed(1),
            },
        )],
        semantic_grammar::StandardGiftVariant::Treasure => {
            vec![standard_gift_create_token_effect(
                "Treasure",
                crate::model::token_definition::TokenDefinitionSpec::Builtin(
                    crate::model::token_definition::BuiltinTokenShape::Treasure,
                ),
                false,
            )]
        }
        semantic_grammar::StandardGiftVariant::Food => {
            vec![standard_gift_create_token_effect(
                "Food",
                crate::model::token_definition::TokenDefinitionSpec::Builtin(
                    crate::model::token_definition::BuiltinTokenShape::Food,
                ),
                false,
            )]
        }
        semantic_grammar::StandardGiftVariant::TappedFish => {
            vec![standard_gift_create_token_effect(
                "1/1 blue Fish creature",
                fixed_standard_gift_creature_definition(
                    "Fish",
                    Subtype::Fish,
                    ColorSet::BLUE,
                    (1, 1),
                ),
                true,
            )]
        }
        semantic_grammar::StandardGiftVariant::ExtraTurn => {
            vec![EffectAst::subject_verb_extra_turn_after_turn(
                PlayerAst::Chosen,
                crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
            )]
        }
        semantic_grammar::StandardGiftVariant::Octopus => vec![standard_gift_create_token_effect(
            "8/8 blue Octopus creature",
            fixed_standard_gift_creature_definition(
                "Octopus",
                Subtype::Octopus,
                ColorSet::BLUE,
                (8, 8),
            ),
            false,
        )],
    }
}

pub(super) fn fixed_standard_gift_creature_definition(
    name: &str,
    subtype: Subtype,
    colors: ColorSet,
    power_toughness: (i32, i32),
) -> crate::model::token_definition::TokenDefinitionSpec {
    crate::model::token_definition::TokenDefinitionSpec::Creature(
        crate::model::token_definition::CreatureTokenShape {
            name: name.to_string(),
            card_types: vec![CardType::Creature],
            subtypes: vec![subtype],
            power_toughness,
            legendary: false,
            colors,
            use_source_chosen_color: false,
            use_source_chosen_creature_type: false,
            keywords: Vec::new(),
            rules: Default::default(),
        },
    )
}
