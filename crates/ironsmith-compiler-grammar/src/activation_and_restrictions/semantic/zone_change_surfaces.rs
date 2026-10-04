use super::*;
use crate::grammar::trigger_clauses::{ZoneOrigin, parse_zone_change_surface};

/// Assemble only complete movement clauses after the older, more specialized
/// trigger readers have had their chance. No suffix or event qualifier is lost.
pub(super) fn parse_complete_zone_change(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    let Some(surface) = parse_zone_change_surface(&words) else {
        return Ok(None);
    };
    let subject_end = trigger_word_token_start(tokens, surface.subject_word_end)
        .ok_or_else(|| CardTextError::ParseError("missing zone-change subject boundary".into()))?;
    let grouped = words.starts_with(&["one", "or", "more"]);
    let subject_start = if grouped {
        trigger_word_token_start(tokens, 3).ok_or_else(|| {
            CardTextError::ParseError("missing grouped zone-change subject".into())
        })?
    } else {
        0
    };
    let subject_tokens = &tokens[subject_start..subject_end];
    let subject_words = &words[if grouped { 3 } else { 0 }..surface.subject_word_end];
    let card_subject = subject_words
        .iter()
        .any(|word| matches!(*word, "card" | "cards"));
    let source_surface = source_reference_surface_for_trigger_subject(subject_tokens);
    let this = is_source_reference_words(subject_words);
    let mut filter = if this {
        ObjectFilter::default()
    } else if let Some((source, other)) =
        parse_source_or_another_trigger_subject_filters(subject_tokens)
    {
        ObjectFilter {
            any_of: vec![source, other],
            ..ObjectFilter::default()
        }
    } else {
        parse_object_filter_lexed(subject_tokens, false)?
    };
    // The zone belongs to the movement predicate, not the object predicate:
    // an origin snapshot and the destination identity inhabit different zones.
    filter.zone = None;
    if card_subject {
        filter.nontoken = true;
        filter.set_explicit_card_noun(true);
        if subject_mentions_permanent(subject_words) && filter.card_types.is_empty() {
            filter.card_types = ObjectFilter::permanent_card().card_types;
        }
    }
    let mut event = ironsmith_core::trigger_model::ZoneChangeTrigger::new();
    let mut owner = surface
        .destination
        .as_ref()
        .and_then(|zone| zone.owner.clone());
    match surface.origin {
        Some(ZoneOrigin::Any) => {}
        Some(ZoneOrigin::Except(zone)) => event = event.from_any_except(zone),
        Some(ZoneOrigin::Zones(zones)) => {
            let origin_owner = zones.first().and_then(|zone| zone.owner.clone());
            if let (Some(destination_owner), Some(origin_owner)) = (&owner, &origin_owner)
                && *destination_owner != PlayerFilter::Any
                && *origin_owner != PlayerFilter::Any
                && destination_owner != origin_owner
            {
                return Err(CardTextError::ParseError(
                    "incompatible private-zone owners in zone-change trigger".into(),
                ));
            }
            if owner.is_none() || owner == Some(PlayerFilter::Any) {
                owner = origin_owner.or(owner);
            }
            event = event.from_any_of(zones.into_iter().map(|zone| zone.zone).collect());
        }
        None if !card_subject => {
            // An unqualified permanent noun denotes a battlefield object;
            // "returned to hand" never means a graveyard-to-hand return here.
            event = event.from(Zone::Battlefield);
        }
        None => {}
    }
    if let Some(destination) = surface.destination {
        event = event.to(destination.zone);
        if destination.zone == Zone::Graveyard {
            event =
                event.graveyard_surface(ironsmith_core::GraveyardTriggerSurface::PutIntoGraveyard);
        }
    }
    if let Some(owner) = owner {
        if let Some(subject_owner) = &filter.owner
            && *subject_owner != PlayerFilter::Any
            && owner != PlayerFilter::Any
            && subject_owner != &owner
        {
            return Err(CardTextError::ParseError(
                "incompatible subject and private-zone owner in zone-change trigger".into(),
            ));
        }
        if filter.owner.is_none() || owner != PlayerFilter::Any {
            filter.owner = Some(owner);
        }
    }
    if grouped {
        event = event.count(ironsmith_core::trigger_model::CountMode::OneOrMore);
    }
    if let Some(player) = surface.during_turn {
        event = event.during_turn(player);
    }
    if this {
        // This reader owns the explicit battlefield/graveyard look-back
        // families. Other source origins (including hidden-zone departures)
        // need their own ability-zone rule, rather than assuming that every
        // explicit origin is a look-back zone.
        if !matches!(event.from, Some(Zone::Battlefield | Zone::Graveyard)) {
            return Ok(None);
        }
        event = event.this();
        event.this_surface = source_surface;
    }
    // Keep the filter even for a source subject: an explicitly named private
    // zone still constrains its owner and can bind "that player".
    event = event.filter(filter);
    Ok(Some(TriggerSpec::ZoneChange(event)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn event(text: &str) -> ironsmith_core::trigger_model::ZoneChangeTrigger {
        let parsed = parse_complete_zone_change(&lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap();
        let TriggerSpec::ZoneChange(event) = parsed else {
            panic!("typed zone event")
        };
        event
    }

    #[test]
    fn battlefield_subject_keeps_controller_and_private_zone_owner_separate() {
        let parsed = event("a permanent you control is put into an opponent's graveyard");
        assert_eq!(parsed.from, Some(Zone::Battlefield));
        assert_eq!(parsed.to, Some(Zone::Graveyard));
        let filter = parsed.filter.unwrap();
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.owner, Some(PlayerFilter::Opponent));
        assert_eq!(filter.zone, None);
        let returned = event("a permanent is returned to a player's hand");
        assert_eq!(returned.from, Some(Zone::Battlefield));
        assert_eq!(returned.filter.unwrap().owner, Some(PlayerFilter::Any));
    }

    #[test]
    fn grouped_turn_guard_and_source_or_other_subject_survive_lowering_boundary() {
        let grouped =
            event("one or more artifacts you control leave the battlefield during your turn");
        assert_eq!(grouped.count, ironsmith_core::trigger_model::CountMode::OneOrMore);
        assert_eq!(grouped.during_turn, Some(PlayerFilter::You));
        assert_eq!(grouped.from, Some(Zone::Battlefield));
        let returned = event(
            "this creature or another creature is returned to your hand from the battlefield",
        );
        let filter = returned.filter.unwrap();
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.any_of.len(), 2);
        assert!(filter.any_of[0].source);
        assert!(filter.any_of[1].other);
    }

    #[test]
    fn card_origins_and_source_functional_zone_are_explicit() {
        let card = event("a creature card leaves an opponent's graveyard");
        assert_eq!(card.from, Some(Zone::Graveyard));
        assert!(card.filter.as_ref().unwrap().nontoken);
        assert_eq!(card.filter.unwrap().owner, Some(PlayerFilter::Opponent));
        let source = event("this card is put into your hand from your graveyard");
        assert!(source.this);
        assert_eq!(source.from, Some(Zone::Graveyard));
        assert_eq!(source.to, Some(Zone::Hand));
        let except = event(
            "an artifact card is put into your graveyard from anywhere other than the battlefield",
        );
        assert_eq!(except.from_excluded, Some(Zone::Battlefield));
        assert_eq!(except.filter.unwrap().owner, Some(PlayerFilter::You));
    }
}

#[cfg(test)]
mod entrypoint_tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn source_anywhere_is_not_silently_reinterpreted_as_a_battlefield_ability() {
        for text in [
            "this card is put into a graveyard from anywhere",
            "this card is put into exile from anywhere",
            "this card is put into your hand",
            "this card is put into your graveyard from your hand",
            "this card is put into your hand from your library",
            "this card is put into your graveyard from anywhere other than the battlefield",
        ] {
            assert!(
                parse_complete_zone_change(&lex_line(text, 0).unwrap())
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
        // The older, already-supported self-anywhere route keeps ownership.
        let old = parse_trigger_clause_lexed(
            &lex_line("this card is put into a graveyard from anywhere", 0).unwrap(),
        )
        .unwrap();
        assert!(matches!(old, TriggerSpec::PutIntoGraveyard(_)));
    }

    #[test]
    fn card_origin_cohort_reaches_typed_trigger_entrypoint_including_unions() {
        for text in [
            "a creature card leaves an opponent's graveyard",
            "a Lhurgoyf permanent card is put into your graveyard from anywhere other than the battlefield",
            "another artifact is put into your graveyard from the battlefield or an artifact card is put into your graveyard from anywhere other than the battlefield",
            "one or more creature cards are put into your graveyard from anywhere during your turn",
            "this card is put into your hand from your graveyard",
            "this artifact is put into exile from the battlefield",
            "one or more cards are put into a library from anywhere",
            "a creature dies or a creature card is put into a graveyard from a library",
            "you sacrifice a Desert and whenever a Desert card is put into your graveyard from your hand or library",
        ] {
            parse_trigger_clause_lexed(&lex_line(text, 0).unwrap())
                .unwrap_or_else(|error| panic!("{text}: {error:?}"));
        }
    }

    #[test]
    fn complete_permanent_zone_cohort_reaches_typed_trigger_entrypoint() {
        for text in [
            "a permanent you control is put into a graveyard",
            "a permanent is put into an opponent's graveyard",
            "a nontoken permanent is put into a player's graveyard from the battlefield",
            "a permanent is returned to your hand",
            "another nonland permanent you control is returned to its owner's hand",
            "this creature or another creature is returned to your hand from the battlefield",
            "one or more noncreature permanents are returned to hand",
            "a permanent is returned to a player's hand",
            "one or more artifacts you control leave the battlefield during your turn",
            "another permanent you control leaves the battlefield during your turn",
        ] {
            parse_trigger_clause_lexed(&lex_line(text, 0).unwrap())
                .unwrap_or_else(|error| panic!("{text}: {error:?}"));
        }
    }
}
