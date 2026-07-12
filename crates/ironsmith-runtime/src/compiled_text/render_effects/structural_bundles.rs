use super::*;

pub(super) fn choose_search_zones(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<Vec<Zone>> {
    let primary_zone = choose.filter.zone.or(choose.zone)?;
    let mut zones = vec![primary_zone];
    for zone in &choose.additional_zones {
        if !zones.contains(zone) {
            zones.push(*zone);
        }
    }
    Some(zones)
}

pub(super) fn search_split_filter_is_tagged_as(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(super) fn downcast_search_split_move_to_zone(
    effect: &Effect,
) -> Option<&crate::effects::MoveToZoneEffect> {
    unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
}

pub(super) fn search_split_move_to_zone_uses_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
    zone: Zone,
) -> bool {
    move_to_zone.zone == zone
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
}

/// "Exile target X. Search its controller's graveyard, hand, and library for
/// all cards / any number of cards with the same name as that X and exile
/// them. Then that player shuffles." (Eradicate, Splinter, Sowing Salt,
/// Scour, Crumble to Dust).
pub(super) fn describe_exile_target_search_same_name_exile_shuffle_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [exile_effect, search_effect, for_each_effect, shuffle_effect] = filtered else {
        return None;
    };
    let exile_tag = wrapped_effect_tag(exile_effect)?;
    let exile = downcast_search_split_move_to_zone(exile_effect)?;
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if exile.zone != Zone::Exile || !exile.target.is_target() {
        return None;
    }
    let ChooseSpec::Object(exiled_filter) = exile.target.base() else {
        return None;
    };
    let controller_of_target = PlayerFilter::ControllerOf(crate::target::ObjectRef::Target);
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || choose_search_zones(search)? != vec![Zone::Graveyard, Zone::Hand, Zone::Library]
        || search.count.min != 0
        || search.count.max.is_some()
        || search.filter.owner.as_ref() != Some(&controller_of_target)
        || !search.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                && constraint.tag == *exile_tag
        })
    {
        return None;
    }
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_exile = downcast_search_split_move_to_zone(move_effect)?;
    if for_each.tag != search.tag
        || !search_split_move_to_zone_uses_tag(move_to_exile, search.tag.as_str(), Zone::Exile)
        || shuffle.player != controller_of_target
    {
        return None;
    }

    let selection = match search.search_mode {
        SearchSelectionMode::Optional => "any number of cards",
        SearchSelectionMode::AllMatching | SearchSelectionMode::Exact => "all cards",
    };
    let noun = match exiled_filter.card_types.as_slice() {
        [card_type] => card_type.selection_name(),
        _ => "permanent",
    };
    Some(format!(
        "Exile {}. Search its controller's graveyard, hand, and library for {selection} with the same name as that {noun} and exile them. Then that player shuffles.",
        describe_choose_spec(&exile.target)
    ))
}

pub(super) fn describe_reveal_hand_choose_graveyard_exile_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        hand_choose_effect,
        graveyard_choose_effect,
        exile_effect,
    ] = filtered
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let hand_choose = hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let graveyard_choose =
        graveyard_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = downcast_search_split_move_to_zone(exile_effect)?;

    if !look.reveal
        || !matches!(
            look.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        )
        || hand_choose.chooser != PlayerFilter::You
        || graveyard_choose.chooser != PlayerFilter::You
        || choose_exact_count(hand_choose) != Some(1)
        || choose_exact_count(graveyard_choose) != Some(1)
        || choose_primary_zone(hand_choose) != Some(Zone::Hand)
        || choose_primary_zone(graveyard_choose) != Some(Zone::Graveyard)
        || hand_choose.filter.owner != Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        || graveyard_choose.filter.owner
            != Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        || hand_choose.filter.card_types != graveyard_choose.filter.card_types
        || !search_split_move_to_zone_uses_tag(exile, hand_choose.tag.as_str(), Zone::Exile)
        || hand_choose.tag != graveyard_choose.tag
    {
        return None;
    }

    let mut display_filter = hand_choose.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    display_filter.controller = None;
    let mut display_description = display_filter.description();
    if !display_description.contains("card") {
        display_description.push_str(" card");
    }
    let choice_text = with_indefinite_article(&display_description);

    Some(format!(
        "Target opponent reveals their hand. You choose {choice_text} from it, then choose {choice_text} from their graveyard. Exile the chosen cards."
    ))
}

pub(super) fn describe_choose_card_name_selection(
    choose_name: &crate::effects::ChooseCardNameEffect,
) -> String {
    if let Some(filter) = &choose_name.filter {
        let mut filter_text = strip_leading_article(&filter.description()).to_string();
        if filter.card_types.is_empty() {
            // Card names are card properties: the filter's default
            // battlefield noun ("permanent") reads wrong here.
            filter_text = filter_text
                .replace("permanents", "cards")
                .replace("permanent", "card");
        }
        if !filter_text.to_ascii_lowercase().contains("card") {
            filter_text.push_str(" card");
        }
        with_indefinite_article(&filter_text)
    } else {
        "a card".to_string()
    }
}

/// "Choose a <kind> card name. <Player> reveals their hand and discards all
/// cards with that name." (Cabal Therapy).
pub(super) fn describe_choose_name_reveal_hand_discard_named_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [choose_name_effect, look_effect, discard_effect] = filtered else {
        return None;
    };
    let choose_name = choose_name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;

    if !look.reveal
        || discard.random
        || discard.any_number
        || choose_name.chooser != PlayerFilter::You
    {
        return None;
    }
    let card_filter = discard.card_filter.as_ref()?;
    if card_filter.name.as_deref() != Some("{chosen name}") {
        return None;
    }
    // The revealed hand and the discard must belong to the same player.
    let look_player = choose_spec_player_filter(&look.target)?;
    if look_player != discard.player {
        return None;
    }

    let discard_count = describe_discard_count(&discard.count, Some(card_filter));
    let player = describe_player_filter(&discard.player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let discard_verb = player_verb(&player, "discard", "discards");
    let hand = if player == "you" {
        "your hand"
    } else {
        "their hand"
    };
    Some(format!(
        "Choose {} name. {} {reveal_verb} {hand} and {discard_verb} {discard_count}.",
        describe_choose_card_name_selection(choose_name),
        capitalize_first(&player),
    ))
}

/// "Reveal any number of <kind> cards in your hand" — the parser models this
/// as choosing any number of matching cards in hand, then revealing the
/// chosen cards (Scent of Cinder and friends).
pub(super) fn describe_choose_hand_then_reveal_chosen_pair(
    choose_effect: &Effect,
    reveal_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = unwrap_basic_tag_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != choose.tag
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.count.dynamic_x
        || choose.count.random
    {
        return None;
    }

    let mut display_filter = choose.filter.clone();
    display_filter.zone = None;
    display_filter.owner = None;
    let mut selection = strip_indefinite_article(&display_filter.description()).to_string();
    if choose.filter.card_types.is_empty() {
        // The cards live in hand: the filter's default battlefield noun
        // ("permanent") reads wrong here.
        selection = selection
            .replace("permanents", "cards")
            .replace("permanent", "card");
    }
    Some(format!(
        "Reveal any number of {} in your hand",
        pluralize_hand_card_selection(&selection)
    ))
}

pub(super) fn pluralize_hand_card_selection(selection: &str) -> String {
    let plural = pluralize_noun_phrase(selection);
    if plural.contains("card") {
        return plural;
    }
    for (plural_type, card_type) in [
        ("creatures", "creature"),
        ("artifacts", "artifact"),
        ("enchantments", "enchantment"),
        ("lands", "land"),
        ("planeswalkers", "planeswalker"),
        ("battles", "battle"),
        ("instants", "instant"),
        ("sorceries", "sorcery"),
        ("permanents", "permanent"),
    ] {
        if plural == plural_type {
            return format!("{card_type} cards");
        }
        if let Some(rest) = plural.strip_prefix(&format!("{plural_type} ")) {
            return format!("{card_type} cards {rest}");
        }
    }
    format!("{plural} cards")
}

pub(super) fn search_split_effect_moves_chosen_to_hand(effect: &Effect, chosen_tag: &str) -> bool {
    if let Some(hand_move) = downcast_search_split_move_to_zone(effect) {
        return search_split_move_to_zone_uses_tag(hand_move, chosen_tag, Zone::Hand);
    }
    unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some_and(|return_to_hand| {
            matches!(
                return_to_hand.spec.base(),
                ChooseSpec::Tagged(found) if found.as_str() == chosen_tag
            )
        })
}

pub(super) fn search_split_effect_moves_unselected_to_zone(
    effect: &Effect,
    source_tag: &str,
    chosen_tag: &str,
    zone: Zone,
) -> bool {
    for_each_tagged_for_compaction(effect).is_some_and(|(_, for_each)| {
        for_each_moves_unselected_to_zone(for_each, source_tag, chosen_tag, zone)
    })
}

