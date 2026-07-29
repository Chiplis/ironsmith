use super::*;

pub(super) fn describe_for_players_choose_each_graveyard_then_owner_shuffle(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [choose_effect, shuffle_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || choose.zone != Some(Zone::Graveyard)
        || !choose.additional_zones.is_empty()
    {
        return None;
    }
    let exact = match (choose.count.min, choose.count.max) {
        (min, Some(max))
            if min == max && min >= 2 && !choose.count.dynamic_x && !choose.count.random =>
        {
            min
        }
        _ => return None,
    };

    let mut normalized_filter = choose.filter.clone();
    normalized_filter.union_surface = Default::default();
    let mut expected_filter = ObjectFilter::default();
    expected_filter.zone = Some(Zone::Graveyard);
    expected_filter.owner = Some(PlayerFilter::IteratedPlayer);
    if normalized_filter != expected_filter {
        return None;
    }

    let (shuffle_effect, _) = unwrap_with_id(shuffle_effect);
    let shuffle =
        shuffle_effect.downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    let ChooseSpec::Tagged(shuffle_tag) = shuffle.target.base() else {
        return None;
    };
    if shuffle_tag != &choose.tag
        || shuffle.owner_library_destination
        || shuffle.possessive_owner_subject
        || !matches!(
            &shuffle.player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(owner_tag))
                | PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(owner_tag))
                if owner_tag == shuffle_tag
        )
    {
        return None;
    }

    let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
    Some(format!(
        "Choose {count} cards in each graveyard. Their owners shuffle those cards into their libraries"
    ))
}

pub(super) fn describe_for_players_unless_pays(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [effect] = for_players.effects.as_slice() else {
        return None;
    };
    let unless_pays = effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless_pays.player != PlayerFilter::IteratedPlayer {
        return None;
    }

    let player_filter_text = describe_for_each_player_filter(&for_players.filter);
    let each_player = strip_leading_article(&player_filter_text);
    let consequence = describe_effect_list(&unless_pays.effects);
    let payment = describe_total_cost_payment(&unless_pays.cost);
    Some(format!(
        "For each {each_player}, {consequence} unless they pay {payment}"
    ))
}

pub(super) fn describe_for_each_prevent_combat_damage_unless_pays(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [effect] = for_each.effects.as_slice() else {
        return None;
    };
    let unless_pays = effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [prevent_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let prevents_iterated_until_eot = prevent_effect
        .downcast_ref::<crate::effects::PreventAllCombatDamageFromEffect>()
        .is_some_and(|prevent_from| {
            matches!(prevent_from.source, ChooseSpec::Iterated)
                && matches!(prevent_from.until, Until::EndOfTurn)
        })
        || prevent_effect
            .downcast_ref::<crate::effects::PreventAllCombatDamageEffect>()
            .is_some_and(|prevent_combat| {
                matches!(
                    prevent_combat.target,
                    crate::effects::CombatDamagePreventionTarget::From(ChooseSpec::Iterated)
                ) && matches!(prevent_combat.until, Until::EndOfTurn)
            });
    if !prevents_iterated_until_eot {
        return None;
    }

    let filter_text = strip_indefinite_article(&for_each.filter.description()).to_string();
    let source_text = if for_each
        .filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        "that creature"
    } else {
        "that object"
    };
    let payer = describe_player_filter(&unless_pays.player);
    let payment = describe_total_cost_payment(&unless_pays.cost);
    Some(format!(
        "For each {filter_text}, prevent all combat damage that would be dealt by {source_text} this turn unless {payer} pays {payment}"
    ))
}

pub(super) fn filter_is_tagged_it(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == "__it__"
    })
}

pub(super) fn describe_tagged_it_damage_source(filter: &ObjectFilter) -> Option<&'static str> {
    if !filter_is_tagged_it(filter) {
        return None;
    }
    if filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return Some("that creature");
    }
    if filter
        .card_types
        .contains(&crate::types::CardType::Artifact)
    {
        return Some("that artifact");
    }
    if filter
        .card_types
        .contains(&crate::types::CardType::Enchantment)
    {
        return Some("that enchantment");
    }
    if filter.card_types.contains(&crate::types::CardType::Land) {
        return Some("that land");
    }
    Some("that object")
}

pub(super) fn describe_colored_card_type_damage_sources(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.len() != 1 {
        return None;
    }
    let colors = filter.colors?;
    if colors.is_empty() {
        return None;
    }
    if filter.zone.is_some_and(|zone| zone != Zone::Battlefield) {
        return None;
    }
    let expected = ObjectFilter {
        zone: filter.zone,
        card_types: filter.card_types.clone(),
        colors: Some(colors),
        ..Default::default()
    };
    if filter != &expected {
        return None;
    }
    let type_text = pluralize_noun_phrase(filter.card_types[0].name());
    let sources = crate::color::Color::ALL
        .into_iter()
        .filter(|color| colors.contains(*color))
        .map(|color| format!("{} {type_text}", color.name()))
        .collect::<Vec<_>>();
    Some(join_with_and(&sources))
}

pub(crate) fn describe_tagged_this_way_action(filter: &ObjectFilter) -> Option<&'static str> {
    filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        let tag = constraint.tag.as_str();
        if tag == "__it__" && filter.zone == Some(Zone::Exile) {
            return Some("exiled");
        }
        this_way_action_from_tag(&constraint.tag)
    })
}

pub(crate) fn describe_each_controlled_by_iterated(filter: &ObjectFilter) -> Option<String> {
    if filter.controller != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    if !filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.colors.is_none()
        && filter.excluded_card_types.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.excluded_colors.is_empty()
        && !filter.token
        && !filter.nontoken
        && !filter.tapped
        && !filter.untapped
        && !filter.attacking
        && !filter.nonattacking
        && !filter.blocking
        && !filter.nonblocking
        && !filter.blocked
        && !filter.unblocked
        && matches!(filter.zone, None | Some(Zone::Battlefield))
        && filter.tagged_constraints.is_empty()
        && filter.targets_object.is_none()
        && filter.targets_player.is_none()
        && filter.ability_markers.is_empty()
        && filter.excluded_ability_markers.is_empty()
        && !filter.noncommander
    {
        let words = filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        let list = match words.len() {
            0 => String::new(),
            1 => words[0].clone(),
            2 => format!("{} and {}", words[0], words[1]),
            _ => {
                let mut out = words[..words.len() - 1].join(", ");
                out.push_str(", and ");
                out.push_str(&words[words.len() - 1]);
                out
            }
        };
        let other = if filter.other { "other " } else { "" };
        return Some(format!("each {other}{list} they control"));
    }
    None
}

pub(crate) fn describe_for_players_damage_and_controlled_damage(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let deal_player = for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        deal_player.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }
    let for_each = for_players.effects[1].downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let deal_object = if let Some(deal) =
        for_each.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()
    {
        deal
    } else if let Some(tagged) = for_each.effects[0].downcast_ref::<crate::effects::TaggedEffect>()
    {
        tagged
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()?
    } else {
        return None;
    };
    if deal_object.amount != deal_player.amount {
        return None;
    }
    if !matches!(deal_object.target, ChooseSpec::Iterated) {
        return None;
    }
    let objects = describe_each_controlled_by_iterated(&for_each.filter)?;
    if for_players.filter == PlayerFilter::Any
        && matches!(
            for_each.filter.controller,
            Some(PlayerFilter::IteratedPlayer)
        )
        && for_each.filter.other
        && for_each.filter.card_types == vec![CardType::Creature]
    {
        return Some(format!(
            "Deal {} damage to each player and each other creature",
            describe_value(&deal_player.amount)
        ));
    }
    let player_filter_text = describe_for_each_player_filter(&for_players.filter);
    let each_player = strip_leading_article(&player_filter_text);
    Some(format!(
        "Deal {} damage to each {} and {}",
        describe_value(&deal_player.amount),
        each_player,
        objects
    ))
}

pub(crate) fn describe_for_players_reveal_top_mana_value_life_then_put_into_hand(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let (subject, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "their"),
        PlayerFilter::Opponent => ("Each opponent", "their"),
        _ => return None,
    };
    if for_players.effects.len() != 3 {
        return None;
    }
    let reveal_effect =
        crate::compiled_text::render_effects::sequences_and_votes::unwrap_basic_tag_wrappers(
            &for_players.effects[0],
        );
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    // The reveal's tag may live on the effect itself or on a Tagged/WithId
    // wrapper the sentence lowering added around it.
    let reveal_tag = reveal.tag.as_ref().or_else(|| {
        crate::compiled_text::render_effects::sequences_and_votes::wrapped_effect_tag(
            &for_players.effects[0],
        )
    })?;
    if reveal.player != PlayerFilter::IteratedPlayer
        || !(reveal_tag.as_str().starts_with("revealed_")
            || crate::cards::is_sentence_helper_tag(reveal_tag.as_str(), "revealed"))
    {
        return None;
    }
    let lose =
        crate::compiled_text::render_effects::sequences_and_votes::unwrap_basic_tag_wrappers(
            &for_players.effects[1],
        )
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let Value::ManaValueOf(spec) = lose.amount.unhinted() else {
        return None;
    };
    if !matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == reveal_tag) {
        return None;
    }
    let move_to_zone =
        crate::compiled_text::render_effects::sequences_and_votes::unwrap_basic_tag_wrappers(
            &for_players.effects[2],
        )
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand {
        return None;
    }
    if !matches!(
        move_to_zone.target.base(),
        ChooseSpec::Tagged(tag) if tag == reveal_tag
    ) {
        return None;
    }

    Some(format!(
        "{subject} reveals the top card of {possessive} library, loses life equal to that card's mana value, then puts it into {possessive} hand"
    ))
}

pub(super) fn describe_for_players_shuffle_then_conditional_consult(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    fn unwrap_tagged<'a>(effect: &'a Effect) -> &'a Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_tagged(&tagged.effect);
        }
        effect
    }

    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 5 {
        return None;
    }

    let tagged_all =
        for_players.effects[0].downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if tagged_all.zone != Some(Zone::Battlefield) || !tagged_all.additional_zones.is_empty() {
        return None;
    }
    if tagged_all.filter.zone != Some(Zone::Battlefield)
        || tagged_all.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || tagged_all.filter.card_types != vec![crate::types::CardType::Creature]
        || tagged_all.filter.nontoken
    {
        return None;
    }

    let tagged =
        for_players.effects[1].downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if tagged.zone != Some(Zone::Battlefield) || !tagged.additional_zones.is_empty() {
        return None;
    }
    if tagged.filter.zone != Some(Zone::Battlefield)
        || tagged.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || tagged.filter.card_types != vec![crate::types::CardType::Creature]
        || !tagged.filter.nontoken
    {
        return None;
    }

    let move_to_library = unwrap_tagged(&for_players.effects[2])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_library.zone != Zone::Library || move_to_library.to_top {
        return None;
    }
    let ChooseSpec::Tagged(shuffle_tag) = move_to_library.target.base() else {
        return None;
    };
    if shuffle_tag != &tagged_all.tag {
        return None;
    }

    let is_iterated_or_shuffled_owner = |player: &PlayerFilter| {
        *player == PlayerFilter::IteratedPlayer
            || matches!(
                player,
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag))
                    if tag == &tagged_all.tag
            )
    };

    let shuffle = for_players.effects[3].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !is_iterated_or_shuffled_owner(&shuffle.player) {
        return None;
    }

    let conditional = for_players.effects[4].downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
        mode,
    } = &conditional.condition
    else {
        return None;
    };
    if !is_iterated_or_shuffled_owner(player)
        || *mode != crate::effect::TaggedObjectMatchMode::CurrentOrLastKnown
        || tag != &tagged.tag
        || filter.zone != Some(Zone::Library)
        || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 3
    {
        return None;
    }

    let consult =
        conditional.if_true[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if !is_iterated_or_shuffled_owner(&consult.player)
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.stop_rule != crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        || consult.filter.zone.is_some()
        || consult.filter.card_types != vec![crate::types::CardType::Creature]
    {
        return None;
    }

    let move_to_battlefield = unwrap_tagged(&conditional.if_true[1])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_battlefield.zone != Zone::Battlefield
        || move_to_battlefield.to_top
        || move_to_battlefield.enters_tapped
    {
        return None;
    }
    let ChooseSpec::Tagged(matched_tag) = move_to_battlefield.target.base() else {
        return None;
    };
    if matched_tag != &consult.match_tag {
        return None;
    }

    let rest = conditional.if_true[2]
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if !is_iterated_or_shuffled_owner(&rest.player)
        || rest.tag != consult.all_tag
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag != &consult.match_tag)
        || rest.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
    {
        return None;
    }

    Some("Each player shuffles all creatures they own into their library. Each player who shuffled a nontoken creature into their library this way reveals cards from the top of their library until they reveal a creature card, then puts that card onto the battlefield and the rest on the bottom of their library in a random order.".to_string())
}

pub(super) fn describe_for_players_shuffle_reveal_permanents_put_rest_bottom(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 3 {
        return None;
    }

    let tagged_shuffle = for_players.effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let moved_tag = tagged_shuffle.tag.clone();
    let with_id = tagged_shuffle
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let shuffle = with_id
        .effect
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    if shuffle.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let ChooseSpec::Object(shuffled_filter) = shuffle.target.base() else {
        return None;
    };
    if shuffled_filter.zone != Some(Zone::Battlefield)
        || shuffled_filter.owner != Some(PlayerFilter::IteratedPlayer)
        || !shuffled_filter.card_types.contains(&CardType::Artifact)
        || !shuffled_filter.card_types.contains(&CardType::Creature)
        || !shuffled_filter.card_types.contains(&CardType::Enchantment)
        || !shuffled_filter.card_types.contains(&CardType::Land)
    {
        return None;
    }

    let look = for_players.effects[1].downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let look_player_matches = look.player == PlayerFilter::IteratedPlayer
        || look.player == PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(moved_tag));
    if !look_player_matches {
        return None;
    }
    if !matches!(
        &look.count,
        Value::EffectMetric {
            effect_id,
            metric: crate::effect::EffectMetric::Count,
            ..
        } if *effect_id == with_id.id
    ) {
        return None;
    }

    let for_each = for_players.effects[2].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let conditional = for_each.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        return None;
    };
    if tag.as_str() != "__it__"
        || !filter.card_types.contains(&CardType::Artifact)
        || !filter.card_types.contains(&CardType::Creature)
        || !filter.card_types.contains(&CardType::Land)
        || !filter.card_types.contains(&CardType::Enchantment)
    {
        return None;
    }
    let [move_if_true] = conditional.if_true.as_slice() else {
        return None;
    };
    let move_to_battlefield = move_if_true.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_battlefield.zone != Zone::Battlefield
        || move_to_battlefield.to_top
        || move_to_battlefield.battlefield_controller
            != crate::effects::BattlefieldController::Owner
    {
        return None;
    }
    let [move_if_false] = conditional.if_false.as_slice() else {
        return None;
    };
    let move_to_library = move_if_false.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_library.zone != Zone::Library || move_to_library.to_top {
        return None;
    }

    Some("Each player shuffles all permanents they own into their library, then reveals that many cards from the top of their library. Each player puts all artifact, creature, and land cards revealed this way onto the battlefield, then does the same for enchantment cards, then puts all cards revealed this way that weren't put onto the battlefield on the bottom of their library".to_string())
}

