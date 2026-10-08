use crate::cards::builders::{CardTextError, ChoiceCount};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::{CompilerCost, CompilerTotalCost};
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::CardType;

#[derive(Debug, Clone, PartialEq)]
enum MaterializationCost {
    Mana(ManaCost),
    DynamicMana(ironsmith_core::DynamicManaCost),
    Tap,
    TapChosen {
        count: ChoiceCount,
        filter: ObjectFilter,
    },
    Untap,
    UntapChosen {
        count: ChoiceCount,
        filter: ObjectFilter,
    },
    Life(crate::effect::Value),
    Energy(u32),
    DiscardSource,
    DiscardHand,
    DiscardCard(u32),
    DiscardFiltered {
        count: u32,
        card_types: Vec<CardType>,
        supertypes: Vec<crate::types::Supertype>,
        filter: Option<ObjectFilter>,
        random: bool,
        name: Option<String>,
        other: bool,
    },
    Mill(u32),
    SacrificeSelf {
        surface: Option<crate::target::SourceReferenceSurface>,
    },
    SacrificeChosen {
        count: ChoiceCount,
        filter: ObjectFilter,
    },
    SacrificeAll {
        filter: ObjectFilter,
    },
    UnattachChosen {
        count: u32,
        filter: ObjectFilter,
    },
    ExileSelf,
    ExileSelfFromGraveyard,
    ExileFromHand {
        count: u32,
        color_filter: Option<crate::color::ColorSet>,
    },
    ExileChosen {
        choice_count: ChoiceCount,
        filter: ObjectFilter,
        top_only: bool,
        turn_face_up: bool,
    },
    ExileSourceAndChosen {
        source_filter: ObjectFilter,
        choice_count: ChoiceCount,
        filter: ObjectFilter,
    },
    ExileSelfAndNamedArtifacts {
        names: Vec<String>,
    },
    ExileTopLibrary {
        count: u32,
    },
    RevealChosenSubtype,
    RevealSourceFromHand,
    RevealSourceFromHandUntilUpkeepEnds,
    RevealFromHand {
        count: crate::effect::Value,
        color_filter: Option<crate::color::ColorSet>,
        card_type: Option<CardType>,
    },
    ReturnSelfToHand,
    ReturnChosenToHand {
        count: u32,
        filter: ObjectFilter,
    },
    MoveChosenToZone { filter: ObjectFilter, destination: crate::zone::Zone },
    MoveChosenToLibraryTop {
        filter: ObjectFilter,
    },
    MoveChosenToLibraryBottom {
        count: u32,
        filter: ObjectFilter,
    },
    MoveSelfToLibraryBottom {
        surface: crate::target::SourceReferenceSurface,
    },
    MoveOpponentOwnedExiledCardToGraveyard,
    ExertSelf {
        display_text: String,
    },
    EmitKeywordAction {
        kind: crate::events::KeywordActionKind,
        amount: u32,
    },
    Crew {
        amount: u32,
    },
    Teamwork {
        amount: u32,
    },
    Sneak,
    Effect(Box<crate::model::ast::EffectAst>),
    ValidatedEffect(Box<crate::model::ast::EffectAst>),
    PutCounters {
        counter_type: CounterType,
        count: u32,
    },
    PutCountersChosen {
        counter_type: CounterType,
        count: u32,
        filter: ObjectFilter,
    },
    Blight {
        count: u32,
        x: bool,
    },
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveCountersAmong {
        counter_type: Option<CounterType>,
        count: u32,
        filter: ObjectFilter,
        display_x: bool,
        dynamic: bool,
        single_object: bool,
        remove_all: bool,
    },
    RemoveCountersDynamic {
        counter_type: Option<CounterType>,
        display_x: bool,
        remove_all: bool,
    },
    Behold {
        subtype: crate::types::Subtype,
        count: u32,
    },
}

fn apply_activation_cost_default_battlefield_scope(filter: &mut ObjectFilter) {
    if !filter.any_of.is_empty() {
        for arm in &mut filter.any_of {
            apply_activation_cost_default_battlefield_scope(arm);
        }
        return;
    }
    if filter.controller.is_none() && filter.owner.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
}

/// The sole runtime allocation boundary for compiler-owned activation costs.
pub fn materialize_compiler_total_cost(
    cost: &CompilerTotalCost,
) -> Result<TotalCost, CardTextError> {
    let mut materialized = Vec::with_capacity(cost.branches.len());
    for branch in &cost.branches {
        let segments = branch.iter().map(materialization_cost).collect::<Vec<_>>();
        materialized.push(lower_materialization_costs(&segments)?);
    }
    match materialized.len() {
        0 => Ok(TotalCost::from_costs(Vec::new())),
        1 => Ok(materialized.remove(0)),
        _ => Ok(TotalCost::one_of(materialized)),
    }
}

