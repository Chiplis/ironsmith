//! Exiled-collection procedures composed statement by statement.
//!
//! "Exile the top five cards of your library. You may cast spells with mana
//! value 3 or less from among them without paying their mana costs. Then put
//! the rest into your graveyard." is an exile statement that binds the exiled
//! collection, followed by statements over it: casting from among the
//! collection, putting some of it onto the battlefield, partitioning what was
//! not cast, or a permission to play the exiled card this turn with an event
//! that follows it. The exile sentence is the ordinary sentence grammar's,
//! whose exile-top action already tags the collection; this module carries that
//! tag to the sentences that follow, as [`super::looked_procedure`] carries a
//! viewed group.

use super::dispatch_entry::SentenceInput;
use super::sequence_rules::generic_subject_verb_sequences::exile_permission_followups::rebind_permission_tag;
use super::sequence_rules::generic_subject_verb_sequences::exiled_collections::{
    find_exiled_top_collection_tag, parse_collection_cast_filter,
    parse_exile_top_then_put_from_among_tokens, parse_remaining_exiled_partition,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, TriggerSpec,
};
use crate::grammar::effects::{
    ExilePermissionFollowupKind, clause_dispatch_shapes, parse_exile_permission_followup_shape,
};
use crate::lexer::OwnedLexToken;
use crate::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::tag::TagKey;
use crate::target::{PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

/// What has been said about the exiled collection so far.
enum Statements {
    /// Only the exile.
    None,
    /// Some of the collection was cast; the rest may still be partitioned.
    Cast { chosen: TagKey, partitioned: bool },
    /// Some of the collection was put onto the battlefield.
    Battlefield,
    /// The exiled card may be played this turn; the event that follows it is
    /// awaited, and the permission is spelled with it.
    Permission(EffectAst),
    /// The permission and its event were spelled.
    PermissionWithEvent,
}

/// The exiled collection an exile statement bound, and the statements made
/// over it so far.
pub(super) struct ExiledTopGroup {
    effects: Vec<EffectAst>,
    tag: TagKey,
    /// The exile sentence's tokens: the battlefield statement reads the exile
    /// and itself as one clause.
    first_tokens: Vec<OwnedLexToken>,
    statements: Statements,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

/// "You may cast <count> <filter> spells [with mana value N or less] from
/// among them without paying their mana costs."
fn cast_collection(
    sentence: &SentenceInput,
    exiled: &TagKey,
) -> Result<Option<(Vec<EffectAst>, TagKey)>, CardTextError> {
    let Some(shape) = clause_dispatch_shapes::parse_cast_tagged_collection_shape(sentence.lowered())
    else {
        return Ok(None);
    };
    let Some(mut filter) = parse_collection_cast_filter(&shape)? else {
        return Ok(None);
    };
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "cast_from_exiled_collection");
    Ok(Some((
        vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: shape.count,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
                zone: Zone::Exile,
            },
            EffectAst::ForEachTagged {
                tag: chosen_tag.clone(),
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    crate::tag::CompilerReferenceTag::It.key(),
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                )],
            },
        ],
        chosen_tag,
    )))
}

/// "You may play that card this turn.": the permission over the exiled card.
fn play_this_turn_permission(
    sentence: &SentenceInput,
    exiled: &TagKey,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(permission) = parse_cast_or_play_tagged_clause(sentence.lowered())? else {
        return Ok(None);
    };
    let Some(permission) = rebind_permission_tag(permission, exiled.clone()) else {
        return Ok(None);
    };
    Ok(matches!(
        &permission,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. },
            ..
        })
    )
    .then_some(permission))
}

/// "When you exile a nonland card this way, ..." / "When you play that card
/// this turn, ...": the event that follows the permission, with its effects.
fn permission_event(
    sentence: &SentenceInput,
) -> Result<Option<(ExilePermissionFollowupKind, Vec<EffectAst>)>, CardTextError> {
    let Some(shape) = parse_exile_permission_followup_shape(sentence.lowered()) else {
        return Ok(None);
    };
    let effects = super::parse_effect_chain(shape.effect_tokens)?;
    Ok((!effects.is_empty()).then_some((shape.kind, effects)))
}

