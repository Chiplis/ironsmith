#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericPermissionVerb {
    PlayAndCast,
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
    aggregate_constraint: Option<crate::effect::ChoiceAggregateConstraint>,
}

impl GenericChoiceComplementProgram {
    fn lower(self) -> EffectAst {
        let mut effects = Vec::new();
        if let Some(constraint) = self.aggregate_constraint {
            effects.push(EffectAst::ChooseObjectsWithAggregateConstraint {
                filter: self.base_filter.clone().not_tagged(self.keep_tag.clone()),
                count: self.keep_count,
                player: PlayerAst::That,
                tag: self.keep_tag.clone(),
                constraint,
            });
        } else {
            for keep_filter in self.keep_filters {
                let mut filter = merge_filters(&self.base_filter, &keep_filter);
                filter = filter.not_tagged(self.keep_tag.clone());
                effects.push(EffectAst::ChooseObjects {
                    filter,
                    count: self.keep_count,
                    count_value: None,
                    player: PlayerAst::That,
                    tag: self.keep_tag.clone(),
                });
            }
        }
        effects.push(EffectAst::subject_verb_sacrifice_all(
            PlayerAst::That,
            self.base_filter.not_tagged(self.keep_tag),
        ));
        match self.chooser_scope {
            PlayerAst::Any | PlayerAst::Implicit => EffectAst::ForEachPlayer { effects },
            _ => EffectAst::ForEachPlayer { effects },
        }
    }
}

#[derive(Debug, Clone)]
struct TargetControlledPumpProgram {
    filter: ObjectFilter,
    power: Value,
    toughness: Value,
    abilities: Vec<GrantedAbilityAst>,
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
                self.filter,
                self.abilities,
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
            Self::Start { options, secret } => EffectAst::VoteStart { options, secret },
            Self::OptionEffects { option, effects } => EffectAst::VoteOption { option, effects },
            Self::Extra { count, optional } => EffectAst::VoteExtra { count, optional },
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
const FUTURE_GRAVEYARD_DESTINATION_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(&[
        "into",
        "your",
        "graveyard",
    ])]);
const EXILE_THAT_CARD_INSTEAD_PATTERN: effect_grammar::EffectSequence<'static> =
    effect_grammar::EffectSequence::new(&[effect_grammar::EffectSequence::phrase(
        EXILE_THAT_CARD_INSTEAD_PHRASE,
    )]);
const EACH_PLAYER_PHRASES: &[&[&str]] = &[&["each", "player"]];
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

fn parse_any_player_may_have_source_deal_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_any_player_source_damage(tokens) else {
        return Ok(None);
    };
    let deal_tail = trim_edge_punctuation(shape.damage_tokens);
    let Some((amount, used)) = parse_value(&deal_tail) else {
        return Ok(None);
    };
    if !deal_tail
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "damage")
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

    Ok(Some(vec![EffectAst::MayByPlayer {
        player: shape.player,
        effects: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(shape.player_filter, None),
        )],
    }]))
}

pub(crate) fn parse_top_level_subject_verb_recognition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    if let Some(effects) = parse_any_player_may_have_source_deal_damage(tokens)? {
        return Ok(Some((
            "subject-verb verb=Deal subject=source recognizer=any-player-may-have-source-damage",
            effects,
        )));
    }
    if let Some(effect) = parse_generic_play_exiled_cards_for_as_long_as_exiled(tokens) {
        return Ok(Some((
            "subject-verb verb=Play subject=implicit recognizer=exiled-cards-play-permission",
            vec![effect],
        )));
    }
    if let Some(effect) = parse_generic_mana_any_type_cast_tagged_this_way(tokens) {
        return Ok(Some((
            "subject-verb verb=Cast subject=implicit recognizer=tagged-any-mana-permission",
            vec![effect],
        )));
    }
    if let Some(effects) = parse_source_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=source recognizer=source-pump-unblockable",
            effects,
        )));
    }
    if let Some(effects) = parse_target_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=target recognizer=target-pump-unblockable",
            effects,
        )));
    }
    if let Some(effects) = parse_source_gets_filter_gains_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=source recognizer=source-pump-filter-gain",
            effects,
        )));
    }
    if let Some(effects) = parse_target_player_controls_get_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=target-player-controls recognizer=embedded-controller-pump",
            effects,
        )));
    }
    if let Some(effects) = parse_target_gains_then_gets_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Gain subject=target recognizer=shared-subject-gain-get",
            effects,
        )));
    }
    if let Some(effects) = parse_target_gets_then_gains_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=target recognizer=shared-subject-get-gain",
            effects,
        )));
    }
    if let Some(effects) = parse_target_has_base_pt_then_loses_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Have subject=target recognizer=shared-subject-base-pt-lose",
            effects,
        )));
    }

    let program = if let Some(effect) = parse_generic_meld_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::Meld { effect })
    } else if let Some(effect) = parse_generic_control_combat_choices_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::ControlCombatChoices { effect })
    } else if let Some(effect) = parse_generic_damage_replacement_counters_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::PreventDamageAndPutCounters { effect })
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
    } else {
        if has_where_x_value_binding(tokens) {
            let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            Some(GenericTopLevelProgram::ValueBinding { effects })
        } else {
            None
        }
    };

    Ok(program.map(|program| {
        let route = program.route();
        (route, program.lower())
    }))
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
            TagKey::from(IT_TAG),
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
            TagKey::from(IT_TAG),
            PlayerAst::You,
            false,
            false,
            true,
            None,
        )
    })
}

