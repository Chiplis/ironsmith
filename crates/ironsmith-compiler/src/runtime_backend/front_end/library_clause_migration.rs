//! Finite PR-25 adapter from library recipe leaves to common compiler clauses.
//!
//! The adapter is intentionally placed at the semantic-document boundary:
//! library sentence modules may still recognize legacy shapes internally, but
//! no such shape leaves the front end. PR-31 removes the now-dead recipe AST
//! variants after lowering consumes these clauses directly.

use crate::model::ast::{EffectAst, SubjectVerbActionAst};
use crate::model::clauses::{
    ClauseActionAst, ClauseActorAst, ClauseDestinationAst, ClauseDestinationRelationAst,
    ClausePolarityAst, ClauseSubjectAst, ClauseVerbAst, ClauseZonePlacementAst, CompilerClauseAst,
    CompilerPlayerAst,
};
use crate::model::compiler_semantic::{
    LineAst, ParsedCardItem, ParsedLevelAbilityItemAst, ParsedModalAst,
};
use crate::model::coordination::{
    CarriedFactAst, CoordinationAst, CoordinationBoundaryAst, CoordinationCarryAst,
    CoordinationKindAst, CoordinationMemberAst, CoordinationOperatorAst, EffectDependencyAst,
    EffectOrderingAst,
};
use crate::model::library_clauses::{
    CompilerLibraryClauseAst, LibraryExposureAst, LibraryOrderAst, LibraryPositionAst,
    LibraryRemainderAst, LibraryResultBindingAst, LibraryResultKindAst, LibrarySelectionAst,
    LibrarySelectionModeAst,
};
use crate::model::parse_types::{LibraryBottomOrderAst, LibraryConsultModeAst, PlayerAst};
use crate::model::selections::{CompilerFilterAst, CompilerValueAst};
use crate::model::symbols::{Cardinality, ReferenceRole, SymbolReference, SymbolResolutionError};
use crate::model::visit::for_each_nested_effect_vec_mut;
use crate::runtime_backend::front_end::semantic_migration_context::SemanticMigrationContext;
use crate::tag::TagKey;
use crate::zone::Zone;

struct LibraryMigration<'migration, 'symbols> {
    context: &'migration mut SemanticMigrationContext<'symbols>,
}

impl<'migration, 'symbols> LibraryMigration<'migration, 'symbols> {
    fn new(context: &'migration mut SemanticMigrationContext<'symbols>) -> Self {
        Self { context }
    }

    fn migrate_effects(
        &mut self,
        effects: &mut Vec<EffectAst>,
    ) -> Result<(), SymbolResolutionError> {
        for effect in effects.iter_mut() {
            self.migrate_effect(effect)?;
        }
        if effects.len() > 1
            && effects.iter().any(
                |effect| matches!(effect, EffectAst::Clause(clause) if clause.library.is_some()),
            )
        {
            let members: Vec<CoordinationMemberAst> = std::mem::take(effects)
                .into_iter()
                .map(library_coordination_member)
                .collect();
            let boundaries = (1..members.len())
                .map(|member| {
                    let carries = members[member]
                        .imports
                        .iter()
                        .find_map(|reference| {
                            (0..member).rev().find_map(|from_member| {
                                members[from_member].exports.contains(reference).then_some(
                                    CoordinationCarryAst {
                                        from_member,
                                        to_member: member,
                                        fact: CarriedFactAst::Reference(Some(*reference)),
                                    },
                                )
                            })
                        })
                        .into_iter()
                        .collect();
                    CoordinationBoundaryAst {
                        operator: CoordinationOperatorAst::SentenceBoundary,
                        ordering: EffectOrderingAst::Ordered,
                        dependency: EffectDependencyAst::DependsOnMembers(vec![member - 1]),
                        carries,
                        provenance: None,
                    }
                })
                .collect();
            effects.push(EffectAst::Coordination(CoordinationAst {
                kind: CoordinationKindAst::Sequence,
                members,
                boundaries,
                provenance: None,
            }));
        }
        Ok(())
    }

    fn migrate_effect(&mut self, effect: &mut EffectAst) -> Result<(), SymbolResolutionError> {
        if let Some(clause) = self.library_clause(effect)? {
            *effect = EffectAst::Clause(clause);
            return Ok(());
        }
        let mut result = Ok(());
        for_each_nested_effect_vec_mut(effect, true, |nested| {
            if result.is_ok() {
                result = self.migrate_effects(nested);
            }
        });
        result
    }

