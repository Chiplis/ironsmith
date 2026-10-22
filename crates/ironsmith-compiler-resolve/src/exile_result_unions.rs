//! Plural references that span more than one exile instruction.
//!
//! "When this creature dies, exile it. If you do, exile the top card of your
//! library. Until the end of your next turn, you may play one of those cards."
//! (Flameskull): "those cards" is plural, but the instruction right before it
//! exiled exactly one card. The plural set is every card the preceding exile
//! instructions moved: the dying creature and the library card. Both exiles
//! record into one shared tag, and the permission reads that tag.

use crate::cards::builders::{
    ConditionalEffectAst, EffectAst, GrantActionAst, IdGenContext, LibraryActionAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, ZoneMoveActionAst,
};
use crate::tag::TagRef;
use crate::Value;

fn is_it(tag: &TagRef) -> bool {
    tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
}

/// The single trailing instruction of a sentence wrapper or result gate.
fn sole_instruction_mut(effect: &mut EffectAst) -> &mut EffectAst {
    let descend = matches!(
        effect,
        EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects }
            if effects.len() == 1
    ) || matches!(
        effect,
        EffectAst::Conditionals(
            ConditionalEffectAst::IfResult { effects, .. }
                | ConditionalEffectAst::ResolvedIfResult { effects, .. }
        ) if effects.len() == 1
    );
    if !descend {
        return effect;
    }
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::Conditionals(
            ConditionalEffectAst::IfResult { effects, .. }
            | ConditionalEffectAst::ResolvedIfResult { effects, .. },
        ) => sole_instruction_mut(&mut effects[0]),
        _ => unreachable!("guarded single-instruction wrapper"),
    }
}

/// A "one of those cards" play permission still naming the last object.
fn plural_one_of_permission_tag(effect: &mut EffectAst) -> Option<&mut TagRef> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = sole_instruction_mut(effect)
    else {
        return None;
    };
    match action {
        SubjectVerbActionAst::Grants(
            GrantActionAst::GrantPlayTaggedUntilEndOfTurn { tag, max_plays: Some(1), .. }
            | GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, max_plays: Some(1), .. }
            | GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                max_plays: Some(1),
                ..
            },
        ) => Some(tag),
        _ => None,
    }
}

/// An exile of exactly the top card of a library.
fn single_top_card_exile(
    effect: &mut EffectAst,
) -> Option<(&mut Vec<TagRef>, &mut Vec<TagRef>)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = sole_instruction_mut(effect)
    else {
        return None;
    };
    match action {
        SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
            count: Value::Fixed(1),
            tags,
            accumulated_tags,
            ..
        }) if !tags.is_empty() => Some((tags, accumulated_tags)),
        _ => None,
    }
}

/// An exile of one referenced object ("exile it").
fn is_single_object_exile(effect: &mut EffectAst) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                target,
                target_plural_surface: false,
                ..
            }),
        ..
    }) = sole_instruction_mut(effect)
    else {
        return false;
    };
    match target {
        crate::cards::builders::TargetAst::Tagged(..)
        | crate::cards::builders::TargetAst::Source(..) => true,
        crate::cards::builders::TargetAst::Object(filter, None, _) => {
            filter.tagged_constraints.len() == 1
                && filter.tagged_constraints[0].tag.as_str()
                    == crate::tag::CompilerReferenceTag::It.as_str()
                && !filter.has_plural_pronoun_reference_surface()
        }
        _ => false,
    }
}

