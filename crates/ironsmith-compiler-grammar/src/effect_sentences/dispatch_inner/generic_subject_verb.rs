#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericPermissionVerb {
    PlayAndCast,
}

fn parse_source_exiled_owner_library_bottom_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let shape =
        effect_grammar::control_copy_attach_shapes::parse_source_exiled_owner_library_bottom_shape(
            tokens,
        )?;
    let source_words = crate::lexer::token_word_refs(shape.source_tokens);
    let source_surface = crate::util::source_reference_surface_for_words(&source_words)
        .or_else(|| crate::util::this_source_surface_for_words(&source_words))?;
    let target = TargetAst::Object(
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
            .in_zone(Zone::Exile),
        None,
        None,
    );
    Some(
        EffectAst::subject_verb_move_all_to_zone(
            target,
            Zone::Library,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_exiled_with_source_surface(Some(
            ironsmith_core::ExiledWithSourceMoveSurface {
                verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
                subject: ironsmith_core::ExiledWithSourceSubjectSurface::OwnerOfEachCard,
                source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(source_surface),
                destination: ironsmith_core::ExiledWithSourceDestinationSurface::TheirOwner,
            },
        )),
    )
}

fn parse_triggering_object_had_counters_create_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let wrap_condition = crate::word_primitives::parse_sequence_prefix(
        &words,
        &["if", "it", "had", "counters", "on", "it", "create"],
    );
    if !wrap_condition && words.first().is_none_or(|word| *word != "create") {
        return Ok(None);
    }
    let where_words = [
        "where", "x", "is", "the", "number", "of", "counters", "it", "had", "on", "it",
    ];
    let Some(where_word_index) = crate::word_primitives::parse_sequence_start(&words, &where_words)
    else {
        return Ok(None);
    };
    if where_word_index == 0 || where_word_index + where_words.len() != words.len() {
        return Ok(None);
    }

    let create_start =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("create"))
            .ok_or_else(|| {
                CardTextError::ParseError("missing token-creation clause".to_string())
            })?;
    let where_start =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("where"))
            .ok_or_else(|| {
                CardTextError::ParseError("missing counter-count definition".to_string())
            })?;
    let mut create_effects = parse_effect_chain_lexed(&tokens[create_start..where_start])?;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. }),
            ..
        }),
    ] = create_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    let triggering = ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::Triggering.bind()).into());
    *count = Value::CountersOn(Box::new(triggering.clone()), None).with_surface_hints([
        ironsmith_core::ValueSurfaceHint::WhereXIs,
        ironsmith_core::ValueSurfaceHint::TriggeringObjectCountersItHad,
    ]);

    if wrap_condition {
        Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: Value::CountersOn(Box::new(triggering), None),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            },
            if_true: create_effects,
            if_false: Vec::new(),
        })))
    } else {
        Ok(create_effects.pop())
    }
}

fn parse_effect_chain_preserving_source_exiled_owner_library_bottom(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effect) = parse_source_exiled_owner_library_bottom_subject_verb(tokens) {
        Ok(vec![effect])
    } else {
        parse_effect_chain_lexed(tokens)
    }
}

fn parse_source_exiled_counted_return_remainder_to_owners_libraries(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let split = tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.is_word("and")
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("put"))
            && tokens.get(idx + 2).is_some_and(|next| next.is_word("the"))
            && tokens.get(idx + 3).is_some_and(|next| next.is_word("rest")))
        .then_some(idx)
    })?;
    let suffix_words = crate::lexer::token_word_refs(&tokens[split + 1..]);
    if suffix_words.len() != 10
        || !crate::word_primitives::parse_sequence_prefix(
            &suffix_words,
            &["put", "the", "rest", "on", "the", "bottom", "of", "their"],
        )
        || !matches!(suffix_words[8], "owner" | "owners" | "owner's" | "owners'")
        || !matches!(suffix_words[9], "library" | "libraries")
    {
        return None;
    }
    let prefix = trim_edge_punctuation(&tokens[..split]);
    let return_tokens = prefix
        .first()
        .is_some_and(|token| token.is_word("return"))
        .then_some(&prefix[1..])?;
    let return_effect =
        crate::grammar::primitives::probe_shape(super::zone_handlers::parse_return(return_tokens))?;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target: TargetAst::WithCount(inner, count),
                zone: Zone::Battlefield,
                battlefield_controller: crate::cards::builders::ReturnControllerAst::Owner,
                exiled_with_source_surface: Some(surface),
                all: false,
                ..
            }),
        ..
    }) = &return_effect
    else {
        return None;
    };
    let TargetAst::Object(filter, _, _) = inner.as_ref() else {
        return None;
    };
    if count.min == 0
        || count.max != Some(count.min)
        || count.dynamic_x
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
        })
        || filter.zone != Some(Zone::Exile)
        || !matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(_)
        )
    {
        return None;
    }
    let remainder = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
            tag: crate::tag::CompilerReferenceTag::SourceExiled.bind(),
            keep_tagged: crate::tag::CompilerReferenceTag::ReturnedSourceExiled.bind(),
            zone: Zone::Library,
            surface: ironsmith_core::LibraryRemainderSurface::Rest,
        }),
    );
    Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(return_effect),
            tag: crate::tag::CompilerReferenceTag::ReturnedSourceExiled.bind(),
        },
        remainder,
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericPermissionDuration {
    UntilEndOfTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GenericPermissionProgram {
    player: PlayerAst,
    verb: GenericPermissionVerb,
    from_zone: Zone,
    duration: GenericPermissionDuration,
}

impl GenericPermissionProgram {
    fn lower(self) -> EffectAst {
        match (self.player, self.verb, self.from_zone, self.duration) {
            (
                PlayerAst::You,
                GenericPermissionVerb::PlayAndCast,
                Zone::Graveyard,
                GenericPermissionDuration::UntilEndOfTurn,
            ) => EffectAst::subject_verb_play_from_graveyard_until_eot(PlayerAst::You),
            _ => unreachable!("unrecognized generic permission program"),
        }
    }
}

#[derive(Debug, Clone)]
struct GenericZoneReplacementProgram {
    player: PlayerAst,
    from_zone: Zone,
    replacement_zone: Zone,
    duration: Until,
}

impl GenericZoneReplacementProgram {
    fn lower(self) -> EffectAst {
        match (
            self.player,
            self.from_zone,
            self.replacement_zone,
            self.duration,
        ) {
            (PlayerAst::You, Zone::Graveyard, Zone::Exile, Until::EndOfTurn) => {
                EffectAst::subject_verb_exile_instead_of_graveyard_this_turn(PlayerAst::You)
            }
            _ => unreachable!("unrecognized generic zone replacement program"),
        }
    }
}

#[derive(Debug, Clone)]
struct GenericChoiceComplementProgram {
    chooser_scope: PlayerAst,
    base_filter: ObjectFilter,
    keep_tag: TagKey,
    keep_filters: Vec<ObjectFilter>,
    keep_count: ChoiceCount,
    distinct_slots: bool,
    aggregate_constraint: Option<crate::effect::ChoiceAggregateConstraint>,
}

impl GenericChoiceComplementProgram {
    fn lower(self) -> EffectAst {
        let mut effects = Vec::new();
        if let Some(constraint) = self.aggregate_constraint {
            // The one choice happens before anything is kept; excluding the
            // kept would only read as "other".
            effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
                filter: self.base_filter.clone(),
                count: self.keep_count,
                player: PlayerAst::That,
                tag: crate::tag::TagRef::of(self.keep_tag.clone()),
                constraint,
            }));
        } else {
            // Each later slot of a party complement chooses among what earlier
            // slots did not keep. A single slot chooses before anything is
            // kept, so the exclusion would only read as "other".
            let sequential_slots = self.distinct_slots;
            for keep_filter in self.keep_filters {
                let mut filter = merge_filters(&self.base_filter, &keep_filter);
                if sequential_slots {
                    filter = filter.not_tagged(self.keep_tag.clone());
                }
                effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: self.keep_count,
                    count_value: None,
                    player: PlayerAst::That,
                    tag: crate::tag::TagRef::of(self.keep_tag.clone()),
                }));
            }
        }
        effects.push(EffectAst::subject_verb_sacrifice_all(
            PlayerAst::That,
            self.base_filter.not_tagged(self.keep_tag),
        ));
        match self.chooser_scope {
            PlayerAst::Opponent => EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }),
            PlayerAst::Any | PlayerAst::Implicit => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }),
            _ => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }),
        }
    }
}

#[derive(Debug, Clone)]
struct TargetControlledPumpProgram {
    filter: ObjectFilter,
    power: Value,
    toughness: Value,
    abilities: Vec<GrantedAbilityAst>,
    add_all_creature_types: bool,
    remove_all_creature_types: bool,
}

impl TargetControlledPumpProgram {
    fn lower(self) -> Vec<EffectAst> {
        let mut effects = vec![EffectAst::subject_verb_pump_all(
            self.filter.clone(),
            self.power,
            self.toughness,
            Until::EndOfTurn,
        )];
        if !self.abilities.is_empty() {
            effects.push(EffectAst::subject_verb_grant_abilities_all(
                self.filter.clone(),
                self.abilities,
                Until::EndOfTurn,
            ));
        }
        if self.add_all_creature_types {
            effects.push(EffectAst::subject_verb_add_all_subtypes_of_family(
                TargetAst::Object(self.filter.clone(), None, None),
                crate::types::SubtypeFamily::Creature,
                Until::EndOfTurn,
            ));
        }
        if self.remove_all_creature_types {
            effects.push(EffectAst::subject_verb_remove_all_subtypes_of_family(
                TargetAst::Object(self.filter, None, None),
                crate::types::SubtypeFamily::Creature,
                Until::EndOfTurn,
            ));
        }
        effects
    }
}

#[derive(Debug, Clone)]
enum GenericVoteProgram {
    Start {
        options: Vec<String>,
        secret: bool,
        starting_with_controller: bool,
    },
    OptionEffects {
        option: String,
        effects: Vec<EffectAst>,
    },
    Extra {
        count: u32,
        optional: bool,
    },
}

impl GenericVoteProgram {
    fn lower(self) -> EffectAst {
        match self {
            Self::Start {
                options,
                secret,
                starting_with_controller,
            } => EffectAst::Votes(VoteEffectAst::VoteStart {
                options,
                secret,
                starting_with_controller,
            }),
            Self::OptionEffects { option, effects } => EffectAst::Votes(VoteEffectAst::VoteOption { option, effects }),
            Self::Extra { count, optional } => EffectAst::Votes(VoteEffectAst::VoteExtra { count, optional }),
        }
    }
}

#[derive(Debug, Clone)]
enum GenericTopLevelProgram {
    Meld { effect: EffectAst },
    ControlCombatChoices { effect: EffectAst },
    PreventDamageAndPutCounters { effect: EffectAst },
    ConsultRevealUntil { effects: Vec<EffectAst> },
    LookedCardsCountedRemainder { effects: Vec<EffectAst> },
    ConsultRevealUntilHand { effects: Vec<EffectAst> },
    ConsultRevealUntilGraveyard { effects: Vec<EffectAst> },
    ConsultRevealUntilBattlefieldBottom { effects: Vec<EffectAst> },
    EachPlayerExileTopCast { effects: Vec<EffectAst> },
    Cant { effects: Vec<EffectAst> },
    ValueBinding { effects: Vec<EffectAst> },
}

impl GenericTopLevelProgram {
    fn route(&self) -> &'static str {
        match self {
            Self::Meld { .. } => "subject-verb verb=Meld subject=explicit recognizer=meld-result",
            Self::ControlCombatChoices { .. } => {
                "subject-verb verb=Choose subject=explicit recognizer=combat-choice-control"
            }
            Self::PreventDamageAndPutCounters { .. } => {
                "subject-verb verb=Prevent subject=implicit recognizer=damage-replacement-counters"
            }
            Self::ConsultRevealUntil { .. } => {
                "subject-verb verb=Reveal subject=explicit recognizer=consult-reveal-until"
            }
            Self::LookedCardsCountedRemainder { .. } => {
                "subject-verb verb=Look subject=explicit recognizer=counted-looked-cards-remainder"
            }
            Self::ConsultRevealUntilHand { .. } => {
                "subject-verb verb=Reveal subject=explicit recognizer=consult-reveal-until-hand"
            }
            Self::ConsultRevealUntilGraveyard { .. } => {
                "subject-verb verb=Reveal subject=explicit recognizer=consult-reveal-until-graveyard"
            }
            Self::ConsultRevealUntilBattlefieldBottom { .. } => {
                "subject-verb verb=Reveal subject=explicit recognizer=consult-reveal-until-battlefield-bottom"
            }
            Self::EachPlayerExileTopCast { .. } => {
                "subject-verb verb=Exile subject=explicit recognizer=each-player-exile-top-cast"
            }
            Self::Cant { .. } => "subject-verb verb=Cant subject=explicit recognizer=restriction",
            Self::ValueBinding { .. } => {
                "subject-verb verb=Bind subject=implicit recognizer=value-binding"
            }
        }
    }

    fn lower(self) -> Vec<EffectAst> {
        match self {
            Self::Meld { effect }
            | Self::ControlCombatChoices { effect }
            | Self::PreventDamageAndPutCounters { effect } => vec![effect],
            Self::ConsultRevealUntil { effects }
            | Self::LookedCardsCountedRemainder { effects }
            | Self::ConsultRevealUntilHand { effects }
            | Self::ConsultRevealUntilGraveyard { effects }
            | Self::ConsultRevealUntilBattlefieldBottom { effects }
            | Self::EachPlayerExileTopCast { effects }
            | Self::Cant { effects }
            | Self::ValueBinding { effects } => effects,
        }
    }
}

