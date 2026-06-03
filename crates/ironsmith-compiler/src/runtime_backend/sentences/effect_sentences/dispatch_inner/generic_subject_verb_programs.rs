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
        match (self.player, self.from_zone, self.replacement_zone, self.duration) {
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
}

impl GenericChoiceComplementProgram {
    fn lower(self) -> EffectAst {
        let mut effects = Vec::new();
        for keep_filter in self.keep_filters {
            let mut filter = merge_filters(&self.base_filter, &keep_filter);
            filter = filter.not_tagged(self.keep_tag.clone());
            effects.push(EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::Implicit,
                tag: self.keep_tag.clone(),
            });
        }
        effects.push(EffectAst::subject_verb_sacrifice_all(
            PlayerAst::Implicit,
            self.base_filter.not_tagged(self.keep_tag),
        ));
        match self.chooser_scope {
            PlayerAst::Any | PlayerAst::Implicit => EffectAst::ForEachPlayer { effects },
            _ => EffectAst::ForEachPlayer { effects },
        }
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
            Self::ConsultRevealUntilBattlefieldBottom { .. } => {
                "subject-verb verb=Reveal subject=explicit recognizer=consult-reveal-until-battlefield-bottom"
            }
            Self::EachPlayerExileTopCast { .. } => {
                "subject-verb verb=Exile subject=explicit recognizer=each-player-exile-top-cast"
            }
            Self::Cant { .. } => "subject-verb verb=Cant subject=explicit recognizer=restriction",
            Self::ValueBinding { .. } => "subject-verb verb=Bind subject=implicit recognizer=value-binding",
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
            | Self::ConsultRevealUntilBattlefieldBottom { effects }
            | Self::EachPlayerExileTopCast { effects }
            | Self::Cant { effects }
            | Self::ValueBinding { effects } => effects,
        }
    }
}

const THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const ALL_REVEALED_INTO_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "put", "all", "cards", "revealed", "this", "way", "into", "your", "hand",
            ],
            &["put", "all", "revealed", "cards", "into", "your", "hand"],
        ]
);
const MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["put", "it", "onto", "the", "battlefield"],
            &["put", "that", "card", "onto", "the", "battlefield"],
        ]
);
const REST_BOTTOM_LIBRARY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["rest", "bottom", "library"]);
const EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["exile", "the", "top", "card", "of", "each"],
            &["exile", "top", "card", "of", "each"],
        ]
);
const EACH_PLAYER_EXILE_UNTIL_NONLAND_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "each",
            "player",
            "exiles",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            "until",
            "they",
            "exile",
            "a",
            "nonland",
            "card",
        ]
);
const PLAYER_LIBRARY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["player", "players"]]; contains_words & ["library"]);
const CAST_ANY_NUMBER_FREE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "may", "cast", "any", "number", "of", "spells"]);
const FROM_THOSE_OR_THEM_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["among"]; contains_any_words & [&["those", "them"]]);
const WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["without", "paying", "their", "mana", "costs"]);
const FROM_NONLAND_EXILED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["from", "among", "the", "nonland", "cards", "exiled", "this", "way"]]
);
const THEN_MELD_THEM_INTO_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["then", "meld", "them", "into"]);
const CHOOSE_ATTACK_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you",
            "choose",
            "which",
            "creatures",
            "attack",
            "this",
            "turn",
        ]
);
const CHOOSE_BLOCK_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you",
            "choose",
            "which",
            "creatures",
            "block",
            "this",
            "turn",
            "and",
            "how",
            "those",
            "creatures",
            "block",
        ]
);
const WITH_LESSER_MANA_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["with", "lesser", "mana", "value"]);
const WITH_MANA_VALUE_EQUAL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["with", "mana", "value", "equal"]);
const UNTIL_END_OF_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["until", "end", "of", "turn"]);
const THIS_TURN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this", "turn"]);
const PLAY_LANDS_CAST_SPELLS_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
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
        ]
);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const EXILE_THAT_CARD_INSTEAD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["exile", "that", "card", "instead"]);
const YOUR_GRAVEYARD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["your", "graveyard"]);
const CARD_WOULD_BE_PUT_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["card", "would", "be", "put"]]);
const THIS_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["this", "turn"]);
const EACH_PLAYER_CHOOSES_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each", "player", "chooses"], &["each", "player", "choose"]]);
const SACRIFICE_THE_REST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["sacrifice", "the", "rest"], &["sacrifices", "the", "rest"]]);
const WHERE_X_IS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["where", "x", "is"]]);
const SOURCE_GETS_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["this", "creature"], &["this", "permanent"]]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const GET_OR_GETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"]]);
const GAIN_OR_GAINS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"]]);
const GAIN_HAVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"], &["have"], &["has"]]);
const ABILITY_HASTE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["haste"]);
const ABILITY_TRAMPLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["trample"]);
const ABILITY_FIRST_STRIKE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["first", "strike"]]);
const AND_GET_OR_GETS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["and", "get"], &["and", "gets"]]]);
const AND_GAIN_OR_GAINS_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["and", "gain"],
            &["and", "gains"],
            &["and", "have"],
            &["and", "has"],
        ]]
);
const AND_GAIN_HAVE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["and", "gain"],
            &["and", "gains"],
            &["and", "have"],
            &["and", "has"],
        ]
);
const UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "until", "end", "of", "turn", "and", "cant", "be", "blocked", "this", "turn",
            ],
            &[
                "until", "end", "of", "turn", "and", "can't", "be", "blocked", "this", "turn",
            ],
        ]
);
const TARGET_PLAYER_CONTROLS_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "player", "controls"],
            &["target", "players", "control"],
            &["target", "opponent", "controls"],
            &["target", "opponents", "control"],
        ]
);
const PUT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["put"]);
const OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const THOSE_CARD_OR_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["those", "card"], &["those", "cards"]]);
const THEM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["them"]);
const INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const YOUR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your"]);
const THEIR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["their"]);
const YOUR_OR_THEIR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["your"], &["their"]]);
const HAND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["hand"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const REST_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["rest"]);
const GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["graveyards"]]);
const THAT_PLAYER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "player"]);
const THEN_THOSE_VOTES_ARE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["then", "those", "votes", "are"]);
const THEN_THOSE_CHOICES_ARE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["then", "those", "choices", "are"]);
const VOTE_OR_VOTES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["vote"], &["votes"]]);
const CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
const EACH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["each"]);
const PLAYER_OR_PLAYERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["player"], &["players"]]);
const SECRET_OR_SECRETLY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["secret"], &["secretly"]]);
const FOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["for"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const VOTE_EXTRA_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["vote", "votes"]]);
const TIME_OR_TIMES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["time", "times"]]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const OPPONENT_OR_OPPONENTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent"], &["opponents"]]);

fn find_generic_word_matching_shape(words: &[&str], shape: ClauseShape<'static>) -> Option<usize> {
    find_index(words, |word| shape.matches_word(word))
}

fn find_generic_phrase_start(words: &[&str], shape: ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|idx| shape.matches_words(&words[*idx..]))
}


