use super::*;

pub(super) fn parse_attachment_transition_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    let Some(index) = words.windows(3).position(|words| {
        matches!(
            words,
            ["become" | "becomes", "attached", "to"] | ["become" | "becomes", "unattached", "from"]
        )
    }) else {
        return Ok(None);
    };
    let recipient_start = trigger_word_token_start(tokens, index + 3).unwrap_or(tokens.len());
    let attachment_end = trigger_word_token_start(tokens, index).unwrap_or(tokens.len());
    if attachment_end == 0 || recipient_start >= tokens.len() {
        return Ok(None);
    }
    let participant = |tokens: &[OwnedLexToken]| -> Result<ObjectFilter, CardTextError> {
        if let Some(filter) = parse_trigger_subject_filter_lexed(tokens)? {
            return Ok(filter);
        }
        source_reference_surface_for_trigger_subject(tokens)
            .map(ObjectFilter::source_with_surface)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "attachment transition requires an object participant (clause: '{}')",
                    crate::lexer::render_token_slice(tokens),
                ))
            })
    };
    let attachment = participant(&tokens[..attachment_end])?;
    let recipient = participant(&tokens[recipient_start..])?;
    Ok(Some(TriggerSpec::AttachmentChanged {
        attachment,
        recipient,
        attached: words[index + 1] == "attached",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_or_incomplete_attachment_participants_do_not_become_source() {
        for text in [
            "other becomes attached to a creature",
            "target becomes attached to a creature",
            "an Aura becomes attached to other",
            "this Equipment becomes unattached from target",
            "unrecognizedword becomes attached to a creature",
            "an Aura becomes attached to unrecognizedword",
        ] {
            let result =
                parse_attachment_transition_trigger(&crate::lexer::lex_line(text, 0).unwrap());
            assert!(
                !matches!(result, Ok(Some(_))),
                "unrecognized participant: {text}"
            );
        }
    }

    #[test]
    fn attachment_and_recipient_filters_have_independent_sources_and_controllers() {
        let parse = |text| {
            parse_attachment_transition_trigger(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap()
        };
        let TriggerSpec::AttachmentChanged {
            attachment,
            recipient,
            attached,
        } = parse("an Aura you control becomes attached to a creature you control")
        else {
            panic!("attachment")
        };
        assert_eq!(attachment.controller, Some(PlayerFilter::You));
        assert_eq!(recipient.controller, Some(PlayerFilter::You));
        assert!(attachment.subtypes.contains(&Subtype::Aura));
        assert!(recipient.card_types.contains(&CardType::Creature));
        assert!(attached);
        let TriggerSpec::AttachmentChanged {
            attachment,
            recipient,
            attached,
        } = parse("this Equipment becomes unattached from a permanent")
        else {
            panic!("unattachment")
        };
        assert!(attachment.source && !recipient.source && !attached);
        assert!(attachment.source_surface.is_some());
        let TriggerSpec::AttachmentChanged {
            attachment,
            recipient,
            ..
        } = parse("an Aura becomes attached to this creature")
        else {
            panic!("self recipient")
        };
        assert!(!attachment.source && recipient.source);
        assert!(
            parse_attachment_transition_trigger(
                &crate::lexer::lex_line("an Aura becomes attached to", 0).unwrap()
            )
            .unwrap()
            .is_none()
        );
    }
}
