const SOURCE_ATTACHMENT_PREFIXES: &[&[&str]] = &[
    &["this", "equipment"],
    &["this", "aura"],
    &["this", "enchantment"],
    &["this", "artifact"],
];
const ADDITIONAL_PREFIXES: &[&[&str]] = &[&["an", "additional"], &["additional"]];
const FOR_EACH_OPPONENT_WHO_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent", "who"],
    &["for", "each", "opponents", "who"],
];
const FOR_EACH_PLAYER_WHO_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player", "who"],
    &["for", "each", "players", "who"],
];
const EACH_OPPONENT_WHO_PREFIXES: &[&[&str]] =
    &[&["each", "opponent", "who"], &["each", "opponents", "who"]];
const EACH_PLAYER_WHO_PREFIXES: &[&[&str]] =
    &[&["each", "player", "who"], &["each", "players", "who"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[
    &["that", "amount", "of"],
    &["that", "much"],
    &["that", "many"],
];
const DAMAGE_TO_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[&["damage", "to", "each", "opponent"]];
const EACH_OF_PREFIXES: &[&[&str]] = &[&["each", "of"]];
const YOU_CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controlled"]];
const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"]];
const EACH_OPPONENT_AND_EACH_PREFIXES: &[&[&str]] = &[&["each", "opponent", "and", "each"]];

const TAKE_EXTRA_TURN_AFTER_THIS_ONE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["an", "extra", "turn", "after", "this", "one"]);
const PROLIFERATE_TRAILING_OK_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["time"],
            &["times"],
            &["instead"],
            &["time", "instead"],
            &["times", "instead"],
        ]
);
const NTH_FROM_TOP_DESTINATION_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "top"]);
const THAT_LIBRARY_AMOUNT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["of", "that", "library"]);
const RESOURCE_LIBRARY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["library"], &["libraries"]]);
const RESOURCE_AT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at"]);
const RESOURCE_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the"], &["a"], &["an"]]);
const RESOURCE_PLAY_THOSE_EXILED_PATTERN: ClauseShape<'static> = clause_shape!(exact & [
    "and", "play", "those", "cards", "for", "as", "long", "as", "they", "remain", "exiled",
]);
const RESOURCE_TOP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["top"]);
const RESOURCE_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const RESOURCE_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const RESOURCE_ANY_OR_ALL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["any"], &["all"]]);
const RESOURCE_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const RESOURCE_AS_YOU_CHOOSE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["as", "you", "choose"]);
const RESOURCE_INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const NOTE_YOUR_LIFE_TOTAL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["your", "life", "total"]);
const RESOURCE_THE_REST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "rest"]);
const RESOURCE_ALL_OTHER_REVEALED_OR_EXILED_CARDS_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["all", "other"];
    contains_words & ["cards"];
    contains_any_words & [&["revealed", "exiled"]]
);
const RESOURCE_ITS_OWNER_LIBRARY_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["them", "into", "their", "libraries"],
            &["them", "into", "their", "library"],
            &["those", "cards", "into", "their", "libraries"],
            &["those", "cards", "into", "their", "library"],
        ]
);
const RESOURCE_UNSUPPORTED_SHUFFLE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["graveyard"], &["cards"], &["card"], &["into"], &["from"]]);
const RESOURCE_IT_OR_THEM_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const RESOURCE_WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const RESOURCE_NAME_OR_NAMES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["name"], &["names"]]);
const RESOURCE_CHOSEN_NAME_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["chosen", "for", "this"]);
const RESOURCE_CHOSEN_NAME_OBJECT_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["artifact"],
            &["card"],
            &["creature"],
            &["enchantment"],
            &["permanent"],
            &["source"],
        ]
);

const LOOK_YOUR_HAND_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["your", "hand"]);
const LOOK_EACH_PLAYER_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each", "player", "hand"], &["each", "players", "hand"]]);
const LOOK_THEIR_HAND_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["their", "hand"]);
const LOOK_THAT_PLAYER_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["that", "player", "hand"], &["that", "players", "hand"]]);
const LOOK_TARGET_PLAYER_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "player", "hand"],
            &["target", "players", "hand"]
        ]
);
const LOOK_TARGET_OPPONENT_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "opponent", "hand"],
            &["target", "opponents", "hand"]
        ]
);
const LOOK_OPPONENT_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent", "hand"], &["opponents", "hand"]]);
const LOOK_HIS_OR_HER_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["his", "or", "her", "hand"]);

