//! The additional part of a prevention effect (CR 615.5) that reads the
//! prevented damage's source: "If damage is prevented this way, you may have
//! ~ deal that much damage to target creature." (Channel Harm), "If damage
//! from a creature source is prevented this way, ~ deals that much damage to
//! that creature." (Comeuppance), "Whenever damage from a black or red source
//! is prevented this way this turn, you gain that much life." (Samite
//! Ministration), "Whenever damage from a creature is prevented this way, each
//! commander creature you control deals damage equal to its power to that
//! creature." (Judgment of Alexander).
//!
//! The rider runs with each prevented damage event; "that much" is the
//! prevented amount and "that creature" / "the source's controller" name the
//! prevented damage's source, which lowering tags before the rider runs. A
//! fresh "target" in the rider is announced with the spell (CR 601.2c).
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, DamageActionAst, EffectAst, LifeResourceActionAst,
    PermissionEffectAst, PlayerAst, PredicateAst, SubjectVerbActionAst, SubjectVerbRoleAst,
    TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;
use crate::target::{ObjectFilter, PlayerFilter};
use winnow::Parser;
use winnow::combinator::{alt, opt};

/// How a prevention rider attaches to its shield.
pub(super) enum PreventionRider {
    /// "If damage is prevented this way, ...": the additional part of the
    /// prevention effect, run as each damage event is prevented (CR 615.5).
    Inline(EffectAst),
    /// "Whenever damage is prevented this way, ...": a delayed triggered
    /// ability linked to the shield, put on the stack each time it triggers
    /// (CR 603.7).
    Delayed(EffectAst),
}

/// Returns the rider, gated on the prevented source's quality when the
/// opening names one. `None` when the sentence is not such a rider.
pub(super) fn parse(sentence: &[OwnedLexToken]) -> Result<Option<PreventionRider>, CardTextError> {
    let clean = crate::util::trim_edge_punctuation_tokens(sentence);
    let Some((opening_end, (), after_opening)) = primitives::find_prefix(clean, || {
        primitives::phrase(&["is", "prevented", "this", "way"])
    }) else {
        return Ok(None);
    };
    let opening = &clean[..opening_end];
    let triggered = opening.first().is_some_and(|token| token.is_word("whenever"));
    let Some(quality) = parse_opening(opening) else {
        return Ok(None);
    };
    let after_opening = primitives::parse_prefix(after_opening, primitives::phrase(&["this", "turn"]))
        .map(|((), rest)| rest)
        .unwrap_or(after_opening);
    let body = primitives::parse_prefix(after_opening, primitives::comma())
        .map(|(_, rest)| rest)
        .unwrap_or(after_opening);
    let Some(body) = parse_body(body)? else {
        return Ok(None);
    };
    if triggered {
        // The source quality is part of the trigger event, not a resolution
        // check (CR 603.4 does not apply; this is no intervening "if").
        return Ok(Some(PreventionRider::Delayed(EffectAst::Delayed(
            crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn {
                trigger: crate::cards::builders::TriggerSpec::DamagePreventedThisWay {
                    source_filter: quality,
                },
                effects: vec![body],
                one_shot: false,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            },
        ))));
    }
    Ok(Some(PreventionRider::Inline(match quality {
        None => body,
        Some(filter) => EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::tag::CompilerReferenceTag::Triggering.bind(),
                filter,
            ),
            if_true: vec![body],
            if_false: Vec::new(),
        }),
    })))
}

/// "If damage" / "Whenever damage" [from a/an <quality> [source]].
/// `Some(None)` is the unqualified opening.
fn parse_opening(tokens: &[OwnedLexToken]) -> Option<Option<ObjectFilter>> {
    let ((), rest) = primitives::parse_prefix(
        tokens,
        (
            alt((primitives::kw("if"), primitives::kw("whenever"))),
            primitives::kw("damage"),
        )
            .void(),
    )?;
    if rest.is_empty() {
        return Some(None);
    }
    let ((), quality) = primitives::parse_prefix(
        rest,
        (
            primitives::kw("from"),
            alt((primitives::kw("a"), primitives::kw("an"))),
        )
            .void(),
    )?;
    let quality = match quality.split_last() {
        Some((last, head)) if last.is_word("source") => head,
        _ => quality,
    };
    source_quality_filter(quality).map(Some)
}