pub(super) fn describe_search_two_split_hand_graveyard_sequence(
    effects: &[&Effect],
) -> Option<String> {
    let [
        search_effect,
        choose_effect,
        hand_effect,
        graveyard_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !search.is_search
        || choose.is_search
        || search.count.min != 2
        || search.count.max != Some(2)
        || search.count_value.is_some()
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count_value.is_some()
        || search.chooser != choose.chooser
        || shuffle.player != search.chooser
        || choose_search_zones(search)? != vec![Zone::Library]
        || !choose_search_zones(choose)?.contains(&Zone::Library)
        || !search_split_filter_is_tagged_as(&choose.filter, search.tag.as_str())
        || !search_split_effect_moves_chosen_to_hand(hand_effect, choose.tag.as_str())
        || !search_split_effect_moves_unselected_to_zone(
            graveyard_effect,
            search.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    if search.chooser == PlayerFilter::You {
        return Some(
            "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle"
                .to_string(),
        );
    }

    let player = describe_player_filter(&search.chooser);
    let capitalized = capitalize_first(&player);
    let possessive = describe_possessive_player_filter(&search.chooser);
    let shuffle_verb = player_verb(&player, "shuffle", "shuffles");
    Some(format!(
        "{capitalized} searches {possessive} library for two cards. Put one into {possessive} hand and the other into {possessive} graveyard. Then {player} {shuffle_verb}"
    ))
}

pub(super) fn normalize_search_descriptor_for_origin(
    descriptor: &str,
    searched_library: bool,
) -> String {
    let mut descriptor = descriptor.trim().to_string();
    if searched_library {
        for phrase in [
            " in your library",
            " in target opponent's library",
            " in target player's library",
            " in that player's library",
            " in their library",
            " in library",
            " in the library",
        ] {
            descriptor = descriptor.replace(phrase, "");
        }
    }
    descriptor = descriptor.replace("permanent you own named ", "card you own named ");
    descriptor = descriptor.replace("permanent named ", "card named ");
    descriptor = descriptor.replace("card you own named ", "card named ");
    descriptor
}

pub(super) fn describe_search_selection_from_filter_text(
    choose: &crate::effects::ChooseObjectsEffect,
    filter_text: &str,
) -> String {
    let filter_text = filter_text.trim();
    let where_clause = describe_runtime_choice_where_clause(choose).unwrap_or_default();
    let filter_is_generic_card = filter_text.eq_ignore_ascii_case("card");
    let simple_land_subtype = (choose.filter.card_types.as_slice() == [CardType::Land]
        && choose.filter.subtypes.len() == 1)
        .then(|| {
            let subtype = choose.filter.subtypes[0];
            let mut remainder = choose.filter.clone();
            remainder.zone = None;
            remainder.owner = None;
            remainder.card_types.clear();
            remainder.subtypes.clear();
            (remainder == ObjectFilter::default()).then_some(subtype)
        })
        .flatten();

    if choose.count.max == Some(1) {
        if let Some(subtype) = simple_land_subtype {
            return format!("a {subtype} card");
        }
        return if filter_is_generic_card {
            "a card".to_string()
        } else {
            with_indefinite_article(filter_text)
        };
    }

    if let Some(runtime_count) = describe_runtime_choice_count(choose) {
        if let Some(subtype) = simple_land_subtype {
            return format!("{runtime_count} {subtype} cards{where_clause}");
        }
        return if filter_is_generic_card {
            format!("{runtime_count} cards{where_clause}")
        } else {
            format!("{runtime_count} {filter_text}{where_clause}")
        };
    }

    let count_text = describe_choice_count(&choose.count);
    if filter_is_generic_card {
        if count_text == "all" {
            "all cards".to_string()
        } else if count_text == "any number of" {
            "any number of cards".to_string()
        } else {
            format!("{count_text} cards")
        }
    } else {
        format!("{count_text} {filter_text}")
    }
}

pub(super) fn describe_search_selection_with_cards_preserving_where(selection: &str) -> String {
    if let Some((head, tail)) = selection.split_once(", where X is ") {
        return format!(
            "{}, where X is {}",
            describe_search_selection_with_cards(head),
            tail
        );
    }
    describe_search_selection_with_cards(selection)
}

pub(super) fn for_each_subject_reference_phrase(subject: &str) -> &'static str {
    let lower = subject.to_ascii_lowercase();
    if lower.contains("creature") {
        "that creature"
    } else if lower.contains("permanent") {
        "that permanent"
    } else if lower.contains("artifact") {
        "that artifact"
    } else if lower.contains("enchantment") {
        "that enchantment"
    } else if lower.contains("land") {
        "that land"
    } else if lower.contains("spell") {
        "that spell"
    } else if lower.contains("card") {
        "that card"
    } else {
        "that object"
    }
}

pub(super) fn describe_stack_object_copy_target(target: &ChooseSpec) -> String {
    match target {
        ChooseSpec::Source => "this spell".to_string(),
        ChooseSpec::Tagged(tag) if matches!(tag.as_str(), "triggering" | "__it__" | "it") => {
            "that spell".to_string()
        }
        _ => {
            let described = describe_choose_spec(target);
            if described == "it" {
                "that spell".to_string()
            } else {
                described
            }
        }
    }
}

pub(super) fn describe_counter_all_stack_abilities(target: &ChooseSpec) -> Option<&'static str> {
    let ChooseSpec::All(filter) = target else {
        return None;
    };
    if filter.zone != Some(Zone::Stack)
        || filter.controller != Some(PlayerFilter::Opponent)
        || filter.stack_kind != Some(StackObjectKind::Ability)
    {
        return None;
    }

    let mut base = filter.clone();
    base.zone = None;
    base.controller = None;
    base.stack_kind = None;
    (base == ObjectFilter::default()).then_some("all abilities your opponents control")
}

pub(super) fn copy_target_player_candidate_text(filter: &PlayerFilter, plural: bool) -> String {
    match (filter, plural) {
        (PlayerFilter::Any, false) => "player".to_string(),
        (PlayerFilter::Any, true) => "players".to_string(),
        (PlayerFilter::Opponent, false) => "opponent".to_string(),
        (PlayerFilter::Opponent, true) => "opponents".to_string(),
        (PlayerFilter::You, _) => "you".to_string(),
        (_, false) => strip_leading_article(&describe_player_filter(filter)).to_string(),
        (_, true) => pluralize_noun_phrase(&describe_player_filter(filter)),
    }
}

pub(super) fn describe_copy_target_candidates(
    object_filter: Option<&ObjectFilter>,
    player_filter: Option<&PlayerFilter>,
    plural: bool,
) -> String {
    let object_text = object_filter.map(|filter| {
        let description = strip_leading_article(&filter.description()).to_string();
        if plural {
            pluralize_noun_phrase(&description)
        } else {
            description
        }
    });
    let player_text = player_filter.map(|filter| copy_target_player_candidate_text(filter, plural));

    match (object_text, player_text, plural) {
        (Some(object), Some(player), false) => format!("{object} or {player}"),
        (Some(object), Some(player), true) => format!("{object} and {player}"),
        (Some(object), None, _) => object,
        (None, Some(player), _) => player,
        (None, None, false) => "target".to_string(),
        (None, None, true) => "targets".to_string(),
    }
}

pub(super) fn describe_copy_spell_for_each_target(
    effect: &crate::effects::CopySpellForEachTargetEffect,
) -> String {
    let stack_object = describe_stack_object_copy_target(&effect.target);
    let candidate = describe_copy_target_candidates(
        effect.object_filter.as_ref(),
        effect.player_filter.as_ref(),
        false,
    );
    let candidate = if effect.exclude_current_targets {
        format!("other {candidate}")
    } else {
        candidate
    };
    let plural_candidate = describe_copy_target_candidates(
        effect.object_filter.as_ref(),
        effect.player_filter.as_ref(),
        true,
    );

    let mut text = format!(
        "Copy {stack_object} for each {candidate} {stack_object} could target. Each copy targets a different one of those {plural_candidate}"
    );
    if effect
        .removed_supertypes
        .contains(&crate::types::Supertype::Legendary)
    {
        text.push_str(". The copies aren't legendary");
    }
    text
}

pub(super) fn copy_spell_from_effect(effect: &Effect) -> Option<&crate::effects::CopySpellEffect> {
    if let Some(copy_spell) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        return Some(copy_spell);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return copy_spell_from_effect(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return copy_spell_from_effect(&tagged.effect);
    }
    None
}

pub(super) fn describe_draw_count_for_each_phrase(count: &Value) -> Option<String> {
    match count {
        Value::SurfaceHinted { value, hints } => {
            if hints.contains(&ValueSurfaceHint::CardsDiscardedThisWay) {
                return Some("a card for each card discarded this way".to_string());
            }
            if hints.contains(&ValueSurfaceHint::CardsExiledThisWay) {
                return Some("a card for each card exiled this way".to_string());
            }
            if hints.contains(&ValueSurfaceHint::PermanentsSacrificedThisWay) {
                return Some("a card for each permanent sacrificed this way".to_string());
            }
            describe_draw_count_for_each_phrase(value)
        }
        Value::Count(filter) => Some(format!(
            "a card for each {}",
            describe_for_each_filter(filter)
        )),
        Value::CreaturesDiedThisTurnControlledBy(controller) => {
            let suffix = match controller {
                PlayerFilter::You => "under your control this turn".to_string(),
                PlayerFilter::Opponent => "under an opponent's control this turn".to_string(),
                PlayerFilter::Any => "this turn".to_string(),
                other => format!(
                    "under {} control this turn",
                    describe_possessive_player_filter(other)
                ),
            };
            Some(format!("a card for each creature that died {suffix}"))
        }
        Value::SpellsCastThisTurn(spell_caster) => Some(format!(
            "a card for each {}",
            describe_spells_cast_this_turn_each(spell_caster)
        )),
        Value::KickCount => Some("a card for each time this spell was kicked".to_string()),
        Value::SpellsCastThisTurnMatching {
            player: spell_caster,
            filter,
            exclude_source,
        } => {
            let base = describe_for_each_filter(filter);
            let prefix = if *exclude_source && !base.starts_with("other ") {
                "other "
            } else {
                ""
            };
            let tail = match spell_caster {
                PlayerFilter::You => "you've cast this turn".to_string(),
                PlayerFilter::Opponent => "an opponent has cast this turn".to_string(),
                PlayerFilter::Any => "cast this turn".to_string(),
                other => format!(
                    "cast this turn by {}",
                    strip_leading_article(&describe_player_filter(other))
                ),
            };
            Some(format!("a card for each {prefix}{base} {tail}"))
        }
        Value::PlayerCounters(counter_player, counter_type) => Some(format!(
            "a card for each {} counter {}",
            describe_counter_type(*counter_type),
            describe_player_counter_holder(counter_player)
        )),
        Value::CountersOnSource(counter_type) => Some(format!(
            "a card for each {} counter on this permanent",
            describe_counter_type(*counter_type)
        )),
        Value::CountersOn(spec, Some(counter_type)) => Some(format!(
            "a card for each {} counter on {}",
            describe_counter_type(*counter_type),
            describe_choose_spec(spec)
        )),
        Value::CountersOn(spec, None) => Some(format!(
            "a card for each counter on {}",
            describe_choose_spec(spec)
        )),
        Value::BasicLandTypesAmong(filter) => Some(format!(
            "a card for each {}",
            describe_basic_land_types_among(filter)
        )),
        Value::CreatureTypesAmong(filter) => Some(format!(
            "a card for each creature type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::CardTypesAmong(filter) => Some(format!(
            "a card for each card type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::ColorsAmong(filter) => {
            Some(format!("a card for each {}", describe_colors_among(filter)))
        }
        _ => None,
    }
}

pub(super) fn describe_for_players_vote_received_repeat(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let [effect] = for_players.effects.as_slice() else {
        return None;
    };
    let repeat = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
    if repeat.count != Value::PlayerVoteCount(PlayerFilter::IteratedPlayer) {
        return None;
    }

    let player = match for_players.filter {
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        _ => {
            strip_leading_article(&describe_for_each_player_filter(&for_players.filter)).to_string()
        }
    };
    let repeated = describe_damage_and_controlled_damage_pair(&repeat.effects)
        .unwrap_or_else(|| describe_effect_list(&repeat.effects));
    let repeated = lowercase_first(repeated.trim().trim_end_matches('.'));
    Some(format!("For each vote {player} received, {repeated}"))
}

pub(super) fn describe_damage_and_controlled_damage_pair(effects: &[Effect]) -> Option<String> {
    fn source_damage(
        effect: &Effect,
    ) -> Option<(Option<&ChooseSpec>, &crate::effects::DealDamageEffect)> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return source_damage(&tagged.effect);
        }
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && let Some(damage) = with_source
                .effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
        {
            return Some((Some(&with_source.source), damage));
        }
        effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .map(|damage| (None, damage))
    }

    fn source_for_each(
        effect: &Effect,
    ) -> Option<(Option<&ChooseSpec>, &crate::effects::ForEachObject)> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return source_for_each(&tagged.effect);
        }
        if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            && let Some(for_each) = with_source
                .effect
                .downcast_ref::<crate::effects::ForEachObject>()
        {
            return Some((Some(&with_source.source), for_each));
        }
        effect
            .downcast_ref::<crate::effects::ForEachObject>()
            .map(|for_each| (None, for_each))
    }

    let [first, second] = effects else {
        return None;
    };
    let (source, player_damage) = source_damage(first)?;
    if !matches!(
        player_damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }
    let (for_each_source, for_each) = source_for_each(second)?;
    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let (inner_source, object_damage) = source_damage(inner)?;
    if object_damage.amount != player_damage.amount
        || !matches!(object_damage.target, ChooseSpec::Iterated)
    {
        return None;
    }
    let mut objects = describe_each_controlled_by_iterated(&for_each.filter)?;
    objects = objects.replace(" they control", " that player controls");
    let amount = describe_damage_amount_clause(&player_damage.amount).0;
    if let Some(subject) = source
        .or(for_each_source)
        .or(inner_source)
        .map(describe_choose_spec)
    {
        return Some(format!(
            "{subject} deals {amount} to that player and {objects}"
        ));
    }
    Some(format!("Deal {amount} to that player and {objects}"))
}