pub(crate) fn parse_top_level_subject_verb_recognition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    if let Some(effects) = parse_source_gets_unblockable_subject_verb(tokens)? {
        return Ok(Some((
            "subject-verb verb=Get subject=source recognizer=source-pump-unblockable",
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

    let program = if let Some(effect) = parse_generic_meld_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::Meld { effect })
    } else if let Some(effect) = parse_generic_control_combat_choices_subject_verb(tokens)? {
        Some(GenericTopLevelProgram::ControlCombatChoices { effect })
    } else if let Some(effect) =
        parse_generic_damage_replacement_counters_subject_verb(tokens)?
    {
        Some(GenericTopLevelProgram::PreventDamageAndPutCounters { effect })
    } else if let Some(effects) =
        parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(tokens)
    {
        Some(GenericTopLevelProgram::LookedCardsCountedRemainder { effects })
    } else if let Some(effects) =
        parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb(tokens)?
    {
        Some(GenericTopLevelProgram::ConsultRevealUntilHand { effects })
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
        let clause_word_storage = DispatchInnerNormalizedWords::new(tokens);
        let clause_words = clause_word_storage.to_word_refs();
        if WHERE_X_IS_PATTERN.matches_words(clause_words.as_slice()) {
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

fn parse_source_gets_unblockable_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(get_idx) = find_generic_word_matching_shape(&words, GET_OR_GETS_WORD_PATTERN) else {
        return Ok(None);
    };
    let source_words = &words[..get_idx];
    if !SOURCE_GETS_SUBJECT_PATTERN.matches_words(source_words) {
        return Ok(None);
    }

    let collapsed_modifier_tail = collapse_leading_signed_pt_modifier_tokens(&tokens[get_idx + 1..]);
    let modifier_tail = collapsed_modifier_tail
        .as_deref()
        .unwrap_or(&tokens[get_idx + 1..]);
    let modifier_words = crate::runtime_backend::token_word_refs(modifier_tail);
    let Some(modifier_word) = modifier_words.first().copied() else {
        return Ok(None);
    };
    let Ok((power, toughness)) =
        crate::runtime_backend::keyword_static::parse_pt_modifier_values(modifier_word)
    else {
        return Ok(None);
    };

    let tail = &modifier_words[1..];
    if !UNTIL_END_OF_TURN_CANT_BE_BLOCKED_TAIL_PATTERN.matches_words(tail) {
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

fn parse_source_gets_filter_gains_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(get_idx) = find_generic_word_matching_shape(&words, GET_OR_GETS_WORD_PATTERN) else {
        return Ok(None);
    };
    let source_words = &words[..get_idx];
    if !SOURCE_GETS_SUBJECT_PATTERN.matches_words(source_words) {
        return Ok(None);
    }
    let Some(modifier_word) = words.get(get_idx + 1).copied() else {
        return Ok(None);
    };
    let Ok((power, toughness)) =
        crate::runtime_backend::keyword_static::parse_pt_modifier_values(modifier_word)
    else {
        return Ok(None);
    };
    let Some(and_idx) = find_generic_word_matching_shape(&words[get_idx + 2..], AND_WORD_PATTERN)
        .map(|offset| get_idx + 2 + offset)
    else {
        return Ok(None);
    };
    let Some(gain_idx) =
        find_generic_word_matching_shape(&words[and_idx + 1..], GAIN_HAVE_WORD_PATTERN)
            .map(|offset| and_idx + 1 + offset)
    else {
        return Ok(None);
    };
    if gain_idx <= and_idx + 1 {
        return Ok(None);
    }
    let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&words[and_idx + 1..gain_idx]);
    let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
        return Ok(None);
    };
    let ability_words = &words[gain_idx + 1..];
    let mut abilities = Vec::new();
    if ABILITY_HASTE_WORD_PATTERN.matches_words(ability_words) {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
    }
    if ABILITY_TRAMPLE_WORD_PATTERN.matches_words(ability_words) {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Trample));
    }
    if ABILITY_FIRST_STRIKE_PATTERN.matches_words(ability_words) {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::FirstStrike));
    }
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
        EffectAst::subject_verb_grant_abilities_all(
            filter,
            abilities,
            Until::EndOfTurn,
        ),
    ]))
}

