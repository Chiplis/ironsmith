use super::*;

pub(super) fn try_parse_chosen_type_behold_two_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    let words = crate::lexer::parser_token_word_refs(parse_tokens);
    if line.kind != RewriteKeywordLineKind::AdditionalCost
        || !crate::word_primitives::parse_any_sequence_complete(
            &words,
            &[
                &[
                    "as",
                    "an",
                    "additional",
                    "cost",
                    "to",
                    "cast",
                    "this",
                    "spell",
                    "you",
                    "may",
                    "choose",
                    "a",
                    "creature",
                    "type",
                    "and",
                    "behold",
                    "two",
                    "cards",
                    "of",
                    "that",
                    "type",
                ],
                &[
                    "as",
                    "an",
                    "additional",
                    "cost",
                    "to",
                    "cast",
                    "this",
                    "spell",
                    "you",
                    "may",
                    "choose",
                    "a",
                    "creature",
                    "type",
                    "and",
                    "behold",
                    "two",
                    "creatures",
                    "of",
                    "that",
                    "type",
                ],
            ],
        )
    {
        return None;
    }

    let mut battlefield = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    battlefield.chosen_creature_type = true;
    let mut hand = ObjectFilter::default()
        .owned_by(PlayerFilter::You)
        .in_zone(Zone::Hand);
    hand.chosen_creature_type = true;
    let total_cost = ironsmith_core::TotalCost::from_costs(vec![
        crate::model::CompilerCost::ValidatedEffect(Box::new(
            EffectAst::subject_verb_choose_creature_type(PlayerAst::You, Vec::new()),
        )),
        crate::model::CompilerCost::ValidatedEffect(Box::new(EffectAst::ChooseObjects {
            filter: ObjectFilter {
                any_of: vec![battlefield, hand],
                ..Default::default()
            },
            count: crate::effect::ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::BeheldChosenType.key(),
        })),
    ]);
    let mut optional_cost = OptionalCost::custom(line.info.raw_line.trim(), total_cost);
    optional_cost.reference =
        crate::cost::OptionalCostRef::new(crate::cost::OptionalCostKind::Additional);
    Some(LineAst::OptionalCost(optional_cost.into()))
}

pub fn rewrite_modal_to_parsed_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let header_text = modal.header.raw_line.clone();
    let Some(mut header) = parse_modal_header(&modal.header, &modal.header_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    if let Some(replacement) = header.x_replacement.as_ref() {
        replace_modal_header_x_in_effects_ast(
            &mut header.common_prefix_effects_ast,
            replacement,
            header_text.as_str(),
        )?;
    }

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info.semantic_info(),
            description: mode.text,
            point_cost: mode.point_cost,
            additional_mana_cost: mode.additional_mana_cost,
            effects_ast,
        });
    }

    specialize_modal_common_target_suffix(
        &mut modes,
        &header.common_suffix_effects_ast,
        header_text.as_str(),
    )?;

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

/// Specialize a demonstrative modal-header suffix into every bare target
/// mode. The shared clause supplies zone/controller facts, while each bullet
/// supplies its own target characteristic. Each resulting mode therefore
/// owns a complete executable target action, and the modal model separately
/// records that its trailing action was authored only once.
pub(super) fn specialize_modal_common_target_suffix(
    modes: &mut [ParsedModalModeAst],
    suffix: &[EffectAst],
    header_text: &str,
) -> Result<(), CardTextError> {
    if suffix.is_empty() {
        return Ok(());
    }
    let [EffectAst::SubjectVerb(common)] = suffix else {
        return Err(CardTextError::ParseError(format!(
            "unsupported modal common suffix in '{header_text}'"
        )));
    };
    let SubjectVerbActionAst::ReturnToHand {
        target: TargetAst::Object(common_filter, _, _),
        ..
    } = &common.action
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported modal common target action in '{header_text}'"
        )));
    };

    for mode in modes {
        let [EffectAst::SubjectVerb(mode_target)] = mode.effects_ast.as_slice() else {
            return Err(CardTextError::ParseError(format!(
                "modal common target suffix requires one bare target per mode in '{header_text}'"
            )));
        };
        let SubjectVerbActionAst::TargetOnly {
            target: TargetAst::Object(mode_filter, target_span, object_span),
            ..
        } = &mode_target.action
        else {
            return Err(CardTextError::ParseError(format!(
                "modal common target suffix requires object target modes in '{header_text}'"
            )));
        };

        let mut specialized = common.clone();
        let SubjectVerbActionAst::ReturnToHand { target, .. } = &mut specialized.action else {
            unreachable!("common suffix action was validated above");
        };
        *target = TargetAst::Object(
            merge_filters(common_filter, mode_filter),
            *target_span,
            *object_span,
        );
        mode.effects_ast = vec![EffectAst::SubjectVerb(specialized)];
    }
    Ok(())
}