pub(super) fn tagged_copy_spell_from_effect(
    effect: &Effect,
) -> Option<(&crate::TagKey, &crate::effects::CopySpellEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let copy_spell = copy_spell_from_effect(&tagged.effect)?;
    Some((&tagged.tag, copy_spell))
}

pub(super) fn retarget_fixed_spec_uses_chosen_tag(
    spec: &ChooseSpec,
    chosen_tag: &crate::TagKey,
) -> bool {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == *chosen_tag
            })
        }
        ChooseSpec::Tagged(tag) => tag == chosen_tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            retarget_fixed_spec_uses_chosen_tag(inner, chosen_tag)
        }
        _ => false,
    }
}

pub(super) fn copy_retarget_reference_noun(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if filter.card_types.contains(&CardType::Land) {
        "land"
    } else if filter.zone == Some(Zone::Battlefield) {
        "permanent"
    } else {
        "object"
    }
}

pub(super) fn describe_choose_copy_spell_and_retarget_copy_to_chosen(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, copy_effect, retarget_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search || !choose.count.is_single() {
        return None;
    }

    let (copied_tag, copy_spell) = tagged_copy_spell_from_effect(copy_effect)?;
    if copy_spell.count != Value::Fixed(1)
        || !copy_spell.removed_supertypes.is_empty()
        || copy_spell.copier != choose.chooser
    {
        return None;
    }
    let copied_spell_text = describe_stack_object_copy_target(&copy_spell.target);
    if copied_spell_text != "that spell" && copied_spell_text != "this spell" {
        return None;
    }

    let retarget = retarget_effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if retarget.chooser != choose.chooser
        || retarget.require_change
        || retarget.new_target_restriction.is_some()
        || !matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == copied_tag)
    {
        return None;
    }
    let crate::effects::RetargetMode::OneToFixed(fixed_spec) = &retarget.mode else {
        return None;
    };
    if !retarget_fixed_spec_uses_chosen_tag(fixed_spec, &choose.tag) {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let noun = copy_retarget_reference_noun(&choose.filter);
    let plural_noun = pluralize_noun_phrase(noun);
    let copy_verb = if copy_spell.copier == PlayerFilter::You {
        "Copy".to_string()
    } else {
        let copier = describe_player_filter(&copy_spell.copier);
        format!(
            "{} {}",
            capitalize_first(&copier),
            player_verb(&copier, "copy", "copies")
        )
    };
    Some(format!(
        "{chooser} {choose_verb} one of those {plural_noun}. {copy_verb} {copied_spell_text}. The copy targets the chosen {noun}"
    ))
}

pub(super) fn describe_phase_in_out_pair(first: &Effect, second: &Effect) -> Option<String> {
    let phase_in = first.downcast_ref::<crate::effects::PhaseInEffect>()?;
    let phase_out = second.downcast_ref::<crate::effects::PhaseOutEffect>()?;
    let ChooseSpec::All(phase_in_filter) = phase_in.spec.base() else {
        return None;
    };
    let ChooseSpec::All(phase_out_filter) = phase_out.spec.base() else {
        return None;
    };
    let phase_in_is_all_creatures = phase_in_filter.card_types == vec![CardType::Creature]
        && phase_in_filter.subtypes.is_empty()
        && phase_in_filter.static_abilities.is_empty();
    let phase_out_is_creatures_with_phasing = phase_out_filter.card_types
        == vec![CardType::Creature]
        && phase_out_filter.subtypes.is_empty()
        && phase_out_filter
            .static_abilities
            .contains(&crate::static_abilities::StaticAbilityId::Phasing);
    if phase_in_is_all_creatures && phase_out_is_creatures_with_phasing {
        Some(
            "Simultaneously, all phased-out creatures phase in and all creatures with phasing phase out"
                .to_string(),
        )
    } else {
        None
    }
}

pub(super) fn describe_for_players_target_return_unless_draw(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Opponent || for_players.effects.len() != 2 {
        return None;
    }
    let targeted = for_players.effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = targeted
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let unless_action =
        for_players.effects[1].downcast_ref::<crate::effects::UnlessActionEffect>()?;
    if unless_action.effects.len() != 1 || unless_action.alternative.len() != 1 {
        return None;
    }
    let returned = unless_action.effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let return_to_hand = returned
        .effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if !matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(tag) if tag == &targeted.tag) {
        return None;
    }
    if !matches!(
        &unless_action.player,
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag)) if tag == &targeted.tag
    ) {
        return None;
    }
    let draw = unless_action.alternative[0].downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }
    let target_text = describe_choose_spec(&target_only.target);
    let returned_text = for_each_subject_reference_phrase(&target_text);
    Some(format!(
        "For each opponent, choose {target_text}, then return {returned_text} to its owner's hand unless its controller has you draw a card"
    ))
}

pub(super) fn choose_spec_contains_hand_advantage_player_filter(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_contains_hand_advantage_player_filter(inner)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            player_filter_contains_hand_advantage_filter(filter)
        }
        _ => false,
    }
}

pub(super) fn choose_spec_is_player_choice(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_is_player_choice(inner)
        }
        ChooseSpec::Player(_) | ChooseSpec::PlayerOrPlaneswalker(_) => true,
        _ => false,
    }
}

pub(super) fn player_filter_references_target_player(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::Target(_) => true,
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_references_target_player(base)
                || player_filter_references_target_player(excluded)
        }
        _ => false,
    }
}

pub(super) fn object_filter_references_target_player(filter: &ObjectFilter) -> bool {
    filter
        .controller
        .as_ref()
        .is_some_and(player_filter_references_target_player)
        || filter
            .owner
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .cast_by
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .targets_player
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .targets_only_player
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .attached_to_player
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .dealt_damage_to_player_this_turn
            .as_ref()
            .is_some_and(player_filter_references_target_player)
        || filter
            .any_of
            .iter()
            .any(object_filter_references_target_player)
}

pub(super) fn choose_spec_references_target_player(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_target_player(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_target_player(filter)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            player_filter_references_target_player(filter)
        }
        _ => false,
    }
}

pub(super) fn value_references_target_player(value: &Value) -> bool {
    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_target_player(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_target_player(left) || value_references_target_player(right)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => object_filter_references_target_player(filter),
        Value::StaticAbilitiesAmong { filter, .. } => {
            object_filter_references_target_player(filter)
        }
        Value::CreaturesDiedThisTurnControlledBy(player)
        | Value::CountPlayers(player)
        | Value::PartySize(player)
        | Value::LifeTotal(player)
        | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player)
        | Value::UnspentMana(player)
        | Value::Speed(player)
        | Value::StartingLifeTotal(player)
        | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player)
        | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player)
        | Value::CardsInHand(player)
        | Value::CardsInLibrary(player)
        | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player)
        | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player)
        | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player)
        | Value::MaxDiceRolledThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player)
        | Value::MaxCardsInHand(player)
        | Value::CardsInGraveyard(player)
        | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player)
        | Value::CommanderCastCount(player)
        | Value::CardTypesInGraveyard(player) => player_filter_references_target_player(player),
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, .. }
        | Value::Devotion { player, .. } => player_filter_references_target_player(player),
        Value::SpellsCastThisTurnMatching { player, filter, .. } => {
            player_filter_references_target_player(player)
                || object_filter_references_target_player(filter)
        }
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) => {
            choose_spec_references_target_player(spec)
        }
        _ => false,
    }
}

pub(super) fn effect_references_target_player(effect: &Effect) -> bool {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(energy) = effect.downcast_ref::<crate::effects::EnergyCountersEffect>() {
        return value_references_target_player(&energy.count);
    }
    if let Some(ticket) = effect.downcast_ref::<crate::effects::TicketCountersEffect>() {
        return value_references_target_player(&ticket.count);
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        return value_references_target_player(&draw.count);
    }
    if let Some(gain_life) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        return value_references_target_player(&gain_life.amount);
    }
    if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        return value_references_target_player(&lose_life.amount);
    }
    false
}

pub(super) fn controlled_filter_suffix(player: &PlayerFilter, verb: &str) -> String {
    match player {
        PlayerFilter::You => format!("you {verb}"),
        PlayerFilter::NotYou => format!("you don't {verb}"),
        PlayerFilter::Opponent => format!("an opponent {verb}s"),
        PlayerFilter::Any => format!("a player {verb}s"),
        PlayerFilter::Defending => format!("defending player {verb}s"),
        PlayerFilter::Attacking => format!("attacking player {verb}s"),
        PlayerFilter::DamagedPlayer
        | PlayerFilter::Specific(_)
        | PlayerFilter::Target(_)
        | PlayerFilter::IteratedPlayer
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::ChosenPlayer => format!("that player {verb}s"),
        other => format!("{} {verb}s", describe_player_filter(other)),
    }
}

pub(super) fn insert_filter_suffix_before_qualifier(subject: &str, suffix: &str) -> String {
    for marker in [" without ", " with ", " named ", " not named "] {
        if let Some((head, tail)) = subject.split_once(marker) {
            return format!("{head} {suffix}{marker}{tail}");
        }
    }
    format!("{subject} {suffix}")
}

pub(super) fn describe_plural_block_restriction_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.as_slice() != [CardType::Creature] || filter.source {
        return None;
    }
    let mut bare = filter.clone();
    let controller = bare.controller.take();
    let owner = bare.owner.take();
    let mut subject = pluralize_noun_phrase(strip_indefinite_article(&bare.description()));
    if let Some(controller) = controller.as_ref() {
        let suffix = controlled_filter_suffix(controller, "control");
        subject = insert_filter_suffix_before_qualifier(&subject, &suffix);
    } else if let Some(owner) = owner.as_ref() {
        let suffix = controlled_filter_suffix(owner, "own");
        subject = insert_filter_suffix_before_qualifier(&subject, &suffix);
    }
    Some(capitalize_first(&subject))
}

pub(super) fn describe_destroy_then_temporary_cant_attack_block(
    destroy_effect: &Effect,
    cant_effect: &Effect,
) -> Option<String> {
    let destroy = destroy_effect.downcast_ref::<crate::effects::DestroyEffect>()?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if cant.duration != Until::EndOfTurn {
        return None;
    }
    let ChooseSpec::Object(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    let destroy_controller = destroy_filter.controller.as_ref()?;
    let (restriction_filter, restriction_text) = match &cant.restriction {
        crate::effect::Restriction::Attack(filter) => (filter, "can't attack this turn"),
        crate::effect::Restriction::Block(filter) => (filter, "can't block this turn"),
        crate::effect::Restriction::AttackOrBlock(filter) => {
            (filter, "can't attack or block this turn")
        }
        _ => return None,
    };
    if restriction_filter.controller.as_ref() != Some(destroy_controller) {
        return None;
    }
    let mut subject = describe_plural_block_restriction_subject(restriction_filter)?;
    match destroy_controller {
        PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::Target(_) => {
            subject = subject
                .replace("Defending player controls", "that player controls")
                .replace("defending player controls", "that player controls")
                .replace("Attacking player controls", "that player controls")
                .replace("attacking player controls", "that player controls");
        }
        _ => {}
    }
    Some(format!(
        "{}, and {} {restriction_text}",
        describe_effect(destroy_effect).trim_end_matches('.'),
        lowercase_first(&subject)
    ))
}

pub(super) fn player_filter_contains_hand_advantage_filter(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. } => true,
        PlayerFilter::Target(inner) => player_filter_contains_hand_advantage_filter(inner),
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_contains_hand_advantage_filter(base)
                || player_filter_contains_hand_advantage_filter(excluded)
        }
        _ => false,
    }
}

