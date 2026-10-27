use crate::cards::builders::CounterActionAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::LifeResourceActionAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::ZoneMoveActionAst;
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
        Verb::Destroy => {
            let mut effect = parse_destroy(tokens)?;
            // "target opponent destroys four lands they control" (Burning of
            // Xinye): a named non-you actor performs the destroy and chooses
            // the objects; keep that actor on the instruction.
            if let Some(player) = extract_subject_player(subject)
                && !matches!(player, PlayerAst::You | PlayerAst::Implicit)
                && let EffectAst::SubjectVerb(subject_verb) = &mut effect
                && matches!(
                    subject_verb.action,
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })
                )
                && subject_verb.subject.player == PlayerAst::Implicit
            {
                subject_verb.subject.player = player;
            }
            Ok(effect)
        }
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
                    TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        span_from_tokens(tokens),
                    ),
                    Vec::new(),
                    Until::Forever,
                ));
            }
            if let Some(effect) = parse_lose_all_player_counters(tokens, subject)? {
                return Ok(effect);
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
            let has_from_into_zone_move = crate::lexer::contains_token_word(tokens, "from")
                && crate::lexer::contains_token_word(tokens, "into");
            let has_counter_words =
                crate::lexer::contains_token_any_word(tokens, &["counter", "counters"]);

            // Prefer zone moves like "... onto the battlefield" over counter placement because
            // "counter(s)" may appear in subordinate clauses (e.g. "mana value equal to the number
            // of charge counters on this artifact").
            // "They put two +1/+1 counters on a creature they control": the
            // authored player subject is the actor who makes any untargeted
            // object choice.
            let parse_put_counters_with_subject = |tokens: &[OwnedLexToken]| {
                if let Some(player) = extract_subject_player(subject)
                    && !matches!(player, PlayerAst::You | PlayerAst::Implicit)
                    && let Some(effect) =
                        crate::effect_sentences::zone_counter_helpers::parse_put_counters_with_subject_chosen_target(
                            tokens, player,
                        )
                {
                    return Ok(effect);
                }
                let mut effect = parse_put_counters(tokens)?;
                if let Some(player) = extract_subject_player(subject)
                    && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        subject: counter_subject,
                        action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { .. }),
                    }) = &mut effect
                    && counter_subject.player == PlayerAst::Implicit
                {
                    counter_subject.player = player;
                }
                Ok::<_, CardTextError>(effect)
            };
            // "Put all exiled cards you own with intel counters on them into
            // your hand" (Flamewar): a destination-zone tail names a zone
            // move even without "from"; the counters only qualify the cards.
            let has_into_zone_tail = crate::lexer::contains_token_word(tokens, "into")
                && crate::lexer::token_word_refs(tokens)
                    .last()
                    .is_some_and(|word| matches!(*word, "hand" | "hands" | "graveyard" | "graveyards"));
            if has_onto || has_from_into_zone_move || has_into_zone_tail {
                if let Ok(effect) = parse_put_into_hand(tokens, subject) {
                    Ok(effect)
                } else if has_counter_words {
                    parse_put_counters_with_subject(tokens)
                } else {
                    parse_put_into_hand(tokens, subject)
                }
            } else if has_counter_words {
                parse_put_counters_with_subject(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject, None),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens, subject),
        Verb::Assimilate => parse_assimilate(tokens, subject),
        Verb::Incubate => parse_incubate(tokens, subject),
        Verb::Proliferate => parse_proliferate(tokens),
        Verb::Tap => {
            let player = extract_subject_player(subject);
            let mut effect = parse_tap(tokens)?;
            if let Some(player) = player {
                super::bind_implicit_player_context(&mut effect, player);
            }
            Ok(effect)
        }
        Verb::Attach => {
            let player = extract_subject_player(subject);
            let mut effect = parse_attach(tokens)?;
            if let Some(player) = player {
                super::bind_implicit_player_context(&mut effect, player);
            }
            Ok(effect)
        }
        Verb::Unattach => parse_unattach(tokens),
        Verb::Untap => {
            let player = extract_subject_player(subject);
            let mut effect = parse_untap(tokens)?;
            if let Some(player) = player {
                super::bind_implicit_player_context(&mut effect, player);
            }
            Ok(effect)
        }
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
                crate::grammar::choices::parse_possessive_object_choice_tokens(tokens)
                && choice_shape.actor
                    == crate::grammar::choices::PossessiveObjectChoiceActor::Opponent
            {
                let mut effect = parse_return(&choice_shape.object_tokens)?;
                if let Some(player) = player {
                    super::bind_implicit_player_context(&mut effect, player);
                }
                if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                            target, ..
                        })
                        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                            target, ..
                        }),
                    ..
                }) = &mut effect
                {
                    if let Some((filter, count, count_value)) =
                        untargeted_object_choice_parts(target)
                    {
                        let object_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");
                        *target =
                            TargetAst::Tagged(crate::tag::TagRef::of(object_tag.clone()), None);
                        return Ok(EffectAst::Sequence {
                            effects: vec![
                                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                                    filter,
                                    count,
                                    count_value,
                                    player: PlayerAst::Opponent,
                                    tag: crate::tag::TagRef::of(object_tag),
                                }),
                                effect,
                            ],
                        });
                    }

                    if !matches!(target, TargetAst::Object(..) | TargetAst::WithCount(..)) {
                        return Ok(effect);
                    }
                    let declared = std::mem::replace(
                        target,
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
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
        Verb::Assign => {
            let (duration, body) = crate::effect_sentences::search_library::parse_restriction_duration(tokens)?
                .unwrap_or_else(|| (Until::Forever, tokens.to_vec()));
            if let Some(no_defender) = crate::grammar::effects::toughness_assignment::assignment_body(&body) {
                let target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span_from_tokens(tokens));
                let mut abilities = vec![crate::cards::builders::GrantedAbilityAst::StaticAbility(Box::new(
                    crate::cards::builders::StaticAbilityAst::Static(
                        crate::model::CompilerStaticAbilityCore::this_creature_assigns_combat_damage_using_toughness(),
                    ),
                ))];
                if no_defender {
                    abilities.push(crate::cards::builders::GrantedAbilityAst::CanAttackAsThoughNoDefender);
                }
                Ok(EffectAst::subject_verb_grant_abilities_to_target(target, abilities, duration))
            } else if let Some(unblocked_duration) = may_assign_as_unblocked_body(tokens) {
                // "You may have this creature assign its combat damage this
                // turn as though it weren't blocked." (Gurzigost): a resolving
                // grant of the optional unblocked assignment (CR 510.1c); the
                // choice itself stays with the creature's controller as
                // combat damage is assigned.
                let target = match subject {
                    Some(SubjectAst::This) => TargetAst::Source(span_from_tokens(tokens)),
                    _ => TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span_from_tokens(tokens)),
                };
                Ok(EffectAst::subject_verb_grant_abilities_to_target(
                    target,
                    vec![crate::cards::builders::GrantedAbilityAst::StaticAbility(Box::new(
                        crate::cards::builders::StaticAbilityAst::Static(
                            crate::model::CompilerStaticAbilityCore::may_assign_damage_as_unblocked(),
                        ),
                    ))],
                    unblocked_duration,
                ))
            } else {
                Err(CardTextError::ParseError(format!("unsupported generic assign clause (clause: '{}')", crate::lexer::token_word_refs(tokens).join(" "))))
            }
        }
        Verb::Goad => parse_goad(tokens),
        Verb::Suspect => parse_suspect(tokens),
        Verb::Note => parse_note(tokens, subject),
        Verb::End => parse_end(tokens, subject),
        Verb::Activate => {
            // "<player> activates a mana ability of each land they control"
            // (Drain Power, Pygmy Hippo).
            let words = crate::lexer::token_word_refs(tokens);
            if matches!(
                words.as_slice(),
                ["a", "mana", "ability", "of", "each", "land", "they" | "you", "control"]
            ) {
                Ok(EffectAst::subject_verb_activate_mana_ability_of_each_land(
                    extract_subject_player(subject).unwrap_or(PlayerAst::Implicit),
                ))
            } else {
                Err(CardTextError::ParseError(format!(
                    "unsupported activate clause (clause: '{}')",
                    words.join(" ")
                )))
            }
        }
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
    // "unlock a locked door of up to one target Room you control" (Ghostly
    // Keybearer): the Room is announced as a target (CR 709.5f). "Lock or
    // unlock a door of target Room you control" (Keys to the House) names any
    // door, locked or unlocked: the only authored unlock of an unqualified
    // door is the lock-or-unlock choice (CR 709.5c).
    let allow_lock = matches!(words, ["a", "door", "of", ..]);
    if let ["a", "locked", "door", "of", rest @ ..] | ["a", "door", "of", rest @ ..] = words
        && rest.iter().any(|word| *word == "target")
    {
        let skip = tokens
            .iter()
            .position(|token| token.is_word("of"))
            .map(|idx| idx + 1)
            .unwrap_or(tokens.len());
        let target = crate::util::parse_target_phrase(&tokens[skip..])?;
        return Ok(EffectAst::subject_verb(
            crate::cards::builders::SubjectVerbRoleAst::Actor,
            extract_subject_player(subject).unwrap_or(PlayerAst::You),
            crate::cards::builders::SubjectVerbActionAst::KeywordActions(
                crate::cards::builders::KeywordActionAst::UnlockTargetRoomDoor { target, allow_lock },
            ),
        ));
    }
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