fn parse_target_gains_then_gets_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(gain_idx) = find_generic_word_matching_shape(&words, GAIN_OR_GAINS_WORD_PATTERN) else {
        return Ok(None);
    };
    let has_get_tail = AND_GET_OR_GETS_PATTERN.matches_words(&words[gain_idx + 1..]);
    if !has_get_tail {
        return Ok(None);
    }
    if WHERE_X_IS_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn parse_target_gets_then_gains_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(get_idx) = find_generic_word_matching_shape(&words, GET_OR_GETS_WORD_PATTERN) else {
        return Ok(None);
    };
    let has_gain_tail = AND_GAIN_OR_GAINS_PATTERN.matches_words(&words[get_idx + 1..]);
    if !has_gain_tail {
        return Ok(None);
    }
    if WHERE_X_IS_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn parse_target_player_controls_get_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(target_idx) = find_generic_phrase_start(&words, TARGET_PLAYER_CONTROLS_PATTERN) else {
        return Ok(None);
    };
    let Some(get_idx) =
        find_generic_word_matching_shape(&words[target_idx + 3..], GET_OR_GETS_WORD_PATTERN)
            .map(|offset| target_idx + 3 + offset)
    else {
        return Ok(None);
    };
    let Some(modifier_word) = words.get(get_idx + 1).copied() else {
        return Ok(None);
    };
    let Ok((power, toughness)) =
        crate::runtime_backend::keyword_static::parse_pt_modifier_values(modifier_word)
    else {
        return Ok(None);
    };
    let subject_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&words[..target_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter(&subject_tokens, false)?;
    filter.controller = if words
        .get(target_idx + 1)
        .is_some_and(|word| OPPONENT_OR_OPPONENTS_WORD_PATTERN.matches_word(word))
    {
        Some(PlayerFilter::target_opponent())
    } else {
        Some(PlayerFilter::target_player())
    };

    let mut effects = vec![EffectAst::subject_verb_pump_all(
        filter.clone(),
        power,
        toughness,
        Until::EndOfTurn,
    )];
    let tail = &words[get_idx + 2..];
    if AND_GAIN_HAVE_TAIL_PATTERN.matches_words(tail) {
        let ability_tail = &tail[2..];
        let mut abilities = Vec::new();
        if ABILITY_FIRST_STRIKE_PATTERN.matches_words(ability_tail) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::FirstStrike));
        }
        if ABILITY_HASTE_WORD_PATTERN.matches_words(ability_tail) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
        }
        if ABILITY_TRAMPLE_WORD_PATTERN.matches_words(ability_tail) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Trample));
        }
        if !abilities.is_empty() {
            effects.push(EffectAst::subject_verb_grant_abilities_all(
                filter,
                abilities,
                Until::EndOfTurn,
            ));
        }
    }
    Ok(Some(effects))
}

pub(crate) fn parse_generic_top_cards_put_counted_into_hand_rest_graveyard_subject_verb(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let clause_tokens = trim_commas(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(&clause_tokens);
    let then_word_idx = find_generic_word_matching_shape(&clause_words, THEN_WORD_PATTERN)?;
    let clause_word_view = TokenWordView::new(&clause_tokens);
    let then_token_idx = clause_word_view.token_index_for_word_index(then_word_idx)?;
    let prefix_tokens = trim_commas(&clause_tokens[..then_token_idx]);
    let (player, count, reveal_top) = super::parse_top_cards_view_sentence(&prefix_tokens)?;

    let tail_start = clause_word_view
        .token_index_after_words(then_word_idx + 1)
        .unwrap_or(clause_tokens.len());
    let tail_tokens = trim_commas(&clause_tokens[tail_start..]);
    let tail_word_view = TokenWordView::new(&tail_tokens);
    if !tail_word_view
        .first()
        .is_some_and(|word| PUT_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let count_start = tail_word_view.token_index_for_word_index(1)?;
    let count_tokens = &tail_tokens[count_start..];
    let (put_count, used) =
        crate::runtime_backend::grammar::values::parse_number_from_lexed(count_tokens)?;
    let tail_refs = TokenWordView::new(&count_tokens[used..]).word_refs();

    let mut idx = 0usize;
    if OF_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        idx += 1;
    }
    if THEM_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        idx += 1;
    } else if THOSE_CARD_OR_CARDS_PATTERN.matches_words(&tail_refs[idx..]) {
        idx += 2;
    } else {
        return None;
    }

    if !INTO_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        return None;
    }
    idx += 1;

    let chooser = if YOUR_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        idx += 1;
        PlayerAst::You
    } else if THEIR_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        idx += 1;
        PlayerAst::That
    } else if THAT_PLAYER_PREFIX_PATTERN.matches_words(&tail_refs[idx..]) {
        idx += 2;
        PlayerAst::That
    } else {
        player
    };

    if !HAND_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        return None;
    }
    idx += 1;
    if !AND_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        return None;
    }
    idx += 1;
    if THE_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        idx += 1;
    }
    if !REST_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        return None;
    }
    idx += 1;
    if !INTO_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        return None;
    }
    idx += 1;

    if YOUR_OR_THEIR_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        idx += 1;
    } else if THAT_PLAYER_PREFIX_PATTERN.matches_words(&tail_refs[idx..]) {
        idx += 2;
    }

    if !GRAVEYARD_OR_GRAVEYARDS_WORD_PATTERN.matches_word_at(&tail_refs, idx) {
        return None;
    }
    idx += 1;
    if idx != tail_refs.len() {
        return None;
    }

    let looked_tag = TagKey::from(IT_TAG);
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone())];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag));
    }
    effects.push(EffectAst::subject_verb_put_some_into_hand_rest_into_graveyard(
        chooser,
        put_count,
    ));
    Some(effects)
}

