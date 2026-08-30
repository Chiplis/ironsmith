fn subject_verb_player_resource_effect(
    role: SubjectVerbRoleAst,
    player: PlayerAst,
    action: SubjectVerbActionAst,
) -> EffectAst {
    EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { role, player },
        action,
    })
}

pub fn parse_effect_with_verb(
    verb: Verb,
    subject: Option<SubjectAst>,
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject.is_some() {
            "explicit"
        } else {
            "implicit"
        }
    ));
    match verb {
        Verb::Add => parse_add_mana(tokens, subject),
        Verb::Move => parse_move(tokens),
        Verb::Deal => parse_deal_damage(tokens),
        Verb::Draw => parse_draw(tokens, subject),
        Verb::Counter => parse_counter(tokens),
        Verb::Destroy => parse_destroy(tokens),
        Verb::Exile => parse_exile(tokens, subject),
        Verb::Reveal => parse_reveal(tokens, subject),
        Verb::Look => parse_look(tokens, subject),
        Verb::Lose => {
            if resource_grammar::parse_resource_all_unspent_mana_shape(tokens) {
                return Ok(EffectAst::subject_verb_empty_mana_pool(
                    extract_subject_player(subject).unwrap_or(PlayerAst::Implicit),
                ));
            }
            if resource_grammar::parse_resource_all_abilities_shape(tokens)
                && matches!(subject, Some(SubjectAst::This) | None)
            {
                return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span_from_tokens(tokens)),
                    Vec::new(),
                    Until::Forever,
                ));
            }
            parse_lose_life(tokens, subject)
        }
        Verb::Gain => {
            if token_slice_first_is(tokens, "control") {
                parse_gain_control(tokens, subject)
            } else if token_slice_first_is(tokens, "gain")
                && token_slice_at_is(tokens, 1, "control")
            {
                parse_gain_control(&tokens[1..], subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            let has_onto = crate::lexer::contains_token_word(tokens, "onto");
            let has_from_into_zone_move =
                crate::lexer::contains_token_word(tokens, "from")
                    && crate::lexer::contains_token_word(tokens, "into");
            let has_counter_words = crate::lexer::contains_token_any_word(
                tokens,
                &["counter", "counters"],
            );

            // Prefer zone moves like "... onto the battlefield" over counter placement because
            // "counter(s)" may appear in subordinate clauses (e.g. "mana value equal to the number
            // of charge counters on this artifact").
            if has_onto || has_from_into_zone_move {
                if let Ok(effect) = parse_put_into_hand(tokens, subject) {
                    Ok(effect)
                } else if has_counter_words {
                    parse_put_counters(tokens)
                } else {
                    parse_put_into_hand(tokens, subject)
                }
            } else if has_counter_words {
                parse_put_counters(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject, None),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens, subject),
        Verb::Incubate => parse_incubate(tokens, subject),
        Verb::Proliferate => parse_proliferate(tokens),
        Verb::Tap => parse_tap(tokens),
        Verb::Attach => {
            let player = extract_subject_player(subject);
            let mut effect = parse_attach(tokens)?;
            if let Some(player) = player {
                super::bind_implicit_player_context(&mut effect, player);
            }
            Ok(effect)
        }
        Verb::Unattach => parse_unattach(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Unlock => parse_unlock_room_door(tokens, subject),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Convert => parse_convert(tokens),
        Verb::Flip => parse_flip(tokens, subject),
        Verb::Roll => parse_roll(tokens, subject),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Heal => Err(CardTextError::ParseError(format!(
            "unsupported heal clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => {
            let player = extract_subject_player(subject);
            // An explicit target "of an opponent's choice" is declared by
            // that opponent during announcement. An untargeted object "of an
            // opponent's choice" is chosen during resolution instead (for
            // example, Tasigur's graveyard return).
            if let Some(choice_shape) =
                crate::grammar::choices::parse_possessive_object_choice_tokens(
                    tokens,
                )
                && choice_shape.actor
                    == crate::grammar::choices::PossessiveObjectChoiceActor::Opponent
            {
                let mut effect = parse_return(&choice_shape.object_tokens)?;
                if let Some(player) = player {
                    super::bind_implicit_player_context(&mut effect, player);
                }
                if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::MoveToZone { target, .. }
                        | SubjectVerbActionAst::ReturnToHand { target, .. },
                    ..
                }) = &mut effect
                {
                    if let Some((filter, count, count_value)) =
                        untargeted_object_choice_parts(target)
                    {
                        let object_tag =
                            crate::util::helper_tag_for_tokens(
                                tokens,
                                "chosen",
                            );
                        *target = TargetAst::Tagged(object_tag.clone(), None);
                        return Ok(EffectAst::Sequence {
                            effects: vec![
                                EffectAst::ChooseObjects {
                                    filter,
                                    count,
                                    count_value,
                                    player: PlayerAst::Opponent,
                                    tag: object_tag,
                                },
                                effect,
                            ],
                        });
                    }

                    if !matches!(target, TargetAst::Object(..) | TargetAst::WithCount(..)) {
                        return Ok(effect);
                    }
                    let declared = std::mem::replace(
                        target,
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.as_str().into(), None),
                    );
                    return Ok(EffectAst::Sequence {
                        effects: vec![
                            EffectAst::subject_verb_explicit_target_only_for_chooser(
                                declared,
                                PlayerAst::Opponent,
                            ),
                            effect,
                        ],
                    });
                }
            }
            let mut effect = parse_return(tokens)?;
            if let Some(player) = player {
                super::bind_implicit_player_context(&mut effect, player);
            }
            Ok(effect)
        }
        Verb::Exchange => parse_exchange(tokens, subject),
        Verb::Become => parse_become(tokens, subject),
        Verb::Switch => parse_switch(tokens),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
        Verb::Shuffle => parse_shuffle(tokens, subject),
        Verb::Reorder => parse_reorder(tokens, subject),
        Verb::Reverse => parse_reverse(tokens),
        Verb::Pay => parse_pay(tokens, subject),
        Verb::Take => parse_take(tokens, subject),
        Verb::Detain => parse_detain(tokens),
        Verb::Assign => Err(CardTextError::ParseError(format!(
            "unsupported generic assign clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))),
        Verb::Goad => parse_goad(tokens),
        Verb::Suspect => parse_suspect(tokens),
        Verb::Note => parse_note(tokens),
        Verb::End => parse_end(tokens, subject),
    }
}

fn untargeted_object_choice_parts(
    target: &TargetAst,
) -> Option<(ObjectFilter, ChoiceCount, Option<Value>)> {
    match target {
        TargetAst::Object(filter, explicit_target_span, _) if explicit_target_span.is_none() => {
            Some((filter.clone(), ChoiceCount::exactly(1), None))
        }
        TargetAst::WithCount(inner, count) => {
            let TargetAst::Object(filter, explicit_target_span, _) = inner.as_ref() else {
                return None;
            };
            explicit_target_span
                .is_none()
                .then(|| (filter.clone(), *count, None))
        }
        TargetAst::WithCountValue(inner, count, value) => {
            let TargetAst::Object(filter, explicit_target_span, _) = inner.as_ref() else {
                return None;
            };
            explicit_target_span
                .is_none()
                .then(|| (filter.clone(), *count, Some(value.clone())))
        }
        _ => None,
    }
}

fn parse_unlock_room_door(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    let words = if words
        .first()
        .is_some_and(|word| matches!(*word, "unlock" | "unlocks"))
    {
        &words[1..]
    } else {
        words.as_slice()
    };
    if !crate::word_primitives::parse_sequence_complete(
        words,
        &["a", "locked", "door", "of", "a", "room", "you", "control"],
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported unlock clause (clause: '{}')",
            words.join(" ")
        )));
    }
    Ok(EffectAst::subject_verb_unlock_room_door(
        extract_subject_player(subject).unwrap_or(PlayerAst::You),
    ))
}

fn parse_reverse(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if resource_grammar::parse_resource_reverse_turn_order_shape(tokens) {
        return Ok(EffectAst::subject_verb_reverse_turn_order());
    }
    Err(CardTextError::ParseError(format!(
        "unsupported reverse clause: '{}'",
        crate::lexer::token_word_refs(tokens).join(" ")
    )))
}

fn parse_note(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if resource_grammar::parse_resource_note_life_total_shape(tokens) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::NoteLifeTotal,
        ));
    }
    Err(CardTextError::ParseError(format!(
        "unsupported note clause: '{}'",
        crate::lexer::token_word_refs(tokens).join(" ")
    )))
}