fn parse_note(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if crate::lexer::parser_token_word_refs(tokens) == ["the", "type", "and", "amount", "of", "mana", "spent", "to", "pay", "this", "activation", "cost"] {
        return Ok(EffectAst::NoteActivationManaPayment);
    }

    if matches!(
        subject,
        None | Some(SubjectAst::This)
            | Some(SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit))
    ) && resource_grammar::parse_resource_note_life_total_shape(tokens)
    {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::NoteLifeTotal),
        ));
    }
    // "note one of its creature types that hasn't been noted for Volo's
    // Journal": the noted set lives on the ability's source, so the
    // trailing source reference is the ability's own permanent.
    let words = crate::lexer::token_word_refs(tokens);
    if let ["one", "of", "its", "creature", "types", "that", rest @ ..] = words.as_slice()
        && let Some(noted_at) = rest
            .windows(3)
            .position(|window| window == ["been", "noted", "for"])
        && rest[..noted_at]
            .iter()
            .all(|word| matches!(*word, "hasn't" | "hasnt" | "has" | "not" | "n't"))
        && noted_at + 3 < rest.len()
    {
        return Ok(EffectAst::NoteTriggeringCreatureType);
    }
    // "Note a creature type that hasn't been noted for this Saga" (Long
    // List of the Ents): any creature type the source hasn't noted yet; it
    // becomes the source's noted and chosen ("that") type.
    if let ["a", "creature", "type", "that", rest @ ..] = words.as_slice()
        && let Some(noted_at) = rest
            .windows(3)
            .position(|window| window == ["been", "noted", "for"])
        && noted_at > 0
        && rest[..noted_at]
            .iter()
            .all(|word| matches!(*word, "hasn't" | "hasnt" | "has" | "not" | "n't"))
        && noted_at + 3 < rest.len()
    {
        return Ok(EffectAst::subject_verb_note_unnoted_creature_type(PlayerAst::You));
    }
    // "note the number and kind of counters that were on that creature"
    // (Tawnos's Coffin): the referenced object's last-known counters, read
    // back by a later "with the noted number and kind of counters on it".
    if let [
        "the",
        "number",
        "and",
        "kind" | "kinds",
        "of",
        "counters",
        "that",
        "were" | "are",
        "on",
        object_words @ ..,
    ] = words.as_slice()
        && !object_words.is_empty()
    {
        let prefix_words = words.len() - object_words.len();
        let mut seen_words = 0usize;
        let mut object_start = None;
        for (index, token) in tokens.iter().enumerate() {
            if token.as_word().is_some() {
                if seen_words == prefix_words {
                    object_start = Some(index);
                    break;
                }
                seen_words += 1;
            }
        }
        if let Some(object_start) = object_start {
            let object = parse_target_phrase(&trim_commas(&tokens[object_start..]))?;
            return Ok(EffectAst::NoteCounters {
                object,
                tag: crate::util::noted_counters_tag(),
            });
        }
    }
    Err(CardTextError::ParseError(format!(
        "unsupported note clause: '{}' (subject: {subject:?})",
        crate::lexer::token_word_refs(tokens).join(" ")
    )))
}

