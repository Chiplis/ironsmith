//! "You may cast it this turn. If a spell cast this way would be put into
//! your graveyard, exile it instead." (Radiant Scrollwielder, Mission
//! Briefing): a one-turn cast permission and the zone-change replacement on
//! the spell it permits (CR 614.1a). The permission names the card by a tag;
//! the card keeps its identity as it becomes a spell (CR 601.2a), so the
//! replacement watches that tagged object moving from the stack to a
//! graveyard for as long as the permission lasts.

use crate::cards::builders::{
    EffectAst, FutureZoneReplacementCausePolicyAst, GrantActionAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, ZoneReplacementDurationAst,
};
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use crate::tag::TagRef;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

/// What the rider says about the spell.
struct GraveyardExileRider {
    cast_this_way: bool,
    those_spells: bool,
    your_graveyard: bool,
}

/// "a spell cast this way". "If that spell would be put into ..." after a
/// this-turn permission already reads through the anaphoric spell reference.
fn rider_subject<'a>(input: &mut LexStream<'a>) -> WResult<(bool, bool)> {
    alt((
        primitives::phrase(&["a", "spell", "cast", "this", "way"]).value((true, false)),
        primitives::phrase(&[
            "an", "instant", "or", "sorcery", "spell", "cast", "this", "way",
        ])
        .value((true, false)),
        // "If those spells would be put into your graveyard, exile them
        // instead." (Invoke Calamity): the spells an immediate collection
        // cast just put on the stack.
        primitives::phrase(&["those", "spells"]).value((false, true)),
    ))
    .parse_next(input)
}

fn rider_line<'a>(input: &mut LexStream<'a>) -> WResult<GraveyardExileRider> {
    primitives::kw("if").parse_next(input)?;
    let (cast_this_way, those_spells) = rider_subject.parse_next(input)?;
    primitives::phrase(&["would", "be", "put", "into"]).parse_next(input)?;
    let your_graveyard = alt((
        primitives::kw("your").value(true),
        primitives::kw("a").value(false),
    ))
    .parse_next(input)?;
    primitives::kw("graveyard").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    alt((
        primitives::phrase(&["exile", "it", "instead"]),
        primitives::phrase(&["exile", "them", "instead"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(GraveyardExileRider {
        cast_this_way,
        those_spells,
        your_graveyard,
    })
}

/// The tag of the last this-turn cast permission among the statement's
/// effects, looking through plain sequences.
fn last_this_turn_permission_tag(effects: &[EffectAst]) -> Option<TagRef> {
    effects.iter().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                    tag,
                    while_on_top_of_library: false,
                    until_source_exiles_another: false,
                    ..
                }),
            ..
        }) => Some(tag.clone()),
        // An immediate collection cast: the chosen cards each cast in turn.
        EffectAst::ForEach(crate::cards::builders::ForEachEffectAst::ForEachTagged {
            tag,
            effects,
        }) if effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Stack(
                        crate::cards::builders::StackActionAst::CastTagged { .. }
                    ),
                    ..
                })
            )
        }) => Some(tag.clone()),
        EffectAst::Sequence { effects } => last_this_turn_permission_tag(effects),
        _ => None,
    })
}

/// The last filtered graveyard cast permission among the statement's effects.
fn last_graveyard_grant_spec(
    effects: &mut [EffectAst],
) -> Option<&mut crate::model::CompilerGrantSpecCore> {
    effects.iter_mut().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec { spec, .. }),
            ..
        }) if spec.zone == Zone::Graveyard => Some(spec.as_mut()),
        EffectAst::Sequence { effects } => last_graveyard_grant_spec(effects),
        _ => None,
    })
}

/// Bind the rider to the permission the statement granted this turn.
pub(super) fn bind_cast_spell_graveyard_exile_rider(
    effects: &mut Vec<EffectAst>,
    sentence: &[OwnedLexToken],
) -> bool {
    let Some(rider) =
        primitives::probe_all(sentence, rider_line, "cast spell graveyard exile rider")
    else {
        return false;
    };
    let Some(tag) = last_this_turn_permission_tag(effects) else {
        // "Until end of turn, you may cast instant and sorcery spells from
        // the top of your graveyard. If a spell cast this way would be put
        // into a graveyard, exile it instead." (Bösium Strip): a filtered
        // zone permission carries the replacement as a cast-this-way grant
        // on each spell it permits.
        if !rider.cast_this_way {
            return false;
        }
        let Some(spec) = last_graveyard_grant_spec(effects) else {
            return false;
        };
        let owner = if rider.your_graveyard {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        };
        spec.cast_this_way_grants.push(
            crate::model::CompilerStaticAbilityCore::exile_to_exile_instead_of_graveyard(
                ObjectFilter::source(),
                owner,
            ),
        );
        return true;
    };
    let mut spell = ObjectFilter::tagged(tag.key().clone()).in_zone(Zone::Stack);
    if rider.your_graveyard {
        // A spell goes to its owner's graveyard (CR 400.3): "your graveyard"
        // is a spell you own.
        spell = spell.owned_by(PlayerFilter::You);
    }
    let replacement = EffectAst::subject_verb_register_future_zone_replacement(
        spell,
        Some(Zone::Stack),
        Some(Zone::Graveyard),
        Zone::Exile,
        // The permission lasts this turn; so does the spell it permits.
        ZoneReplacementDurationAst::UntilEndOfTurn,
        FutureZoneReplacementCausePolicyAst::Any,
        false,
    );
    effects.push(if rider.cast_this_way {
        replacement.with_cast_this_way_replacement_surface()
    } else if rider.those_spells {
        replacement.with_those_spells_replacement_surface()
    } else {
        replacement
    });
    true
}

