use super::*;
use crate::cards::builders::{CharacteristicActionAst, GrantedAbilityAst};
use ironsmith_core::effect::BattlefieldEntryTypeChanges;

fn is_arrival_reference(target: &TargetAst) -> bool {
    matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
}

/// CR 611.2e: a descriptive type rider belongs to the arrival itself. Only
/// additive type, fixed color, and ability descriptions are owned here;
/// ordinary "becomes" and "gains" instructions retain their separate owners.
pub(crate) fn parse_entry_type_rider(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [prefix @ .., rider] = sentences.as_slice() else {
        return Ok(None);
    };
    if prefix.is_empty() {
        return Ok(None);
    }
    let words = crate::lexer::token_word_refs(rider);
    // Only a descriptive clause belongs to entry. "It gains" and "it
    // becomes" remain ordinary instructions after the move.
    if !matches!(words.as_slice(), ["it", "has" | "is", ..] | ["it's" | "its" | "it’s", ..] | ["that", "creature" | "permanent", "has" | "is", ..]) {
        return Ok(None);
    }
    if words.iter().any(|word| matches!(*word, "becomes" | "become" | "gains" | "gain")) { return Ok(None); }
    // A rider this reading can't parse belongs to another owner (token
    // definitions, "its ... cost" prices, "still a" type riders).
    let Ok(parsed) = effect_sentences::parse_effect_sentence_lexed(rider) else { return Ok(None); };
    let mut changes = BattlefieldEntryTypeChanges::default();
    let mut grants = Vec::new();
    if !collect_descriptions(&parsed, &mut changes, &mut grants) { return Ok(None); }
    let mut prior_tokens = Vec::new();
    for sentence in prefix {
        prior_tokens.extend_from_slice(sentence);
        prior_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    }
    let Ok(mut effects) = effect_sentences::parse_effect_sentences_lexed(&prior_tokens) else { return Ok(None); };
    if !attach_to_arrival(&mut effects, &changes, &grants) {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn attach_to_arrival(effects: &mut [EffectAst], changes: &BattlefieldEntryTypeChanges, grants: &[GrantedAbilityAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        // Inline counters already belong to this arrival and are fused by the
        // normal lowering pass. They do not replace its antecedent.
        if matches!(effect, EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Counters(crate::cards::builders::CounterActionAst::PutCounters { count, target, .. }), ..
        }) if count.has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter) && is_arrival_reference(target))
        {
            continue;
        }
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    target,
                    result_tag: None,
                    target_reference_surface,
                    from_graveyard_or_exile: false,
                    tapped,
                    transformed: false,
                    converted: false,
                    controller,
                    count_value: None,
                    as_aura: None,
                    top_only: false,
                    as_non_aura: false,
                    tapped_if: None,
                    noted_counters: None,
                }),
            subject,
        }) = effect
        {
            let mut movement = EffectAst::subject_verb_move_to_zone(
                target.clone(),
                Zone::Battlefield,
                false,
                *controller,
                *tapped,
                None,
            );
            if let EffectAst::SubjectVerb(movement) = &mut movement {
                movement.subject = subject.clone();
                if let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    verb_surface,
                    target_reference_surface: reference,
                    ..
                }) = &mut movement.action
                {
                    *verb_surface = ironsmith_core::MoveToZoneVerbSurface::Return;
                    *reference = target_reference_surface.clone();
                }
            }
            *effect = movement;
        }
        return match effect {
            EffectAst::Coordination(coordination) => coordination.members.last_mut().is_some_and(|member| attach_to_arrival(&mut member.effects, changes, grants)),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                        zone: Zone::Battlefield,
                        entry_type_changes,
                        entry_grants,
                        ..
                    }),
                ..
            }) => {
                entry_grants.extend_from_slice(grants);
                let entry = entry_type_changes.get_or_insert_with(Default::default);
                entry
                    .color_changes
                    .extend(changes.color_changes.iter().copied());
                entry
                    .added_card_types
                    .extend(changes.added_card_types.iter().copied());
                entry
                    .added_subtypes
                    .extend(changes.added_subtypes.iter().copied());
                true
            }
            EffectAst::Permissions(
                PermissionEffectAst::May { effects }
                | PermissionEffectAst::MayByPlayer { effects, .. },
            )
            | EffectAst::Sequence { effects }
            | EffectAst::Coordinated { effects, .. }
            | EffectAst::CommaThen { effects } => attach_to_arrival(effects, changes, grants),
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                if_true,
                if_false,
                ..
            }) if if_false.is_empty() => attach_to_arrival(if_true, changes, grants),
            _ => false,
        };
    }
    false
}

fn collect_descriptions(effects: &[EffectAst], changes: &mut BattlefieldEntryTypeChanges, grants: &mut Vec<GrantedAbilityAst>) -> bool {
    for effect in effects {
        match effect {
            EffectAst::Coordination(coordination) => {
                for member in &coordination.members {
                    if !collect_descriptions(&member.effects, changes, grants) { return false; }
                }
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes {
                target, subtypes, duration: ironsmith_core::Until::Forever,
            }), .. }) if is_arrival_reference(target) => changes.added_subtypes.extend_from_slice(subtypes),
            EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } | EffectAst::CommaThen { effects } => {
                if !collect_descriptions(effects, changes, grants) { return false; }
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target, abilities, duration: ironsmith_core::Until::Forever, condition: None, only_if_absent: false, ..
            }), .. }) if is_arrival_reference(target) => grants.extend_from_slice(abilities),
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                target, name_override: None, add_supertypes, remove_all_abilities: false, remove_other_abilities: false,
                base_power_toughness: None, card_types, subtypes, subtype_families, colors, abilities, granted_abilities,
                preserve_other_types: true, preserve_other_colors, duration: ironsmith_core::Until::Forever, ..
            }), .. }) if is_arrival_reference(target) && add_supertypes.is_empty() && subtype_families.is_empty() => {
                changes.added_card_types.extend_from_slice(card_types);
                changes.added_subtypes.extend_from_slice(subtypes);
                if let Some(colors) = colors {
                    changes.color_changes.push(if *preserve_other_colors { ironsmith_core::effect::BattlefieldEntryColorChange::Add(*colors) } else { ironsmith_core::effect::BattlefieldEntryColorChange::Set(*colors) });
                }
                grants.extend_from_slice(granted_abilities);
                grants.extend(abilities.iter().cloned().map(|ability| GrantedAbilityAst::StaticAbility(Box::new(crate::cards::builders::StaticAbilityAst::Static(ability)))));
            }
            _ => return false,
        }
    }
    true
}