pub(crate) fn describe_draw_for_each(draw: &crate::effects::DrawCardsEffect) -> Option<String> {
    let player = describe_player_filter(&draw.player);
    let verb = player_verb(&player, "draw", "draws");
    if let Value::Count(filter) = draw.count.unhinted()
        && filter.zone == Some(Zone::Hand)
        && matches!(draw.player, PlayerFilter::Target(_))
        && filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(&draw.player, owner))
    {
        return Some(format!(
            "{player} {verb} cards equal to the number of cards in their hand"
        ));
    }
    if let Some(equal_to) = describe_equal_to_card_action_count(&draw.count) {
        return Some(format!("{player} {verb} {equal_to}"));
    }
    if let Some(dynamic_for_each) = describe_draw_count_for_each_phrase(&draw.count) {
        return Some(format!("{player} {verb} {dynamic_for_each}"));
    }
    match &draw.count {
        Value::SourcePower => Some(format!("{player} {verb} cards equal to its power")),
        Value::PowerOf(spec) => {
            if let Some(basis) = describe_tagged_creature_power_count_basis(spec) {
                Some(format!("{player} {verb} X cards, where X is {basis}"))
            } else {
                Some(format!(
                    "{player} {verb} cards equal to {}",
                    describe_power_card_count_basis(spec)
                ))
            }
        }
        Value::Count(filter) => Some(format!(
            "{player} {verb} a card for each {}",
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
            Some(format!(
                "{player} {verb} a card for each creature that died {suffix}"
            ))
        }
        Value::SpellsCastThisTurn(spell_caster) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_spells_cast_this_turn_each(spell_caster)
        )),
        Value::KickCount => Some(format!(
            "{player} {verb} a card for each time this spell was kicked"
        )),
        Value::SpellsCastThisTurnMatching {
            player: spell_caster,
            filter,
            exclude_source,
        } => {
            let base = describe_for_each_filter(filter);
            let prefix = if *exclude_source { "other " } else { "" };
            let tail = match spell_caster {
                PlayerFilter::You => "you've cast this turn".to_string(),
                PlayerFilter::Opponent => "an opponent has cast this turn".to_string(),
                PlayerFilter::Any => "cast this turn".to_string(),
                other => format!(
                    "cast this turn by {}",
                    strip_leading_article(&describe_player_filter(other))
                ),
            };
            Some(format!(
                "{player} {verb} a card for each {prefix}{base} {tail}"
            ))
        }
        Value::PlayerCounters(counter_player, counter_type) => Some(format!(
            "{player} {verb} a card for each {} counter {}",
            describe_counter_type(*counter_type),
            describe_player_counter_holder(counter_player)
        )),
        Value::CountersOnSource(counter_type) => Some(format!(
            "{player} {verb} a card for each {} counter on this permanent",
            describe_counter_type(*counter_type)
        )),
        Value::CountersOn(spec, Some(counter_type)) => Some(format!(
            "{player} {verb} a card for each {} counter on {}",
            describe_counter_type(*counter_type),
            describe_choose_spec(spec)
        )),
        Value::CountersOn(spec, None) => Some(format!(
            "{player} {verb} a card for each counter on {}",
            describe_choose_spec(spec)
        )),
        Value::BasicLandTypesAmong(filter) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_basic_land_types_among(filter)
        )),
        Value::CreatureTypesAmong(filter) => Some(format!(
            "{player} {verb} a card for each creature type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::CardTypesAmong(filter) => Some(format!(
            "{player} {verb} a card for each card type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::ColorsAmong(filter) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_colors_among(filter)
        )),
        _ => None,
    }
}

/// Render an authored `cards equal to ...` count for actions such as draw and
/// mill. Other card-count effects use an object noun phrase (for example,
/// "discard a number of cards equal to ..."), but draw and mill take the
/// shorter oracle action surface without the indefinite quantifier.
pub(crate) fn describe_equal_to_card_action_count(value: &Value) -> Option<String> {
    if !value.has_surface_hint(ValueSurfaceHint::EqualTo)
        || describe_effect_count_backref(value).is_some()
    {
        return None;
    }

    let amount = value
        .clone()
        .without_surface_hint(ValueSurfaceHint::EqualTo);
    let basis = match amount.unhinted() {
        Value::PowerOf(spec) => describe_power_card_count_basis(spec),
        _ => describe_value(&amount),
    };
    let count = format!("cards equal to {basis}");
    Some(
        if value.has_surface_hint(ValueSurfaceHint::AdditionalCards) {
            additionalize_card_count_phrase(&count)
        } else {
            count
        },
    )
}

pub(super) fn singularize_for_each_basis(basis: &str) -> String {
    if let Some((head, tail)) = basis.split_once(" counters on ") {
        return format!("{head} counter on {tail}");
    }
    basis.to_string()
}

pub(super) fn describe_tagged_creature_power_count_basis(
    spec: &ChooseSpec,
) -> Option<&'static str> {
    match spec.base() {
        ChooseSpec::Tagged(tag)
            if tag.as_str() == "__it__" || tag.as_str().starts_with("sacrifice_cost_") =>
        {
            Some("that creature's power")
        }
        _ => None,
    }
}

pub(super) fn describe_power_card_count_basis(spec: &ChooseSpec) -> String {
    match spec.base() {
        ChooseSpec::Tagged(tag)
            if tag.as_str() == "__it__"
                || tag.as_str().starts_with("sacrifice_cost_")
                || tag_action_from_name(tag.as_str()) == Some("sacrificed") =>
        {
            "that creature's power".to_string()
        }
        ChooseSpec::Source => "its power".to_string(),
        _ => format!("{} power", describe_possessive_choose_spec(spec)),
    }
}

pub(super) fn describe_dynamic_counter_amount_phrase(
    value: &Value,
    counter_type: CounterType,
    target: &str,
) -> Option<(String, String)> {
    let counter_name = describe_counter_type(counter_type);
    let amount_text = format!("X {counter_name} counters");
    let attribute = match value {
        Value::SourcePower | Value::PowerOf(_) => "power",
        Value::SourceToughness | Value::ToughnessOf(_) => "toughness",
        Value::ManaValueOf(_) => "mana value",
        _ => return None,
    };
    let basis = match value {
        Value::SourcePower => "its power".to_string(),
        Value::SourceToughness => "its toughness".to_string(),
        Value::PowerOf(spec) => describe_dynamic_counter_basis(spec, "power"),
        Value::ToughnessOf(spec) => describe_dynamic_counter_basis(spec, "toughness"),
        Value::ManaValueOf(spec) => describe_dynamic_counter_basis(spec, "mana value"),
        _ => return None,
    };
    let basis = if target.contains("target creature")
        && basis == format!("target permanent's {attribute}")
    {
        format!("that creature's {attribute}")
    } else {
        basis
    };
    Some((amount_text, basis))
}

pub(super) fn describe_dynamic_counter_basis(spec: &ChooseSpec, attribute: &str) -> String {
    if let Some(kind) = spec.sacrificed_object_kind() {
        return format!("the sacrificed {}'s {attribute}", kind.noun());
    }
    if let Some(surface) = spec.source_reference_surface()
        && matches!(spec.unhinted(), ChooseSpec::Source)
    {
        return format!("{}'s {attribute}", surface.display_text());
    }
    if spec.is_target() {
        return format!("that creature's {attribute}");
    }
    match spec.base() {
        ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG => {
            format!("the exiled card's {attribute}")
        }
        ChooseSpec::Tagged(tag) if tag.as_str() == crate::effects::PUBLIC_REVEALED_TAG => {
            format!("the revealed card's {attribute}")
        }
        ChooseSpec::Tagged(tag)
            if tag.as_str().starts_with("revealed_")
                || crate::cards::is_sentence_helper_tag(tag.as_str(), "revealed") =>
        {
            format!("the revealed card's {attribute}")
        }
        ChooseSpec::Tagged(tag) if tag_action_from_name(tag.as_str()) == Some("sacrificed") => {
            format!("the sacrificed creature's {attribute}")
        }
        ChooseSpec::Tagged(tag)
            if tag.as_str() == "discarded_cost" || tag.as_str().starts_with("discard_cost_") =>
        {
            format!("the discarded card's {attribute}")
        }
        ChooseSpec::Tagged(tag) if tag.as_str().starts_with("exile_cost_") => {
            format!("the exiled card's {attribute}")
        }
        ChooseSpec::Tagged(tag)
            if attribute == "mana value" && tag.as_str().starts_with("countered_") =>
        {
            "that spell's mana value".to_string()
        }
        ChooseSpec::Tagged(tag)
            if tag.as_str().starts_with("exiled_")
                || crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled") =>
        {
            format!("that card's {attribute}")
        }
        ChooseSpec::Tagged(_) | ChooseSpec::Target(_) => {
            format!("that creature's {attribute}")
        }
        ChooseSpec::Source => format!("its {attribute}"),
        _ => format!("{} {attribute}", describe_possessive_choose_spec(spec)),
    }
}

pub(crate) fn describe_create_for_each_count(value: &Value) -> Option<String> {
    if value.has_surface_hint(ValueSurfaceHint::EqualTo) || value_prefers_where_x(value) {
        return None;
    }
    if let Some(history) = describe_turn_history_for_each_basis(value) {
        return Some(history);
    }
    if let Some((1, party)) = describe_party_size_for_each_basis(value) {
        return Some(party);
    }
    if value.has_surface_hint(ValueSurfaceHint::CardsDrawnThisWay) {
        return Some("card drawn this way".to_string());
    }
    if value.has_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay) {
        return Some("card discarded this way".to_string());
    }
    // Let typed prior-effect metrics render their own filter below.  A broad
    // "cards revealed this way" hint is useful for an unfiltered reveal, but
    // it must not erase distinctions such as "nonland card revealed this
    // way".
    if value.has_surface_hint(ValueSurfaceHint::CardsPutIntoYourGraveyardThisWay) {
        return Some("creature card put into your graveyard this way".to_string());
    }
    if value.has_surface_hint(ValueSurfaceHint::CardsLookedAtWhileScryingThisWay) {
        return Some("card looked at while scrying this way".to_string());
    }
    if value.has_surface_hint(ValueSurfaceHint::CreaturesBlockingIt) {
        return match value.unhinted() {
            Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier: 1 }) => {
                Some("creature blocking it beyond the first".to_string())
            }
            Value::EventValueOffset(EventValueSpec::BlockersBeyondFirst { multiplier: 1 }, 1) => {
                Some("creature blocking it".to_string())
            }
            _ => None,
        };
    }
    if value.has_surface_hint(ValueSurfaceHint::CreaturesChosenBeforeIt) {
        return Some("creature chosen before it".to_string());
    }
    match value.unhinted() {
        Value::Count(filter) => Some(
            describe_prior_effect_source_count_basis(filter, false)
                .unwrap_or_else(|| describe_for_each_count_filter(filter)),
        ),
        Value::ManaSymbolsInManaCostOf { spec, color } => {
            let ChooseSpec::All(filter) = spec.unhinted() else {
                return None;
            };
            Some(format!(
                "{} mana symbol in the mana costs of {}",
                color.name(),
                describe_count_filter_value_subject(filter)
            ))
        }
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query)
            if query.metric == crate::effect::EffectMetric::Count =>
        {
            Some(describe_prior_effect_metric_basis(query, false))
        }
        Value::BasicLandTypesAmong(filter) => Some(
            describe_basic_land_types_among(filter).replace("basic land types", "basic land type"),
        ),
        Value::CreatureTypesAmong(filter) => Some(format!(
            "creature type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::CardTypesAmong(filter) => Some(format!(
            "card type among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::ColorsAmong(filter) => Some(describe_colors_among(filter)),
        Value::ColorsOfManaSpentToCastThisSpell => {
            Some("color of mana spent to cast this spell".to_string())
        }
        Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
        } => {
            let mut source = source_filter.description();
            if *include_source_noun {
                source.push_str(" source");
            }
            Some(format!(
                "mana from {source} that was spent to cast this spell"
            ))
        }
        Value::CreaturesDiedThisTurn => Some("creature that died this turn".to_string()),
        Value::CreaturesDiedThisTurnControlledBy(controller) => {
            let controller = match controller {
                PlayerFilter::You => "you controlled".to_string(),
                PlayerFilter::Opponent => "an opponent controlled".to_string(),
                PlayerFilter::Any => return Some("creature that died this turn".to_string()),
                other => format!("{} controlled", describe_player_filter(other)),
            };
            Some(format!("creature {controller} that died this turn"))
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Died {
            filter,
            controller_surface,
        }) => {
            let mut subject_filter = filter.clone();
            let controller = subject_filter.controller.take();
            subject_filter.zone = None;
            let subject = describe_for_each_filter(&subject_filter);
            Some(describe_death_history_subject(
                &subject,
                controller.as_ref(),
                *controller_surface,
            ))
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::EnteredBattlefield(filter)) => {
            let mut subject_filter = filter.clone();
            let controller = subject_filter.controller.take();
            subject_filter.zone = None;
            let subject = describe_for_each_filter(&subject_filter);
            Some(match controller {
                Some(controller) => format!(
                    "{subject} that entered the battlefield under {} control this turn",
                    describe_possessive_player_filter(&controller)
                ),
                None => format!("{subject} that entered the battlefield this turn"),
            })
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersDiscarded(player)) => {
            let player =
                strip_leading_article(&describe_for_each_player_filter(player)).to_string();
            Some(format!("{player} who discarded a card this turn"))
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersDealtDamage(player)) => {
            let player =
                strip_leading_article(&describe_for_each_player_filter(player)).to_string();
            Some(format!("{player} who was dealt damage this turn"))
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersDealtCombatDamageBy {
            players,
            sources,
        }) => {
            let player =
                strip_leading_article(&describe_for_each_player_filter(players)).to_string();
            if sources == &ObjectFilter::default() {
                Some(format!("{player} who was dealt combat damage this turn"))
            } else {
                Some(format!(
                    "{player} who was dealt combat damage by {} this turn",
                    describe_for_each_filter(sources)
                ))
            }
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersLostLife(player)) => {
            let player =
                strip_leading_article(&describe_for_each_player_filter(player)).to_string();
            Some(format!("{player} who lost life this turn"))
        }
        Value::KickCount => Some("time it was kicked".to_string()),
        Value::SpellsCastThisTurn(player) => Some(describe_spells_cast_this_turn_each(player)),
        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let described = describe_for_each_filter(filter);
            let prefix = if *exclude_source && !described.starts_with("other ") {
                "other "
            } else {
                ""
            };
            let tail = match player {
                PlayerFilter::You => "you've cast this turn".to_string(),
                PlayerFilter::Opponent => "an opponent has cast this turn".to_string(),
                PlayerFilter::Any => "cast this turn".to_string(),
                other => format!("cast this turn by {}", describe_player_filter(other)),
            };
            Some(format!("{prefix}{described} {tail}"))
        }
        Value::SourceRegeneratedThisTurnCount => Some("time it regenerated this turn".to_string()),
        Value::Add(inner, offset)
            if matches!(offset.unhinted(), Value::Fixed(-1))
                && matches!(inner.unhinted(), Value::SpellsCastThisTurn(_)) =>
        {
            let Value::SpellsCastThisTurn(player) = inner.unhinted() else {
                unreachable!();
            };
            Some(describe_for_each_spells_cast_this_turn(player, true))
        }
        Value::PlayerCounters(player, counter_type) => Some(format!(
            "{} counter {}",
            describe_counter_type(*counter_type),
            describe_player_counter_holder(player)
        )),
        Value::CountersOnSource(counter_type) => Some(format!(
            "{} counter on it",
            describe_counter_type(*counter_type)
        )),
        Value::CountersOn(spec, Some(counter_type)) => {
            let objects = match spec.unhinted() {
                ChooseSpec::All(filter) => {
                    pluralize_noun_phrase(strip_indefinite_article(&filter.description()))
                }
                _ => describe_choose_spec(spec),
            };
            Some(format!(
                "{} counter on {objects}",
                describe_counter_type(*counter_type),
            ))
        }
        Value::CountersOn(spec, None) => Some(format!("counter on {}", describe_choose_spec(spec))),
        _ => None,
    }
}

fn factor_positive_for_each_value(value: Value) -> Option<(i32, Value)> {
    match value {
        Value::SurfaceHinted { value, hints } => {
            let (multiplier, basis) = factor_positive_for_each_value(*value)?;
            Some((multiplier, basis.with_surface_hints(hints)))
        }
        Value::Scaled(value, multiplier) if multiplier > 0 => {
            let (inner_multiplier, basis) = factor_positive_for_each_value(*value)?;
            Some((inner_multiplier.checked_mul(multiplier)?, basis))
        }
        Value::CountScaled(filter, multiplier) if multiplier > 0 => {
            Some((multiplier, Value::Count(filter)))
        }
        Value::Add(left, right) if left == right => {
            let (multiplier, basis) = factor_positive_for_each_value(*left)?;
            Some((multiplier.checked_mul(2)?, basis))
        }
        basis => Some((1, basis)),
    }
}

/// Decompose a typed authored `for each` value into the number applied per
/// qualifying object/event and its singular basis. This keeps the multiplier
/// structural (`Scaled`, `CountScaled`, or repeated equal addends) instead of
/// inferring it from rendered "twice the number of ..." text.
pub(crate) fn describe_for_each_multiplier_and_basis(value: &Value) -> Option<(i32, String)> {
    if !value.has_surface_hint(ValueSurfaceHint::ForEach)
        || value.has_surface_hint(ValueSurfaceHint::EqualTo)
        || value_prefers_where_x(value)
    {
        return None;
    }
    let value = value
        .clone()
        .without_surface_hint(ValueSurfaceHint::ForEach);
    let (multiplier, basis) = factor_positive_for_each_value(value)?;
    let basis = basis.with_surface_hint(ValueSurfaceHint::ForEach);
    Some((multiplier, describe_create_for_each_count(&basis)?))
}

pub(super) fn is_graveyard_same_stable_tagged_spec(spec: &ChooseSpec) -> bool {
    let ChooseSpec::All(filter) = spec else {
        return false;
    };
    filter.zone == Some(Zone::Graveyard)
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameStableId
        })
}

