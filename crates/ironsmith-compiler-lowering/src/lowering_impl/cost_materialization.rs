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
        count: u32,
        filter: ObjectFilter,
    },
    Untap,
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
        if let [CompilerCost::VariableMana { generic }] = branch.as_slice() {
            materialized.push(crate::compile_support::waterbend_optional_total_cost(
                *generic,
            ));
            continue;
        }
        if branch
            .iter()
            .any(|cost| matches!(cost, CompilerCost::VariableMana { .. }))
        {
            return Err(CardTextError::ParseError(
                "variable waterbend cost cannot be combined with another cost".to_string(),
            ));
        }
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
        CompilerCost::VariableMana { .. } => {
            unreachable!("variable mana is handled at the total-cost boundary")
        }
        CompilerCost::Tap => MaterializationCost::Tap,
        CompilerCost::TapChosen { count, filter } => MaterializationCost::TapChosen {
            count: *count,
            filter: filter.clone(),
        },
        CompilerCost::Untap => MaterializationCost::Untap,
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
            filter: filter.clone(),
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
        CompilerCost::Blight { count } => MaterializationCost::Blight { count: *count },
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
            ..
        } => MaterializationCost::RemoveCountersAmong {
            counter_type: *counter_type,
            count: *count,
            filter: filter.clone(),
            display_x: *display_x,
            dynamic: *dynamic,
            single_object: *single_object,
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
    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut Vec<Vec<ManaSymbol>>) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(ManaCost::from_pips(std::mem::take(pending))));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = Vec::new();
    let mut tap_tag_id = 0usize;
    let mut discard_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    let mut library_tag_id = 0usize;
    for segment in segments {
        match segment {
            MaterializationCost::Mana(cost) => {
                pending_mana_pips.extend(cost.pips().to_vec());
            }
            MaterializationCost::DynamicMana(cost) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::dynamic_mana(cost.clone()));
            }
            MaterializationCost::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            MaterializationCost::TapChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                filter.untapped = true;
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("tap_cost_{tap_tag_id}"));
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
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
                    costs.push(Cost::validated_effect(Effect::lose_life_player(
                        amount.clone(),
                        PlayerFilter::You,
                    )));
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
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!("discard_cost_{discard_tag_id}"));
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
            MaterializationCost::Blight { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("blight_cost_{tap_tag_id}"));
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    CounterType::MinusOneMinusOne,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            MaterializationCost::SacrificeSelf { surface } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
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
                let exact_count =
                    (!count.dynamic_x && count.max == Some(count.min)).then_some(count.min as u32);
                if let Some(exact_count) = exact_count {
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!("sacrifice_cost_{sacrifice_tag_id}"));
                    sacrifice_tag_id += 1;
                    costs.push(Cost::validated_effect(
                        Effect::sacrifice(filter, exact_count).tag(tag),
                    ));
                } else {
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!("sacrifice_cost_{sacrifice_tag_id}"));
                    sacrifice_tag_id += 1;
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        *count,
                        PlayerFilter::You,
                        tag.clone(),
                    )));
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
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("sacrifice_cost_{sacrifice_tag_id}"));
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
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("unattach_cost_{return_tag_id}"));
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
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.zone == Some(crate::zone::Zone::Battlefield)
                    && filter.controller.is_none()
                {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("exile_cost_{exile_tag_id}"));
                exile_tag_id += 1;
                let mut choose = crate::effects::ChooseObjectsEffect::new(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                );
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
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!("exile_cost_{exile_tag_id}"));
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
                    let tag = ironsmith_compiler_semantic::tag::declared_key(format!("exile_cost_{exile_tag_id}"));
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
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("return_cost_{return_tag_id}"));
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
            MaterializationCost::MoveChosenToLibraryTop { filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("library_cost_{library_tag_id}"));
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
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("library_cost_{library_tag_id}"));
                library_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter.clone(),
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Library,
                    false,
                )));
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
                let tag = ironsmith_compiler_semantic::tag::declared_key(format!("graveyard_cost_{return_tag_id}"));
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
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                let mut remove = if *dynamic {
                    crate::effects::RemoveAnyCountersAmongEffect::dynamic(
                        *count,
                        u32::MAX / 4,
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
