use super::*;
use crate::cards::builders::{EffectAst, ForEachEffectAst, LibraryActionAst, LifeResourceActionAst, SubjectVerbEffectAst, TokenActionAst};
use super::damage_prevention::{needs_shared_recipient_allocation, prevention_recipient_filters};

/// Own the complete replacement and its bounded amount-dependent additional
/// part. Reflexive triggers, choices and shared prevention allocations remain
/// with their own readers rather than being flattened into immediate effects.
pub fn parse_prevention_amount_follow_up_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(period) = tokens.iter().position(|token| token.kind == TokenKind::Period)
    else { return Ok(None); };
    let head = &tokens[..=period];
    let tail = trim_edge_punctuation_tokens(&tokens[period + 1..]);
    if tail.is_empty() || tail.iter().any(|token| token.kind == TokenKind::Period) {
        return Ok(None);
    }
    let (source_filter, recipient, combat_only, noncombat_only) =
        if let Some(shape) = keyword_static_lines::parse_filtered_damage_prevention_tokens(head) {
            if !matches!(shape.amount, keyword_static_lines::FilteredPreventionAmountShape::All)
                || shape.optional_by_controller
                || shape.maximum_damage.is_some()
                || shape.repeated_spell_target_tokens.is_some()
            { return Ok(None); }
            (damage_source_filter_from_shape(shape.source)?, shape.damaged_tokens,
                shape.combat_only, shape.noncombat_only)
        } else if let Some(shape) = keyword_static_lines::parse_passive_damage_prevention_head(head) {
            (ObjectFilter::default(), shape.damaged_tokens, shape.combat_only, shape.noncombat_only)
        } else { return Ok(None); };
    if needs_shared_recipient_allocation(recipient) { return Ok(None); }
    let (target_player_filter, target_object_filter) = prevention_recipient_filters(recipient)?;
    let amount = Value::EventValue(EventValueSpec::Amount);
    let mut damage_source_tag = None;
    let effects = if crate::grammar::effects::generic_sequence_shapes::parse_prevention_gain_life_followup_shape(tail) {
        vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer, PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
        )]
    } else if parser_token_word_refs(tail) == [
        "the", "sources", "controller", "draws", "cards", "equal", "to", "the",
        "damage", "prevented", "this", "way",
    ] {
        damage_source_tag = Some(crate::tag::CompilerReferenceTag::TriggeringSource.bind().key.clone());
        vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer, PlayerAst::TriggeringSourceController,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: amount }),
        )]
    } else if let Some(effects) = controller_and_damaged_player_each_draw(tail) {
        effects
    } else {
        let view = TokenWordView::new(tail);
        let words = view.word_refs();
        let suffix = ["for", "each", "1", "damage", "prevented", "this", "way"];
        if !words.ends_with(&suffix) { return Ok(None); }
        let Some(range) = view.token_span_for_words(0, words.len() - suffix.len())
        else { return Ok(None); };
        let body = trim_edge_punctuation_tokens(&tail[range]);
        let mut effects = crate::clause_support::parse_effect_sentences_lexed(body)?;
        if let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. }),
            ..
        })] = effects.as_mut_slice()
        {
            if !matches!(count.unhinted(), Value::Fixed(1)) { return Ok(None); }
            *count = amount;
            effects
        } else {
            // "Exile a card from your graveyard for each 1 damage prevented
            // this way." (Immortal Coil): the single action happens once per
            // point prevented.
            if effects.is_empty() { return Ok(None); }
            vec![EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count: amount, effects })]
        }
    };
    Ok(Some(StaticAbility::prevent_matching_damage_with_follow_up(
        ironsmith_core::StaticDamagePreventionFollowUp {
            source_filter, target_player_filter, target_object_filter,
            combat_only, noncombat_only, damage_source_tag, effects,
            amount_basis: ironsmith_core::PreventionFollowUpAmount::Prevented,
            display: render_token_slice(tokens),
        },
    )))
}

/// "This creature's controller and that player each draw half that many
/// cards, rounded down." (Sokrates, Athenian Teacher's granted ability): the
/// prevented damage's source controller and the damaged player each draw.
fn controller_and_damaged_player_each_draw(tail: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let words = parser_token_word_refs(tail);
    let count_words = match words.as_slice() {
        [
            "this",
            "creatures" | "creature's" | "creature’s",
            "controller",
            "and",
            "that",
            "player",
            "each",
            "draw",
            rest @ ..,
        ] => rest,
        _ => return None,
    };
    let amount = Value::EventValue(EventValueSpec::Amount);
    let count = match count_words {
        ["half", "that", "many", "cards", "rounded", "down"] => {
            Value::HalfRoundedDown(Box::new(amount))
        }
        ["that", "many", "cards"] => amount,
        _ => return None,
    };
    let draw = |player| {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                count: count.clone(),
            }),
        )
    };
    Some(vec![
        draw(PlayerAst::You),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: PlayerFilter::DamagedPlayer,
            effects: vec![draw(PlayerAst::Implicit)],
        }),
    ])
}