pub(super) fn describe_for_each_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let base_description = describe_player_filter(base);
            let base_text = strip_leading_article(&base_description);
            if *count == 1 {
                format!("{base_text} who has more cards in hand than you")
            } else {
                let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
                format!("{base_text} who has at least {count_text} more cards in hand than you")
            }
        }
        _ => describe_player_filter(filter),
    }
}

pub(super) fn describe_next_end_step_cleanup_timing(player: &PlayerFilter) -> String {
    match player {
        PlayerFilter::Any => "the next end step".to_string(),
        PlayerFilter::You => "your next end step".to_string(),
        other => format!("{} next end step", describe_possessive_player_filter(other)),
    }
}

pub(super) fn describe_choose_each_basic_land_type_then_destroy(
    effects: &[&Effect],
) -> Option<String> {
    let [plains, island, swamp, mountain, forest, destroy] = effects else {
        return None;
    };
    let expected_subtypes = [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ];
    let mut tag: Option<&str> = None;
    for (effect, subtype) in [plains, island, swamp, mountain, forest]
        .into_iter()
        .zip(expected_subtypes)
    {
        let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        if !choose.count.is_single()
            || choose.is_search
            || choose.top_only
            || choose_primary_zone(choose) != Some(Zone::Battlefield)
            || choose.filter.card_types != vec![CardType::Land]
            || choose.filter.subtypes != vec![subtype]
        {
            return None;
        }
        if let Some(existing_tag) = tag {
            if existing_tag != choose.tag.as_str() {
                return None;
            }
        } else {
            tag = Some(choose.tag.as_str());
        }
    }

    let tag = tag?;
    let destroy = destroy.downcast_ref::<crate::effects::DestroyEffect>()?;
    let destroys_tagged_lands = match &destroy.spec {
        ChooseSpec::Tagged(found) => found.as_str() == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.card_types == vec![CardType::Land]
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == tag
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                })
        }
        _ => false,
    };
    destroys_tagged_lands
        .then_some("Choose a land of each basic land type, then destroy those lands".to_string())
}

pub(super) fn describe_distributed_damage_target(target: &ChooseSpec) -> String {
    match target {
        ChooseSpec::WithCount(inner, count)
            if matches!(inner.as_ref(), ChooseSpec::AnyTarget)
                && count.min == 1
                && count.max == Some(3) =>
        {
            "one, two, or three targets".to_string()
        }
        ChooseSpec::WithCount(inner, count) if !inner.is_target() => {
            describe_choose_spec(&ChooseSpec::target(inner.as_ref().clone()).with_count(*count))
        }
        _ => describe_choose_spec(target),
    }
}

pub(super) fn describe_distributed_damage_amount(value: &Value) -> String {
    if let Value::ManaValueOf(spec) = value
        && matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str().starts_with("unattach_cost_"))
    {
        return "that Equipment's mana value".to_string();
    }
    describe_value(value)
}

pub(super) fn describe_for_each_tagged_shuffle_into_owner_library(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if for_each.effects.len() != 2 {
        return None;
    }
    let move_to_zone = for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
        || !matches!(move_to_zone.target, ChooseSpec::Iterated)
    {
        return None;
    }
    let shuffle = for_each.effects[1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !matches!(
        &shuffle.player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == &for_each.tag
    ) {
        return None;
    }
    Some("Its owner shuffles it into their library".to_string())
}

pub(super) fn describe_source_and_blocked_creatures_top_library_shuffle(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [move_effect, shuffle_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let [source_filter, blocked_filter] = for_each.filter.any_of.as_slice() else {
        return None;
    };
    let mut expected_blocked_filter = ObjectFilter::creature();
    expected_blocked_filter.blocked_by_source = true;
    if source_filter != &ObjectFilter::source() || blocked_filter != &expected_blocked_filter {
        return None;
    }
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || !move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !matches!(
        &shuffle.player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag.as_str() == "__it__"
    ) {
        return None;
    }
    Some(
        "Put this creature and each creature it's blocking on top of their owners' libraries, then those players shuffle"
            .to_string(),
    )
}

pub(super) fn describe_source_owner_shuffle_then_reveal_named_to_battlefield(
    effects: &[&Effect],
) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_effect(&tagged.effect);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return unwrap_effect(&tag_all.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_effect(&with_id.effect);
        }
        effect
    }

    let [
        shuffle_effect,
        consult_effect,
        move_effect,
        remainder_effect,
    ] = effects
    else {
        return None;
    };
    let shuffle = unwrap_effect(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    if !matches!(shuffle.target, ChooseSpec::Source)
        || !matches!(
            shuffle.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
        )
    {
        return None;
    }

    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !matches!(
            consult.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
        )
    {
        return None;
    }
    match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {}
        _ => return None,
    }
    let card_name = consult.filter.name.as_ref()?;

    let move_to_zone =
        unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
    {
        return None;
    }

    let remainder = remainder_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag != consult.all_tag {
        return None;
    }
    let [conditional_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let condition_ok = match &conditional.condition {
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__"
                && filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag == consult.match_tag) =>
        {
            true
        }
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag == &consult.match_tag
                && *filter
                    == ObjectFilter::default()
                        .same_stable_id_as_tagged(crate::tag::TagKey::from("__it__")) =>
        {
            true
        }
        _ => false,
    };
    if !condition_ok || !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return None;
    }
    let graveyard = unwrap_effect(&conditional.if_false[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if graveyard.zone != Zone::Graveyard
        || graveyard.to_top
        || !matches!(graveyard.target, ChooseSpec::Iterated)
    {
        return None;
    }

    Some(format!(
        "This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named {card_name} is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    ))
}

pub(super) fn filter_has_same_name_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    })
}

pub(super) fn describe_choose_name_exile_top_consult_hand_rest_exile(
    effects: &[&Effect],
) -> Option<String> {
    let [
        choose_name_effect,
        exile_top_effect,
        consult_effect,
        move_effect,
        remainder_effect,
    ] = effects
    else {
        return None;
    };

    let choose_name = choose_name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose_name.chooser != PlayerFilter::You {
        return None;
    }

    let exile_top = exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile_top.player != PlayerFilter::You {
        return None;
    }

    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !filter_has_same_name_tag(&consult.filter, &choose_name.tag)
    {
        return None;
    }
    match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {}
        _ => return None,
    }

    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand
        || move_to_zone.to_top
        || !matches!(
            &move_to_zone.target,
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }

    let remainder = remainder_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag != consult.all_tag {
        return None;
    }
    let [conditional_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let condition_ok = matches!(
        &conditional.condition,
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag == &consult.match_tag
                && *filter
                    == ObjectFilter::default()
                        .same_stable_id_as_tagged(crate::tag::TagKey::from("__it__"))
    ) || matches!(
        &conditional.condition,
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__"
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag == consult.match_tag
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
    );
    if !condition_ok || !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return None;
    }

    let exile_remainder = unwrap_basic_tag_wrappers(&conditional.if_false[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_remainder.zone != Zone::Exile
        || !exile_remainder.to_top
        || !matches!(&exile_remainder.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let count_text = match exile_top.count.unhinted() {
        Value::Fixed(count) if *count >= 0 => {
            small_number_word(*count as u32).unwrap_or_else(|| count.to_string())
        }
        _ => describe_value(&exile_top.count),
    };
    let card_noun = match exile_top.count.unhinted() {
        Value::Fixed(1) => "card",
        _ => "cards",
    };

    Some(format!(
        "Choose a card name. Exile the top {count_text} {card_noun} of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way"
    ))
}

pub(crate) fn describe_chosen_name_consult_after_top_exile_effects(
    effects: &[Effect],
) -> Option<String> {
    let refs = effects.iter().collect::<Vec<_>>();
    describe_choose_name_exile_top_consult_hand_rest_exile(&refs)
}

pub(crate) fn describe_reveal_hand_choose_discard_then_random_effects(
    effects: &[Effect],
) -> Option<String> {
    let [
        look_effect,
        choose_effect,
        discard_chosen_effect,
        discard_random_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| describe_player_filter(owner) != describe_choose_spec(&look.target))
    {
        return None;
    }

    let discard_chosen = discard_chosen_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let revealer = describe_choose_spec(&look.target);
    if discard_chosen.count != Value::Fixed(1)
        || discard_chosen.random
        || discard_chosen.any_number
        || describe_player_filter(&discard_chosen.player) != revealer
        || !discard_chosen.card_filter.as_ref().is_some_and(|filter| {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == choose.tag
            })
        })
    {
        return None;
    }

    let discard_random = discard_random_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard_random.count != Value::Fixed(1)
        || !discard_random.random
        || discard_random.any_number
        || discard_random.card_filter.is_some()
        || describe_player_filter(&discard_random.player) != revealer
    {
        return None;
    }

    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let hand = if revealer == "you" {
        "your hand"
    } else {
        "their hand"
    };
    let mut selection = choose.filter.description();
    for suffix in [
        format!(" in {revealer}'s hand"),
        " in their hand".to_string(),
        " in your hand".to_string(),
        " in hand".to_string(),
    ] {
        if let Some(rest) = selection.strip_suffix(&suffix) {
            selection = rest.trim().to_string();
            break;
        }
    }
    let selection = with_indefinite_article(&selection);
    let discard_subject = if revealer == "you" {
        "You"
    } else {
        "That player"
    };
    let discard_verb = player_verb(&discard_subject.to_ascii_lowercase(), "discard", "discards");

    Some(format!(
        "{} {} {hand}. You choose {selection} from it. {discard_subject} {discard_verb} that card, then {discard_verb} a card at random",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

pub(crate) fn describe_choose_sacrifice_then_source_damage_effects(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, damage_effect] = effects else {
        return None;
    };

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    describe_choose_then_sacrifice(choose, sacrifice)?;

    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !matches!(damage.target, ChooseSpec::SourceController)
    {
        return None;
    }

    let mut sacrificed_filter = choose.filter.clone();
    sacrificed_filter.zone = None;
    if sacrificed_filter.controller == Some(PlayerFilter::You) {
        sacrificed_filter.controller = None;
    }
    let sacrificed =
        with_indefinite_article(strip_leading_article(&sacrificed_filter.description()));
    let damage_text = lowercase_first(&describe_effect(damage_effect));

    Some(format!("Sacrifice {sacrificed} and {damage_text}"))
}

pub(super) fn normalize_reflexive_sacrifice_setup(setup: String) -> String {
    if let Some(rest) = setup.strip_prefix("you sacrifice ") {
        format!("Sacrifice {rest}")
    } else {
        capitalize_first(&setup)
    }
}

pub(super) fn describe_reflexive_sacrifice_condition(
    predicate: &EffectPredicate,
) -> Option<String> {
    match predicate {
        EffectPredicate::Happened => Some("When you do".to_string()),
        EffectPredicate::HappenedNotReplaced => {
            Some("When you do and it isn't replaced".to_string())
        }
        _ => None,
    }
}

pub(super) fn describe_counted_reflexive_sacrifice_condition(
    predicate: &EffectPredicate,
    choose: &crate::effects::ChooseObjectsEffect,
    sacrifice: SacrificeView<'_>,
) -> Option<String> {
    if predicate == &EffectPredicate::Happened
        && choose.chooser == PlayerFilter::You
        && sacrifice.player == &PlayerFilter::You
        && (choose.count.dynamic_x || choose.count.max.map_or(true, |max| max > 1))
    {
        let sacrificed = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
        return Some(format!(
            "When you sacrifice one or more {sacrificed} this way"
        ));
    }

    describe_reflexive_sacrifice_condition(predicate)
}

pub(super) fn rewrite_sacrificed_reflexive_value_references(text: &str) -> String {
    text.replace(
        "where X is its toughness",
        "where X is the sacrificed creature's toughness",
    )
    .replace(
        "where X is its power",
        "where X is the sacrificed creature's power",
    )
    .replace(
        "where X is its mana value",
        "where X is the sacrificed creature's mana value",
    )
}

pub(super) fn describe_choose_sacrifice_then_reflexive_trigger_effects(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, reflexive_effect] = effects else {
        return None;
    };

    describe_choose_sacrifice_then_reflexive_trigger(
        choose_effect,
        sacrifice_effect,
        reflexive_effect,
    )
}

pub(super) fn describe_choose_sacrifice_then_reflexive_trigger_refs(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, reflexive_effect] = effects else {
        return None;
    };

    describe_choose_sacrifice_then_reflexive_trigger(
        choose_effect,
        sacrifice_effect,
        reflexive_effect,
    )
}