const CONSULT_REVEAL_UNTIL_HAND_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::capture(
            "consult_clause",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["then"]),
        ),
        effect_grammar::EffectSequence::word("then"),
        effect_grammar::EffectSequence::tail("followup", effect_grammar::EffectCaptureKind::Rest),
    ]);
const ALL_REVEALED_INTO_HAND_PHRASES: &[&[&str]] = &[
    &[
        "put", "all", "cards", "revealed", "this", "way", "into", "your", "hand",
    ],
    &["put", "all", "revealed", "cards", "into", "your", "hand"],
];
const ALL_REVEALED_INTO_HAND_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::object(
        "revealed_cards_destination",
        effect_grammar::EffectCaptureKind::OneOfPhrase(ALL_REVEALED_INTO_HAND_PHRASES),
    )]);
const ALL_REVEALED_INTO_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &[
        "put",
        "all",
        "cards",
        "revealed",
        "this",
        "way",
        "into",
        "their",
        "graveyard",
    ],
    &[
        "puts",
        "all",
        "cards",
        "revealed",
        "this",
        "way",
        "into",
        "their",
        "graveyard",
    ],
    &["put", "those", "cards", "into", "their", "graveyard"],
    &["puts", "those", "cards", "into", "their", "graveyard"],
    &[
        "put",
        "those",
        "cards",
        "into",
        "that",
        "player's",
        "graveyard",
    ],
    &[
        "puts",
        "those",
        "cards",
        "into",
        "that",
        "player's",
        "graveyard",
    ],
];
const ALL_REVEALED_INTO_GRAVEYARD_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::object(
        "revealed_cards_destination",
        effect_grammar::EffectCaptureKind::OneOfPhrase(ALL_REVEALED_INTO_GRAVEYARD_PHRASES),
    )]);
const MATCH_ONTO_BATTLEFIELD_PREFIX_PHRASES: &[&[&str]] = &[
    &["put", "it", "onto", "the", "battlefield"],
    &["put", "that", "card", "onto", "the", "battlefield"],
    &[
        "put",
        "those",
        "land",
        "cards",
        "onto",
        "the",
        "battlefield",
    ],
    &["put", "those", "lands", "onto", "the", "battlefield"],
];
const MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::object(
            "battlefield_destination",
            effect_grammar::EffectCaptureKind::OneOfPhrase(MATCH_ONTO_BATTLEFIELD_PREFIX_PHRASES),
        ),
        effect_grammar::EffectSequence::tail("remainder", effect_grammar::EffectCaptureKind::Rest),
    ]);
const CONSULT_REVEAL_UNTIL_BATTLEFIELD_BOTTOM_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::capture(
            "consult_clause",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(
                MATCH_ONTO_BATTLEFIELD_PREFIX_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::tail("followup", effect_grammar::EffectCaptureKind::Rest),
    ]);
const REST_BOTTOM_LIBRARY_ORDER_PHRASES: &[&[&str]] = &[&["random", "order"], &["any", "order"]];
const REST_BOTTOM_LIBRARY_WITH_ORDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("rest"),
        effect_grammar::EffectSequence::capture(
            "before_bottom",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["bottom"]),
        ),
        effect_grammar::EffectSequence::word("bottom"),
        effect_grammar::EffectSequence::capture(
            "before_library",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["library"]),
        ),
        effect_grammar::EffectSequence::word("library"),
        effect_grammar::EffectSequence::capture(
            "before_order",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(REST_BOTTOM_LIBRARY_ORDER_PHRASES),
        ),
        effect_grammar::EffectSequence::amount(
            "order",
            effect_grammar::EffectCaptureKind::OneOfPhrase(REST_BOTTOM_LIBRARY_ORDER_PHRASES),
        ),
    ]);
const REST_BOTTOM_LIBRARY_RANDOM_ORDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "random", "order",
    ])]);
const REST_BOTTOM_LIBRARY_ANY_ORDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "any", "order",
    ])]);
const EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PHRASES: &[&[&str]] = &[
    &["exile", "the", "top", "card", "of", "each"],
    &["exile", "top", "card", "of", "each"],
];
const EACH_PLAYER_EXILE_TOP_CARD_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::action(
            "exile_top_action",
            effect_grammar::EffectCaptureKind::OneOfPhrase(
                EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::tail(
            "library_clause",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const EACH_PLAYER_EXILE_UNTIL_NONLAND_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "each", "player", "exiles", "cards", "from", "the", "top", "of", "their", "library",
        "until", "they", "exile", "a", "nonland", "card",
    ])]);
const PLAYER_LIBRARY_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::any_word(&["player", "players"]),
        effect_grammar::EffectSequence::capture(
            "owner_library_gap",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["library"]),
        ),
        effect_grammar::EffectSequence::word("library"),
    ]);
const WITHOUT_PAYING_THEIR_MANA_COSTS_PHRASE: &[&str] =
    &["without", "paying", "their", "mana", "costs"];
const CAST_ANY_NUMBER_FREE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::phrase(&[
            "you", "may", "cast", "any", "number", "of", "spells",
        ]),
        effect_grammar::EffectSequence::object(
            "cast_scope",
            effect_grammar::EffectCaptureKind::UntilPhrase(WITHOUT_PAYING_THEIR_MANA_COSTS_PHRASE),
        ),
        effect_grammar::EffectSequence::phrase(WITHOUT_PAYING_THEIR_MANA_COSTS_PHRASE),
    ]);
const FROM_THOSE_OR_THEM_SCOPE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("among"),
        effect_grammar::EffectSequence::capture(
            "chosen_cards",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["those"], &["them"]]),
        ),
        effect_grammar::EffectSequence::any_word(&["those", "them"]),
    ]);
const FROM_NONLAND_EXILED_THIS_WAY_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "from", "among", "the", "nonland", "cards", "exiled", "this", "way",
    ])]);
const EACH_PLAYER_EXILE_TOP_CAST_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::capture(
            "exile_clause",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["then"]),
        ),
        effect_grammar::EffectSequence::word("then"),
        effect_grammar::EffectSequence::tail(
            "cast_clause",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const MELD_RESULT_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::phrase(&["exile", "them"]),
        effect_grammar::EffectSequence::phrase(&["then", "meld", "them", "into"]),
        effect_grammar::EffectSequence::object(
            "result",
            effect_grammar::EffectCaptureKind::OneOrMoreWords,
        ),
    ]);
const CONTROL_COMBAT_CHOICE_OBJECT_PHRASES: &[&[&str]] = &[&["creatures"]];
const CONTROL_COMBAT_CHOICES_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "chooser",
            effect_grammar::EffectCaptureKind::OneOf(&["you"]),
        ),
        effect_grammar::EffectSequence::phrase(&["choose", "which"]),
        effect_grammar::EffectSequence::object(
            "objects",
            effect_grammar::EffectCaptureKind::OneOfPhrase(CONTROL_COMBAT_CHOICE_OBJECT_PHRASES),
        ),
        effect_grammar::EffectSequence::action(
            "combat_action",
            effect_grammar::EffectCaptureKind::OneOf(&["attack", "block"]),
        ),
        effect_grammar::EffectSequence::tail(
            "choice_scope",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const CONTROL_COMBAT_HOW_THOSE_BLOCK_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "you",
        "choose",
        "how",
        "those",
        "creatures",
        "block",
    ])]);
const CONTROL_COMBAT_ATTACK_ACTION_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::action(
        "combat_action",
        effect_grammar::EffectCaptureKind::OneOf(&["attack"]),
    )]);
const CONTROL_COMBAT_BLOCK_ACTION_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::action(
        "combat_action",
        effect_grammar::EffectCaptureKind::OneOf(&["block"]),
    )]);
const CONTROL_COMBAT_SCOPE_PHRASES: &[&[&str]] = &[&["this", "turn"], &["this", "combat"]];
const CONTROL_COMBAT_ATTACK_SCOPE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::tail(
        "choice_scope",
        effect_grammar::EffectCaptureKind::OneOfPhrase(CONTROL_COMBAT_SCOPE_PHRASES),
    )]);
const CONTROL_COMBAT_BLOCK_SCOPE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::tail(
            "choice_scope",
            effect_grammar::EffectCaptureKind::OneOfPhrase(CONTROL_COMBAT_SCOPE_PHRASES),
        ),
        effect_grammar::EffectSequence::phrase(&["and", "how", "those", "creatures", "block"]),
    ]);
const DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES: &[&[&str]] = &[
    &["with", "lesser", "mana", "value"],
    &["with", "mana", "value", "equal"],
];
const DEFERRED_MANA_VALUE_CLAUSE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::capture(
            "effect",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(
                DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::any_phrase(DEFERRED_MANA_VALUE_CONSTRAINT_PHRASES),
        effect_grammar::EffectSequence::tail(
            "constraint_tail",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const PLAY_PERMISSION_DURATION_PHRASES: &[&[&str]] =
    &[&["until", "end", "of", "turn"], &["this", "turn"]];
const PLAY_PERMISSION_GRAVEYARD_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::modifier(
            "duration",
            effect_grammar::EffectCaptureKind::OneOfPhrase(PLAY_PERMISSION_DURATION_PHRASES),
        ),
        effect_grammar::EffectSequence::tail("permission", effect_grammar::EffectCaptureKind::Rest),
    ]);
const PLAY_LANDS_CAST_SPELLS_GRAVEYARD_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "spells",
        "from",
        "your",
        "graveyard",
    ])]);
const EXILE_THAT_CARD_INSTEAD_PHRASE: &[&str] = &["exile", "that", "card", "instead"];
const ZONE_REPLACEMENT_GRAVEYARD_EXILE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::condition(
            "condition",
            effect_grammar::EffectCaptureKind::UntilPhrase(EXILE_THAT_CARD_INSTEAD_PHRASE),
        ),
        effect_grammar::EffectSequence::tail(
            "replacement",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const FUTURE_GRAVEYARD_EXILE_CONDITION_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("if"),
        effect_grammar::EffectSequence::condition(
            "object",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["would", "be", "put"]),
        ),
        effect_grammar::EffectSequence::phrase(&["would", "be", "put"]),
        effect_grammar::EffectSequence::capture(
            "destination",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["this", "turn"]),
        ),
        effect_grammar::EffectSequence::phrase(&["this", "turn"]),
    ]);
const FUTURE_GRAVEYARD_DESTINATION_PHRASES: &[&[&str]] = &[
    &["into", "your", "graveyard"],
    &["into", "your", "graveyard", "from", "anywhere"],
];
const FUTURE_GRAVEYARD_DESTINATION_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::any_phrase(
        FUTURE_GRAVEYARD_DESTINATION_PHRASES,
    )]);
const EXILE_THAT_CARD_INSTEAD_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(
        EXILE_THAT_CARD_INSTEAD_PHRASE,
    )]);
const EACH_PLAYER_PHRASES: &[&[&str]] = &[&["each", "player"], &["each", "opponent"]];
const CHOICE_COMPLEMENT_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "chooser",
            effect_grammar::EffectCaptureKind::OneOfPhrase(EACH_PLAYER_PHRASES),
        ),
        effect_grammar::EffectSequence::action(
            "choose",
            effect_grammar::EffectCaptureKind::OneOf(&["choose", "chooses"]),
        ),
        effect_grammar::EffectSequence::object(
            "choice_clause",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["then"]),
        ),
        effect_grammar::EffectSequence::word("then"),
        effect_grammar::EffectSequence::action(
            "sacrifice",
            effect_grammar::EffectCaptureKind::OneOf(&["sacrifice", "sacrifices"]),
        ),
        effect_grammar::EffectSequence::phrase(&["the", "rest"]),
    ]);
const CHOICE_COMPLEMENT_LIST_FROM_AMONG_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::object(
            "choice_list",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["from", "among"]),
        ),
        effect_grammar::EffectSequence::phrase(&["from", "among"]),
        effect_grammar::EffectSequence::tail(
            "base_filter",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const WHERE_X_IS_PHRASE: &[&str] = &["where", "x", "is"];
const WHERE_X_VALUE_BINDING_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::condition(
            "effect",
            effect_grammar::EffectCaptureKind::UntilPhrase(WHERE_X_IS_PHRASE),
        ),
        effect_grammar::EffectSequence::phrase(WHERE_X_IS_PHRASE),
        effect_grammar::EffectSequence::tail("definition", effect_grammar::EffectCaptureKind::Rest),
    ]);
const SOURCE_GETS_SUBJECT_PHRASES: &[&[&str]] =
    &[&["this", "creature"], &["this", "permanent"], &["this"]];
const SOURCE_GETS_SUBJECT_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::subject(
        "source",
        effect_grammar::EffectCaptureKind::OneOfPhrase(SOURCE_GETS_SUBJECT_PHRASES),
    )]);
const ABILITY_HASTE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::word("haste")]);
const ABILITY_TRAMPLE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::word("trample")]);
const ABILITY_FIRST_STRIKE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "first", "strike",
    ])]);
