use super::*;
use ironsmith_core::{PreventMatchingDamageSpec, StaticDamagePreventionAmount};

/// The event's recipient is a player, an object, or an authored union of both.
/// This reuses ordinary object filtering rather than a finite card-name table.
pub(super) fn prevention_recipient_filters(
    tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    let words = parser_token_word_refs(tokens);
    let known = parse_damage_amount_replacement_target_filters(&words)?;
    if known.0.is_some() || known.1.is_some() {
        return Ok(known);
    }
    for (index, token) in tokens.iter().enumerate() {
        if !token.is_any_word(&["or", "and"]) {
            continue;
        }
        let left = parser_token_word_refs(&tokens[..index]);
        let (player, object) = parse_damage_amount_replacement_target_filters(&left)?;
        if player.is_some() && object.is_none() {
            let object = parse_object_filter_lexed(&tokens[index + 1..], false)?;
            return Ok((player, Some(object)));
        }
    }
    let filter = if is_source_reference_words(&words) {
        let mut filter = ObjectFilter::source();
        filter.source_surface = source_reference_surface_for_words(&words);
        filter
    } else {
        parse_object_filter_lexed(tokens, false)?
    };
    Ok((None, Some(filter)))
}

pub(super) fn needs_shared_recipient_allocation(tokens: &[OwnedLexToken]) -> bool {
    parser_token_word_refs(tokens)
        .windows(3)
        .any(|words| words == ["one", "or", "more"])
        || tokens.iter().any(|token| token.is_word("and/or"))
}

pub fn parse_filtered_damage_prevention_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // The complete fixed-to-you production already has a canonical typed
    // payload. Keep its language disjoint from this more general production;
    // otherwise registry ambiguity hides the static reading of an `if` line.
    // Only a successful full specialist parse excludes an input here, so new
    // recipients, combat restrictions, thresholds, and source predicates stay
    // available to the general matcher.
    if parse_prevent_damage_to_you_from_source_filter_line(tokens)?.is_some() {
        return Ok(None);
    }
    let words = parser_token_word_refs(tokens);
    if words
        .windows(2)
        .any(|words| words == ["this", "turn"] || words == ["this", "combat"])
    {
        return Ok(None);
    }
    let Some(shape) = keyword_static_lines::parse_filtered_damage_prevention_tokens(tokens) else {
        return Ok(None);
    };
    // Quantified simultaneous recipients can share ONE prevention budget
    // (Cover of Winter), which needs a player-chosen batch allocation. The
    // single-recipient runtime action must not apply that budget once per target.
    if needs_shared_recipient_allocation(shape.damaged_tokens) {
        return Ok(None);
    }
    let (target_player_filter, target_object_filter) =
        prevention_recipient_filters(shape.damaged_tokens)?;
    let source_filter = damage_source_filter_from_shape(shape.source)?;
    if let Some(repeated) = shape.repeated_spell_target_tokens {
        // The repeated spell and recipient must refer to the event just read;
        // neither a different source kind nor a narrowed recipient is omitted.
        let source_words = parser_token_word_refs(shape.source.filter_tokens);
        if source_filter.zone != Some(Zone::Stack)
            || !source_words.contains(&"spell")
            || repeated.iter().any(|token| token.kind == TokenKind::Period)
            || needs_shared_recipient_allocation(repeated)
        {
            return Ok(None);
        }
        let repeated_filters = prevention_recipient_filters(repeated)?;
        if repeated_filters.0 != target_player_filter || repeated_filters.1 != target_object_filter
        {
            return Ok(None);
        }
    }
    let amount = match shape.amount {
        keyword_static_lines::FilteredPreventionAmountShape::All => {
            StaticDamagePreventionAmount::All
        }
        keyword_static_lines::FilteredPreventionAmountShape::AllBut(remaining) => {
            StaticDamagePreventionAmount::AllBut(remaining)
        }
        keyword_static_lines::FilteredPreventionAmountShape::Fixed(amount) => {
            let Ok(amount) = i32::try_from(amount) else {
                return Ok(None);
            };
            StaticDamagePreventionAmount::Amount(Value::Fixed(amount))
        }
        keyword_static_lines::FilteredPreventionAmountShape::Dynamic(tokens) => {
            let Some((value, used)) = parse_value(tokens) else {
                return Ok(None);
            };
            // A trailing sentence, optional action, or qualifier cannot disappear
            // behind a partially recognized value expression.
            if used != tokens.len() || tokens.iter().any(|token| token.kind == TokenKind::Period) {
                return Ok(None);
            }
            StaticDamagePreventionAmount::Amount(value)
        }
    };
    Ok(Some(StaticAbility::prevent_matching_damage(
        PreventMatchingDamageSpec {
            source_filter,
            target_player_filter,
            target_object_filter,
            combat_only: shape.combat_only,
            noncombat_only: shape.noncombat_only,
            maximum_damage: shape.maximum_damage,
            amount,
            display: render_token_slice(tokens),
        },
    )))
}