pub(super) fn describe_choose_sacrifice_then_reflexive_trigger(
    choose_effect: &Effect,
    sacrifice_effect: &Effect,
    reflexive_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let with_id = sacrifice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = sacrifice_view(&with_id.effect)?;
    let setup =
        normalize_reflexive_sacrifice_setup(describe_choose_then_sacrifice(choose, sacrifice)?);

    let reflexive = reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
    if reflexive.condition != with_id.id {
        return None;
    }
    let condition =
        describe_counted_reflexive_sacrifice_condition(&reflexive.predicate, choose, sacrifice)?;
    let triggered = lowercase_first(&describe_effect_list(&reflexive.effects));
    let triggered = rewrite_sacrificed_reflexive_value_references(&triggered);

    Some(format!("{setup}. {condition}, {triggered}"))
}

pub(super) fn describe_add_mana_then_conditional_consult_hand_bottom(
    effects: &[&Effect],
) -> Option<String> {
    let [mana_effect, conditional_effect] = effects else {
        return None;
    };
    if mana_effect
        .downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
        .is_none()
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 3 {
        return None;
    }

    let consult =
        conditional.if_true[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
    {
        return None;
    }

    let move_to_hand = conditional.if_true[1].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_hand.zone != Zone::Hand || move_to_hand.to_top {
        return None;
    }
    if !matches!(
        &move_to_hand.target,
        ChooseSpec::Tagged(tag) if tag == &consult.match_tag
    ) {
        return None;
    }

    let remainder = conditional.if_true[2]
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.player != PlayerFilter::You
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
    {
        return None;
    }

    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => format!(
            "{} {}",
            describe_value(count),
            pluralize_noun_phrase(&selection)
        ),
    };
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in an order chosen by you"
        }
    };
    let mana_text = cleanup_decompiled_text(&describe_effect(mana_effect))
        .trim_end_matches('.')
        .to_string();

    Some(format!(
        "{mana_text}. Then if {}, reveal cards from the top of your library until you reveal {stop_text}. Put that card into your hand and the rest on the bottom of your library{order_text}",
        describe_condition(&conditional.condition)
    ))
}

pub(super) fn describe_choose_then_put_counter_on_each(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, for_each_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = unwrap_basic_tag_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated)
        || put.target_count.is_some()
        || put.distributed
        || put.amount != Value::Fixed(1)
    {
        return None;
    }
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == choose.tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    }) {
        return None;
    }

    fn normalize_relative_clause_plural(selection: String) -> String {
        let mut normalized = selection;
        for (singular, plural) in [
            ("artifact", "artifacts"),
            ("battle", "battles"),
            ("card", "cards"),
            ("creature", "creatures"),
            ("enchantment", "enchantments"),
            ("land", "lands"),
            ("permanent", "permanents"),
            ("planeswalker", "planeswalkers"),
            ("spell", "spells"),
        ] {
            normalized = normalized.replace(
                &format!(" {singular} you don't controls"),
                &format!(" {plural} you don't control"),
            );
            normalized = normalized.replace(
                &format!(" {singular} you controls"),
                &format!(" {plural} you control"),
            );
        }
        normalized
    }

    let selection = normalize_relative_clause_plural(describe_choose_selection(choose));
    let counter = describe_put_counter_phrase(&put.amount, put.counter_type);
    let chooser = describe_player_filter(&choose.chooser);
    if choose.chooser == PlayerFilter::You {
        Some(format!(
            "Choose {selection} and put {counter} on each of them"
        ))
    } else {
        Some(format!(
            "{} {} {selection} and put {counter} on each of them",
            chooser,
            player_verb(&chooser, "choose", "chooses")
        ))
    }
}