fn parse_take(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if resource_grammar::parse_resource_take_extra_turn_shape(tokens) {
        return Ok(EffectAst::subject_verb_extra_turn_after_turn(
            extract_subject_player(subject).unwrap_or(PlayerAst::You),
            ExtraTurnAnchorAst::CurrentTurn,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported take clause (clause: '{}')",
        crate::lexer::token_word_refs(tokens).join(" ")
    )))
}

fn parse_proliferate(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::subject_verb_proliferate(Value::Fixed(1)));
    }

    let (count, used) = if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word) {
        match first {
            "once" => (Value::Fixed(1), 1),
            "twice" => (Value::Fixed(2), 1),
            _ => parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing proliferate count (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                ))
            })?,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing proliferate count (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_ok = resource_grammar::parse_resource_proliferate_tail_shape(&trailing);
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing proliferate clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_proliferate(count))
}

fn parse_library_nth_from_top_destination(tokens: &[OwnedLexToken]) -> Option<Value> {
    resource_grammar::parse_resource_library_position_shape(tokens)
}

pub fn parse_look(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let subject_player = subject.and_then(|subject| match subject {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    });
    let shape =
        resource_grammar::parse_resource_look_shape(tokens, subject_player).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported look clause (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            ))
        })?;
    match shape {
        ResourceLookShape::PlayTaggedWhileExiled => Ok(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                crate::tag::CompilerReferenceTag::It.key(),
                PlayerAst::You,
                true,
                false,
                false,
                None,
            ),
        ),
        ResourceLookShape::EachPlayerHand => Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_look_at_hand(TargetAst::Player(
                PlayerFilter::IteratedPlayer,
                None,
            ))],
        }),
        ResourceLookShape::Hand {
            player,
            surface_tokens,
            followup,
        } => {
            let target = match player {
                PlayerAst::You => TargetAst::Player(PlayerFilter::You, None),
                PlayerAst::Opponent => TargetAst::Player(PlayerFilter::Opponent, None),
                PlayerAst::Target => TargetAst::Player(
                    PlayerFilter::target_player(),
                    span_from_tokens(surface_tokens),
                ),
                PlayerAst::TargetOpponent => {
                    TargetAst::Player(PlayerFilter::Opponent, span_from_tokens(surface_tokens))
                }
                PlayerAst::That => TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported look clause (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    )));
                }
            };
            let hand = EffectAst::subject_verb_look_at_hand(target);
            match followup {
                ResourceLookHandFollowup::None => Ok(hand),
                ResourceLookHandFollowup::TopCard => Ok(EffectAst::Sequence {
                    effects: vec![
                        hand,
                        EffectAst::subject_verb_look_at_top_cards(
                            PlayerAst::That,
                            Value::Fixed(1),
                            crate::tag::CompilerReferenceTag::It.key(),
                        ),
                    ],
                }),
                ResourceLookHandFollowup::TopCardAndFaceDownCreatures => Ok(EffectAst::Sequence {
                    effects: vec![
                        hand,
                        EffectAst::subject_verb_look_at_top_cards(
                            PlayerAst::That,
                            Value::Fixed(1),
                            crate::tag::CompilerReferenceTag::It.key(),
                        ),
                        EffectAst::subject_verb_look_at_objects(
                            PlayerAst::That,
                            ObjectFilter::creature().face_down(),
                        ),
                    ],
                }),
            }
        }
        ResourceLookShape::Object {
            kind,
            surface_tokens,
        } => {
            let filter = match kind {
                ResourceLookObjectKind::FaceDownCreature => ObjectFilter::creature().face_down(),
                ResourceLookObjectKind::FaceDownPermanent => ObjectFilter::permanent().face_down(),
            };
            Ok(EffectAst::subject_verb_look_at_target(TargetAst::Object(
                filter,
                span_from_tokens(surface_tokens),
                None,
            )))
        }
        ResourceLookShape::TopCards { player, count } => Ok(
            EffectAst::subject_verb_look_at_top_cards(player, count, crate::tag::CompilerReferenceTag::It.key()),
        ),
        ResourceLookShape::EachPlayerTopCards { count } => Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                crate::tag::CompilerReferenceTag::It.key(),
            )],
        }),
    }
}