const SOURCE_GETS_UNBLOCKABLE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "subject",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["get"], &["gets"]]),
        ),
        effect_grammar::EffectSequence::action(
            "pump_action",
            effect_grammar::EffectCaptureKind::OneOf(&["get", "gets"]),
        ),
        effect_grammar::EffectSequence::modifier(
            "modifier",
            effect_grammar::EffectCaptureKind::WordCount(1),
        ),
        effect_grammar::EffectSequence::tail("tail", effect_grammar::EffectCaptureKind::Rest),
    ]);
const SOURCE_GETS_FILTER_GAINS_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "subject",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["get"], &["gets"]]),
        ),
        effect_grammar::EffectSequence::action(
            "pump_action",
            effect_grammar::EffectCaptureKind::OneOf(&["get", "gets"]),
        ),
        effect_grammar::EffectSequence::modifier(
            "modifier",
            effect_grammar::EffectCaptureKind::WordCount(1),
        ),
        effect_grammar::EffectSequence::word("and"),
        effect_grammar::EffectSequence::object(
            "granted_filter",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[
                &["gain"],
                &["gains"],
                &["have"],
                &["has"],
            ]),
        ),
        effect_grammar::EffectSequence::action(
            "grant_action",
            effect_grammar::EffectCaptureKind::OneOf(&["gain", "gains", "have", "has"]),
        ),
        effect_grammar::EffectSequence::tail("ability", effect_grammar::EffectCaptureKind::Rest),
    ]);
const TARGET_HAS_BASE_PT_THEN_LOSES_PHRASES: &[&[&str]] = &[&["and", "lose"], &["and", "loses"]];
const TARGET_HAS_BASE_PT_THEN_LOSES_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "subject",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["has"], &["have"]]),
        ),
        effect_grammar::EffectSequence::action(
            "has_action",
            effect_grammar::EffectCaptureKind::OneOf(&["has", "have"]),
        ),
        effect_grammar::EffectSequence::capture(
            "base_pt_clause",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(
                TARGET_HAS_BASE_PT_THEN_LOSES_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::any_phrase(TARGET_HAS_BASE_PT_THEN_LOSES_PHRASES),
        effect_grammar::EffectSequence::tail(
            "ability_tail",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const TARGET_GETS_THEN_GAINS_GRANT_PHRASES: &[&[&str]] = &[
    &["and", "gain"],
    &["and", "gains"],
    &["and", "have"],
    &["and", "has"],
    &["and", "lose"],
    &["and", "loses"],
];
const TARGET_CONTROLLED_PUMP_GRANTED_ABILITY_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::any_phrase(TARGET_GETS_THEN_GAINS_GRANT_PHRASES),
        effect_grammar::EffectSequence::tail(
            "ability_tail",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const SOURCE_GETS_UNBLOCKABLE_TAIL_PHRASES: &[&[&str]] = &[
    &[
        "until", "end", "of", "turn", "and", "cant", "be", "blocked", "this", "turn",
    ],
    &[
        "until", "end", "of", "turn", "and", "can't", "be", "blocked", "this", "turn",
    ],
];
const UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::modifier(
        "duration_and_restriction",
        effect_grammar::EffectCaptureKind::OneOfPhrase(SOURCE_GETS_UNBLOCKABLE_TAIL_PHRASES),
    )]);
const TARGET_CONTROLLED_PUMP_CONTROLLER_PHRASES: &[&[&str]] = &[
    &["target", "player", "controls"],
    &["target", "players", "control"],
    &["target", "opponent", "controls"],
    &["target", "opponents", "control"],
];
const TARGET_CONTROLLED_PUMP_PLAYER_CONTROLLER_PHRASES: &[&[&str]] = &[
    &["target", "player", "controls"],
    &["target", "players", "control"],
];
const TARGET_CONTROLLED_PUMP_OPPONENT_CONTROLLER_PHRASES: &[&[&str]] = &[
    &["target", "opponent", "controls"],
    &["target", "opponents", "control"],
];
const TARGET_CONTROLLED_PUMP_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "affected",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(
                TARGET_CONTROLLED_PUMP_CONTROLLER_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::condition(
            "controller",
            effect_grammar::EffectCaptureKind::OneOfPhrase(
                TARGET_CONTROLLED_PUMP_CONTROLLER_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::action(
            "action",
            effect_grammar::EffectCaptureKind::OneOf(&["get", "gets"]),
        ),
        effect_grammar::EffectSequence::amount(
            "modifier",
            effect_grammar::EffectCaptureKind::WordCount(1),
        ),
        effect_grammar::EffectSequence::tail("tail", effect_grammar::EffectCaptureKind::Rest),
    ]);
const TARGET_CONTROLLED_PUMP_PLAYER_CONTROLLER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::condition(
        "controller",
        effect_grammar::EffectCaptureKind::OneOfPhrase(
            TARGET_CONTROLLED_PUMP_PLAYER_CONTROLLER_PHRASES,
        ),
    )]);
const TARGET_CONTROLLED_PUMP_OPPONENT_CONTROLLER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::condition(
        "controller",
        effect_grammar::EffectCaptureKind::OneOfPhrase(
            TARGET_CONTROLLED_PUMP_OPPONENT_CONTROLLER_PHRASES,
        ),
    )]);
const PUT_COUNTED_TOP_CARDS_OBJECT_PHRASES: &[&[&str]] = &[
    &["of", "them"],
    &["them"],
    &["of", "those", "card"],
    &["of", "those", "cards"],
    &["those", "card"],
    &["those", "cards"],
];
const PUT_COUNTED_TOP_CARDS_HAND_PHRASES: &[&[&str]] = &[&["hand"]];
const PUT_COUNTED_TOP_CARDS_GRAVEYARD_PHRASES: &[&[&str]] = &[&["graveyard"], &["graveyards"]];
const PUT_COUNTED_TOP_CARDS_YOU_OWNER_PHRASES: &[&[&str]] = &[&["your"]];
const PUT_COUNTED_TOP_CARDS_THAT_OWNER_PHRASES: &[&[&str]] = &[
    &["their"],
    &["that", "player"],
    &["that", "players"],
    &["that", "player's"],
    &["that", "players'"],
];
const PUT_COUNTED_TOP_CARDS_YOU_OWNER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::subject(
        "owner",
        effect_grammar::EffectCaptureKind::OneOfPhrase(PUT_COUNTED_TOP_CARDS_YOU_OWNER_PHRASES),
    )]);
const PUT_COUNTED_TOP_CARDS_THAT_OWNER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::subject(
        "owner",
        effect_grammar::EffectCaptureKind::OneOfPhrase(PUT_COUNTED_TOP_CARDS_THAT_OWNER_PHRASES),
    )]);
const OPTIONAL_THE_PATTERN_ATOMS: &[effect_grammar::EffectAtom<'static>] =
    &[effect_grammar::EffectSequence::word("the")];
const PUT_COUNTED_TOP_CARDS_VIEW_THEN_REMAINDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::capture(
            "view_clause",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["then"]),
        ),
        effect_grammar::EffectSequence::word("then"),
        effect_grammar::EffectSequence::tail("remainder", effect_grammar::EffectCaptureKind::Rest),
    ]);
const PUT_COUNTED_TOP_CARDS_REMAINDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("put"),
        effect_grammar::EffectSequence::amount(
            "put_count",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(PUT_COUNTED_TOP_CARDS_OBJECT_PHRASES),
        ),
        effect_grammar::EffectSequence::any_phrase(PUT_COUNTED_TOP_CARDS_OBJECT_PHRASES),
        effect_grammar::EffectSequence::word("into"),
        effect_grammar::EffectSequence::capture(
            "hand_owner",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(PUT_COUNTED_TOP_CARDS_HAND_PHRASES),
        ),
        effect_grammar::EffectSequence::word("hand"),
        effect_grammar::EffectSequence::word("and"),
        effect_grammar::EffectSequence::optional(OPTIONAL_THE_PATTERN_ATOMS),
        effect_grammar::EffectSequence::word("rest"),
        effect_grammar::EffectSequence::word("into"),
        effect_grammar::EffectSequence::capture(
            "graveyard_owner",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(
                PUT_COUNTED_TOP_CARDS_GRAVEYARD_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::any_phrase(PUT_COUNTED_TOP_CARDS_GRAVEYARD_PHRASES),
    ]);
const VOTE_REVEAL_TAIL_PREFIX_PHRASES: &[&[&str]] = &[
    &["then", "those", "votes", "are"],
    &["then", "those", "choices", "are"],
];
const VOTE_REVEAL_TAIL_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::any_phrase(VOTE_REVEAL_TAIL_PREFIX_PHRASES),
        effect_grammar::EffectSequence::tail(
            "reveal_tail",
            effect_grammar::EffectCaptureKind::Rest,
        ),
    ]);
const OPTIONAL_THEN_PATTERN_ATOMS: &[effect_grammar::EffectAtom<'static>] =
    &[effect_grammar::EffectSequence::word("then")];
const THOSE_CHOICES_PHRASES: &[&[&str]] = &[&["those", "choices"]];
const VOTE_REVEAL_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::optional(OPTIONAL_THEN_PATTERN_ATOMS),
        effect_grammar::EffectSequence::subject(
            "choices",
            effect_grammar::EffectCaptureKind::OneOfPhrase(THOSE_CHOICES_PHRASES),
        ),
        effect_grammar::EffectSequence::word("are"),
        effect_grammar::EffectSequence::action(
            "reveal",
            effect_grammar::EffectCaptureKind::OneOf(&["revealed"]),
        ),
    ]);
const SECRET_CHOICE_PARTICIPANTS_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::phrase(&["you", "and", "target", "opponent"]),
        effect_grammar::EffectSequence::capture(
            "between_opponent_each",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["each"]),
        ),
        effect_grammar::EffectSequence::word("each"),
        effect_grammar::EffectSequence::capture(
            "secret_intro",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["secret"], &["secretly"]]),
        ),
        effect_grammar::EffectSequence::any_word(&["secret", "secretly"]),
    ]);
const EACH_PLAYER_VOTER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("each"),
        effect_grammar::EffectSequence::capture(
            "between_each_player",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["player"], &["players"]]),
        ),
        effect_grammar::EffectSequence::any_word(&["player", "players"]),
    ]);
const STARTING_WITH_CONTROLLER_VOTER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "starting", "with", "you",
    ])]);
const SECRET_VOTER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::any_word(&[
        "secret", "secretly",
    ])]);
const VOTE_OPTION_DELIMITER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::action(
        "delimiter",
        effect_grammar::EffectCaptureKind::OneOf(&["or"]),
    )]);
const SECRET_NUMBER_CHOICE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "participants",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["choose"]),
        ),
        effect_grammar::EffectSequence::action(
            "choose",
            effect_grammar::EffectCaptureKind::OneOf(&["choose"]),
        ),
        effect_grammar::EffectSequence::tail("options", effect_grammar::EffectCaptureKind::Rest),
    ]);
const GENERIC_VOTE_START_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "voters",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["vote"], &["votes"]]),
        ),
        effect_grammar::EffectSequence::action(
            "vote",
            effect_grammar::EffectCaptureKind::OneOf(&["vote", "votes"]),
        ),
        effect_grammar::EffectSequence::word("for"),
        effect_grammar::EffectSequence::tail("options", effect_grammar::EffectCaptureKind::Rest),
    ]);
const GENERIC_PLAYER_VOTE_RECEIVED_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::phrase(&["for", "each"]),
        effect_grammar::EffectSequence::action(
            "vote",
            effect_grammar::EffectCaptureKind::OneOf(&["vote", "votes"]),
        ),
        effect_grammar::EffectSequence::subject(
            "player",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(&[&["received"], &["receives"]]),
        ),
        effect_grammar::EffectSequence::action(
            "received",
            effect_grammar::EffectCaptureKind::OneOf(&["received", "receives"]),
        ),
        effect_grammar::EffectSequence::tail("effects", effect_grammar::EffectCaptureKind::Rest),
    ]);
const OPTIONAL_AN_PATTERN_ATOMS: &[effect_grammar::EffectAtom<'static>] =
    &[effect_grammar::EffectSequence::word("an")];
const OPTIONAL_EXTRA_VOTE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "voter",
            effect_grammar::EffectCaptureKind::OneOf(&["you"]),
        ),
        effect_grammar::EffectSequence::capture(
            "may",
            effect_grammar::EffectCaptureKind::OneOf(&["may"]),
        ),
        effect_grammar::EffectSequence::action(
            "vote",
            effect_grammar::EffectCaptureKind::OneOf(&["vote", "votes"]),
        ),
        effect_grammar::EffectSequence::optional(OPTIONAL_AN_PATTERN_ATOMS),
        effect_grammar::EffectSequence::word("additional"),
        effect_grammar::EffectSequence::amount(
            "time",
            effect_grammar::EffectCaptureKind::OneOf(&["time", "times"]),
        ),
    ]);
const REQUIRED_EXTRA_VOTE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::subject(
            "voter",
            effect_grammar::EffectCaptureKind::OneOf(&["you"]),
        ),
        effect_grammar::EffectSequence::action(
            "vote",
            effect_grammar::EffectCaptureKind::OneOf(&["vote", "votes"]),
        ),
        effect_grammar::EffectSequence::optional(OPTIONAL_AN_PATTERN_ATOMS),
        effect_grammar::EffectSequence::word("additional"),
        effect_grammar::EffectSequence::amount(
            "time",
            effect_grammar::EffectCaptureKind::OneOf(&["time", "times"]),
        ),
    ]);
