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
    your_graveyard: bool,
}

/// "a spell cast this way". "If that spell would be put into ..." after a
/// this-turn permission already reads through the anaphoric spell reference.
fn rider_subject<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        primitives::phrase(&["a", "spell", "cast", "this", "way"]).value(true),
        primitives::phrase(&[
            "an", "instant", "or", "sorcery", "spell", "cast", "this", "way",
        ])
        .value(true),
        // "If those spells would be put into your graveyard, exile them
        // instead." (Invoke Calamity): the spells an immediate collection
        // cast just put on the stack.
        primitives::phrase(&["those", "spells"]).value(false),
    ))
    .parse_next(input)
}

fn rider_line<'a>(input: &mut LexStream<'a>) -> WResult<GraveyardExileRider> {
    primitives::kw("if").parse_next(input)?;
    let cast_this_way = rider_subject.parse_next(input)?;
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
    } else {
        replacement
    });
    true
}