/// "black or red", "creature", "noncreature": the prevented source's
/// quality as it is when the damage is prevented. A bare color names only
/// the color; a type word ranges over every zone the source may be in.
fn source_quality_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    if tokens.is_empty() {
        return None;
    }
    let mut colors: Option<crate::color::ColorSet> = None;
    let mut only_colors = true;
    for (index, token) in tokens.iter().enumerate() {
        if index % 2 == 1 {
            if !token.is_word("or") {
                only_colors = false;
                break;
            }
            continue;
        }
        let Some(color) = token.as_word().and_then(crate::util::parse_color) else {
            only_colors = false;
            break;
        };
        colors = Some(colors.map_or(color, |existing| existing.union(color)));
    }
    if only_colors && tokens.len() % 2 == 1 {
        let mut filter = ObjectFilter::default();
        filter.colors = colors;
        return Some(filter);
    }
    let mut filter = crate::object_filters::parse_object_filter(tokens, false).ok()?;
    if filter.colors.is_none()
        && filter.card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
    {
        return None;
    }
    filter.zone = None;
    Some(filter)
}

fn prevented_amount() -> Value {
    Value::EventValue(EventValueSpec::Amount)
}

fn prevented_source() -> TargetAst {
    TargetAst::Tagged(crate::tag::CompilerReferenceTag::TriggeringSource.bind(), None)
}

fn prevented_source_controller() -> TargetAst {
    TargetAst::Player(
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(
            crate::tag::CompilerReferenceTag::TriggeringSource.bind(),
        )),
        None,
    )
}

fn parse_body(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.is_empty() {
        return Ok(None);
    }
    // "you gain that much life"
    if primitives::parse_all(
        tokens,
        primitives::phrase(&["you", "gain", "that", "much", "life"]),
        "prevention rider life gain",
    )
    .is_ok()
    {
        return Ok(Some(EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                amount: prevented_amount(),
            }),
        )));
    }
    // "each <sources> deal(s) damage equal to its power to that creature"
    if let Some(effect) = parse_each_source_power_damage(tokens)? {
        return Ok(Some(effect));
    }
    // "[you may have] <self> deal(s) that much damage to <recipient>"
    let (optional, rest) = match primitives::parse_prefix(
        tokens,
        primitives::phrase(&["you", "may", "have"]),
    ) {
        Some(((), rest)) => (true, rest),
        None => (false, tokens),
    };
    let Some((deal_idx, (), after_deal)) = primitives::find_prefix(rest, || {
        (
            alt((primitives::kw("deal"), primitives::kw("deals"))),
            primitives::phrase(&["that", "much", "damage", "to"]),
        )
            .void()
    }) else {
        return Ok(None);
    };
    if deal_idx == 0 || rest[..deal_idx].iter().any(|token| token.is_word("each")) {
        return Ok(None);
    }
    let recipient = if primitives::parse_all(
        after_deal,
        alt((
            primitives::phrase(&["that", "creature"]),
            primitives::phrase(&["that", "source"]),
        )),
        "prevention rider source recipient",
    )
    .is_ok()
    {
        prevented_source()
    } else if primitives::parse_all(
        after_deal,
        (
            alt((primitives::kw("the"), primitives::kw("that"))),
            alt((primitives::kw("source's"), primitives::kw("sources"))),
            primitives::kw("controller"),
        )
            .void(),
        "prevention rider source controller",
    )
    .is_ok()
    {
        prevented_source_controller()
    } else if after_deal.iter().any(|token| token.is_word("target")) {
        crate::util::parse_target_phrase(after_deal)?
    } else {
        return Ok(None);
    };
    let damage = EffectAst::subject_verb_damage(prevented_amount(), recipient);
    Ok(Some(if optional {
        EffectAst::Permissions(PermissionEffectAst::May {
            effects: vec![damage],
        })
    } else {
        damage
    }))
}