/// Materialize the shared cost algebra when it is instantiated with the
/// compiler-owned cost component. This is used by compiler ability/static
/// nodes at the lowering boundary; recognition never receives the runtime
/// cost value.
/// "Sacrifice enchanted creature": a cost is paid before resolution, so no
/// `enchanted`/`equipped` tag has been seeded yet. The paid object is the one
/// this source is attached to.
fn bind_cost_attachment_reference_to_source(
    filter: &ObjectFilter,
) -> ObjectFilter {
    let mut filter = filter.clone();
    let attachment_noun = filter.tagged_constraints.iter().find_map(|constraint| {
        (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
            .then(|| match constraint.tag.as_str() {
                "enchanted" => Some("Aura"),
                "equipped" => Some("Equipment"),
                _ => None,
            })
            .flatten()
    });
    let before = filter.tagged_constraints.len();
    filter.tagged_constraints.retain(|constraint| {
        !(matches!(constraint.tag.as_str(), "enchanted" | "equipped")
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
    });
    if filter.tagged_constraints.len() != before && filter.with_attached_object.is_none() {
        // Keep the authored attachment noun so the cost still reads
        // "enchanted creature" / "equipped creature".
        let mut source = ObjectFilter::source();
        source.source_surface = attachment_noun.map(|noun| {
            ironsmith_core::SourceReferenceSurface::ThisPermanentType(noun.to_string())
        });
        filter.with_attached_object = Some(Box::new(source));
    }
    filter
}

fn identity_aware_activation_cost_scope(filter: &ObjectFilter) -> ObjectFilter {
    let mut filter = bind_cost_attachment_reference_to_source(filter);
    // An explicit attachment/source identity does not imply control by the
    // payer. The action is a written tap/untap instruction, not {T}/{Q}.
    let anchored = filter.source || filter.with_attached_object.is_some()
        || filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == ironsmith_compiler_semantic::tag::CompilerReferenceTag::GrantingSource.key()
        });
    if anchored {
        filter.zone.get_or_insert(crate::zone::Zone::Battlefield);
    } else {
        apply_activation_cost_default_battlefield_scope(&mut filter);
    }
    filter
}

pub fn materialize_compiler_core_total_cost(
    cost: &ironsmith_core::TotalCost<CompilerCost>,
) -> Result<TotalCost, CardTextError> {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(components) => {
            materialize_compiler_total_cost(&CompilerTotalCost::ordered(components.clone()))
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let branches = branches
                .iter()
                .map(materialize_compiler_core_total_cost)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TotalCost::one_of(branches))
        }
    }
}