fn parse_source_gets_unblockable_subject_verb(
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

fn parse_target_gets_unblockable_subject_verb(
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
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        blocked_filter = blocked_filter
            .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
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
        let attached_tag = match shape.subject {
            effect_grammar::gain_ability_shapes::AttachedReferenceSubject::EnchantedCreature => {
                "enchanted"
            }
            effect_grammar::gain_ability_shapes::AttachedReferenceSubject::EquippedCreature => {
                "equipped"
            }
        };
        let attached = ObjectFilter::creature()
            .match_tagged(attached_tag, TaggedOpbjectRelation::IsTaggedObject);
        let related = ObjectFilter::creature()
            .not_tagged(attached_tag)
            .shares_subtype_with_tagged(attached_tag);
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.any_of = vec![attached, related];
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
    super::gain_ability::parse_gain_ability_sentence_with_typed_subject(
        tokens,
        shape.subject_tokens,
    )
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
    if let Some(tail_match) = TARGET_CONTROLLED_PUMP_GRANTED_ABILITY_PATTERN.parse_full(tail_clause)
    {
        let Some(ability_clause) =
            tail_match.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, tail_clause)
        else {
            return Ok(None);
        };
        abilities.extend(keyword_abilities_from_clause(ability_clause.trimmed()));
    }
    Ok(Some(TargetControlledPumpProgram {
        filter,
        power,
        toughness,
        abilities,
    }))
}

fn keyword_abilities_from_clause(ability_clause: LexedClause<'_>) -> Vec<GrantedAbilityAst> {
    let ability_clause = ability_clause.trimmed();
    let mut abilities = Vec::new();
    if ABILITY_FIRST_STRIKE_PATTERN
        .locate_in(ability_clause)
        .is_some()
    {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::FirstStrike));
    }
    if ABILITY_HASTE_PATTERN.locate_in(ability_clause).is_some() {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
    }
    if ABILITY_TRAMPLE_PATTERN.locate_in(ability_clause).is_some() {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Trample));
    }
    abilities
}

fn parse_pt_modifier_capture(clause: LexedClause<'_>) -> Option<(Value, Value)> {
    let modifier_word = clause
        .trimmed()
        .tokens()
        .first()
        .and_then(OwnedLexToken::as_word)?;
    crate::runtime_backend::keyword_static::parse_pt_modifier_values(modifier_word).ok()
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

pub(crate) fn parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(
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
        crate::runtime_backend::grammar::values::parse_number_prefix_lexed(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    let hand_owner_clause = matched.capture_clause("hand_owner", tail_clause)?;
    let chooser = put_counted_top_cards_owner(hand_owner_clause, player)?;
    let graveyard_owner_clause = matched.capture_clause("graveyard_owner", tail_clause)?;
    put_counted_top_cards_owner(graveyard_owner_clause, player)?;

    let looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        &prefix_tokens,
        "revealed",
    );
    let chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        &tail_tokens,
        "chosen",
    );
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.extend(EffectAst::compose_put_some_into_hand_rest_into_graveyard(
        chooser,
        crate::effect::ChoiceCount::exactly(put_count as usize),
        looked_tag,
        chosen_tag,
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
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
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
        TargetAst::Tagged(parts.all_tag, None),
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
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
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
        TargetAst::Tagged(parts.all_tag, None),
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
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }
    apply_lesser_mana_value_consult_constraint(&sentence_tokens, &mut parts.effects);
    Ok(Some(parts.effects))
}

fn parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(
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
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
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
        TargetAst::Tagged(parts.match_tag.clone(), None),
        Zone::Battlefield,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        None,
    ));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag,
            Some(parts.match_tag),
            order,
            parts.player,
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

    let exiled_tag = crate::runtime_backend::util::helper_tag_for_tokens(tokens, "exiled");
    let consult_match_tag = crate::runtime_backend::util::helper_tag_for_tokens(tokens, "match");
    let consult_filter = ObjectFilter::nonland();
    let exile_effects = if starts_with_each_player_exile_until_nonland {
        vec![EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            crate::cards::builders::LibraryConsultModeAst::Exile,
            consult_filter,
            crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1)),
            exiled_tag.clone(),
            consult_match_tag.clone(),
        )]
    } else {
        vec![EffectAst::subject_verb_exile_top_of_library(
            PlayerAst::That,
            Value::Fixed(1),
            Vec::new(),
            vec![exiled_tag.clone()],
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
        EffectAst::ForEachPlayer {
            effects: exile_effects,
        },
        EffectAst::ForEachObject {
            filter: cast_filter,
            effects: vec![EffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    TagKey::from(IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                )],
            }],
        },
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
    let result_name = crate::runtime_backend::lexer::render_token_slice(result_clause.tokens())
        .trim()
        .to_ascii_lowercase()
        .to_string();
    if result_name.is_empty() {
        let clause_display = crate::runtime_backend::lexer::render_token_slice(tokens);
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

fn parse_generic_control_combat_choices_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
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
    let this_combat = scope_clause
        .words()
        .to_word_refs()
        .iter()
        .any(|word| *word == "combat");
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

fn parse_generic_damage_replacement_counters_subject_verb(
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
    crate::runtime_backend::lexer::contains_token_any_word(tokens, &["lesser", "less"])
        && crate::runtime_backend::lexer::contains_token_word(tokens, "mana")
        && crate::runtime_backend::lexer::contains_token_word(tokens, "value")
}

fn apply_lesser_mana_value_consult_constraint(tokens: &[OwnedLexToken], effects: &mut [EffectAst]) {
    if !tokens_contain_relative_lesser_mana_value(tokens) {
        return;
    }

    for effect in effects {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. } = &mut subject_verb.action
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
                constraint.tag = TagKey::from("sacrificed_0");
                had_lesser_constraint = true;
            }
        }
        if had_lesser_constraint {
            continue;
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrificed_0"),
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

pub(crate) fn parse_play_permission_subject_verb(
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

pub(crate) fn parse_zone_replacement_subject_verb(
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

pub(crate) fn parse_choice_complement_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(shape) = effect_grammar::parse_party_choice_complement_shape(tokens) {
        return Ok(Some(
            GenericChoiceComplementProgram {
                chooser_scope: shape.chooser,
                base_filter: shape.filter,
                keep_tag: TagKey::from("keep"),
                keep_filters: shape.slot_filters,
                keep_count: shape.count_per_slot,
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
                keep_tag: TagKey::from("keep"),
                keep_filters: Vec::new(),
                keep_count: shape.count,
                aggregate_constraint: Some(shape.constraint),
            }
            .lower(),
        ));
    }
    let clause = LexedClause::new(tokens).trimmed();
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
    let clause_display = crate::runtime_backend::lexer::render_token_slice(clause.tokens())
        .trim()
        .to_string();

    let choice_clause = choice_clause.trimmed();
    let choice_tokens = choice_clause.tokens();
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
            chooser_scope: PlayerAst::Any,
            base_filter,
            keep_tag: TagKey::from("keep"),
            keep_filters,
            keep_count: ChoiceCount::exactly(1),
            aggregate_constraint: None,
        }
        .lower(),
    ))
}