const SUBJECTLESS_EXTRA_VOTE_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::action(
            "vote",
            effect_grammar::EffectCaptureKind::OneOf(&["vote", "votes"]),
        ),
        effect_grammar::EffectSequence::optional(OPTIONAL_AN_PATTERN_ATOMS),
        effect_grammar::EffectSequence::word("additional"),
        effect_grammar::EffectSequence::amount(
            "time",
            effect_grammar::EffectCaptureKind::OneOf(&["time", "times"]),
        ),
    ]);
const DAMAGE_REPLACEMENT_COUNTER_TARGET_PHRASE: &[&str] = &["damage", "would", "be", "dealt", "to"];
const DAMAGE_REPLACEMENT_COUNTER_DURATION_PHRASE: &[&str] = &["this", "turn"];
const DAMAGE_REPLACEMENT_COUNTER_PREVENT_PUT_PHRASE: &[&str] = &[
    "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1",
];
const DAMAGE_REPLACEMENT_COUNTER_RECIPIENT_PHRASES: &[&[&str]] = &[&["it"], &["that", "creature"]];

fn has_where_x_value_binding(tokens: &[OwnedLexToken]) -> bool {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = WHERE_X_VALUE_BINDING_PATTERN.parse_full(clause) else {
        return false;
    };
    matched
        .capture_clause_by_role(effect_grammar::EffectCaptureRole::Condition, clause)
        .is_some()
        && matched
            .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
            .is_some()
}

pub fn parse_any_player_may_have_source_deal_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_any_player_source_damage(tokens) else {
        return Ok(None);
    };
    let deal_tail = trim_edge_punctuation(shape.damage_tokens);
    let Some((amount, used)) = parse_value(&deal_tail) else {
        return Ok(None);
    };
    if deal_tail
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "damage")
    {
        return Ok(None);
    }
    let target_tail = trim_edge_punctuation(&deal_tail[used + 1..]);
    if !effect_grammar::exact_any_tokens(
        &target_tail,
        &[&["to", "them"], &["to", "that", "player"]],
    ) {
        return Ok(None);
    }

    let damage = EffectAst::subject_verb_damage(
        amount,
        TargetAst::Player(
            if matches!(shape.player, PlayerAst::Any | PlayerAst::Opponent) {
                PlayerFilter::IteratedPlayer
            } else {
                shape.player_filter.clone()
            },
            None,
        ),
    );
    if matches!(shape.player, PlayerAst::Any | PlayerAst::Opponent) {
        Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::AnyPlayerMay {
            players: shape.player_filter,
            effects: vec![damage],
        })]))
    } else {
        Ok(Some(vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: shape.player,
            effects: vec![damage],
        })]))
    }
}

fn parse_branch_scoped_collection_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<(&'static str, Vec<EffectAst>)> {
    fn is_conjunctive_collection(effect: &EffectAst) -> bool {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return false;
        };
        let filter = match action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }) => filter,
            _ => return false,
        };
        filter.any_of.len() >= 2 && filter.has_conjunctive_set_surface()
    }

    let clause = trim_edge_punctuation(tokens);
    let clause = if clause.first().is_some_and(|token| token.is_word("then")) {
        trim_edge_punctuation(&clause[1..])
    } else {
        clause
    };
    let (route, effect) = if clause.first().is_some_and(|token| token.is_word("return")) {
        (
            "subject-verb verb=Return subject=implicit recognizer=branch-scoped-collection",
            crate::grammar::primitives::probe_shape(super::zone_handlers::parse_return(
                &clause[1..],
            ))?,
        )
    } else if clause.first().is_some_and(|token| token.is_word("destroy")) {
        (
            "subject-verb verb=Destroy subject=implicit recognizer=branch-scoped-collection",
            crate::grammar::primitives::probe_shape(super::zone_handlers::parse_destroy(
                &clause[1..],
            ))?,
        )
    } else {
        return None;
    };

    is_conjunctive_collection(&effect).then_some((route, vec![effect]))
}


use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::VoteEffectAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::TokenActionAst;
use crate::cards::builders::LibraryActionAst;
use crate::recognition::ParseOutcome;
#[path = "generic_subject_verb/top_level_readings.rs"]
mod top_level_readings;

pub fn parse_top_level_subject_verb_recognition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    let input = top_level_readings::TopLevelSentence { tokens };
    match top_level_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let program = if let Some(effect) = parse_generic_meld_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::Meld { effect })
    } else if let Some(effect) = parse_generic_control_combat_choices_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::ControlCombatChoices { effect })
    } else if let Some(effect) = parse_generic_damage_replacement_counters_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::PreventDamageAndPutCounters { effect })
    } else if let Some(effects) =
        parse_generic_top_cards_cloak_counted_rest_bottom_subject_verb(tokens)
    {
        Some(GenericTopLevelProgram::LookedCardsCountedRemainder { effects })
    } else if let Some(effects) =
        parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(tokens)
    {
        Some(GenericTopLevelProgram::LookedCardsCountedRemainder { effects })
    } else if let Some(effects) =
        parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(tokens)
    {
        Some(GenericTopLevelProgram::LookedCardsCountedRemainder { effects })
    } else if let Some(effects) =
        parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb(tokens)?
    {
        Some(GenericTopLevelProgram::ConsultRevealUntilHand { effects })
    } else if let Some(effects) =
        parse_generic_consult_reveal_until_put_all_revealed_into_graveyard_subject_verb(tokens)?
    {
        Some(GenericTopLevelProgram::ConsultRevealUntilGraveyard { effects })
    } else if let Some(effects) =
        parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(tokens)?
    {
        Some(GenericTopLevelProgram::ConsultRevealUntilBattlefieldBottom { effects })
    } else if let Some(effects) = parse_generic_consult_reveal_until_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::ConsultRevealUntil { effects })
    } else if let Some(effects) =
        parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(tokens)?
    {
        Some(GenericTopLevelProgram::EachPlayerExileTopCast { effects })
    } else if let Some(effects) = parse_cant_effect_sentence_lexed(tokens)? {
        Some(GenericTopLevelProgram::Cant { effects })
    } else if has_where_x_value_binding(tokens) {
        let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        Some(GenericTopLevelProgram::ValueBinding { effects })
    } else {
        None
    };
    Ok(program.map(|program| {
        let route = program.route();
        (route, program.lower())
    }))
}

fn parse_as_you_cast_from_zone_this_turn_grant(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let word_view = clause.words();
    let words = word_view.word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&words, &["as", "you", "cast"])
        || words.len() < 9
    {
        return Ok(None);
    }
    let Some(turn_index) = crate::word_primitives::parse_sequence_start(&words, &["this", "turn"])
    else {
        return Ok(None);
    };
    if turn_index <= 3
        || !words.get(turn_index + 2..).is_some_and(|tail| {
            crate::word_primitives::parse_sequence_prefix(tail, &["they", "gain"])
        })
    {
        return Ok(None);
    }
    let Some(subject_range) = word_view.token_span_for_words(3, turn_index) else {
        return Ok(None);
    };
    let Some(ability_range) = word_view.token_span_for_words(turn_index + 4, words.len()) else {
        return Ok(None);
    };
    let subject_tokens = &clause.tokens()[subject_range];
    let subject_words = TokenWordView::new(subject_tokens).word_refs();
    let Some(from_index) =
        crate::word_primitives::parse_last_sequence_start(&subject_words, &["from"])
    else {
        return Ok(None);
    };
    let origin_words = &subject_words[from_index + 1..];
    let origin_word = if origin_words.len() == 1 {
        origin_words[0]
    } else if origin_words.len() == 2
        && crate::word_primitives::first_is_any(
            origin_words,
            &["a", "an", "the", "your", "their", "its"],
        )
    {
        origin_words[1]
    } else {
        return Ok(None);
    };
    let Some(origin_zone) = crate::util::parse_zone_word(origin_word) else {
        return Ok(None);
    };

    let mut filter = parse_object_filter_lexed(subject_tokens, false)?;
    filter.cast_by = Some(PlayerFilter::You);
    if filter.stack_kind != Some(crate::filter::StackObjectKind::Spell) {
        return Ok(None);
    }
    // The ordinary filter parser quite reasonably models `spells` as stack
    // objects. In this as-you-cast trigger, however, the authored `from ...`
    // phrase is the pre-cast origin and must replace that default stack zone.
    filter.zone = Some(origin_zone);
    let Some(ability) =
        crate::activation_and_restrictions::parse_ability_phrase(&clause.tokens()[ability_range])
    else {
        return Ok(None);
    };
    filter.set_as_you_cast_this_turn_surface(true);
    Ok(Some(EffectAst::subject_verb_grant_abilities_all(
        filter,
        vec![ability.into()],
        Until::EndOfTurn,
    )))
}

fn parse_generic_play_exiled_cards_for_as_long_as_exiled(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let trimmed = trim_commas(tokens);
    let words = TokenWordView::new(&trimmed).word_refs();
    let matches = effect_grammar::exact_any_words(
        &words,
        &[
            &[
                "play", "the", "exiled", "cards", "for", "as", "long", "as", "they", "remain",
                "exiled",
            ],
            &[
                "play", "exiled", "cards", "for", "as", "long", "as", "they", "remain", "exiled",
            ],
        ],
    );
    matches.then(|| {
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            crate::tag::CompilerReferenceTag::It.bind(),
            PlayerAst::You,
            true,
            false,
            false,
            None,
        )
    })
}

fn parse_generic_mana_any_type_cast_tagged_this_way(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let trimmed = trim_commas(tokens);
    let words = TokenWordView::new(&trimmed).word_refs();
    let matches = effect_grammar::exact_any_words(
        &words,
        &[
            &[
                "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "spells", "this",
                "way",
            ],
            &[
                "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "them", "this",
                "way",
            ],
            &[
                "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "that", "spell",
                "this", "way",
            ],
        ],
    );
    matches.then(|| {
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            crate::tag::CompilerReferenceTag::It.bind(),
            PlayerAst::You,
            false,
            false,
            true,
            None,
        )
    })
}

pub fn parse_source_gets_unblockable_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = SOURCE_GETS_UNBLOCKABLE_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(subject_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(modifier_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };
    let Some(tail_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    if !SOURCE_GETS_SUBJECT_PATTERN.accepts_full(subject_clause.trimmed()) {
        return Ok(None);
    }
    let Some((power, toughness)) = parse_pt_modifier_capture(modifier_clause) else {
        return Ok(None);
    };

    if !UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN.accepts_full(tail_clause.trimmed()) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Source(None),
            Until::EndOfTurn,
            None,
        ),
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::be_blocked(ObjectFilter::source()),
            Until::EndOfTurn,
            None,
        ),
    ]))
}

