use crate::cards::builders::CardTextError;
use crate::effect::ChoiceCount;
use crate::model::{CompilerCost, CompilerTotalCost, CostRelationship};

use crate::grammar::activation_costs::{ActivationCostCst, ActivationCostSegmentCst};

/// Semantic boundary for activation costs. Grammar CST is consumed here and
/// the returned tree contains no runtime `Cost` or effect payload objects.
pub fn recognize_activation_cost_cst(
    cst: &ActivationCostCst,
) -> Result<CompilerTotalCost, CardTextError> {
    if let Some(generic) = cst.waterbend_generic {
        let mut total = CompilerTotalCost::ordered(vec![CompilerCost::VariableMana { generic }]);
        total.is_loyalty_shorthand = cst.is_loyalty_shorthand;
        return Ok(total);
    }

    if !cst.alternative_branches.is_empty() {
        let mut branches = Vec::with_capacity(cst.alternative_branches.len());
        for branch in &cst.alternative_branches {
            let branch = recognize_activation_cost_cst(branch)?;
            let [costs] = branch.branches.as_slice() else {
                return Err(CardTextError::ParseError(
                    "nested alternative activation cost is not supported".to_string(),
                ));
            };
            branches.push(costs.clone());
        }
        return Ok(CompilerTotalCost {
            branches,
            relationship: CostRelationship::Alternative,
            repeatable: false,
            is_loyalty_shorthand: cst.is_loyalty_shorthand,
            provenance: None,
        });
    }

    let costs = cst.segments.iter().map(recognize_segment).collect();
    Ok(CompilerTotalCost {
        branches: vec![costs],
        relationship: CostRelationship::Ordinary,
        repeatable: false,
        is_loyalty_shorthand: cst.is_loyalty_shorthand,
        provenance: None,
    })
}