const LOOK_YOUR_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["your", "library"]);
const LOOK_EACH_PLAYER_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["each", "player", "library"],
            &["each", "players", "library"]
        ]
);
const LOOK_THEIR_LIBRARY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["their", "library"]);
const LOOK_THAT_PLAYER_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "player", "library"],
            &["that", "players", "library"]
        ]
);
const LOOK_TARGET_PLAYER_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "player", "library"],
            &["target", "players", "library"]
        ]
);
const LOOK_TARGET_OPPONENT_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["target", "opponent", "library"],
            &["target", "opponents", "library"]
        ]
);
const LOOK_ITS_OWNER_LIBRARY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["its", "owner", "library"], &["its", "owners", "library"]]);
const LOOK_HIS_OR_HER_LIBRARY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["his", "or", "her", "library"]);
const LOOK_TOP_THAT_PLAYER_LIBRARY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "top", "card", "of", "that", "player", "library"],
            &["the", "top", "card", "of", "that", "players", "library"],
            &["top", "card", "of", "that", "player", "library"],
            &["top", "card", "of", "that", "players", "library"],
            &["the", "top", "card", "of", "their", "library"],
            &["top", "card", "of", "their", "library"],
        ]
);

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

pub(crate) fn parse_effect_with_verb(
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
        Verb::Lose => parse_lose_life(tokens, subject),
        Verb::Gain => {
            if token_slice_first_is(tokens, "control") {
                parse_gain_control(tokens, subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            let has_onto = crate::runtime_backend::lexer::contains_token_word(tokens, "onto");
            let has_counter_words = crate::runtime_backend::lexer::contains_token_any_word(
                tokens,
                &["counter", "counters"],
            );

            // Prefer zone moves like "... onto the battlefield" over counter placement because
            // "counter(s)" may appear in subordinate clauses (e.g. "mana value equal to the number
            // of charge counters on this artifact").
            if has_onto {
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
        Verb::Attach => parse_attach(tokens),
        Verb::Unattach => parse_unattach(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Convert => parse_convert(tokens),
        Verb::Flip => parse_flip(tokens, subject),
        Verb::Roll => parse_roll(tokens, subject),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => parse_return(tokens),
        Verb::Exchange => parse_exchange(tokens, subject),
        Verb::Become => parse_become(tokens, subject),
        Verb::Switch => parse_switch(tokens),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
        Verb::Shuffle => parse_shuffle(tokens, subject),
        Verb::Reorder => parse_reorder(tokens, subject),
        Verb::Pay => parse_pay(tokens, subject),
        Verb::Take => parse_take(tokens, subject),
        Verb::Detain => parse_detain(tokens),
        Verb::Goad => parse_goad(tokens),
        Verb::Suspect => parse_suspect(tokens),
        Verb::Note => parse_note(tokens),
        Verb::End => parse_end(tokens, subject),
    }
}

fn parse_note(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if NOTE_YOUR_LIFE_TOTAL_PATTERN.matches_words(&words) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::NoteLifeTotal,
        ));
    }
    Err(CardTextError::ParseError(format!(
        "unsupported note clause: '{}'",
        words.join(" ")
    )))
}

fn parse_take(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if TAKE_EXTRA_TURN_AFTER_THIS_ONE_PATTERN.matches_words(&words) {
        return Ok(EffectAst::subject_verb_extra_turn_after_turn(
            extract_subject_player(subject).unwrap_or(PlayerAst::You),
            ExtraTurnAnchorAst::CurrentTurn,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported take clause (clause: '{}')",
        words.join(" ")
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
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing proliferate count (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_words = crate::runtime_backend::token_word_refs(&trailing);
    let trailing_ok =
        trailing_words.is_empty() || PROLIFERATE_TRAILING_OK_PATTERN.matches_words(&trailing_words);
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing proliferate clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_proliferate(count))
}

fn parse_library_nth_from_top_destination(tokens: &[OwnedLexToken]) -> Option<Value> {
    let library_idx = find_index(tokens, |token: &OwnedLexToken| {
        RESOURCE_LIBRARY_WORD_PATTERN.matches_token(token)
    })?;
    let tail_tokens = trim_commas(&tokens[library_idx + 1..]);
    if tail_tokens.is_empty() {
        return None;
    }

    let filtered_tail = crate::runtime_backend::util::non_article_token_word_refs(&tail_tokens);
    if let Some((position, used)) = ironsmith_core::parse_ordinal_words(&filtered_tail)
        && filtered_tail
            .get(used..)
            .is_some_and(|tail| NTH_FROM_TOP_DESTINATION_TAIL_PATTERN.matches_words(tail))
    {
        return Some(Value::Fixed(position as i32));
    }

    let (_, amount_words) = word_slice_strip_prefix_value(
        &filtered_tail,
        &[(&["just", "beneath", "top"], ()), (&["beneath", "top"], ())],
    )?;
    let amount_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(amount_words);
    let (amount, used) = parse_value(&amount_tokens)?;
    let amount_words = crate::runtime_backend::token_word_refs(&amount_tokens);
    if !RESOURCE_CARD_OR_CARDS_WORD_PATTERN.matches_word_at(&amount_words, used) {
        return None;
    }
    if used + 1 > amount_words.len() {
        return None;
    }
    if !THAT_LIBRARY_AMOUNT_TAIL_PATTERN.matches_words(&amount_words[used + 1..]) {
        return None;
    }

    Some(Value::Add(Box::new(amount), Box::new(Value::Fixed(1))))
}

pub(crate) fn parse_look(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_hand_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if LOOK_YOUR_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::You, 2));
        }
        if LOOK_EACH_PLAYER_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::Any, 3));
        }
        if LOOK_THEIR_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::That, 2));
        }
        if LOOK_THAT_PLAYER_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::That, 3));
        }
        if LOOK_TARGET_PLAYER_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::Target, 3));
        }
        if LOOK_TARGET_OPPONENT_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if LOOK_OPPONENT_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::Opponent, 2));
        }
        if LOOK_HIS_OR_HER_HAND_PATTERN.matches_words(words) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    fn parse_library_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if LOOK_YOUR_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::You, 2));
        }
        if LOOK_EACH_PLAYER_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::Any, 3));
        }
        if LOOK_THEIR_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::That, 2));
        }
        if LOOK_THAT_PLAYER_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::That, 3));
        }
        if LOOK_TARGET_PLAYER_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::Target, 3));
        }
        if LOOK_TARGET_OPPONENT_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if LOOK_ITS_OWNER_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::ItsOwner, 3));
        }
        if LOOK_HIS_OR_HER_LIBRARY_PATTERN.matches_words(words) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    fn parse_look_tail_at_same_player(words: &[&str]) -> Option<Vec<EffectAst>> {
        let top_prefix_len = LOOK_TOP_THAT_PLAYER_LIBRARY_PREFIX_PATTERN
            .matched_prefix_len(words)?;
        let mut rest = &words[top_prefix_len..];
        let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::That,
            Value::Fixed(1),
            TagKey::from(IT_TAG),
        )];

        if rest.is_empty() {
            return Some(effects);
        }
        if RESOURCE_AND_WORD_PATTERN.matches_first_word(rest) {
            rest = &rest[1..];
        }
        if RESOURCE_ANY_OR_ALL_WORD_PATTERN.matches_first_word(rest) {
            rest = &rest[1..];
        }
        if matches!(
            rest,
            ["face", "down", "creatures", "they", "control"]
                | ["face", "down", "creature", "they", "control"]
                | ["face", "down", "creatures", "that", "player", "controls"]
                | ["face", "down", "creatures", "that", "players", "control"]
                | ["face", "down", "creature", "that", "player", "controls"]
                | ["face", "down", "creature", "that", "players", "control"]
        ) {
            effects.push(EffectAst::subject_verb_look_at_objects(
                PlayerAst::That,
                ObjectFilter::creature().face_down(),
            ));
            return Some(effects);
        }

        None
    }

    // "Look at the top N cards of your library."
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| RESOURCE_AT_WORD_PATTERN.matches_token(token))
    {
        clause_tokens = trim_commas(&clause_tokens[1..]);
    }
    let clause_word_storage = TokenWordView::new(&clause_tokens).owned_words();
    let clause_words = clause_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if RESOURCE_PLAY_THOSE_EXILED_PATTERN.matches_words(&clause_words) {
        return Ok(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                TagKey::from(IT_TAG),
                PlayerAst::You,
                true,
                false,
                false,
                None,
            ),
        );
    }

    let mut hand_tokens = clause_tokens.clone();
    while hand_tokens
        .first()
        .is_some_and(|token| RESOURCE_ARTICLE_WORD_PATTERN.matches_token(token))
    {
        hand_tokens = hand_tokens[1..].to_vec();
    }
    let hand_word_storage = TokenWordView::new(&hand_tokens).owned_words();
    let hand_words = hand_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some((player, used_words)) = parse_hand_owner(&hand_words) {
        let target = match player {
            PlayerAst::You => TargetAst::Player(PlayerFilter::You, None),
            PlayerAst::Opponent => TargetAst::Player(PlayerFilter::Opponent, None),
            PlayerAst::Target => TargetAst::Player(
                PlayerFilter::target_player(),
                span_from_tokens(&hand_tokens),
            ),
            PlayerAst::TargetOpponent => {
                TargetAst::Player(PlayerFilter::Opponent, span_from_tokens(&hand_tokens))
            }
            PlayerAst::That => TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            PlayerAst::Any => {
                return Ok(EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::subject_verb_look_at_hand(TargetAst::Player(
                        PlayerFilter::IteratedPlayer,
                        None,
                    ))],
                });
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported look clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        };

        if used_words < hand_words.len() {
            if let Some(mut followups) = parse_look_tail_at_same_player(&hand_words[used_words..]) {
                let mut effects = vec![EffectAst::subject_verb_look_at_hand(target)];
                effects.append(&mut followups);
                return Ok(EffectAst::Sequence { effects });
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing look clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(EffectAst::subject_verb_look_at_hand(target));
    }

    if let Some(filter) = match hand_words.as_slice() {
        ["target", "face", "down", "creature"]
        | ["target", "face", "down", "creatures"] => Some(ObjectFilter::creature().face_down()),
        ["target", "face", "down", "permanent"]
        | ["target", "face", "down", "permanents"] => Some(ObjectFilter::permanent().face_down()),
        _ => None,
    } {
        let target = TargetAst::Object(filter, span_from_tokens(&hand_tokens), None);
        return Ok(EffectAst::subject_verb_look_at_target(target));
    }

    let Some(top_idx) = find_index(&clause_tokens, |t| {
        RESOURCE_TOP_WORD_PATTERN.matches_token(t)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if top_idx + 1 >= clause_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing look top noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let count_before_top = parse_value(&clause_tokens[..top_idx]).and_then(|(value, used)| {
        let mut probe = used;
        if !clause_tokens
            .get(probe)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| RESOURCE_CARD_OR_CARDS_WORD_PATTERN.matches_word(w))
        {
            return None;
        }
        probe += 1;
        if clause_tokens
            .get(probe)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| w == "from")
        {
            probe += 1;
        }
        while clause_tokens
            .get(probe)
            .is_some_and(|t| RESOURCE_ARTICLE_WORD_PATTERN.matches_token(t))
        {
            probe += 1;
        }
        (probe == top_idx).then_some(value)
    });

    let mut idx = top_idx + 1;
    let count = if let Some(value) = count_before_top {
        value
    } else {
        let count = if clause_tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| RESOURCE_CARD_OR_CARDS_WORD_PATTERN.matches_word(w))
        {
            Value::Fixed(1)
        } else {
            let (value, used) = parse_value(&clause_tokens[idx..]).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing look count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            idx += used;
            value
        };

        // Consume "card(s)"
        if clause_tokens
            .get(idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|w| RESOURCE_CARD_OR_CARDS_WORD_PATTERN.matches_word(w))
        {
            idx += 1;
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing look card noun (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        count
    };

    // Consume "of <player> library"
    if !clause_tokens
        .get(idx)
        .is_some_and(|t| RESOURCE_OF_WORD_PATTERN.matches_token(t))
    {
        return Err(CardTextError::ParseError(format!(
            "missing 'of' in look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;
    let mut owner_tokens = &clause_tokens[idx..];
    while owner_tokens
        .first()
        .is_some_and(|t| RESOURCE_ARTICLE_WORD_PATTERN.matches_token(t))
    {
        owner_tokens = &owner_tokens[1..];
    }
    let owner_word_storage = TokenWordView::new(owner_tokens).owned_words();
    let owner_words = owner_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (player, used_words) = parse_library_owner(&owner_words)
        .or_else(|| {
            // If the clause uses a subject ("target player looks ..."), treat that as the default.
            subject.and_then(|s| match s {
                SubjectAst::Player(p) => Some((p, 0)),
                _ => None,
            })
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported look library owner (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    // No trailing words supported for now (based on word tokens).
    if used_words < owner_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if matches!(player, PlayerAst::Any) {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                TagKey::from(IT_TAG),
            )],
        });
    }

    Ok(EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        TagKey::from(IT_TAG),
    ))
}

pub(crate) fn parse_reorder(
    tokens: &[OwnedLexToken],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause = crate::runtime_backend::token_word_refs(tokens).join(" ");
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing reorder target".to_string(),
        ));
    }

    let Some((player, consumed)) = parse_graveyard_owner_prefix(&clause_words) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    };
    if !matches!(
        player,
        PlayerAst::You | PlayerAst::That | PlayerAst::ItsController | PlayerAst::ItsOwner
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause (clause: '{clause}')"
        )));
    }
    let rest = &clause_words[consumed..];

    if !rest.is_empty()
        && !RESOURCE_AS_YOU_CHOOSE_PATTERN.matches_words(rest)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported reorder clause tail (clause: '{clause}')"
        )));
    }

    Ok(EffectAst::subject_verb_reorder_graveyard(player))
}