/// The entry flag of the last tagged one-turn play permission that may play
/// a land or cast a permanent spell.
fn last_this_turn_permission_enters_tapped(effects: &mut [EffectAst]) -> Option<&mut bool> {
    effects.iter_mut().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                    while_on_top_of_library: false,
                    until_source_exiles_another: false,
                    permanents_enter_tapped,
                    ..
                }),
            ..
        }) if !*permanents_enter_tapped => Some(permanents_enter_tapped),
        EffectAst::Sequence { effects }
        | EffectAst::Permissions(
            crate::cards::builders::PermissionEffectAst::May { effects }
            | crate::cards::builders::PermissionEffectAst::MayByPlayer { effects, .. },
        ) => last_this_turn_permission_enters_tapped(effects),
        _ => None,
    })
}

/// What "If you do, it enters ..." says about the permanent a this-turn
/// graveyard cast permission produces.
struct GraveyardEntryRider<'a> {
    tapped: bool,
    counter_tokens: Option<&'a [OwnedLexToken]>,
    addition_tokens: Option<&'a [OwnedLexToken]>,
}

fn entry_rider_line<'a>(input: &mut LexStream<'a>) -> WResult<GraveyardEntryRider<'a>> {
    use winnow::combinator::{peek, repeat_till};
    use winnow::token::any;
    primitives::phrase(&["if", "you", "do"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "enters"]).parse_next(input)?;
    if opt((primitives::kw("tapped"), primitives::sentence_end()))
        .parse_next(input)?
        .is_some()
    {
        return Ok(GraveyardEntryRider {
            tapped: true,
            counter_tokens: None,
            addition_tokens: None,
        });
    }
    primitives::kw("with").parse_next(input)?;
    alt((primitives::kw("a"), primitives::kw("an"), primitives::kw("one"))).parse_next(input)?;
    let counter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["counter", "on", "it"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["counter", "on", "it"]).parse_next(input)?;
    let addition_tokens = opt(winnow::combinator::preceded(
        primitives::kw("and"),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), _)| ())
            .take(),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(GraveyardEntryRider {
        tapped: false,
        counter_tokens: Some(counter_tokens),
        addition_tokens,
    })
}

/// "{T}: You may cast a creature spell from your graveyard this turn. If you
/// do, it enters with a finality counter on it and is a Vampire in addition
/// to its other types." (The Tomb of Aclazotz): the entry rider is a
/// cast-this-way grant on the spell the permission permits (it enters with
/// the counter, CR 614.1c), and the added type stays with the permanent.
pub(super) fn bind_cast_spell_graveyard_entry_rider(
    effects: &mut [EffectAst],
    sentence: &[OwnedLexToken],
) -> bool {
    let Some(rider) =
        primitives::probe_all(sentence, entry_rider_line, "cast spell graveyard entry rider")
    else {
        return false;
    };
    let Some(spec) = last_graveyard_grant_spec(effects) else {
        // "You may play target snow permanent card from your graveyard this
        // turn. If you do, it enters tapped." (Kaldring, the Rimestaff): the
        // tagged one-turn permission's played land or cast permanent spell
        // enters tapped.
        if !rider.tapped {
            return false;
        }
        let Some(permanents_enter_tapped) = last_this_turn_permission_enters_tapped(effects) else {
            return false;
        };
        *permanents_enter_tapped = true;
        return true;
    };
    if !matches!(spec.grantable, crate::model::CompilerGrantableCore::PlayFrom)
        || !spec.cast_this_way_grants.is_empty()
        || !spec.permanent_this_way_grants.is_empty()
    {
        return false;
    }
    let mut cast_this_way = Vec::new();
    let mut permanent_this_way = Vec::new();
    if rider.tapped {
        cast_this_way.push(crate::model::CompilerStaticAbilityCore::enters_tapped_ability());
    }
    if let Some(counter_tokens) = rider.counter_tokens {
        let Some(counter) =
            crate::grammar::filters::parse_counter_type_from_tokens(counter_tokens)
        else {
            return false;
        };
        cast_this_way.push(
            crate::model::CompilerStaticAbilityCore::enters_with_counters_value(
                counter,
                crate::effect::Value::Fixed(1),
            ),
        );
    }
    if let Some(addition_tokens) = rider.addition_tokens {
        let Ok(Some(additions)) =
            crate::keyword_static::parse_type_color_addition_clause(addition_tokens)
        else {
            return false;
        };
        if additions.subtypes.is_empty()
            || !additions.card_types.is_empty()
            || !additions.supertypes.is_empty()
            || !additions.added_colors.is_empty()
            || !additions.set_colors.is_empty()
        {
            return false;
        }
        permanent_this_way.push(crate::model::CompilerStaticAbilityCore::add_subtypes(
            ObjectFilter::source(),
            additions.subtypes,
        ));
    }
    spec.cast_this_way_grants.extend(cast_this_way);
    spec.permanent_this_way_grants.extend(permanent_this_way);
    true
}