/// Open a procedure at an exile-top sentence when the next sentence makes a
/// statement over the exiled collection.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ExiledTopGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let Some(effects) =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(sentence.lowered()))
    else {
        return Ok(None);
    };
    let Some(tag) = find_exiled_top_collection_tag(&effects) else {
        return Ok(None);
    };
    let continues = cast_collection(next, &tag)?.is_some()
        || parse_exile_top_then_put_from_among_tokens(sentence.lowered(), next.lowered())?.is_some()
        || (effects.len() == 1
            && sentences
                .get(sentence_idx + 2)
                .is_some_and(|following| matches!(permission_event(following), Ok(Some(_))))
            && crate::grammar::primitives::probe_shape(play_this_turn_permission(next, &tag))
                .flatten()
                .is_some());
    if !continues {
        return Ok(None);
    }
    Ok(Some(ExiledTopGroup {
        effects,
        tag,
        first_tokens: sentence.lowered().to_vec(),
        statements: Statements::None,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// Continue an open procedure with the next sentence. Returns false, leaving
/// the group untouched, when the sentence is not one of its statements.
pub(super) fn continue_with(
    group: &mut ExiledTopGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    match &group.statements {
        Statements::None => {
            if let Some((effects, chosen)) = cast_collection(sentence, &group.tag)? {
                group.effects.extend(effects);
                group.statements = Statements::Cast {
                    chosen,
                    partitioned: false,
                };
            } else if let Some(effects) =
                parse_exile_top_then_put_from_among_tokens(&group.first_tokens, sentence.lowered())?
            {
                // The battlefield statement reads the exile and itself as one
                // clause and spells the exile itself.
                group.effects = effects;
                group.statements = Statements::Battlefield;
            } else if group.effects.len() == 1
                && let Some(permission) =
                    crate::grammar::primitives::probe_shape(play_this_turn_permission(
                        sentence, &group.tag,
                    ))
                    .flatten()
            {
                group.statements = Statements::Permission(permission);
            } else {
                return Ok(false);
            }
        }
        Statements::Cast {
            chosen,
            partitioned: false,
        } => {
            let Some(partition) =
                parse_remaining_exiled_partition(sentence.lowered(), &group.tag, chosen)?
            else {
                return Ok(false);
            };
            let chosen = chosen.clone();
            group.effects.extend(partition);
            group.statements = Statements::Cast {
                chosen,
                partitioned: true,
            };
        }
        Statements::Permission(_) => {
            let Some((kind, followup_effects)) = permission_event(sentence)? else {
                return Ok(false);
            };
            let Statements::Permission(permission) =
                std::mem::replace(&mut group.statements, Statements::PermissionWithEvent)
            else {
                unreachable!("matched a pending permission");
            };
            match kind {
                ExilePermissionFollowupKind::ReflexiveExileNonland => {
                    group.effects.push(EffectAst::WhenResult {
                        predicate: IfResultPredicate::AffectedObjectMatchesCardType {
                            card_type: CardType::Land,
                            negated: true,
                        },
                        effects: followup_effects,
                    });
                    group.effects.push(permission);
                }
                ExilePermissionFollowupKind::DelayedPlayCard => {
                    group.effects.push(permission);
                    let tagged = ObjectFilter::tagged(group.tag.clone());
                    let trigger = TriggerSpec::Either(
                        Box::new(TriggerSpec::SpellCast {
                            filter: Some(tagged.clone()),
                            mana_source_filter: None,
                            caster: PlayerFilter::You,
                            timing: None,
                            during_turn: None,
                            min_spells_this_turn: None,
                            exact_spells_this_turn: None,
                            from_not_hand: false,
                        }),
                        Box::new(TriggerSpec::PlayerPlaysLand {
                            player: PlayerFilter::You,
                            filter: tagged,
                        }),
                    );
                    group.effects.push(EffectAst::DelayedTriggerThisTurn {
                        trigger,
                        effects: followup_effects,
                        one_shot: true,
                        until_end_of_combat: false,
                        attach_to_previous_ability: false,
                    });
                }
            }
        }
        Statements::Cast {
            partitioned: true, ..
        }
        | Statements::Battlefield
        | Statements::PermissionWithEvent => return Ok(false),
    }
    group.consumed += 1;
    Ok(true)
}

/// The feature the registry programs these statements replace reported.
pub(super) fn feature_tag(group: &ExiledTopGroup) -> &'static str {
    match &group.statements {
        Statements::Cast {
            partitioned: true, ..
        } => "exiled-collection-cast-partition",
        Statements::Cast { .. } => "exiled-collection-cast-choice",
        Statements::Battlefield => "exiled-collection-battlefield",
        Statements::Permission(_) | Statements::PermissionWithEvent => "exile-play-event-followup",
        Statements::None => "exiled-collection",
    }
}

/// Close the procedure: the exile, then its statements in order. A permission
/// whose event never arrived is spelled on its own.
pub(super) fn finish(group: ExiledTopGroup) -> Vec<EffectAst> {
    let mut effects = group.effects;
    if let Statements::Permission(permission) = group.statements {
        effects.push(permission);
    }
    effects
}