pub(super) fn describe_tagged_effect_then_put_counter_on_each(
    effects: &[Effect],
) -> Option<String> {
    let [tagged_effect, for_each_effect] = effects else {
        return None;
    };
    let tagged = tagged_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .is_some()
    {
        // Pure choose/target effects carry their own surface (e.g. kicked target
        // clauses); let the structural renderers preserve it.
        return None;
    }
    let for_each = unwrap_basic_tag_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated) || put.target_count.is_some() || put.distributed
    {
        return None;
    }
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == tagged.tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    }) {
        return None;
    }

    Some(format!(
        "{}. Put {} on each of them",
        describe_effect(&tagged.effect),
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn describe_structural_multisentence_effect_list(effects: &[Effect]) -> Option<String> {
    if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        return describe_structural_multisentence_effect_list(rest);
    }
    if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
            .is_some()
    {
        return describe_structural_multisentence_effect_list(rest);
    }
    if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
            .is_some()
    {
        return describe_structural_multisentence_effect_list(rest);
    }

    if let Some(compact) = describe_leading_effect_then_pump_and_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_looked_card_split_destinations_structural(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_each_opponent_exile_top_then_cast_until_eot_any_color(effects) {
        return Some(compact);
    }

    let refs = effects.iter().collect::<Vec<_>>();
    if let Some(compact) = describe_exile_creatures_consult_that_many_battlefield_shuffle(&refs) {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_top_opponent_exiles_rest_hand_then_may_cast(effects) {
        return Some(compact);
    }
    if let Some(compact) =
        describe_destroy_land_then_controller_reveals_until_land_graveyard(effects)
    {
        return Some(compact);
    }
    if let Some(compact) =
        describe_each_player_mill_exile_milled_creatures_create_power_token(effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_exile_all_creatures_each_player_fractal_power_counters(effects)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_draw_reveal_discard_nonland(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_choose_color_target_and_shared_color_protection(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_target_and_shared_color_inline_ability_grant(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_look_reorder_then_may_shuffle(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_discard_then_draw_for_discarded(effects) {
        return Some(compact);
    }
    if let Some(compact) = describe_for_players_may_discard_then_draw_if_discarded(effects) {
        return Some(compact);
    }
    if let [choose_effect, look_effect, reveal_effect, distribute_effect] = effects
        && let Some(choose_name) =
            choose_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(reveal_tagged) =
            reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some((_, distribute)) = for_each_tagged_for_compaction(distribute_effect)
        && let Some(compact) = describe_choose_name_then_reveal_matching_hand_rest_graveyard(
            choose_name,
            look_at_top,
            reveal_tagged,
            distribute,
        )
    {
        return Some(compact);
    }
    if let [first, second, third, fourth] = effects
        && let Some(tagged_mill) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(mill) = tagged_mill
            .effect
            .downcast_ref::<crate::effects::MillEffect>()
        && let Some(choose) = second.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((Some(move_to_hand_with_id), move_to_hand)) =
            for_each_tagged_for_compaction(third)
        && let Some(if_effect) = fourth.downcast_ref::<crate::effects::IfEffect>()
        && let Some(compact) = describe_tagged_mill_then_put_milled_card_into_hand_with_fallback(
            tagged_mill,
            mill,
            choose,
            move_to_hand_with_id,
            move_to_hand,
            if_effect,
        )
    {
        return Some(compact);
    }
    if let [first, second, third] = effects
        && let Some(tagged_mill) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(mill) = tagged_mill
            .effect
            .downcast_ref::<crate::effects::MillEffect>()
        && let Some(choose) = second.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(third)
        && let Some(compact) = describe_tagged_mill_then_put_milled_card_into_hand(
            tagged_mill,
            mill,
            choose,
            move_to_hand,
        )
    {
        return Some(compact);
    }
    if let Some(compact) = describe_reveal_top_one_hand_gain_mana_value_rest_graveyard(effects) {
        return Some(compact);
    }

    fn early_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return Some(&tagged.tag);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return Some(&tag_all.tag);
        }
        None
    }

    fn early_create_token(effect: &Effect) -> Option<&crate::effects::CreateTokenEffect> {
        unwrap_tag_wrapped_effect(effect).downcast_ref()
    }

    fn early_set_base_pt(effect: &Effect) -> Option<&crate::effects::SetBasePowerToughnessEffect> {
        unwrap_tag_wrapped_effect(effect).downcast_ref()
    }

    fn early_clean_count_subject(filter: &ObjectFilter) -> String {
        let mut subject = describe_count_filter_value_subject(filter);
        for suffix in [
            " in exile",
            " in all graveyards",
            " in a graveyard",
            " in graveyard",
            " on the battlefield",
        ] {
            if let Some(stripped) = subject.strip_suffix(suffix) {
                subject = stripped.to_string();
                break;
            }
        }
        subject
    }

    fn early_prior_count_subject(effect: &Effect) -> Option<(String, &'static str)> {
        let effect = unwrap_basic_tag_wrappers(effect);
        if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
            if let ChooseSpec::All(filter) | ChooseSpec::Object(filter) = destroy.spec.base() {
                return Some((early_clean_count_subject(filter), "destroyed"));
            }
        }
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
            if let ChooseSpec::All(filter) | ChooseSpec::Object(filter) = exile.spec.base() {
                return Some((early_clean_count_subject(filter), "exiled"));
            }
        }
        None
    }

    fn early_dynamic_token_phrase(
        create_effect: &Effect,
        set_pt_effect: &Effect,
        where_x: String,
    ) -> Option<String> {
        let create = early_create_token(create_effect)?;
        let set_pt = early_set_base_pt(set_pt_effect)?;
        let created_tag = early_effect_tag(create_effect)?;
        if create.count != Value::Fixed(1)
            || create.enters_tapped
            || create.enters_attacking
            || set_pt.duration != Until::Forever
            || set_pt.power.unhinted() != set_pt.toughness.unhinted()
            || matches!(set_pt.power.unhinted(), Value::Fixed(_))
            || !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag)
        {
            return None;
        }
        let token_phrase = describe_token_blueprint(&create.token).replacen("0/0 ", "X/X ", 1);
        (token_phrase != describe_token_blueprint(&create.token)).then(|| {
            format!(
                "create {}, where X is {where_x}",
                with_indefinite_article(&token_phrase)
            )
        })
    }

    fn early_prior_effect_dynamic_count_token_bundle(effects: &[&Effect]) -> Option<String> {
        let [prior_effect, create_effect, set_pt_effect] = effects else {
            return None;
        };
        let with_id = prior_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        if !is_effect_count_reference(&early_set_base_pt(set_pt_effect)?.power, Some(with_id.id)) {
            return None;
        }
        let (subject, action) = early_prior_count_subject(&with_id.effect)?;
        let token_text = early_dynamic_token_phrase(
            create_effect,
            set_pt_effect,
            format!("the number of {subject} {action} this way"),
        )?;
        Some(format!(
            "{}, then {token_text}",
            describe_effect(prior_effect).trim_end_matches('.')
        ))
    }

    fn early_create_token_then_set_base_pt_bundle(effects: &[&Effect]) -> Option<String> {
        let [create_effect, set_pt_effect] = effects else {
            return None;
        };
        let where_x = describe_where_x_basis(&early_set_base_pt(set_pt_effect)?.power)?;
        early_dynamic_token_phrase(create_effect, set_pt_effect, where_x)
            .map(|text| capitalize_first(&text))
    }

    if let Some(compact) = early_prior_effect_dynamic_count_token_bundle(&refs) {
        return Some(compact);
    }
    if refs.len() == 3
        && let Some(token_text) = early_create_token_then_set_base_pt_bundle(&refs[1..])
    {
        return Some(format!(
            "{}. {token_text}",
            describe_effect(refs[0]).trim_end_matches('.')
        ));
    }
    if let [
        look_effect,
        choose_effect,
        reveal_effect,
        move_effect,
        rest_effect,
    ] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, reveal)) = for_each_tagged_for_compaction(reveal_effect)
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            Some(reveal),
            move_to_hand,
            rest,
        )
    {
        return Some(compact);
    }
    if let [look_effect, choose_effect, move_effect, rest_effect] = effects
        && let Some(look_at_top) =
            look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        && let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some((_, move_to_hand)) = for_each_tagged_for_compaction(move_effect)
        && let Some((_, rest)) = for_each_tagged_for_compaction(rest_effect)
        && let Some(compact) = describe_look_at_top_then_put_into_hand_rest_graveyard(
            look_at_top,
            None,
            choose,
            None,
            move_to_hand,
            rest,
        )
    {
        return Some(compact);
    }
    if let Some(compact) = describe_player_protection_from_everything_pair(&refs) {
        return Some(compact);
    }

    if let Some(compact) = describe_draw_then_for_players_choose_exile(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_untap_attacking_then_additional_combat(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_double_power_then_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_pump_all_then_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_put_counters_then_grant_same_filter(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_secret_named_vote_followup_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_council_dilemma_named_vote_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_secret_choice_match_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_each_player_repeat_pay_life_tokens_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_sacrificed_object_conditional_sequence(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_exile_target_and_attached_objects(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_countered_spell_exile_with_counters_gain_suspend(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_put_counters_then_gain_suspend(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_tagged_effect_then_put_counter_on_each(effects) {
        return Some(compact);
    }

    if effects.len() == 3 {
        if let Some(compact) = describe_choose_sacrifice_then_gain_life_for_sacrificed(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_choose_sacrifice_then_draw_for_sacrificed(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_discard_hand_add_mana_draw_sequence(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_planeswalk_chaos_vote_sequence(&refs) {
            return Some(compact);
        }
        if let Some(compact) = describe_named_vote_conditional_sequence(&refs) {
            return Some(compact);
        }
    }
    if effects.len() == 4
        && let Some(compact) = describe_choose_sacrifice_then_return_from_graveyard(&refs)
    {
        return Some(compact);
    }

    if let Some(compact) = describe_counter_unless_then_controller_discards(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_counter_unless_then_kick_count_draw(effects) {
        return Some(compact);
    }

    if let Some(compact) = describe_return_to_hand_then_owner_discards(effects) {
        return Some(compact);
    }

    describe_source_exiled_graveyard_token_sacrifice_structural(effects)
        .or_else(|| describe_roll_choose_destroy_create_structural(effects))
        .or_else(|| describe_roll_choose_draw_then_may_cast_structural(effects))
        .or_else(|| describe_draw_discard_then_conditional_untap_structural(effects))
        .or_else(|| describe_draw_discard_then_create_structural(effects))
        .or_else(|| describe_reveal_top_choice_to_hand_rest_graveyard_structural(effects))
        .or_else(|| describe_reciprocal_creature_control_structural(effects))
        .or_else(|| describe_gain_control_untap_haste_structural(effects))
        .or_else(|| describe_exile_then_free_cast_while_exiled_structural(effects))
        .or_else(|| describe_choose_top_exile_then_conditional_cast_structural(effects))
        .or_else(|| describe_choose_top_exile_then_play_structural(effects))
        .or_else(|| describe_target_card_then_cast_this_turn_structural(effects))
        .or_else(|| describe_choose_name_target_mills_conditional_draw(effects))
        .or_else(|| describe_each_creature_and_player_damage_cant_regenerate_structural(effects))
}

pub(super) fn describe_source_exiled_graveyard_token_sacrifice_structural(
    effects: &[Effect],
) -> Option<String> {
    let [move_effect, create_effect, sacrifice_effect] = effects else {
        return None;
    };
    let with_id = move_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let move_to_zone = with_id
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard || move_to_zone.to_top {
        return None;
    }
    let ChooseSpec::All(filter) = move_to_zone.target.base() else {
        return None;
    };
    if !is_source_exiled_cards_filter(filter) {
        return None;
    }
    let create = create_effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !is_effect_count_reference(&create.count, Some(with_id.id)) {
        return None;
    }
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }

    let token_blueprint = describe_token_blueprint(&create.token);
    let create_text = describe_create_token_action(
        &format!("a {token_blueprint} for each card put into a graveyard this way"),
        &create.controller,
    );
    Some(format!(
        "Put each card exiled with this artifact into its owner's graveyard, then {}. Sacrifice this artifact.",
        lowercase_first(&create_text)
    ))
}

pub(super) fn keyword_label_from_static_ability_id(
    ability: crate::static_abilities::StaticAbilityId,
) -> Option<&'static str> {
    Some(match ability {
        crate::static_abilities::StaticAbilityId::Flying => "flying",
        crate::static_abilities::StaticAbilityId::FirstStrike => "first strike",
        crate::static_abilities::StaticAbilityId::DoubleStrike => "double strike",
        crate::static_abilities::StaticAbilityId::Deathtouch => "deathtouch",
        crate::static_abilities::StaticAbilityId::Haste => "haste",
        crate::static_abilities::StaticAbilityId::Hexproof => "hexproof",
        crate::static_abilities::StaticAbilityId::Indestructible => "indestructible",
        crate::static_abilities::StaticAbilityId::Lifelink => "lifelink",
        crate::static_abilities::StaticAbilityId::Menace => "menace",
        crate::static_abilities::StaticAbilityId::Reach => "reach",
        crate::static_abilities::StaticAbilityId::Trample => "trample",
        crate::static_abilities::StaticAbilityId::Vigilance => "vigilance",
        _ => return None,
    })
}

pub(super) fn describe_double_power_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [for_each_effect, grant_effect] = effects else {
        return None;
    };
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let [pump_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let pump = pump_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if pump.until != Until::EndOfTurn
        || pump.condition.is_some()
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !matches!(pump.target_spec.as_ref(), Some(ChooseSpec::Iterated))
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = pump.runtime_modifications.as_slice()
    else {
        return None;
    };
    if !matches!(power, Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Iterated))
        || !matches!(toughness, Value::Fixed(0))
    {
        return None;
    }

    let grant = grant_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != Until::EndOfTurn
        || grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let filter = match (&grant.target, grant.target_spec.as_ref()) {
        (crate::continuous::EffectTarget::Filter(filter), _) => filter,
        (_, Some(ChooseSpec::Object(filter))) => filter,
        _ => return None,
    };
    if filter != &for_each.filter {
        return None;
    }

    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&description);
    let pronoun = if for_each.filter.card_types.contains(&CardType::Creature) {
        "Those creatures"
    } else {
        "Those objects"
    };
    Some(format!(
        "Double the power of each {filter_text} until end of turn. {pronoun} gain {ability_text} until end of turn"
    ))
}

pub(super) fn apply_continuous_filter(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<&ObjectFilter> {
    match (&effect.target, effect.target_spec.as_ref()) {
        (crate::continuous::EffectTarget::Filter(filter), None) => Some(filter),
        (
            crate::continuous::EffectTarget::Filter(filter),
            Some(ChooseSpec::Object(spec_filter)),
        ) if filter == spec_filter => Some(filter),
        (_, Some(ChooseSpec::Object(filter))) => Some(filter),
        _ => None,
    }
}

pub(super) fn describe_pump_all_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [pump_effect, grant_effect] = effects else {
        return None;
    };
    let pump = unwrap_basic_tag_wrappers(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if pump.until != grant.until
        || pump.condition.is_some()
        || grant.condition.is_some()
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = pump.runtime_modifications.as_slice()
    else {
        return None;
    };
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let pump_filter = apply_continuous_filter(pump)?;
    let grant_filter = apply_continuous_filter(grant)?;
    if pump_filter != grant_filter {
        return None;
    }

    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let subject = capitalize_first(&pluralize_noun_phrase(&pump_filter.description()));
    Some(format!(
        "{subject} get {}/{} and gain {ability_text} {}",
        describe_signed_value(power),
        describe_toughness_delta_with_power_context(power, toughness),
        describe_until(&pump.until)
    ))
}

pub(super) fn describe_leading_effect_then_pump_and_grant_same_filter(
    effects: &[Effect],
) -> Option<String> {
    let [leading, _, _] = effects else {
        return None;
    };
    let suffix = describe_pump_all_then_grant_same_filter(&effects[1..])?;
    let leading = capitalize_first(describe_effect(leading).trim_end_matches('.'));
    Some(format!("{leading}. {suffix}"))
}

pub(super) fn describe_put_counters_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [put_effect, grant_effect] = effects else {
        return None;
    };
    let (put_text, put_filter, put_tag) = put_counters_each_filter_view(put_effect)?;

    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let grant_matches_countered_group = if let Some(put_tag) = put_tag {
        matches!(grant.target_spec.as_ref(), Some(ChooseSpec::Tagged(grant_tag)) if grant_tag == put_tag)
    } else {
        apply_continuous_filter(grant).is_some_and(|grant_filter| grant_filter == put_filter)
    };
    if !grant_matches_countered_group {
        return None;
    }

    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    Some(format!(
        "{put_text}. They gain {ability_text} {}",
        describe_until(&grant.until)
    ))
}

pub(super) fn describe_create_token_then_grant_same_tag(effects: &[Effect]) -> Option<String> {
    let [create_effect, grant_effect] = effects else {
        return None;
    };
    let (created_tag, create) = tagged_create_token_effect(create_effect)?;
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != Until::EndOfTurn
        || grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
        || !matches!(grant.target_spec.as_ref(), Some(ChooseSpec::Tagged(tag)) if tag == created_tag)
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    let token_subject = if matches!(create.count.unhinted(), Value::Fixed(1)) {
        "that token gains"
    } else {
        "they gain"
    };
    let mut create_text = describe_effect(create_effect)
        .trim_end_matches('.')
        .to_string();
    if let Value::Fixed(count) = create.count.unhinted()
        && let Some(count_word) = number_word(*count)
    {
        create_text = create_text
            .replace(
                &format!("Create {count} "),
                &format!("Create {count_word} "),
            )
            .replace(
                &format!("creates {count} "),
                &format!("creates {count_word} "),
            );
    }
    Some(format!(
        "{create_text}. {} {ability_text} {}",
        capitalize_first(token_subject),
        describe_until(&grant.until)
    ))
}

pub(super) fn choose_spec_tag(spec: &ChooseSpec) -> Option<&crate::TagKey> {
    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag),
        _ => None,
    }
}

pub(super) fn find_choice_filter_for_tag(
    effect: &Effect,
    tag: &crate::TagKey,
) -> Option<ObjectFilter> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.tag == *tag
    {
        return Some(choose.filter.clone());
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_choice_filter_for_tag(child, tag);
        }
    });
    found
}

pub(super) fn find_battlefield_move_source_tag(
    effect: &Effect,
    moved_tag: &crate::TagKey,
) -> Option<crate::TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && tagged.tag == *moved_tag
    {
        if let Some(move_to_zone) = tagged
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            && move_to_zone.zone == Zone::Battlefield
        {
            return choose_spec_tag(&move_to_zone.target).cloned();
        }
        if let Some(put_onto_battlefield) = tagged
            .effect
            .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
        {
            return choose_spec_tag(&put_onto_battlefield.target).cloned();
        }
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_battlefield_move_source_tag(child, moved_tag);
        }
    });
    found
}