fn parse_generic_consult_reveal_until_put_all_revealed_into_hand_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(then_idx) = find_index(&sentence_tokens, |token: &OwnedLexToken| {
        token
            .as_word()
            .is_some_and(|word| THEN_WORD_PATTERN.matches_word(word))
    }) else {
        return Ok(None);
    };

    let consult_tokens = trim_commas(&sentence_tokens[..then_idx]);
    let followup_tokens = trim_commas(&sentence_tokens[then_idx + 1..]);
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
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
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

    let followup_words = TokenWordView::new(&followup_tokens).word_refs();
    let puts_all_revealed_into_hand = ALL_REVEALED_INTO_HAND_PATTERN.matches_words(&followup_words);
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
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
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
    let sentence_tokens = trim_commas(
        super::super::token_primitives::strip_leading_if_you_do_lexed(tokens),
    );
    let Some((consult_tokens, followup_tokens)) = split_once_on_comma(&sentence_tokens) else {
        return Ok(None);
    };
    let consult_tokens = trim_commas(consult_tokens);
    let followup_tokens = trim_commas(followup_tokens);
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
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                    ..
                },
            ..
        }))
    ) {
        return Ok(None);
    }

    let followup_words = TokenWordView::new(&followup_tokens).word_refs();
    let puts_match_onto_battlefield =
        MATCH_ONTO_BATTLEFIELD_PREFIX_PATTERN.matches_words(followup_words.as_slice());
    let puts_rest_bottom = REST_BOTTOM_LIBRARY_PATTERN.matches_words(&followup_words);
    if !puts_match_onto_battlefield || !puts_rest_bottom {
        return Ok(None);
    }
    let Some(order) = super::consult_family::parse_consult_remainder_order(&followup_words) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    apply_lesser_mana_value_consult_constraint(&sentence_tokens, &mut effects);
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        Zone::Battlefield,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
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

fn parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(then_idx) = find_index(&sentence_tokens, |token: &OwnedLexToken| {
        token
            .as_word()
            .is_some_and(|word| THEN_WORD_PATTERN.matches_word(word))
    }) else {
        return Ok(None);
    };

    let exile_tokens = trim_commas(&sentence_tokens[..then_idx]);
    let cast_tokens = trim_commas(&sentence_tokens[then_idx + 1..]);
    if exile_tokens.is_empty() || cast_tokens.is_empty() {
        return Ok(None);
    }

    let exile_words = TokenWordView::new(&exile_tokens).word_refs();
    let starts_with_each_player_exile =
        EACH_PLAYER_EXILE_TOP_CARD_PREFIX_PATTERN.matches_words(exile_words.as_slice());
    let starts_with_each_player_exile_until_nonland =
        EACH_PLAYER_EXILE_UNTIL_NONLAND_PREFIX_PATTERN.matches_words(exile_words.as_slice());
    let mentions_player_library = PLAYER_LIBRARY_MARKER_PATTERN.matches_words(&exile_words);
    if !(starts_with_each_player_exile || starts_with_each_player_exile_until_nonland)
        || !mentions_player_library
    {
        return Ok(None);
    }

    let cast_words = TokenWordView::new(&cast_tokens).word_refs();
    let casts_any_number_from_those_cards =
        CAST_ANY_NUMBER_FREE_PREFIX_PATTERN.matches_words(cast_words.as_slice())
            && FROM_THOSE_OR_THEM_MARKER_PATTERN.matches_words(&cast_words)
            && WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN.matches_words(cast_words.as_slice());

    let casts_any_number_from_nonland_exiled_this_way =
        CAST_ANY_NUMBER_FREE_PREFIX_PATTERN.matches_words(cast_words.as_slice())
            && FROM_NONLAND_EXILED_THIS_WAY_PATTERN.matches_words(cast_words.as_slice())
            && WITHOUT_PAYING_THEIR_MANA_COSTS_SUFFIX_PATTERN.matches_words(cast_words.as_slice());

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
                effects: vec![EffectAst::subject_verb_cast_tagged(TagKey::from(IT_TAG), PlayerAst::You, false, false, true, None)],
            }],
        },
    ]))
}