pub(crate) fn parse_shuffle(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    #[derive(Clone, Copy)]
    enum LibraryDestinationPlayer {
        Default,
        You,
        DefaultOrController,
        That,
        ItsOwner,
    }

    const LIBRARY_DESTINATION_PLAYER_PHRASES: &[(&[&str], LibraryDestinationPlayer)] = &[
        (&["library"], LibraryDestinationPlayer::Default),
        (&["your", "library"], LibraryDestinationPlayer::You),
        (
            &["their", "library"],
            LibraryDestinationPlayer::DefaultOrController,
        ),
        (
            &["that", "player", "library"],
            LibraryDestinationPlayer::That,
        ),
        (
            &["that", "players", "library"],
            LibraryDestinationPlayer::That,
        ),
        (&["its", "owner", "library"], LibraryDestinationPlayer::ItsOwner),
        (
            &["its", "owners", "library"],
            LibraryDestinationPlayer::ItsOwner,
        ),
        (
            &["his", "or", "her", "library"],
            LibraryDestinationPlayer::DefaultOrController,
        ),
    ];

    const SUPPORTED_SHUFFLE_SOURCE_TAILS: &[&[&str]] = &[
        &[],
        &["from", "graveyard"],
        &["from", "your", "graveyard"],
        &["from", "their", "graveyard"],
        &["from", "that", "player", "graveyard"],
        &["from", "that", "players", "graveyard"],
        &["from", "its", "owner", "graveyard"],
        &["from", "its", "owners", "graveyard"],
        &["from", "his", "or", "her", "graveyard"],
    ];

    fn default_or_controller_player(default_player: PlayerAst) -> PlayerAst {
        if matches!(default_player, PlayerAst::Implicit) {
            PlayerAst::ItsController
        } else {
            default_player
        }
    }

    fn resolve_library_destination_player(
        player: LibraryDestinationPlayer,
        default_player: PlayerAst,
    ) -> PlayerAst {
        match player {
            LibraryDestinationPlayer::Default => default_player,
            LibraryDestinationPlayer::You => PlayerAst::You,
            LibraryDestinationPlayer::DefaultOrController => {
                default_or_controller_player(default_player)
            }
            LibraryDestinationPlayer::That => PlayerAst::That,
            LibraryDestinationPlayer::ItsOwner => PlayerAst::ItsOwner,
        }
    }

    fn parse_library_destination_player(
        words: &[&str],
        default_player: PlayerAst,
    ) -> Option<(PlayerAst, usize)> {
        LIBRARY_DESTINATION_PLAYER_PHRASES
            .iter()
            .find_map(|(phrase, player)| {
                words.starts_with(phrase).then(|| {
                    (
                        resolve_library_destination_player(*player, default_player),
                        phrase.len(),
                    )
                })
            })
    }

    fn is_supported_shuffle_source_tail(words: &[&str]) -> bool {
        SUPPORTED_SHUFFLE_SOURCE_TAILS
            .iter()
            .any(|tail| *tail == words)
    }

    fn is_simple_library_phrase(words: &[&str]) -> bool {
        LIBRARY_DESTINATION_PLAYER_PHRASES
            .iter()
            .any(|(phrase, _)| *phrase == words)
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if tokens.is_empty() {
        // Support standalone "Shuffle." clauses. If the sentence includes an explicit player
        // subject, use it; otherwise return an implicit player that can be filled in by the
        // carry-context logic (and compiles to "you" by default).
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if let Some(into_idx) = find_index(&clause_words, |word| {
        RESOURCE_INTO_WORD_PATTERN.matches_word(word)
    }) {
        let target_words = &clause_words[..into_idx];
        let destination_words =
            crate::runtime_backend::util::non_article_word_refs(&clause_words[into_idx + 1..]);
        if matches!(
            target_words,
            ["it"] | ["them"] | ["that", "card"] | ["those", "cards"]
        ) && let Some((destination_player, consumed)) =
            parse_library_destination_player(&destination_words, player)
        {
            let trailing_words = &destination_words[consumed..];
            if is_supported_shuffle_source_tail(trailing_words) {
                return Ok(EffectAst::ForEachTagged {
                    tag: TagKey::from(IT_TAG),
                    effects: vec![
                        EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                            Zone::Library,
                            false,
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
                });
            }
        }

        let consult_style_remainder_shuffle =
            RESOURCE_THE_REST_PREFIX_PATTERN.matches_words(&target_words)
                || RESOURCE_ALL_OTHER_REVEALED_OR_EXILED_CARDS_PATTERN.matches_words(&target_words);
        if consult_style_remainder_shuffle
            && let Some((destination_player, consumed)) =
                parse_library_destination_player(&destination_words, player)
            && is_supported_shuffle_source_tail(&destination_words[consumed..])
        {
            return Ok(subject_verb_player_resource_effect(
                SubjectVerbRoleAst::LibraryOwner,
                destination_player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }

    if matches!(player, PlayerAst::ItsOwner)
        && RESOURCE_ITS_OWNER_LIBRARY_TARGET_PATTERN.matches_words(&clause_words)
    {
        return Ok(EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
                    Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                subject_verb_player_resource_effect(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
            ],
        });
    }
    if RESOURCE_UNSUPPORTED_SHUFFLE_MARKER_PATTERN.matches_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported shuffle clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_simple_library_phrase(&clause_words) {
        return Ok(subject_verb_player_resource_effect(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported shuffle clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_goad(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError("missing goad target".to_string()));
    }

    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if let Some(target) = parse_chosen_name_goad_target(&target_tokens, &target_words)? {
        return Ok(EffectAst::subject_verb_goad(target));
    }
    if RESOURCE_IT_OR_THEM_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::subject_verb_goad(TargetAst::Tagged(
            TagKey::from(IT_TAG),
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
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_goad(target))
}

fn parse_chosen_name_goad_target(
    target_tokens: &[OwnedLexToken],
    target_words: &[&str],
) -> Result<Option<TargetAst>, CardTextError> {
    for with_word_idx in 0..target_words.len() {
        if !RESOURCE_WITH_WORD_PATTERN.matches_word(target_words[with_word_idx]) {
            continue;
        }

        let tail = strip_leading_article_word_refs(&target_words[with_word_idx + 1..]);
        let chosen_name_tail = tail.len() >= 5
            && RESOURCE_NAME_OR_NAMES_WORD_PATTERN.matches_word(tail[0])
            && RESOURCE_CHOSEN_NAME_TAIL_PATTERN.matches_words(&tail[1..])
            && RESOURCE_CHOSEN_NAME_OBJECT_NOUN_WORD_PATTERN.matches_word(tail[4])
            && word_slice_all_words_are_any(&tail[5..], &["this", "way"]);
        if !chosen_name_tail {
            continue;
        }

        let Some(with_token_idx) = token_index_for_word_index(target_tokens, with_word_idx) else {
            continue;
        };
        let base_tokens = trim_commas(&target_tokens[..with_token_idx]);
        if base_tokens.is_empty() {
            continue;
        }

        let mut target = parse_target_phrase(&base_tokens)?;
        add_chosen_name_constraint_to_target(&mut target);
        return Ok(Some(target));
    }

    Ok(None)
}

fn add_chosen_name_constraint_to_target(target: &mut TargetAst) {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from("__chosen_name__"),
                relation: TaggedOpbjectRelation::SameNameAsTagged,
            });
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            add_chosen_name_constraint_to_target(inner);
        }
        _ => {}
    }
}

pub(crate) fn parse_detain(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing detain target".to_string(),
        ));
    }

    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if RESOURCE_IT_OR_THEM_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::subject_verb_detain(TargetAst::Tagged(
            TagKey::from(IT_TAG),
            span_from_tokens(&target_tokens),
        )));
    }

    Ok(EffectAst::subject_verb_detain(parse_target_phrase(
        &target_tokens,
    )?))
}

pub(crate) fn parse_suspect(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing suspect target".to_string(),
        ));
    }

    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if RESOURCE_IT_OR_THEM_PATTERN.matches_words(&target_words) {
        return Ok(EffectAst::subject_verb_suspect(TargetAst::Tagged(
            TagKey::from(IT_TAG),
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
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_suspect(target))
}