fn materialization_cost(cost: &CompilerCost) -> MaterializationCost {
    match cost {
        CompilerCost::Mana(cost) => MaterializationCost::Mana(cost.clone()),
        CompilerCost::DynamicMana(cost) => MaterializationCost::DynamicMana(cost.clone()),
        CompilerCost::VariableMana { generic } => MaterializationCost::Mana(
            ManaCost::new().add_generic(*generic).with_waterbend(),
        ),
        CompilerCost::Tap => MaterializationCost::Tap,
        CompilerCost::TapChosen { count, filter } => MaterializationCost::TapChosen {
            count: *count,
            filter: filter.clone(),
        },
        CompilerCost::Untap => MaterializationCost::Untap,
        CompilerCost::UntapChosen { count, filter } => MaterializationCost::UntapChosen {
            count: *count,
            filter: filter.clone(),
        },
        CompilerCost::Life(amount) => MaterializationCost::Life(amount.clone()),
        CompilerCost::Energy(amount) => MaterializationCost::Energy(*amount),
        CompilerCost::DiscardSource => MaterializationCost::DiscardSource,
        CompilerCost::DiscardHand => MaterializationCost::DiscardHand,
        CompilerCost::Discard {
            count,
            card_types,
            supertypes,
            filter,
            random,
            name,
            other,
            ..
        } if card_types.is_empty()
            && supertypes.is_empty()
            && filter.is_none()
            && !*random
            && name.is_none()
            && !*other =>
        {
            MaterializationCost::DiscardCard(*count)
        }
        CompilerCost::Discard {
            count,
            card_types,
            supertypes,
            filter,
            random,
            name,
            other,
            ..
        } => MaterializationCost::DiscardFiltered {
            count: *count,
            card_types: card_types.clone(),
            supertypes: supertypes.clone(),
            filter: filter.clone(),
            random: *random,
            name: name.clone(),
            other: *other,
        },
        CompilerCost::Mill(count) => MaterializationCost::Mill(*count),
        CompilerCost::SacrificeSelf { surface } => MaterializationCost::SacrificeSelf {
            surface: surface.clone(),
        },
        CompilerCost::Sacrifice {
            count: _,
            filter,
            all: true,
            ..
        } => MaterializationCost::SacrificeAll {
            filter: filter.clone(),
        },
        CompilerCost::Sacrifice { count, filter, .. } => MaterializationCost::SacrificeChosen {
            count: *count,
            filter: bind_cost_attachment_reference_to_source(filter),
        },
        CompilerCost::Unattach { count, filter } => MaterializationCost::UnattachChosen {
            count: *count,
            filter: filter.clone(),
        },
        CompilerCost::ExileSelf { from_graveyard } => {
            if *from_graveyard {
                MaterializationCost::ExileSelfFromGraveyard
            } else {
                MaterializationCost::ExileSelf
            }
        }
        CompilerCost::ExileFromHand {
            count,
            color_filter,
        } => MaterializationCost::ExileFromHand {
            count: *count,
            color_filter: *color_filter,
        },
        CompilerCost::ExileChosen {
            count,
            filter,
            top_only,
            turn_face_up,
            ..
        } => MaterializationCost::ExileChosen {
            choice_count: *count,
            filter: filter.clone(),
            top_only: *top_only,
            turn_face_up: *turn_face_up,
        },
        CompilerCost::ExileSourceAndChosen {
            source_filter,
            count,
            filter,
        } => MaterializationCost::ExileSourceAndChosen {
            source_filter: source_filter.clone(),
            choice_count: *count,
            filter: filter.clone(),
        },
        CompilerCost::ExileSelfAndNamedArtifacts { names } => {
            MaterializationCost::ExileSelfAndNamedArtifacts {
                names: names.clone(),
            }
        }
        CompilerCost::ExileTopLibrary { count } => {
            MaterializationCost::ExileTopLibrary { count: *count }
        }
        CompilerCost::RevealChosenSubtype => MaterializationCost::RevealChosenSubtype,
        CompilerCost::RevealSourceFromHand => MaterializationCost::RevealSourceFromHand,
        CompilerCost::RevealSourceFromHandUntilUpkeepEnds => {
            MaterializationCost::RevealSourceFromHandUntilUpkeepEnds
        }
        CompilerCost::RevealFromHand {
            count,
            color_filter,
            card_type,
            ..
        } => MaterializationCost::RevealFromHand {
            count: count.clone(),
            color_filter: *color_filter,
            card_type: *card_type,
        },
        CompilerCost::ReturnSelfToHand => MaterializationCost::ReturnSelfToHand,
        CompilerCost::ReturnChosenToHand { count, filter } => {
            MaterializationCost::ReturnChosenToHand {
                count: *count,
                filter: filter.clone(),
            }
        }
        CompilerCost::MoveChosenToZone { filter, destination } => MaterializationCost::MoveChosenToZone { filter: filter.clone(), destination: *destination },
        CompilerCost::MoveChosenToLibraryTop { filter } => {
            MaterializationCost::MoveChosenToLibraryTop {
                filter: filter.clone(),
            }
        }
        CompilerCost::MoveChosenToLibraryBottom { count, filter } => {
            MaterializationCost::MoveChosenToLibraryBottom {
                count: *count,
                filter: filter.clone(),
            }
        }
        CompilerCost::MoveSelfToLibraryBottom { surface } => {
            MaterializationCost::MoveSelfToLibraryBottom {
                surface: surface.clone(),
            }
        }
        CompilerCost::MoveOpponentOwnedExiledCardToGraveyard => {
            MaterializationCost::MoveOpponentOwnedExiledCardToGraveyard
        }
        CompilerCost::ExertSelf { display } => MaterializationCost::ExertSelf {
            display_text: display.clone(),
        },
        CompilerCost::EmitKeywordAction { kind, amount } => {
            MaterializationCost::EmitKeywordAction {
                kind: *kind,
                amount: *amount,
            }
        }
        CompilerCost::Teamwork { amount } => MaterializationCost::Teamwork { amount: *amount },
        CompilerCost::Crew { amount } => MaterializationCost::Crew {
            amount: amount.clone(),
        },
        CompilerCost::Sneak => MaterializationCost::Sneak,
        CompilerCost::Effect(effect) => MaterializationCost::Effect(effect.clone()),
        CompilerCost::ValidatedEffect(effect) => {
            MaterializationCost::ValidatedEffect(effect.clone())
        }
        CompilerCost::PutCounters {
            counter_type,
            count,
            filter: None,
        } => MaterializationCost::PutCounters {
            counter_type: *counter_type,
            count: *count,
        },
        CompilerCost::PutCounters {
            counter_type,
            count,
            filter: Some(filter),
        } => MaterializationCost::PutCountersChosen {
            counter_type: *counter_type,
            count: *count,
            filter: filter.clone(),
        },
        CompilerCost::Blight { count, x } => MaterializationCost::Blight {
            count: *count,
            x: *x,
        },
        CompilerCost::RemoveCounters {
            counter_type: Some(counter_type),
            count,
            filter: None,
            dynamic: false,
            ..
        } => MaterializationCost::RemoveCounters {
            counter_type: *counter_type,
            count: *count,
        },
        CompilerCost::RemoveCounters {
            counter_type,
            count,
            filter: Some(filter),
            display_x,
            dynamic,
            single_object,
            remove_all,
        } => MaterializationCost::RemoveCountersAmong {
            counter_type: *counter_type,
            count: *count,
            filter: filter.clone(),
            display_x: *display_x,
            dynamic: *dynamic,
            single_object: *single_object,
            remove_all: *remove_all,
        },
        CompilerCost::RemoveCounters {
            counter_type,
            display_x,
            remove_all,
            ..
        } => MaterializationCost::RemoveCountersDynamic {
            counter_type: *counter_type,
            display_x: *display_x,
            remove_all: *remove_all,
        },
        CompilerCost::Behold { subtype, count } => MaterializationCost::Behold {
            subtype: *subtype,
            count: *count,
        },
    }
}