pub fn parse_reorder(
    tokens: &[OwnedLexToken],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause = crate::lexer::token_word_refs(tokens).join(" ");
    let clause_words = crate::lexer::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing reorder target".to_string(),
        ));
    }

    let Some(owner) = parse_graveyard_owner_prefix_lexed(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    };
    if !matches!(
        owner.player,
        PlayerAst::You | PlayerAst::That | PlayerAst::ItsController | PlayerAst::ItsOwner
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    }
    let rest_token_idx = LexedClause::new(tokens)
        .words()
        .map_word_or_end_to_token_boundary(owner.consumed_words)
        .unwrap_or(tokens.len());
    let rest = trim_commas(&tokens[rest_token_idx..]);

    if !resource_grammar::parse_resource_reorder_tail_shape(&rest) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause tail (clause: '{clause}')"
        )));
    }

    Ok(EffectAst::subject_verb_reorder_graveyard(owner.player))
}

pub fn parse_shuffle(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if trim_edge_punctuation(tokens).is_empty() {
        // Support standalone "Shuffle." clauses. If the sentence includes an explicit player
        // subject, use it; otherwise return an implicit player that can be filled in by the
        // carry-context logic (and compiles to "you" by default).
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    let shape =
        resource_grammar::parse_resource_shuffle_shape(tokens, player).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported shuffle clause (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            ))
        })?;
    match shape {
        ResourceShuffleShape::TaggedIntoLibrary {
            player: destination_player,
            to_bottom,
        } => Ok(EffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::It.key(),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span_from_tokens(tokens)),
                    Zone::Library,
                    to_bottom,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                subject_verb_player_resource_effect(
                    SubjectVerbRoleAst::LibraryOwner,
                    destination_player,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
            ],
        }),
        ResourceShuffleShape::ShuffleLibrary {
            player: destination_player,
        } => Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            destination_player,
            SubjectVerbActionAst::ShuffleLibrary,
        )),
        ResourceShuffleShape::SimpleLibrary => Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        )),
    }
}