pub fn union_singular_exiles_for_plural_permission(
    effects: &mut [EffectAst],
    id_gen: &mut IdGenContext,
) {
    for index in 2..effects.len() {
        let Some(top_tags) = single_top_card_exile(&mut effects[index - 1])
            .map(|(tags, _)| tags.clone())
        else {
            continue;
        };
        // The permission names the latest exile result: either the bare
        // anaphor or that instruction's own result tag.
        if !plural_one_of_permission_tag(&mut effects[index])
            .is_some_and(|tag| {
                is_it(tag) || top_tags.iter().any(|top| top.as_str() == tag.as_str())
            })
            || !is_single_object_exile(&mut effects[index - 2])
        {
            continue;
        }
        let union: TagRef = ironsmith_compiler_semantic::tag::declared_key(format!(
            "__sentence_helper_exiled_l0_s0_e{}",
            id_gen.next_tag_id
        ))
        .into();
        id_gen.next_tag_id += 1;
        let first = sole_instruction_mut(&mut effects[index - 2]);
        let exile = std::mem::replace(first, EffectAst::Sequence { effects: Vec::new() });
        *first = EffectAst::TagAffected {
            effect: Box::new(exile),
            tag: union.clone(),
        };
        if let Some((_, accumulated)) = single_top_card_exile(&mut effects[index - 1])
            && !accumulated.iter().any(|tag| tag.as_str() == union.as_str())
        {
            accumulated.push(union.clone());
        }
        if let Some(tag) = plural_one_of_permission_tag(&mut effects[index]) {
            *tag = union;
        }
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                    surface,
                    ..
                }),
            ..
        }) = sole_instruction_mut(&mut effects[index])
        {
            let surface = surface.get_or_insert_with(Default::default);
            surface.object = Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThoseCards);
        }
    }
}

/// Alias for the triggered ability's own event antecedent (the object(s) the
/// trigger event names), recorded once before the body is annotated.
pub fn trigger_event_antecedent_alias() -> crate::TagKey {
    crate::TagKey::from("__trigger_event_antecedent__")
}

/// "Whenever one or more Dragons you control deal combat damage to a player
/// or battle, look at the top three cards of your library. Put one of them
/// into your hand ... You may return one of those Dragons to its owner's
/// hand." (Zurgo and Ojutai): a plural demonstrative "those <permanents>"
/// never names the single card just selected from a looked-at library
/// partition; it names the trigger's event objects.
pub fn rebind_plural_demonstrative_past_library_selection(
    effect: &mut EffectAst,
    env: &crate::model::reference_state::ReferenceEnv,
) {
    let Some(current) = env.known_last_object_tag() else {
        return;
    };
    let current = current.as_str();
    let library_selection = current.starts_with("__sentence_helper_partition_selected")
        || current.starts_with("__sentence_helper_looked");
    if !library_selection {
        return;
    }
    let alias = trigger_event_antecedent_alias();
    let Some(event_tag) = env
        .snapshot_tag_aliases
        .iter()
        .find(|(existing, _)| existing == &alias)
        .map(|(_, tag)| tag.clone())
    else {
        return;
    };
    fn visit(effect: &mut EffectAst, event_tag: &crate::TagKey) {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                    target,
                    set_quantifier_surface: Some(ironsmith_core::SetQuantifierSurface::Those),
                    ..
                }),
            ..
        }) = effect
        {
            let filter = match target {
                crate::cards::builders::TargetAst::WithCount(inner, _) => match inner.as_mut() {
                    crate::cards::builders::TargetAst::Object(filter, None, _) => Some(filter),
                    _ => None,
                },
                crate::cards::builders::TargetAst::Object(filter, None, _) => Some(filter),
                _ => None,
            };
            if let Some(filter) = filter
                && (!filter.subtypes.is_empty() || !filter.card_types.is_empty())
            {
                for constraint in &mut filter.tagged_constraints {
                    if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                        || constraint.tag.as_str().starts_with("__sentence_helper_")
                    {
                        constraint.tag = event_tag.clone();
                    }
                }
            }
            return;
        }
        if matches!(effect, EffectAst::Permissions(_) | EffectAst::Conditionals(_)) {
            crate::effect_ast_traversal::for_each_nested_effects_mut(effect, false, |nested| {
                for child in nested.iter_mut() {
                    visit(child, event_tag);
                }
            });
        }
    }
    visit(effect, &event_tag);
}