pub(crate) fn parse_triggered_spell_opponent_damage_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_triggered_spell_opponent_damage_shape(tokens) else {
        return Ok(None);
    };
    let triggering_spell = TargetAst::Tagged(TagKey::from("triggering"), None);
    Ok(Some(EffectAst::ForEachOpponent {
        effects: vec![EffectAst::subject_verb_damage_with_source(
            triggering_spell,
            shape.amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    }))
}

fn choice_complement_choice_clause_from_word_order<'a>(
    clause: LexedClause<'a>,
) -> Option<LexedClause<'a>> {
    effect_grammar::parse_choice_complement_clause(clause.tokens())
}

pub(crate) fn parse_vote_affinity_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_each_opponent_voted_with_you_sentence(tokens)
}

pub(crate) fn parse_vote_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_secret_number_choice_vote_start(tokens)? {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_vote_reveal_sentence(tokens) {
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_generic_vote_start(tokens)? {
        if let EffectAst::VoteStart { options, secret } = effect {
            return Ok(Some(GenericVoteProgram::Start { options, secret }.lower()));
        }
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_generic_vote_option_effects(tokens)? {
        if let EffectAst::VoteOption { option, effects } = effect {
            return Ok(Some(
                GenericVoteProgram::OptionEffects { option, effects }.lower(),
            ));
        }
        return Ok(Some(effect));
    }
    if let Some(effect) = parse_generic_extra_vote(tokens) {
        if let EffectAst::VoteExtra { count, optional } = effect {
            return Ok(Some(GenericVoteProgram::Extra { count, optional }.lower()));
        }
        return Ok(Some(effect));
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

fn parse_vote_reveal_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    if VOTE_REVEAL_PATTERN
        .parse_full(LexedClause::new(tokens).trimmed())
        .is_some()
    {
        return Some(EffectAst::SecretChoiceReveal);
    }
    None
}

fn parse_secret_number_choice_vote_start(
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
    if options.len() < 2 {
        return Err(CardTextError::ParseError(
            "secret choice clause requires at least two numeric options".to_string(),
        ));
    }

    Ok(Some(EffectAst::SecretChoiceStart {
        options,
        participants: vec![PlayerFilter::You, PlayerFilter::target_opponent()],
    }))
}

fn parse_generic_vote_start(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = GENERIC_VOTE_START_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(voters_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(options_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };

    let voters_clause = voters_clause.trimmed();
    if EACH_PLAYER_VOTER_PATTERN.locate_in(voters_clause).is_none() {
        return Ok(None);
    }
    let secret = SECRET_VOTER_PATTERN.locate_in(voters_clause).is_some();

    let option_clause = vote_options_clause_before_reveal_tail(options_clause);
    if let Some(options) = named_vote_options_from_clause(option_clause) {
        return Ok(Some(EffectAst::VoteStart { options, secret }));
    }

    let option_tokens = option_clause.tokens().to_vec();
    if let Ok(target) = parse_target_phrase(&option_tokens) {
        match target {
            TargetAst::Player(filter, _) => {
                let exclude_voter = option_clause
                    .first_word()
                    .is_some_and(|word| matches!(word, "other" | "another"));
                let filter = if exclude_voter && matches!(filter, PlayerFilter::NotYou) {
                    PlayerFilter::Any
                } else {
                    filter
                };
                return Ok(Some(EffectAst::VoteStartPlayers {
                    filter,
                    exclude_voter,
                    secret,
                }));
            }
            TargetAst::Object(filter, _, _) => {
                return Ok(Some(EffectAst::VoteStartObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                    secret,
                }));
            }
            TargetAst::WithCount(inner, count) => {
                if let TargetAst::Object(filter, _, _) = *inner {
                    return Ok(Some(EffectAst::VoteStartObjects {
                        filter,
                        count,
                        secret,
                    }));
                }
            }
            _ => {}
        }
    }
    if let Ok(filter) = parse_object_filter_lexed(&option_tokens, false)
        && filter != ObjectFilter::default()
    {
        return Ok(Some(EffectAst::VoteStartObjects {
            filter,
            count: ChoiceCount::exactly(1),
            secret,
        }));
    }

    let Some(options) = named_vote_options_from_clause(option_clause) else {
        return Err(CardTextError::ParseError(
            "vote clause requires at least two options".to_string(),
        ));
    };

    Ok(Some(EffectAst::VoteStart { options, secret }))
}

fn parse_generic_vote_option_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = parse_generic_player_vote_received_effects(tokens)? {
        return Ok(Some(effect));
    }

    let Some(shape) = effect_grammar::parse_named_vote_option_effects_shape(tokens) else {
        return Ok(None);
    };
    let option_clause = LexedClause::new(shape.option_tokens);
    let Some(option) = captured_non_article_label(option_clause) else {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    };

    let effect_tokens = trim_commas(shape.effect_tokens);
    let effects = parse_effect_chain_lexed(&effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}

fn parse_generic_player_vote_received_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    let Some(matched) = GENERIC_PLAYER_VOTE_RECEIVED_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let Some(player_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Subject, clause)
    else {
        return Ok(None);
    };
    let Some(effect_clause) =
        matched.capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, clause)
    else {
        return Ok(None);
    };
    let player_tokens = captured_non_article_tokens(player_clause);
    if player_tokens.is_empty() {
        return Ok(None);
    }
    let TargetAst::Player(filter, _) = parse_target_phrase(&player_tokens)? else {
        return Ok(None);
    };
    let effect_tokens = trim_commas(effect_clause.tokens());
    let effects = parse_effect_chain_lexed(&effect_tokens)?;
    if filter == PlayerFilter::You {
        return Ok(Some(EffectAst::RepeatEffects {
            count: Value::PlayerVoteCount(PlayerFilter::You),
            effects,
        }));
    }
    Ok(Some(EffectAst::ForEachPlayersFiltered {
        filter,
        effects: vec![EffectAst::RepeatEffects {
            count: Value::PlayerVoteCount(PlayerFilter::IteratedPlayer),
            effects,
        }],
    }))
}

fn captured_non_article_tokens(clause: LexedClause<'_>) -> Vec<OwnedLexToken> {
    clause
        .trimmed()
        .tokens()
        .iter()
        .filter(|token| token.as_word().is_none_or(|word| !is_article(word)))
        .cloned()
        .collect()
}

fn captured_non_article_label(clause: LexedClause<'_>) -> Option<String> {
    let tokens = captured_non_article_tokens(clause);
    (!tokens.is_empty()).then(|| render_token_slice(&tokens).trim().to_string())
}

fn captured_numeric_label(clause: LexedClause<'_>) -> Option<String> {
    let tokens = captured_non_article_tokens(clause);
    if tokens.len() == 1
        && let Some(word) = tokens[0].as_word()
        && word.chars().all(|ch| ch.is_ascii_digit())
    {
        return Some(word.to_string());
    }
    None
}

fn parse_generic_extra_vote(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let clause = LexedClause::new(tokens).trimmed();
    if OPTIONAL_EXTRA_VOTE_PATTERN.parse_full(clause).is_some() {
        return Some(EffectAst::VoteExtra {
            count: 1,
            optional: true,
        });
    }
    if REQUIRED_EXTRA_VOTE_PATTERN.parse_full(clause).is_some() {
        return Some(EffectAst::VoteExtra {
            count: 1,
            optional: false,
        });
    }
    None
}

const EXILE_COUNTED_FACE_DOWN_OBJECT_PHRASES: &[&[&str]] = &[
    &["of", "them"],
    &["them"],
    &["of", "those", "card"],
    &["of", "those", "cards"],
    &["those", "card"],
    &["those", "cards"],
];
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
                EXILE_COUNTED_FACE_DOWN_OBJECT_PHRASES,
            ),
        ),
        effect_grammar::EffectSequence::any_phrase(EXILE_COUNTED_FACE_DOWN_OBJECT_PHRASES),
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

