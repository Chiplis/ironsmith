//! Complete lifecycle heads; characteristics describe the completed permanent.
use super::*;

fn subject(tokens: &[OwnedLexToken]) -> Result<ObjectFilter, CardTextError> {
    match parse_trigger_subject_filter_lexed(tokens)? {
        Some(filter) => Ok(filter),
        None => source_reference_surface_for_trigger_subject(tokens)
            .map(ObjectFilter::source_with_surface)
            .ok_or_else(|| {
                CardTextError::ParseError("lifecycle trigger requires an object subject".into())
            }),
    }
}

pub(super) fn parse_permanent_lifecycle_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    if let Some(index) = words
        .iter()
        .position(|word| matches!(*word, "transforms" | "transform"))
    {
        if index == 0 {
            return Ok(None);
        }
        let end = trigger_word_token_start(tokens, index).unwrap_or(tokens.len());
        let tail = &words[index + 1..];
        // Self + named destination remains owned by the existing self-face reader.
        if source_reference_surface_for_trigger_subject(&tokens[..end]).is_some() {
            return Ok(None);
        }
        if tail.is_empty() {
            return Ok(Some(TriggerSpec::PermanentTransforms(subject(
                &tokens[..end],
            )?)));
        }
        if tail.first() == Some(&"into") && tail.len() > 1 {
            let start = trigger_word_token_start(tokens, index + 2).unwrap_or(tokens.len());
            let destination = parse_object_filter_lexed(&tokens[start..], false)?;
            return Ok(Some(TriggerSpec::PermanentTransformsInto {
                filter: subject(&tokens[..end])?,
                destination,
            }));
        }
        return Ok(None);
    }
    if words
        .last()
        .is_some_and(|word| matches!(*word, "mutates" | "mutate"))
        && words.len() > 1
    {
        let end = trigger_word_token_start(tokens, words.len() - 1).unwrap_or(tokens.len());
        if source_reference_surface_for_trigger_subject(&tokens[..end]).is_some() {
            return Ok(None);
        }
        return Ok(Some(TriggerSpec::PermanentMutates(subject(
            &tokens[..end],
        )?)));
    }
    if words.ends_with(&["becomes", "renowned"]) || words.ends_with(&["become", "renowned"]) {
        let end = trigger_word_token_start(tokens, words.len() - 2).unwrap_or(tokens.len());
        if end == 0 {
            return Ok(None);
        }
        return Ok(Some(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Renown,
            player: PlayerFilter::Any,
            source_filter: Some(subject(&tokens[..end])?),
            during_your_turn: false,
        }));
    }
    if words.ends_with(&["face", "up"]) {
        if let Some(index) = words
            .iter()
            .position(|word| matches!(*word, "turn" | "turns"))
        {
            let Some(player) = parse_trigger_subject_player_filter(&words[..index]) else {
                return Ok(None);
            };
            let Some(start) = trigger_word_token_start(tokens, index + 1) else {
                return Ok(None);
            };
            let end = trigger_word_token_start(tokens, words.len() - 2).unwrap_or(tokens.len());
            if start >= end {
                return Ok(None);
            }
            return Ok(Some(TriggerSpec::PlayerTurnsFaceUp {
                player,
                filter: subject(&tokens[start..end])?,
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> TriggerSpec {
        parse_permanent_lifecycle_trigger(&crate::lexer::lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap()
    }
    #[test]
    fn complete_heads_keep_post_change_types_subjects_and_actor_distinct() {
        let TriggerSpec::PermanentTransformsInto {
            filter,
            destination,
        } = parse("a permanent you control transforms into a non-Human creature")
        else {
            panic!("transform");
        };
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(
            destination
                .card_types
                .contains(&crate::types::CardType::Creature)
        );
        assert!(
            destination
                .excluded_subtypes
                .contains(&crate::types::Subtype::Human)
        );
        let TriggerSpec::PermanentTransformsInto { destination, .. } =
            parse("a permanent you control transforms into a Phyrexian")
        else {
            panic!("transform subtype");
        };
        assert!(
            destination
                .subtypes
                .contains(&crate::types::Subtype::Phyrexian)
        );
        let TriggerSpec::PermanentTransforms(filter) = parse("equipped creature transforms") else {
            panic!("equipped");
        };
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == "equipped")
        );
        let TriggerSpec::PermanentMutates(filter) = parse("a creature you control mutates") else {
            panic!("mutate");
        };
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        let TriggerSpec::KeywordAction {
            action,
            player,
            source_filter: Some(filter),
            ..
        } = parse("a creature you control becomes renowned")
        else {
            panic!("renown");
        };
        assert_eq!(action, crate::events::KeywordActionKind::Renown);
        assert_eq!(player, PlayerFilter::Any);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        let TriggerSpec::PlayerTurnsFaceUp { player, filter } =
            parse("you turn a permanent face up")
        else {
            panic!("actor");
        };
        assert_eq!(player, PlayerFilter::You);
        assert_eq!(filter.controller, None);
    }
    #[test]
    fn incomplete_heads_and_unknown_participants_do_not_become_self() {
        for text in [
            "transforms",
            "becomes renowned",
            "you turn face up",
            "a permanent transforms sideways",
            "someone imaginary turns a permanent face up",
        ] {
            let result =
                parse_permanent_lifecycle_trigger(&crate::lexer::lex_line(text, 0).unwrap());
            assert!(!matches!(result, Ok(Some(_))), "{text}: {result:?}");
        }
        assert!(
            parse_permanent_lifecycle_trigger(
                &crate::lexer::lex_line("this creature transforms into this creature", 0).unwrap()
            )
            .unwrap()
            .is_none()
        );
    }
}