/// Preserve a mixed-subject attachment action as one coordinated program:
/// `destroy enchanted land and this Aura deals N damage to that land's
/// controller`. Both halves are independently typed, but the ordinary chain
/// splitter can hand the possessive controller tail to the damage-trigger
/// probe before it has established the attachment reference.
fn parse_destroy_attached_object_then_source_damage_to_controller(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
    let [destroy_tokens, damage_tokens] = segments.as_slice() else {
        return Ok(None);
    };
    let destroy_words = crate::lexer::parser_token_word_refs(destroy_tokens);
    let ["destroy", attachment_word, object_noun] = destroy_words.as_slice() else {
        return Ok(None);
    };
    if !matches!(*attachment_word, "enchanted" | "equipped") {
        return Ok(None);
    }

    let damage_words = crate::lexer::parser_token_word_refs(damage_tokens);
    let Some(deals_idx) = crate::word_primitives::parse_sequence_start(&damage_words, &["deals"])
    else {
        return Ok(None);
    };
    if deals_idx == 0
        || crate::util::source_reference_surface_for_words(&damage_words[..deals_idx]).is_none()
    {
        return Ok(None);
    }
    let suffix = &damage_words[deals_idx..];
    let [
        "deals",
        amount_word,
        "damage",
        "to",
        "that",
        possessive_noun,
        "controller",
    ] = suffix
    else {
        return Ok(None);
    };
    if *possessive_noun != format!("{object_noun}s") {
        return Ok(None);
    }
    let Some(amount) = crate::util::parse_number_word_u32(amount_word)
        .and_then(|amount| crate::util::narrowed_i32(amount))
    else {
        return Ok(None);
    };

    let filter = parse_object_filter(&destroy_tokens[1..], false)?;
    let attachment_tag = match *attachment_word {
        "enchanted" => crate::tag::CompilerReferenceTag::Enchanted.bind(),
        "equipped" => crate::tag::CompilerReferenceTag::Equipped.bind(),
        _ => unreachable!("attachment word was lexically constrained"),
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == attachment_tag.key.clone()
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return Ok(None);
    }

    let destroy = EffectAst::subject_verb_destroy(TargetAst::Object(filter, None, None));
    let damage = EffectAst::subject_verb_damage(
        Value::Fixed(amount),
        TargetAst::Player(
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(attachment_tag.key.clone())),
            None,
        ),
    );
    Ok(Some(vec![EffectAst::Coordinated {
        effects: vec![destroy, damage],
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub fn parse_target_gets_unblockable_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // The public lexer keeps a signed P/T modifier as one token, but quote
    // normalization can represent `can't` differently from the declarative
    // sequence pattern below. Prove the same exact clause from its stable
    // verb/duration boundaries before consulting that surface-sensitive
    // pattern. This is still deliberately narrow: one target subject, one
    // P/T modifier, and the complete same-turn blocking restriction.
    let gets_idx = crate::slice_primitives::select_position(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    });
    let until_idx = gets_idx.and_then(|gets_idx| {
        crate::slice_primitives::select_position(&tokens[gets_idx + 1..], |token| {
            token.is_word("until")
        })
        .map(|offset| gets_idx + 1 + offset)
    });
    if let (Some(gets_idx), Some(until_idx)) = (gets_idx, until_idx) {
        let subject_tokens = trim_edge_punctuation(&tokens[..gets_idx]);
        let modifier_tokens = trim_edge_punctuation(&tokens[gets_idx + 1..until_idx]);
        let tail_words = crate::util::words(&tokens[until_idx..]);
        let tail_text = tail_words.join(" ").replace("can't", "cant");
        let exact_tail = matches!(
            tail_text.as_str(),
            "until end of turn and cant be blocked this turn"
                | "until end of turn and can t be blocked this turn"
        );
        if exact_tail
            && starts_with_target_indicator(&subject_tokens)
            && let Some((power, toughness)) =
                parse_pt_modifier_capture(LexedClause::new(&modifier_tokens))
        {
            let target = parse_target_phrase(&subject_tokens)?;
            let Some(mut blocked_filter) = target_ast_to_object_filter(target.clone()) else {
                return Ok(None);
            };
            if !blocked_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            }) {
                blocked_filter = blocked_filter.match_tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    TaggedOpbjectRelation::IsTaggedObject,
                );
            }
            return Ok(Some(vec![
                EffectAst::subject_verb_pump(power, toughness, target, Until::EndOfTurn, None),
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::be_blocked(blocked_filter),
                    Until::EndOfTurn,
                    None,
                ),
            ]));
        }
    }

    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = SOURCE_GETS_UNBLOCKABLE_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(subject_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(modifier_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };
    let Some(tail_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    let subject_tokens = subject_clause.trimmed().tokens();
    if !starts_with_target_indicator(subject_tokens) {
        return Ok(None);
    }
    let target = parse_target_phrase(subject_tokens)?;
    let Some(mut blocked_filter) = target_ast_to_object_filter(target.clone()) else {
        return Ok(None);
    };
    if !blocked_filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        blocked_filter = blocked_filter.match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }
    let Some((power, toughness)) = parse_pt_modifier_capture(modifier_clause) else {
        return Ok(None);
    };

    if !UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN.accepts_full(tail_clause.trimmed()) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_pump(power, toughness, target, Until::EndOfTurn, None),
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::be_blocked(blocked_filter),
            Until::EndOfTurn,
            None,
        ),
    ]))
}

fn parse_cant_blocked_then_base_pt_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_cant_blocked_base_power_toughness_tokens(tokens) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.subject_tokens)?;
    let Some(mut blocked_filter) = target_ast_to_object_filter(target.clone()) else {
        return Ok(None);
    };
    if !blocked_filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        blocked_filter = blocked_filter.match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    Ok(Some(vec![
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::be_blocked(blocked_filter),
            Until::EndOfTurn,
            None,
        ),
        EffectAst::subject_verb_set_base_power_toughness(
            shape.power,
            shape.toughness,
            target,
            Until::EndOfTurn,
        ),
    ]))
}

fn parse_source_gets_filter_gains_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = SOURCE_GETS_FILTER_GAINS_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(subject_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(modifier_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };
    let Some(granted_filter_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let Some(ability_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };

    if !SOURCE_GETS_SUBJECT_PATTERN.accepts_full(subject_clause.trimmed()) {
        return Ok(None);
    }
    let Some((power, toughness)) = parse_pt_modifier_capture(modifier_clause) else {
        return Ok(None);
    };
    let Ok(filter) = parse_object_filter(granted_filter_clause.trimmed().tokens(), false) else {
        return Ok(None);
    };
    let abilities = keyword_abilities_from_clause(ability_clause.trimmed());
    if abilities.is_empty() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_pump(
            power,
            toughness,
            TargetAst::Source(None),
            Until::EndOfTurn,
            None,
        ),
        EffectAst::subject_verb_grant_abilities_all(filter, abilities, Until::EndOfTurn),
    ]))
}

fn parse_target_gains_then_gets_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::gain_ability_shapes::parse_gain_then_get_shape(tokens) else {
        return Ok(None);
    };
    let _typed_captures = (
        shape.subject_tokens,
        shape.ability_tokens,
        shape.pump_tokens,
    );
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn attached_and_related_creatures_filter(
    subject: effect_grammar::gain_ability_shapes::AttachedReferenceSubject,
) -> ObjectFilter {
    let attached_tag = match subject {
        effect_grammar::gain_ability_shapes::AttachedReferenceSubject::EnchantedCreature => {
            "enchanted"
        }
        effect_grammar::gain_ability_shapes::AttachedReferenceSubject::EquippedCreature => {
            "equipped"
        }
    };
    let attached =
        ObjectFilter::creature().match_tagged(attached_tag, TaggedOpbjectRelation::IsTaggedObject);
    let related = ObjectFilter::creature()
        .not_tagged(attached_tag)
        .shares_subtype_with_tagged(attached_tag);
    let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    filter.any_of = vec![attached, related];
    filter
}

fn parse_attached_and_related_get_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        effect_grammar::gain_ability_shapes::parse_attached_and_related_get_shape(tokens)
    else {
        return Ok(None);
    };
    let Some((power, toughness)) = parse_pt_modifier_capture(LexedClause::new(shape.pump_tokens))
    else {
        return Ok(None);
    };
    let filter = attached_and_related_creatures_filter(shape.subject);
    Ok(Some(vec![EffectAst::subject_verb_pump_all(
        filter,
        power,
        toughness,
        shape.duration,
    )]))
}

fn parse_target_gets_then_gains_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(shape) =
        effect_grammar::gain_ability_shapes::parse_attached_and_related_get_ability_shape(tokens)
    {
        let Some((power, toughness)) =
            parse_pt_modifier_capture(LexedClause::new(shape.pump_tokens))
        else {
            return Ok(None);
        };
        let abilities = keyword_abilities_from_clause(LexedClause::new(shape.ability_tokens));
        if abilities.is_empty() {
            return Ok(None);
        }
        let filter = attached_and_related_creatures_filter(shape.subject);
        return Ok(Some(vec![
            EffectAst::subject_verb_pump_all(
                filter.clone(),
                power,
                toughness,
                shape.duration.clone(),
            ),
            EffectAst::subject_verb_grant_abilities_all(filter, abilities, shape.duration),
        ]));
    }
    let Some(shape) = effect_grammar::gain_ability_shapes::parse_get_then_ability_shape(tokens)
    else {
        return Ok(None);
    };
    if shape.ability_verb == effect_grammar::gain_ability_shapes::SharedAbilityVerb::Lose {
        return Ok(None);
    }
    let Some(mut effects) = super::gain_ability::parse_gain_ability_sentence_with_typed_subject(
        tokens,
        shape.subject_tokens,
    )?
    else {
        return Ok(None);
    };
    // "Creatures of the creature type of your choice get ... and gain ..."
    // — the plain subject-filter parse drops the choice qualifier; restore
    // it the way the standalone creature-type-choice pump primitive does.
    if subject_has_creature_type_choice(shape.subject_tokens) {
        patch_creature_type_choice_effects(&mut effects);
    }
    Ok(Some(effects))
}

fn subject_has_creature_type_choice(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::token_word_refs(tokens);
    crate::word_primitives::sequence_occurs(&words, &["creature", "type", "of", "your", "choice"])
}

fn patch_creature_type_choice_effect(effect: &mut EffectAst) -> bool {
    // Compound gain sentences wrap their members in coordination nodes;
    // patch through them.
    match effect {
        EffectAst::Coordination(coordination) => {
            let mut patched = false;
            for inner in coordination.effects_mut() {
                patched |= patch_creature_type_choice_effect(inner);
            }
            return patched;
        }
        EffectAst::Coordinated { effects, .. } | EffectAst::Sequence { effects, .. } => {
            let mut patched = false;
            for inner in effects.iter_mut() {
                patched |= patch_creature_type_choice_effect(inner);
            }
            return patched;
        }
        _ => {}
    }
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return false;
    };
    match action {
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { filter, .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { filter, .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll { filter, .. }) => {
            filter.chosen_creature_type = true;
            true
        }
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
            target: TargetAst::Object(filter, _, _),
            ..
        })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
            target: TargetAst::Object(filter, _, _),
            ..
        }) => {
            filter.chosen_creature_type = true;
            true
        }
        _ => false,
    }
}

fn patch_creature_type_choice_effects(effects: &mut Vec<EffectAst>) {
    let mut patched = false;
    for effect in effects.iter_mut() {
        patched |= patch_creature_type_choice_effect(effect);
    }
    if patched {
        effects.insert(
            0,
            EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        );
    }
}

fn parse_target_has_base_pt_then_loses_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = TARGET_HAS_BASE_PT_THEN_LOSES_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(base_pt_clause) = matched.capture_clause("base_pt_clause", clause) else {
        return Ok(None);
    };
    if !effect_grammar::prefix_words(
        &base_pt_clause.word_refs(),
        &["base", "power", "and", "toughness"],
    ) {
        return Ok(None);
    }
    let Some(_ability_tail) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn parse_target_player_controls_get_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(trailing_if) = crate::grammar::structure::split_trailing_if_clause_lexed(tokens) {
        let Some(program) = parse_target_controlled_pump_program(trailing_if.leading_tokens)?
        else {
            return Ok(None);
        };
        return Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects: program.lower(),
        })]));
    }

    let Some(program) = parse_target_controlled_pump_program(tokens)? else {
        return Ok(None);
    };
    Ok(Some(program.lower()))
}

fn parse_target_controlled_pump_program(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetControlledPumpProgram>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let Some(matched) = TARGET_CONTROLLED_PUMP_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(subject_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(controller_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Condition, clause)
    else {
        return Ok(None);
    };
    let Some(modifier_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Amount, clause)
    else {
        return Ok(None);
    };
    let Some((power, toughness)) = parse_pt_modifier_capture(modifier_clause) else {
        return Ok(None);
    };
    let subject_tokens = subject_clause.tokens();
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter(subject_tokens, false)?;
    filter.controller = target_controlled_pump_controller(controller_clause.trimmed());

    let tail_clause = matched
        .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
        .unwrap_or_else(|| LexedClause::new(&[]))
        .trimmed();
    let mut abilities = Vec::new();
    let mut add_all_creature_types = false;
    let mut remove_all_creature_types = false;
    if let Some(tail_match) = TARGET_CONTROLLED_PUMP_GRANTED_ABILITY_PATTERN.parse_full(tail_clause)
    {
        let Some(ability_clause) =
            tail_match.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, tail_clause)
        else {
            return Ok(None);
        };
        let ability_clause = ability_clause.trimmed();
        let ability_words = ability_clause.word_refs();
        add_all_creature_types = crate::word_primitives::parse_sequence_complete(
            &ability_words,
            &["all", "creature", "types", "until", "end", "of", "turn"],
        );
        remove_all_creature_types = add_all_creature_types
            && tail_clause
                .word_refs()
                .iter()
                .any(|word| matches!(*word, "lose" | "loses"));
        add_all_creature_types &= !remove_all_creature_types;
        abilities.extend(keyword_abilities_from_clause(ability_clause));
    }
    Ok(Some(TargetControlledPumpProgram {
        filter,
        power,
        toughness,
        abilities,
        add_all_creature_types,
        remove_all_creature_types,
    }))
}

fn keyword_abilities_from_clause(ability_clause: LexedClause<'_>) -> Vec<GrantedAbilityAst> {
    let ability_clause = ability_clause.trimmed();
    let mut abilities = Vec::new();
    if ABILITY_FIRST_STRIKE_PATTERN
        .locate_in(ability_clause)
        .is_some()
    {
        abilities.push(KeywordAction::FirstStrike.into());
    }
    if ABILITY_HASTE_PATTERN.locate_in(ability_clause).is_some() {
        abilities.push(KeywordAction::Haste.into());
    }
    if ABILITY_TRAMPLE_PATTERN.locate_in(ability_clause).is_some() {
        abilities.push(KeywordAction::Trample.into());
    }
    abilities
}

fn parse_pt_modifier_capture(clause: LexedClause<'_>) -> Option<(Value, Value)> {
    let modifier_word = clause
        .trimmed()
        .tokens()
        .first()
        .and_then(OwnedLexToken::as_word)?;
    crate::grammar::primitives::probe_shape(crate::keyword_static::parse_pt_modifier_values(
        modifier_word,
    ))
}

fn target_controlled_pump_controller(controller_clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if TARGET_CONTROLLED_PUMP_OPPONENT_CONTROLLER_PATTERN.accepts_full(controller_clause) {
        Some(PlayerFilter::target_opponent())
    } else if TARGET_CONTROLLED_PUMP_PLAYER_CONTROLLER_PATTERN.accepts_full(controller_clause) {
        Some(PlayerFilter::target_player())
    } else {
        None
    }
}