pub fn parse_goad(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError("missing goad target".to_string()));
    }

    if let Some(target) = parse_chosen_name_goad_target(&target_tokens)? {
        return Ok(EffectAst::subject_verb_goad(target));
    }
    if resource_grammar::parse_resource_tagged_reference_shape(&target_tokens) {
        return Ok(EffectAst::subject_verb_goad(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span_from_tokens(&target_tokens),
        )));
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_goad(target))
}

fn parse_chosen_name_goad_target(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let Some(shape) = resource_grammar::parse_resource_chosen_name_target_shape(target_tokens)
    else {
        return Ok(None);
    };
    let mut target = parse_target_phrase(shape.base_tokens)?;
    add_chosen_name_constraint_to_target(&mut target, shape.chosen_name_source);
    Ok(Some(target))
}

fn add_chosen_name_constraint_to_target(
    target: &mut TargetAst,
    chosen_name_source: ironsmith_core::ChosenNameSourceSurface,
) {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::ChosenName.key(),
                relation: TaggedOpbjectRelation::SameNameAsTagged,
            });
            filter.set_chosen_name_source_surface(Some(chosen_name_source));
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            add_chosen_name_constraint_to_target(inner, chosen_name_source);
        }
        _ => {}
    }
}

pub fn parse_detain(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing detain target".to_string(),
        ));
    }

    if resource_grammar::parse_resource_tagged_reference_shape(&target_tokens) {
        return Ok(EffectAst::subject_verb_detain(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span_from_tokens(&target_tokens),
        )));
    }

    Ok(EffectAst::subject_verb_detain(parse_target_phrase(
        &target_tokens,
    )?))
}

pub fn parse_suspect(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing suspect target".to_string(),
        ));
    }

    if resource_grammar::parse_resource_tagged_reference_shape(&target_tokens) {
        return Ok(EffectAst::subject_verb_suspect(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span_from_tokens(&target_tokens),
        )));
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "suspect target must be a creature (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_suspect(target))
}

#[cfg(test)]
mod counter_qualified_zone_move_tests {
    use super::*;

    #[test]
    fn counter_in_source_filter_does_not_turn_zone_move_into_counter_placement() {
        let tokens = crate::lexer::lex_line(
            "a card you own with a silver counter on it from exile into your hand",
            0,
        )
        .expect("zone move should lex");
        let effect = parse_effect_with_verb(Verb::Put, None, &tokens)
            .expect("counter-qualified zone move should parse");

        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Hand,
                    ..
                },
                ..
            })
        ));
    }
}