pub(crate) fn value_is_iterated_object_count(value: &Value) -> bool {
    let Value::Count(filter) = value.unhinted() else {
        return false;
    };
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == "__it__"
    })
}

pub(crate) fn pluralize_token_phrase(token_phrase: &str) -> String {
    if let Some((head, tail)) = token_phrase.split_once(" token. It has ") {
        return format!("{head} tokens. They have {tail}");
    }
    if let Some((head, tail)) = token_phrase.split_once(" token. It gains ") {
        return format!("{head} tokens. They gain {tail}");
    }
    if let Some((head, tail)) = token_phrase.split_once(" token with ") {
        return format!("{head} tokens with {tail}");
    }
    if let Some((head, tail)) = token_phrase.split_once(" token named ") {
        return format!("{head} tokens named {tail}");
    }
    if let Some((head, tail)) = token_phrase.split_once(" token of ") {
        return format!("{head} tokens of {tail}");
    }
    if let Some(head) = token_phrase.strip_suffix(" token") {
        return format!("{head} tokens");
    }
    format!("{token_phrase}s")
}

pub(super) fn split_token_ability_sentence(token_phrase: &str) -> (&str, Option<String>) {
    if let Some((head, tail)) = token_phrase.split_once(". It has ") {
        return (head, Some(format!(". It has {tail}")));
    }
    if let Some((head, tail)) = token_phrase.split_once(". They have ") {
        return (head, Some(format!(". They have {tail}")));
    }
    if let Some((head, tail)) = token_phrase.split_once(". It gains ") {
        return (head, Some(format!(". It gains {tail}")));
    }
    if let Some((head, tail)) = token_phrase.split_once(". They gain ") {
        return (head, Some(format!(". They gain {tail}")));
    }
    (token_phrase, None)
}

pub(super) fn normalize_dynamic_equal_pt_token_phrase(
    token_main: &str,
) -> Option<(String, String)> {
    let (left, right) = token_main.split_once('/')?;
    let left = left.trim();
    let right = right.trim_start();
    if left.is_empty()
        || left.chars().all(|ch| ch.is_ascii_digit())
        || matches!(left, "*" | "X" | "x")
    {
        return None;
    }
    let rest = right.strip_prefix(left)?.trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((format!("X/X {rest}"), left.to_string()))
}

pub(super) fn normalize_dynamic_equal_pt_create_text(text: &str) -> Option<String> {
    if let Some((subject, token_main)) = text.split_once(" creates ") {
        let (token_main, where_x) = normalize_dynamic_equal_pt_token_phrase(token_main)?;
        return Some(format!(
            "{subject} creates {}, where X is {where_x}",
            with_indefinite_article(&token_main)
        ));
    }
    if let Some(token_main) = text.strip_prefix("Create ") {
        let (token_main, where_x) = normalize_dynamic_equal_pt_token_phrase(token_main)?;
        return Some(format!(
            "Create {}, where X is {where_x}",
            with_indefinite_article(&token_main)
        ));
    }
    if let Some(token_main) = text.strip_prefix("create ") {
        let (token_main, where_x) = normalize_dynamic_equal_pt_token_phrase(token_main)?;
        return Some(format!(
            "create {}, where X is {where_x}",
            with_indefinite_article(&token_main)
        ));
    }
    None
}

pub(super) fn singular_token_phrase_with_article(token_main: &str) -> String {
    let trimmed = token_main.trim();
    if trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("a {trimmed}")
    } else {
        with_indefinite_article(trimmed)
    }
}

pub(super) fn append_token_ability_sentence(
    mut text: String,
    ability_sentence: Option<String>,
) -> String {
    if let Some(sentence) = ability_sentence {
        text.push_str(&sentence);
    }
    text
}

pub(super) fn append_token_where_x_continuation(mut text: String, basis: &str) -> String {
    if let Some(before_quote) = text.strip_suffix('"') {
        let before_quote = before_quote.trim_end_matches(['.', ',', '!', '?']);
        return format!("{before_quote},\" where X is {basis}");
    }
    text.push_str(", where X is ");
    text.push_str(basis);
    text
}

pub(super) fn describe_token_creator_subject(controller: &PlayerFilter) -> Option<String> {
    match controller {
        PlayerFilter::You => None,
        PlayerFilter::DamagedPlayer => Some("that player".to_string()),
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Target) => {
            Some("its controller".to_string())
        }
        other => Some(describe_player_filter(other)),
    }
}

pub(super) fn describe_create_token_action(
    object_text: &str,
    controller: &PlayerFilter,
    actor_surface_explicit: bool,
) -> String {
    if actor_surface_explicit && matches!(controller, PlayerFilter::You) {
        return format!("You create {object_text}");
    }
    if let Some(subject) = describe_token_creator_subject(controller) {
        format!("{} creates {object_text}", capitalize_first(&subject))
    } else {
        format!("Create {object_text}")
    }
}

/// Restore an authored post-create token definition when lowering represents
/// its intrinsic end-step sacrifice as cleanup on the same creation effect.
/// The cleanup flag schedules the exact created object, while the separate
/// ability presentation proves that the source used `It has ...` rather than
/// an independent delayed instruction.
pub(super) fn describe_token_definition_with_end_step_sacrifice(
    create: &crate::effects::CreateTokenEffect,
) -> Option<String> {
    if create.count != Value::Fixed(1)
        || !matches!(
            create.ability_presentation,
            Some(
                ironsmith_core::TokenAbilityPresentation::SeparateSentence
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined
            )
        )
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || !create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
        || create.next_end_step_player != PlayerFilter::Any
    {
        return None;
    }

    let mut ability_texts = Vec::new();
    for ability in &create.token.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        if !static_ability.is_keyword() {
            return None;
        }
        let keyword = static_ability.display().to_ascii_lowercase();
        if !ability_texts.contains(&keyword) {
            ability_texts.push(keyword);
        }
    }
    ability_texts.push("\"At the beginning of the end step, sacrifice this token.\"".to_string());

    let mut creation_only = create.clone();
    creation_only.sacrifice_at_next_end_step = false;
    let rendered_creation = describe_effect(&Effect::new(creation_only));
    let creation = rendered_creation
        .split_once(". It has ")
        .map_or(rendered_creation.as_str(), |(creation, _)| creation)
        .trim()
        .trim_end_matches('.');
    if creation.is_empty() {
        return None;
    }

    Some(format!(
        "{creation}. It has {}",
        join_with_and(&ability_texts)
    ))
}

pub(crate) fn should_render_token_count_with_where_x(value: &Value) -> bool {
    if value_has_surface_hint(value, ValueSurfaceHint::ForEach)
        || value_has_surface_hint(value, ValueSurfaceHint::EqualTo)
    {
        return false;
    }
    if matches!(
        value.unhinted(),
        Value::Fixed(_)
            | Value::X
            | Value::XTimes(_)
            | Value::EffectValue(_)
            | Value::EffectValueOffset(_, _)
            | Value::EventValue(_)
            | Value::EventValueOffset(_, _)
            | Value::EffectMetric { .. }
            | Value::EffectMetricOffset { .. }
            | Value::PendingEffectMetric { .. }
            | Value::PendingEffectMetricOffset { .. }
            | Value::WasKicked
            | Value::WasBoughtBack
            | Value::WasEntwined
            | Value::WasPaid(_)
            | Value::WasPaidLabel(_)
            | Value::TimesPaid(_)
            | Value::TimesPaidLabel(_)
            | Value::KickCount
            | Value::SourceRegeneratedThisTurnCount
            | Value::MagicGamesLostToOpponentsSinceLastWin
    ) {
        return false;
    }

    let rendered = describe_value(value);
    rendered.chars().any(char::is_whitespace) || rendered.contains('\'')
}

pub(crate) fn describe_compact_token_count(value: &Value, token_name: &str) -> String {
    if value.has_surface_hint(ValueSurfaceHint::DamageDealt) {
        return format!("a number of {token_name} tokens equal to the damage dealt this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::EqualTo) {
        let amount = value
            .clone()
            .without_surface_hint(ValueSurfaceHint::EqualTo);
        return format!(
            "a number of {token_name} tokens equal to {}",
            describe_value(&amount)
        );
    }
    if let Some(amount) = describe_effect_count_backref(value) {
        return format!("{amount} {token_name} tokens");
    }
    match value.unhinted() {
        Value::Fixed(1) => format!("a {token_name} token"),
        Value::Fixed(n) => format!("{n} {token_name} tokens"),
        Value::X => format!("X {token_name} tokens"),
        Value::EffectMetric {
            metric: crate::effect::EffectMetric::OtherNumber,
            ..
        }
        | Value::PendingEffectMetric {
            metric: crate::effect::EffectMetric::OtherNumber,
            ..
        } => format!("a number of {token_name} tokens equal to the other result"),
        Value::Count(filter) => {
            format!(
                "a {token_name} token for each {}",
                describe_for_each_count_filter(filter)
            )
        }
        Value::CountScaled(filter, multiplier) => {
            if *multiplier == 1 {
                format!(
                    "a {token_name} token for each {}",
                    describe_for_each_count_filter(filter)
                )
            } else {
                format!(
                    "{multiplier} {token_name} tokens for each {}",
                    describe_for_each_count_filter(filter)
                )
            }
        }
        Value::BasicLandTypesAmong(filter) => {
            let lands = describe_for_each_filter(filter);
            let lands = if lands == "land" {
                "lands".to_string()
            } else if let Some(rest) = lands.strip_prefix("land ") {
                format!("lands {rest}")
            } else {
                lands
            };
            format!("a {token_name} token for each basic land type among {lands}")
        }
        Value::CardTypesAmong(filter) => {
            format!(
                "a {token_name} token for each card type among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::ColorsAmong(filter) => {
            format!(
                "a {token_name} token for each color among {}",
                describe_for_each_filter(filter)
            )
        }
        Value::CreaturesDiedThisTurn => {
            format!("a {token_name} token for each creature that died this turn")
        }
        Value::CreaturesDiedThisTurnControlledBy(filter) => {
            let suffix = match filter {
                PlayerFilter::You => "under your control this turn".to_string(),
                PlayerFilter::Opponent => "under an opponent's control this turn".to_string(),
                PlayerFilter::Any => "this turn".to_string(),
                other => format!(
                    "under {} control this turn",
                    describe_possessive_player_filter(other)
                ),
            };
            format!("a {token_name} token for each creature that died {suffix}")
        }
        Value::ColorsOfManaSpentToCastThisSpell => {
            format!("a {token_name} token for each color of mana spent to cast this spell")
        }
        Value::SourceRegeneratedThisTurnCount => {
            format!("a {token_name} token for each time it regenerated this turn")
        }
        _ => format!("{} {token_name} token(s)", describe_value(value)),
    }
}

pub(crate) fn describe_compact_create_token(
    create_token: &crate::effects::CreateTokenEffect,
) -> Option<String> {
    if create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
    {
        return None;
    }

    let token_name = create_token.token.name();
    let is_compact_named_token = matches!(
        token_name,
        "Treasure" | "Clue" | "Food" | "Blood" | "Gold" | "Powerstone" | "Junk" | "Mutagen"
    );
    if !is_compact_named_token {
        return None;
    }

    if value_prefers_where_x(&create_token.count)
        && !create_token
            .count
            .has_surface_hint(ValueSurfaceHint::DamageDealt)
    {
        let mut amount = format!("X {token_name} tokens");
        let state = if create_token.enters_tapped && create_token.enters_attacking {
            Some("tapped and attacking")
        } else if create_token.enters_tapped {
            Some("tapped")
        } else if create_token.enters_attacking {
            Some("attacking")
        } else {
            None
        };
        if let Some(state) = state {
            amount = amount.replacen(token_name, &format!("{state} {token_name}"), 1);
        }
        let mut text = describe_create_token_action(
            &amount,
            &create_token.controller,
            create_token.actor_surface_explicit,
        );
        text.push_str(&format!(
            ", where X is {}",
            describe_value(&create_token.count)
        ));
        return Some(text);
    }

    let mut amount = describe_compact_token_count(&create_token.count, token_name);
    let state = if create_token.enters_tapped && create_token.enters_attacking {
        Some("tapped and attacking")
    } else if create_token.enters_tapped {
        Some("tapped")
    } else if create_token.enters_attacking {
        Some("attacking")
    } else {
        None
    };
    if let Some(state) = state {
        amount = amount.replacen(token_name, &format!("{state} {token_name}"), 1);
    }

    Some(describe_create_token_action(
        &amount,
        &create_token.controller,
        create_token.actor_surface_explicit,
    ))
}

pub(crate) fn describe_create_token_and_manifest_top_card(
    create: &crate::effects::CreateTokenEffect,
    manifest: &crate::effects::ManifestTopCardOfLibraryEffect,
) -> Option<String> {
    if manifest.cloak {
        return None;
    }
    let create_text = describe_compact_create_token(create)?;
    let owner = match manifest.player {
        crate::filter::PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player's".to_string()
        }
        _ => describe_possessive_player_filter(&manifest.player),
    };
    Some(format!(
        "{create_text} and manifest the top card of {owner} library"
    ))
}

pub(crate) fn choose_exact_count(choose: &crate::effects::ChooseObjectsEffect) -> Option<usize> {
    choose.count.max.filter(|max| *max == choose.count.min)
}

pub(super) fn describe_runtime_choice_count(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if !choose.count.dynamic_x || choose.count_value.is_none() {
        return None;
    }
    if let Some(count_value) = choose.count_value.as_ref() {
        if (choose.count.up_to_x || choose.search_mode == SearchSelectionMode::Optional)
            && count_value.has_surface_hint(ValueSurfaceHint::Difference)
        {
            return Some("a number of".to_string());
        }
        if is_effect_count_reference(count_value, None) {
            return Some(
                if choose.count.up_to_x || choose.search_mode == SearchSelectionMode::Optional {
                    "up to that many".to_string()
                } else {
                    "that many".to_string()
                },
            );
        }
    }
    Some(
        if choose.count.up_to_x || choose.search_mode == SearchSelectionMode::Optional {
            "up to X"
        } else {
            "X"
        }
        .to_string(),
    )
}

pub(super) fn describe_runtime_choice_where_clause(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let count_value = choose.count_value.as_ref()?;
    if !choose.count.dynamic_x {
        return None;
    }
    if (choose.count.up_to_x || choose.search_mode == SearchSelectionMode::Optional)
        && count_value.has_surface_hint(ValueSurfaceHint::Difference)
    {
        return Some(" less than or equal to the difference".to_string());
    }
    if is_effect_count_reference(count_value, None) {
        return None;
    }
    Some(format!(", where X is {}", describe_value(count_value)))
}

fn contextualize_choice_count_basis(basis: String, chooser: &PlayerFilter) -> String {
    if matches!(chooser, PlayerFilter::IteratedPlayer | PlayerFilter::Active) {
        basis
            .replace("that player's", "their")
            .replace("that player", "they")
    } else {
        basis
    }
}

pub(super) fn revealed_keyword_choice_label(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<&'static str> {
    if choose.is_search
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose.filter.static_abilities.len() != 1
    {
        return None;
    }
    let zones = choose_search_zones(choose)?;
    if ![
        Zone::Battlefield,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Library,
        Zone::Exile,
    ]
    .iter()
    .all(|zone| zones.contains(zone))
    {
        return None;
    }
    let references_revealed_cards = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint
                .tag
                .as_str()
                .starts_with("__sentence_helper_revealed")
    });
    references_revealed_cards
        .then(|| keyword_label_from_static_ability_id(choose.filter.static_abilities[0]))
        .flatten()
}

pub(super) fn describe_revealed_keyword_choice_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let label = revealed_keyword_choice_label(choose)?;
    Some(format!("up to one other cards with {label}"))
}