fn parse_take(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        None => PlayerAst::You,
        Some(subject) => extract_subject_player(Some(subject)).ok_or_else(|| {
            CardTextError::ParseError("extra turn requires a player subject".to_string())
        })?,
    };
    if let Some(shape) = resource_grammar::parse_resource_take_extra_turn_shape(tokens, player) {
        return Ok(shape.into_effect());
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

    // "proliferate a number of times equal to the difference" (Expand the
    // Sphere): the repetition count is an ordinary value expression.
    if let ["a", "number", "of", "times", "equal", "to", ..] =
        crate::lexer::token_word_refs(tokens).as_slice()
    {
        let value_tokens = trim_commas(&tokens[6..]);
        if let Some((count, used)) = parse_value(&value_tokens)
            && used == value_tokens.len()
        {
            return Ok(EffectAst::subject_verb_proliferate(count));
        }
    }

    let (count, used) = if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word) {
        match first {
            "once" | "again" => (Value::Fixed(1), 1),
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

/// "at each|all <face-down permanent filter>": the face-down battlefield
/// objects a look instruction names without a possessive owner.
fn parse_look_at_each_face_down_permanent(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = crate::lexer::token_word_refs(&trimmed);
    if !matches!(words.as_slice(), ["at", "each" | "all", ..]) {
        return None;
    }
    let filter_tokens = &trimmed[2..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    if filter.face_down != Some(true)
        || !filter.tagged_constraints.is_empty()
        || !matches!(filter.zone, None | Some(Zone::Battlefield))
    {
        return None;
    }
    Some(filter.in_zone(Zone::Battlefield))
}

pub fn parse_look(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let subject_player = subject.and_then(|subject| match subject {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    });
    // "Then look at the exiled cards and put them on top of your library"
    // (Scroll Rack): looking at the cards this ability exiled.
    if matches!(
        crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(tokens)).as_slice(),
        ["at", "the", "exiled", "card" | "cards"]
    ) {
        // "the exiled cards" names the cards this ability exiled, not the
        // most recent object antecedent (the cards just put into hand).
        let mut exiled = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind())
            .in_zone(Zone::Exile);
        exiled.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Exiled));
        return Ok(EffectAst::subject_verb_look_at_objects(
            subject_player.unwrap_or(PlayerAst::You),
            exiled,
        ));
    }
    // "look at each face-down creature that's attacking or blocking"
    // (Revealing Wind): a one-shot private look at every face-down permanent
    // the filter selects, whoever controls it (CR 708.5 lets only the
    // controller look otherwise).
    if let Some(filter) = parse_look_at_each_face_down_permanent(tokens) {
        return Ok(EffectAst::subject_verb_look_at_objects(
            subject_player.unwrap_or(PlayerAst::You),
            filter,
        ));
    }
    // "Until end of turn, you may look at face-down creatures you don't
    // control any time." (Lumbering Laundry): a resolving grant of the
    // continuous look permission to this permanent's controller. A leading
    // duration scope replaces the grant's duration when it is lowered.
    {
        let trimmed = crate::util::trim_edge_punctuation_tokens(tokens);
        let words = crate::lexer::token_word_refs(trimmed);
        let (body_len, duration) = if words.ends_with(&["this", "turn"]) {
            (words.len() - 2, Until::EndOfTurn)
        } else {
            (words.len(), Until::Forever)
        };
        let body_end = crate::lexer::LexedClause::new(trimmed)
            .words()
            .map_word_or_end_to_token_boundary(body_len)
            .unwrap_or(trimmed.len());
        let mut line = crate::lexer::synthetic_word_tokens(&["you", "may", "look"]);
        line.extend_from_slice(&trimmed[..body_end]);
        // Under a leading "you may" the clause reaches here verb-first with
        // the implicit actor, which is the same controller.
        let body_words = &words[..body_len];
        let surface_matches = matches!(
            body_words,
            ["at", "face-down", "creatures", "you", "don't" | "dont", "control", "any", "time"]
        );
        if matches!(subject_player, None | Some(PlayerAst::You) | Some(PlayerAst::Implicit))
            && (surface_matches
                || crate::grammar::abilities::is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed(&line))
        {
            // The permission belongs to the player for the duration, not to
            // this permanent: it survives the source leaving the battlefield
            // or losing its abilities (CR 708.6, 611.2).
            let mut canonical = crate::lexer::synthetic_word_tokens(&["you", "may", "look"]);
            canonical.extend(crate::lexer::synthetic_word_tokens(&[
                "at", "face-down", "creatures", "you", "don't", "control", "any", "time",
            ]));
            let objects = crate::grammar::abilities::face_down_look_permission_objects_lexed(&line)
                .or_else(|| crate::grammar::abilities::face_down_look_permission_objects_lexed(&canonical))
                .expect("canonical face-down look permission");
            return Ok(EffectAst::subject_verb_cant(
                crate::effect::Restriction::object_rule(
                    objects,
                    ironsmith_core::ObjectRule::FaceDownVisibleTo(PlayerFilter::You),
                ),
                duration,
                None,
            ));
        }
    }
    let shape =
        resource_grammar::parse_resource_look_shape(tokens, subject_player).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported look clause (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            ))
        })?;
    match shape {
        ResourceLookShape::RandomHandCard { player } => {
            let owner = match player {
                PlayerAst::You => PlayerFilter::You,
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                PlayerAst::Opponent => PlayerFilter::Opponent,
                PlayerAst::That => PlayerFilter::IteratedPlayer,
                _ => {
                    return Err(CardTextError::ParseError(
                        "unsupported random hand owner".into(),
                    ));
                }
            };
            let tag = crate::util::helper_tag_for_tokens(tokens, "looked");
            Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                        filter: ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner),
                        count: ChoiceCount::exactly(1).at_random(),
                        count_value: None,
                        player,
                        tag: crate::tag::TagRef::of(tag.clone()),
                    }),
                    EffectAst::subject_verb_look_at_target(TargetAst::Tagged(
                        tag,
                        span_from_tokens(tokens),
                    )),
                ],
            })
        }
        ResourceLookShape::Tagged => Ok(EffectAst::subject_verb_look_at_target(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(tokens),
        ))),
        ResourceLookShape::PlayTaggedWhileExiled => Ok(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                crate::tag::CompilerReferenceTag::It.bind(),
                PlayerAst::You,
                true,
                false,
                false,
                None,
            ),
        ),
        ResourceLookShape::EachPlayerHand => {
            Ok(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_look_at_hand(TargetAst::Player(
                    PlayerFilter::IteratedPlayer,
                    None,
                ))],
            }))
        }
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
                PlayerAst::Defending => TargetAst::Player(PlayerFilter::Defending, None),
                // The controller of the object the instruction already
                // referenced ("Counter target spell. Look at its
                // controller's hand.").
                PlayerAst::ItsController => TargetAst::Player(
                    PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                    )),
                    None,
                ),
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
                            crate::tag::CompilerReferenceTag::It.bind(),
                        ),
                    ],
                }),
                ResourceLookHandFollowup::TopCardAndFaceDownCreatures => Ok(EffectAst::Sequence {
                    effects: vec![
                        hand,
                        EffectAst::subject_verb_look_at_top_cards(
                            PlayerAst::That,
                            Value::Fixed(1),
                            crate::tag::CompilerReferenceTag::It.bind(),
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
        // "Target player looks at the top three cards of their library": that
        // player, not the ability's controller, sees the cards.
        ResourceLookShape::TopCards { player, count }
            if subject_player.is_some_and(|subject| {
                subject == player && !matches!(subject, PlayerAst::You | PlayerAst::Implicit)
            }) =>
        {
            Ok(EffectAst::subject_verb_player_looks_at_own_top_cards(
                player,
                count,
                crate::tag::CompilerReferenceTag::It.bind(),
            ))
        }
        // "That player looks at the top three cards of your library": the
        // subject privately looks at another player's library.
        ResourceLookShape::TopCards { player, count }
            if player != PlayerAst::That
                && subject_player.is_some_and(|subject| {
                    subject != player && !matches!(subject, PlayerAst::You | PlayerAst::Implicit)
                }) =>
        {
            Ok(EffectAst::PlayerLooksAtTopCardsOfLibrary {
                viewer: subject_player.expect("guarded by the match arm"),
                library_owner: player,
                count,
                tag: crate::tag::CompilerReferenceTag::It.bind(),
            })
        }
        ResourceLookShape::TopCards { player, count } => {
            Ok(EffectAst::subject_verb_look_at_top_cards(
                player,
                count,
                crate::tag::CompilerReferenceTag::It.bind(),
            ))
        }
        ResourceLookShape::EachOpponentTopCards { count } => {
            Ok(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
                effects: vec![EffectAst::subject_verb_look_at_top_cards(
                    PlayerAst::That,
                    count,
                    crate::tag::CompilerReferenceTag::It.bind(),
                )],
            }))
        }
        ResourceLookShape::EachPlayerTopCards { count } => {
            Ok(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_look_at_top_cards(
                    PlayerAst::That,
                    count,
                    crate::tag::CompilerReferenceTag::It.bind(),
                )],
            }))
        }
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

    // "Reorder your graveyard at random" (Search for Survivors).
    if crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(&rest))
        == ["at", "random"]
    {
        return Ok(EffectAst::subject_verb_reorder_graveyard_at_random(owner.player));
    }

    if !resource_grammar::parse_resource_reorder_tail_shape(&rest) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause tail (clause: '{clause}')"
        )));
    }

    Ok(EffectAst::subject_verb_reorder_graveyard(owner.player))
}