fn lower_materialization_costs(
    segments: &[MaterializationCost],
) -> Result<TotalCost, CardTextError> {
    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut ManaCost) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(std::mem::take(pending)));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = ManaCost::new();
    let mut tap_tag_id = 0usize;
    let mut untap_tag_id = 0usize;
    let mut discard_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    let mut library_tag_id = 0usize;
    // "Sacrifice this creature and any number of other ... : ... for each
    // creature sacrificed this way" (Emrakul's Evangel): the source joins the
    // chosen set, so the count of permanents sacrificed this way includes it.
    let mut source_joins_next_chosen_sacrifice = false;
    for (segment_index, segment) in segments.iter().enumerate() {
        match segment {
            MaterializationCost::Mana(cost) => {
                pending_mana_pips = pending_mana_pips.combined_with(cost);
            }
            MaterializationCost::DynamicMana(cost) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut cost = cost.clone();
                if matches!(cost.mana_cost_of.as_deref().map(crate::target::ChooseSpec::base),
                    Some(crate::target::ChooseSpec::Tagged(tag)) if *tag == ironsmith_compiler_semantic::tag::CompilerReferenceTag::It.key()) {
                    // "Exile a card and pay its mana cost": bind the preceding
                    // declaration, not a source-name or a guessed graveyard card.
                    let tag = costs.iter().rev().filter_map(|component| component.effect_ref())
                        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
                        .map(|choose| choose.tag.clone())
                        .ok_or_else(|| CardTextError::ParseError("referenced mana cost has no preceding cost-object choice".into()))?;
                    cost.mana_cost_of = Some(Box::new(crate::target::ChooseSpec::tagged(tag)));
                }
                costs.push(Cost::dynamic_mana(cost));
            }
            MaterializationCost::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            MaterializationCost::TapChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = identity_aware_activation_cost_scope(filter);
                filter.untapped = true;
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "tap_cost_{tap_tag_id}"
                ));
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    *count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            MaterializationCost::UntapChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = identity_aware_activation_cost_scope(filter);
                filter.tapped = true;
                let tag = ironsmith_compiler_semantic::tag::CompilerCostObjectTag::Untap.key(untap_tag_id);
                untap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter, *count, PlayerFilter::You, tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::untap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            MaterializationCost::Untap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::untap());
            }
            MaterializationCost::Life(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if matches!(amount, crate::effect::Value::Fixed(_)) {
                    costs.push(Cost::life(amount.clone()));
                } else {
                    // Dynamic life is still a payment, not an ordinary
                    // life-loss instruction. PayLife retains cost validation
                    // and its source snapshot (notably Ward after departure).
                    costs.push(Cost::validated_effect(Effect::pay_life(amount.clone())));
                }
            }
            MaterializationCost::Energy(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::energy(*amount));
            }
            MaterializationCost::DiscardSource => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_source());
            }
            MaterializationCost::DiscardHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_hand());
            }
            MaterializationCost::DiscardCard(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard(*count, None));
            }
            MaterializationCost::DiscardFiltered {
                count,
                card_types,
                supertypes,
                filter,
                random,
                name,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *random || name.is_some() || *other || filter.is_some() || !supertypes.is_empty()
                {
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                        "discard_cost_{discard_tag_id}"
                    ));
                    discard_tag_id += 1;
                    let card_filter = if let Some(filter) = filter {
                        Some(filter.clone())
                    } else if card_types.is_empty()
                        && supertypes.is_empty()
                        && name.is_none()
                        && !*other
                    {
                        None
                    } else {
                        let mut filter = ObjectFilter {
                            zone: Some(crate::zone::Zone::Hand),
                            card_types: card_types.clone(),
                            supertypes: supertypes.clone(),
                            ..Default::default()
                        };
                        if let Some(name) = name {
                            filter = filter.named(name.clone());
                        }
                        if *other {
                            filter.other = true;
                        }
                        Some(filter)
                    };
                    costs.push(Cost::validated_effect(Effect::new(
                        crate::effects::DiscardEffect::new_with_filter(
                            *count as i32,
                            PlayerFilter::You,
                            *random,
                            card_filter,
                        )
                        .with_tag(tag),
                    )));
                } else if card_types.len() > 1 {
                    costs.push(Cost::discard_types(*count, card_types.clone()));
                } else if let Some(card_type) = card_types.first().copied() {
                    costs.push(Cost::discard(*count, Some(card_type)));
                } else {
                    costs.push(Cost::discard(*count, None));
                }
            }
            MaterializationCost::Mill(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::mill(*count));
            }
            MaterializationCost::Behold { subtype, count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::behold(*subtype, *count)));
            }
            MaterializationCost::Blight { count, x } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                // Keep activation-cost antecedent indices aligned with the grammar.
                tap_tag_id += 1;
                // "Blight X" puts the announced X counters (CR 601.2b, 601.2h).
                let amount = if *x {
                    crate::effect::Value::X
                } else {
                    crate::effect::Value::Fixed(*count as i32)
                };
                costs.push(Cost::validated_effect(Effect::new(
                    crate::effects::PutCountersEffect::new(
                        CounterType::MinusOneMinusOne,
                        amount,
                        crate::target::ChooseSpec::Object(ObjectFilter::creature().you_control()),
                    )
                    .with_completion_action(crate::events::KeywordActionKind::Blight),
                )));
            }
            MaterializationCost::SacrificeSelf { surface } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if surface.is_none()
                    && matches!(
                        segments.get(segment_index + 1),
                        Some(MaterializationCost::SacrificeChosen { count, .. })
                            if count.dynamic_x || count.max != Some(count.min)
                    )
                {
                    source_joins_next_chosen_sacrifice = true;
                    continue;
                }
                if let Some(surface) = surface {
                    costs.push(Cost::validated_effect(Effect::new(
                        crate::effects::SacrificeTargetEffect::new(
                            crate::model::ast::source_choose_spec_for_surface(surface.clone()),
                        ),
                    )));
                } else {
                    costs.push(Cost::sacrifice_self());
                }
            }
            MaterializationCost::SacrificeChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                // Only permanents can be sacrificed (CR 701.21a); the choice
                // needs its search zone ("Sacrifice X Goats").
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let exact_count =
                    (!count.dynamic_x && count.max == Some(count.min)).then_some(count.min as u32);
                if let Some(exact_count) = exact_count {
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                        "sacrifice_cost_{sacrifice_tag_id}"
                    ));
                    sacrifice_tag_id += 1;
                    costs.push(Cost::validated_effect(
                        Effect::sacrifice(filter, exact_count).tag(tag),
                    ));
                } else {
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                        "sacrifice_cost_{sacrifice_tag_id}"
                    ));
                    sacrifice_tag_id += 1;
                    if std::mem::take(&mut source_joins_next_chosen_sacrifice) {
                        let mut source = ObjectFilter::source();
                        source.zone = Some(crate::zone::Zone::Battlefield);
                        costs.push(Cost::validated_effect(Effect::new(
                            crate::effects::ChooseObjectsEffect::new(
                                source,
                                ChoiceCount::exactly(1),
                                PlayerFilter::You,
                                tag.clone(),
                            ),
                        )));
                    }
                    let aggregate_constraint = filter.target_set_aggregate_constraint.take();
                    let mut choose = crate::effects::ChooseObjectsEffect::new(
                        filter,
                        *count,
                        PlayerFilter::You,
                        tag.clone(),
                    );
                    if let Some(constraint) = aggregate_constraint {
                        choose = choose.with_aggregate_constraint(*constraint);
                    }
                    costs.push(Cost::validated_effect(Effect::new(choose)));
                    costs.push(Cost::validated_effect(Effect::sacrifice_player(
                        ObjectFilter::tagged(tag.clone()),
                        crate::effect::Value::Count(ObjectFilter::tagged(tag)),
                        PlayerFilter::You,
                    )));
                }
            }
            MaterializationCost::SacrificeAll { filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "sacrifice_cost_{sacrifice_tag_id}"
                ));
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(
                    Effect::sacrifice_player(
                        filter.clone(),
                        crate::effect::Value::Count(filter),
                        PlayerFilter::You,
                    )
                    .tag(tag),
                ));
            }
            MaterializationCost::UnattachChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "unattach_cost_{return_tag_id}"
                ));
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::unattach_objects(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            MaterializationCost::ExileSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            MaterializationCost::ExileSelfFromGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            MaterializationCost::ExileFromHand {
                count,
                color_filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_from_hand(*count, *color_filter));
            }
            MaterializationCost::ExileChosen {
                choice_count,
                filter,
                top_only,
                turn_face_up,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                // A union whose branches name their own zones ("permanents
                // you control and/or cards in your graveyard", craft
                // materials) searches each branch zone; an outer zone or
                // controller would exclude every other branch.
                let mut branch_zones = Vec::new();
                if filter.zone.is_none()
                    && !filter.any_of.is_empty()
                    && filter.any_of.iter().all(|branch| branch.zone.is_some())
                {
                    for zone in filter.any_of.iter().filter_map(|branch| branch.zone) {
                        if !branch_zones.contains(&zone) {
                            branch_zones.push(zone);
                        }
                    }
                }
                if branch_zones.len() < 2 {
                    branch_zones.clear();
                    if filter.zone.is_none() {
                        filter.zone = Some(crate::zone::Zone::Battlefield);
                    }
                    if filter.zone == Some(crate::zone::Zone::Battlefield)
                        && filter.controller.is_none()
                    {
                        filter.controller = Some(PlayerFilter::You);
                    }
                }
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "exile_cost_{exile_tag_id}"
                ));
                exile_tag_id += 1;
                let aggregate_constraint = filter.target_set_aggregate_constraint.take();
                let mut choose = crate::effects::ChooseObjectsEffect::new(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                );
                if !branch_zones.is_empty() {
                    choose = choose.in_zones(branch_zones);
                }
                if let Some(constraint) = aggregate_constraint {
                    choose = choose.with_aggregate_constraint(*constraint);
                }
                if *top_only {
                    choose = choose.top_only();
                }
                costs.push(Cost::validated_effect(Effect::new(choose)));
                let exile =
                    crate::effects::ExileEffect::with_spec(crate::target::ChooseSpec::tagged(tag));
                let exile = if *turn_face_up {
                    exile.turn_face_up()
                } else {
                    exile
                };
                costs.push(Cost::validated_effect(Effect::new(exile)));
            }
            MaterializationCost::ExileSourceAndChosen {
                source_filter,
                choice_count,
                filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                for (mut filter, count) in [
                    (source_filter.clone(), ChoiceCount::exactly(1)),
                    (filter.clone(), *choice_count),
                ] {
                    if filter.zone.is_none() {
                        filter.zone = Some(crate::zone::Zone::Battlefield);
                    }
                    if filter.zone == Some(crate::zone::Zone::Battlefield)
                        && filter.controller.is_none()
                        && !filter.source
                    {
                        filter.controller = Some(PlayerFilter::You);
                    }
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                        "exile_cost_{exile_tag_id}"
                    ));
                    exile_tag_id += 1;
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        count,
                        PlayerFilter::You,
                        tag.clone(),
                    )));
                    costs.push(Cost::validated_effect(Effect::exile(
                        crate::target::ChooseSpec::tagged(tag),
                    )));
                }
            }
            MaterializationCost::ExileSelfAndNamedArtifacts { names } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
                for name in names {
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                        "exile_cost_{exile_tag_id}"
                    ));
                    exile_tag_id += 1;
                    let mut filter = ObjectFilter {
                        zone: Some(crate::zone::Zone::Battlefield),
                        controller: Some(PlayerFilter::You),
                        card_types: vec![CardType::Artifact],
                        ..Default::default()
                    };
                    filter.name = Some(name.clone());
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        ChoiceCount::exactly(1),
                        PlayerFilter::You,
                        tag.clone(),
                    )));
                    costs.push(Cost::validated_effect(Effect::exile(
                        crate::target::ChooseSpec::tagged(tag),
                    )));
                }
            }
            MaterializationCost::ExileTopLibrary { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                    crate::tag::CompilerReferenceTag::CostExiledTop.bind(),
                    None,
                )));
            }
            MaterializationCost::RevealChosenSubtype => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::new(
                    crate::effects::RevealChosenSubtypeEffect,
                )));
            }
            MaterializationCost::RevealSourceFromHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_source_from_hand()));
            }
            MaterializationCost::RevealSourceFromHandUntilUpkeepEnds => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(
                    Effect::reveal_source_from_hand_until_upkeep_ends(),
                ));
            }
            MaterializationCost::RevealFromHand {
                count,
                color_filter,
                card_type,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_from_hand(
                    count.clone(),
                    *card_type,
                    *color_filter,
                )));
            }
            MaterializationCost::ReturnSelfToHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::return_self_to_hand());
            }
            MaterializationCost::ReturnChosenToHand { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "return_cost_{return_tag_id}"
                ));
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::return_to_hand(
                    ObjectFilter::tagged(tag),
                )));
            }
            MaterializationCost::MoveChosenToZone { filter, destination } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("zone_cost_{return_tag_id}"));
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(filter.clone(), ChoiceCount::exactly(1), PlayerFilter::You, tag.clone())));
                costs.push(Cost::validated_effect(Effect::move_to_zone(crate::target::ChooseSpec::tagged(tag), *destination, false)));
            }
            MaterializationCost::MoveChosenToLibraryTop { filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "library_cost_{library_tag_id}"
                ));
                library_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter.clone(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Library,
                    true,
                )));
            }
            MaterializationCost::MoveChosenToLibraryBottom { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "library_cost_{library_tag_id}"
                ));
                library_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter.clone(),
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::new(crate::effects::MoveToZoneEffect::new(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Library,
                    false,
                ).with_library_order(ironsmith_core::LibraryPlacementOrder::Owners))));
            }
            MaterializationCost::MoveSelfToLibraryBottom { surface } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::model::ast::source_choose_spec_for_surface(surface.clone()),
                    crate::zone::Zone::Library,
                    false,
                )));
            }
            MaterializationCost::MoveOpponentOwnedExiledCardToGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!(
                    "graveyard_cost_{return_tag_id}"
                ));
                return_tag_id += 1;
                let filter = ObjectFilter {
                    zone: Some(crate::zone::Zone::Exile),
                    owner: Some(PlayerFilter::Opponent),
                    ..Default::default()
                };
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Graveyard,
                    false,
                )));
            }
            MaterializationCost::ExertSelf { display_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(crate::effects::ExertCostEffect::new(
                    display_text.clone(),
                )));
            }
            MaterializationCost::EmitKeywordAction { kind, amount } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::emit_keyword_action(
                    *kind, *amount,
                )));
            }
            MaterializationCost::Teamwork { amount } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::new(
                    crate::effects::CrewCostEffect::teamwork(*amount),
                )));
            }
            MaterializationCost::Crew { amount } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::new(
                    crate::effects::CrewCostEffect::new(*amount),
                )));
            }
            MaterializationCost::Sneak => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::new(
                    crate::effects::SneakCostEffect::new(),
                )));
            }
            MaterializationCost::Effect(effect) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(
                    crate::lowering_support::lower_compiler_child_effect((**effect).clone())?,
                ));
            }
            MaterializationCost::ValidatedEffect(effect) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(
                    crate::lowering_support::lower_compiler_child_effect((**effect).clone())?,
                ));
            }
            MaterializationCost::PutCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::add_counters(*counter_type, *count));
            }
            MaterializationCost::PutCountersChosen {
                counter_type,
                count,
                filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                if filter.source {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                costs.push(Cost::validated_effect(Effect::put_counters(
                    *counter_type,
                    *count as i32,
                    crate::target::ChooseSpec::Object(filter),
                )));
            }
            MaterializationCost::RemoveCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_counters(*counter_type, *count));
            }
            MaterializationCost::RemoveCountersAmong {
                counter_type,
                count,
                filter,
                display_x,
                dynamic,
                single_object,
                remove_all,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let filter = identity_aware_activation_cost_scope(filter);
                if *remove_all {
                    let kind = counter_type.ok_or_else(|| CardTextError::ParseError("scoped all-counter cost requires a named counter kind".into()))?;
                    let grantor = crate::tag::CompilerReferenceTag::GrantingSource.key();
                    if filter != ObjectFilter::tagged(grantor.clone()).in_zone(crate::zone::Zone::Battlefield) {
                        return Err(CardTextError::ParseError("scoped all-counter cost requires an exact granting object".into()));
                    }
                    let target = crate::target::ChooseSpec::Tagged(grantor);
                    let count = crate::effect::Value::CountersOn(Box::new(target.clone()), Some(kind));
                    costs.push(Cost::validated_effect(Effect::remove_counters(kind, count, target)));
                    continue;
                }
                let mut remove = if *dynamic {
                    crate::effects::RemoveAnyCountersAmongEffect::dynamic(
                        *count,
                        u32::MAX,
                        filter,
                        *display_x,
                    )
                } else {
                    crate::effects::RemoveAnyCountersAmongEffect::new(*count, filter)
                }
                .with_counter_type(*counter_type);
                if *single_object {
                    remove = remove.from_single_object();
                }
                costs.push(Cost::validated_effect(Effect::new(remove)));
            }
            MaterializationCost::RemoveCountersDynamic {
                counter_type,
                display_x,
                remove_all,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let cost = if *remove_all {
                    Cost::remove_all_counters_from_source(*counter_type)
                } else {
                    Cost::remove_any_counters_from_source(*counter_type, *display_x)
                };
                costs.push(cost);
            }
        }
    }
    flush_pending_mana(&mut costs, &mut pending_mana_pips);
    Ok(TotalCost::from_costs(costs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_life_materializes_as_payment_with_its_exact_typed_amount() {
        for amount in [
            crate::effect::Value::SourcePower,
            crate::effect::Value::PowerOf(Box::new(crate::target::ChooseSpec::Source)),
            crate::effect::Value::CardsInHand(PlayerFilter::You),
        ] {
            let cost = lower_materialization_costs(&[MaterializationCost::Life(amount.clone())])
                .expect("dynamic payment lowers");
            let [component] = cost.costs() else {
                panic!("one life cost");
            };
            let effect = component.effect_ref().expect("dynamic cost effect");
            let payment = effect
                .downcast_ref::<crate::effects::PayLifeEffect>()
                .expect("life cost must retain payment semantics");
            assert_eq!(payment.amount, amount);
            assert!(
                effect
                    .downcast_ref::<crate::effects::LoseLifeEffect>()
                    .is_none()
            );
        }
    }
}

/// Only activation costs publish this producer; optional/body costs keep their
/// original ownership and cannot overwrite the activation's retained result.
pub fn materialize_compiler_activation_total_cost(cost: &ironsmith_core::TotalCost<CompilerCost>)
    -> Result<TotalCost, CardTextError>
{
    let lowered = materialize_compiler_core_total_cost(cost)?;
    let Some(producer) = crate::model::costs::unique_counter_removal_cost(cost) else { return Ok(lowered); };
    let components = lowered.as_all().ok_or_else(|| CardTextError::InvariantViolation("counter producer lost ordinary cost branch".into()))?;
    let mut count = 0;
    let components = components.iter().map(|component| {
        let effect = match component {
            Cost::RemoveCounters { counter_type, count } => Some(Effect::remove_counters(*counter_type, *count, crate::target::ChooseSpec::Source)),
            Cost::RemoveAnyCountersFromSource { counter_type, display_x, remove_all } => Some(Effect::new(
                crate::effects::RemoveAnyCountersFromSourceEffect { counter_type: *counter_type, display_x: *display_x, remove_all: *remove_all })),
            Cost::Effect(effect) if effect.downcast_ref::<crate::effects::RemoveCountersEffect>().is_some()
                || effect.downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>().is_some()
                || effect.downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>().is_some() => Some(effect.clone()),
            _ => None,
        };
        if let Some(effect) = effect {
            count += 1;
            Cost::validated_effect(Effect::with_id(producer.effect_id.0, effect))
        } else { component.clone() }
    }).collect();
    if count != 1 { return Err(CardTextError::InvariantViolation("counter producer must materialize exactly once".into())); }
    Ok(TotalCost::from_costs(components))
}