pub(crate) fn describe_choose_selection(choose: &crate::effects::ChooseObjectsEffect) -> String {
    if choose
        .count_value
        .as_ref()
        .is_some_and(|value| value.has_surface_hint(ValueSurfaceHint::ChooseAllInOrder))
    {
        let singular = describe_for_each_count_filter(&choose.filter);
        let plural = pluralize_noun_phrase(&singular);
        return format!("{plural} one at a time until each {singular} has been chosen");
    }
    if choose.top_only {
        if let Some(exact) = choose_exact_count(choose) {
            if exact > 1 {
                let count_text = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
                return format!("the top {count_text} cards");
            }
        }
        let mut ordinary_choice = choose.clone();
        ordinary_choice.top_only = false;
        let ordinary_selection = describe_choose_selection(&ordinary_choice);
        let noun = ordinary_selection
            .strip_prefix("a ")
            .or_else(|| ordinary_selection.strip_prefix("an "))
            .unwrap_or(ordinary_selection.as_str());
        return format!("the top {noun}");
    }

    // Source-object choices are an implementation detail used to make costs
    // executable. Oracle refers to that object demonstratively ("this"), not
    // as an ordinary filtered selection such as "a this you control".
    if choose.count.is_single() && choose.filter.source {
        return "this".to_string();
    }
    if choose.count.is_single() && choose.filter.has_one_of_tagged_set_surface() {
        return "one of them".to_string();
    }

    if let Some(selection) = describe_source_exiled_choose_selection(choose) {
        return selection;
    }
    if let Some(selection) = describe_revealed_keyword_choice_selection(choose) {
        return selection;
    }

    let described_filter = choose.filter.clone();
    let has_extremum = [
        &described_filter.power,
        &described_filter.toughness,
        &described_filter.mana_value,
    ]
    .into_iter()
    .flatten()
    .any(|comparison| {
        let crate::filter::Comparison::EqualExpr(value) = comparison else {
            return false;
        };
        matches!(
            value.unhinted(),
            Value::GreatestPower(_)
                | Value::GreatestToughness(_)
                | Value::GreatestManaValue(_)
                | Value::LeastPower(_)
                | Value::LeastToughness(_)
                | Value::LeastManaValue(_)
        )
    });
    let filter_text = described_filter
        .zone
        .or(choose.zone)
        .filter(|zone| is_nonbattlefield_card_zone(*zone))
        .map_or_else(
            || described_filter.description(),
            |zone| describe_nonbattlefield_card_filter_without_zone(&described_filter, zone),
        );
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    if has_extremum {
        card_desc = card_desc.replace("that player controls", "they control");
    }
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    if let Some(rest) = card_desc.strip_suffix(" hands") {
        card_desc = format!("{rest} hand");
    }
    if let Some(action) = describe_tagged_this_way_action(&choose.filter)
        && !card_desc.ends_with("this way")
    {
        card_desc.push_str(&format!(" {action} this way"));
    }
    let where_x_suffix = apply_mana_value_where_x_surface(&mut card_desc, &choose.filter);
    if card_desc == "with different names" {
        card_desc = "card with different names".to_string();
    } else if card_desc == "with different powers" {
        card_desc = "card with different powers".to_string();
    } else if let Some(base) = card_desc.strip_prefix("with different names ") {
        card_desc = format!("{base} with different names");
    } else if let Some(base) = card_desc.strip_prefix("with different powers ") {
        card_desc = format!("{base} with different powers");
    } else if let Some(rest) = card_desc.strip_prefix("card ")
        && !rest.starts_with("with ")
        && !rest.starts_with("without ")
        && !rest.starts_with("that ")
    {
        card_desc = format!("{rest} card");
    }
    let distinct_names_suffix = if choose.filter.distinct_names {
        if let Some(base) = card_desc.strip_suffix(" with different names") {
            card_desc = base.to_string();
            true
        } else {
            false
        }
    } else {
        false
    };
    let distinct_powers_suffix = if choose.filter.distinct_powers {
        if let Some(base) = card_desc.strip_suffix(" with different powers") {
            card_desc = base.to_string();
            true
        } else {
            false
        }
    } else {
        false
    };
    let describe_plural_selection = |count_prefix: String, card_desc: &str| -> String {
        let mut selection = format!("{count_prefix} {}", pluralize_noun_phrase(card_desc));
        if distinct_names_suffix {
            selection.push_str(" with different names");
        }
        if distinct_powers_suffix {
            selection.push_str(" with different powers");
        }
        selection
    };

    if choose.count.is_dynamic_x()
        && !choose.count.is_up_to_dynamic_x()
        && !choose.count.is_random()
        && let Some(count_value) = choose.count_value.as_ref()
        && count_value.has_surface_hint(ValueSurfaceHint::ForEach)
        && let Some(basis) = describe_create_for_each_count(count_value)
    {
        let basis = contextualize_choice_count_basis(basis, &choose.chooser);
        let mut selection = with_indefinite_article(&card_desc);
        if distinct_names_suffix {
            selection.push_str(" with a different name");
        }
        if distinct_powers_suffix {
            selection.push_str(" with a different power");
        }
        selection.push_str(&format!(" for each {basis}"));
        selection.push_str(&where_x_suffix);
        return selection;
    }
    if choose.count.is_single() {
        let mut selection = with_indefinite_article(&card_desc);
        if choose.count.random {
            selection.push_str(" at random");
        }
        selection.push_str(&where_x_suffix);
        return selection;
    }
    if let Some(runtime_count) = describe_runtime_choice_count(choose) {
        let mut selection = describe_plural_selection(runtime_count, &card_desc);
        selection.push_str(&describe_runtime_choice_where_clause(choose).unwrap_or_default());
        selection.push_str(&where_x_suffix);
        return selection;
    }
    if let Some(exact) = choose_exact_count(choose) {
        let count_text = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        let count_text = if choose.count.explicit_exactly {
            format!("exactly {count_text}")
        } else {
            count_text
        };
        let mut selection = describe_plural_selection(count_text, &card_desc);
        selection.push_str(&where_x_suffix);
        return selection;
    }
    // A max-one choice keeps its singular noun ("up to one creature").
    let mut selection = if choose.count.max == Some(1) {
        format!("{} {}", describe_choice_count(&choose.count), card_desc)
    } else {
        let count_prefix = if choose.count.is_any_number() {
            format!("{} of", describe_choice_count(&choose.count))
        } else {
            describe_choice_count(&choose.count)
        };
        describe_plural_selection(count_prefix, &card_desc)
    };
    if let Some(constraint) = &choose.aggregate_constraint {
        let metric = match constraint.metric {
            crate::effect::ChoiceAggregateMetric::Power => "power",
            crate::effect::ChoiceAggregateMetric::Toughness => "toughness",
            crate::effect::ChoiceAggregateMetric::ManaValue => "mana value",
        };
        let maximum = describe_value(&constraint.maximum);
        if matches!(constraint.maximum.unhinted(), Value::Fixed(_)) {
            selection.push_str(&format!(" with total {metric} {maximum} or less"));
        } else {
            selection.push_str(&format!(
                " with total {metric} less than or equal to {maximum}"
            ));
        }
    }
    selection.push_str(&where_x_suffix);
    selection
}

pub(crate) fn describe_for_players_choose_then_untap_chosen(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [choose_effect, untap_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || !untap_target_exactly_matches_choice(untap, choose)
    {
        return None;
    }

    let subject = describe_for_players_subject(&for_players.filter)?;
    let chosen = describe_choose_selection(choose);
    let chosen_noun = pluralize_noun_phrase(choose_reference_noun(choose));
    let (choose_verb, untap_verb) = if subject == "You" {
        ("choose", "untap")
    } else {
        ("chooses", "untaps")
    };
    Some(format!(
        "{subject} {choose_verb} {chosen}, then {untap_verb} those {chosen_noun}"
    ))
}

pub(crate) fn untap_target_exactly_matches_choice(
    untap: &crate::effects::UntapEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    match &untap.target {
        ChooseSpec::Tagged(tag) => tag == &choose.tag,
        ChooseSpec::All(filter) => {
            let [constraint] = filter.tagged_constraints.as_slice() else {
                return false;
            };
            if constraint.tag != choose.tag
                || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            {
                return false;
            }
            let mut untap_kind = filter.clone();
            untap_kind.tagged_constraints.clear();
            let mut chosen_kind = choose.filter.clone();
            if choose_primary_zone(choose) == Some(Zone::Battlefield) {
                untap_kind.zone = None;
                chosen_kind.zone = None;
            }
            untap_kind == chosen_kind
        }
        _ => false,
    }
}

pub(crate) fn choose_reference_noun(choose: &crate::effects::ChooseObjectsEffect) -> &'static str {
    let permanent_types = [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    if (choose.filter.card_types.is_empty()
        || (choose.filter.card_types.len() == permanent_types.len()
            && permanent_types
                .iter()
                .all(|card_type| choose.filter.card_types.contains(card_type))))
        && choose_primary_zone(choose) == Some(Zone::Battlefield)
    {
        "permanent"
    } else if choose.filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if choose.filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if choose.filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if choose.filter.card_types.contains(&CardType::Land) {
        "land"
    } else {
        "object"
    }
}

pub(super) fn source_exiled_with_phrase(filter: &ObjectFilter) -> String {
    let description = filter.description();
    let base = description
        .split(" in ")
        .next()
        .unwrap_or(description.as_str())
        .trim();
    if let Some(source) = base
        .strip_prefix("exiled with ")
        .and_then(|rest| rest.strip_suffix(" card"))
    {
        return format!("exiled with {source}");
    }
    "exiled with this source".to_string()
}

pub(super) fn describe_source_exiled_choose_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if !choose.count.is_single() || !is_source_exiled_cards_filter(&choose.filter) {
        return None;
    }

    let mut selection = "a card".to_string();
    if choose.count.random {
        selection.push_str(" at random");
    }
    selection.push(' ');
    selection.push_str(&source_exiled_with_phrase(&choose.filter));
    Some(selection)
}

pub(super) fn apply_mana_value_where_x_surface(
    card_desc: &mut String,
    filter: &ObjectFilter,
) -> String {
    let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) = filter.mana_value.as_ref()
    else {
        return String::new();
    };
    if !value_prefers_where_x(value) {
        return String::new();
    }

    let basis = describe_value(value);
    let needle = format!("mana value {basis} or less");
    if card_desc.contains(&needle) {
        *card_desc = card_desc.replace(&needle, "mana value X or less");
    } else if let Some(start) = card_desc.find("mana value ") {
        let tail_start = start + "mana value ".len();
        if let Some(rel_end) = card_desc[tail_start..].find(" or less") {
            let end = tail_start + rel_end + " or less".len();
            card_desc.replace_range(start..end, "mana value X or less");
        }
    }
    format!(", where X is {basis}")
}

pub(super) fn describe_stack_spell_choice(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if !choose.count.is_single() {
        return None;
    }
    let filter = &choose.filter;
    if !filter.all_card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || filter.colors.is_some()
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || !filter.excluded_colors.is_empty()
        || filter.targets_object.is_some()
        || filter.targets_player.is_some()
    {
        return None;
    }

    let mut spell_text = if filter.card_types.is_empty() {
        "spell".to_string()
    } else {
        let type_names = filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        format!("{} spell", join_with_or(&type_names))
    };
    if filter.controller == Some(PlayerFilter::You) {
        spell_text.push_str(" you control");
    } else if filter.controller.is_some() {
        return None;
    }
    Some(with_indefinite_article(&spell_text))
}

pub(crate) fn describe_choose_then_exile(
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if choose.is_search || !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str()) {
        return None;
    }

    if choose_primary_zone(choose) == Some(Zone::Stack)
        && choose.filter.stack_kind == Some(crate::filter::StackObjectKind::Spell)
    {
        let chosen = describe_stack_spell_choice(choose)
            .unwrap_or_else(|| describe_choose_selection(choose));
        let face_down_suffix = if exile.face_down { " face down" } else { "" };
        return Some(format!("Exile {chosen}{face_down_suffix}"));
    }

    if choose_primary_zone(choose) == Some(Zone::Battlefield) {
        let chooser = describe_player_filter(&choose.chooser);
        let verb = player_verb(&chooser, "exile", "exiles");
        let chosen = describe_choose_selection(choose);
        let face_down_suffix = if exile.face_down { " face down" } else { "" };
        return Some(format!("{chooser} {verb} {chosen}{face_down_suffix}"));
    }

    let zones = choose_search_zones(choose)?;
    if !zones
        .iter()
        .all(|zone| matches!(zone, Zone::Hand | Zone::Graveyard | Zone::Library))
    {
        return None;
    }
    let owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let owner_text = describe_possessive_player_filter(owner);
    let origin_zones =
        if zones.len() == 2 && zones.contains(&Zone::Hand) && zones.contains(&Zone::Graveyard) {
            format!("{owner_text} hand or graveyard")
        } else {
            match zones.as_slice() {
                [Zone::Hand] => format!("{owner_text} hand"),
                [Zone::Graveyard] => format!("{owner_text} graveyard"),
                [Zone::Library] => format!("{owner_text} library"),
                _ => describe_search_origin_zones(choose)?,
            }
        };
    let primary_zone = choose_primary_zone(choose)?;
    let origin_prefix = match primary_zone {
        Zone::Library | Zone::Graveyard if choose.top_only => "of",
        Zone::Hand | Zone::Graveyard | Zone::Library => "from",
        _ => return None,
    };
    let chooser = describe_player_filter(&choose.chooser);
    let verb = player_verb(&chooser, "exile", "exiles");
    let mut chosen = describe_choose_selection(choose);
    if zones
        .iter()
        .any(|zone| matches!(zone, Zone::Hand | Zone::Graveyard | Zone::Library))
    {
        if let Some(stripped) = chosen.strip_suffix(" you own") {
            chosen = stripped.to_string();
        }
        if !chosen.contains(" card") && !chosen.contains(" spell") {
            chosen.push_str(" card");
        }
    }
    let face_down_suffix = if exile.face_down { " face down" } else { "" };
    Some(format!(
        "{chooser} {verb} {chosen} {origin_prefix} {origin_zones}{face_down_suffix}"
    ))
}