/// "shuffles their hand, graveyard, and all permanents they own into their
/// library" (Sway of the Stars): one shuffle of every listed card set the
/// player owns into that player's library.
fn parse_shuffle_zone_list_into_library(
    tokens: &[OwnedLexToken],
    player: PlayerAst,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(into_idx) = tokens.iter().position(|token| token.is_word("into")) else {
        return Ok(None);
    };
    let destination = crate::lexer::token_word_refs(&tokens[into_idx + 1..]);
    if !matches!(
        destination.as_slice(),
        ["their" | "your", "library"] | ["his", "or", "her", "library"]
    ) {
        return Ok(None);
    }
    let target_tokens = &tokens[..into_idx];
    if !target_tokens.iter().any(|token| token.is_comma()) {
        return Ok(None);
    }
    let Some(owner) =
        crate::grammar::effects::zone_counter_shapes::player_filter_for_half_reference(player)
    else {
        return Ok(None);
    };
    let mut parts = Vec::new();
    for item in target_tokens.split(|token| token.is_comma()) {
        let words = crate::lexer::token_word_refs(item);
        let words = match words.as_slice() {
            ["and", rest @ ..] => rest.to_vec(),
            rest => rest.to_vec(),
        };
        let filter = match words.as_slice() {
            ["their" | "your", "hand"] | ["hand"] => {
                ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner.clone())
            }
            ["their" | "your", "graveyard"] | ["graveyard"] => {
                ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(owner.clone())
            }
            ["all", "permanents", "they" | "you", "own"] => ObjectFilter::default()
                .in_zone(Zone::Battlefield)
                .owned_by(owner.clone()),
            _ => return Ok(None),
        };
        parts.push(filter);
    }
    if parts.len() < 2 {
        return Ok(None);
    }
    let mut union = ObjectFilter::default();
    union.any_of = parts;
    Ok(Some(EffectAst::subject_verb_shuffle_all_objects_into_library(
        player,
        TargetAst::Object(union, None, None),
    )))
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
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        ));
    }

    if let Some(effect) = parse_shuffle_zone_list_into_library(tokens, player)? {
        return Ok(effect);
    }

    let shape =
        resource_grammar::parse_resource_shuffle_shape(tokens, player).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported shuffle clause (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            ))
        })?;
    match shape {
        ResourceShuffleShape::ObjectsIntoOwnersLibraries { target_len } => {
            let target_tokens = trim_commas(&tokens[..target_len]);
            // "Then they shuffle each nonland permanent they control into its
            // owner's library": an authored player subject performs the
            // shuffle; "its owner's" only names each destination library.
            let explicit_actor = !matches!(player, PlayerAst::Implicit | PlayerAst::You);
            // "each nonland permanent they control" (Teferi, Temporal
            // Pilgrim) names every matching object, not one chosen object.
            let quantified_all = target_tokens
                .first()
                .is_some_and(|token| token.is_word("each") || token.is_word("all"));
            let shuffle_into_owner_library = |target: TargetAst| {
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    if explicit_actor {
                        player
                    } else {
                        PlayerAst::ItsOwner
                    },
                    SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                        target,
                        all: quantified_all,
                        owner_library_destination: true,
                        possessive_owner_subject: !explicit_actor,
                        shuffle_subject_library: false,
                    }),
                )
            };
            // "this creature and target creature with a stun counter on it"
            // (Floodpits Drowner) names two independent operands; each is
            // shuffled into its own owner's library.
            if let Some(and_idx) = target_tokens.iter().position(|token| token.is_word("and"))
                && and_idx > 0
                && and_idx + 1 < target_tokens.len()
                && let Ok(first) = parse_target_phrase(&trim_commas(&target_tokens[..and_idx]))
                && let Ok(second) = parse_target_phrase(&trim_commas(&target_tokens[and_idx + 1..]))
            {
                return Ok(EffectAst::Sequence {
                    effects: vec![
                        shuffle_into_owner_library(first),
                        shuffle_into_owner_library(second),
                    ],
                });
            }
            let target = parse_target_phrase(&target_tokens)?;
            Ok(shuffle_into_owner_library(target))
        }
        ResourceShuffleShape::GraveyardIntoLibrary {
            player,
            explicit_all_cards_from,
        } => Ok(
            EffectAst::subject_verb_shuffle_graveyard_into_library_with_surface(
                player,
                explicit_all_cards_from,
            ),
        ),
        ResourceShuffleShape::ObjectsIntoSubjectLibrary {
            target_len,
            player,
            all,
        } => {
            // "Shuffle this card into your library from your graveyard"
            // (Kogla and Yidaro): the trailing origin restricts the object to
            // that zone, exactly as "this card from your graveyard" does; a
            // card a replacement put elsewhere isn't shuffled.
            let mut target = match graveyard_origin_tail(tokens, target_len) {
                Some(tail) => {
                    let mut with_origin = trim_commas(&tokens[..target_len]).to_vec();
                    with_origin.extend(tail.iter().cloned());
                    parse_target_phrase(&with_origin)?
                }
                None => parse_target_phrase(&trim_commas(&tokens[..target_len]))?,
            };
            super::zone_counter_helpers::apply_shuffle_subject_graveyard_owner_context(
                &mut target,
                SubjectAst::Player(player),
            );
            Ok(if all {
                EffectAst::subject_verb_shuffle_all_objects_into_library(player, target)
            } else {
                EffectAst::subject_verb_shuffle_objects_into_library(player, target)
            })
        }
        ResourceShuffleShape::HandIntoLibrary { player } => {
            let owner =
                crate::grammar::effects::zone_counter_shapes::player_filter_for_half_reference(
                    player,
                )
                .ok_or_else(|| {
                    CardTextError::ParseError("unsupported hand owner in shuffle".to_string())
                })?;
            // "shuffles their hand into their library" names every card in
            // that hand, not a target (CR 701.24a, 115.1).
            Ok(EffectAst::subject_verb_shuffle_all_objects_into_library(
                player,
                TargetAst::Object(
                    ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner),
                    None,
                    None,
                ),
            ))
        }
        ResourceShuffleShape::TaggedIntoLibrary {
            player: destination_player,
            to_bottom,
        } if !to_bottom
            && tokens
                .iter()
                .position(|token| token.is_word("into"))
                .and_then(|into| graveyard_origin_tail(tokens, into))
                .is_some() =>
        {
            // "shuffle it into your library from your graveyard" (Yidaro,
            // Wandering Monster): only the card still in that graveyard is
            // shuffled, exactly as "shuffle it from your graveyard into your
            // library"; a card a replacement put elsewhere stays there.
            let into = tokens
                .iter()
                .position(|token| token.is_word("into"))
                .expect("guarded above");
            let tail = graveyard_origin_tail(tokens, into).expect("guarded above");
            let mut with_origin = trim_commas(&tokens[..into]).to_vec();
            with_origin.extend(tail.iter().cloned());
            let mut target = parse_target_phrase(&with_origin)?;
            super::zone_counter_helpers::apply_shuffle_subject_graveyard_owner_context(
                &mut target,
                SubjectAst::Player(destination_player),
            );
            Ok(EffectAst::subject_verb_shuffle_objects_into_library(
                destination_player,
                target,
            ))
        }
        ResourceShuffleShape::TaggedIntoLibrary {
            player: destination_player,
            to_bottom,
        } => Ok(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        span_from_tokens(tokens),
                    ),
                    Zone::Library,
                    to_bottom,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                subject_verb_player_resource_effect(
                    SubjectVerbRoleAst::LibraryOwner,
                    destination_player,
                    SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
                ),
            ],
        })),
        ResourceShuffleShape::ShuffleLibrary {
            player: destination_player,
        } => Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            destination_player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        )),
        ResourceShuffleShape::SimpleLibrary => Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        )),
    }
}