fn put_counted_top_cards_owner(
    owner_clause: LexedClause<'_>,
    default: PlayerAst,
) -> Option<PlayerAst> {
    let owner_clause = owner_clause.trimmed();
    if owner_clause.is_empty() {
        Some(default)
    } else if PUT_COUNTED_TOP_CARDS_YOU_OWNER_PATTERN.accepts_full(owner_clause) {
        Some(PlayerAst::You)
    } else if PUT_COUNTED_TOP_CARDS_THAT_OWNER_PATTERN.accepts_full(owner_clause) {
        Some(PlayerAst::That)
    } else {
        None
    }
}

pub fn parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let clause_tokens = trim_commas(tokens);
    let clause = LexedClause::new(&clause_tokens).trimmed();
    let matched = PUT_COUNTED_TOP_CARDS_VIEW_THEN_REMAINDER_PATTERN.parse_full(clause)?;
    let view_clause = matched.capture_clause("view_clause", clause)?.trimmed();
    let remainder_clause = matched
        .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)?
        .trimmed();
    let prefix_tokens = trim_commas(view_clause.tokens());
    let (player, count, reveal_top) = super::parse_top_cards_view_sentence(&prefix_tokens)?;

    let tail_tokens = trim_commas(remainder_clause.tokens());
    let tail_clause = LexedClause::new(&tail_tokens).trimmed();
    let matched = PUT_COUNTED_TOP_CARDS_REMAINDER_PATTERN.parse_full(tail_clause)?;
    let count_clause = matched.capture_clause("put_count", tail_clause)?.trimmed();
    let (put_count, used) =
        crate::grammar::values::parse_number_prefix_lexed(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    let hand_owner_clause = matched.capture_clause("hand_owner", tail_clause)?;
    let chooser = put_counted_top_cards_owner(hand_owner_clause, player)?;
    let graveyard_owner_clause = matched.capture_clause("graveyard_owner", tail_clause)?;
    put_counted_top_cards_owner(graveyard_owner_clause, player)?;

    let looked_tag = crate::util::helper_tag_for_tokens(&prefix_tokens, "revealed");
    let chosen_tag = crate::util::helper_tag_for_tokens(&tail_tokens, "chosen");
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        crate::tag::TagRef::of(looked_tag.clone()),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(looked_tag.clone())));
    }
    effects.extend(EffectAst::compose_put_some_into_hand_rest_into_graveyard(
        chooser,
        crate::effect::ChoiceCount::exactly(put_count as usize),
        crate::tag::TagRef::of(looked_tag),
        crate::tag::TagRef::of(chosen_tag),
    ));
    Some(effects)
}

fn parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_clause = LexedClause::new(&sentence_tokens);
    let Some(matched) = CONSULT_REVEAL_UNTIL_HAND_PATTERN.parse_full(sentence_clause) else {
        return Ok(None);
    };
    let Some(consult_clause) = matched.capture_clause("consult_clause", sentence_clause) else {
        return Ok(None);
    };
    let Some(followup_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, sentence_clause)
    else {
        return Ok(None);
    };
    let consult_tokens = trim_commas(consult_clause.tokens());
    let followup_tokens = trim_commas(followup_clause.tokens());
    if consult_tokens.is_empty() || followup_tokens.is_empty() {
        return Ok(None);
    }

    let parts = if let Some(parts) =
        super::consult_family::parse_consult_traversal_sentence(&consult_tokens)?
    {
        parts
    } else {
        let stripped_consult_tokens = without_deferred_mana_value_clause(&consult_tokens);
        let Some(parts) =
            super::consult_family::parse_consult_traversal_sentence(&stripped_consult_tokens)?
        else {
            return Ok(None);
        };
        parts
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            }),
            ..
        }))
    ) {
        return Ok(None);
    }
    let mut parts = parts;
    apply_lesser_mana_value_consult_constraint(&sentence_tokens, &mut parts.effects);

    let followup_clause = LexedClause::new(&followup_tokens).trimmed();
    let puts_all_revealed_into_hand = ALL_REVEALED_INTO_HAND_PATTERN.accepts_full(followup_clause);
    if !puts_all_revealed_into_hand {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(parts.all_tag), None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Ok(Some(effects))
}

fn parse_generic_consult_reveal_until_put_all_revealed_into_graveyard_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_clause = LexedClause::new(&sentence_tokens);
    let Some(matched) = CONSULT_REVEAL_UNTIL_HAND_PATTERN.parse_full(sentence_clause) else {
        return Ok(None);
    };
    let Some(consult_clause) = matched.capture_clause("consult_clause", sentence_clause) else {
        return Ok(None);
    };
    let Some(followup_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, sentence_clause)
    else {
        return Ok(None);
    };
    let consult_tokens = trim_commas(consult_clause.tokens());
    let followup_tokens = trim_commas(followup_clause.tokens());
    if consult_tokens.is_empty() || followup_tokens.is_empty() {
        return Ok(None);
    }

    let parts = if let Some(parts) =
        super::consult_family::parse_consult_traversal_sentence(&consult_tokens)?
    {
        parts
    } else {
        let stripped_consult_tokens = without_deferred_mana_value_clause(&consult_tokens);
        let Some(parts) =
            super::consult_family::parse_consult_traversal_sentence(&stripped_consult_tokens)?
        else {
            return Ok(None);
        };
        parts
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            }),
            ..
        }))
    ) {
        return Ok(None);
    }
    let mut parts = parts;
    apply_lesser_mana_value_consult_constraint(&sentence_tokens, &mut parts.effects);

    let followup_clause = LexedClause::new(&followup_tokens).trimmed();
    if !ALL_REVEALED_INTO_GRAVEYARD_PATTERN.accepts_full(followup_clause) {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(parts.all_tag), None),
        Zone::Graveyard,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Ok(Some(effects))
}

fn parse_generic_consult_reveal_until_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let mut parts = if let Some(parts) =
        super::consult_family::parse_consult_traversal_sentence(&sentence_tokens)?
    {
        parts
    } else {
        let stripped_tokens = without_deferred_mana_value_clause(&sentence_tokens);
        let Some(parts) =
            super::consult_family::parse_consult_traversal_sentence(&stripped_tokens)?
        else {
            return Ok(None);
        };
        parts
    };
    if !parts.effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { .. }),
                ..
            })
        )
    }) {
        return Ok(None);
    }
    apply_lesser_mana_value_consult_constraint(&sentence_tokens, &mut parts.effects);
    Ok(Some(parts.effects))
}

pub(super) fn parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens =
        trim_commas(super::super::token_primitives::strip_leading_if_you_do_lexed(tokens));
    let sentence_clause = LexedClause::new(&sentence_tokens);
    let Some(matched) = CONSULT_REVEAL_UNTIL_BATTLEFIELD_BOTTOM_PATTERN.parse_full(sentence_clause)
    else {
        return Ok(None);
    };
    let Some(consult_clause) = matched.capture_clause("consult_clause", sentence_clause) else {
        return Ok(None);
    };
    let Some(followup_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, sentence_clause)
    else {
        return Ok(None);
    };
    let consult_tokens = trim_commas(consult_clause.tokens());
    let followup_tokens = trim_commas(followup_clause.tokens());
    if consult_tokens.is_empty() || followup_tokens.is_empty() {
        return Ok(None);
    }

    let Some(parts) = super::consult_family::parse_consult_traversal_sentence(&consult_tokens)?
    else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            }),
            ..
        }))
    ) {
        return Ok(None);
    }

    let followup_clause = LexedClause::new(&followup_tokens).trimmed();
    let Some(followup_match) = MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN.parse_full(followup_clause)
    else {
        return Ok(None);
    };
    let Some(remainder_clause) = followup_match
        .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, followup_clause)
    else {
        return Ok(None);
    };
    let Some(order) = consult_remainder_order_from_capture(remainder_clause.trimmed()) else {
        return Ok(None);
    };
    let battlefield_tapped = followup_tokens
        .iter()
        .any(|token| token.as_word().is_some_and(|word| word == "tapped"));

    let mut effects = parts.effects;
    apply_lesser_mana_value_consult_constraint(&sentence_tokens, &mut effects);
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(parts.match_tag.clone()), None),
        Zone::Battlefield,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        None,
    ));
    // Honor the authored remainder wording: bare "the rest" (Kethek) vs
    // "the rest of the revealed cards" (Fathom Trawl).
    let followup_words = crate::lexer::token_word_refs(&followup_tokens);
    let bare_rest = crate::word_primitives::sequence_occurs(&followup_words, &["the", "rest"])
        && !crate::word_primitives::sequence_occurs(&followup_words, &["rest", "of", "the"]);
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
            crate::tag::TagRef::of(parts.all_tag),
            Some(crate::tag::TagRef::of(parts.match_tag)),
            order,
            parts.player,
            if bare_rest {
                ironsmith_core::LibraryRemainderSurface::RestBare
            } else {
                ironsmith_core::LibraryRemainderSurface::Rest
            },
        ),
    );
    Ok(Some(effects))
}

fn consult_remainder_order_from_capture(
    clause: LexedClause<'_>,
) -> Option<crate::cards::builders::LibraryBottomOrderAst> {
    let matched = REST_BOTTOM_LIBRARY_WITH_ORDER_PATTERN.locate_in(clause)?;
    let order_clause =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Amount, clause)?;
    let order_clause = order_clause.trimmed();
    if REST_BOTTOM_LIBRARY_RANDOM_ORDER_PATTERN.accepts_full(order_clause) {
        Some(crate::cards::builders::LibraryBottomOrderAst::Random)
    } else if REST_BOTTOM_LIBRARY_ANY_ORDER_PATTERN.accepts_full(order_clause) {
        Some(crate::cards::builders::LibraryBottomOrderAst::ChooserChooses)
    } else {
        None
    }
}

fn parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_clause = LexedClause::new(&sentence_tokens);
    let Some(matched) = EACH_PLAYER_EXILE_TOP_CAST_PATTERN.parse_full(sentence_clause) else {
        return Ok(None);
    };
    let Some(exile_clause) = matched.capture_clause("exile_clause", sentence_clause) else {
        return Ok(None);
    };
    let Some(cast_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, sentence_clause)
    else {
        return Ok(None);
    };
    let exile_tokens = trim_commas(exile_clause.tokens());
    let cast_tokens = trim_commas(cast_clause.tokens());
    if exile_tokens.is_empty() || cast_tokens.is_empty() {
        return Ok(None);
    }

    let exile_clause = LexedClause::new(&exile_tokens).trimmed();
    let starts_with_each_player_exile_until_nonland =
        EACH_PLAYER_EXILE_UNTIL_NONLAND_PATTERN.accepts_full(exile_clause);
    let starts_with_each_player_exile =
        if let Some(exile_match) = EACH_PLAYER_EXILE_TOP_CARD_PATTERN.parse_full(exile_clause) {
            exile_match
                .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, exile_clause)
                .is_some_and(|library_clause| {
                    PLAYER_LIBRARY_PATTERN
                        .locate_in(library_clause.trimmed())
                        .is_some()
                })
        } else {
            false
        };
    if !starts_with_each_player_exile && !starts_with_each_player_exile_until_nonland {
        return Ok(None);
    }

    let cast_clause = LexedClause::new(&cast_tokens).trimmed();
    let Some(cast_match) = CAST_ANY_NUMBER_FREE_PATTERN.parse_full(cast_clause) else {
        return Ok(None);
    };
    let Some(cast_scope_clause) =
        cast_match.capture_clause_by_role(effect_grammar::EffectCaptureRole::Object, cast_clause)
    else {
        return Ok(None);
    };
    let casts_any_number_from_those_cards = FROM_THOSE_OR_THEM_SCOPE_PATTERN
        .locate_in(cast_scope_clause.trimmed())
        .is_some();
    let casts_any_number_from_nonland_exiled_this_way = FROM_NONLAND_EXILED_THIS_WAY_PATTERN
        .locate_in(cast_scope_clause.trimmed())
        .is_some();

    if !casts_any_number_from_those_cards && !casts_any_number_from_nonland_exiled_this_way {
        return Ok(None);
    }

    let exiled_tag = crate::util::helper_tag_for_tokens(tokens, "exiled");
    let consult_match_tag = crate::util::helper_tag_for_tokens(tokens, "match");
    let consult_filter = ObjectFilter::nonland();
    let exile_effects = if starts_with_each_player_exile_until_nonland {
        vec![EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            crate::cards::builders::LibraryConsultModeAst::Exile,
            consult_filter,
            crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1)),
            crate::tag::TagRef::of(exiled_tag.clone()),
            crate::tag::TagRef::of(consult_match_tag.clone()),
        )]
    } else {
        vec![EffectAst::subject_verb_exile_top_of_library(
            PlayerAst::That,
            Value::Fixed(1),
            Vec::new(),
            vec![crate::tag::TagRef::of(exiled_tag.clone())],
        )]
    };

    let cast_filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
        if starts_with_each_player_exile_until_nonland {
            consult_match_tag
        } else {
            exiled_tag.clone()
        },
        TaggedOpbjectRelation::IsTaggedObject,
    );

    Ok(Some(vec![
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: exile_effects,
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachObject {
            filter: cast_filter,
            effects: vec![EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                )],
            })],
        }),
    ]))
}