pub(super) fn describe_choose_exile_then_put_counter(
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
    put: &crate::effects::PutCountersEffect,
) -> Option<String> {
    let counter_follows_chosen_object = matches!(
        &put.target,
        ChooseSpec::Tagged(tag) if tag == &choose.tag
    );
    let counter_follows_source_exiled_object =
        matches!(
            &put.target,
            ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        ) && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str());
    if (!counter_follows_chosen_object && !counter_follows_source_exiled_object)
        || !choose.count.is_single()
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }
    let exile_text = describe_choose_then_exile(choose, exile)?;
    Some(format!(
        "{exile_text} and put {} on it",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn choose_spec_references_tagged_object(spec: &ChooseSpec, tag: &crate::TagKey) -> bool {
    match spec.base() {
        ChooseSpec::Tagged(found) => found == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
        }
        _ => false,
    }
}

pub(super) fn filter_checks_for_suspend(filter: &ObjectFilter) -> bool {
    filter.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
}

pub(super) fn condition_is_tagged_object_without_suspend(
    condition: &Condition,
    tag: &crate::TagKey,
) -> bool {
    let Condition::Not(inner) = condition else {
        return false;
    };
    matches!(
        inner.as_ref(),
        Condition::TaggedObjectMatches(found, filter)
            if found == tag && filter_checks_for_suspend(filter)
    )
}

pub(super) fn ability_is_suspend_exile_trigger(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    matches!(
        triggered.presentation_label.as_ref(),
        Some(crate::ability::PresentationLabel::Keyword(
            crate::ability::PresentationKeyword::Suspend
        ))
    ) && ability.functional_zones == [Zone::Exile]
}

pub(super) fn modification_adds_suspend_exile_trigger(
    modification: &crate::continuous::Modification,
) -> bool {
    matches!(
        modification,
        crate::continuous::Modification::AddAbilityGeneric(ability)
            if ability_is_suspend_exile_trigger(ability)
    )
}

pub(super) fn apply_grants_suspend_to_tag(
    apply: &crate::effects::ApplyContinuousEffect,
    tag: &crate::TagKey,
) -> bool {
    if apply.until != Until::Forever
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !matches!(apply.target_spec.as_ref(), Some(spec) if choose_spec_references_tagged_object(spec, tag))
    {
        return false;
    }

    let modifications = apply
        .modification
        .iter()
        .chain(apply.additional_modifications.iter())
        .collect::<Vec<_>>();
    modifications.len() == 2
        && modifications
            .iter()
            .all(|modification| modification_adds_suspend_exile_trigger(modification))
}

pub(in crate::compiled_text) fn describe_put_counters_then_gain_suspend(
    effects: &[Effect],
) -> Option<String> {
    let [put_effect, conditional_effect] = effects else {
        return None;
    };
    let tagged_put = put_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let put = tagged_put
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::Time || put.target_count.is_some() || put.distributed {
        return None;
    }
    let ChooseSpec::Tagged(target_tag) = put.target.base() else {
        return None;
    };

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    // Source-sentence lowering may bind a following "it" to the result tag of
    // this tagged counter effect rather than to the tag used by its target.
    // `TaggedEffect` captures that exact target, so the two references are
    // semantically equivalent for this single-target, nondistributed effect.
    let suspend_target_is_proven = [target_tag, &tagged_put.tag].into_iter().any(|candidate| {
        condition_is_tagged_object_without_suspend(&conditional.condition, candidate)
            && apply_grants_suspend_to_tag(apply, candidate)
    });
    if !suspend_target_is_proven {
        return None;
    }

    let target_text = describe_choose_spec(&put.target);
    let target_text = if target_text.starts_with("the tagged object '") {
        "the exiled card"
    } else {
        target_text.as_str()
    };
    Some(format!(
        "Put {} on {target_text}. If it doesn't have suspend, it gains suspend",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(in crate::compiled_text) fn describe_exile_with_counters_then_gain_suspend(
    effects: &[Effect],
) -> Option<String> {
    let [exile_effect, put_effect, conditional_effect] = effects else {
        return None;
    };
    let exile_with_counters = describe_source_exile_with_counters_pair(exile_effect, put_effect)?;
    let put_then_suspend =
        describe_put_counters_then_gain_suspend(&[put_effect.clone(), conditional_effect.clone()])?;
    let (_, suspend_clause) = put_then_suspend.split_once(". ")?;
    Some(format!("{exile_with_counters}. {suspend_clause}"))
}

pub(super) fn describe_countered_spell_exile_with_counters_gain_suspend(
    effects: &[Effect],
) -> Option<String> {
    let [local_effect, conditional_effect] = effects else {
        return None;
    };
    let local = local_effect.downcast_ref::<crate::effects::LocalRewriteEffect>()?;
    let counter =
        unwrap_basic_tag_wrappers(&local.effect).downcast_ref::<crate::effects::CounterEffect>()?;
    if !describe_choose_spec(&counter.target)
        .to_ascii_lowercase()
        .contains("spell")
    {
        return None;
    }
    let countered_tag = wrapped_effect_tag(&local.effect)?;
    let [replacement] = local.zone_replacements.as_slice() else {
        return None;
    };
    if replacement.from_zone != Some(Zone::Stack)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.optional
        || !choose_spec_references_tagged_object(&replacement.target, countered_tag)
    {
        return None;
    }
    let [(counter_type, count)] = replacement.counters.as_slice() else {
        return None;
    };
    if *counter_type != CounterType::Time {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !condition_is_tagged_object_without_suspend(&conditional.condition, countered_tag)
    {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_suspend_to_tag(apply, countered_tag) {
        return None;
    }

    let counter_text = describe_put_counter_phrase(&Value::Fixed(*count as i32), *counter_type);
    Some(format!(
        "{}. If that spell is countered this way, exile it with {counter_text} on it instead of putting it into its owner's graveyard. If it doesn't have suspend, it gains suspend",
        describe_effect(&local.effect).trim_end_matches('.')
    ))
}

pub(in crate::compiled_text) fn describe_separated_countered_spell_exile_with_counters_gain_suspend(
    effects: &[Effect],
) -> Option<String> {
    let [counter_effect, replacement_effect, conditional_effect] = effects else {
        return None;
    };
    let counter = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::CounterEffect>()?;
    if !describe_choose_spec(&counter.target)
        .to_ascii_lowercase()
        .contains("spell")
    {
        return None;
    }
    let countered_tag = wrapped_effect_tag(counter_effect)?;
    let replacement =
        replacement_effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?;
    if replacement.from_zone != Some(Zone::Stack)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.optional
        || !choose_spec_references_tagged_object(&replacement.target, countered_tag)
    {
        return None;
    }
    let [(counter_type, count)] = replacement.counters.as_slice() else {
        return None;
    };
    if *counter_type != CounterType::Time {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !condition_is_tagged_object_without_suspend(&conditional.condition, countered_tag)
    {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_suspend_to_tag(apply, countered_tag) {
        return None;
    }

    let counter_text = describe_put_counter_phrase(&Value::Fixed(*count as i32), *counter_type);
    Some(format!(
        "{}. If that spell is countered this way, exile it with {counter_text} on it instead of putting it into its owner's graveyard. If it doesn't have suspend, it gains suspend",
        describe_effect(counter_effect).trim_end_matches('.')
    ))
}

pub(super) fn describe_second_spell_counter_conditional(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if !conditional.if_false.is_empty()
        || !matches!(
            conditional.condition,
            crate::effect::Condition::TargetSpellCastOrderThisTurn(2)
        )
    {
        return None;
    }
    let [counter_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let counter = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::CounterEffect>()?;
    if !describe_choose_spec(&counter.target)
        .to_ascii_lowercase()
        .contains("spell")
    {
        return None;
    }
    Some("Counter target spell that's the second spell cast this turn".to_string())
}

pub(crate) fn move_to_library_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Library
        // Some parser lowerings route the chosen cards through a tagged
        // for-each wrapper and leave the move target as `Iterated`.
        && match move_to_zone.target.base() {
            ChooseSpec::Iterated => true,
            ChooseSpec::Tagged(t) => t.as_str() == tag,
            _ => false,
        }
}

pub(crate) fn move_to_battlefield_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Battlefield
        && match move_to_zone.target.base() {
            ChooseSpec::Iterated => true,
            ChooseSpec::Tagged(t) => t.as_str() == tag,
            _ => false,
        }
}

pub(crate) fn move_to_hand_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Hand
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

pub(crate) fn return_to_hand_uses_chosen_tag(
    return_to_hand: &crate::effects::ReturnToHandEffect,
    tag: &str,
) -> bool {
    match return_to_hand.spec.base() {
        ChooseSpec::Iterated => true,
        ChooseSpec::Tagged(found) => found.as_str() == tag,
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == tag
            })
        }
        _ => false,
    }
}

pub(super) fn describe_exiled_with_source_move(
    surface: &ironsmith_core::ExiledWithSourceMoveSurface,
    zone: Zone,
    contextual_player: Option<&PlayerFilter>,
    battlefield_controller: Option<&crate::effects::BattlefieldController>,
    enters_tapped: bool,
) -> String {
    use ironsmith_core::{
        ExiledWithSourceDestinationSurface as DestinationSurface,
        ExiledWithSourceReferenceSurface as ReferenceSurface,
        ExiledWithSourceSubjectSurface as SubjectSurface,
    };

    let subject = match &surface.subject {
        SubjectSurface::AllCards => "all cards".to_string(),
        SubjectSurface::EachCard => "each card".to_string(),
        SubjectSurface::OwnerOfEachCard => "each card".to_string(),
        SubjectSurface::OneCard => "a card".to_string(),
        SubjectSurface::TheExiledCard => "the exiled card".to_string(),
        SubjectSurface::TheExiledCards => "the exiled cards".to_string(),
        SubjectSurface::TheCards => "the cards".to_string(),
        SubjectSurface::Custom(text) => text.clone(),
    };
    let source = match &surface.source {
        ReferenceSurface::Source(source) => format!(" exiled with {}", source.display_text()),
        ReferenceSurface::It => " exiled with it".to_string(),
        ReferenceSurface::Omitted => String::new(),
    };
    if matches!(&surface.subject, SubjectSurface::OwnerOfEachCard) && zone == Zone::Library {
        return format!(
            "The owner of each card{source} puts that card on the bottom of their library"
        );
    }
    let zone_noun = match zone {
        Zone::Hand => "hand",
        Zone::Graveyard => "graveyard",
        Zone::Library => "library",
        Zone::Exile => "exile",
        Zone::Battlefield => "battlefield",
        Zone::Stack => "stack",
        Zone::Command => "command zone",
        Zone::Ante => "ante",
        Zone::OutsideGame => "outside the game",
    };
    let plural_zone_noun = match zone {
        Zone::Hand => "hands",
        Zone::Graveyard => "graveyards",
        Zone::Library => "libraries",
        _ => zone_noun,
    };
    let destination = if zone == Zone::Battlefield {
        let controller = match battlefield_controller
            .copied()
            .unwrap_or(crate::effects::BattlefieldController::Preserve)
        {
            crate::effects::BattlefieldController::Preserve => String::new(),
            crate::effects::BattlefieldController::You => " under your control".to_string(),
            crate::effects::BattlefieldController::Owner => match surface.destination {
                DestinationSurface::TheirOwners => " under their owners' control".to_string(),
                DestinationSurface::TheirOwner => " under their owner's control".to_string(),
                DestinationSurface::ContextualPlayer | DestinationSurface::ItsOwner => {
                    " under its owner's control".to_string()
                }
            },
        };
        format!(
            "the battlefield{}{controller}",
            if enters_tapped { " tapped" } else { "" }
        )
    } else {
        match surface.destination {
            DestinationSurface::ContextualPlayer => contextual_player
                .map(|player| format!("{} {zone_noun}", describe_possessive_player_filter(player)))
                .unwrap_or_else(|| format!("its owner's {zone_noun}")),
            DestinationSurface::ItsOwner => format!("its owner's {zone_noun}"),
            DestinationSurface::TheirOwner => format!("their owner's {zone_noun}"),
            DestinationSurface::TheirOwners => format!("their owners' {plural_zone_noun}"),
        }
    };

    let preposition = if matches!(
        surface.verb,
        ironsmith_core::ExiledWithSourceMoveVerbSurface::Return
    ) {
        "to"
    } else if zone == Zone::Battlefield {
        "onto"
    } else {
        "into"
    };
    let verb = match surface.verb {
        ironsmith_core::ExiledWithSourceMoveVerbSurface::Put => "Put",
        ironsmith_core::ExiledWithSourceMoveVerbSurface::Return => "Return",
    };
    format!("{verb} {subject}{source} {preposition} {destination}")
}

pub(super) fn describe_return_to_hand_excluded_subtypes(
    return_to_hand: &crate::effects::ReturnToHandEffect,
) -> Option<String> {
    let ChooseSpec::All(filter) = &return_to_hand.spec else {
        return None;
    };
    if filter.excluded_subtypes.is_empty() {
        return None;
    }
    if return_to_hand.destination_player_surface.is_none()
        && filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Each)
        && let Some(relative) =
            ironsmith_core::filter_model::describe_relative_characteristic_list_filter(filter)
    {
        return Some(format!("each {relative} to its owner's hand"));
    }

    let mut base_filter = filter.clone();
    base_filter.excluded_subtypes.clear();
    let target_text = describe_choose_spec(&ChooseSpec::All(base_filter));
    let excluded = filter
        .excluded_subtypes
        .iter()
        .map(|subtype| pluralize_word(&subtype.to_string()))
        .collect::<Vec<_>>();

    Some(format!(
        "{target_text} to {} except for {}",
        owner_hand_phrase_for_spec(&return_to_hand.spec),
        join_with_and(&excluded)
    ))
}

pub(super) fn describe_each_player_return_all_from_their_graveyard(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let (_, return_all) = tagged_return_all_from_graveyard(&for_players.effects[0])?;
    if return_all.filter.zone != Some(Zone::Graveyard)
        || return_all.filter.owner != Some(PlayerFilter::IteratedPlayer)
    {
        return None;
    }

    if return_all.filter.has_return_destination_first_surface()
        && !return_all.tapped
        && !return_all.face_down
        && return_all.battlefield_controller == ironsmith_core::BattlefieldController::Owner
        && return_all.verb_surface == ironsmith_core::MoveToZoneVerbSurface::Return
    {
        let mut remaining = return_all.filter.clone();
        remaining.zone = None;
        remaining.owner = None;
        remaining.card_types.clear();
        remaining.entered_graveyard_this_turn = false;
        remaining.entered_graveyard_from_battlefield_this_turn = false;
        if !return_all.filter.card_types.is_empty() && remaining == ObjectFilter::default() {
            let card_types = return_all
                .filter
                .card_types
                .iter()
                .map(|card_type| card_type.name().to_string())
                .collect::<Vec<_>>();
            let target_text = format!("all {} cards", join_with_and(&card_types));
            let history_clause = graveyard_entry_history_clause_for_spec(&ChooseSpec::All(
                return_all.filter.clone(),
            ));
            return Some(format!(
                "Each player returns to the battlefield {target_text} in their graveyard{history_clause}"
            ));
        }
    }

    let target_text =
        describe_choose_spec_without_graveyard_zone(&ChooseSpec::All(return_all.filter.clone()));
    Some(format!(
        "Each player returns {target_text} from their graveyard to the battlefield{}",
        if return_all.tapped { " tapped" } else { "" }
    ))
}

pub(super) fn describe_each_player_return_all_from_their_graveyard_with_counters(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 2 {
        return None;
    }

    let (return_all_tag, return_all) = tagged_return_all_from_graveyard(&for_players.effects[0])?;
    if return_all.filter.zone != Some(Zone::Graveyard)
        || return_all.filter.owner != Some(PlayerFilter::IteratedPlayer)
    {
        return None;
    }

    let put_counters = tagged_put_counters_effect(&for_players.effects[1])?;
    if put_counters.distributed
        || put_counters.target_count.is_some()
        || !matches!(
            &put_counters.target,
            ChooseSpec::Tagged(tag) if Some(tag) == return_all_tag
        )
        || !matches!(put_counters.amount, Value::Fixed(1))
    {
        return None;
    }

    let target_text =
        describe_choose_spec_without_graveyard_zone(&ChooseSpec::All(return_all.filter.clone()));
    let target_text = target_text
        .strip_prefix("all ")
        .map(|rest| {
            rest.strip_suffix(" cards")
                .map(|singular| format!("each {singular} card"))
                .unwrap_or_else(|| format!("each {rest}"))
        })
        .unwrap_or(target_text);
    Some(format!(
        "Each player returns {target_text} from their graveyard to the battlefield with an additional {} counter on it",
        describe_counter_type(put_counters.counter_type),
    ))
}

pub(super) fn describe_move_to_battlefield_with_additional_counters(
    effects: &[Effect],
) -> Option<String> {
    let [move_effect, counter_effect] = effects else {
        return None;
    };
    let moved_tag = structural_effect_tag(move_effect)?;
    let move_to_zone = unwrap_structural_effect_tag(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
    {
        return None;
    }

    let put_counters = tagged_put_counters_effect(counter_effect)?;
    if put_counters.distributed
        || put_counters.target_count.is_some()
        || !matches!(put_counters.amount, Value::Fixed(1))
        || !matches!(
            &put_counters.target,
            ChooseSpec::Tagged(tag) if tag == moved_tag
        )
    {
        return None;
    }

    let target_text = if let Some(owner) = graveyard_owner_from_spec(&move_to_zone.target) {
        let target_text = describe_choose_spec_without_graveyard_zone(&move_to_zone.target);
        let from_text = match owner {
            Some(owner) => format!(
                "{} graveyard",
                describe_possessive_graveyard_owner_filter(&owner)
            ),
            None if choose_spec_allows_multiple(&move_to_zone.target) => "graveyards".to_string(),
            None => "a graveyard".to_string(),
        };
        format!("{target_text} from {from_text}")
    } else {
        describe_choose_spec(&move_to_zone.target)
    };
    let controller_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => {
            if choose_spec_allows_multiple(&move_to_zone.target) {
                " under their owners' control"
            } else {
                " under its owner's control"
            }
        }
        crate::effects::BattlefieldController::You => " under your control",
    };
    let entering_subject = if choose_spec_allows_multiple(&move_to_zone.target) {
        "Each of them enters"
    } else {
        "It enters"
    };

    if move_to_zone.verb_surface == ironsmith_core::MoveToZoneVerbSurface::Return
        && matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
        )
    {
        let move_text = describe_effect(&Effect::new(move_to_zone.clone()))
            .trim_end_matches('.')
            .to_string();
        return Some(format!(
            "{move_text} and put {} on it",
            describe_put_counter_phrase(&put_counters.amount, put_counters.counter_type)
        ));
    }

    Some(format!(
        "Put {target_text} onto the battlefield{controller_suffix}. {entering_subject} with an additional {} counter on it",
        describe_counter_type(put_counters.counter_type),
    ))
}

pub(super) fn describe_return_from_graveyard_with_counters(effects: &[Effect]) -> Option<String> {
    let [return_effect, counter_effect] = effects else {
        return None;
    };

    fn collect_wrapper_tags<'a>(effect: &'a Effect, tags: &mut Vec<&'a TagKey>) {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            tags.push(&tagged.tag);
            collect_wrapper_tags(&tagged.effect, tags);
        } else if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            tags.push(&tag_all.tag);
            collect_wrapper_tags(&tag_all.effect, tags);
        } else if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            collect_wrapper_tags(&with_id.effect, tags);
        }
    }

    let mut return_tags = Vec::new();
    collect_wrapper_tags(return_effect, &mut return_tags);
    if return_tags.is_empty() {
        return None;
    }

    let return_to_battlefield = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;
    if return_to_battlefield.as_aura.is_some() {
        return None;
    }

    let put_counters = unwrap_basic_tag_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put_counters.distributed || put_counters.target_count.is_some() {
        return None;
    }
    if !return_tags
        .iter()
        .any(|tag| choose_spec_references_exact_tag(&put_counters.target, tag))
    {
        return None;
    }

    let source_return = matches!(return_to_battlefield.target.unhinted(), ChooseSpec::Source);
    let counter_text = if source_return {
        describe_put_counter_phrase(&put_counters.amount, put_counters.counter_type)
    } else {
        match &put_counters.amount {
            Value::Fixed(1) => format!(
                "an additional {} counter",
                describe_counter_type(put_counters.counter_type)
            ),
            Value::Fixed(amount) => {
                let count_text = number_word(*amount).unwrap_or_else(|| amount.to_string());
                format!(
                    "{count_text} additional {} counters",
                    describe_counter_type(put_counters.counter_type)
                )
            }
            _ => return None,
        }
    };

    let mut text = if source_return {
        format!(
            "Return this card from your graveyard to the battlefield{}",
            if return_to_battlefield.tapped {
                " tapped"
            } else {
                ""
            }
        )
    } else {
        describe_effect(return_effect)
            .trim_end_matches('.')
            .to_string()
    };
    text.push_str(" with ");
    text.push_str(&counter_text);
    text.push_str(" on it");
    Some(text)
}