/// A conjoined "that many" action refers to the proposed damage, unlike an
/// action explicitly counted "for each damage prevented this way".
pub fn parse_prevention_proposed_amount_follow_up_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens_without_period = trim_edge_punctuation_tokens(tokens);
    if tokens_without_period.iter().any(|token| token.kind == TokenKind::Period) {
        return Ok(None);
    }
    let view = TokenWordView::new(tokens_without_period);
    let words = view.word_refs();
    // "prevent that damage and <program>" or, with a serial program, "prevent
    // that damage, <first>, and <second>" (Sekki, Seasons' Guide).
    let Some((boundary, tail_start)) = words.windows(4).position(|words|
        words == ["prevent", "that", "damage", "and"])
        .map(|start| (start, start + 4))
        .or_else(|| {
            let start = words.windows(3).position(|words| words == ["prevent", "that", "damage"])?;
            let range = view.token_span_for_words(start, start + 3)?;
            tokens_without_period.get(range.end).filter(|token| token.is_comma())?;
            Some((start, start + 3))
        })
    else { return Ok(None); };
    let Some(head_range) = view.token_span_for_words(0, boundary + 3)
    else { return Ok(None); };
    let Some(tail_range) = view.token_span_for_words(tail_start, words.len())
    else { return Ok(None); };
    let head = &tokens_without_period[head_range];
    let tail = &tokens_without_period[tail_range];
    let (source_filter, recipient, combat_only, noncombat_only) =
        if let Some(shape) = keyword_static_lines::parse_filtered_damage_prevention_tokens(head) {
            if !matches!(shape.amount, keyword_static_lines::FilteredPreventionAmountShape::All)
                || shape.optional_by_controller
                || shape.maximum_damage.is_some()
                || shape.repeated_spell_target_tokens.is_some()
            { return Ok(None); }
            (damage_source_filter_from_shape(shape.source)?, shape.damaged_tokens,
                shape.combat_only, shape.noncombat_only)
        } else if let Some(shape) = keyword_static_lines::parse_passive_damage_prevention_head(head) {
            (ObjectFilter::default(), shape.damaged_tokens, shape.combat_only, shape.noncombat_only)
        } else { return Ok(None); };
    if needs_shared_recipient_allocation(recipient) { return Ok(None); }
    let (target_player_filter, target_object_filter) = prevention_recipient_filters(recipient)?;
    let amount = Value::EventValue(EventValueSpec::Amount);
    let words = parser_token_word_refs(tail);
    let recipient_words = parser_token_word_refs(recipient);
    let recipient_is_attached = matches!(
        recipient_words.as_slice(),
        ["equipped" | "enchanted", "creature" | "permanent"]
    );
    let effects = if recipient_is_attached
        && words == ["put", "that", "many", "+1/+1", "counters", "on", "it"]
    {
        // "If equipped creature would be dealt damage, prevent that damage and
        // put that many +1/+1 counters on it." (Panther Habit): `it` is the
        // attached recipient, not the Equipment.
        vec![EffectAst::subject_verb_put_counters(
            crate::object::CounterType::PlusOnePlusOne,
            amount,
            crate::util::parse_target_phrase(recipient)?,
            None,
            false,
        )]
    } else if crate::util::is_source_reference_words(&recipient_words)
        && words.ends_with(&["gains", "control", "of", "it"])
        && !words[..words.len() - 1].iter().any(|word| matches!(*word, "it" | "its" | "may" | "unless" | "if" | "instead"))
    {
        // "If damage would be dealt to Khârn the Betrayer, prevent that damage
        // and an opponent of your choice gains control of it." (Khârn the
        // Betrayer): `it` is the protected source itself.
        let tail = trim_edge_punctuation_tokens(tail);
        let Some(it_index) = tail.iter().rposition(|token| token.is_word("it")) else {
            return Ok(None);
        };
        let mut rewritten = tail[..it_index].to_vec();
        rewritten.extend_from_slice(recipient);
        rewritten.extend_from_slice(&tail[it_index + 1..]);
        let effects = crate::clause_support::parse_effect_sentences_lexed(&rewritten)?;
        if effects.is_empty() { return Ok(None); }
        effects
    } else if words == ["mill", "twice", "that", "many", "cards"] {
        vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer, PlayerAst::You,
            SubjectVerbActionAst::Library(LibraryActionAst::Mill {
                count: Value::Scaled(Box::new(amount), 2),
            }),
        )]
    } else if words == ["each", "opponent", "mills", "that", "many", "cards"] {
        vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer, PlayerAst::Implicit,
                SubjectVerbActionAst::Library(LibraryActionAst::Mill { count: amount }),
            )],
        })]
    } else {
        // Any other complete program reads "that many" as the proposed damage
        // (Gloom Surgeon, Nine Lives). Programs that name a pronoun
        // antecedent, a choice, or a reflexive/conditional part keep their
        // own readers.
        const DECLINED: &[&str] = &[
            "it", "its", "may", "unless", "if", "when", "whenever", "instead", "damage",
            // Counter removal / +1/+1 placement on the recipient are the
            // put-counter and remove-counter prevention readers' programs.
            "remove", "+1/+1",
        ];
        // A serial program that both removes counters and acts again
        // ("remove that many +1/+1 counters from Sekki, and create that many
        // Spirit tokens") belongs to neither single-program counter reader.
        let serial_counter_program = words.contains(&"and")
            && words.iter().any(|word| *word == "remove" || *word == "+1/+1")
            && !words.iter().any(|word| matches!(*word, "it" | "its" | "may" | "unless" | "if" | "when" | "whenever" | "instead" | "damage"));
        if !serial_counter_program && words.iter().any(|word| DECLINED.contains(word)) { return Ok(None); }
        let tail = trim_edge_punctuation_tokens(tail);
        if tail.is_empty() { return Ok(None); }
        let effects = crate::clause_support::parse_effect_sentences_lexed(tail)?;
        if effects.is_empty() { return Ok(None); }
        effects
    };
    Ok(Some(StaticAbility::prevent_matching_damage_with_follow_up(
        ironsmith_core::StaticDamagePreventionFollowUp {
            source_filter, target_player_filter, target_object_filter,
            combat_only, noncombat_only, damage_source_tag: None, effects,
            amount_basis: ironsmith_core::PreventionFollowUpAmount::Proposed,
            display: render_token_slice(tokens),
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn complete_amount_followups_retain_event_amount_and_source_controller() {
        for text in [
            "If noncombat damage would be dealt to you, prevent that damage. You gain life equal to the damage prevented this way.",
            "If a source would deal damage to this creature, prevent that damage. The source's controller draws cards equal to the damage prevented this way.",
            "If a spell you control would deal damage to an opponent, prevent that damage. Create a 3/1 red Elemental Shaman creature token with haste for each 1 damage prevented this way.",
        ] {
            let ability = parse_prevention_amount_follow_up_line(&lex_line(text, 0).unwrap()).unwrap().unwrap();
            assert!(matches!(ability.payload, ironsmith_core::StaticAbilityPayload::PreventMatchingDamageWithFollowUp(_)));
        }
    }

    #[test]
    fn conjoined_milling_binds_proposed_amount_and_rejects_partial_programs() {
        for text in [
            "If damage would be dealt to you, prevent that damage and mill twice that many cards.",
            "If a source you control would deal damage to an opponent, prevent that damage and each opponent mills that many cards.",
        ] {
            let ability = parse_prevention_proposed_amount_follow_up_line(&lex_line(text, 0).unwrap()).unwrap().unwrap();
            let ironsmith_core::StaticAbilityPayload::PreventMatchingDamageWithFollowUp(spec) = ability.payload else { panic!() };
            assert_eq!(spec.amount_basis, ironsmith_core::PreventionFollowUpAmount::Proposed);
        }
        for text in [
            "If damage would be dealt to you, prevent that damage and mill twice that many cards. Draw a card.",
            "If a source would deal damage to you, prevent that damage and each opponent mills that many cards unless they pay 1 life.",
            "If damage would be dealt to you, you may prevent that damage and mill twice that many cards.",
        ] {
            assert!(!matches!(parse_prevention_proposed_amount_follow_up_line(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
        }
    }

    #[test]
    fn immediate_followups_do_not_claim_reflexive_or_optional_or_extra_sentences() {
        for text in [
            "If damage would be dealt to this creature, prevent that damage. When damage is prevented this way, this creature deals that much damage to any other target.",
            "If a source would deal damage to a player, you may prevent that damage. You gain life equal to the damage prevented this way.",
            "If damage would be dealt to you, prevent that damage. You gain life equal to the damage prevented this way. Draw a card.",
            "If a source would deal 3 or less damage to this creature, prevent that damage. You gain life equal to the damage prevented this way.",
        ] {
            assert!(!matches!(parse_prevention_amount_follow_up_line(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
        }
    }
}