fn parse_generic_meld_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let Some(matched) = MELD_RESULT_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(result_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let result_name = crate::lexer::render_token_slice(result_clause.tokens())
        .trim()
        .to_ascii_lowercase()
        .to_string();
    if result_name.is_empty() {
        let clause_display = crate::lexer::render_token_slice(tokens);
        return Err(CardTextError::ParseError(format!(
            "missing meld result name (clause: '{}')",
            clause_display.trim()
        )));
    }
    Ok(Some(EffectAst::subject_verb_meld(
        result_name,
        false,
        false,
    )))
}

pub fn parse_generic_control_combat_choices_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    // A preceding must-block clause can establish the creature set, leaving
    // only the assignment-control half of the normal combat-choice wording.
    // Runtime blocker control is the same reusable capability; the renderer
    // can recover the shorter anaphoric surface when the two effects remain
    // coordinated.
    if CONTROL_COMBAT_HOW_THOSE_BLOCK_PATTERN.accepts_full(clause) {
        return Ok(Some(
            EffectAst::subject_verb_control_combat_choices_this_turn(false, true),
        ));
    }
    let Some(matched) = CONTROL_COMBAT_CHOICES_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(action_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Action, clause)
    else {
        return Ok(None);
    };
    let Some(scope_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };

    let action_clause = action_clause.trimmed();
    let scope_clause = scope_clause.trimmed();
    let this_combat =
        crate::word_primitives::sequence_occurs(&scope_clause.words().to_word_refs(), &["combat"]);
    if CONTROL_COMBAT_ATTACK_ACTION_PATTERN.accepts_full(action_clause)
        && CONTROL_COMBAT_ATTACK_SCOPE_PATTERN.accepts_full(scope_clause)
    {
        Ok(Some(if this_combat {
            EffectAst::subject_verb_control_combat_choices(true, false, true)
        } else {
            EffectAst::subject_verb_control_combat_choices_this_turn(true, false)
        }))
    } else if CONTROL_COMBAT_BLOCK_ACTION_PATTERN.accepts_full(action_clause)
        && CONTROL_COMBAT_BLOCK_SCOPE_PATTERN.accepts_full(scope_clause)
    {
        Ok(Some(if this_combat {
            EffectAst::subject_verb_control_combat_choices(false, true, true)
        } else {
            EffectAst::subject_verb_control_combat_choices_this_turn(false, true)
        }))
    } else {
        Ok(None)
    }
}

pub fn parse_generic_damage_replacement_counters_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        effect_grammar::EffectSequence::word("if"),
        effect_grammar::EffectSequence::phrase(DAMAGE_REPLACEMENT_COUNTER_TARGET_PHRASE),
        effect_grammar::EffectSequence::object(
            "target",
            effect_grammar::EffectCaptureKind::UntilPhrase(
                DAMAGE_REPLACEMENT_COUNTER_DURATION_PHRASE,
            ),
        ),
        effect_grammar::EffectSequence::phrase(DAMAGE_REPLACEMENT_COUNTER_DURATION_PHRASE),
        effect_grammar::EffectSequence::phrase(DAMAGE_REPLACEMENT_COUNTER_PREVENT_PUT_PHRASE),
        effect_grammar::EffectSequence::any_word(&["counter", "counters"]),
        effect_grammar::EffectSequence::word("on"),
        effect_grammar::EffectSequence::any_phrase(DAMAGE_REPLACEMENT_COUNTER_RECIPIENT_PHRASES),
    ];
    let Some(matched) = effect_grammar::EffectSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let Some(target_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let target_tokens = target_clause.tokens();
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(target_tokens)?;

    Ok(Some(
        EffectAst::subject_verb_prevent_damage_to_target_put_counters(
            None,
            target,
            Until::EndOfTurn,
            CounterType::PlusOnePlusOne,
        ),
    ))
}

fn tokens_contain_relative_lesser_mana_value(tokens: &[OwnedLexToken]) -> bool {
    crate::lexer::contains_token_any_word(tokens, &["lesser", "less"])
        && crate::lexer::contains_token_word(tokens, "mana")
        && crate::lexer::contains_token_word(tokens, "value")
}

fn apply_lesser_mana_value_consult_constraint(tokens: &[OwnedLexToken], effects: &mut [EffectAst]) {
    if !tokens_contain_relative_lesser_mana_value(tokens) {
        return;
    }

    for effect in effects {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { filter, .. }) = &mut subject_verb.action
        else {
            continue;
        };
        if filter.mana_value.is_some() {
            continue;
        }
        let mut had_lesser_constraint = false;
        for constraint in &mut filter.tagged_constraints {
            if matches!(
                constraint.relation,
                TaggedOpbjectRelation::ManaValueLtTagged
            ) {
                constraint.tag = (crate::tag::CompilerReferenceTag::Sacrificed0.bind()).into();
                had_lesser_constraint = true;
            }
        }
        if had_lesser_constraint {
            continue;
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::Sacrificed0.bind()).into(),
            relation: TaggedOpbjectRelation::ManaValueLtTagged,
        });
    }
}

fn without_deferred_mana_value_clause(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = DEFERRED_MANA_VALUE_CLAUSE_PATTERN.parse_full(clause) else {
        return tokens.to_vec();
    };
    let Some(effect_range) = matched.capture_word_range("effect") else {
        return tokens.to_vec();
    };
    let Some(effect_end) = clause.token_index_after_words(effect_range.end) else {
        return tokens.to_vec();
    };
    trim_commas(&tokens[..effect_end]).to_vec()
}

pub fn parse_play_permission_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = PLAY_PERMISSION_GRAVEYARD_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(_duration_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };
    let Some(permission_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    let rest = trim_commas(permission_clause.tokens());
    let permission_clause = LexedClause::new(&rest).trimmed();
    if !PLAY_LANDS_CAST_SPELLS_GRAVEYARD_PATTERN.accepts_full(permission_clause) {
        return Ok(None);
    }

    Ok(Some(
        GenericPermissionProgram {
            player: PlayerAst::You,
            verb: GenericPermissionVerb::PlayAndCast,
            from_zone: Zone::Graveyard,
            duration: GenericPermissionDuration::UntilEndOfTurn,
        }
        .lower(),
    ))
}

pub fn parse_zone_replacement_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = ZONE_REPLACEMENT_GRAVEYARD_EXILE_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(condition_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Condition, clause)
    else {
        return Ok(None);
    };
    let Some(replacement_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    let condition_clause = condition_clause.trimmed();
    let Some(condition_match) =
        FUTURE_GRAVEYARD_EXILE_CONDITION_PATTERN.parse_full(condition_clause)
    else {
        return Ok(None);
    };
    let Some(destination_clause) = condition_match.capture_clause("destination", condition_clause)
    else {
        return Ok(None);
    };
    if !FUTURE_GRAVEYARD_DESTINATION_PATTERN.accepts_full(destination_clause.trimmed()) {
        return Ok(None);
    }

    if !EXILE_THAT_CARD_INSTEAD_PATTERN.accepts_full(replacement_clause.trimmed()) {
        return Ok(None);
    }

    Ok(Some(
        GenericZoneReplacementProgram {
            player: PlayerAst::You,
            from_zone: Zone::Graveyard,
            replacement_zone: Zone::Exile,
            duration: Until::EndOfTurn,
        }
        .lower(),
    ))
}

/// Whether the sentence has a choice complement's shape: a party or aggregate
/// complement, or a choice clause followed by the disposition of the rest.
pub fn is_choice_complement_shape(tokens: &[OwnedLexToken]) -> bool {
    if effect_grammar::parse_party_choice_complement_shape(tokens).is_some()
        || effect_grammar::parse_aggregate_choice_complement_shape(tokens).is_some()
    {
        return true;
    }
    let clause = LexedClause::new(tokens).trimmed();
    choice_complement_choice_clause_from_word_order(clause).is_some()
        || CHOICE_COMPLEMENT_PATTERN.parse_full(clause).is_some()
}

pub fn parse_choice_complement_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(shape) = effect_grammar::parse_party_choice_complement_shape(tokens) {
        return Ok(Some(
            GenericChoiceComplementProgram {
                chooser_scope: shape.chooser,
                base_filter: shape.filter,
                keep_tag: (crate::tag::CompilerReferenceTag::Keep.bind()).into(),
                keep_filters: shape.slot_filters,
                keep_count: shape.count_per_slot,
                distinct_slots: true,
                aggregate_constraint: None,
            }
            .lower(),
        ));
    }
    if let Some(shape) = effect_grammar::parse_aggregate_choice_complement_shape(tokens) {
        return Ok(Some(
            GenericChoiceComplementProgram {
                chooser_scope: shape.chooser,
                base_filter: shape.filter,
                keep_tag: (crate::tag::CompilerReferenceTag::Keep.bind()).into(),
                keep_filters: Vec::new(),
                keep_count: shape.count,
                distinct_slots: false,
                aggregate_constraint: Some(shape.constraint),
            }
            .lower(),
        ));
    }
    let clause = LexedClause::new(tokens).trimmed();
    let chooser_scope = if crate::word_primitives::parse_sequence_prefix(
        &clause.word_refs(),
        &["each", "opponent"],
    ) {
        PlayerAst::Opponent
    } else {
        PlayerAst::Any
    };
    let choice_clause =
        if let Some(choice_clause) = choice_complement_choice_clause_from_word_order(clause) {
            choice_clause
        } else if let Some(matched) = CHOICE_COMPLEMENT_PATTERN.parse_full(clause) {
            let Some(choice_clause) =
                matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Object, clause)
            else {
                return Ok(None);
            };
            choice_clause
        } else {
            return Ok(None);
        };
    let clause_display = crate::lexer::render_token_slice(clause.tokens())
        .trim()
        .to_string();

    let choice_clause = choice_clause.trimmed();
    let choice_tokens = choice_clause.tokens();
    if let Some(suffix) = choice_tokens.windows(4).position(|tokens| tokens[0].is_word("of") && tokens[1].is_word("each") && tokens[2].is_word("permanent") && tokens[3].is_word("type"))
        && choice_tokens[suffix + 4..].iter().all(|token| token.as_word().is_none())
    {
        let mut filter = parse_object_filter(&choice_tokens[..suffix], false)?;
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        return Ok(Some(GenericChoiceComplementProgram {
            chooser_scope, base_filter: filter,
            keep_tag: crate::tag::CompilerReferenceTag::Keep.bind().into(),
            keep_filters: [CardType::Artifact, CardType::Battle, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker]
                .into_iter().map(|kind| ObjectFilter::default().with_type(kind)).collect(),
            keep_count: ChoiceCount::exactly(1), distinct_slots: false, aggregate_constraint: None,
        }.lower()));
    }
    if find_from_among(choice_tokens).is_none()
        && !crate::lexer::token_word_refs(choice_tokens).windows(4).any(|words| words == ["of", "each", "permanent", "type"])
        && !choice_tokens.iter().any(|token| token.is_word("and"))
        && let Some((keep_count, count_used)) =
            crate::util::parse_choice_count_token_prefix_consumed(choice_tokens)
    {
        let base_tokens = trim_commas(choice_tokens.get(count_used..).unwrap_or_default());
        if !base_tokens.is_empty() {
            let mut base_filter = parse_object_filter(&base_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported counted filter in choose-and-sacrifice clause (clause: '{}')",
                    clause_display
                ))
            })?;
            if base_filter.controller.is_none() {
                base_filter.controller = Some(PlayerFilter::IteratedPlayer);
            }
            return Ok(Some(
                GenericChoiceComplementProgram {
                    chooser_scope,
                    base_filter,
                    keep_tag: (crate::tag::CompilerReferenceTag::Keep.bind()).into(),
                    keep_filters: vec![ObjectFilter::default()],
                    keep_count,
                    distinct_slots: false,
                    aggregate_constraint: None,
                }
                .lower(),
            ));
        }
    }
    let starts_with_from_among = find_from_among(choice_tokens) == Some(0);
    let (list_tokens, base_tokens) = if !starts_with_from_among
        && let Some(matched) = CHOICE_COMPLEMENT_LIST_FROM_AMONG_PATTERN.parse_full(choice_clause)
    {
        let Some(choice_list) = matched
            .capture_clause_by_role(effect_grammar::EffectCaptureRole::Object, choice_clause)
        else {
            return Ok(None);
        };
        let Some(base_filter) =
            matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, choice_clause)
        else {
            return Ok(None);
        };
        (choice_list.tokens(), base_filter.tokens())
    } else {
        let choose_tokens = choice_tokens;
        let Some(from_idx) = find_from_among(choose_tokens) else {
            return Ok(None);
        };
        if from_idx != 0 {
            return Ok(None);
        }
        let list_start = find_list_start(&choose_tokens[2..])
            .map(|idx| idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError("missing choice list after 'from among'".to_string())
            })?;
        (
            choose_tokens.get(list_start..).unwrap_or_default(),
            choose_tokens.get(2..list_start).unwrap_or_default(),
        )
    };

    let list_tokens = trim_commas(list_tokens);
    let base_tokens = trim_commas(base_tokens);
    if list_tokens.is_empty() || base_tokens.is_empty() {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&base_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported base filter in choose-and-sacrifice clause (clause: '{}')",
            clause_display
        ))
    })?;
    if base_filter.controller.is_none() {
        base_filter.controller = Some(PlayerFilter::IteratedPlayer);
    }

    let mut keep_filters = Vec::new();
    for segment in split_choose_list(&list_tokens) {
        let segment = strip_leading_articles(&segment);
        if segment.is_empty() {
            continue;
        }
        keep_filters.push(parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported choice filter in choose-and-sacrifice clause (clause: '{}')",
                clause_display
            ))
        })?);
    }
    if keep_filters.is_empty() {
        return Ok(None);
    }

    Ok(Some(
        GenericChoiceComplementProgram {
            chooser_scope,
            base_filter,
            keep_tag: (crate::tag::CompilerReferenceTag::Keep.bind()).into(),
            keep_filters,
            keep_count: ChoiceCount::exactly(1),
            distinct_slots: false,
            aggregate_constraint: None,
        }
        .lower(),
    ))
}

