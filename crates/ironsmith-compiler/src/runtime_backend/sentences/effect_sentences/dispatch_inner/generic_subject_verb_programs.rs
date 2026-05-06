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

pub(crate) fn parse_top_level_subject_verb_recognition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
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
        if contains_word_window(clause_words.as_slice(), &["where", "x", "is"]) {
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

fn parse_source_gets_filter_gains_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(get_idx) = words
        .iter()
        .position(|word| matches!(*word, "get" | "gets"))
    else {
        return Ok(None);
    };
    let source_words = &words[..get_idx];
    if !matches!(
        source_words,
        ["this"] | ["this", "creature"] | ["this", "permanent"]
    ) {
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
    let Some(and_idx) = words
        .iter()
        .enumerate()
        .skip(get_idx + 2)
        .find_map(|(idx, word)| (*word == "and").then_some(idx))
    else {
        return Ok(None);
    };
    let Some(gain_idx) = words
        .iter()
        .enumerate()
        .skip(and_idx + 1)
        .find_map(|(idx, word)| matches!(*word, "gain" | "gains" | "have" | "has").then_some(idx))
    else {
        return Ok(None);
    };
    if gain_idx <= and_idx + 1 {
        return Ok(None);
    }
    let filter_tokens = words[and_idx + 1..gain_idx]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
        return Ok(None);
    };
    let ability_words = &words[gain_idx + 1..];
    let mut abilities = Vec::new();
    if ability_words.iter().any(|word| *word == "haste") {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
    }
    if ability_words.iter().any(|word| *word == "trample") {
        abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Trample));
    }
    if ability_words.windows(2).any(|window| window == ["first", "strike"]) {
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
    let Some(gain_idx) = words
        .iter()
        .position(|word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };
    let has_get_tail = words[gain_idx + 1..]
        .windows(2)
        .any(|window| matches!(window, ["and", "get"] | ["and", "gets"]));
    if !has_get_tail {
        return Ok(None);
    }
    if words
        .windows(3)
        .any(|window| matches!(window, ["where", "x", "is"]))
    {
        return Ok(None);
    }
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn parse_target_player_controls_get_subject_verb(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(target_idx) = words.windows(3).position(|window| {
        matches!(
            window,
            ["target", "player", "controls"]
                | ["target", "players", "control"]
                | ["target", "opponent", "controls"]
                | ["target", "opponents", "control"]
        )
    }) else {
        return Ok(None);
    };
    let Some(get_idx) = words
        .iter()
        .enumerate()
        .skip(target_idx + 3)
        .find_map(|(idx, word)| matches!(*word, "get" | "gets").then_some(idx))
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
    let subject_tokens = words[..target_idx]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter(&subject_tokens, false)?;
    filter.controller = if matches!(
        words.get(target_idx + 1).copied(),
        Some("opponent" | "opponents")
    ) {
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
    if tail.starts_with(&["and", "gain"])
        || tail.starts_with(&["and", "gains"])
        || tail.starts_with(&["and", "have"])
        || tail.starts_with(&["and", "has"])
    {
        let ability_tail = &tail[2..];
        let mut abilities = Vec::new();
        if ability_tail.starts_with(&["first", "strike"]) {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::FirstStrike));
        }
        if ability_tail.iter().any(|word| *word == "haste") {
            abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Haste));
        }
        if ability_tail.iter().any(|word| *word == "trample") {
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
    let then_word_idx = clause_words.iter().position(|word| *word == "then")?;
    let clause_word_view = TokenWordView::new(&clause_tokens);
    let then_token_idx = clause_word_view.token_index_for_word_index(then_word_idx)?;
    let prefix_tokens = trim_commas(&clause_tokens[..then_token_idx]);
    let (player, count, reveal_top) = super::parse_top_cards_view_sentence(&prefix_tokens)?;

    let tail_start = clause_word_view
        .token_index_after_words(then_word_idx + 1)
        .unwrap_or(clause_tokens.len());
    let tail_tokens = trim_commas(&clause_tokens[tail_start..]);
    let tail_word_view = TokenWordView::new(&tail_tokens);
    if tail_word_view.first() != Some("put") {
        return None;
    }

    let count_start = tail_word_view.token_index_for_word_index(1)?;
    let count_tokens = &tail_tokens[count_start..];
    let (put_count, used) =
        crate::runtime_backend::grammar::values::parse_number_from_lexed(count_tokens)?;
    let tail_refs = TokenWordView::new(&count_tokens[used..]).word_refs();

    let mut idx = 0usize;
    if tail_refs.get(idx).copied() == Some("of") {
        idx += 1;
    }
    match tail_refs.get(idx).copied() {
        Some("them") => idx += 1,
        Some("those") => {
            idx += 1;
            if matches!(tail_refs.get(idx).copied(), Some("card" | "cards")) {
                idx += 1;
            } else {
                return None;
            }
        }
        _ => return None,
    }

    if tail_refs.get(idx).copied() != Some("into") {
        return None;
    }
    idx += 1;

    let chooser = if tail_refs.get(idx).copied() == Some("your") {
        idx += 1;
        PlayerAst::You
    } else if tail_refs.get(idx).copied() == Some("their") {
        idx += 1;
        PlayerAst::That
    } else if tail_refs.get(idx..idx + 2) == Some(&["that", "player"]) {
        idx += 2;
        PlayerAst::That
    } else {
        player
    };

    if tail_refs.get(idx).copied() != Some("hand") {
        return None;
    }
    idx += 1;
    if tail_refs.get(idx).copied() != Some("and") {
        return None;
    }
    idx += 1;
    if tail_refs.get(idx).copied() == Some("the") {
        idx += 1;
    }
    if tail_refs.get(idx).copied() != Some("rest") {
        return None;
    }
    idx += 1;
    if tail_refs.get(idx).copied() != Some("into") {
        return None;
    }
    idx += 1;

    if tail_refs.get(idx).copied() == Some("your") || tail_refs.get(idx).copied() == Some("their") {
        idx += 1;
    } else if tail_refs.get(idx..idx + 2) == Some(&["that", "player"]) {
        idx += 2;
    }

    if !matches!(
        tail_refs.get(idx).copied(),
        Some("graveyard" | "graveyards")
    ) {
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
        token.is_word("then")
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
    let puts_all_revealed_into_hand = followup_words.as_slice()
        == [
            "put", "all", "cards", "revealed", "this", "way", "into", "your", "hand",
        ]
        || followup_words.as_slice() == ["put", "all", "revealed", "cards", "into", "your", "hand"];
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
    let puts_match_onto_battlefield = slice_starts_with(
        followup_words.as_slice(),
        &["put", "it", "onto", "the", "battlefield"],
    ) || slice_starts_with(
        followup_words.as_slice(),
        &["put", "that", "card", "onto", "the", "battlefield"],
    );
    let puts_rest_bottom = slice_contains(&followup_words, &"rest")
        && slice_contains(&followup_words, &"bottom")
        && slice_contains(&followup_words, &"library");
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
        token.is_word("then")
    }) else {
        return Ok(None);
    };

    let exile_tokens = trim_commas(&sentence_tokens[..then_idx]);
    let cast_tokens = trim_commas(&sentence_tokens[then_idx + 1..]);
    if exile_tokens.is_empty() || cast_tokens.is_empty() {
        return Ok(None);
    }

    let exile_words = TokenWordView::new(&exile_tokens).word_refs();
    let starts_with_each_player_exile = slice_starts_with(
        exile_words.as_slice(),
        &["exile", "the", "top", "card", "of", "each"],
    ) || slice_starts_with(
        exile_words.as_slice(),
        &["exile", "top", "card", "of", "each"],
    );
    let mentions_player_library = exile_words
        .iter()
        .any(|word| matches!(*word, "player" | "players"))
        && exile_words.last().is_some_and(|word| *word == "library");
    if !starts_with_each_player_exile || !mentions_player_library {
        return Ok(None);
    }

    let cast_words = TokenWordView::new(&cast_tokens).word_refs();
    let casts_any_number_from_those_cards = slice_starts_with(
        cast_words.as_slice(),
        &["you", "may", "cast", "any", "number", "of", "spells"],
    ) && slice_contains(&cast_words, &"among")
        && (slice_contains(&cast_words, &"those") || slice_contains(&cast_words, &"them"))
        && slice_ends_with(
            cast_words.as_slice(),
            &["without", "paying", "their", "mana", "costs"],
        );
    if !casts_any_number_from_those_cards {
        return Ok(None);
    }

    let exiled_tag = crate::runtime_backend::util::helper_tag_for_tokens(tokens, "exiled");
    let cast_filter = ObjectFilter::nonland().in_zone(Zone::Exile).match_tagged(
        exiled_tag.clone(),
        TaggedOpbjectRelation::IsTaggedObject,
    );

    Ok(Some(vec![
        EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::That,
                Value::Fixed(1),
                Vec::new(),
                vec![exiled_tag.clone()],
            )],
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
        window == ["then", "meld", "them", "into"]
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
    if words.as_slice()
        == [
            "you",
            "choose",
            "which",
            "creatures",
            "attack",
            "this",
            "turn",
        ]
    {
        return Ok(Some(EffectAst::subject_verb_control_combat_choices_this_turn(
            true, false,
        )));
    }
    if words.as_slice()
        == [
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
    {
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

fn normalized_words_without_articles(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect()
}

fn split_once_on_comma(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let idx = tokens.iter().position(OwnedLexToken::is_comma)?;
    Some((&tokens[..idx], &tokens[idx + 1..]))
}

fn tokens_contain_relative_lesser_mana_value(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .any(|token| token.is_word("lesser") || token.is_word("less"))
        && tokens.iter().any(|token| token.is_word("mana"))
        && tokens.iter().any(|token| token.is_word("value"))
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
        TokenWordView::new(window).word_refs().as_slice() == ["with", "lesser", "mana", "value"]
    });
    let equal_start = find_window_by(tokens, 4, |window| {
        TokenWordView::new(window).word_refs().as_slice() == ["with", "mana", "value", "equal"]
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
    let duration_words = if words.starts_with(&["until", "end", "of", "turn"]) {
        4
    } else if words.starts_with(&["this", "turn"]) {
        2
    } else {
        return Ok(None);
    };
    let Some(tail_idx) = token_index_for_word_index(tokens, duration_words) else {
        return Ok(None);
    };
    let rest = trim_commas(&tokens[tail_idx..]);
    let remaining_words = normalized_words_without_articles(&rest);
    if remaining_words.as_slice()
        != [
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
    {
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
    if line_words.first().copied() != Some("if") {
        return Ok(None);
    }
    let has_graveyard_clause =
        grammar::words_find_phrase(tokens, &["into", "your", "graveyard", "from"]).is_some()
            || grammar::words_find_phrase(tokens, &["your", "graveyard", "from"]).is_some()
            || (grammar::contains_word(tokens, "your")
                && grammar::contains_word(tokens, "graveyard"));
    let has_would_put =
        grammar::words_find_phrase(tokens, &["card", "would", "be", "put"]).is_some();
    let has_this_turn =
        grammar::contains_word(tokens, "this") && grammar::contains_word(tokens, "turn");
    if !has_graveyard_clause || !has_would_put || !has_this_turn {
        return Ok(None);
    }

    let Some((_, remainder)) = split_once_on_comma(tokens) else {
        return Ok(None);
    };
    if normalized_words_without_articles(remainder).as_slice() != ["exile", "that", "card", "instead"]
    {
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
    if all_words.len() < 6
        || (!slice_starts_with(&all_words, &["each", "player", "chooses"])
            && !slice_starts_with(&all_words, &["each", "player", "choose"]))
    {
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
    if !slice_starts_with(&after_words, &["sacrifice", "the", "rest"])
        && !slice_starts_with(&after_words, &["sacrifices", "the", "rest"])
    {
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

fn truncate_vote_reveal_tail<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    for idx in 0..words.len().saturating_sub(3) {
        if words[idx..].starts_with(&["then", "those", "votes", "are"]) {
            return &words[..idx];
        }
    }
    words
}

fn parse_generic_vote_start(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(vote_idx) = find_index(&clause_words, |word| *word == "vote" || *word == "votes")
    else {
        return Ok(None);
    };

    let has_each = slice_contains(&clause_words[..vote_idx], &"each");
    let has_player = clause_words[..vote_idx]
        .iter()
        .any(|word| *word == "player" || *word == "players");
    if !has_each || !has_player {
        return Ok(None);
    }
    let secret = clause_words[..vote_idx]
        .iter()
        .any(|word| *word == "secretly" || *word == "secret");

    let for_idx = find_index(&clause_words, |word| *word == "for")
        .ok_or_else(|| CardTextError::ParseError("missing 'for' in vote clause".to_string()))?;
    if for_idx < vote_idx {
        return Ok(None);
    }

    let option_words = truncate_vote_reveal_tail(&clause_words[for_idx + 1..]).to_vec();
    let option_tokens = option_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if let Ok(target) = parse_target_phrase(&option_tokens) {
        match target {
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
        if *word == "or" {
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

    let Some(vote_idx) = find_index(&words, |word| *word == "vote" || *word == "votes") else {
        return Ok(None);
    };
    if vote_idx <= 2 {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }

    let option_words: Vec<&str> = words[2..vote_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
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

fn parse_generic_extra_vote(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 3 || words.first().copied() != Some("you") {
        return None;
    }

    let has_vote = words.iter().any(|word| *word == "vote" || *word == "votes");
    let has_additional = grammar::contains_word(tokens, "additional");
    let has_time = words.iter().any(|word| *word == "time" || *word == "times");
    if !has_vote || !has_additional || !has_time {
        return None;
    }

    let optional = grammar::contains_word(tokens, "may");
    Some(EffectAst::VoteExtra { count: 1, optional })
}