    fn library_clause(
        &mut self,
        effect: &EffectAst,
    ) -> Result<Option<CompilerClauseAst>, SymbolResolutionError> {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return Ok(None);
        };
        let actor = compiler_actor(subject_verb.subject.player);
        let (verb, library) = match &subject_verb.action {
            SubjectVerbActionAst::Scry { count } | SubjectVerbActionAst::Surveil { count } => {
                let exposed = self.bind_result(
                    None,
                    ReferenceRole::Revealed,
                    Cardinality::Any,
                    LibraryResultKindAst::Exposed,
                )?;
                let chosen = self.bind_result(
                    None,
                    ReferenceRole::Chosen,
                    Cardinality::Any,
                    LibraryResultKindAst::Chosen,
                )?;
                let chosen_destination =
                    if matches!(&subject_verb.action, SubjectVerbActionAst::Scry { .. }) {
                        zone_destination(Zone::Library, ClauseZonePlacementAst::Bottom)
                    } else {
                        zone_destination(Zone::Graveyard, ClauseZonePlacementAst::Default)
                    };
                (
                    ClauseVerbAst::Look,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: Some(actor.clone()),
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::Top(CompilerValueAst::Dynamic(count.clone())),
                        exposure: LibraryExposureAst::Inspect,
                        selections: vec![LibrarySelectionAst {
                            qualification: None,
                            minimum: CompilerValueAst::Fixed(0),
                            maximum: Some(CompilerValueAst::Dynamic(count.clone())),
                            mode: LibrarySelectionModeAst::Optional,
                            random: false,
                        }],
                        destination: Some(chosen_destination),
                        results: vec![exposed.clone(), chosen.clone()],
                        remainder: Some(LibraryRemainderAst {
                            collection: exposed.reference,
                            excluding: vec![chosen.reference],
                            destination: zone_destination(
                                Zone::Library,
                                ClauseZonePlacementAst::Top,
                            ),
                            order: LibraryOrderAst::Chosen,
                        }),
                        reveal_results: false,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::Mill { count } => {
                let result = self.bind_result(
                    None,
                    ReferenceRole::Milled,
                    Cardinality::Any,
                    LibraryResultKindAst::Milled,
                )?;
                (
                    ClauseVerbAst::Mill,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::Top(CompilerValueAst::Dynamic(count.clone())),
                        exposure: LibraryExposureAst::Mill,
                        selections: Vec::new(),
                        destination: None,
                        results: vec![result],
                        remainder: None,
                        reveal_results: false,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::RevealTop => (
                ClauseVerbAst::Reveal,
                CompilerLibraryClauseAst {
                    owner: actor.clone(),
                    chooser: None,
                    source_zones: vec![Zone::Library],
                    position: LibraryPositionAst::Top(CompilerValueAst::Fixed(1)),
                    exposure: LibraryExposureAst::Reveal,
                    selections: Vec::new(),
                    destination: None,
                    results: vec![self.bind_result(
                        None,
                        ReferenceRole::Revealed,
                        Cardinality::ExactlyOne,
                        LibraryResultKindAst::Exposed,
                    )?],
                    remainder: None,
                    reveal_results: true,
                    shuffle_after: false,
                    tapped: false,
                    enters_under_source_controller: false,
                    any_order_surface: false,
                },
            ),
            SubjectVerbActionAst::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
                face_down,
                ..
            } => {
                let mut results = Vec::new();
                for tag in tags.iter().chain(accumulated_tags) {
                    results.push(self.bind_result(
                        Some(tag.clone()),
                        ReferenceRole::Exiled,
                        Cardinality::Any,
                        LibraryResultKindAst::Exposed,
                    )?);
                }
                if results.is_empty() {
                    results.push(self.bind_result(
                        None,
                        ReferenceRole::Exiled,
                        Cardinality::Any,
                        LibraryResultKindAst::Exposed,
                    )?);
                }
                (
                    ClauseVerbAst::Exile,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::Top(CompilerValueAst::Dynamic(count.clone())),
                        exposure: if *face_down {
                            LibraryExposureAst::ExileFaceDown
                        } else {
                            LibraryExposureAst::ExileFaceUp
                        },
                        selections: Vec::new(),
                        destination: None,
                        results,
                        remainder: None,
                        reveal_results: !*face_down,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::LookAtTopCards { count, tag, reveal } => (
                if *reveal {
                    ClauseVerbAst::Reveal
                } else {
                    ClauseVerbAst::Look
                },
                CompilerLibraryClauseAst {
                    owner: actor.clone(),
                    chooser: None,
                    source_zones: vec![Zone::Library],
                    position: LibraryPositionAst::Top(CompilerValueAst::Dynamic(count.clone())),
                    exposure: if *reveal {
                        LibraryExposureAst::Reveal
                    } else {
                        LibraryExposureAst::Inspect
                    },
                    selections: Vec::new(),
                    destination: None,
                    results: vec![self.bind_result(
                        Some(tag.clone()),
                        ReferenceRole::Revealed,
                        Cardinality::Any,
                        LibraryResultKindAst::Exposed,
                    )?],
                    remainder: None,
                    reveal_results: *reveal,
                    shuffle_after: false,
                    tapped: false,
                    enters_under_source_controller: false,
                    any_order_surface: false,
                },
            ),
            SubjectVerbActionAst::RevealTagged { tag } => {
                let Some(reference) = self.context.object_reference(tag) else {
                    return Ok(None);
                };
                (
                    ClauseVerbAst::Reveal,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::BoundCollection(reference),
                        exposure: LibraryExposureAst::Reveal,
                        selections: Vec::new(),
                        destination: None,
                        results: Vec::new(),
                        remainder: None,
                        reveal_results: true,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::ReorderTopOfLibrary { tag } => {
                let Some(reference) = self.context.object_reference(tag) else {
                    return Ok(None);
                };
                (
                    ClauseVerbAst::Put,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::BoundCollection(reference),
                        exposure: LibraryExposureAst::None,
                        selections: Vec::new(),
                        destination: None,
                        results: Vec::new(),
                        remainder: None,
                        reveal_results: false,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: true,
                    },
                )
            }
            SubjectVerbActionAst::ShuffleLibrary => (
                ClauseVerbAst::Shuffle,
                CompilerLibraryClauseAst {
                    owner: actor.clone(),
                    chooser: None,
                    source_zones: vec![Zone::Library],
                    position: LibraryPositionAst::WholeZone,
                    exposure: LibraryExposureAst::None,
                    selections: Vec::new(),
                    destination: None,
                    results: Vec::new(),
                    remainder: None,
                    reveal_results: false,
                    shuffle_after: false,
                    tapped: false,
                    enters_under_source_controller: false,
                    any_order_surface: false,
                },
            ),
            SubjectVerbActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed,
                all_tag,
                match_tag,
            } => {
                let all = self.bind_result(
                    Some(all_tag.clone()),
                    ReferenceRole::Revealed,
                    Cardinality::Any,
                    LibraryResultKindAst::Exposed,
                )?;
                let matched = self.bind_result(
                    Some(match_tag.clone()),
                    ReferenceRole::Revealed,
                    Cardinality::Any,
                    LibraryResultKindAst::Matched,
                )?;
                let match_count = match stop_rule {
                    crate::model::LibraryConsultStopRuleAst::FirstMatch => {
                        CompilerValueAst::Fixed(1)
                    }
                    crate::model::LibraryConsultStopRuleAst::MatchCount(value) => {
                        CompilerValueAst::Dynamic(value.clone())
                    }
                };
                (
                    match mode {
                        LibraryConsultModeAst::Reveal => ClauseVerbAst::Reveal,
                        LibraryConsultModeAst::Exile => ClauseVerbAst::Exile,
                    },
                    CompilerLibraryClauseAst {
                        owner: compiler_actor(*player),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::UntilMatch {
                            qualification: CompilerFilterAst::Card(filter.clone()),
                            match_count,
                            maximum_exposed: max_exposed.clone().map(CompilerValueAst::Dynamic),
                        },
                        exposure: match mode {
                            LibraryConsultModeAst::Reveal => LibraryExposureAst::Reveal,
                            LibraryConsultModeAst::Exile => LibraryExposureAst::ExileFaceUp,
                        },
                        selections: Vec::new(),
                        destination: None,
                        results: vec![all, matched],
                        remainder: None,
                        reveal_results: matches!(mode, LibraryConsultModeAst::Reveal),
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::SearchLibrary {
                filter,
                search_zones,
                destination,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle,
                count,
                count_value,
                library_position_from_top,
                search_top_in_any_order_surface,
                tapped,
                enters_with_counters,
                enters_under_your_control,
                ..
            } if enters_with_counters.is_empty() => {
                let maximum = count_value
                    .clone()
                    .map(CompilerValueAst::Dynamic)
                    .or_else(|| count.max.map(fixed_usize));
                let minimum = if count.dynamic_x {
                    CompilerValueAst::X
                } else {
                    fixed_usize(count.min)
                };
                let position = if let Some(position) = library_position_from_top.clone() {
                    LibraryPositionAst::NthFromTop(CompilerValueAst::Dynamic(position))
                } else if count.random {
                    LibraryPositionAst::Random(maximum.clone().unwrap_or_else(|| minimum.clone()))
                } else {
                    LibraryPositionAst::WholeZone
                };
                (
                    ClauseVerbAst::Search,
                    CompilerLibraryClauseAst {
                        owner: compiler_actor(*player),
                        chooser: Some(compiler_actor(*chooser)),
                        source_zones: search_zones.clone(),
                        position,
                        exposure: LibraryExposureAst::Search,
                        selections: vec![LibrarySelectionAst {
                            qualification: Some(CompilerFilterAst::Card(filter.clone())),
                            minimum,
                            maximum,
                            mode: match search_mode {
                                crate::effect::SearchSelectionMode::Exact => {
                                    LibrarySelectionModeAst::Exact
                                }
                                crate::effect::SearchSelectionMode::Optional => {
                                    LibrarySelectionModeAst::Optional
                                }
                                crate::effect::SearchSelectionMode::AllMatching => {
                                    LibrarySelectionModeAst::AllMatching
                                }
                            },
                            random: count.random,
                        }],
                        destination: Some(zone_destination(
                            *destination,
                            ClauseZonePlacementAst::Default,
                        )),
                        results: vec![self.bind_result(
                            None,
                            ReferenceRole::Searched,
                            choice_cardinality(*count),
                            LibraryResultKindAst::Found,
                        )?],
                        remainder: None,
                        reveal_results: *reveal,
                        shuffle_after: *shuffle,
                        tapped: *tapped,
                        enters_under_source_controller: *enters_under_your_control,
                        any_order_surface: *search_top_in_any_order_surface,
                    },
                )
            }
            SubjectVerbActionAst::SearchLibrarySlotsToHand {
                slots,
                destination,
                reveal,
                progress_tag,
            } => {
                let minimum = slots.iter().filter(|slot| !slot.optional).count();
                let cardinality = Cardinality::Range {
                    min: u32::try_from(minimum).unwrap_or(u32::MAX),
                    max: Some(u32::try_from(slots.len()).unwrap_or(u32::MAX)),
                };
                (
                    ClauseVerbAst::Search,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: Some(actor.clone()),
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::WholeZone,
                        exposure: LibraryExposureAst::Search,
                        selections: slots
                            .iter()
                            .map(|slot| LibrarySelectionAst {
                                qualification: Some(CompilerFilterAst::Card(slot.filter.clone())),
                                minimum: CompilerValueAst::Fixed(if slot.optional { 0 } else { 1 }),
                                maximum: Some(CompilerValueAst::Fixed(1)),
                                mode: if slot.optional {
                                    LibrarySelectionModeAst::Optional
                                } else {
                                    LibrarySelectionModeAst::Exact
                                },
                                random: false,
                            })
                            .collect(),
                        destination: Some(zone_destination(
                            *destination,
                            ClauseZonePlacementAst::Default,
                        )),
                        results: vec![self.bind_result(
                            Some(progress_tag.clone()),
                            ReferenceRole::Searched,
                            cardinality,
                            LibraryResultKindAst::Found,
                        )?],
                        remainder: None,
                        reveal_results: *reveal,
                        shuffle_after: true,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
                ..
            } => {
                let Some(collection) = self.context.object_reference(tag) else {
                    return Ok(None);
                };
                let excluding = keep_tagged
                    .as_ref()
                    .and_then(|tag| self.context.object_reference(tag))
                    .into_iter()
                    .collect();
                (
                    ClauseVerbAst::Put,
                    CompilerLibraryClauseAst {
                        owner: compiler_actor(*player),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::BoundCollection(collection),
                        exposure: LibraryExposureAst::None,
                        selections: Vec::new(),
                        destination: None,
                        results: Vec::new(),
                        remainder: Some(LibraryRemainderAst {
                            collection,
                            excluding,
                            destination: zone_destination(
                                Zone::Library,
                                ClauseZonePlacementAst::Bottom,
                            ),
                            order: match order {
                                LibraryBottomOrderAst::Random => LibraryOrderAst::Random,
                                LibraryBottomOrderAst::ChooserChooses => LibraryOrderAst::Chosen,
                            },
                        }),
                        reveal_results: false,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag,
                keep_tagged,
                zone,
                ..
            } => {
                let Some(collection) = self.context.object_reference(tag) else {
                    return Ok(None);
                };
                let Some(excluding) = self.context.object_reference(keep_tagged) else {
                    return Ok(None);
                };
                (
                    ClauseVerbAst::Move,
                    CompilerLibraryClauseAst {
                        owner: actor.clone(),
                        chooser: None,
                        source_zones: vec![Zone::Library],
                        position: LibraryPositionAst::BoundCollection(collection),
                        exposure: LibraryExposureAst::None,
                        selections: Vec::new(),
                        destination: None,
                        results: Vec::new(),
                        remainder: Some(LibraryRemainderAst {
                            collection,
                            excluding: vec![excluding],
                            destination: zone_destination(*zone, ClauseZonePlacementAst::Default),
                            order: LibraryOrderAst::Preserve,
                        }),
                        reveal_results: false,
                        shuffle_after: false,
                        tapped: false,
                        enters_under_source_controller: false,
                        any_order_surface: false,
                    },
                )
            }
            _ => return Ok(None),
        };

        Ok(Some(CompilerClauseAst {
            actor: actor.clone(),
            subject: ClauseSubjectAst::Actor(actor),
            action: ClauseActionAst {
                verb,
                polarity: ClausePolarityAst::Positive,
            },
            object: None,
            quantity: None,
            destination: library.destination.clone().or_else(|| {
                library
                    .remainder
                    .as_ref()
                    .map(|remainder| remainder.destination.clone())
            }),
            duration: None,
            condition: None,
            bindings: Vec::new(),
            complements: Vec::new(),
            library: Some(library),
            object_action: None,
            provenance: None,
        }))
    }

    fn bind_result(
        &mut self,
        tag: Option<TagKey>,
        role: ReferenceRole,
        cardinality: Cardinality,
        kind: LibraryResultKindAst,
    ) -> Result<LibraryResultBindingAst, SymbolResolutionError> {
        let reference = self.context.bind_object(tag, role, cardinality)?;
        Ok(LibraryResultBindingAst { kind, reference })
    }
}

fn library_coordination_member(effect: EffectAst) -> CoordinationMemberAst {
    let mut member = CoordinationMemberAst::new(vec![effect]);
    let [EffectAst::Clause(clause)] = member.effects.as_slice() else {
        return member;
    };
    let Some(library) = &clause.library else {
        return member;
    };
    member.exports = library
        .results
        .iter()
        .map(|result| result.reference)
        .collect();
    if let LibraryPositionAst::BoundCollection(reference) = &library.position {
        push_unique_reference(&mut member.imports, *reference);
    }
    if let Some(remainder) = &library.remainder {
        push_unique_reference(&mut member.imports, remainder.collection);
        for reference in &remainder.excluding {
            push_unique_reference(&mut member.imports, *reference);
        }
    }
    let exports = member.exports.clone();
    member
        .imports
        .retain(|reference| !exports.contains(reference));
    member
}

fn push_unique_reference(references: &mut Vec<SymbolReference>, reference: SymbolReference) {
    if !references.contains(&reference) {
        references.push(reference);
    }
}

pub(crate) fn migrate_library_clauses(
    items: &mut [ParsedCardItem],
    context: &mut SemanticMigrationContext<'_>,
) -> Result<(), SymbolResolutionError> {
    let mut migration = LibraryMigration::new(context);
    for item in items {
        migrate_item(item, &mut migration)?;
    }
    Ok(())
}

fn migrate_item(
    item: &mut ParsedCardItem,
    migration: &mut LibraryMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    match item {
        ParsedCardItem::Line(line) => {
            for chunk in &mut line.chunks {
                migrate_line_chunk(chunk, migration)?;
            }
            if let Some(ability) = &mut line.semantic_facts.triggered_ability.compiler_ability {
                migration.migrate_effects(&mut ability.effects)?;
                for program in &mut ability.program.programs {
                    migration.migrate_effects(&mut program.effects)?;
                }
            }
        }
        ParsedCardItem::Modal(modal) => migrate_modal(modal, migration)?,
        ParsedCardItem::LevelAbility(level) => {
            for item in &mut level.items {
                if let ParsedLevelAbilityItemAst::ActivatedAbility(activated) = item {
                    migrate_line_chunk(&mut activated.chunk, migration)?;
                }
            }
        }
    }
    Ok(())
}

fn migrate_line_chunk(
    chunk: &mut LineAst,
    migration: &mut LibraryMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    match chunk {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                migrate_line_chunk(chunk, migration)?;
            }
        }
        LineAst::Ability(ability) => {
            if let Some(effects) = &mut ability.effects_ast {
                migration.migrate_effects(effects)?;
            }
        }
        LineAst::Triggered { effects, .. }
        | LineAst::Statement { effects }
        | LineAst::AdditionalCost { effects }
        | LineAst::GiftKeyword { effects, .. }
        | LineAst::OptionalCostWithCastTrigger { effects, .. } => {
            migration.migrate_effects(effects)?;
        }
        LineAst::AdditionalCostChoice { options } => {
            for option in options {
                migration.migrate_effects(&mut option.effects)?;
            }
        }
        LineAst::Abilities(_)
        | LineAst::StaticAbility(_)
        | LineAst::StaticAbilities(_)
        | LineAst::OptionalCost(_)
        | LineAst::AlternativeCastingMethod(_) => {}
    }
    Ok(())
}

fn migrate_modal(
    modal: &mut ParsedModalAst,
    migration: &mut LibraryMigration<'_, '_>,
) -> Result<(), SymbolResolutionError> {
    migration.migrate_effects(&mut modal.header.prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_prefix_effects_ast)?;
    migration.migrate_effects(&mut modal.header.common_suffix_effects_ast)?;
    for mode in &mut modal.modes {
        migration.migrate_effects(&mut mode.effects_ast)?;
    }
    Ok(())
}

pub(super) fn compiler_actor(player: PlayerAst) -> ClauseActorAst {
    match player {
        PlayerAst::You | PlayerAst::Implicit => ClauseActorAst::SourceController,
        PlayerAst::Active => ClauseActorAst::ActivePlayer,
        PlayerAst::Any => ClauseActorAst::Player(CompilerPlayerAst::Any),
        PlayerAst::Chosen => ClauseActorAst::Player(CompilerPlayerAst::Chosen),
        PlayerAst::Defending => ClauseActorAst::Player(CompilerPlayerAst::Defending),
        PlayerAst::Attacking => ClauseActorAst::Player(CompilerPlayerAst::Attacking),
        PlayerAst::Target => ClauseActorAst::Player(CompilerPlayerAst::Target),
        PlayerAst::TargetOpponent => ClauseActorAst::Player(CompilerPlayerAst::TargetOpponent),
        PlayerAst::Opponent => ClauseActorAst::Player(CompilerPlayerAst::Opponent),
        PlayerAst::Enchanted => ClauseActorAst::Player(CompilerPlayerAst::Enchanted),
        PlayerAst::NotYou => ClauseActorAst::Player(CompilerPlayerAst::OtherThanSourceController),
        PlayerAst::TriggeringSourceController => {
            ClauseActorAst::Player(CompilerPlayerAst::TriggeringSourceController)
        }
        PlayerAst::ItsController => {
            ClauseActorAst::Player(CompilerPlayerAst::ReferencedObjectController)
        }
        PlayerAst::ItsOwner => ClauseActorAst::Player(CompilerPlayerAst::ReferencedObjectOwner),
        PlayerAst::MostCardsInHand
        | PlayerAst::MostLifeTied
        | PlayerAst::LowestLifeTied
        | PlayerAst::PlayerToYourLeft
        | PlayerAst::PlayerToYourRight
        | PlayerAst::That
        | PlayerAst::ThatPlayerOrTargetController => {
            ClauseActorAst::Player(CompilerPlayerAst::Contextual)
        }
    }
}

fn zone_destination(zone: Zone, placement: ClauseZonePlacementAst) -> ClauseDestinationAst {
    ClauseDestinationAst {
        relation: ClauseDestinationRelationAst::To,
        zone,
        placement,
        controller: None,
    }
}

fn fixed_usize(value: usize) -> CompilerValueAst {
    CompilerValueAst::Fixed(i32::try_from(value).unwrap_or(i32::MAX))
}

fn choice_cardinality(count: crate::effect::ChoiceCount) -> Cardinality {
    match (count.min, count.max) {
        (1, Some(1)) if !count.dynamic_x => Cardinality::ExactlyOne,
        (0, Some(1)) if !count.dynamic_x => Cardinality::ZeroOrOne,
        (min, max) => Cardinality::Range {
            min: u32::try_from(min).unwrap_or(u32::MAX),
            max: max.map(|max| u32::try_from(max).unwrap_or(u32::MAX)),
        },
    }
}