/// The trailing "from <player's> graveyard" origin after a shuffle's
/// destination ("shuffle it into your library from your graveyard"), as the
/// tokens to append to the shuffled object's phrase.
fn graveyard_origin_tail(tokens: &[OwnedLexToken], target_len: usize) -> Option<&[OwnedLexToken]> {
    let rest = tokens.get(target_len..)?;
    let from = rest.iter().position(|token| token.is_word("from"))?;
    let tail = crate::util::trim_edge_punctuation_tokens(&rest[from..]);
    let words = crate::lexer::token_word_refs(tail);
    (words.len() <= 5
        && words
            .last()
            .is_some_and(|word| matches!(*word, "graveyard" | "graveyards")))
    .then_some(tail)
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
            crate::tag::CompilerReferenceTag::It.bind(),
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
                tag: (crate::tag::CompilerReferenceTag::ChosenName.bind()).into(),
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
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(&target_tokens),
        )));
    }

    Ok(EffectAst::subject_verb_detain(parse_target_phrase(
        &target_tokens,
    )?))
}

pub fn parse_suspect(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if resource_grammar::parse_suspect_triggering_group_choice_shape(&target_tokens) {
        // "you may suspect one of the other creatures" (Frantic Scapegoat,
        // after "Whenever one or more other creatures you control enter"):
        // choose exactly one member of the triggering entry batch, never the
        // source antecedent of the intervening-if. Suspecting reports zero
        // when the chosen creature is already suspected, so a following
        // "If you do" sees whether it actually became suspected.
        let group_tag = crate::util::helper_tag_for_tokens(&target_tokens, "suspect_group_choice");
        let mut filter = ObjectFilter::creature()
            .in_zone(crate::zone::Zone::Battlefield)
            .match_tagged(
                ironsmith_core::ZONE_CHANGE_GROUP_TAG,
                TaggedOpbjectRelation::IsTaggedObject,
            );
        // "the other creatures": never the source itself.
        filter.other = true;
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: crate::tag::TagRef::of(group_tag.clone()),
                }),
                EffectAst::subject_verb_suspect(TargetAst::Tagged(
                    crate::tag::TagRef::of(group_tag),
                    span_from_tokens(&target_tokens),
                )),
            ],
        });
    }
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing suspect target".to_string(),
        ));
    }

    if resource_grammar::parse_resource_tagged_reference_shape(&target_tokens) {
        return Ok(EffectAst::subject_verb_suspect(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
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
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    zone: Zone::Hand,
                    ..
                }),
                ..
            })
        ));
    }
}