pub(crate) fn parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_clause = LexedClause::new(&sentence_tokens).trimmed();
    let matched = LOOK_EXILE_COUNTED_FACE_DOWN_PATTERN.parse_full(sentence_clause)?;
    let look_clause = matched
        .capture_clause("look_clause", sentence_clause)?
        .trimmed();
    let look_effect = super::verb_handlers::parse_look(look_clause.tokens(), None).ok()?;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    }) = look_effect
    else {
        return None;
    };

    let count_clause = matched
        .capture_clause("exile_count", sentence_clause)?
        .trimmed();
    let count_tokens = trim_commas(count_clause.tokens());
    let (exile_count, _used) =
        crate::runtime_backend::util::parse_choice_count_token_prefix_consumed(&count_tokens)?;

    let remainder_clause = matched
        .capture_clause_by_role(effect_grammar::EffectCaptureRole::Tail, sentence_clause)?
        .trimmed();
    if EXILE_FACE_DOWN_REST_BOTTOM_PATTERN
        .locate_in(remainder_clause)
        .is_none()
        || EXILE_FACE_DOWN_REST_LIBRARY_PATTERN
            .locate_in(remainder_clause)
            .is_none()
    {
        return None;
    }
    let singleton_remainder = matches!(count.unhinted(), Value::Fixed(2))
        && exile_count.min == 1
        && exile_count.max == Some(1)
        && !exile_count.dynamic_x
        && !exile_count.up_to_x
        && !exile_count.random;
    let bottom_order = if EXILE_FACE_DOWN_REST_RANDOM_ORDER_PATTERN
        .locate_in(remainder_clause)
        .is_some()
    {
        crate::cards::builders::LibraryBottomOrderAst::Random
    } else if EXILE_FACE_DOWN_REST_ANY_ORDER_PATTERN
        .locate_in(remainder_clause)
        .is_some()
    {
        crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
    } else if singleton_remainder {
        // Ordering a one-card complement is meaningless, so Oracle omits an
        // order clause ("put the other on the bottom of that library").  The
        // runtime still uses the ordinary chooser-order primitive; with one
        // card it has exactly one legal ordering.
        crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
    } else {
        return None;
    };

    let looked_tag = crate::runtime_backend::util::helper_tag_for_tokens(tokens, "looked");
    let exiled_tag = TagKey::from(IT_TAG);
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player.clone(), count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: exile_count,
            count_value: None,
            player: PlayerAst::You,
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag),
            bottom_order,
            player,
        ),
    ])
}

#[cfg(test)]
mod generic_subject_verb_program_tests {
    use super::*;
    use crate::Subtype;