fn parse_generic_meld_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["exile", "them"]).is_none() {
        return Ok(None);
    }
    let Some(meld_idx) = find_window_by(&clause_words, 4, |window| {
        THEN_MELD_THEM_INTO_PATTERN.matches_words(window)
    }) else {
        return Ok(None);
    };
    let result_words = &clause_words[meld_idx + 4..];
    if result_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing meld result name (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(Some(EffectAst::subject_verb_meld(
        result_words.join(" "),
        false,
        false,
    )))
}

fn parse_generic_control_combat_choices_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if CHOOSE_ATTACK_THIS_TURN_PATTERN.matches_words(&words) {
        return Ok(Some(EffectAst::subject_verb_control_combat_choices_this_turn(
            true, false,
        )));
    }
    if CHOOSE_BLOCK_THIS_TURN_PATTERN.matches_words(&words) {
        return Ok(Some(EffectAst::subject_verb_control_combat_choices_this_turn(
            false, true,
        )));
    }
    Ok(None)
}

fn parse_generic_damage_replacement_counters_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["if", "damage", "would", "be", "dealt", "to"])
        .is_none()
    {
        return Ok(None);
    }

    let Some(this_turn_rel) =
        crate::runtime_backend::grammar::primitives::find_phrase_start(&tokens[6..], &["this", "turn"])
    else {
        return Ok(None);
    };
    let this_turn_idx = 6 + this_turn_rel;
    let tail = &clause_words[this_turn_idx + 2..];
    let valid_tail = matches!(
        tail,
        [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "that", "creature"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "that", "creature"
        ]
    );
    if !valid_tail {
        return Ok(None);
    }

    let target_tokens = &tokens[6..this_turn_idx];
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

fn split_once_on_comma(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let idx = crate::runtime_backend::lexer::find_token_kind(
        tokens,
        crate::runtime_backend::lexer::TokenKind::Comma,
    )?;
    Some((&tokens[..idx], &tokens[idx + 1..]))
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
    let lesser_start = find_window_by(tokens, 4, |window| {
        WITH_LESSER_MANA_VALUE_PATTERN.matches_words(&TokenWordView::new(window).word_refs())
    });
    let equal_start = find_window_by(tokens, 4, |window| {
        WITH_MANA_VALUE_EQUAL_PATTERN.matches_words(&TokenWordView::new(window).word_refs())
    });
    let Some(start) = lesser_start.or(equal_start) else {
        return tokens.to_vec();
    };
    trim_commas(&tokens[..start]).to_vec()
}