fn battlefield_entry_object_noun(filter: Option<&ObjectFilter>) -> String {
    filter
        .map(|filter| strip_leading_article(&filter.description()).to_string())
        .filter(|description| !description.is_empty())
        .unwrap_or_else(|| "permanent".to_string())
}

fn battlefield_entry_counter_phrase(
    counter: &ironsmith_core::BattlefieldEntryCounterSpec,
    additional: bool,
) -> String {
    let counter_type = describe_counter_type(counter.counter_type);
    let modifier = if additional { "additional " } else { "" };
    match counter.amount.unhinted() {
        Value::Fixed(1) => {
            let article = if additional { "an" } else { "a" };
            format!("{article} {modifier}{counter_type} counter")
        }
        Value::Fixed(amount) if *amount > 1 => {
            let amount = number_word(*amount).unwrap_or_else(|| amount.to_string());
            format!("{amount} {modifier}{counter_type} counters")
        }
        amount => format!(
            "{} {modifier}{counter_type} counters",
            describe_value(amount)
        ),
    }
}

pub(super) fn append_battlefield_entry_counter_surface(
    base: String,
    counters: &[ironsmith_core::BattlefieldEntryCounterSpec],
) -> String {
    let mut rendered = base.trim_end_matches('.').to_string();
    let mut index = 0usize;
    while index < counters.len() {
        if counters[index].surface
            == ironsmith_core::BattlefieldEntryCounterSurface::EachOfThemEnters
            && counters[index].object_filter.is_some()
        {
            let end = counters[index..]
                .iter()
                .position(|counter| {
                    counter.surface
                        != ironsmith_core::BattlefieldEntryCounterSurface::EachOfThemEnters
                        || counter.object_filter.is_none()
                })
                .map(|offset| index + offset)
                .unwrap_or(counters.len());
            let conditional_counters = &counters[index..end];
            if conditional_counters.len() >= 2 {
                let each_return_surface = if rendered.starts_with("Return ") {
                    rendered
                        .split_once(" to the battlefield")
                        .map(|(_, suffix)| ("Return", "to", suffix))
                } else if rendered.starts_with("Put ") {
                    rendered
                        .split_once(" onto the battlefield")
                        .map(|(_, suffix)| ("Put", "onto", suffix))
                } else {
                    None
                };
                if let Some((verb, destination, suffix)) = each_return_surface {
                    let suffix = suffix.replacen(
                        "under their owners' control",
                        "under its owner's control",
                        1,
                    );
                    rendered = format!("{verb} each of them {destination} the battlefield{suffix}");
                }
                rendered.push_str(". Each of them enters with ");
                for (arm_index, counter) in conditional_counters.iter().enumerate() {
                    if arm_index > 0 {
                        if arm_index + 1 == conditional_counters.len() {
                            rendered.push_str(" and ");
                        } else {
                            rendered.push_str(", ");
                        }
                    }
                    let additional = counter
                        .amount
                        .has_surface_hint(ValueSurfaceHint::AdditionalEntryCounter);
                    rendered.push_str(&battlefield_entry_counter_phrase(counter, additional));
                    rendered.push_str(" on it if it's ");
                    let noun = battlefield_entry_object_noun(counter.object_filter.as_ref());
                    rendered.push_str(&with_indefinite_article(&noun));
                }
                index = end;
                continue;
            }
        }

        let counter = &counters[index];
        let noun = battlefield_entry_object_noun(counter.object_filter.as_ref());
        let additional = counter
            .amount
            .has_surface_hint(ValueSurfaceHint::AdditionalEntryCounter);
        let counter_phrase = battlefield_entry_counter_phrase(counter, additional);
        let clause = match counter.surface {
            ironsmith_core::BattlefieldEntryCounterSurface::Inline => {
                rendered.push_str(" with ");
                rendered.push_str(&counter_phrase);
                rendered.push_str(" on it");
                index += 1;
                continue;
            }
            ironsmith_core::BattlefieldEntryCounterSurface::EachOfThemEnters => {
                format!("Each of them enters with {counter_phrase} on it")
            }
            ironsmith_core::BattlefieldEntryCounterSurface::IfObjectEntersThisWay => {
                let counter_phrase = battlefield_entry_counter_phrase(counter, true);
                format!(
                    "If {}, it enters with {counter_phrase} on it",
                    with_indefinite_article(&format!("{noun} enters this way"))
                )
            }
            ironsmith_core::BattlefieldEntryCounterSurface::IfItEntersAsObject => {
                let counter_phrase = battlefield_entry_counter_phrase(counter, true);
                format!(
                    "If it enters as {}, it enters with {counter_phrase} on it",
                    with_indefinite_article(&noun)
                )
            }
            ironsmith_core::BattlefieldEntryCounterSurface::ItEntersIfObject => {
                let counter_phrase = battlefield_entry_counter_phrase(counter, true);
                format!(
                    "It enters with {counter_phrase} on it if it's {}",
                    with_indefinite_article(&noun)
                )
            }
            ironsmith_core::BattlefieldEntryCounterSurface::ThatObjectEntersIfCondition => {
                let counter_phrase = battlefield_entry_counter_phrase(counter, true);
                let condition = counter
                    .condition
                    .as_ref()
                    .map(describe_condition)
                    .unwrap_or_else(|| "the condition is met".to_string());
                format!("If {condition}, that {noun} enters with {counter_phrase} on it")
            }
        };
        rendered.push_str(". ");
        rendered.push_str(&clause);
        index += 1;
    }
    rendered
}

pub(super) fn tagged_return_all_from_graveyard(
    effect: &Effect,
) -> Option<(
    Option<&crate::TagKey>,
    &crate::effects::ReturnAllToBattlefieldEffect,
)> {
    if let Some(return_all) = effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        return Some((None, return_all));
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let return_all = tagged
        .effect
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    Some((Some(&tagged.tag), return_all))
}

pub(super) fn tagged_put_counters_effect(
    effect: &Effect,
) -> Option<&crate::effects::PutCountersEffect> {
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        return Some(put_counters);
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()?
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
}

pub(super) fn describe_no_more_counters_move_then_each_player_return(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    let Condition::SourceHasNoCounter(counter_type) = conditional.condition else {
        return None;
    };
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 2 {
        return None;
    }
    let move_to_zone = conditional.if_true[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(_) | ChooseSpec::Source
        )
    {
        return None;
    }
    let for_players = conditional.if_true[1].downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let return_text = describe_each_player_return_all_from_their_graveyard(for_players)?;

    Some(format!(
        "If there are no more {} counters on it, put it into your graveyard and {}",
        counter_type.description(),
        lowercase_first(&return_text)
    ))
}

pub(super) fn half_rounded_up_subject(value: &Value) -> Option<String> {
    let Value::HalfRoundedDown(inner) = value else {
        return None;
    };
    let Value::Add(left, right) = inner.as_ref() else {
        return None;
    };
    let count_filter = match (left.as_ref(), right.as_ref()) {
        (Value::Count(filter), Value::Fixed(1)) | (Value::Fixed(1), Value::Count(filter)) => {
            Some(filter)
        }
        _ => None,
    }?;
    Some(describe_count_filter_value_subject(count_filter))
}

pub(crate) fn describe_choose_zone_origin(
    choose: &crate::effects::ChooseObjectsEffect,
    zone_text: &str,
) -> String {
    match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => format!("from their {zone_text}"),
        Some(owner) => format!(
            "from {} {zone_text}",
            describe_possessive_player_filter(owner)
        ),
        None => format!("from a {zone_text}"),
    }
}

pub(super) fn describe_choose_zone_location(
    choose: &crate::effects::ChooseObjectsEffect,
    zone_text: &str,
) -> String {
    match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => format!("in their {zone_text}"),
        Some(owner) => format!(
            "in {} {zone_text}",
            describe_possessive_player_filter(owner)
        ),
        None => format!("in a {zone_text}"),
    }
}

pub(crate) fn describe_choose_then_move_to_battlefield(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if choose.is_search || !move_to_battlefield_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }

    let zones = choose_search_zones(choose)?;
    let primary_zone = *zones.first()?;
    let origin =
        if zones.len() == 2 && zones.contains(&Zone::Hand) && zones.contains(&Zone::Graveyard) {
            let owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
            format!(
                "from {} hand or graveyard",
                describe_possessive_player_filter(owner)
            )
        } else {
            match primary_zone {
                Zone::Hand => describe_choose_zone_origin(choose, "hand"),
                Zone::Graveyard => describe_choose_zone_origin(choose, "graveyard"),
                Zone::Library => {
                    if choose.top_only {
                        match choose.filter.owner.as_ref() {
                            Some(PlayerFilter::IteratedPlayer) => {
                                "from the top of their library".to_string()
                            }
                            Some(owner) => format!(
                                "from the top of {} library",
                                describe_possessive_player_filter(owner)
                            ),
                            None => "from the top of a library".to_string(),
                        }
                    } else {
                        describe_choose_zone_origin(choose, "library")
                    }
                }
                _ => return None,
            }
        };

    let chooser = describe_player_filter(&choose.chooser);
    let mut chosen = describe_choose_selection(choose);
    let where_x_clause = if let Some((head, tail)) = chosen.split_once(", where X is ") {
        let tail = tail.to_string();
        chosen = head.to_string();
        format!(", where X is {tail}")
    } else {
        String::new()
    };
    let tapped = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let attacking = if move_to_zone.enters_attacking {
        " and attacking"
    } else {
        ""
    };
    let control_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => String::new(),
        crate::effects::BattlefieldController::Owner => " under its owner's control".to_string(),
        crate::effects::BattlefieldController::You => {
            if chooser == "you" && !move_to_zone.controller_surface_explicit {
                String::new()
            } else {
                " under your control".to_string()
            }
        }
    };

    if let Some(actor) = move_to_zone.actor_surface.as_ref()
        && !player_filters_refer_to_same_player(actor, &choose.chooser)
    {
        // A choice and its linked move are not necessarily performed by the
        // same player ("that player chooses ..., then you put it ...").
        // Keep the two typed roles explicit instead of folding the chooser
        // into an "of their choice" modifier on the move.
        if !choose.count.is_single() {
            return None;
        }
        let location = if zones.len() == 1 {
            match primary_zone {
                Zone::Hand => describe_choose_zone_location(choose, "hand"),
                Zone::Graveyard => describe_choose_zone_location(choose, "graveyard"),
                _ => origin.clone(),
            }
        } else {
            origin.clone()
        };
        let choose_clause = if chooser == "you" {
            format!("Choose {chosen} {location}")
        } else {
            let choose_verb = player_verb(&chooser, "choose", "chooses");
            format!(
                "{} {choose_verb} {chosen} {location}",
                capitalize_first(&chooser)
            )
        };
        let move_clause = describe_effect(&Effect::new(move_to_zone.clone()));
        return Some(format!(
            "{choose_clause}, then {}",
            lowercase_first(&move_clause)
        ));
    }

    if chooser != "you"
        && move_to_zone.battlefield_controller == crate::effects::BattlefieldController::You
    {
        let origin = if choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| player_filters_refer_to_same_player(owner, &choose.chooser))
        {
            match primary_zone {
                Zone::Hand => "from their hand".to_string(),
                Zone::Graveyard => "from their graveyard".to_string(),
                Zone::Library if choose.top_only => "from the top of their library".to_string(),
                Zone::Library => "from their library".to_string(),
                _ => origin,
            }
        } else {
            origin
        };
        let put_verb = player_verb(&chooser, "put", "puts");
        return Some(format!(
            "{} {put_verb} {chosen} of their choice {origin} onto the battlefield{tapped}{attacking}{control_suffix}{where_x_clause}",
            capitalize_first(&chooser)
        ));
    }

    let put_verb = player_verb(&chooser, "put", "puts");
    Some(format!(
        "{chooser} {put_verb} {chosen} {origin} onto the battlefield{tapped}{attacking}{control_suffix}{where_x_clause}"
    ))
}

pub(crate) fn describe_choose_then_move_to_hand(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if choose.is_search
        || !is_source_exiled_cards_filter(&choose.filter)
        || !move_to_hand_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let put_verb = player_verb(&chooser, "put", "puts");
    let chosen = describe_choose_selection(choose);
    let moved_ref = if choose.count.is_single() {
        "that card"
    } else {
        "those cards"
    };

    Some(format!(
        "{chooser} {choose_verb} {chosen} and {put_verb} {moved_ref} into its owner's hand"
    ))
}

pub(crate) fn describe_choose_then_move_to_library(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if !move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }

    let primary_zone = choose_primary_zone(choose)?;
    let origin = match primary_zone {
        Zone::Hand => describe_choose_zone_origin(choose, "hand"),
        Zone::Graveyard => describe_choose_zone_origin(choose, "graveyard"),
        Zone::Library => {
            if choose.top_only {
                match choose.filter.owner.as_ref() {
                    Some(PlayerFilter::IteratedPlayer) => {
                        "from the top of their library".to_string()
                    }
                    Some(owner) => format!(
                        "from the top of {} library",
                        describe_possessive_player_filter(owner)
                    ),
                    None => "from the top of a library".to_string(),
                }
            } else {
                describe_choose_zone_origin(choose, "library")
            }
        }
        _ => return None,
    };

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let put_verb = player_verb(&chooser, "put", "puts");
    let chosen = describe_choose_selection(choose);
    let moved_ref = if choose.count.is_single() {
        "it"
    } else {
        "them"
    };

    let destination = match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => "their library".to_string(),
        Some(owner) => format!("{} library", describe_possessive_player_filter(owner)),
        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
    };
    let placement = if move_to_zone.to_top {
        "on top of"
    } else {
        "on the bottom of"
    };
    let order_suffix = if choose.count.is_single() {
        ""
    } else {
        " in any order"
    };
    let connector = if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.filter.owner.as_ref() == Some(&PlayerFilter::IteratedPlayer)
    {
        " and "
    } else {
        ", then "
    };

    if choose.top_only && primary_zone == Zone::Graveyard {
        let origin = origin
            .strip_prefix("from ")
            .map_or(origin.clone(), |rest| format!("of {rest}"));
        return Some(format!(
            "{chooser} {put_verb} {chosen} {origin} {placement} {destination}{order_suffix}"
        ));
    }

    Some(format!(
        "{chooser} {choose_verb} {chosen} {origin}{connector}{put_verb} {moved_ref} {placement} {destination}{order_suffix}"
    ))
}

pub(crate) fn describe_target_player_choose_half_then_return_to_hand(
    choose: &crate::effects::ChooseObjectsEffect,
    return_to_hand: &crate::effects::ReturnToHandEffect,
) -> Option<String> {
    if choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.is_search
        || !choose.count.dynamic_x
        || !return_to_hand_uses_chosen_tag(return_to_hand, choose.tag.as_str())
    {
        return None;
    }

    // Reference resolution intentionally turns a declared target into an
    // alias for subsequent effects. This bundle is itself responsible for
    // rendering that declaration, so recover the underlying targeted player
    // surface from either representation.
    let chooser = match &choose.chooser {
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            format!(
                "target {}",
                strip_indefinite_article(&describe_player_filter(inner))
            )
        }
        _ => return None,
    };

    let subject = half_rounded_up_subject(choose.count_value.as_ref()?)?;
    Some(format!(
        "Choose {chooser}. Return half the {subject} to their owner's hand, rounded up"
    ))
}

pub(crate) fn describe_look_at_top_then_choose_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str()) {
        return None;
    }
    describe_look_at_top_then_choose_exile_text(look_at_top, choose, exile.face_down)
}

pub(crate) fn describe_look_at_top_then_choose_move_to_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if !move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }
    describe_look_at_top_then_choose_exile_text(look_at_top, choose, false)
}

pub(crate) fn describe_look_at_top_then_move_to_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if move_to_zone.zone != Zone::Exile {
        return None;
    }
    if !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &look_at_top.tag) {
        return None;
    }

    Some(describe_exile_top_of_library(
        &look_at_top.player,
        &look_at_top.count,
        false,
    ))
}