pub(super) fn find_reference_filter_for_tag(
    effects: &[Effect],
    tag: &crate::TagKey,
) -> Option<ObjectFilter> {
    for effect in effects {
        if let Some(filter) = find_choice_filter_for_tag(effect, tag) {
            return Some(filter);
        }
    }
    for effect in effects {
        let source_tag = find_battlefield_move_source_tag(effect, tag)?;
        for effect in effects {
            if let Some(filter) = find_choice_filter_for_tag(effect, &source_tag) {
                return Some(filter);
            }
        }
    }
    None
}

pub(super) fn demonstrative_subject_for_filter(filter: &ObjectFilter) -> Option<String> {
    if filter.subtypes.len() == 1 {
        return Some(format!("That {}", filter.subtypes[0]));
    }
    if filter.card_types.len() == 1 {
        return Some(format!(
            "That {}",
            filter.card_types[0].name().to_ascii_lowercase()
        ));
    }
    if filter.card_types.contains(&CardType::Creature) {
        return Some("That creature".to_string());
    }
    if filter.card_types.contains(&CardType::Artifact) {
        return Some("That artifact".to_string());
    }
    None
}

pub(super) fn tagged_haste_grant(effect: &Effect) -> Option<(&crate::TagKey, &Until)> {
    let apply = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !apply.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return None;
    };
    if ability.id() != crate::static_abilities::StaticAbilityId::Haste {
        return None;
    }
    let Some(ChooseSpec::Tagged(tag)) = apply.target_spec.as_ref() else {
        return None;
    };
    Some((tag, &apply.until))
}

pub(super) fn delayed_next_end_step_cleanup(
    effect: &Effect,
    tag: &crate::TagKey,
) -> Option<&'static str> {
    let schedule = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if !schedule.one_shot
        || schedule.start_next_turn
        || schedule.until_end_of_turn
        || !schedule
            .trigger
            .display()
            .to_ascii_lowercase()
            .contains("end step")
    {
        return None;
    }
    let delayed = schedule.effects.flattened_default_effects();
    let [cleanup] = delayed else {
        return None;
    };
    if let Some(sacrifice) = cleanup.downcast_ref::<crate::effects::SacrificeTargetEffect>()
        && matches!(choose_spec_tag(&sacrifice.target), Some(candidate) if candidate == tag)
    {
        return Some("Sacrifice it at the beginning of the next end step");
    }
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(cleanup).downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Exile
        && matches!(choose_spec_tag(&move_to_zone.target), Some(candidate) if candidate == tag)
    {
        return Some("Exile it at the beginning of the next end step");
    }
    None
}

pub(super) fn describe_moved_object_haste_delayed_cleanup(effects: &[Effect]) -> Option<String> {
    if effects.len() < 3 {
        return None;
    }
    let grant_idx = effects.len() - 2;
    let cleanup_idx = effects.len() - 1;
    let prefix_effects = &effects[..grant_idx];
    let (tag, duration) = tagged_haste_grant(&effects[grant_idx])?;
    let cleanup = delayed_next_end_step_cleanup(&effects[cleanup_idx], tag)?;
    let filter = find_reference_filter_for_tag(prefix_effects, tag)?;
    let _ = demonstrative_subject_for_filter(&filter)?;
    let duration_text = match duration {
        Until::Forever => "",
        Until::EndOfTurn => " until end of turn",
        _ => return None,
    };
    let prefix = describe_effect_list(prefix_effects)
        .replace(
            "put it onto the battlefield",
            "put that card onto the battlefield",
        )
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{prefix}. It gains haste{duration_text}. {cleanup}"
    ))
}

pub(super) fn describe_draw_count_then_grant_same_filter(effects: &[Effect]) -> Option<String> {
    let [draw_effect, grant_effect] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let Value::Count(draw_filter) = &draw.count else {
        return None;
    };
    let grant = unwrap_basic_tag_wrappers(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.condition.is_some()
        || grant.target_spec.is_some()
        || !grant.runtime_modifications.is_empty()
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(grant_filter) = &grant.target else {
        return None;
    };
    if grant_filter != draw_filter {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    let ability_text = keyword_label_from_static_ability_id(ability.id())?;
    Some(format!(
        "{}. They gain {ability_text} {}",
        describe_effect(draw_effect).trim_end_matches('.'),
        describe_until(&grant.until)
    ))
}

pub(super) fn describe_sacrificed_tagged_condition(condition: &Condition) -> Option<String> {
    let Condition::TaggedObjectMatches(tag, filter) = condition else {
        return None;
    };
    if !tag.as_str().starts_with("sacrificed_") {
        return None;
    }

    let mut filter = filter.clone();
    filter.zone = None;
    let subject = with_indefinite_article(strip_indefinite_article(&filter.description()));
    Some(format!("{subject} is sacrificed this way"))
}

pub(super) fn describe_exile_target_and_attached_objects(effects: &[Effect]) -> Option<String> {
    let [target_effect, attached_exile_effect, target_exile_effect] = effects else {
        return None;
    };
    let tagged_target = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged_target
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let target_exile = target_exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if target_exile.zone != Zone::Exile
        || !matches!(&target_exile.target, ChooseSpec::Tagged(tag) if tag == &tagged_target.tag)
    {
        return None;
    }

    let attached_exile = attached_exile_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(attached_exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if attached_exile.face_down {
        return None;
    }
    let ChooseSpec::All(attached_filter) = &attached_exile.spec else {
        return None;
    };
    let matching_constraints = attached_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
                && constraint.tag == tagged_target.tag
        })
        .count();
    if matching_constraints != 1 {
        return None;
    }

    let mut described_filter = attached_filter.clone();
    described_filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
            && constraint.tag == tagged_target.tag)
    });
    if described_filter == ObjectFilter::default() {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    let attached_text = described_filter.description();
    Some(format!(
        "Exile {target_text} and all {attached_text} attached to it"
    ))
}

pub(super) fn describe_sacrificed_object_conditional_sequence(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let mut parts = Vec::with_capacity(effects.len());
    for effect in effects {
        let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_false.is_empty() || conditional.if_true.is_empty() {
            return None;
        }
        let condition_text = describe_condition(&conditional.condition);
        if !condition_text.starts_with("the sacrificed ") {
            return None;
        }
        let rendered = describe_effect(effect);
        let trimmed = rendered.trim().trim_end_matches('.');
        if trimmed.is_empty()
            || trimmed.contains(". ")
            || trimmed.contains(": ")
            || trimmed.starts_with("If ")
            || trimmed.starts_with("When ")
            || trimmed.starts_with("Whenever ")
            || trimmed.starts_with("At ")
        {
            return None;
        }
        parts.push(trimmed.to_string());
    }
    Some(parts.join(". "))
}

pub(super) fn describe_sacrifice_then_sacrificed_conditional_sequence(
    effects: &[Effect],
) -> Option<String> {
    let (conditional_effect, prior_effects) = effects.split_last()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(tag, _) = &conditional.condition else {
        return None;
    };
    if !tag.as_str().starts_with("sacrificed_") {
        return None;
    }
    let has_matching_sacrifice = prior_effects.iter().any(|effect| {
        sacrifice_view_unwrapped(effect)
            .is_some_and(|view| filter_is_exactly_tagged(view.filter, tag))
    });
    if !has_matching_sacrifice {
        return None;
    }

    let prefix = describe_effect_list(prior_effects);
    let conditional_text = describe_effect(conditional_effect);
    let prefix = prefix.trim().trim_end_matches('.');
    let conditional_text = conditional_text.trim().trim_end_matches('.');
    if prefix.is_empty() || conditional_text.is_empty() || !conditional_text.starts_with("If ") {
        return None;
    }
    Some(format!("{prefix}. {conditional_text}"))
}

pub(super) fn unwrap_with_id(effect: &Effect) -> (&Effect, Option<crate::effect::EffectId>) {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return (&with_id.effect, Some(with_id.id));
    }
    (effect, None)
}