pub(crate) fn parse_play_permission_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let duration_words = if UNTIL_END_OF_TURN_PREFIX_PATTERN.matches_words(&words) {
        4
    } else if THIS_TURN_PREFIX_PATTERN.matches_words(&words) {
        2
    } else {
        return Ok(None);
    };
    let Some(tail_idx) = token_index_for_word_index(tokens, duration_words) else {
        return Ok(None);
    };
    let rest = trim_commas(&tokens[tail_idx..]);
    let remaining_words = non_article_token_word_refs(&rest);
    if !PLAY_LANDS_CAST_SPELLS_GRAVEYARD_PATTERN.matches_words(&remaining_words) {
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
    let line_words = crate::runtime_backend::token_word_refs(tokens);
    if !line_words
        .first()
        .is_some_and(|word| IF_WORD_PATTERN.matches_words(&[*word]))
    {
        return Ok(None);
    }
    let has_graveyard_clause = YOUR_GRAVEYARD_MARKER_PATTERN.matches_words(&line_words);
    let has_would_put = CARD_WOULD_BE_PUT_MARKER_PATTERN.matches_words(&line_words);
    let has_this_turn = THIS_TURN_MARKER_PATTERN.matches_words(&line_words);
    if !has_graveyard_clause || !has_would_put || !has_this_turn {
        return Ok(None);
    }

    let Some((_, remainder)) = split_once_on_comma(tokens) else {
        return Ok(None);
    };
    if !EXILE_THAT_CARD_INSTEAD_PATTERN.matches_words(&non_article_token_word_refs(remainder)) {
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
    let all_words = crate::runtime_backend::token_word_refs(tokens);
    if all_words.len() < 6 || !EACH_PLAYER_CHOOSES_PREFIX_PATTERN.matches_words(&all_words) {
        return Ok(None);
    }

    let Some((before_then, after_then)) =
        grammar::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            grammar::kw("then").void()
        })
    else {
        return Ok(None);
    };
    let then_idx = before_then.len();
    let after_words = crate::runtime_backend::token_word_refs(after_then);
    if !SACRIFICE_THE_REST_PREFIX_PATTERN.matches_words(&after_words) {
        return Ok(None);
    }

    let choose_tokens = &tokens[3..then_idx];
    let Some(from_idx) = find_from_among(choose_tokens) else {
        return Ok(None);
    };
    let (list_tokens, base_tokens) = if from_idx == 0 {
        let list_start = find_list_start(&choose_tokens[2..])
            .map(|idx| idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError("missing choice list after 'from among'".to_string())
            })?;
        (
            choose_tokens.get(list_start..).unwrap_or_default(),
            choose_tokens.get(2..list_start).unwrap_or_default(),
        )
    } else {
        (
            choose_tokens.get(..from_idx).unwrap_or_default(),
            choose_tokens.get(from_idx + 2..).unwrap_or_default(),
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
            all_words.join(" ")
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
                all_words.join(" ")
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
        }
        .lower(),
    ))
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
        if let EffectAst::VoteStart { options, secret } = effect
        {
            return Ok(Some(
                GenericVoteProgram::Start {
                    options,
                    secret,
                }
                .lower(),
            ));
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

fn truncate_vote_reveal_tail<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    for idx in 0..words.len().saturating_sub(3) {
        if THEN_THOSE_VOTES_ARE_PREFIX_PATTERN.matches_words(&words[idx..])
            || THEN_THOSE_CHOICES_ARE_PREFIX_PATTERN.matches_words(&words[idx..])
        {
            return &words[..idx];
        }
    }
    words
}

fn parse_vote_reveal_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        ["then", "those", "choices", "are", "revealed"]
            | ["those", "choices", "are", "revealed"]
    ) {
        return Some(EffectAst::SecretChoiceReveal);
    }
    None
}