pub(super) fn looked_card_choice_filter_is_plain_remainder(
    choose: &crate::effects::ChooseObjectsEffect,
    looked_tag: &str,
    excluded_tags: &[&str],
) -> bool {
    if choose_primary_zone(choose) != Some(Zone::Library)
        || choose.is_search
        || !choose.count.is_single()
    {
        return false;
    }

    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == looked_tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    });
    if !references_looked {
        return false;
    }

    let excludes_expected = excluded_tags.iter().all(|expected| {
        choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == *expected
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                )
        })
    });
    if !excludes_expected {
        return false;
    }

    let mut bare = choose.filter.clone();
    bare.zone = None;
    bare.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == looked_tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ))
            && !(excluded_tags.contains(&constraint.tag.as_str())
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                ))
    });
    bare == ObjectFilter::default()
}

pub(super) fn tagged_move_to_zone(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &TagKey,
    zone: Zone,
    to_top: bool,
) -> bool {
    move_to_zone.zone == zone
        && move_to_zone.to_top == to_top
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(move_tag) if move_tag == tag)
}

pub(super) fn describe_reveal_top_then_temporarily_play_revealed_top_card(
    reveal_top: &crate::effects::RevealTopEffect,
    reveal_permission: &crate::effects::ApplyContinuousEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
    grant_free_cast: &crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
) -> Option<String> {
    let tag = reveal_top.tag.as_ref()?;
    if reveal_top.player != PlayerFilter::You
        || !matches!(
            reveal_permission.target,
            crate::continuous::EffectTarget::Source
        )
        || reveal_permission.until != Until::EndOfTurn
        || !apply_continuous_adds_static_ability(
            reveal_permission,
            crate::static_abilities::StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
        )
        || grant_play.tag != *tag
        || grant_free_cast.tag != *tag
        || grant_play.player != grant_free_cast.player
        || grant_play.player != PlayerFilter::You
        || grant_play.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || !grant_play.allow_land
        || !grant_play.while_on_top_of_library
        || !grant_free_cast.while_on_top_of_library
        || !matches!(
            reveal_permission.condition,
            Some(crate::ConditionExpr::TaggedObjectIsTopOfLibrary { .. })
                | Some(crate::ConditionExpr::StableObjectIsTopOfLibrary { .. })
        )
    {
        return None;
    }

    Some(
        concat!(
            "Reveal the top card of your library. Until end of turn, ",
            "for as long as that card remains on top of your library, ",
            "play with the top card of your library revealed and you may play ",
            "that card without paying its mana cost"
        )
        .to_string(),
    )
}

pub(in crate::compiled_text) fn describe_shuffle_then_reveal_top_then_temporarily_play_revealed_top_card(
    shuffle: &crate::effects::ShuffleLibraryEffect,
    reveal_top: &crate::effects::RevealTopEffect,
    reveal_permission: &crate::effects::ApplyContinuousEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
    grant_free_cast: &crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
) -> Option<String> {
    if shuffle.player != PlayerFilter::You {
        return None;
    }

    describe_reveal_top_then_temporarily_play_revealed_top_card(
        reveal_top,
        reveal_permission,
        grant_play,
        grant_free_cast,
    )?;

    Some(
        concat!(
            "Shuffle your library, then reveal the top card. Until end of turn, ",
            "for as long as that card remains on top of your library, ",
            "play with the top card of your library revealed and you may play ",
            "that card without paying its mana cost"
        )
        .to_string(),
    )
}

pub(crate) fn describe_exile_top_then_play_without_paying_mana(
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
    grant_free_cast: &crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
) -> Option<String> {
    let Some(first_tag) = exile_top.moved_tags.first() else {
        return None;
    };
    if grant_play.player != grant_free_cast.player
        || grant_play.tag != grant_free_cast.tag
        || grant_play.tag != *first_tag
        || grant_play.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || !grant_play.allow_land
        || !matches!(
            exile_top.player,
            PlayerFilter::DamagedPlayer | PlayerFilter::You
        )
        || !matches!(grant_play.player, PlayerFilter::You)
    {
        return None;
    }

    let (count_text, noun, singular_count) = describe_look_count_and_noun(&exile_top.count);
    let cards_text = if singular_count {
        "that card"
    } else {
        "those cards"
    };
    let mana_cost_text = if singular_count {
        "its mana cost"
    } else {
        "their mana costs"
    };
    let exile_clause = if exile_top.player == PlayerFilter::DamagedPlayer {
        // Preserve the established combat-damage antecedent surface while the
        // same structural helper also accepts a controller-owned exile.
        format!("That player exiles the top {count_text} {noun} of their library")
    } else {
        describe_exile_top_clause(exile_top, false)?.0
    };
    if let Some(permission) = describe_temporary_tagged_permission_surface(grant_play, true) {
        return Some(format!("{exile_clause}. {}", capitalize_first(&permission)));
    }
    Some(format!(
        "{exile_clause}. Until end of turn, you may play {cards_text} without paying {mana_cost_text}"
    ))
}

pub(super) fn describe_exile_top_clause(
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    suppress_count_where_clause: bool,
) -> Option<(String, bool)> {
    let owner = if exile_top.player == PlayerFilter::DamagedPlayer {
        // This action immediately follows the combat-damage trigger that
        // introduced the player, so Oracle keeps the demonstrative antecedent
        // rather than switching to a free pronoun.
        "that player's".to_string()
    } else {
        describe_possessive_player_filter(&exile_top.player)
    };
    let dynamic_count_basis = match &exile_top.count {
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering" || tag.as_str().starts_with("sacrificed")) => {
            Some("its mana value")
        }
        _ => None,
    };
    let singular_count = matches!(exile_top.count, Value::Fixed(1));
    let exile_clause = if let Some(backref) = describe_effect_count_backref(&exile_top.count) {
        format!("Exile {backref} cards from the top of {owner} library")
    } else if let Some(basis) = dynamic_count_basis {
        format!("Exile cards equal to {basis} from the top of {owner} library")
    } else if let Value::Fixed(n) = &exile_top.count {
        if *n < 0 {
            return None;
        }
        let count_u32 = *n as u32;
        let (count_text, noun) = if *n == 1 {
            (String::new(), "card")
        } else {
            let word = small_number_word(count_u32).unwrap_or_else(|| n.to_string());
            (format!("{word} "), "cards")
        };
        format!("Exile the top {count_text}{noun} of {owner} library")
    } else {
        let value_text = describe_value(&exile_top.count);
        if value_text == "X"
            || (suppress_count_where_clause
                && value_has_surface_hint(&exile_top.count, ValueSurfaceHint::WhereXIs))
        {
            format!("Exile the top X cards of {owner} library")
        } else {
            format!("Exile the top X cards of {owner} library, where X is {value_text}")
        }
    };
    let exile_clause =
        if exile_top.surface == Some(ironsmith_core::ExileTopLibrarySurface::LibraryOwnerAsActor) {
            let actor = capitalize_first(&describe_player_filter(&exile_top.player));
            let owner_library = format!("{owner} library");
            let pronoun_library = if exile_top.player == PlayerFilter::You {
                "your library"
            } else {
                "their library"
            };
            let action = exile_clause.strip_prefix("Exile ")?;
            format!(
                "{actor} exiles {}",
                action.replace(&owner_library, pronoun_library)
            )
        } else {
            exile_clause
        };
    Some((exile_clause, singular_count))
}

pub(super) fn filter_is_exactly_tagged_in_zone(
    filter: &ObjectFilter,
    tag: &crate::TagKey,
    zone: Zone,
) -> bool {
    let mut expected = ObjectFilter::default().in_zone(zone);
    expected
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    filter == &expected
}

pub(in crate::compiled_text) fn describe_exile_top_choose_one_then_play(
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    let [exiled_tag] = exile_top.moved_tags.as_slice() else {
        return None;
    };
    if !exile_top.accumulated_tags.is_empty()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || !choose.additional_zones.is_empty()
        || choose_exact_count(choose) != Some(1)
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !filter_is_exactly_tagged_in_zone(&choose.filter, exiled_tag, Zone::Exile)
        || grant_play.tag != choose.tag
        || grant_play.player != PlayerFilter::You
        || grant_play.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant_play.allow_any_color_for_cast
        || grant_play.while_on_top_of_library
        || grant_play.filter.is_some()
        || grant_play.cast_pool_is_plural
    {
        return None;
    }

    let (exile_clause, singular_count) = describe_exile_top_clause(exile_top, false)?;
    if singular_count {
        return None;
    }
    let verb = if grant_play.allow_land {
        "play"
    } else {
        "cast"
    };
    if grant_play
        .surface
        .as_ref()
        .is_some_and(|surface| !surface.leading_duration)
    {
        return Some(format!(
            "{exile_clause}. Choose one of them. You may {verb} that card this turn"
        ));
    }
    Some(format!(
        "{exile_clause}. Choose one of them. Until end of turn, you may {verb} that card"
    ))
}

pub(super) fn describe_exile_top_then_may_cast(
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
    {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.player != PlayerFilter::You || cast.as_copy || cast.cost_reduction.is_some() {
        return None;
    }
    let exiled_tag_matches = exile_top
        .moved_tags
        .iter()
        .chain(exile_top.accumulated_tags.iter())
        .any(|tag| tag == &cast.tag)
        || cast.tag.as_str() == crate::tag::SOURCE_EXILED_TAG;
    if !exiled_tag_matches {
        return None;
    }

    let (exile_clause, singular_count) = describe_exile_top_clause(exile_top, false)?;
    if !singular_count {
        return None;
    }
    let verb = if cast.allow_land { "play" } else { "cast" };
    let mut permission = format!("You may {verb} it");
    if cast.without_paying_mana_cost {
        permission.push_str(" without paying its mana cost");
    }
    Some(format!("{exile_clause}. {permission}"))
}

pub(super) fn describe_exile_top_then_play(
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
    suppress_count_where_clause: bool,
) -> Option<String> {
    let Some(first_tag) = exile_top.moved_tags.first() else {
        return None;
    };
    if grant_play.tag != *first_tag {
        return None;
    }
    let duration_text = match grant_play.duration {
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "Until end of turn",
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            "Until the end of your next turn"
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => "Until your next end step",
        _ => return None,
    };

    let (exile_clause, singular_count) =
        describe_exile_top_clause(exile_top, suppress_count_where_clause)?;
    let cards_text = if singular_count {
        "that card"
    } else {
        "those cards"
    };
    let verb = if grant_play.allow_land {
        "play"
    } else {
        "cast"
    };
    let player = describe_player_filter(&grant_play.player);
    let spell_ref = if grant_play.allow_land {
        if singular_count {
            "that spell"
        } else {
            "those spells"
        }
    } else if singular_count {
        "that spell"
    } else {
        "them"
    };
    let mana_suffix = grant_play
        .mana_spend_cast_clause(spell_ref)
        .map(|clause| format!(", and {clause}"))
        .unwrap_or_default();

    if !grant_play.allow_land && !singular_count {
        let pool_text = if grant_play.tag.as_str() == "exiled"
            || crate::cards::is_sentence_helper_tag(grant_play.tag.as_str(), "exiled")
        {
            "those exiled cards"
        } else {
            "those cards"
        };
        return Some(format!(
            "{exile_clause}. {duration_text}, {player} may cast spells from among {pool_text}{mana_suffix}"
        ));
    }

    if grant_play.duration == crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        && !grant_play.allow_any_color_for_cast
        && grant_play.player == exile_top.player
    {
        return Some(format!(
            "{exile_clause}. You may {verb} {cards_text} this turn"
        ));
    }

    if player == "you" {
        return Some(format!(
            "{exile_clause}. {duration_text}, you may {verb} {cards_text}{mana_suffix}"
        ));
    }

    Some(format!(
        "{exile_clause}. {duration_text}, {player} may {verb} {cards_text}{mana_suffix}"
    ))
}

pub(super) fn describe_triggering_counter_count_exile_top_then_play(
    tag_triggering: &crate::effects::TagTriggeringObjectEffect,
    exile_top: &crate::effects::ExileTopOfLibraryEffect,
    grant_play: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    let Some(first_tag) = exile_top.moved_tags.first() else {
        return None;
    };
    if grant_play.tag != *first_tag
        || grant_play.player != PlayerFilter::You
        || !matches!(
            grant_play.duration,
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd
                | crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep
        )
        || grant_play.allow_any_color_for_cast
        || grant_play.while_on_top_of_library
        || grant_play.filter.is_some()
    {
        return None;
    }
    let Value::CountersOn(spec, None) = &exile_top.count else {
        return None;
    };
    if !matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &tag_triggering.tag) {
        return None;
    }

    let owner = describe_possessive_player_filter(&exile_top.player);
    let verb = if grant_play.allow_land {
        "play"
    } else {
        "cast"
    };
    let duration = match grant_play.duration {
        crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
            "Until the end of your next turn"
        }
        crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => "Until your next end step",
        _ => return None,
    };
    Some(format!(
        "Exile that many cards from the top of {owner} library. {duration}, you may {verb} those cards"
    ))
}

pub(crate) fn describe_look_at_top_split_hand_bottom_exile_then_play_exiled(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    hand_choose: &crate::effects::ChooseObjectsEffect,
    bottom_choose: &crate::effects::ChooseObjectsEffect,
    exile_choose: &crate::effects::ChooseObjectsEffect,
    hand_move: &crate::effects::MoveToZoneEffect,
    bottom_move: &crate::effects::MoveToZoneEffect,
    exile_move: &crate::effects::MoveToZoneEffect,
    grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    if !looked_card_choice_filter_is_plain_remainder(hand_choose, look_at_top.tag.as_str(), &[])
        || !looked_card_choice_filter_is_plain_remainder(
            bottom_choose,
            look_at_top.tag.as_str(),
            &[hand_choose.tag.as_str()],
        )
        || !looked_card_choice_filter_is_plain_remainder(
            exile_choose,
            look_at_top.tag.as_str(),
            &[hand_choose.tag.as_str(), bottom_choose.tag.as_str()],
        )
        || !tagged_move_to_zone(hand_move, &hand_choose.tag, Zone::Hand, false)
        || !tagged_move_to_zone(bottom_move, &bottom_choose.tag, Zone::Library, false)
        || exile_move.zone != Zone::Exile
        || !matches!(exile_move.target.base(), ChooseSpec::Tagged(tag) if tag == &exile_choose.tag)
        || grant.tag != exile_choose.tag
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || !matches!(grant.player, PlayerFilter::You)
    {
        return None;
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    if singular_count {
        return None;
    }
    let play_verb = if grant.allow_land { "play" } else { "cast" };
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may {play_verb} the exiled card this turn"
    ))
}

pub(super) fn describe_looked_card_split_destinations_structural(
    effects: &[Effect],
) -> Option<String> {
    let [
        look_at_top_effect,
        hand_choose_effect,
        bottom_choose_effect,
        exile_choose_effect,
        hand_move_effect,
        bottom_move_effect,
        exile_move_effect,
        grant_effect,
    ] = effects
    else {
        return None;
    };
    let look_at_top = look_at_top_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let hand_choose = hand_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let bottom_choose =
        bottom_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile_choose = exile_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let hand_move = unwrap_basic_tag_wrappers(hand_move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let bottom_move = unwrap_basic_tag_wrappers(bottom_move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let exile_move = unwrap_basic_tag_wrappers(exile_move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;

    describe_look_at_top_split_hand_bottom_exile_then_play_exiled(
        look_at_top,
        hand_choose,
        bottom_choose,
        exile_choose,
        hand_move,
        bottom_move,
        exile_move,
        grant,
    )
}

pub(crate) fn describe_look_at_top_exile_face_down_then_play_while_exiled(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    exile: &crate::effects::ExileEffect,
    grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    if !exile.face_down
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &look_at_top.tag)
        || grant.tag != look_at_top.tag
        || grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
    {
        return None;
    }

    let owner = if look_at_top.player == PlayerFilter::DamagedPlayer {
        "their".to_string()
    } else {
        describe_possessive_player_filter(&look_at_top.player)
    };
    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    let look_clause = if singular_count {
        format!("Look at the top card of {owner} library")
    } else {
        format!("Look at the top {count_text} {noun} of {owner} library")
    };
    let object_ref = if singular_count { "it" } else { "them" };
    let duration_ref = if singular_count { "it" } else { "they" };
    let cast_ref = if singular_count { "that spell" } else { "them" };
    let player = describe_player_filter(&grant.player);
    let verb = if grant.allow_land { "play" } else { "cast" };
    let mana_suffix = grant
        .mana_spend_cast_clause(cast_ref)
        .map(|clause| format!(", and {clause}"))
        .unwrap_or_default();

    Some(format!(
        "{look_clause}, then exile {object_ref} face down. For as long as {duration_ref} remains exiled, {player} may {verb} {object_ref}{mana_suffix}"
    ))
}

pub(super) fn describe_look_at_top_choose_exile_face_down_rest_bottom_then_play_while_exiled(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
    grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    if look_at_top.reveal
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == look_at_top.tag
        })
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !exile.face_down
        || rest.tag != look_at_top.tag
        || rest.keep_tagged.as_ref() != Some(&choose.tag)
        || rest.player != look_at_top.player
        || grant.tag != choose.tag
        || grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
    {
        return None;
    }

    let singleton_complement = matches!(look_at_top.count.unhinted(), Value::Fixed(2))
        && choose.count.is_single()
        && rest.order == crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses;
    let randomly_ordered_remainder =
        rest.order == crate::effects::consult_helpers::LibraryBottomOrder::Random;
    if !singleton_complement && !randomly_ordered_remainder {
        return None;
    }

    let chosen_ref = if choose.count.is_single() {
        "one of them".to_string()
    } else if choose.count.min == 0 {
        let max = choose.count.max?;
        let count_text = small_number_word(max as u32).unwrap_or_else(|| max.to_string());
        format!("up to {count_text} of them")
    } else if choose.count.max == Some(choose.count.min) {
        let count_text = small_number_word(choose.count.min as u32)
            .unwrap_or_else(|| choose.count.min.to_string());
        format!("{count_text} of them")
    } else {
        return None;
    };

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    if singular_count {
        return None;
    }

    if singleton_complement {
        if grant.player != PlayerFilter::You || !grant.allow_land {
            return None;
        }
        let mana_clause = grant.mana_spend_cast_clause("that spell")?;
        return Some(format!(
            "Look at the top {count_text} {noun} of {owner} library. Exile one of them face down and put the other on the bottom of that library. You may play the exiled card for as long as it remains exiled, and {mana_clause}"
        ));
    }

    let player = describe_player_filter(&grant.player);
    let verb = if grant.allow_land { "play" } else { "cast" };
    let mana_sentence = if grant.allow_any_color_for_cast {
        ". Mana of any type can be spent to cast them"
    } else {
        ""
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library, exile {chosen_ref} face down, then put the rest on the bottom of {owner} library in a random order. {player} may {verb} the exiled cards for as long as they remain exiled{mana_sentence}"
    ))
}

