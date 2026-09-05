use super::*;

pub(super) fn parse_value_expr_term_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.is_empty() {
        return None;
    }
    if let Some(devotion) = parse_devotion_value_words(words) {
        return Some(devotion);
    }
    for (phrase, player) in [
        (
            &[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "the", "opponent",
                "with", "the", "most", "cards", "in", "hand",
            ][..],
            PlayerFilter::Opponent,
        ),
        (
            &[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "an", "opponent",
                "with", "the", "most", "cards", "in", "hand",
            ][..],
            PlayerFilter::Opponent,
        ),
        (
            &[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "the", "player", "with",
                "the", "most", "cards", "in", "hand",
            ][..],
            PlayerFilter::Any,
        ),
    ] {
        if permission_shapes::prefix_words(words, phrase) {
            return Some((Value::MaxCardsInHand(player), phrase.len()));
        }
    }
    if let Some(value) = colored_mana_symbols_in_costs(words) {
        return Some(value);
    }
    if permission_shapes::prefix_words(words, &["half"]) {
        if let Some((round_idx, rounding)) = first_rounding(&words[1..]) {
            let round_idx = round_idx + 1;
            let (base, used_inner) = parse_value_expr_term_words(&words[1..round_idx])?;
            if used_inner != round_idx - 1 {
                return None;
            }
            return Some((rounded_half(base, rounding), round_idx + 2));
        }
        let (base, used_inner) = parse_value_expr_term_words(&words[1..])?;
        let used = 1 + used_inner;
        if permission_shapes::starts_at_words(words, used, &["rounded", "down"]) {
            return Some((rounded_half(base, Rounding::Down), used + 2));
        }
        if permission_shapes::starts_at_words(words, used, &["rounded", "up"]) {
            return Some((rounded_half(base, Rounding::Up), used + 2));
        }
    }

    if let Some((_, used)) = DAMAGE_EVENT_AMOUNT_PREFIXES
        .iter()
        .find(|(expected, _)| permission_shapes::prefix_words(words, expected))
    {
        return Some((
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ValueSurfaceHint::DamageDealt),
            *used,
        ));
    }

    if let Some(used) = prefix_len(
        words,
        &[&["the", "result"], &["that", "result"], &["result"]],
    ) {
        return Some((
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ValueSurfaceHint::PriorEffectResult),
            used,
        ));
    }

    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "excess", "damage", "dealt", "to", "that", "creature", "this", "way",
            ],
            &[
                "excess", "damage", "dealt", "to", "that", "creature", "this", "way",
            ],
            &["the", "excess", "damage", "dealt", "this", "way"],
            &["excess", "damage", "dealt", "this", "way"],
            &["that", "amount", "of", "excess", "damage"],
        ],
    ) {
        return Some((
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::ExcessDamage,
            },
            used,
        ));
    }

    if let Some((_, used)) = EVENT_AMOUNT_PREFIXES
        .iter()
        .find(|(expected, _)| permission_shapes::prefix_words(words, expected))
    {
        return Some((Value::EventValue(EventValueSpec::Amount), *used));
    }
    if permission_shapes::prefix_words(words, &["the", "other", "result"]) {
        return Some((
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::OtherNumber,
            },
            3,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "amount", "of", "mana", "spent", "to", "cast", "that", "spell",
            ],
            &[
                "amount", "of", "mana", "spent", "to", "cast", "that", "spell",
            ],
        ],
    ) {
        return Some((Value::ManaSpentToCastTriggeringObject, used));
    }
    if words.len() >= 5
        && (permission_shapes::prefix_words(words, &["the", "number", "of"])
            || permission_shapes::prefix_words(words, &["number", "of"]))
        && permission_shapes::suffix_words(words, &["removed", "this", "way"])
    {
        return Some((Value::EventValue(EventValueSpec::Amount), words.len()));
    }
    if permission_shapes::prefix_words(words, &["twice", "x"]) {
        return Some((Value::XTimes(2), 2));
    }
    if permission_shapes::prefix_words(words, &["twice"]) {
        let (value, used) = parse_value_expr_term_words(&words[1..])?;
        return Some((Value::Scaled(Box::new(value), 2), used + 1));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "number", "of", "times", "this", "creature", "has", "mutated",
            ],
            &[
                "the",
                "number",
                "of",
                "times",
                "this",
                "permanent",
                "has",
                "mutated",
            ],
            &[
                "number", "of", "times", "this", "creature", "has", "mutated",
            ],
            &[
                "number",
                "of",
                "times",
                "this",
                "permanent",
                "has",
                "mutated",
            ],
            &[
                "number", "of", "times", "this", "creature", "has", "mutated",
            ],
            &["number", "of", "times", "this", "has", "mutated"],
            &["times", "this", "creature", "has", "mutated"],
            &["times", "this", "permanent", "has", "mutated"],
            &["times", "this", "has", "mutated"],
        ],
    ) {
        return Some((Value::SourceMutationCount, used));
    }
    if permission_shapes::prefix_words(words, &["x"]) {
        return Some((Value::X, 1));
    }
    if let Ok(value) = leaf::parse_number_i32_complete(words[0]) {
        return Some((Value::Fixed(value), 1));
    }

    for (characteristic, constructor) in [
        (
            &["mana", "value"][..],
            Value::ManaValueOf as fn(Box<ChooseSpec>) -> Value,
        ),
        (
            &["power"][..],
            Value::PowerOf as fn(Box<ChooseSpec>) -> Value,
        ),
        (
            &["toughness"][..],
            Value::ToughnessOf as fn(Box<ChooseSpec>) -> Value,
        ),
    ] {
        if let Some((kind, used)) =
            sacrificed_postpositive_characteristic_prefix(words, characteristic)
        {
            return Some((
                constructor(Box::new(ChooseSpec::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                )))
                .with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
                used,
            ));
        }
    }

    if let Some(used) = prefix_len(
        words,
        &[
            &["the", "amount", "of", "unspent", "mana", "you", "have"],
            &["amount", "of", "unspent", "mana", "you", "have"],
            &["unspent", "mana", "you", "have"],
        ],
    ) {
        return Some((Value::UnspentMana(PlayerFilter::You), used));
    }
    if permission_shapes::prefix_words(words, &["your", "life", "total"]) {
        return Some((Value::LifeTotal(PlayerFilter::You), 3));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "amount", "of", "life", "you", "gained", "this", "turn",
            ],
            &["amount", "of", "life", "you", "gained", "this", "turn"],
        ],
    ) {
        return Some((Value::LifeGainedThisTurn(PlayerFilter::You), used));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["target", "players", "life", "total"],
            &["target", "player", "life", "total"],
            &["that", "players", "life", "total"],
            &["that", "player", "life", "total"],
        ],
    ) {
        return Some((Value::LifeTotal(PlayerFilter::target_player()), used));
    }
    if permission_shapes::prefix_words(words, &["your", "speed"]) {
        return Some((Value::Speed(PlayerFilter::You), 2));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["target", "players", "speed"],
            &["target", "player", "speed"],
            &["that", "players", "speed"],
            &["that", "player", "speed"],
        ],
    ) {
        return Some((Value::Speed(PlayerFilter::target_player()), used));
    }

    for source_len in (1..words.len()).rev() {
        if let Some(surface) = source_reference_surface_for_possessive_words(&words[..source_len]) {
            match words.get(source_len).copied() {
                Some("power") => {
                    return Some((
                        Value::PowerOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 1,
                    ));
                }
                Some("toughness") => {
                    return Some((
                        Value::ToughnessOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 1,
                    ));
                }
                Some("mana")
                    if permission_shapes::starts_at_words(words, source_len + 1, &["value"]) =>
                {
                    return Some((
                        Value::ManaValueOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 2,
                    ));
                }
                _ => {}
            }
        }
    }

    if permission_shapes::prefix_words(words, &["its", "power"]) {
        return Some((
            Value::PowerOf(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind()).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            2,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["this", "power"],
            &["thiss", "power"],
            &["this", "creature", "power"],
            &["thiss", "creature", "power"],
            &["this", "creatures", "power"],
            &["thiss", "creatures", "power"],
            &["this", "permanent", "power"],
            &["thiss", "permanent", "power"],
            &["this", "permanents", "power"],
            &["thiss", "permanents", "power"],
        ],
    ) {
        return Some((Value::SourcePower, used));
    }
    if permission_shapes::prefix_words(words, &["his", "power"]) {
        return Some((
            Value::SourcePower
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::MasculineSourcePossessive),
            2,
        ));
    }
    if permission_shapes::prefix_words(words, &["her", "power"]) {
        return Some((
            Value::SourcePower
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::FeminineSourcePossessive),
            2,
        ));
    }
    if permission_shapes::prefix_words(words, &["its", "toughness"]) {
        return Some((
            Value::ToughnessOf(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind()).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            2,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["this", "toughness"],
            &["thiss", "toughness"],
            &["this", "creature", "toughness"],
            &["thiss", "creature", "toughness"],
            &["this", "creatures", "toughness"],
            &["thiss", "creatures", "toughness"],
            &["this", "permanent", "toughness"],
            &["thiss", "permanent", "toughness"],
            &["this", "permanents", "toughness"],
            &["thiss", "permanents", "toughness"],
        ],
    ) {
        return Some((Value::SourceToughness, used));
    }
    if permission_shapes::prefix_words(words, &["its", "mana", "value"]) {
        return Some((
            Value::ManaValueOf(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind()).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            3,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["this", "mana", "value"],
            &["thiss", "mana", "value"],
            &["this", "creature", "mana", "value"],
            &["thiss", "creature", "mana", "value"],
            &["this", "creatures", "mana", "value"],
            &["thiss", "creatures", "mana", "value"],
        ],
    ) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            ))),
            used,
        ));
    }
    // In an attack-group trigger, plural "their" denotes the creatures that
    // jointly satisfied the one-or-more trigger, not every creature currently
    // on the battlefield.  Trigger queuing captures that exact group under
    // crate::tag::CompilerReferenceTag::AttackingGroup.as_str() so the value remains stable while the ability is on
    // the stack.
    if permission_shapes::prefix_words(words, &["their", "total", "power"]) {
        return Some((
            Value::TotalPower(crate::target::ObjectFilter::tagged(
                crate::tag::CompilerReferenceTag::AttackingGroup.bind(),
            )),
            3,
        ));
    }
    if let Some(used) = prefix_len(words, COLORS_SPENT_PREFIXES) {
        return Some((Value::ColorsOfManaSpentToCastThisSpell, used));
    }
    const ITERATED_PLAYER_EXILED_OBJECT_POWER: &[&str] =
        &["the", "power", "of", "the", "creature", "they", "exiled"];
    if permission_shapes::prefix_words(words, ITERATED_PLAYER_EXILED_OBJECT_POWER) {
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::FirstPower,
        )
        .with_filter(ObjectFilter::creature())
        .with_player(PlayerFilter::IteratedPlayer)
        .with_action(ironsmith_core::PriorEffectAction::Exiled);
        return Some((
            Value::PendingPriorEffectMetric(query),
            ITERATED_PLAYER_EXILED_OBJECT_POWER.len(),
        ));
    }
    if let Some(used) = prefix_len(words, TAGGED_POWER_PREFIXES) {
        let tag = tagged_characteristic_reference_tag(&words[..used]);
        return Some((
            with_sacrificed_object_surface(
                Value::PowerOf(Box::new(ChooseSpec::Tagged(tag.bind()))),
                &words[..used],
            ),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, TAGGED_TOUGHNESS_PREFIXES) {
        let tag = tagged_characteristic_reference_tag(&words[..used]);
        return Some((
            with_sacrificed_object_surface(
                Value::ToughnessOf(Box::new(ChooseSpec::Tagged(tag.bind()))),
                &words[..used],
            ),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, EXILED_MANA_VALUE_PREFIXES) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                crate::tag::CompilerReferenceTag::SourceExiled.bind(),
            ))),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, REVEALED_MANA_VALUE_PREFIXES) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                crate::tag::CompilerReferenceTag::PublicRevealed.bind(),
            )))
            .with_surface_hint(ValueSurfaceHint::RevealedCardReference),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, TAGGED_MANA_VALUE_PREFIXES) {
        return Some((
            with_sacrificed_object_surface(
                Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                ))),
                &words[..used],
            ),
            used,
        ));
    }
    if let Some(value) = value_helper_shapes::parse_aggregate_scope_value_words(words) {
        return Some((value, words.len()));
    }
    if let Some(value) = value_helper_shapes::parse_spells_cast_this_turn_value_words(words) {
        return Some((value, words.len()));
    }

    parse_number_of_value(words)
}