/// Persistent self-prevention uses the shared damage matcher, so the source's
/// live characteristics (or exact LKI) are tested at every damage event.
pub fn parse_permanent_self_damage_prevention_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words.iter().any(|word| matches!(*word,
        "target" | "turn" | "until" | "during" | "while" | "unless" | "if" | "then"))
        || words.windows(3).any(|words| words == ["as", "long", "as"])
    {
        return Ok(None);
    }
    let Some(shape) = keyword_static_lines::parse_permanent_damage_prevention_tokens(tokens) else {
        return Ok(None);
    };
    // Attached-object references and combat relationships retain their own
    // productions; this rule owns only a complete self recipient.
    if !is_source_reference_words(&parser_token_word_refs(shape.damaged_tokens)) {
        return Ok(None);
    }
    // Existing complete rules own unqualified self-prevention, combat-only
    // prevention, and the bare "by creatures" form. Keep registry claims
    // disjoint instead of giving equivalent behavior different AST payloads.
    let Some(source) = shape.source else { return Ok(None); };
    if shape.combat_only || shape.noncombat_only
        || parser_token_word_refs(source.filter_tokens).as_slice() == ["creatures"]
            && !source.source_noun && source.trailing_filter_tokens.is_empty()
    {
        return Ok(None);
    }
    let source_filter = damage_source_filter_from_shape(source)?;
    let mut recipient = ObjectFilter::source();
    recipient.source_surface = source_reference_surface_for_words(
        &parser_token_word_refs(shape.damaged_tokens),
    );
    Ok(Some(StaticAbility::prevent_matching_damage(PreventMatchingDamageSpec {
        source_filter,
        target_player_filter: None,
        target_object_filter: Some(recipient),
        combat_only: shape.combat_only,
        noncombat_only: shape.noncombat_only,
        maximum_damage: None,
        amount: StaticDamagePreventionAmount::All,
        display: render_token_slice(tokens),
    })))
}

/// Persistent prevention over a source/recipient relation. Unqualified attached
/// source prevention also uses this canonical Aura-owned payload, rather than
/// granting a replacement ability to the enchanted creature.
pub fn parse_persistent_filtered_damage_prevention_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words.iter().any(|word| matches!(*word,
        "target" | "turn" | "until" | "during" | "while" | "unless" | "if" | "then"))
        || words.windows(3).any(|words| words == ["as", "long", "as"])
    {
        return Ok(None);
    }
    let Some(shape) = keyword_static_lines::parse_permanent_damage_prevention_tokens(tokens) else {
        return Ok(None);
    };
    if is_source_reference_words(&parser_token_word_refs(shape.damaged_tokens)) {
        return Ok(None);
    }
    let Some(source) = shape.source else { return Ok(None); };
    if needs_shared_recipient_allocation(shape.damaged_tokens) {
        return Ok(None);
    }
    let (target_player_filter, target_object_filter) = if shape.damaged_tokens.is_empty() {
        (Some(PlayerFilter::Any), Some(ObjectFilter::permanent()))
    } else {
        prevention_recipient_filters(shape.damaged_tokens)?
    };
    Ok(Some(StaticAbility::prevent_matching_damage(PreventMatchingDamageSpec {
        source_filter: damage_source_filter_from_shape(source)?,
        target_player_filter,
        target_object_filter,
        combat_only: shape.combat_only,
        noncombat_only: shape.noncombat_only,
        maximum_damage: None,
        amount: StaticDamagePreventionAmount::All,
        display: render_token_slice(tokens),
    })))
}