    #[test]
    fn top_cards_counted_hand_remainder_uses_captured_owners() {
        let tokens = crate::runtime_backend::lex_line(
            "look at the top three cards of your library, then put one of those cards into that player's hand and the rest into that player's graveyard.",
            0,
        )
        .expect("rewrite lexer should classify looked-card bundle");
        let effects =
            parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(&tokens)
                .expect("top-card hand/remainder parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("LookAtTopCards"), "{debug}");
        assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
        assert!(debug.contains("MoveTaggedGroupToZone"), "{debug}");
        assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
        assert!(debug.contains("player: That"), "{debug}");
        assert!(!debug.contains("Unsupported"), "{debug}");
    }

    #[test]
    fn counted_face_down_exile_keeps_target_opponents_library_owner() {
        let tokens = crate::runtime_backend::lex_line(
            "Look at the top nine cards of target opponent's library, exile two of them face down, then put the rest on the bottom of their library in a random order.",
            0,
        )
        .expect("counted face-down exile bundle should lex");
        let effects =
            parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&tokens)
                .expect("counted face-down exile bundle should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("LookAtTopCards"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
            "{debug}"
        );
        assert!(
            debug.matches("player: TargetOpponent").count() >= 2,
            "{debug}"
        );
    }

    #[test]
    fn two_card_face_down_partition_accepts_the_single_other_without_order_text() {
        let tokens = crate::runtime_backend::lex_line(
            "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library.",
            0,
        )
        .expect("two-card face-down partition should lex");
        let effects =
            parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&tokens)
                .expect("two-card face-down partition should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("LookAtTopCards"), "{debug}");
        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("PutTaggedRemainderOnBottomOfLibrary"), "{debug}");
        assert!(debug.matches("player: TargetOpponent").count() >= 2, "{debug}");
    }

    #[test]
    fn consult_reveal_until_hand_uses_captured_consult_and_followup_clauses() {
        let tokens = crate::runtime_backend::lex_line(
            "Reveal cards from the top of your library until you reveal a nonland card, then put all cards revealed this way into your hand.",
            0,
        )
        .expect("consult all-revealed-to-hand text should lex");
        let effects =
            parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb(&tokens)
                .expect("consult hand parser should not error")
                .expect("consult hand parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
        assert!(debug.contains("MoveToZone"), "{debug}");
        assert!(debug.contains("Hand"), "{debug}");
        assert!(debug.contains("revealed"), "{debug}");
    }

    #[test]
    fn consult_reveal_until_graveyard_moves_all_revealed_cards() {
        let tokens = crate::runtime_backend::lex_line(
            "Each opponent reveals cards from the top of their library until they reveal X land cards, then puts all cards revealed this way into their graveyard.",
            0,
        )
        .expect("consult all-revealed-to-graveyard text should lex");
        let effects =
            parse_generic_consult_reveal_until_put_all_revealed_into_graveyard_subject_verb(
                &tokens,
            )
            .expect("consult graveyard parser should not error")
            .expect("consult graveyard parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
        assert!(debug.contains("MoveToZone"), "{debug}");
        assert!(debug.contains("Graveyard"), "{debug}");
        assert!(debug.contains("revealed"), "{debug}");
        assert!(debug.contains("Opponent"), "{debug}");
    }

    #[test]
    fn consult_reveal_until_battlefield_bottom_uses_captured_consult_and_followup_clauses() {
        let tokens = crate::runtime_backend::lex_line(
            "Reveal cards from the top of your library until you reveal a creature card, put it onto the battlefield, then put the rest on the bottom of your library in any order.",
            0,
        )
        .expect("consult battlefield-bottom text should lex");
        let effects = parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(&tokens)
            .expect("consult battlefield-bottom parser should not error")
            .expect("consult battlefield-bottom parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
        assert!(debug.contains("MoveToZone"), "{debug}");
        assert!(debug.contains("Battlefield"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
            "{debug}"
        );
    }

    #[test]
    fn consult_reveal_until_battlefield_bottom_preserves_tapped_land_group() {
        let tokens = crate::runtime_backend::lex_line(
            "Reveal cards from the top of your library until you reveal X land cards, put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("consult tapped land battlefield-bottom text should lex");
        let effects = parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(&tokens)
            .expect("consult tapped land battlefield-bottom parser should not error")
            .expect("consult tapped land battlefield-bottom parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
        assert!(debug.contains("MatchCount"), "{debug}");
        assert!(debug.contains("zone: Battlefield"), "{debug}");
        assert!(debug.contains("battlefield_tapped: true"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
            "{debug}"
        );
    }

    #[test]
    fn each_player_exile_top_cast_uses_captured_exile_and_cast_clauses() {
        let tokens = crate::runtime_backend::lex_line(
            "Exile the top card of each player's library, then you may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.",
            0,
        )
        .expect("each-player exile-top cast text should lex");
        let effects =
            parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(&tokens)
                .expect("each-player exile-top cast parser should not error")
                .expect("each-player exile-top cast parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ForEachPlayer"), "{debug}");
        assert!(debug.contains("ForEachObject"), "{debug}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    }

    #[test]
    fn zone_replacement_uses_captured_condition_and_replacement_clauses() {
        let tokens = crate::runtime_backend::lex_line(
            "If that card would be put into your graveyard this turn, exile that card instead.",
            0,
        )
        .expect("future graveyard exile replacement text should lex");
        let effect = parse_zone_replacement_subject_verb(&tokens)
            .expect("zone replacement parser should not error")
            .expect("zone replacement parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("AffectedPlayer"), "{debug}");
        assert!(debug.contains("You"), "{debug}");
        assert!(debug.contains("ExileInsteadOfGraveyardThisTurn"), "{debug}");
    }

    #[test]
    fn play_permission_uses_captured_duration_and_permission_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "Until end of turn, you may play lands and cast spells from your graveyard.",
            0,
        )
        .expect("graveyard play permission text should lex");
        let effect = parse_play_permission_subject_verb(&tokens)
            .expect("play permission parser should not error")
            .expect("play permission parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("PlayFromGraveyardUntilEot"), "{debug}");
        assert!(debug.contains("You"), "{debug}");
    }

    #[test]
    fn secret_number_choice_vote_uses_captured_participants_and_options() {
        let tokens = crate::runtime_backend::lex_line(
            "You and target opponent each secretly choose 1, 2, or 3.",
            0,
        )
        .expect("secret numeric choice vote text should lex");
        let effect = parse_secret_number_choice_vote_start(&tokens)
            .expect("secret numeric choice vote parser should not error")
            .expect("secret numeric choice vote parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("SecretChoiceStart"), "{debug}");
        assert!(debug.contains("\"1\""), "{debug}");
        assert!(debug.contains("\"2\""), "{debug}");
        assert!(debug.contains("\"3\""), "{debug}");
        assert!(debug.contains("Target"), "{debug}");
    }

    #[test]
    fn generic_vote_start_uses_captured_voters_and_options() {
        let tokens = crate::runtime_backend::lex_line("Each player votes for death or torture.", 0)
            .expect("generic vote-start text should lex");
        let effect = parse_generic_vote_start(&tokens)
            .expect("generic vote-start parser should not error")
            .expect("generic vote-start parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("VoteStart"), "{debug}");
        assert!(debug.contains("death"), "{debug}");
        assert!(debug.contains("torture"), "{debug}");
    }

    #[test]
    fn generic_vote_start_prefers_named_options_over_source_name_alias() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player secretly votes for truth or consequences, then those votes are revealed.",
            0,
        )
        .expect("source-name vote text should lex");
        let effect = crate::runtime_backend::util::with_source_reference_context(
            "Truth or Consequences",
            || {
                parse_generic_vote_start(&tokens)
                    .expect("generic vote-start parser should not error")
                    .expect("generic vote-start parser should match")
            },
        );
        let debug = format!("{effect:#?}");

        assert!(debug.contains("VoteStart"), "{debug}");
        assert!(debug.contains("truth"), "{debug}");
        assert!(debug.contains("consequences"), "{debug}");
        assert!(!debug.contains("VoteStartObjects"), "{debug}");
    }

    #[test]
    fn generic_vote_option_effect_uses_captured_option_and_effect_tail() {
        let tokens = crate::runtime_backend::lex_line("For each death vote, draw a card.", 0)
            .expect("generic vote-option effect text should lex");
        let effect = parse_generic_vote_option_effects(&tokens)
            .expect("generic vote-option parser should not error")
            .expect("generic vote-option parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("VoteOption"), "{debug}");
        assert!(debug.contains("death"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn player_vote_received_effect_uses_captured_player_and_effect_tail() {
        let tokens =
            crate::runtime_backend::lex_line("For each vote you received, draw a card.", 0)
                .expect("player vote-received effect text should lex");
        let effect = parse_generic_vote_option_effects(&tokens)
            .expect("player vote-received parser should not error")
            .expect("player vote-received parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("RepeatEffects"), "{debug}");
        assert!(debug.contains("PlayerVoteCount"), "{debug}");
        assert!(debug.contains("You"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn extra_vote_uses_captured_optional_vote_shape() {
        let tokens = crate::runtime_backend::lex_line("You may vote an additional time.", 0)
            .expect("optional extra vote text should lex");
        let effect =
            parse_generic_extra_vote(&tokens).expect("optional extra vote parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("VoteExtra"), "{debug}");
        assert!(debug.contains("count: 1"), "{debug}");
        assert!(debug.contains("optional: true"), "{debug}");
    }

    #[test]
    fn extra_vote_uses_captured_required_vote_shape() {
        let tokens = crate::runtime_backend::lex_line("You vote an additional time.", 0)
            .expect("required extra vote text should lex");
        let effect =
            parse_generic_extra_vote(&tokens).expect("required extra vote parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("VoteExtra"), "{debug}");
        assert!(debug.contains("count: 1"), "{debug}");
        assert!(debug.contains("optional: false"), "{debug}");
    }

    #[test]
    fn vote_reveal_uses_captured_choice_reveal_shape() {
        let tokens = crate::runtime_backend::lex_line("Then those choices are revealed.", 0)
            .expect("vote reveal text should lex");
        let effect = parse_vote_reveal_sentence(&tokens).expect("vote reveal parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("SecretChoiceReveal"), "{debug}");
    }

    #[test]
    fn control_combat_choices_uses_captured_attack_shape() {
        let tokens =
            crate::runtime_backend::lex_line("You choose which creatures attack this turn.", 0)
                .expect("combat choice attack text should lex");
        let effect = parse_generic_control_combat_choices_subject_verb(&tokens)
            .expect("combat choice attack parser should not error")
            .expect("combat choice attack parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
        assert!(debug.contains("attackers: true"), "{debug}");
        assert!(debug.contains("blockers: false"), "{debug}");
    }

    #[test]
    fn control_combat_choices_uses_captured_block_shape() {
        for (text, this_combat) in [
            (
                "You choose which creatures block this turn and how those creatures block.",
                false,
            ),
            (
                "You choose which creatures block this combat and how those creatures block.",
                true,
            ),
        ] {
            let tokens = crate::runtime_backend::lex_line(text, 0)
                .expect("combat choice block text should lex");
            let effect = parse_generic_control_combat_choices_subject_verb(&tokens)
                .expect("combat choice block parser should not error")
                .expect("combat choice block parser should match");
            let debug = format!("{effect:#?}");

            assert!(debug.contains("ControlCombatChoicesThisTurn"), "{debug}");
            assert!(debug.contains("attackers: false"), "{debug}");
            assert!(debug.contains("blockers: true"), "{debug}");
            assert!(
                debug.contains(&format!("this_combat: {this_combat}")),
                "{debug}"
            );
        }
    }

    #[test]
    fn where_x_value_binding_uses_captured_effect_and_definition() {
        let tokens = crate::runtime_backend::lex_line(
            "Target creature gets +X/+X until end of turn, where X is the number of cards in your hand.",
            0,
        )
        .expect("where-x value-binding text should lex");
        let non_binding_tokens =
            crate::runtime_backend::lex_line("Target creature gets +1/+1 until end of turn.", 0)
                .expect("non-binding pump text should lex");

        assert!(has_where_x_value_binding(&tokens));
        assert!(!has_where_x_value_binding(&non_binding_tokens));
    }

    #[test]
    fn choice_complement_uses_captured_choice_and_sacrifice_shape() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player chooses a creature from among creatures they control, then sacrifices the rest.",
            0,
        )
        .expect("choice-complement text should lex");
        let effect = parse_choice_complement_subject_verb(&tokens)
            .expect("choice-complement parser should not error")
            .expect("choice-complement parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("ForEachPlayer"), "{debug}");
        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
        assert!(debug.contains("keep"), "{debug}");
    }

    #[test]
    fn aggregate_choice_complement_keeps_the_group_power_constraint() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player chooses any number of creatures they control with total power 4 or less, then sacrifices all other creatures they control.",
            0,
        )
        .expect("aggregate choice-complement text should lex");
        let effect = parse_choice_complement_subject_verb(&tokens)
            .expect("aggregate choice-complement parser should not error")
            .expect("aggregate choice-complement parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("ForEachPlayer"), "{debug}");
        assert!(
            debug.contains("ChooseObjectsWithAggregateConstraint"),
            "{debug}"
        );
        assert!(
            debug.contains("Power") && debug.contains("maximum: Fixed(\n                    4"),
            "{debug}"
        );
        assert!(debug.contains("SacrificeAll"), "{debug}");

        let effects =
            crate::runtime_backend::sentences::effect_sentences::parse_effect_sentence_lexed(
                &tokens,
            )
            .expect("aggregate choice-complement full sentence should parse");
        let full_debug = format!("{effects:#?}");
        assert!(
            full_debug.contains("ChooseObjectsWithAggregateConstraint"),
            "{full_debug}"
        );
    }

    #[test]
    fn party_choice_complement_uses_four_optional_distinct_role_slots() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player chooses a party from among creatures they control, then sacrifices the rest.",
            0,
        )
        .expect("party choice text should lex");
        let effect = parse_choice_complement_subject_verb(&tokens)
            .expect("party choice parser should not error")
            .expect("party choice parser should match");
        let EffectAst::ForEachPlayer { effects } = effect else {
            panic!("party choice must iterate over players: {effect:#?}");
        };
        assert_eq!(effects.len(), 5, "{effects:#?}");
        let mut roles = Vec::new();
        for choice in &effects[..4] {
            let EffectAst::ChooseObjects { filter, count, .. } = choice else {
                panic!("party slot must be an object choice: {choice:#?}");
            };
            assert_eq!(*count, ChoiceCount::up_to(1));
            assert!(filter.card_types.contains(&CardType::Creature));
            assert!(filter.controller == Some(PlayerFilter::IteratedPlayer));
            roles.extend(filter.subtypes.iter().copied());
        }
        assert_eq!(
            roles,
            vec![
                Subtype::Cleric,
                Subtype::Rogue,
                Subtype::Warrior,
                Subtype::Wizard,
            ]
        );
        assert!(
            matches!(effects[4], EffectAst::SubjectVerb(_)),
            "{effects:#?}"
        );

        let effects =
            crate::runtime_backend::sentences::effect_sentences::parse_effect_sentence_lexed(
                &tokens,
            )
            .expect("full effect parser should accept party complement");
        let full_debug = format!("{effects:#?}");
        assert_eq!(
            full_debug.matches("ChooseObjects").count(),
            4,
            "{full_debug}"
        );
        assert!(full_debug.contains("SacrificeAll"), "{full_debug}");
    }

    #[test]
    fn triggering_spell_damage_uses_triggering_spell_as_source_and_fans_out() {
        let tokens = crate::runtime_backend::lex_line(
            "That spell deals damage to each opponent equal to the number of instant and sorcery spells you've cast this turn.",
            0,
        )
        .expect("triggering-spell damage text should lex");
        let effect = parse_triggered_spell_opponent_damage_subject_verb(&tokens)
            .expect("triggering-spell damage parser should not error")
            .expect("triggering-spell damage parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("ForEachOpponent"), "{debug}");
        assert!(debug.contains("triggering"), "{debug}");
        assert!(debug.contains("SpellsCastThisTurnMatching"), "{debug}");
        assert!(
            debug.contains("Instant") && debug.contains("Sorcery"),
            "{debug}"
        );

        let effects =
            crate::runtime_backend::sentences::effect_sentences::parse_effect_sentence_lexed(
                &tokens,
            )
            .expect("triggering-spell damage full sentence should parse");
        let full_debug = format!("{effects:#?}");
        assert!(full_debug.contains("ForEachOpponent"), "{full_debug}");
        assert!(full_debug.contains("triggering"), "{full_debug}");
    }

    #[test]
    fn choice_complement_preserves_independent_keep_slots_for_type_lists() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.",
            0,
        )
        .expect("choice-complement type-list text should lex");
        let recovered = choice_complement_choice_clause_from_word_order(LexedClause::new(&tokens))
            .expect("from-among word-order helper should recover choice clause");
        assert!(
            crate::runtime_backend::lexer::render_token_slice(recovered.tokens())
                .contains("from among"),
            "{}",
            crate::runtime_backend::lexer::render_token_slice(recovered.tokens())
        );
        let recovered_tokens = recovered.tokens();
        let from_idx = find_from_among(recovered_tokens).expect("should find from among");
        assert_eq!(from_idx, 0);
        let list_start = find_list_start(&recovered_tokens[2..])
            .map(|idx| idx + 2)
            .expect("should find choice list start");
        let base_tokens = trim_commas(recovered_tokens.get(2..list_start).unwrap_or_default());
        let list_tokens = trim_commas(recovered_tokens.get(list_start..).unwrap_or_default());
        assert!(
            !base_tokens.is_empty(),
            "base was empty; recovered={}",
            crate::runtime_backend::lexer::render_token_slice(recovered.tokens())
        );
        assert!(
            !list_tokens.is_empty(),
            "list was empty; recovered={}",
            crate::runtime_backend::lexer::render_token_slice(recovered.tokens())
        );
        let effect = parse_choice_complement_subject_verb(&tokens)
            .expect("choice-complement parser should not error")
            .expect("choice-complement parser should match");
        let debug = format!("{effect:#?}");

        assert!(debug.contains("ForEachPlayer"), "{debug}");
        assert_eq!(debug.matches("ChooseObjects").count(), 4, "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Enchantment"), "{debug}");
        assert!(debug.contains("Land"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
    }

    #[test]
    fn choice_complement_full_effect_sentence_keeps_comma_list_together() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.",
            0,
        )
        .expect("choice-complement type-list text should lex");
        let effects =
            crate::runtime_backend::sentences::effect_sentences::parse_effect_sentence_lexed(
                &tokens,
            )
            .expect("choice-complement full sentence should parse");
        let debug = format!("{effects:#?}");

        assert_eq!(debug.matches("ChooseObjects").count(), 4, "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Enchantment"), "{debug}");
        assert!(debug.contains("Land"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
    }

    #[test]
    fn source_gets_unblockable_uses_captured_subject_modifier_and_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "This creature gets +1/+1 until end of turn and can't be blocked this turn.",
            0,
        )
        .expect("source pump plus unblockable text should lex");
        let effects = parse_source_gets_unblockable_subject_verb(&tokens)
            .expect("source pump plus unblockable parser should not error")
            .expect("source pump plus unblockable parser should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("Fixed") && debug.contains("1"), "{debug}");
        assert!(debug.contains("BeBlocked"), "{debug}");
        assert!(debug.contains("source: true"), "{debug}");
    }

    #[test]
    fn source_gets_filter_gains_uses_captured_filter_and_ability_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "This creature gets +1/+1 and creatures you control gain trample until end of turn.",
            0,
        )
        .expect("source pump plus ability-grant text should lex");
        let effects = parse_source_gets_filter_gains_subject_verb(&tokens)
            .expect("source pump plus ability-grant parser should not error")
            .expect("source pump plus ability-grant parser should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(
            debug.contains("power: Fixed") && debug.contains("1"),
            "{debug}"
        );
        assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
        assert!(debug.contains("Trample"), "{debug}");
    }

    #[test]
    fn target_gains_then_gets_gate_uses_captured_ability_and_pump_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "Target creature gains trample and gets +1/+0 until end of turn.",
            0,
        )
        .expect("target gains then gets text should lex");
        let effects = parse_target_gains_then_gets_subject_verb(&tokens)
            .expect("target gains then gets parser should not error")
            .expect("target gains then gets parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("Trample"), "{debug}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("Fixed") && debug.contains("1"), "{debug}");
    }

    #[test]
    fn target_gets_then_gains_gate_uses_captured_pump_and_ability_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "Target creature gets +1/+1 and gains trample until end of turn.",
            0,
        )
        .expect("target gets then gains text should lex");
        let effects = parse_target_gets_then_gains_subject_verb(&tokens)
            .expect("target gets then gains parser should not error")
            .expect("target gets then gains parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("Fixed") && debug.contains("1"), "{debug}");
        assert!(debug.contains("Trample"), "{debug}");
    }

    #[test]
    fn target_gets_then_gains_preserves_other_than_source_filter() {
        let tokens = crate::runtime_backend::lex_line(
            "Target creature other than this creature gets +1/+1 and gains trample until end of turn.",
            0,
        )
        .expect("other-target get-then-gain text should lex");
        let effects = parse_target_gets_then_gains_subject_verb(&tokens)
            .expect("other-target parser should not error")
            .expect("other-target parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("other: true"), "{debug}");
        assert!(debug.contains("source: false"), "{debug}");
        assert!(!debug.contains("target: Source"), "{debug}");
        assert!(debug.contains("Trample"), "{debug}");
    }

    #[test]
    fn target_gets_then_gains_preserves_sticker_filter_before_reflexive_pronoun() {
        let tokens = crate::runtime_backend::lex_line(
            "Another target creature with an art sticker on it gets +2/+0 and gains menace until end of turn.",
            0,
        )
        .expect("stickered-target get-then-gain text should lex");
        let effects = parse_target_gets_then_gains_subject_verb(&tokens)
            .expect("stickered-target parser should not error")
            .expect("stickered-target parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("other: true"), "{debug}");
        assert!(
            debug.contains("sticker: Some") && debug.contains("ArtSticker"),
            "{debug}"
        );
        assert!(!debug.contains("target: Source"), "{debug}");
        assert!(debug.contains("Menace"), "{debug}");
    }

    #[test]
    fn attached_and_related_creatures_keep_both_subject_branches() {
        let tokens = crate::runtime_backend::lex_line(
            "Enchanted creature and other creatures that share a creature type with it get +1/+0 and gain first strike until end of turn.",
            0,
        )
        .expect("attached and related creature text should lex");
        let effects = parse_target_gets_then_gains_subject_verb(&tokens)
            .expect("attached and related parser should not error")
            .expect("attached and related parser should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("IsTaggedObject"), "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        assert!(debug.contains("SharesSubtypeWithTagged"), "{debug}");
        assert!(debug.contains("FirstStrike"), "{debug}");
    }

    #[test]
    fn target_controlled_pump_uses_captured_granted_ability_tail() {
        let tokens = crate::runtime_backend::lex_line(
            "Creatures target player controls get +1/+1 and gain haste until end of turn.",
            0,
        )
        .expect("target-controlled pump plus grant text should lex");
        let effects = parse_target_player_controls_get_subject_verb(&tokens)
            .expect("target-controlled pump parser should not error")
            .expect("target-controlled pump parser should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("Target") && debug.contains("Any"), "{debug}");
        assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
        assert!(debug.contains("Haste"), "{debug}");
    }
}