pub(super) fn describe_look_at_top_choose_exile_rest_bottom_play_and_any_mana_while_exiled(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
    may_play: &crate::effects::MayEffect,
    any_mana_grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    let [play_effect] = may_play.effects.as_slice() else {
        return None;
    };
    let play_grant = play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    describe_look_at_top_choose_exile_rest_bottom_play_grants_and_any_mana_while_exiled(
        look_at_top,
        choose,
        exile,
        rest,
        play_grant,
        any_mana_grant,
    )
}

pub(super) fn describe_look_at_top_choose_exile_rest_bottom_play_grants_and_any_mana_while_exiled(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
    play_grant: &crate::effects::GrantPlayTaggedEffect,
    any_mana_grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    let exiled_tag = rest.keep_tagged.as_ref()?;
    if look_at_top.reveal
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == look_at_top.tag
        })
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !exile.face_down
        || rest.tag != look_at_top.tag
        || rest.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
        || rest.player != look_at_top.player
        || play_grant.tag != *exiled_tag
        || any_mana_grant.tag != *exiled_tag
        || play_grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || any_mana_grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || !play_grant.allow_land
        || play_grant.allow_any_color_for_cast
        || any_mana_grant.allow_land
        || !any_mana_grant.allow_any_color_for_cast
    {
        return None;
    }

    let chosen_ref = if choose.count.is_single() {
        "one of them".to_string()
    } else if choose.count.min == 0 {
        let max = choose.count.max?;
        let count_text = small_number_word(max as u32).unwrap_or_else(|| max.to_string());
        format!("up to {count_text} of them")
    } else if choose.count.max == Some(choose.count.min) {
        let count_text = small_number_word(choose.count.min as u32)
            .unwrap_or_else(|| choose.count.min.to_string());
        format!("{count_text} of them")
    } else {
        return None;
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let remainder_owner = if matches!(
        look_at_top.player,
        PlayerFilter::Target(_) | PlayerFilter::DamagedPlayer | PlayerFilter::ChosenPlayer
    ) {
        "their".to_string()
    } else {
        owner.clone()
    };
    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    if singular_count {
        return None;
    }

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library, exile {chosen_ref} face down, then put the rest on the bottom of {remainder_owner} library in a random order. You may play the exiled cards for as long as they remain exiled. Mana of any type can be spent to cast spells this way"
    ))
}

pub(crate) fn describe_look_at_top_then_choose_exile_text(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    face_down: bool,
) -> Option<String> {
    if choose_primary_zone(choose) != Some(Zone::Library)
        || choose.is_search
        || !choose.count.is_single()
    {
        return None;
    }
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked {
        return None;
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    let exile_ref = if singular_count { "it" } else { "one of them" };
    let face_down_suffix = if face_down { " face down" } else { "" };
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library, then exile {exile_ref}{face_down_suffix}"
    ))
}

pub(crate) fn describe_exile_top_of_library(
    player: &PlayerFilter,
    count: &Value,
    face_down: bool,
) -> String {
    let owner = if *player == PlayerFilter::DamagedPlayer {
        "that player's".to_string()
    } else {
        describe_possessive_player_filter(player)
    };
    let face_down_suffix = if face_down { " face down" } else { "" };
    if let Some(count_text) = describe_effect_count_backref(count) {
        return format!(
            "Exile {count_text} cards from the top of {owner} library{face_down_suffix}"
        );
    }
    if let Value::Fixed(n) = count
        && *n >= 0
    {
        let count_u32 = *n as u32;
        // MTG never writes "the top one card" — elide the count word for a single card.
        let (count_text, noun) = if *n == 1 {
            (String::new(), "card")
        } else {
            let word = small_number_word(count_u32).unwrap_or_else(|| n.to_string());
            (format!("{word} "), "cards")
        };
        return format!("Exile the top {count_text}{noun} of {owner} library{face_down_suffix}");
    }

    if let Some(where_x) = describe_where_x_basis(count) {
        return format!(
            "Exile the top X cards of {owner} library{face_down_suffix}, where X is {where_x}"
        );
    }

    let value_text = describe_value(count);
    if value_text == "X" {
        return format!("Exile the top X cards of {owner} library{face_down_suffix}");
    }

    format!(
        "Exile a number of cards from the top of {owner} library equal to {value_text}{face_down_suffix}"
    )
}

pub(crate) fn for_each_reveals_tag(
    for_each: &crate::effects::ForEachTaggedEffect,
    tag: &str,
) -> bool {
    if for_each.tag.as_str() != tag || for_each.effects.len() != 1 {
        return false;
    }
    matches!(
        for_each.effects[0].downcast_ref::<crate::effects::RevealTaggedEffect>(),
        Some(reveal)
            if reveal.tag.as_str() == tag || reveal.tag.as_str() == "__it__"
    )
}

pub(crate) fn for_each_tagged_for_compaction<'a>(
    effect: &'a Effect,
) -> Option<(
    Option<&'a crate::effects::WithIdEffect>,
    &'a crate::effects::ForEachTaggedEffect,
)> {
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return Some((None, for_each));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>()
        && let Some(for_each) = with_id
            .effect
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        return Some((Some(with_id), for_each));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && is_implicit_reference_tag(tagged.tag.as_str())
    {
        return for_each_tagged_for_compaction(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>()
        && is_implicit_reference_tag(tag_all.tag.as_str())
    {
        return for_each_tagged_for_compaction(&tag_all.effect);
    }
    None
}

pub(crate) fn for_each_moves_tag_to_hand(
    for_each: &crate::effects::ForEachTaggedEffect,
    tag: &str,
) -> bool {
    fn iterated_or_tagged(spec: &ChooseSpec, tag: &str) -> bool {
        match spec.base() {
            ChooseSpec::Iterated => true,
            ChooseSpec::Tagged(move_tag) => move_tag.as_str() == tag,
            _ => false,
        }
    }

    if for_each.tag.as_str() != tag || for_each.effects.len() != 1 {
        return false;
    }
    let unwrapped = unwrap_tag_wrapped_effect(&for_each.effects[0]);
    if matches!(
        unwrapped.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Hand
                && iterated_or_tagged(&move_to_zone.target, tag)
    ) {
        return true;
    }
    matches!(
        unwrapped.downcast_ref::<crate::effects::ReturnToHandEffect>(),
        Some(return_to_hand) if iterated_or_tagged(&return_to_hand.spec, tag)
    )
}

pub(crate) fn filter_is_membership_test_for_chosen(
    filter: &crate::filter::ObjectFilter,
    chosen_tag: &str,
) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "__it__"
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::SameStableId
            )
    }) && chosen_tag.len() > 0
}

pub(crate) fn for_each_moves_unselected_to_zone(
    for_each: &crate::effects::ForEachTaggedEffect,
    looked_tag: &str,
    chosen_tag: &str,
    zone: Zone,
) -> bool {
    if for_each.tag.as_str() != looked_tag || for_each.effects.len() != 1 {
        return false;
    }
    let Some(conditional) = for_each.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    if !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return false;
    }
    let Some(move_to_zone) =
        conditional.if_false[0].downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    let moves_iterated_object = matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
        || matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
        );
    if move_to_zone.zone != zone
        || (zone == Zone::Library && move_to_zone.to_top)
        || (zone != Zone::Library && move_to_zone.to_top)
        || !moves_iterated_object
    {
        return false;
    }
    condition_matches_tagged_object_membership(&conditional.condition, chosen_tag)
}

pub(in crate::compiled_text) fn condition_matches_tagged_object_membership(
    condition: &crate::effect::Condition,
    tag: &str,
) -> bool {
    let matches_membership = |condition_tag: &crate::tag::TagKey,
                              filter: &crate::filter::ObjectFilter| {
        if condition_tag.as_str() == tag {
            return filter_is_membership_test_for_chosen(filter, tag);
        }
        condition_tag.as_str() == "__it__"
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == tag
                    && matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                            | crate::filter::TaggedOpbjectRelation::SameStableId
                    )
            })
    };

    match condition {
        crate::effect::Condition::PlayerTaggedObjectMatches {
            tag: condition_tag,
            filter,
            ..
        }
        | crate::effect::Condition::TaggedObjectMatches(condition_tag, filter) => {
            matches_membership(condition_tag, filter)
        }
        _ => false,
    }
}

pub(super) fn effect_moves_iterated_to_zone(effect: &Effect, zone: Zone) -> bool {
    matches!(
        effect.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == zone
                && !move_to_zone.to_top
                && matches!(move_to_zone.target, ChooseSpec::Iterated)
    )
}

pub(super) fn effect_moves_iterated_if_not_tagged_to_zone(
    effect: &Effect,
    excluded_tags: &[&str],
    zone: Zone,
) -> bool {
    let Some((first_tag, remaining_tags)) = excluded_tags.split_first() else {
        return effect_moves_iterated_to_zone(effect, zone);
    };
    let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() else {
        return false;
    };
    conditional.if_true.is_empty()
        && conditional.if_false.len() == 1
        && condition_matches_tagged_object_membership(&conditional.condition, first_tag)
        && effect_moves_iterated_if_not_tagged_to_zone(
            &conditional.if_false[0],
            remaining_tags,
            zone,
        )
}

pub(super) fn for_each_moves_unselected_from_any_to_zone(
    for_each: &crate::effects::ForEachTaggedEffect,
    looked_tag: &str,
    excluded_tags: &[&str],
    zone: Zone,
) -> bool {
    if for_each.tag.as_str() != looked_tag || for_each.effects.len() != 1 {
        return false;
    }
    effect_moves_iterated_if_not_tagged_to_zone(&for_each.effects[0], excluded_tags, zone)
}

pub(crate) fn describe_choose_filter_from_looked_cards(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked || choose.is_search || choose.count.max != Some(1) {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str())
    });
    if base_filter == ObjectFilter::default() {
        return Some(format!(
            "a card{}",
            describe_choice_aggregate_constraint_suffix(choose)
        ));
    }

    let filter_text = base_filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    card_desc = normalize_looked_card_filter_description(&base_filter, &card_desc);
    if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }
    if !card_desc.contains(" card") {
        card_desc = format!("{card_desc} card");
    }

    Some(format!(
        "{}{}",
        with_indefinite_article(&card_desc),
        describe_choice_aggregate_constraint_suffix(choose)
    ))
}

pub(super) fn describe_choice_aggregate_constraint_suffix(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let Some(constraint) = &choose.aggregate_constraint else {
        return String::new();
    };
    let metric = match constraint.metric {
        crate::effect::ChoiceAggregateMetric::Power => "power",
        crate::effect::ChoiceAggregateMetric::Toughness => "toughness",
        crate::effect::ChoiceAggregateMetric::ManaValue => "mana value",
    };
    let maximum = describe_value(&constraint.maximum);
    if matches!(constraint.maximum.unhinted(), Value::Fixed(_)) {
        format!(" with total {metric} {maximum} or less")
    } else {
        format!(" with total {metric} less than or equal to {maximum}")
    }
}

pub(super) fn describe_counted_choose_filter_from_looked_cards(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if choose.count.max == Some(1) {
        return describe_choose_filter_from_looked_cards(look_at_top, choose);
    }
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked || choose.is_search {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str())
    });

    let singular = if base_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        let filter_text = base_filter.description();
        let mut card_desc = filter_text
            .split(" in ")
            .next()
            .unwrap_or(filter_text.as_str())
            .trim()
            .to_string();
        for owner_prefix in [
            "target player's ",
            "that player's ",
            "their ",
            "your ",
            "an opponent's ",
        ] {
            if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
                card_desc = rest.to_string();
                break;
            }
        }
        card_desc = normalize_looked_card_filter_description(&base_filter, &card_desc);
        if let Some(rest) = card_desc.strip_prefix("card ") {
            card_desc = format!("{rest} card");
        }
        if !card_desc.contains(" card") {
            card_desc = format!("{card_desc} card");
        }
        strip_leading_article(&card_desc).to_string()
    };
    let plural = pluralize_noun_phrase(&singular);
    let aggregate_suffix = describe_choice_aggregate_constraint_suffix(choose);

    if choose.count.dynamic_x {
        let count_text =
            if choose.count.up_to_x || choose.search_mode == SearchSelectionMode::Optional {
                "up to X"
            } else {
                "X"
            };
        let where_clause = describe_runtime_choice_where_clause(choose).unwrap_or_default();
        return Some(format!(
            "{count_text} {plural}{aggregate_suffix}{where_clause}"
        ));
    }
    if choose.count.min == 0 {
        let max = choose.count.max?;
        let count_text = small_number_word(max as u32).unwrap_or_else(|| max.to_string());
        return Some(format!("up to {count_text} {plural}{aggregate_suffix}"));
    }
    if choose.count.max == Some(choose.count.min) && choose.count.min > 0 {
        let count_text = small_number_word(choose.count.min as u32)
            .unwrap_or_else(|| choose.count.min.to_string());
        return Some(format!("{count_text} {plural}{aggregate_suffix}"));
    }

    None
}

pub(super) fn describe_any_number_filter_from_looked_cards(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked || choose.is_search || !choose.count.is_any_number() {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str())
    });
    if base_filter == ObjectFilter::default() {
        return Some(format!(
            "cards{}",
            describe_choice_aggregate_constraint_suffix(choose)
        ));
    }
    if let Some(description) = describe_land_or_legendary_permanent_looked_filter(&base_filter) {
        return Some(format!(
            "{description}{}",
            describe_choice_aggregate_constraint_suffix(choose)
        ));
    }

    let filter_text = base_filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    card_desc = normalize_looked_card_filter_description(&base_filter, &card_desc);
    if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }
    if !card_desc.contains(" card") {
        card_desc = format!("{card_desc} card");
    }
    Some(format!(
        "{}{}",
        pluralize_noun_phrase(strip_leading_article(&card_desc)),
        describe_choice_aggregate_constraint_suffix(choose)
    ))
}

pub(super) fn describe_choose_filter_from_looked_cards_with_ignored_tags(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    ignored_tags: &[&str],
) -> Option<String> {
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked || choose.is_search || choose.count.max != Some(1) {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str())
            && !(matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            ) && ignored_tags.contains(&constraint.tag.as_str()))
    });
    if base_filter == ObjectFilter::default() {
        return Some(format!(
            "a card{}",
            describe_choice_aggregate_constraint_suffix(choose)
        ));
    }

    let filter_text = base_filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    card_desc = normalize_looked_card_filter_description(&base_filter, &card_desc);
    if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }
    if !card_desc.contains(" card") {
        card_desc = format!("{card_desc} card");
    }

    Some(format!(
        "{}{}",
        with_indefinite_article(&card_desc),
        describe_choice_aggregate_constraint_suffix(choose)
    ))
}