/// Parse the choice half of a split-sentence choice/complement program.
///
/// Oracle sometimes puts the choices and the complement action in separate
/// sentences ("For each player, you choose ... . Then each player sacrifices
/// all other ...").  The ordinary object-choice parser correctly identifies
/// the eligible union, but a comma-separated `and` list denotes one choice per
/// slot, not one object from the union.  Preserve those independent slots here
/// so the later cross-sentence correlation pass can link the chosen set to the
/// complement action.
pub fn parse_for_each_type_slot_choice_clause(
    tokens: &[OwnedLexToken],
    chooser: PlayerAst,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(choose_idx) = crate::slice_primitives::select_position(tokens, |token| {
        token
            .as_word()
            .is_some_and(|word| matches!(word, "choose" | "chooses"))
    }) else {
        return Ok(None);
    };
    if tokens[..choose_idx].iter().any(|token| {
        token
            .as_word()
            .is_some_and(|word| !matches!(word, "you" | "that" | "player" | "players"))
    }) {
        return Ok(None);
    }

    let choice_tokens = trim_commas(&tokens[choose_idx + 1..]);
    if find_from_among(&choice_tokens) != Some(0)
        || !choice_tokens.iter().any(|token| token.is_word("and"))
        || choice_tokens
            .iter()
            .any(|token| token.is_word("or") || token.is_word("and/or"))
    {
        return Ok(None);
    }
    let Some(list_start) = find_list_start(&choice_tokens[2..]).map(|idx| idx + 2) else {
        return Ok(None);
    };
    let base_tokens = trim_commas(choice_tokens.get(2..list_start).unwrap_or_default());
    let list_tokens = trim_commas(choice_tokens.get(list_start..).unwrap_or_default());
    if base_tokens.is_empty() || list_tokens.is_empty() {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&base_tokens, false)?;
    if base_filter.controller.is_none() {
        base_filter.controller = Some(PlayerFilter::IteratedPlayer);
    }
    let keep_tag = crate::tag::CompilerReferenceTag::ChosenForEachPlayer.bind();
    let mut choices = Vec::new();
    for segment in split_choose_list(&list_tokens) {
        let segment = strip_leading_articles(&segment);
        if segment.is_empty() {
            continue;
        }
        let slot_filter = parse_object_filter(&segment, false)?;
        // This route is for independent type slots.  Leave descriptive lists
        // with compound constraints to the ordinary single-choice parser.
        if slot_filter.card_types.len() + slot_filter.subtypes.len() != 1 {
            return Ok(None);
        }
        choices.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            // A multitype permanent may represent more than one slot. The
            // shared tag is an accumulating kept set for the later
            // complement action, not an exclusion from subsequent choices.
            filter: merge_filters(&base_filter, &slot_filter),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: chooser,
            tag: keep_tag.clone(),
        }));
    }
    if choices.len() < 2 {
        return Ok(None);
    }
    Ok(Some(choices))
}

pub fn parse_triggered_spell_opponent_damage_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_triggered_spell_opponent_damage_shape(tokens) else {
        return Ok(None);
    };
    let triggering_spell =
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None);
    Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
        effects: vec![EffectAst::subject_verb_damage_with_source(
            triggering_spell,
            shape.amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    })))
}

fn choice_complement_choice_clause_from_word_order<'a>(
    clause: LexedClause<'a>,
) -> Option<LexedClause<'a>> {
    effect_grammar::parse_choice_complement_clause(clause.tokens())
}

pub fn parse_vote_affinity_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(shape) = effect_grammar::parse_voted_against_you_effects_shape(tokens) {
        let effect_tokens = trim_commas(shape.effect_tokens);
        let effects = parse_effect_chain_lexed(&effect_tokens)?;
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer {
            tag: crate::tag::CompilerReferenceTag::VotedAgainstYou.bind(),
            effects,
        })]));
    }
    parse_you_and_each_opponent_voted_with_you_sentence(tokens)
}

#[path = "generic_subject_verb/vote_readings.rs"]
mod vote_readings;

pub fn parse_vote_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let input = vote_readings::VoteSentence { tokens };
    match vote_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }

    Ok(None)
}

fn vote_options_clause_before_reveal_tail<'a>(options_clause: LexedClause<'a>) -> LexedClause<'a> {
    let options_clause = options_clause.trimmed();
    if let Some(matched) = VOTE_REVEAL_TAIL_PATTERN.locate_in(options_clause) {
        return options_clause
            .between_word_range(0, matched.word_range.start)
            .map(LexedClause::trimmed)
            .unwrap_or_else(|| LexedClause::new(&[]));
    }
    options_clause
}

fn split_vote_option_clauses<'a>(options_clause: LexedClause<'a>) -> Vec<LexedClause<'a>> {
    let mut clauses = Vec::new();
    let mut tail = options_clause.trimmed();
    while let Some(matched) = VOTE_OPTION_DELIMITER_PATTERN.locate_in(tail) {
        if let Some(option_clause) = tail
            .between_word_range(0, matched.word_range.start)
            .map(LexedClause::trimmed)
            .filter(|clause| !clause.is_empty())
        {
            clauses.extend(split_vote_option_clause_on_commas(option_clause));
        }
        tail = tail
            .after_words(matched.word_range.end)
            .map(LexedClause::trimmed)
            .unwrap_or_else(|| LexedClause::new(&[]));
    }
    let tail = tail.trimmed();
    if !tail.is_empty() {
        clauses.extend(split_vote_option_clause_on_commas(tail));
    }
    clauses
}

fn split_vote_option_clause_on_commas<'a>(clause: LexedClause<'a>) -> Vec<LexedClause<'a>> {
    let tokens = clause.tokens();
    let mut clauses = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            continue;
        }
        let segment = LexedClause::new(&tokens[start..idx]).trimmed();
        if !segment.is_empty() {
            clauses.push(segment);
        }
        start = idx + 1;
    }
    let tail = LexedClause::new(&tokens[start..]).trimmed();
    if !tail.is_empty() {
        clauses.push(tail);
    }
    clauses
}

fn vote_options_clause_looks_like_target_choice(clause: LexedClause<'_>) -> bool {
    effect_grammar::vote_options_tokens_look_like_target_choice(clause.tokens())
}

fn named_vote_options_from_clause(option_clause: LexedClause<'_>) -> Option<Vec<String>> {
    if vote_options_clause_looks_like_target_choice(option_clause) {
        return None;
    }
    let options = split_vote_option_clauses(option_clause)
        .into_iter()
        .filter_map(captured_non_article_label)
        .collect::<Vec<_>>();
    (options.len() >= 2).then_some(options)
}

pub(super) fn parse_vote_reveal_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    if VOTE_REVEAL_PATTERN
        .parse_full(LexedClause::new(tokens).trimmed())
        .is_some()
    {
        return Some(EffectAst::Votes(VoteEffectAst::SecretChoiceReveal));
    }
    None
}

pub(super) fn parse_secret_number_choice_vote_start(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = SECRET_NUMBER_CHOICE_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(participants_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(options_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    if !SECRET_CHOICE_PARTICIPANTS_PATTERN.accepts_full(participants_clause.trimmed()) {
        return Ok(None);
    }

    let option_clause = vote_options_clause_before_reveal_tail(options_clause);
    let options = split_vote_option_clauses(option_clause)
        .into_iter()
        .filter_map(captured_numeric_label)
        .collect::<Vec<_>>();
    // The participant prefix is shared by object-choice programs (for
    // example, "each secretly choose a creature"). Numeric-choice ownership
    // begins only once this rule has captured at least one numeric option;
    // otherwise this is a structural no-match for the sibling object rule.
    if options.is_empty() {
        return Ok(None);
    }
    if options.len() < 2 {
        return Err(CardTextError::ParseError(
            "secret choice clause requires at least two numeric options".to_string(),
        ));
    }

    Ok(Some(EffectAst::Votes(VoteEffectAst::SecretChoiceStart {
        options,
        participants: vec![PlayerFilter::You, PlayerFilter::target_opponent()],
        object_choice: None,
    })))
}

const EXILE_COUNTED_FACE_DOWN_OBJECT_PHRASES: &[&[&str]] = &[
    &["of", "them"],
    &["them"],
    &["of", "those", "card"],
    &["of", "those", "cards"],
    &["those", "card"],
    &["those", "cards"],
];
const EXILE_COUNTED_FACE_DOWN_COUNT_BOUNDARIES: &[&[&str]] = &[
    &["of", "them"],
    &["them"],
    &["of", "those", "card"],
    &["of", "those", "cards"],
    &["those", "card"],
    &["those", "cards"],
    &["face", "down"],
];
const OPTIONAL_EXILE_COUNTED_FACE_DOWN_OBJECT_ATOMS: &[effect_grammar::EffectAtom<'static>] =
    &[effect_grammar::EffectSequence::any_phrase(
        EXILE_COUNTED_FACE_DOWN_OBJECT_PHRASES,
    )];
const LOOK_EXILE_COUNTED_FACE_DOWN_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::capture(
            "look_clause",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["exile"]),
        ),
        effect_grammar::EffectSequence::word("exile"),
        effect_grammar::EffectSequence::amount(
            "exile_count",
            effect_grammar::EffectCaptureKind::UntilAnyPhrase(
                EXILE_COUNTED_FACE_DOWN_COUNT_BOUNDARIES,
            ),
        ),
        // Oracle normally says "one of them", but some linked-exile cards
        // omit the explicit looked-set pronoun ("exile one face down"). The
        // leading look clause still supplies the unique candidate pool, so
        // both surfaces lower to the same counted selection.
        effect_grammar::EffectSequence::optional(OPTIONAL_EXILE_COUNTED_FACE_DOWN_OBJECT_ATOMS),
        effect_grammar::EffectSequence::phrase(&["face", "down"]),
        effect_grammar::EffectSequence::tail("remainder", effect_grammar::EffectCaptureKind::Rest),
    ]);
const EXILE_FACE_DOWN_REST_BOTTOM_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("put"),
        effect_grammar::EffectSequence::optional(OPTIONAL_THE_PATTERN_ATOMS),
        effect_grammar::EffectSequence::any_word(&["rest", "other"]),
        effect_grammar::EffectSequence::any_word(&["on", "onto"]),
        effect_grammar::EffectSequence::optional(OPTIONAL_THE_PATTERN_ATOMS),
        effect_grammar::EffectSequence::word("bottom"),
    ]);
const EXILE_FACE_DOWN_REST_LIBRARY_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::object(
        "zone",
        effect_grammar::EffectCaptureKind::OneOf(&["library", "libraries"]),
    )]);
const EXILE_FACE_DOWN_REST_RANDOM_ORDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::modifier(
        "order",
        effect_grammar::EffectCaptureKind::OneOfPhrase(&[&["random", "order"]]),
    )]);
const EXILE_FACE_DOWN_REST_ANY_ORDER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::modifier(
        "order",
        effect_grammar::EffectCaptureKind::OneOfPhrase(&[&["any", "order"]]),
    )]);
const EXILE_FACE_DOWN_COUNTER_MODIFIER_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[
        effect_grammar::EffectSequence::word("with"),
        effect_grammar::EffectSequence::modifier(
            "counter_descriptor",
            effect_grammar::EffectCaptureKind::UntilPhrase(&["on", "it"]),
        ),
        effect_grammar::EffectSequence::phrase(&["on", "it"]),
        effect_grammar::EffectSequence::optional(OPTIONAL_THEN_PATTERN_ATOMS),
        effect_grammar::EffectSequence::tail("remainder", effect_grammar::EffectCaptureKind::Rest),
    ]);

#[cfg(test)]
#[path = "generic_subject_verb_programs_inline_generic_subject_verb_program_tests.rs"]
mod generic_subject_verb_program_tests;

#[path = "generic_subject_verb_programs/library.rs"]
mod library_programs;
pub use library_programs::{
    parse_generic_top_cards_cloak_counted_rest_bottom_subject_verb,
    parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb,
};
#[path = "generic_subject_verb_programs/choice.rs"]
mod choice_programs;
use choice_programs::{
    parse_generic_extra_vote, parse_generic_player_vote_received_effects,
    parse_generic_vote_option_effects, parse_generic_vote_start,
};
#[path = "generic_subject_verb_programs/core.rs"]
mod core_programs;
use core_programs::{captured_non_article_label, captured_numeric_label};
#[path = "generic_subject_verb_programs/object_action.rs"]
mod object_action_programs;
use object_action_programs::captured_non_article_tokens;