pub(super) fn parse_number_of_value(words: &[&str]) -> Option<(Value, usize)> {
    let mut idx = usize::from(permission_shapes::prefix_words(words, &["the"]));
    if !permission_shapes::starts_at_words(words, idx, &["number", "of"]) {
        return None;
    }
    idx += 2;
    let characteristic_tail = &words[idx..];
    if characteristic_tail.len() >= 4
        && crate::word_primitives::first_is_any(characteristic_tail, &["color", "colors"])
        && crate::word_primitives::at_is_any(characteristic_tail, 1, &["that", "the"])
        && crate::word_primitives::at_is_any(characteristic_tail, 3, &["was", "were"])
    {
        let mut filter = crate::grammar::primitives::probe_shape(parse_object_filter_words(
            &words[idx + 2..idx + 3],
            false,
        ))?;
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::Sacrificed0.bind(),
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        );
        return Some((Value::ColorsAmong(filter), idx + 4));
    }
    // A singular discarded-card characteristic is a metric over the exact
    // result of the preceding discard, not a count of live objects matching
    // the words `card types`. Keeping the action on the pending query lets
    // reference resolution bind it to the producing discard effect.
    const DISCARDED_CARD_TYPES: &[&str] = &["card", "types", "the", "discarded", "card", "has"];
    if permission_shapes::starts_at_words(words, idx, DISCARDED_CARD_TYPES) {
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::CardTypesAmong,
        )
        .with_action(ironsmith_core::PriorEffectAction::Discarded);
        return Some((
            Value::PendingPriorEffectMetric(query),
            idx + DISCARDED_CARD_TYPES.len(),
        ));
    }
    for visit_surface in [
        &["attractions", "youve", "visited", "this", "turn"][..],
        &["attractions", "you've", "visited", "this", "turn"][..],
        &["attraction", "youve", "visited", "this", "turn"][..],
        &["attraction", "you've", "visited", "this", "turn"][..],
    ] {
        if permission_shapes::starts_at_words(words, idx, visit_surface) {
            return Some((
                Value::AttractionsVisitedThisTurn(PlayerFilter::You),
                idx + visit_surface.len(),
            ));
        }
    }
    if let Some(character_word) = words.get(idx)
        && let Some(character) = character_word
            .strip_suffix("'s")
            .or_else(|| character_word.strip_suffix("’s"))
            .or_else(|| character_word.strip_suffix('s'))
        && character.chars().count() == 1
        && character.chars().all(|character| character.is_alphabetic())
        && words.get(idx + 1..idx + 5) == Some(&["in", "name", "stickers", "on"][..])
    {
        let reference_start = idx + 5;
        let reference_end = value_boundary(&words[reference_start..]) + reference_start;
        let reference = words.get(reference_start..reference_end)?;
        let surface = source_reference_surface_for_words(reference)
            .or_else(|| this_source_surface_for_words(reference))?;
        return Some((
            Value::NameStickerCharacterCountOnSource {
                character: character.chars().next()?.to_ascii_lowercase(),
                surface: Some(surface),
            },
            reference_end,
        ));
    }
    let mut counter_descriptor_start = idx;
    if words
        .get(counter_descriptor_start)
        .is_some_and(|word| leaf::parse_leaf_article_complete(word).is_ok())
        || permission_shapes::starts_at_words(words, counter_descriptor_start, &["one"])
    {
        counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative| counter_descriptor_start + relative)
        .filter(|counter_idx| counter_idx.saturating_sub(counter_descriptor_start) <= 2)
        && let Some(counter_type) = (counter_idx > counter_descriptor_start)
            .then(|| parse_counter_type_words(&words[counter_descriptor_start..=counter_idx]))
            .flatten()
    {
        if permission_shapes::starts_at_words(words, counter_idx + 1, &["you", "have"]) {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 3,
            ));
        }
        if words
            .get(counter_idx + 1)
            .is_some_and(|word| matches!(*word, "youve" | "you've"))
        {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 2,
            ));
        }
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative| counter_descriptor_start + relative)
        .filter(|counter_idx| counter_idx.saturating_sub(counter_descriptor_start) <= 2)
        && permission_shapes::starts_at_words(words, counter_idx + 1, &["on"])
    {
        let parsed_counter_type = (counter_idx > counter_descriptor_start)
            .then(|| parse_counter_type_words(&words[counter_descriptor_start..=counter_idx]))
            .flatten();
        let reference_start = counter_idx + 2;
        let reference_end = value_boundary(&words[reference_start..]) + reference_start;
        let reference = &words[reference_start..reference_end];
        if is_source_counter_reference(reference) {
            let value = match parsed_counter_type {
                Some(counter_type) => {
                    if let Some(surface) = source_reference_surface_for_words(reference) {
                        Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        )
                    } else {
                        Value::CountersOnSource(counter_type)
                    }
                }
                None => Value::CountersOn(
                    Box::new(
                        source_reference_surface_for_words(reference)
                            .map(source_choose_spec_for_surface)
                            .unwrap_or(ChooseSpec::Source),
                    ),
                    None,
                ),
            };
            return Some((value, reference_end));
        }
        if let Some(surface) = source_reference_surface_for_words(reference) {
            return Some((
                Value::CountersOn(
                    Box::new(source_choose_spec_for_surface(surface)),
                    parsed_counter_type,
                ),
                reference_end,
            ));
        }
        if is_tagged_counter_reference(reference) {
            return Some((
                Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                    )),
                    parsed_counter_type,
                ),
                reference_end,
            ));
        }
        if let Ok(filter) = parse_object_filter_words(reference, false) {
            return Some((
                Value::CountersOn(Box::new(ChooseSpec::All(filter)), parsed_counter_type),
                reference_end,
            ));
        }
    }

    let filter_start = idx;
    let filter_end = value_boundary(&words[filter_start..]) + filter_start;
    if filter_end <= filter_start {
        return None;
    }
    let filter_words = &words[filter_start..filter_end];
    let history_tokens = synthetic_word_tokens(filter_words);
    if permission_shapes::find_words(filter_words, &["this", "way"]).is_none()
        && let Some(value) =
            super::super::value_semantics::parse_turn_history_count_value(&history_tokens)
    {
        return Some((value, filter_end));
    }
    // Keep a qualifying hand-size predicate attached to the players it
    // describes. The generic object-filter fallback below otherwise turns
    // this into a count of cards across every player's hand.
    if let Some((players, minimum)) =
        super::super::value_semantics::parse_players_with_cards_in_hand_at_least(&history_tokens)
    {
        return Some((
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum),
            filter_end,
        ));
    }
    if exact_one_of(
        filter_words,
        &[
            &["creatures", "in", "your", "party"],
            &["creature", "in", "your", "party"],
        ],
    ) {
        return Some((Value::PartySize(PlayerFilter::You), filter_end));
    }
    if let Some(value) = value_helper_shapes::parse_aggregate_scope_value_words(filter_words) {
        return Some((value, filter_end));
    }
    if let Some(value) = value_helper_shapes::parse_spells_cast_this_turn_value_words(filter_words)
    {
        return Some((value, filter_end));
    }
    // In an amount modifying a player-directed action, plural `them` is the
    // same player antecedent. Curses are player attachments, so keep that
    // relation typed instead of allowing the generic object-pronoun parser to
    // manufacture an attached card selector.
    if crate::word_primitives::parse_choice_sequence_complete(
        filter_words,
        &[&["curse", "curses"], &["attached"], &["to"], &["them"]],
    ) {
        let mut filter = ObjectFilter::default().with_subtype(crate::Subtype::Curse);
        filter.zone = Some(crate::zone::Zone::Battlefield);
        filter.attached_to_player = Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
        return Some((Value::Count(filter), filter_end));
    }
    // A possessive target-controller hand is a player-relative zone scope,
    // not a characteristic on the counted cards. Parse it before the generic
    // object-filter fallback can absorb `that creature` as a Creature type.
    let possessive = possessive_normalized_word_refs(filter_words);
    if crate::word_primitives::parse_choice_sequence_complete(
        &possessive,
        &[
            &["cards", "card"],
            &["in"],
            &["that"],
            &[
                "creature",
                "creatures",
                "permanent",
                "permanents",
                "object",
                "objects",
            ],
            &["controller", "controllers"],
            &["hand", "hands"],
        ],
    ) {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(crate::zone::Zone::Hand);
        filter.owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
        return Some((Value::Count(filter), filter_end));
    }
    // The generic value-expression path runs before several effect-specific
    // value parsers. Preserve every typed prior-action link here so numeric
    // phrases such as "the number of creatures destroyed this way" and
    // "twice the number of Mountains returned this way" do not collapse to
    // live-zone object counts.
    if permission_shapes::find_words(filter_words, &["this", "way"]).is_some() {
        let mut for_each_words = Vec::with_capacity(filter_words.len() + 2);
        for_each_words.extend(["for", "each"]);
        for_each_words.extend(filter_words.iter().copied());
        if let Some((value @ Value::PendingPriorEffectMetric(_), used)) =
            super::super::count_shapes::parse_for_each_count_value_words(&for_each_words)
            && used == for_each_words.len()
        {
            return Some((value, filter_end));
        }
    }
    if let Some(mut filter) = parse_source_controller_graveyard_filter(filter_words) {
        filter.zone = Some(crate::zone::Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        return Some((Value::Count(filter), filter_end));
    }
    let filter =
        crate::grammar::primitives::probe_shape(parse_object_filter_words(filter_words, false))?;
    let mut value = Value::Count(filter);
    if value_helper_shapes::has_that_player_possessive(filter_words) {
        value = value.with_surface_hint(ValueSurfaceHint::ThatPlayerPossessive);
    }
    Some((value, filter_end))
}