/// "Target player loses all poison counters" (Leeches), "each opponent loses
/// all counters" (Final Act): every counter of the named kind (or of every
/// kind) is removed from the subject player (CR 122.1, 122.8). The subject
/// becomes the removal's non-target holder, or the declared target player.
fn parse_lose_all_player_counters(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let (Some(first), Some(last)) = (tokens.first(), tokens.last()) else {
        return Ok(None);
    };
    if !first.is_word("all") || !last.is_word("counters") {
        return Ok(None);
    }
    let descriptor = &tokens[1..tokens.len() - 1];
    let counter_type = if descriptor.is_empty() {
        None
    } else {
        match super::zone_handlers::parse_counter_type_from_descriptor_tokens(descriptor) {
            Some(counter_type) => Some(counter_type),
            None => return Ok(None),
        }
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let (target, holder) = match player {
        PlayerAst::Target => (
            parse_target_phrase(&crate::lexer::synthetic_word_tokens(&["target", "player"]))?,
            PlayerFilter::target_player(),
        ),
        PlayerAst::TargetOpponent => (
            parse_target_phrase(&crate::lexer::synthetic_word_tokens(&["target", "opponent"]))?,
            PlayerFilter::target_opponent(),
        ),
        PlayerAst::You => (TargetAst::Player(PlayerFilter::You, None), PlayerFilter::You),
        // The participant of "each opponent loses all counters"; outside an
        // iteration the unbound player reference fails closed in lowering.
        PlayerAst::That | PlayerAst::Implicit => (
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            PlayerFilter::IteratedPlayer,
        ),
        _ => return Ok(None),
    };
    let amount = match counter_type {
        Some(counter_type) => Value::PlayerCounters(holder, counter_type),
        None => Value::CountersOn(Box::new(ChooseSpec::EachPlayer(holder)), None),
    };
    Ok(Some(EffectAst::subject_verb_remove_up_to_any_counters(
        amount,
        target,
        counter_type,
        false,
    )))
}

/// "its combat damage [this turn] as though it weren't blocked" after an
/// optional "have <creature> assign": the duration of the resolving grant.
fn may_assign_as_unblocked_body(tokens: &[crate::lexer::OwnedLexToken]) -> Option<Until> {
    let mut words = crate::lexer::token_word_refs(tokens);
    let mut duration = Until::Forever;
    if let Some(index) = words.windows(2).position(|pair| pair == ["this", "turn"]) {
        words.drain(index..index + 2);
        duration = Until::EndOfTurn;
    }
    matches!(
        words.as_slice(),
        [
            "its" | "their",
            "combat",
            "damage",
            "as",
            "though",
            "it" | "they",
            "weren't" | "werent" | "wasn't" | "wasnt",
            "blocked",
        ]
    )
    .then_some(duration)
}

/// A keyword action lowered onto ordinary control, counter, type and entry operations.
fn parse_assimilate(tokens: &[OwnedLexToken], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    if extract_subject_player(subject).is_some_and(|player| !matches!(player, PlayerAst::You | PlayerAst::Implicit)) {
        return Err(CardTextError::ParseError("unsupported explicit assimilation actor".into()));
    }
    let target = parse_target_phrase(tokens)?;
    let is_card = crate::lexer::token_word_refs(tokens).iter().any(|word| matches!(*word, "card" | "cards"));
    if is_card {
        let mut movement = EffectAst::subject_verb_move_to_zone(target, Zone::Battlefield, false, ReturnControllerAst::You, false, None);
        if let EffectAst::SubjectVerb(effect) = &mut movement
            && let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { entry_type_changes, entry_counters, .. }) = &mut effect.action
        {
            *entry_type_changes = Some(ironsmith_core::effect::BattlefieldEntryTypeChanges {
                color_changes: Vec::new(),
                added_card_types: vec![CardType::Artifact, CardType::Creature],
                removed_subtype_families: vec![crate::types::SubtypeFamily::Creature],
                added_subtypes: vec![Subtype::Borg],
            });
            entry_counters.push(ironsmith_core::BattlefieldEntryCounterSpec::new(
                crate::object::CounterType::PlusOnePlusOne, 1, ironsmith_core::BattlefieldEntryCounterSurface::Inline));
        }
        return Ok(movement);
    }
    let tag = crate::util::helper_tag_for_tokens(tokens, "assimilated_permanent");
    let referenced = TargetAst::Tagged(tag.clone().into(), None);
    Ok(EffectAst::Coordinated {
        effects: vec![
            EffectAst::TagReferenced { effect: Box::new(EffectAst::subject_verb_gain_control(PlayerAst::You, target, Until::Forever)), tag: tag.into() },
            EffectAst::subject_verb_put_counters(crate::object::CounterType::PlusOnePlusOne, Value::Fixed(1), referenced.clone(), None, false),
            EffectAst::subject_verb_add_card_types(referenced.clone(), vec![CardType::Artifact, CardType::Creature], Until::Forever),
            EffectAst::subject_verb_set_creature_subtypes(referenced, vec![Subtype::Borg], Until::Forever),
        ],
        leading_duration: false,
        result_conjunction: false,
    })
}