fn recognize_segment(segment: &ActivationCostSegmentCst) -> CompilerCost {
    match segment {
        ActivationCostSegmentCst::Mana(cost) => CompilerCost::Mana(cost.clone()),
        ActivationCostSegmentCst::Tap => CompilerCost::Tap,
        ActivationCostSegmentCst::TapChosen { count, filter } => CompilerCost::TapChosen {
            count: *count,
            filter: filter.clone(),
        },
        ActivationCostSegmentCst::Untap => CompilerCost::Untap,
        ActivationCostSegmentCst::Life(amount) => CompilerCost::Life(amount.clone()),
        ActivationCostSegmentCst::Energy(amount) => CompilerCost::Energy(*amount),
        ActivationCostSegmentCst::DiscardSource => CompilerCost::DiscardSource,
        ActivationCostSegmentCst::DiscardHand => CompilerCost::DiscardHand,
        ActivationCostSegmentCst::DiscardCard(count) => CompilerCost::Discard {
            count: *count,
            card_types: Vec::new(),
            supertypes: Vec::new(),
            filter: None,
            random: false,
            name: None,
            other: false,
            binding: None,
        },
        ActivationCostSegmentCst::DiscardFiltered {
            count,
            card_types,
            supertypes,
            filter,
            random,
            name,
            other,
        } => CompilerCost::Discard {
            count: *count,
            card_types: card_types.clone(),
            supertypes: supertypes.clone(),
            filter: filter.clone(),
            random: *random,
            name: name.clone(),
            other: *other,
            binding: None,
        },
        ActivationCostSegmentCst::Mill(count) => CompilerCost::Mill(*count),
        ActivationCostSegmentCst::SacrificeSelf { surface } => CompilerCost::SacrificeSelf {
            surface: surface.clone(),
        },
        ActivationCostSegmentCst::SacrificeCreature => CompilerCost::Sacrifice {
            count: ChoiceCount::exactly(1),
            filter: crate::filter::ObjectFilter::creature(),
            all: false,
            binding: None,
        },
        ActivationCostSegmentCst::SacrificeChosen { count, filter } => CompilerCost::Sacrifice {
            count: *count,
            filter: filter.clone(),
            all: false,
            binding: None,
        },
        ActivationCostSegmentCst::SacrificeAll { filter } => CompilerCost::Sacrifice {
            count: ChoiceCount::any_number(),
            filter: filter.clone(),
            all: true,
            binding: None,
        },
        ActivationCostSegmentCst::UnattachChosen { count, filter } => CompilerCost::Unattach {
            count: *count,
            filter: filter.clone(),
        },
        ActivationCostSegmentCst::ExileSelf => CompilerCost::ExileSelf {
            from_graveyard: false,
        },
        ActivationCostSegmentCst::ExileSelfFromGraveyard => CompilerCost::ExileSelf {
            from_graveyard: true,
        },
        ActivationCostSegmentCst::ExileFromHand {
            count,
            color_filter,
        } => CompilerCost::ExileFromHand {
            count: *count,
            color_filter: *color_filter,
        },
        ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter,
            top_only,
            turn_face_up,
        } => CompilerCost::ExileChosen {
            count: *choice_count,
            filter: filter.clone(),
            top_only: *top_only,
            turn_face_up: *turn_face_up,
            binding: None,
        },
        ActivationCostSegmentCst::ExileSourceAndChosen {
            source_filter,
            choice_count,
            filter,
        } => CompilerCost::ExileSourceAndChosen {
            source_filter: source_filter.clone(),
            count: *choice_count,
            filter: filter.clone(),
        },
        ActivationCostSegmentCst::ExileSelfAndNamedArtifacts { names } => {
            CompilerCost::ExileSelfAndNamedArtifacts {
                names: names.clone(),
            }
        }
        ActivationCostSegmentCst::ExileTopLibrary { count } => {
            CompilerCost::ExileTopLibrary { count: *count }
        }
        ActivationCostSegmentCst::RevealSourceFromHand => CompilerCost::RevealSourceFromHand,
        ActivationCostSegmentCst::RevealFromHand {
            count,
            color_filter,
            card_type,
        } => CompilerCost::RevealFromHand {
            count: count.clone(),
            color_filter: *color_filter,
            card_type: *card_type,
            binding: None,
        },
        ActivationCostSegmentCst::ReturnSelfToHand => CompilerCost::ReturnSelfToHand,
        ActivationCostSegmentCst::ReturnChosenToHand { count, filter } => {
            CompilerCost::ReturnChosenToHand {
                count: *count,
                filter: filter.clone(),
            }
        }
        ActivationCostSegmentCst::MoveChosenToLibraryTop { filter } => {
            CompilerCost::MoveChosenToLibraryTop {
                filter: filter.clone(),
            }
        }
        ActivationCostSegmentCst::MoveSelfToLibraryBottom { surface } => {
            CompilerCost::MoveSelfToLibraryBottom {
                surface: surface.clone(),
            }
        }
        ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard => {
            CompilerCost::MoveOpponentOwnedExiledCardToGraveyard
        }
        ActivationCostSegmentCst::ExertSelf { display_text } => CompilerCost::ExertSelf {
            display: display_text.clone(),
        },
        ActivationCostSegmentCst::PutCounters {
            counter_type,
            count,
        } => CompilerCost::PutCounters {
            counter_type: *counter_type,
            count: *count,
            filter: None,
        },
        ActivationCostSegmentCst::PutCountersChosen {
            counter_type,
            count,
            filter,
        } => CompilerCost::PutCounters {
            counter_type: *counter_type,
            count: *count,
            filter: Some(filter.clone()),
        },
        ActivationCostSegmentCst::Blight { count } => CompilerCost::Blight { count: *count },
        ActivationCostSegmentCst::RemoveCounters {
            counter_type,
            count,
        } => CompilerCost::RemoveCounters {
            counter_type: Some(*counter_type),
            count: *count,
            filter: None,
            display_x: false,
            dynamic: false,
            single_object: true,
            remove_all: false,
        },
        ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter,
            display_x,
            dynamic,
            single_object,
        } => CompilerCost::RemoveCounters {
            counter_type: *counter_type,
            count: *count,
            filter: Some(filter.clone()),
            display_x: *display_x,
            dynamic: *dynamic,
            single_object: *single_object,
            remove_all: false,
        },
        ActivationCostSegmentCst::RemoveCountersDynamic {
            counter_type,
            display_x,
            remove_all,
        } => CompilerCost::RemoveCounters {
            counter_type: *counter_type,
            count: 0,
            filter: None,
            display_x: *display_x,
            dynamic: true,
            single_object: true,
            remove_all: *remove_all,
        },
        ActivationCostSegmentCst::Behold { subtype, count } => CompilerCost::Behold {
            subtype: *subtype,
            count: *count,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_sacrifice_cost_exports_the_paid_object_snapshot() {
        let cst =
            crate::grammar::activation_costs::parse_activation_cost_rewrite("Sacrifice a creature")
                .expect("activation cost should parse");
        let cost = recognize_activation_cost_cst(&cst).expect("activation cost should lower");
        let [component] = cost.costs().expect("ordinary cost") else {
            panic!("expected one sacrifice cost component: {cost:#?}");
        };
        assert!(
            matches!(component, crate::model::CompilerCost::Sacrifice { count, filter, .. }
                if *count == crate::effect::ChoiceCount::exactly(1)
                    && filter.card_types.contains(&crate::types::CardType::Creature)),
            "the compiler cost must retain the typed sacrifice: {component:#?}"
        );

        let imports =
            crate::util::compiler_activation_cost_reference_imports(&cost.to_core_total_cost());
        assert!(!imports.source_object_antecedent);
    }
}