pub(super) fn first_rounding(words: &[&str]) -> Option<(usize, Rounding)> {
    let down =
        permission_shapes::find_words(words, &["rounded", "down"]).map(|idx| (idx, Rounding::Down));
    let up =
        permission_shapes::find_words(words, &["rounded", "up"]).map(|idx| (idx, Rounding::Up));
    match (down, up) {
        (Some(down), Some(up)) => Some(if down.0 <= up.0 { down } else { up }),
        (Some(down), None) => Some(down),
        (None, Some(up)) => Some(up),
        (None, None) => None,
    }
}

pub(super) fn rounded_half(base: Value, rounding: Rounding) -> Value {
    match rounding {
        Rounding::Down => Value::HalfRoundedDown(Box::new(base)),
        Rounding::Up => Value::HalfRoundedDown(Box::new(Value::Add(
            Box::new(base),
            Box::new(Value::Fixed(1)),
        ))),
    }
}

pub(super) fn prefix_len(words: &[&str], alternatives: &[&[&str]]) -> Option<usize> {
    alternatives
        .iter()
        .find(|expected| permission_shapes::prefix_words(words, expected))
        .map(|expected| expected.len())
}

pub(super) fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

pub(super) fn value_boundary(words: &[&str]) -> usize {
    let arithmetic = ["plus", "minus"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
        .unwrap_or(words.len());
    let in_excess =
        permission_shapes::find_words(words, &["in", "excess", "of"]).unwrap_or(words.len());
    // A "from <zone>" right after a controller/owner clause is the enclosing
    // effect's movement source, never part of the count basis: "the number
    // of lands you control from your hand onto the battlefield" must count
    // battlefield lands, not land cards in hand.
    let movement_source = crate::slice_primitives::find_window_by(words, 2, |pair| {
        matches!(pair[0], "control" | "controls" | "own" | "owns") && pair[1] == "from"
    })
    .map(|idx| idx + 1)
    .unwrap_or(words.len());
    arithmetic.min(in_excess).min(movement_source)
}