fn parse_secret_number_choice_vote_start(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(choose_idx) = find_index(&clause_words, |word| CHOOSE_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if !clause_words[..choose_idx].starts_with(&["you", "and", "target", "opponent"])
        || !clause_words[..choose_idx].contains(&"each")
        || !clause_words[..choose_idx]
            .iter()
            .any(|word| SECRET_OR_SECRETLY_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let option_words = truncate_vote_reveal_tail(&clause_words[choose_idx + 1..]);
    let options = option_words
        .iter()
        .filter(|word| !OR_WORD_PATTERN.matches_word(word))
        .filter(|word| word.chars().all(|ch| ch.is_ascii_digit()))
        .map(|word| (*word).to_string())
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
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(vote_idx) = find_index(&clause_words, |word| {
        VOTE_OR_VOTES_WORD_PATTERN.matches_word(word)
    })
    else {
        return Ok(None);
    };

    let has_each = EACH_WORD_PATTERN.matches_words(&clause_words[..vote_idx]);
    let has_player = clause_words[..vote_idx]
        .iter()
        .any(|word| PLAYER_OR_PLAYERS_WORD_PATTERN.matches_word(word));
    if !has_each || !has_player {
        return Ok(None);
    }
    let secret = clause_words[..vote_idx]
        .iter()
        .any(|word| SECRET_OR_SECRETLY_WORD_PATTERN.matches_word(word));

    let for_idx = find_index(&clause_words, |word| FOR_WORD_PATTERN.matches_word(word))
        .ok_or_else(|| CardTextError::ParseError("missing 'for' in vote clause".to_string()))?;
    if for_idx < vote_idx {
        return Ok(None);
    }

    let option_words = truncate_vote_reveal_tail(&clause_words[for_idx + 1..]).to_vec();
    let option_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&option_words);
    if let Ok(target) = parse_target_phrase(&option_tokens) {
        match target {
            TargetAst::Player(filter, _) => {
                let exclude_voter = option_words
                    .first()
                    .is_some_and(|word| matches!(*word, "other" | "another"));
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

    let mut options = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in &option_words {
        if OR_WORD_PATTERN.matches_word(word) {
            if !current.is_empty() {
                options.push(current.join(" "));
                current.clear();
            }
            continue;
        }
        if is_article(word) {
            continue;
        }
        current.push(word);
    }
    if !current.is_empty() {
        options.push(current.join(" "));
    }
    if options.len() < 2 {
        return Err(CardTextError::ParseError(
            "vote clause requires at least two options".to_string(),
        ));
    }

    Ok(Some(EffectAst::VoteStart { options, secret }))
}

fn parse_generic_vote_option_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 4 || grammar::words_match_prefix(tokens, &["for", "each"]).is_none() {
        return Ok(None);
    }

    let Some(vote_idx) =
        find_index(&words, |word| VOTE_OR_VOTES_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if vote_idx <= 2 {
        if let Some(effect) = parse_generic_player_vote_received_effects(tokens, &words, vote_idx)?
        {
            return Ok(Some(effect));
        }
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }

    let option_words = crate::runtime_backend::util::non_article_word_refs(&words[2..vote_idx]);
    if option_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }
    let option = option_words.join(" ");

    let (_before, effect_tokens) =
        grammar::split_lexed_once_on_delimiter(tokens, super::super::lexer::TokenKind::Comma)
            .ok_or_else(|| {
                CardTextError::ParseError("missing comma in for each vote clause".to_string())
            })?;
    let effects = parse_effect_chain_lexed(effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}

fn parse_generic_player_vote_received_effects(
    tokens: &[OwnedLexToken],
    words: &[&str],
    vote_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(received_idx) = find_index(&words[vote_idx + 1..], |word| {
        matches!(*word, "received" | "receives")
    })
    .map(|idx| idx + vote_idx + 1)
    else {
        return Ok(None);
    };
    if received_idx <= vote_idx + 1 {
        return Ok(None);
    }
    let player_words = crate::runtime_backend::util::non_article_word_refs(
        &words[vote_idx + 1..received_idx],
    );
    if player_words.is_empty() {
        return Ok(None);
    }
    let player_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&player_words);
    let TargetAst::Player(filter, _) = parse_target_phrase(&player_tokens)? else {
        return Ok(None);
    };
    let (_before, effect_tokens) =
        grammar::split_lexed_once_on_delimiter(tokens, super::super::lexer::TokenKind::Comma)
            .ok_or_else(|| {
                CardTextError::ParseError("missing comma in for each vote clause".to_string())
            })?;
    let effects = parse_effect_chain_lexed(effect_tokens)?;
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

fn parse_generic_extra_vote(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 3 || !YOU_WORD_PATTERN.matches_words(&words[..1]) {
        return None;
    }

    let has_vote = VOTE_EXTRA_MARKER_PATTERN.matches_words(&words);
    let has_additional = grammar::contains_word(tokens, "additional");
    let has_time = TIME_OR_TIMES_MARKER_PATTERN.matches_words(&words);
    if !has_vote || !has_additional || !has_time {
        return None;
    }

    let optional = grammar::contains_word(tokens, "may");
    Some(EffectAst::VoteExtra { count: 1, optional })
}
