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
const THAT_PLAYER_PREFIXES: &[&[&str]] = &[&["that", "player"], &["that", "players"]];
const EVENT_AMOUNT_PREFIXES: &[&[&str]] = &[&["that", "much"], &["that", "many"]];
const DAMAGE_TO_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[&["damage", "to", "each", "opponent"]];
const EACH_OF_PREFIXES: &[&[&str]] = &[&["each", "of"]];
const ANY_NUMBER_OF_PREFIXES: &[&[&str]] = &[&["any", "number", "of"]];
const YOU_CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controlled"]];
const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"]];
const EACH_OPPONENT_AND_EACH_PREFIXES: &[&[&str]] = &[&["each", "opponent", "and", "each"]];
const FIRST_CARD_YOU_DRAW_PREFIXES: &[&[&str]] = &[&["the", "first", "card", "you", "draw"]];

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
        if subject.is_some() { "explicit" } else { "implicit" }
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
            if tokens.first().is_some_and(|token| token.is_word("control")) {
                parse_gain_control(tokens, subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            let has_onto = tokens.iter().any(|token| token.is_word("onto"));
            let has_counter_words = tokens
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"));

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
    }
}

fn parse_take(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        ["an", "extra", "turn", "after", "this", "one"]
    ) {
        return Ok(EffectAst::subject_verb_extra_turn_after_turn(extract_subject_player(subject).unwrap_or(PlayerAst::You), ExtraTurnAnchorAst::CurrentTurn));
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
    let trailing_ok = trailing_words.is_empty()
        || trailing_words.as_slice() == ["time"]
        || trailing_words.as_slice() == ["times"];
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
        token.is_word("library") || token.is_word("libraries")
    })?;
    let tail_tokens = trim_commas(&tokens[library_idx + 1..]);
    if tail_tokens.is_empty() {
        return None;
    }

    let filtered_tail: Vec<&str> = crate::runtime_backend::token_word_refs(&tail_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let fixed_position = match filtered_tail.as_slice() {
        ["second", "from", "top"] => Some(2),
        ["third", "from", "top"] => Some(3),
        ["fourth", "from", "top"] => Some(4),
        ["fifth", "from", "top"] => Some(5),
        _ => None,
    };
    if let Some(position) = fixed_position {
        return Some(Value::Fixed(position));
    }

    let amount_start = match filtered_tail.as_slice() {
        ["just", "beneath", "top", ..] => Some(3usize),
        ["beneath", "top", ..] => Some(2usize),
        _ => None,
    }?;
    let amount_tokens = filtered_tail[amount_start..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (amount, used) = parse_value(&amount_tokens)?;
    let amount_words = crate::runtime_backend::token_word_refs(&amount_tokens);
    if !matches!(amount_words.get(used).copied(), Some("card" | "cards")) {
        return None;
    }
    if used + 1 > amount_words.len() {
        return None;
    }
    if amount_words[used + 1..] != ["of", "that", "library"] {
        return None;
    }

    Some(Value::Add(Box::new(amount), Box::new(Value::Fixed(1))))
}

pub(crate) fn parse_look(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    fn parse_hand_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if slice_starts_with(&words, &["your", "hand"]) {
            return Some((PlayerAst::You, 2));
        }
        if slice_starts_with(&words, &["each", "player", "hand"])
            || slice_starts_with(&words, &["each", "players", "hand"])
        {
            return Some((PlayerAst::Any, 3));
        }
        if slice_starts_with(&words, &["their", "hand"]) {
            return Some((PlayerAst::That, 2));
        }
        if slice_starts_with(&words, &["that", "player", "hand"])
            || slice_starts_with(&words, &["that", "players", "hand"])
        {
            return Some((PlayerAst::That, 3));
        }
        if slice_starts_with(&words, &["target", "player", "hand"])
            || slice_starts_with(&words, &["target", "players", "hand"])
        {
            return Some((PlayerAst::Target, 3));
        }
        if slice_starts_with(&words, &["target", "opponent", "hand"])
            || slice_starts_with(&words, &["target", "opponents", "hand"])
        {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if slice_starts_with(&words, &["opponent", "hand"])
            || slice_starts_with(&words, &["opponents", "hand"])
        {
            return Some((PlayerAst::Opponent, 2));
        }
        if slice_starts_with(&words, &["his", "or", "her", "hand"]) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    fn parse_library_owner(words: &[&str]) -> Option<(PlayerAst, usize)> {
        if slice_starts_with(&words, &["your", "library"]) {
            return Some((PlayerAst::You, 2));
        }
        if slice_starts_with(&words, &["each", "player", "library"])
            || slice_starts_with(&words, &["each", "players", "library"])
        {
            return Some((PlayerAst::Any, 3));
        }
        if slice_starts_with(&words, &["their", "library"]) {
            return Some((PlayerAst::That, 2));
        }
        if slice_starts_with(&words, &["that", "player", "library"])
            || slice_starts_with(&words, &["that", "players", "library"])
        {
            return Some((PlayerAst::That, 3));
        }
        if slice_starts_with(&words, &["target", "player", "library"])
            || slice_starts_with(&words, &["target", "players", "library"])
        {
            return Some((PlayerAst::Target, 3));
        }
        if slice_starts_with(&words, &["target", "opponent", "library"])
            || slice_starts_with(&words, &["target", "opponents", "library"])
        {
            return Some((PlayerAst::TargetOpponent, 3));
        }
        if slice_starts_with(&words, &["its", "owner", "library"])
            || slice_starts_with(&words, &["its", "owners", "library"])
        {
            return Some((PlayerAst::ItsOwner, 3));
        }
        if slice_starts_with(&words, &["his", "or", "her", "library"]) {
            return Some((PlayerAst::That, 4));
        }
        None
    }

    // "Look at the top N cards of your library."
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("at"))
    {
        clause_tokens = trim_commas(&clause_tokens[1..]);
    }
    let clause_word_storage = TokenWordView::new(&clause_tokens).owned_words();
    let clause_words = clause_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let mut hand_tokens = clause_tokens.clone();
    while hand_tokens
        .first()
        .is_some_and(|token| token.is_word("the") || token.is_word("a") || token.is_word("an"))
    {
        hand_tokens = hand_tokens[1..].to_vec();
    }
    let hand_word_storage = TokenWordView::new(&hand_tokens).owned_words();
    let hand_words = hand_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if let Some((player, used_words)) = parse_hand_owner(&hand_words) {
        if used_words < hand_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing look clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let target = match player {
            PlayerAst::You => TargetAst::Player(PlayerFilter::You, None),
            PlayerAst::Opponent => TargetAst::Player(PlayerFilter::Opponent, None),
            PlayerAst::Target => TargetAst::Player(
                PlayerFilter::target_player(),
                span_from_tokens(&hand_tokens),
            ),
            PlayerAst::TargetOpponent => TargetAst::Player(
                PlayerFilter::target_opponent(),
                span_from_tokens(&hand_tokens),
            ),
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

        return Ok(EffectAst::subject_verb_look_at_hand(target));
    }

    let Some(top_idx) = find_index(&clause_tokens, |t| t.is_word("top")) else {
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

    let mut idx = top_idx + 1;
    let count = if clause_tokens
        .get(idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|w| w == "card" || w == "cards")
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
        .is_some_and(|w| w == "card" || w == "cards")
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing look card noun (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    // Consume "of <player> library"
    if !clause_tokens.get(idx).is_some_and(|t| t.is_word("of")) {
        return Err(CardTextError::ParseError(format!(
            "missing 'of' in look clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;
    let mut owner_tokens = &clause_tokens[idx..];
    while owner_tokens
        .first()
        .is_some_and(|t| t.is_word("the") || t.is_word("a") || t.is_word("an"))
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

    Ok(EffectAst::subject_verb_look_at_top_cards(player, count, TagKey::from(IT_TAG)))
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

    if !rest.is_empty() && rest != ["as", "you", "choose"] {
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
    fn parse_library_destination_player(
        words: &[&str],
        default_player: PlayerAst,
    ) -> Option<(PlayerAst, usize)> {
        match words {
            ["library", ..] => Some((default_player, 1)),
            ["your", "library", ..] => Some((PlayerAst::You, 2)),
            ["their", "library", ..] => Some((
                if matches!(default_player, PlayerAst::Implicit) {
                    PlayerAst::ItsController
                } else {
                    default_player
                },
                2,
            )),
            ["that", "player", "library", ..] => Some((PlayerAst::That, 3)),
            ["that", "players", "library", ..] => Some((PlayerAst::That, 3)),
            ["its", "owner", "library", ..] => Some((PlayerAst::ItsOwner, 3)),
            ["its", "owners", "library", ..] => Some((PlayerAst::ItsOwner, 3)),
            ["his", "or", "her", "library", ..] => Some((
                if matches!(default_player, PlayerAst::Implicit) {
                    PlayerAst::ItsController
                } else {
                    default_player
                },
                4,
            )),
            _ => None,
        }
    }

    fn is_supported_shuffle_source_tail(words: &[&str]) -> bool {
        matches!(
            words,
            [] | ["from", "graveyard"]
                | ["from", "your", "graveyard"]
                | ["from", "their", "graveyard"]
                | ["from", "that", "player", "graveyard"]
                | ["from", "that", "players", "graveyard"]
                | ["from", "its", "owner", "graveyard"]
                | ["from", "its", "owners", "graveyard"]
                | ["from", "his", "or", "her", "graveyard"]
        )
    }

    fn is_simple_library_phrase(words: &[&str]) -> bool {
        matches!(
            words,
            ["library"]
                | ["your", "library"]
                | ["their", "library"]
                | ["that", "player", "library"]
                | ["that", "players", "library"]
                | ["its", "owner", "library"]
                | ["its", "owners", "library"]
                | ["his", "or", "her", "library"]
        )
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
    if let Some(into_idx) = find_index(&clause_words, |word| *word == "into") {
        let target_words = &clause_words[..into_idx];
        let destination_words: Vec<&str> = clause_words[into_idx + 1..]
            .iter()
            .copied()
            .filter(|word| !is_article(word))
            .collect();
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
                            TargetAst::Tagged(
                                TagKey::from(IT_TAG),
                                span_from_tokens(tokens),
                            ),
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

        let consult_style_remainder_shuffle = slice_starts_with(&target_words, &["the", "rest"])
            || (slice_starts_with(&target_words, &["all", "other"])
                && slice_contains(&target_words, &"cards")
                && (slice_contains(&target_words, &"revealed")
                    || slice_contains(&target_words, &"exiled")));
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
        && matches!(
            clause_words.as_slice(),
            ["them", "into", "their", "libraries"]
                | ["them", "into", "their", "library"]
                | ["those", "cards", "into", "their", "libraries"]
                | ["those", "cards", "into", "their", "library"]
        )
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
    if grammar::contains_word(tokens, "graveyard")
        || grammar::contains_word(tokens, "cards")
        || grammar::contains_word(tokens, "card")
        || grammar::contains_word(tokens, "into")
        || grammar::contains_word(tokens, "from")
    {
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
    if target_words.as_slice() == ["it"] || target_words.as_slice() == ["them"] {
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
        if target_words[with_word_idx] != "with" {
            continue;
        }

        let tail = &target_words[with_word_idx + 1..];
        let name_word_idx = if tail.first().is_some_and(|word| is_article(word)) {
            1
        } else {
            0
        };
        let chosen_name_tail = tail
            .get(name_word_idx..)
            .is_some_and(|tail| {
                tail.len() >= 5
                    && matches!(tail[0], "name" | "names")
                    && tail[1] == "chosen"
                    && tail[2] == "for"
                    && tail[3] == "this"
                    && matches!(
                        tail[4],
                        "artifact" | "card" | "creature" | "enchantment" | "permanent" | "source"
                    )
                    && tail[5..]
                        .iter()
                        .all(|word| matches!(*word, "this" | "way"))
            });
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
    if matches!(target_words.as_slice(), ["it"] | ["them"]) {
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
    if matches!(target_words.as_slice(), ["it"] | ["them"]) {
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