/// "each commander creature you control deals damage equal to its power to
/// that creature": each matching permanent is a damage source dealing its
/// own power to the prevented source, simultaneously.
fn parse_each_source_power_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(((), rest)) = primitives::parse_prefix(tokens, primitives::kw("each").void()) else {
        return Ok(None);
    };
    let Some((deal_idx, (), after)) = primitives::find_prefix(rest, || {
        (
            alt((primitives::kw("deal"), primitives::kw("deals"))),
            primitives::phrase(&["damage", "equal", "to", "its", "power", "to"]),
        )
            .void()
    }) else {
        return Ok(None);
    };
    if deal_idx == 0
        || primitives::parse_all(
            after,
            (
                primitives::kw("that"),
                opt(alt((primitives::kw("creature"), primitives::kw("source")))),
            )
                .void(),
            "prevention rider source recipient",
        )
        .is_err()
    {
        return Ok(None);
    }
    let Ok(sources) = crate::object_filters::parse_object_filter(&rest[..deal_idx], false) else {
        return Ok(None);
    };
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageBySources {
            sources: vec![TargetAst::Object(sources, None, None)],
            source_binding: ironsmith_core::DamageSourceSetBinding::LiveMembers,
            amount: Value::SourcePower,
            target: prevented_source(),
        }),
    )))
}

/// "If this spell was kicked, prevent the next N damage this way instead."
/// (Pollen Remedy): the kicked amount of the preceding finite shield. The
/// kicker is announced before the division (CR 601.2b, 601.2d), so the
/// divided total is fixed when the spell is cast.
pub(super) fn parse_kicked_amount_override(sentence: &[OwnedLexToken]) -> Option<i32> {
    let clean = crate::util::trim_edge_punctuation_tokens(sentence);
    let ((), rest) = primitives::parse_prefix(
        clean,
        (
            primitives::phrase(&["if", "this", "spell", "was", "kicked"]),
            opt(primitives::comma()),
            primitives::phrase(&["prevent", "the", "next"]),
        )
            .void(),
    )?;
    let (amount, rest) = primitives::parse_prefix(rest, primitives::number_token)?;
    primitives::parse_all(
        rest,
        primitives::phrase(&["damage", "this", "way", "instead"]),
        "kicked prevention amount",
    )
    .ok()?;
    i32::try_from(amount).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rider(text: &str) -> Option<EffectAst> {
        match parse(&crate::lexer::lex_line(text, 0).unwrap()).unwrap()? {
            PreventionRider::Inline(effect) | PreventionRider::Delayed(effect) => Some(effect),
        }
    }

    #[test]
    fn source_riders_bind_the_prevented_source_and_amount() {
        assert!(matches!(
            rider("If damage is prevented this way, you may have this spell deal that much damage to target creature."),
            Some(EffectAst::Permissions(PermissionEffectAst::May { .. }))
        ));
        for text in [
            "Whenever damage from a black or red source is prevented this way this turn, you gain that much life.",
            "Whenever damage from a creature is prevented this way, each commander creature you control deals damage equal to its power to that creature.",
        ] {
            assert!(
                matches!(
                    rider(text),
                    Some(EffectAst::Delayed(
                        crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn { .. }
                    ))
                ),
                "{text}"
            );
        }
        for text in [
            "If damage from a creature source is prevented this way, this spell deals that much damage to that creature.",
            "If damage from a noncreature source is prevented this way, this spell deals that much damage to the source's controller.",
        ] {
            assert!(
                matches!(
                    rider(text),
                    Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. }))
                ),
                "{text}"
            );
        }
        assert!(rider("If damage is prevented this way, draw a card.").is_none());
        let kicked = crate::lexer::lex_line(
            "If this spell was kicked, prevent the next 6 damage this way instead.",
            0,
        )
        .unwrap();
        assert_eq!(parse_kicked_amount_override(&kicked), Some(6));
    }
}