/// "Prevent all damage that would be dealt to a creature by another creature
/// if they share a color." (Well-Laid Plans): an ordinary persistent shield
/// whose recipient must share a color with that damage's source when the
/// damage would be dealt; "another" makes source and recipient different
/// objects (checked with the pair, not against this permanent).
pub fn parse_shared_color_pair_damage_prevention_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clean = trim_edge_punctuation_tokens(tokens);
    let Some((head, ())) = crate::grammar::primitives::split_lexed_once_before_suffix(clean, 1, || {
        crate::grammar::primitives::phrase(&["if", "they", "share", "a", "color"])
    }) else {
        return Ok(None);
    };
    let Some(ability) = parse_persistent_filtered_damage_prevention_line(head)? else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(mut spec) = ability.payload
    else {
        return Ok(None);
    };
    if spec.target_player_filter.is_some() || !spec.source_filter.other {
        return Ok(None);
    }
    let Some(recipient) = spec.target_object_filter.as_mut() else {
        return Ok(None);
    };
    spec.source_filter.other = false;
    recipient
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::TriggeringSource.bind().into(),
            relation: crate::filter::TaggedOpbjectRelation::SharesColorWithTagged,
        });
    spec.display = render_token_slice(tokens);
    Ok(Some(StaticAbility::prevent_matching_damage(spec)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn shared_color_pair_relation_rides_on_the_recipient() {
        let ability = parse_shared_color_pair_damage_prevention_line(&lex_line(
            "Prevent all damage that would be dealt to a creature by another creature if they share a color.",
            0,
        ).unwrap()).unwrap().expect("pair shield");
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!("typed prevention payload required");
        };
        assert!(!spec.source_filter.other);
        assert!(spec.target_object_filter.unwrap().tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
        }));
    }

    fn parse(text: &str) -> Option<StaticAbility> {
        parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()).unwrap()
    }

    #[test]
    fn filtered_prevention_retains_amount_recipient_and_threshold() {
        let ability =
            parse("If a source would deal 3 or less damage to this creature, prevent that damage.")
                .unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!("typed prevention payload required");
        };
        assert_eq!(spec.maximum_damage, Some(3));
        assert_eq!(spec.amount, StaticDamagePreventionAmount::All);
        assert!(spec.target_object_filter.unwrap().source);
        let ability = parse("If a source would deal damage to you or a Hero you control, prevent all but 1 of that damage.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!()
        };
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::You));
        assert_eq!(spec.amount, StaticDamagePreventionAmount::AllBut(1));
        assert!(
            spec.target_object_filter
                .unwrap()
                .subtypes
                .contains(&Subtype::Hero)
        );
    }

    #[test]
    fn bound_prevention_amount_is_a_complete_dynamic_value() {
        let ability = parse("If a source would deal damage to equipped creature, prevent X of that damage, where X is the number of creatures you control.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!()
        };
        assert!(
            matches!(spec.amount, StaticDamagePreventionAmount::Amount(ref value) if matches!(value.unhinted(), Value::Count(_)))
        );
    }

    #[test]
    fn repeated_spell_prevention_tail_preserves_both_event_operands() {
        let ability = parse("If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that permanent or player.").unwrap();
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload
        else {
            panic!()
        };
        assert_eq!(spec.source_filter.zone, Some(Zone::Stack));
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::Any));
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent()));
        assert_eq!(
            spec.amount,
            StaticDamagePreventionAmount::Amount(Value::Fixed(1))
        );
        for text in [
            "If a creature would deal damage to a player, prevent 1 damage that spell would deal to that player.",
            "If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that player.",
            "If a spell would deal damage to a permanent or player, prevent 1 damage that spell would deal to that permanent or player. Draw a card.",
        ] {
            assert!(
                !matches!(
                    parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn subject_first_self_prevention_keeps_fixed_counter_removal() {
        use crate::grammar::attached_object_static_lines::{
            RemoveCounterPreventionAmount, parse_remove_counter_prevention_tokens,
        };
        let tokens = lex_line("If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.", 0).unwrap();
        let spec = parse_remove_counter_prevention_tokens(&tokens).unwrap();
        assert_eq!(spec.counter_type, CounterType::PlusOnePlusOne);
        assert_eq!(spec.amount, RemoveCounterPreventionAmount::Fixed(1));
        assert!(!spec.one_damage_per_counter);
        for text in [
            "If another creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.",
            "If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it. Draw a card.",
        ] {
            assert!(parse_remove_counter_prevention_tokens(&lex_line(text, 0).unwrap()).is_none());
        }
    }

    #[test]
    fn counter_shapes_keep_prevention_separate_from_genuine_replacement() {
        use crate::grammar::attached_object_static_lines::{
            PutCounterPreventionSpec, parse_put_counter_prevention_tokens,
        };
        for (text, expected_prevention) in [
            (
                "If damage would be dealt to this creature, put that many +1/+1 counters on it instead.",
                false,
            ),
            (
                "If damage would be dealt to this creature, prevent that damage and put that many +1/+1 counters on it.",
                true,
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let Some(PutCounterPreventionSpec::General {
                prevents_damage, ..
            }) = parse_put_counter_prevention_tokens(&tokens)
            else {
                panic!("{text}")
            };
            assert_eq!(prevents_damage, expected_prevention);
        }
    }

    #[test]
    fn counter_followups_keep_sign_and_whole_sentence_scope() {
        use crate::grammar::attached_object_static_lines::{
            PutCounterPreventionSpec, parse_put_counter_prevention_tokens,
        };
        let tokens = lex_line("If damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on this creature for each 1 damage prevented this way.", 0).unwrap();
        assert!(matches!(
            parse_put_counter_prevention_tokens(&tokens),
            Some(PutCounterPreventionSpec::PerPreventedAmount {
                counter_type: CounterType::MinusOneMinusOne
            })
        ));
        for text in [
            "If damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on that creature for each 1 damage prevented this way.",
            "If damage would be dealt to this creature, prevent that damage. Put a -1/-1 counter on this creature for each 1 damage prevented this way. Draw a card.",
        ] {
            assert!(
                parse_put_counter_prevention_tokens(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }

    #[test]
    fn prevention_does_not_erase_optional_or_follow_up_effects() {
        for text in [
            "If a source would deal damage to a player, you may prevent 1 of that damage.",
            "If a creature would deal combat damage to you and/or one or more creatures you control, prevent X of that damage, where X is the number of age counters on this enchantment.",
            "If a source would deal damage to one or more creatures you control, prevent 1 of that damage.",
            "If a spell you control would deal damage to an opponent, prevent that damage. Create a token.",
            "If a source would deal damage to this creature, prevent that damage and draw a card.",
            "If a source would deal damage to equipped creature, prevent X of that damage, where X is the number of creatures you control. Draw a card.",
            "If a source would deal 3 or more damage to this creature, prevent that damage.",
            "If a source would deal damage to this creature this turn, prevent that damage.",
        ] {
            assert!(
                !matches!(
                    parse_filtered_damage_prevention_line(&lex_line(text, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{text}"
            );
        }
    }
}


#[cfg(test)]
mod permanent_tests {
    use super::*;
    use crate::lexer::lex_line;

    fn payload(text: &str) -> PreventMatchingDamageSpec {
        let ability = parse_permanent_self_damage_prevention_line(&lex_line(text, 0).unwrap())
            .unwrap().expect("complete permanent prevention");
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload else {
            panic!("one declarative prevention payload");
        };
        spec
    }

    #[test]
    fn permanent_instruction_retains_source_domain_and_exact_recipient() {
        let artifact = payload("Prevent all damage that would be dealt to this creature by artifact sources.");
        assert_eq!(artifact.source_filter.zone, None);
        assert_eq!(artifact.source_filter.card_types, vec![CardType::Artifact]);
        assert!(artifact.target_object_filter.unwrap().source);
        assert_eq!(artifact.amount, StaticDamagePreventionAmount::All);
        let creature = payload("Prevent all damage that would be dealt to this creature by artifact creatures.");
        assert_eq!(creature.source_filter.zone, Some(Zone::Battlefield));
        let desert = payload("Prevent all damage that would be dealt to this creature by Deserts.");
        assert!(desert.source_filter.subtypes.contains(&Subtype::Desert));
        assert_eq!(desert.source_filter.zone, Some(Zone::Battlefield));
        let goblins = payload("Prevent all damage that would be dealt to this creature by Goblins.");
        assert_eq!(goblins.source_filter.zone, Some(Zone::Battlefield));
        let goblin_sources = payload("Prevent all damage that would be dealt to this creature by Goblin sources.");
        assert_eq!(goblin_sources.source_filter.zone, None);
        let enchanted = payload("Prevent all damage that would be dealt to this creature by enchanted creatures.");
        assert!(enchanted.source_filter.with_attached_object.is_some());
        let first_strike = payload("Prevent all damage that would be dealt to this creature by creatures with first strike.");
        assert_ne!(first_strike.source_filter, ObjectFilter::creature());
        let controlled = payload("Prevent all damage that would be dealt to this permanent by blue sources you control.");
        assert!(!controlled.combat_only && !controlled.noncombat_only);
        assert_eq!(controlled.source_filter.controller, Some(PlayerFilter::You));
        assert_eq!(controlled.source_filter.zone, None);
    }

    #[test]
    fn permanent_instruction_never_discards_duration_condition_or_another_sentence() {
        for text in [
            "Prevent all damage that would be dealt to this creature this turn.",
            "Prevent all damage that would be dealt to this creature by artifact sources until your next turn.",
            "Prevent all damage that would be dealt to this creature while it's tapped.",
            "Prevent all damage that would be dealt to this creature as long as you control an artifact.",
            "Prevent all damage that would be dealt to this creature. Draw a card.",
            "Prevent all damage that would be dealt to this creature by artifact creatures. Draw a card.",
            "Prevent all damage that would be dealt to this creature by artifact sources and draw a card.",
            "Prevent all damage that would be dealt to enchanted creature by artifact sources.",
            "Prevent all damage that would be dealt to target creature.",
        ] {
            assert!(!matches!(parse_permanent_self_damage_prevention_line(&lex_line(text, 0).unwrap()),
                Ok(Some(_))), "{text}");
        }
    }
}

#[cfg(test)]
mod persistent_relation_tests {
    use super::*;
    use crate::lexer::lex_line;

    fn payload(text: &str) -> PreventMatchingDamageSpec {
        let tokens = lex_line(text, 0).unwrap();
        let ability = parse_persistent_filtered_damage_prevention_line(&tokens)
            .unwrap().or_else(|| parse_permanent_self_damage_prevention_line(&tokens).unwrap())
            .expect("complete persistent prevention relation");
        let ironsmith_core::StaticAbilityPayload::PreventMatchingDamage(spec) = ability.payload else {
            panic!("shared typed prevention owner");
        };
        spec
    }

    #[test]
    fn passive_prevention_keeps_both_controller_scopes_and_attachment_identity() {
        let spec = payload("Prevent all damage that would be dealt to creatures you control by sources you control.");
        assert_eq!(spec.source_filter.controller, Some(PlayerFilter::You));
        assert_eq!(spec.source_filter.zone, None);
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::creature().you_control()));
        let spec = payload("Prevent all damage that would be dealt to you by sources you don't control.");
        assert_eq!(spec.source_filter.controller, Some(PlayerFilter::NotYou));
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::You));
        assert!(spec.target_object_filter.is_none());
        let spec = payload("Prevent all damage that would be dealt to enchanted creature by artifact sources.");
        assert_eq!(spec.source_filter.card_types, vec![CardType::Artifact]);
        assert_eq!(spec.source_filter.zone, None);
        assert!(spec.target_object_filter.unwrap().with_attached_object.unwrap().source);
    }

    #[test]
    fn active_prevention_and_source_only_prevention_preserve_direction_and_zone() {
        for text in [
            "Prevent all damage that this creature would deal to snow creatures.",
            "Prevent all damage that this creature would deal to red creatures.",
        ] {
            let spec = payload(text);
            assert!(spec.source_filter.source);
            assert!(!spec.target_object_filter.unwrap().source);
            assert!(spec.target_player_filter.is_none());
        }
        let spec = payload("Prevent all damage that would be dealt by instant and sorcery spells.");
        assert_eq!(spec.source_filter.zone, Some(Zone::Stack));
        assert!(spec.source_filter.card_types.contains(&CardType::Instant));
        assert!(spec.source_filter.card_types.contains(&CardType::Sorcery));
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::Any));
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent()));
    }

    #[test]
    fn source_choice_and_current_blocking_remain_typed_predicates() {
        let spec = payload("Prevent all damage that would be dealt to enchanted creature by sources of the chosen color.");
        assert!(spec.source_filter.chosen_color);
        assert_eq!(spec.source_filter.zone, None);
        let spec = payload("Prevent all damage that would be dealt to you and permanents you control by sources with the chosen name.");
        assert_eq!(spec.source_filter.name.as_deref(), Some("{chosen name}"));
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::You));
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent().you_control()));
        let spec = payload("Prevent all damage that would be dealt to this creature by creatures it's blocking.");
        assert!(spec.source_filter.blocked_by_source);
        assert!(spec.target_object_filter.unwrap().source);
    }

    #[test]
    fn persistent_prevention_cannot_swallow_temporal_conditional_or_effect_tails() {
        for text in [
            "Prevent all damage that would be dealt to you this turn by sources you don't control.",
            "Prevent all damage that would be dealt to target creature by artifact sources.",
            "Prevent all damage that would be dealt to creatures you control by sources you control until end of turn.",
            "Prevent all damage that would be dealt by instant and sorcery spells. Draw a card.",
            "Prevent all damage that this creature would deal to red creatures and draw a card.",
            "Prevent all damage that would be dealt to you by sources you don't control unless you pay 1 life.",
        ] {
            assert!(!matches!(parse_persistent_filtered_damage_prevention_line(
                &lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}");
        }
    }
}