/// The object set a die roll's result modifier counts ("Roll a d20 and add
/// the number of creature cards in your graveyard that were put there from
/// the battlefield this turn").
fn counted_roll_modifier_set(effect: &EffectAst) -> Option<crate::ObjectFilter> {
    match effect {
        EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects } => {
            effects.iter().find_map(counted_roll_modifier_set)
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Random(crate::cards::builders::RandomActionAst::RollDie {
                    result_modifier: Some(modifier),
                    ..
                }),
            ..
        }) => match modifier.value().unhinted() {
            Value::Count(filter) => Some(filter.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// An unbound plural demonstrative ("those cards") with no other antecedent.
fn unbound_plural_demonstrative_filter(
    target: &mut crate::cards::builders::TargetAst,
) -> Option<&mut crate::ObjectFilter> {
    match target {
        crate::cards::builders::TargetAst::Object(filter, None, _)
            if (filter.has_plural_pronoun_reference_surface()
                || (filter.has_plural_object_noun_surface()
                    && matches!(
                        &filter.source_surface,
                        Some(crate::target::SourceReferenceSurface::ThisPermanentType(text))
                            if text.starts_with("those ")
                    )))
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                }) =>
        {
            Some(filter)
        }
        _ => None,
    }
}

fn bind_plural_demonstrative_in_target(effect: &mut EffectAst, tag: &TagRef) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return false;
    };
    let target = match action {
        SubjectVerbActionAst::ZoneMoves(
            ZoneMoveActionAst::ReturnToBattlefield { target, .. }
            | ZoneMoveActionAst::ReturnToHand { target, .. }
            | ZoneMoveActionAst::MoveToZone { target, .. }
            | ZoneMoveActionAst::Exile { target, .. },
        ) => target,
        _ => return false,
    };
    let Some(filter) = unbound_plural_demonstrative_filter(target) else {
        return false;
    };
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
            constraint.tag = tag.key.clone();
        }
    }
    filter.set_plural_pronoun_reference_surface(true);
    true
}

/// Bind the row's plural demonstratives; when any binds, capture the counted
/// set at the start of that row's instruction list.
fn bind_plural_demonstratives_in_row(
    effect: &mut EffectAst,
    tag: &TagRef,
    filter: &crate::ObjectFilter,
) -> bool {
    let mut bound = false;
    crate::effect_ast_traversal::for_each_nested_effect_vec_mut(effect, false, |nested| {
        let mut here = false;
        for child in nested.iter_mut() {
            here |= bind_plural_demonstrative_in_target(child, tag);
        }
        if here {
            let zones = filter.zone.into_iter().collect::<Vec<_>>();
            nested.insert(
                0,
                EffectAst::subject_verb_tag_matching_objects(filter.clone(), zones, tag.clone()),
            );
            bound = true;
        } else {
            for child in nested.iter_mut() {
                bound |= bind_plural_demonstratives_in_row(child, tag, filter);
            }
        }
    });
    bound
}

/// "Roll a d20 and add the number of creature cards in your graveyard that
/// were put there from the battlefield this turn. ... 15+ | Return those
/// cards from your graveyard to the battlefield." (Revivify): with no other
/// object antecedent, "those cards" is the set the roll counted. Capture it
/// as the roll resolves and let the row read that set.
pub fn bind_plural_demonstratives_to_counted_roll_set(
    effects: &mut Vec<EffectAst>,
    id_gen: &mut IdGenContext,
) {
    let Some(index) = effects
        .iter()
        .position(|effect| counted_roll_modifier_set(effect).is_some())
    else {
        return;
    };
    let filter = counted_roll_modifier_set(&effects[index]).expect("guarded counted roll");
    let tag: TagRef = ironsmith_compiler_semantic::tag::declared_key(format!(
        "counted_roll_set_{}",
        id_gen.next_tag_id
    ));
    fn is_die_row(effect: &EffectAst) -> bool {
        match effect {
            EffectAst::SourceSentence { effects, .. }
            | EffectAst::Sequence { effects }
            | EffectAst::ResultBranchLabel { effects, .. } => {
                effects.len() == 1 && is_die_row(&effects[0])
            }
            EffectAst::Conditionals(
                ConditionalEffectAst::IfResult { predicate, .. }
                | ConditionalEffectAst::ResolvedIfResult { predicate, .. },
            ) => matches!(predicate, crate::IfResultPredicate::DieValue(_)),
            _ => false,
        }
    }
    let mut bound = false;
    for later in effects.iter_mut().skip(index + 1) {
        if !is_die_row(later) {
            break;
        }
        bound |= bind_plural_demonstratives_in_row(later, &tag, &filter);
    }
    if bound {
        id_gen.next_tag_id += 1;
    }
}