pub(super) fn describe_each_player_repeat_pay_life_tokens_sequence(
    effects: &[Effect],
) -> Option<String> {
    let [repeat_effect, token_effect] = effects else {
        return None;
    };
    let (repeat_unwrapped, repeat_id) = unwrap_with_id(repeat_effect);
    let repeat = repeat_unwrapped.downcast_ref::<crate::effects::RepeatProcessEffect>()?;
    if !matches!(repeat.predicate, crate::effect::EffectPredicate::Happened) {
        return None;
    }
    let [pay_players_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let (pay_players_unwrapped, _) = unwrap_with_id(pay_players_effect);
    let pay_players = pay_players_unwrapped.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if pay_players.filter != PlayerFilter::Any || !pay_players.starting_with_controller {
        return None;
    }
    let [pay_life_effect] = pay_players.effects.as_slice() else {
        return None;
    };
    let pay_life = pay_life_effect.downcast_ref::<crate::effects::PayAnyLifeEffect>()?;
    if pay_life.min_amount != 0
        || pay_life.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    {
        return None;
    }

    let token_players = token_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if token_players.filter != PlayerFilter::Any {
        return None;
    }
    let [create_effect] = token_players.effects.as_slice() else {
        return None;
    };
    let create = unwrap_basic_tag_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.controller != PlayerFilter::IteratedPlayer
        || !create.token.card.is_token
        || create.token.card.name != "Rat"
    {
        return None;
    }
    if !create
        .token
        .card
        .card_types
        .contains(&crate::types::CardType::Creature)
        || !create
            .token
            .card
            .subtypes
            .contains(&crate::types::Subtype::Rat)
        || create.token.card.color_indicator != Some(crate::color::ColorSet::BLACK)
        || !create.token.card.power_toughness.is_some_and(|pt| {
            matches!(pt.power, crate::card::PtValue::Fixed(1))
                && matches!(pt.toughness, crate::card::PtValue::Fixed(1))
        })
    {
        return None;
    }
    let Value::EffectMetric {
        effect_id,
        source: crate::effect::EffectMetricSource::Outcome,
        metric: crate::effect::EffectMetric::IteratedPlayerCount,
    } = &create.count
    else {
        return None;
    };
    if repeat_id != Some(*effect_id) {
        return None;
    }

    Some("Starting with you, each player may pay any amount of life. Repeat this process until no one pays life. Each player creates a 1/1 black Rat creature token for each 1 life they paid this way".to_string())
}

pub(super) fn describe_reveal_top_to_hand_then_lose_mana_value_effects(
    effects: &[Effect],
) -> Option<String> {
    let [reveal_effect, move_effect, lose_effect] = effects else {
        return None;
    };
    let reveal = unwrap_basic_tag_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTopEffect>()?;
    if reveal.player != PlayerFilter::You {
        return None;
    }
    let tag = reveal.tag.as_ref()?;
    let move_effect = unwrap_basic_tag_wrappers(move_effect);
    let moves_tag_to_hand = move_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some_and(|return_to_hand| {
            matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(found) if found == tag)
        })
        || move_effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_to_zone| {
                move_to_zone.zone == Zone::Hand
                    && matches!(
                        move_to_zone.target.base(),
                        ChooseSpec::Tagged(found) if found == tag
                    )
            });
    if !moves_tag_to_hand {
        return None;
    }
    let lose_life =
        unwrap_basic_tag_wrappers(lose_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose_life.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    if !matches!(
        &lose_life.amount,
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(found) if found == tag)
    ) {
        return None;
    }
    Some(
        "Reveal the top card of your library and put that card into your hand. You lose life equal to that card's mana value"
            .to_string(),
    )
}

pub(super) fn is_all_attacking_creatures(spec: &ChooseSpec) -> bool {
    let ChooseSpec::All(filter) = spec.base() else {
        return false;
    };
    if !filter.attacking {
        return false;
    }
    let mut base = filter.clone();
    base.attacking = false;
    base == ObjectFilter::creature()
}

pub(super) fn describe_untap_attacking_then_additional_combat(
    effects: &[Effect],
) -> Option<String> {
    let [untap_effect, phases_effect] = effects else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if !is_all_attacking_creatures(&untap.target) {
        return None;
    }
    let additional_phases =
        phases_effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()?;
    if additional_phases.phases != [crate::effects::AdditionalPhase::Combat] {
        return None;
    }
    Some(
        "Untap each attacking creature. After this phase, there is an additional combat phase"
            .to_string(),
    )
}

pub(super) fn describe_counter_unless_then_kick_count_draw(effects: &[Effect]) -> Option<String> {
    let [unless_effect, draw_effect] = effects else {
        return None;
    };
    let unless_pays = unwrap_structural_effect_tag(unless_effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [_counter] = unless_pays.effects.as_slice() else {
        return None;
    };
    unless_pays.effects[0].downcast_ref::<crate::effects::CounterEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::KickCount {
        return None;
    }

    let counter_text = describe_effect(unless_effect)
        .trim_end_matches('.')
        .to_string();
    let draw_text = if draw.player == PlayerFilter::You {
        "Draw a card for each time this spell was kicked".to_string()
    } else {
        describe_draw_for_each(draw)?
            .trim_end_matches('.')
            .to_string()
    };
    Some(format!("{counter_text}. {draw_text}"))
}

pub(super) fn describe_counter_unless_then_controller_discards(
    effects: &[Effect],
) -> Option<String> {
    let [unless_effect, discard_effect] = effects else {
        return None;
    };
    let countered_tag = structural_effect_tag(unless_effect)?.clone();
    let unless_pays = unwrap_structural_effect_tag(unless_effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [counter_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    counter_effect.downcast_ref::<crate::effects::CounterEffect>()?;

    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
        || discard.player
            != PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(countered_tag))
    {
        return None;
    }

    let counter_text = describe_effect(unless_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!("{counter_text}. That player discards a card."))
}

pub(super) fn describe_return_to_hand_then_owner_discards(effects: &[Effect]) -> Option<String> {
    let [return_effect, discard_effect] = effects else {
        return None;
    };
    let returned_tag = structural_effect_tag(return_effect)?;
    unwrap_structural_effect_tag(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
        || discard.player
            != PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(returned_tag.clone()))
    {
        return None;
    }

    let return_text = describe_effect(return_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!("{return_text}, then that player discards a card."))
}

pub(super) fn structural_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    None
}

pub(super) fn unwrap_structural_effect_tag(effect: &Effect) -> &Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_structural_effect_tag(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_structural_effect_tag(&tag_all.effect);
    }
    effect
}

pub(super) fn describe_roll_choose_destroy_create_structural(effects: &[Effect]) -> Option<String> {
    let [roll_effect, destroy_effect, create_effect] = effects else {
        return None;
    };
    let with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    with_id
        .effect
        .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()?;

    fn unwrap_tags(effect: &Effect) -> &Effect {
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return unwrap_tags(&tag_all.effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_tags(&tagged.effect);
        }
        effect
    }

    let destroy = unwrap_tags(destroy_effect).downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(filter) = &destroy.spec else {
        return None;
    };
    if filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }
    let Some(crate::filter::Comparison::GreaterThanOrEqualExpr(value)) = &filter.power else {
        return None;
    };
    if !matches!(value.unhinted(), Value::EffectValue(id) if *id == with_id.id) {
        return None;
    }

    let create = unwrap_tags(create_effect).downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !matches!(
        create.count.unhinted(),
        Value::EffectMetric {
            effect_id,
            metric: crate::effect::EffectMetric::OtherNumber,
            ..
        } if *effect_id == with_id.id
    ) {
        return None;
    }

    let roll_text = describe_effect(roll_effect)
        .trim_end_matches('.')
        .to_string();
    let destroy_text = describe_effect(destroy_effect)
        .trim_end_matches('.')
        .to_string();
    let create_text = lowercase_first(describe_effect(create_effect).trim_end_matches('.'));
    Some(format!("{roll_text}. {destroy_text}. Then {create_text}."))
}

pub(super) fn describe_roll_choose_draw_then_may_cast_structural(
    effects: &[Effect],
) -> Option<String> {
    let [roll_effect, draw_effect, may_effect] = effects else {
        return None;
    };
    let with_id = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    with_id
        .effect
        .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()?;

    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !matches!(draw.count.unhinted(), Value::EffectValue(id) if *id == with_id.id)
    {
        return None;
    }

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    cast_effect
        .downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()?;

    let roll_text = describe_effect(roll_effect)
        .trim_end_matches('.')
        .to_string();
    let may_text = lowercase_first(describe_effect(cast_effect).trim_end_matches('.'));
    Some(format!(
        "{roll_text}. Draw cards equal to that result. Then {may_text}."
    ))
}

pub(super) fn describe_draw_discard_then_create_structural(effects: &[Effect]) -> Option<String> {
    let [draw_effect, discard_effect, create_effect] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let draw_discard = describe_draw_then_discard(draw, discard)?;
    let create = describe_effect(create_effect);
    Some(format!("{}. {}", capitalize_first(&draw_discard), create))
}

pub(super) fn describe_draw_discard_then_conditional_untap_structural(
    effects: &[Effect],
) -> Option<String> {
    let (draw_effect, discard_effect, conditional_effect) = match effects {
        [draw_effect, discard_effect, conditional_effect] => {
            (draw_effect, discard_effect, conditional_effect)
        }
        [
            target_effect,
            draw_effect,
            discard_effect,
            conditional_effect,
        ] => {
            target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            (draw_effect, discard_effect, conditional_effect)
        }
        _ => return None,
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let discard = unwrap_structural_effect_tag(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let discard_tag = structural_effect_tag(discard_effect).or(discard.tag.as_ref())?;
    let draw_discard = describe_draw_then_discard(draw, discard)?;

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
    } = &conditional.condition
    else {
        return None;
    };
    if tag != discard_tag || player != &draw.player {
        return None;
    }
    let [untap_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if !matches!(untap.target.unhinted(), ChooseSpec::Source) {
        return None;
    }

    let subject = if *player == PlayerFilter::You {
        "you".to_string()
    } else {
        "that player".to_string()
    };
    let object_text = describe_player_tagged_object_text(tag, filter);
    let untap_text = lowercase_first(describe_effect(untap_effect).trim_end_matches('.'));
    Some(format!(
        "{}. If {subject} discards {object_text} this way, {untap_text}.",
        capitalize_first(&draw_discard)
    ))
}

pub(super) fn join_or_list(items: &[String]) -> Option<String> {
    match items {
        [] => None,
        [one] => Some(one.clone()),
        [first, second] => Some(format!("{first} or {second}")),
        _ => {
            let (last, rest) = items.split_last()?;
            Some(format!("{}, or {last}", rest.join(", ")))
        }
    }
}

pub(super) fn structural_revealed_choice_label(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if looked_filter_is_creature_land_union(&choose.filter)
        && choose.filter.static_abilities.is_empty()
        && choose.filter.all_card_types.is_empty()
    {
        return Some("creature or land card".to_string());
    }

    if choose.filter.card_types.len() == 1
        && choose.filter.static_abilities.is_empty()
        && choose.filter.any_of.is_empty()
    {
        return Some(format!(
            "{} card",
            describe_card_type_word_local(choose.filter.card_types[0])
        ));
    }

    if choose.filter.card_types.is_empty()
        && choose.filter.static_abilities.is_empty()
        && !choose.filter.any_of.is_empty()
    {
        let mut type_words = Vec::new();
        for candidate in &choose.filter.any_of {
            if candidate.card_types.len() != 1
                || !candidate.all_card_types.is_empty()
                || !candidate.subtypes.is_empty()
                || !candidate.static_abilities.is_empty()
                || !candidate.any_of.is_empty()
            {
                return None;
            }
            type_words.push(describe_card_type_word_local(candidate.card_types[0]).to_string());
        }
        return Some(format!("{} card", join_or_list(&type_words)?));
    }

    None
}

pub(super) fn structural_revealed_choice_phrase(label: &str) -> String {
    with_indefinite_article(label)
}

pub(super) fn choose_references_tag(
    choose: &crate::effects::ChooseObjectsEffect,
    tag: &crate::TagKey,
) -> bool {
    choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    })
}

pub(super) fn for_each_moves_tagged_iterated_to_hand(effect: &Effect, tag: &crate::TagKey) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let inner = structural_unwrap_render_wrappers(&for_each.effects[0]);
    matches!(
        inner.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Hand
                && !move_to_zone.to_top
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    ) || matches!(
        inner.downcast_ref::<crate::effects::ReturnToHandEffect>(),
        Some(return_to_hand) if matches!(return_to_hand.spec.base(), ChooseSpec::Iterated)
    )
}
