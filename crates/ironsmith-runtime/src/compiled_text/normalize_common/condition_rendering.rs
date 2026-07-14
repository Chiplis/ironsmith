use super::*;

pub(crate) fn describe_counter_constraint_phrase(
    counter: crate::filter::CounterConstraint,
) -> String {
    match counter {
        crate::filter::CounterConstraint::Any => "a counter".to_string(),
        crate::filter::CounterConstraint::Typed(counter_type) => {
            with_indefinite_article(&format!("{} counter", describe_counter_type(counter_type)))
        }
    }
}

pub(crate) fn describe_source_matches_keyword_condition(filter: &ObjectFilter) -> Option<String> {
    if filter.static_abilities.len() != 1
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
    {
        return None;
    }

    let label = describe_source_condition_static_ability(filter.static_abilities[0])?;
    let mut subject_filter = filter.clone();
    subject_filter.static_abilities.clear();
    let subject = subject_filter.description();
    let subject = strip_leading_article(&subject);
    if subject == "permanent" || subject == "source" {
        Some(format!("this source has {label}"))
    } else if subject.starts_with("this ") {
        Some(format!("{subject} has {label}"))
    } else {
        Some(format!("this {subject} has {label}"))
    }
}

pub(crate) fn describe_source_condition_static_ability(
    ability_id: crate::static_abilities::StaticAbilityId,
) -> Option<&'static str> {
    use crate::static_abilities::StaticAbilityId::*;
    match ability_id {
        Flying => Some("flying"),
        FirstStrike => Some("first strike"),
        DoubleStrike => Some("double strike"),
        Deathtouch => Some("deathtouch"),
        Defender => Some("defender"),
        Flash => Some("flash"),
        Haste => Some("haste"),
        Hexproof => Some("hexproof"),
        Indestructible => Some("indestructible"),
        Intimidate => Some("intimidate"),
        Lifelink => Some("lifelink"),
        Menace => Some("menace"),
        Reach => Some("reach"),
        Skulk => Some("skulk"),
        Shroud => Some("shroud"),
        Trample => Some("trample"),
        Vigilance => Some("vigilance"),
        Fear => Some("fear"),
        Flanking => Some("flanking"),
        Landwalk => Some("landwalk"),
        Bloodthirst => Some("bloodthirst"),
        Tribute => Some("tribute"),
        Morph => Some("morph"),
        Disguise => Some("disguise"),
        Megamorph => Some("megamorph"),
        Shadow => Some("shadow"),
        Horsemanship => Some("horsemanship"),
        Wither => Some("wither"),
        Infect => Some("infect"),
        Changeling => Some("changeling"),
        _ => None,
    }
}

fn describe_single_basic_land_subtype_choice(filter: &ObjectFilter) -> Option<String> {
    if filter.subtypes.len() < 2
        || !filter.subtypes.iter().all(|subtype| {
            matches!(
                subtype,
                Subtype::Plains
                    | Subtype::Island
                    | Subtype::Swamp
                    | Subtype::Mountain
                    | Subtype::Forest
            )
        })
    {
        return None;
    }

    let mut expected = ObjectFilter::default();
    expected.zone = filter.zone;
    expected.controller = filter.controller.clone();
    expected.subtypes = filter.subtypes.clone();
    if *filter != expected {
        return None;
    }

    let description = filter.description();
    let described = strip_indefinite_article(&description);
    let choices = described
        .split(" or ")
        .map(with_indefinite_article)
        .collect::<Vec<_>>();
    (choices.len() == filter.subtypes.len()).then(|| choices.join(" or "))
}

fn describe_phase_step_value_comparison(
    left: &Value,
    operator: crate::effect::ValueComparisonOperator,
    right: &Value,
) -> Option<String> {
    use crate::effect::ValueComparisonOperator::{Equal, GreaterThanOrEqual};

    if let (Value::LifeTotal(player), GreaterThanOrEqual, Value::Add(starting_total, offset)) =
        (left, operator, right)
        && let (Value::StartingLifeTotal(starting_player), Value::Fixed(offset)) =
            (starting_total.as_ref(), offset.as_ref())
        && player == starting_player
    {
        let subject = describe_player_filter(player);
        return Some(format!(
            "{} {} at least {offset} life more than {} starting life total",
            subject,
            player_verb(&subject, "have", "has"),
            describe_possessive_player_filter(player)
        ));
    }

    if let (Value::CardsInLibrary(player), GreaterThanOrEqual, Value::Fixed(count)) =
        (left, operator, right)
    {
        let subject = describe_player_filter(player);
        return Some(format!(
            "{} {} {count} or more cards in {} library",
            subject,
            player_verb(&subject, "have", "has"),
            describe_possessive_player_filter(player)
        ));
    }

    if let (Value::CardsInHand(player), Equal, Value::Fixed(count)) = (left, operator, right)
        && *count >= 0
    {
        let subject = describe_player_filter(player);
        let count = number_word(*count).unwrap_or_else(|| count.to_string());
        return Some(format!(
            "{} {} exactly {count} cards in {} hand",
            subject,
            player_verb(&subject, "have", "has"),
            describe_possessive_player_filter(player)
        ));
    }

    if let (
        Value::DamageDealtToPlayersThisTurn(PlayerFilter::You),
        GreaterThanOrEqual,
        Value::Fixed(count),
    ) = (left, operator, right)
    {
        return Some(format!("you were dealt {count} or more damage this turn"));
    }

    if let (Value::CardsDiscardedThisTurn(player), GreaterThanOrEqual, Value::Fixed(1)) =
        (left, operator, right)
    {
        let subject = match player {
            PlayerFilter::You => "you".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            _ => describe_player_filter(player),
        };
        return Some(format!("{subject} discarded a card this turn"));
    }

    if let (Value::CountersOn(spec, None), GreaterThanOrEqual, Value::Fixed(count)) =
        (left, operator, right)
        && let ChooseSpec::All(filter) = spec.unhinted()
        && *count >= 0
    {
        let mut objects = filter.clone();
        objects.zone = None;
        let description = objects.description();
        let objects = pluralize_relative_object_phrase(strip_indefinite_article(&description));
        let count = small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
        return Some(format!(
            "there are {count} or more counters among {objects}"
        ));
    }

    if let (Value::Add(first, second), GreaterThanOrEqual, Value::Fixed(count)) =
        (left, operator, right)
        && let (
            Value::Devotion {
                player: first_player,
                color: first_color,
            },
            Value::Devotion {
                player: second_player,
                color: second_color,
            },
        ) = (first.as_ref(), second.as_ref())
        && first_player == second_player
        && *count >= 0
    {
        let count = small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
        return Some(format!(
            "{} devotion to {} and {} is {count} or greater",
            describe_possessive_player_filter(first_player),
            first_color.name(),
            second_color.name()
        ));
    }

    if let (Value::Count(filter), Equal, Value::Fixed(0)) = (left, operator, right)
        && filter.zone == Some(Zone::Battlefield)
    {
        let mut objects = filter.clone();
        objects.zone = None;
        let description = objects.description();
        let objects = pluralize_relative_object_phrase(strip_indefinite_article(&description));
        return Some(format!("there are no {objects} on the battlefield"));
    }

    if let (Value::Count(filter), GreaterThanOrEqual, Value::Fixed(count)) = (left, operator, right)
        && is_source_exiled_count_filter(filter)
        && *count >= 1
    {
        let source = filter
            .source_surface
            .as_ref()
            .map(|surface| surface.display_text())
            .unwrap_or_else(|| "this permanent".to_string());
        if *count == 1 {
            return Some(format!("there are cards exiled with {source}"));
        }
        let count = small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
        return Some(format!(
            "there are {count} or more cards exiled with {source}"
        ));
    }

    None
}

fn describe_history_filter_subject(filter: &ObjectFilter, historical_default: &str) -> String {
    let mut subject_filter = filter.clone();
    subject_filter.zone = None;
    subject_filter.controller = None;
    subject_filter.owner = None;
    let mut subject = describe_for_each_filter(&subject_filter);
    if subject == "permanent" && historical_default == "card" {
        subject = "card".to_string();
    }
    if !subject_filter.subtypes.is_empty()
        && subject_filter.card_types == [CardType::Creature]
        && let Some(stripped) = subject.strip_suffix(" creature")
    {
        subject = stripped.to_string();
    }
    subject
}

fn describe_history_player_subject(player: &PlayerFilter) -> String {
    match player {
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        PlayerFilter::Opponent | PlayerFilter::NotYou => "an opponent".to_string(),
        other => describe_player_filter(other),
    }
}

fn describe_turn_history_value_comparison(
    query: &ironsmith_core::TurnHistoryCount,
    operator: crate::effect::ValueComparisonOperator,
    right: &Value,
) -> Option<String> {
    use crate::effect::ValueComparisonOperator::{Equal, GreaterThan, GreaterThanOrEqual};

    let Value::Fixed(count) = right.unhinted() else {
        return None;
    };
    let count = *count;
    let is_present = matches!(operator, GreaterThan) && count == 0
        || matches!(operator, GreaterThanOrEqual) && count == 1;
    let is_absent = matches!(operator, Equal) && count == 0;
    let at_least = matches!(operator, GreaterThanOrEqual) && count > 0;
    if !is_present && !is_absent && !at_least {
        return None;
    }
    let count_text = small_number_word(count.max(0) as u32).unwrap_or_else(|| count.to_string());

    let quantified_history = |singular: String, plural: String, action: &str| {
        if is_present {
            format!("{} {action} this turn", with_indefinite_article(&singular))
        } else if is_absent {
            format!("no {plural} {action} this turn")
        } else {
            format!("{count_text} or more {plural} {action} this turn")
        }
    };

    match query {
        ironsmith_core::TurnHistoryCount::Died(filter) => {
            let subject = describe_history_filter_subject(filter, "creature");
            let plural = pluralize_relative_object_phrase(&subject);
            let controller = filter.controller.as_ref().map(|controller| {
                format!(
                    " under {} control",
                    describe_possessive_player_filter(controller)
                )
            });
            let controller = controller.as_deref().unwrap_or("");
            if is_present {
                Some(format!(
                    "{} died{controller} this turn",
                    with_indefinite_article(&subject)
                ))
            } else if is_absent {
                Some(format!("no {plural} died{controller} this turn"))
            } else {
                Some(format!(
                    "{count_text} or more {plural} died{controller} this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::EnteredBattlefield(filter) => {
            let subject = describe_history_filter_subject(filter, "permanent");
            let plural = pluralize_relative_object_phrase(&subject);
            let controller = filter.controller.as_ref().map(|controller| {
                format!(
                    " under {} control",
                    describe_possessive_player_filter(controller)
                )
            });
            let controller = controller.as_deref().unwrap_or("");
            let action = format!("entered the battlefield{controller}");
            Some(quantified_history(subject, plural, &action))
        }
        ironsmith_core::TurnHistoryCount::TokensCreated(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} created a token this turn"))
            } else if is_absent {
                Some(format!("{player} didn't create a token this turn"))
            } else {
                Some(format!(
                    "{player} created {count_text} or more tokens this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::PutIntoGraveyard { owner, from } => {
            let graveyard = format!("{} graveyard", describe_possessive_player_filter(owner));
            let origin = match from.as_slice() {
                [] => "from anywhere".to_string(),
                [Zone::Hand, Zone::Library] | [Zone::Library, Zone::Hand] => {
                    "from their hand or library".to_string()
                }
                [zone] => format!("from {}", zone.name()),
                _ => String::new(),
            };
            let origin = origin.trim();
            if is_present {
                Some(format!(
                    "a card was put into {graveyard} {origin} this turn"
                ))
            } else if is_absent {
                Some(format!(
                    "no cards were put into {graveyard} {origin} this turn"
                ))
            } else {
                Some(format!(
                    "{count_text} or more cards were put into {graveyard} {origin} this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::MovedZones { filter, from, to } => {
            let default_subject = if *from == Some(Zone::Battlefield) {
                "permanent"
            } else {
                "card"
            };
            let subject = describe_history_filter_subject(filter, default_subject);
            let plural = pluralize_relative_object_phrase(&subject);
            let owner = filter
                .owner
                .as_ref()
                .map(describe_possessive_player_filter)
                .unwrap_or_else(|| "a player's".to_string());
            let action = match (from, to) {
                (Some(Zone::Graveyard), None) => format!("left {owner} graveyard"),
                (Some(Zone::Battlefield), Some(Zone::Hand)) => {
                    format!("was put into {owner} hand from the battlefield")
                }
                (Some(Zone::Exile), None) => "left exile".to_string(),
                (Some(from), Some(to)) => {
                    format!("was put into {} from {}", to.name(), from.name())
                }
                (Some(from), None) => format!("left {}", from.name()),
                (None, Some(to)) => format!("was put into {}", to.name()),
                (None, None) => "changed zones".to_string(),
            };
            Some(quantified_history(subject, plural, &action))
        }
        ironsmith_core::TurnHistoryCount::Sacrificed { player, filter } => {
            let player = describe_history_player_subject(player);
            let subject = describe_history_filter_subject(filter, "permanent");
            let plural = pluralize_relative_object_phrase(&subject);
            if is_present {
                Some(format!(
                    "{player} sacrificed {} this turn",
                    with_indefinite_article(&subject)
                ))
            } else if is_absent {
                Some(format!("{player} didn't sacrifice a {subject} this turn"))
            } else {
                Some(format!(
                    "{player} sacrificed {count_text} or more {plural} this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::CountersPutOn {
            counter_type,
            filter,
        } => {
            let subject = describe_history_filter_subject(filter, "permanent");
            let counter = counter_type
                .map(|counter_type| format!("{} counter", counter_type.description()))
                .unwrap_or_else(|| "counter".to_string());
            if is_present {
                Some(format!(
                    "a {counter} was put on {} this turn",
                    with_indefinite_article(&subject)
                ))
            } else if is_absent {
                Some(format!(
                    "no {counter}s were put on {} this turn",
                    with_indefinite_article(&subject)
                ))
            } else {
                Some(format!(
                    "{count_text} or more {counter}s were put on {} this turn",
                    with_indefinite_article(&subject)
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::CreaturesAttackedWith { player, filter } => {
            let player = describe_history_player_subject(player);
            let subject = describe_history_filter_subject(filter, "creature");
            let plural = pluralize_relative_object_phrase(&subject);
            if is_present {
                Some(format!(
                    "{player} attacked with {} this turn",
                    with_indefinite_article(&subject)
                ))
            } else if is_absent && matches!(player.as_str(), "a player") && subject == "creature" {
                Some("no creatures attacked this turn".to_string())
            } else if is_absent {
                Some(format!(
                    "{player} didn't attack with {} this turn",
                    with_indefinite_article(&subject)
                ))
            } else {
                Some(format!(
                    "{player} attacked with {count_text} or more {plural} this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::OpponentsAttacked(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} attacked an opponent this turn"))
            } else if is_absent {
                Some(format!("{player} didn't attack an opponent this turn"))
            } else {
                Some(format!(
                    "{player} attacked {count_text} or more opponents this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::PlayersDiscarded(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} discarded a card this turn"))
            } else if is_absent {
                Some(format!("{player} didn't discard a card this turn"))
            } else {
                Some(format!(
                    "{count_text} or more players discarded a card this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::PlayersDealtDamage(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} was dealt damage this turn"))
            } else if is_absent {
                Some(format!("{player} wasn't dealt damage this turn"))
            } else {
                Some(format!(
                    "{count_text} or more players were dealt damage this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::PlayersDealtCombatDamageBy { players, sources } => {
            let player = describe_history_player_subject(players);
            let source = describe_history_filter_subject(sources, "creature");
            if is_present {
                Some(format!(
                    "{player} was dealt combat damage by {} this turn",
                    with_indefinite_article(&source)
                ))
            } else {
                None
            }
        }
        ironsmith_core::TurnHistoryCount::DiscardedOrCycled(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} discarded or cycled a card this turn"))
            } else if is_absent {
                Some(format!("{player} didn't discard or cycle a card this turn"))
            } else {
                Some(format!(
                    "{player} discarded or cycled {count_text} or more cards this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::Cycled(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} cycled a card this turn"))
            } else if is_absent {
                Some(format!("{player} didn't cycle a card this turn"))
            } else {
                Some(format!(
                    "{player} cycled {count_text} or more cards this turn"
                ))
            }
        }
        ironsmith_core::TurnHistoryCount::PlayersLostLife(player) => {
            let player = describe_history_player_subject(player);
            if is_present {
                Some(format!("{player} lost life this turn"))
            } else if is_absent {
                Some(format!("{player} didn't lose life this turn"))
            } else {
                Some(format!("{count_text} or more players lost life this turn"))
            }
        }
        ironsmith_core::TurnHistoryCount::SpellsCast {
            player,
            filter,
            from_zone,
            from_outside_hand,
            exclude_source: _,
            before_triggering_spell,
        } => {
            if *before_triggering_spell {
                return None;
            }
            let player = describe_history_player_subject(player);
            let spell = describe_spell_cast_condition_object(filter);
            let origin = if let Some(zone) = from_zone {
                format!(" from {zone}")
            } else if *from_outside_hand {
                " from anywhere other than their hand".to_string()
            } else {
                String::new()
            };
            if is_present {
                Some(format!("{player} cast {spell}{origin} this turn"))
            } else {
                None
            }
        }
        ironsmith_core::TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(_) => None,
    }
}

fn describe_instant_sorcery_graveyard_threshold(condition: &Condition) -> Option<String> {
    let (filter, threshold) = match condition {
        Condition::CountComparison {
            count: crate::static_abilities::AnthemCountExpression::MatchingFilter(filter),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(threshold),
            ..
        } => (filter, *threshold),
        Condition::ValueComparison {
            left,
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right,
        } => {
            let (Value::Count(filter), Value::Fixed(threshold)) =
                (left.unhinted(), right.unhinted())
            else {
                return None;
            };
            (filter, *threshold)
        }
        _ => return None,
    };

    let expected_filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_type(CardType::Instant)
        .with_type(CardType::Sorcery);
    if *filter != expected_filter {
        return None;
    }

    let threshold = number_word(threshold).unwrap_or_else(|| threshold.to_string());
    Some(format!(
        "there are {threshold} or more instant and/or sorcery cards in your graveyard"
    ))
}

fn describe_each_global_greatest_power_control_condition(
    condition: &Condition,
) -> Option<&'static str> {
    let Condition::ValueComparison {
        left: Value::Count(controlled),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Count(global),
    } = condition
    else {
        return None;
    };

    let global_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
    let mut expected_global = global_creatures.clone();
    expected_global.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
        Value::GreatestPower(global_creatures),
    )));
    let expected_controlled = expected_global.clone().controlled_by(PlayerFilter::You);

    (*global == expected_global && *controlled == expected_controlled)
        .then_some("you control each creature on the battlefield with the greatest power")
}

pub(crate) fn describe_condition(condition: &Condition) -> String {
    if let Some(compact) = describe_happily_ever_after_condition(condition) {
        return compact;
    }
    if let Some(threshold) = describe_instant_sorcery_graveyard_threshold(condition) {
        return threshold;
    }
    if let Some(control) = describe_each_global_greatest_power_control_condition(condition) {
        return control.to_string();
    }

    match condition {
        Condition::YouControl(filter) => {
            // Lieutenant wording: "if you control your commander".
            if filter.is_commander
                && filter.owner == Some(PlayerFilter::You)
                && filter.card_types.is_empty()
                && filter.subtypes.is_empty()
            {
                "you control your commander".to_string()
            } else {
                format!("you control {}", filter.description())
            }
        }
        Condition::OpponentControls(filter) => {
            format!("an opponent controls {}", filter.description())
        }
        Condition::PlayerControls { player, filter } => {
            let subject = describe_player_filter(player);
            if let Some(text) =
                describe_player_controls_only_implicit_tagged_object(player, filter, false)
            {
                return text;
            }
            // Lieutenant wording: "if you control your commander".
            if filter.is_commander
                && matches!(player, PlayerFilter::You)
                && filter.card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.owner.as_ref().is_none_or(|owner| *owner == PlayerFilter::You)
            {
                return "you control your commander".to_string();
            }
            // Lieutenant wording: "if you control your commander".
            if filter.is_commander
                && matches!(player, PlayerFilter::You)
                && filter.card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.owner.as_ref().is_none_or(|owner| *owner == PlayerFilter::You)
            {
                return "you control your commander".to_string();
            }
            if filter.is_commander
                && filter.card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.owner.is_none()
            {
                return format!(
                    "{} {} a commander",
                    subject,
                    player_verb(&subject, "control", "controls")
                );
            }
            if matches!(
                filter.zone,
                Some(
                    Zone::Graveyard
                        | Zone::Hand
                        | Zone::Library
                        | Zone::Exile
                        | Zone::Command
                )
            ) {
                let mut described_filter = filter.clone();
                if described_filter.owner.is_none() {
                    described_filter.owner = Some(player.clone());
                }
                return format!(
                    "there is {}",
                    with_indefinite_article(&described_filter.description())
                );
            }
            if is_owned_player_zone(filter.zone) {
                let object_text = with_indefinite_article(&describe_owned_player_zone_filter(
                    player, filter,
                ));
                return format!(
                    "{} {} {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    object_text
                );
            }
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            if described_filter.could_be_targeted_by.is_some() {
                let described =
                    strip_indefinite_article(&described_filter.description()).to_string();
                let noun = pluralize_noun_phrase(&described);
                return format!(
                    "{} {} one or more {}",
                    subject,
                    player_verb(&subject, "control", "controls"),
                    noun
                );
            }
            let described = with_indefinite_article(strip_indefinite_article(&described_filter.description()));
            format!(
                "{} {} {}",
                subject,
                player_verb(&subject, "control", "controls"),
                described
            )
        }
        Condition::PlayerHasAtLeast {
            player,
            filter,
            count,
        } => {
            let subject = describe_player_filter(player);
            if is_owned_player_zone(filter.zone) {
                let described =
                    strip_leading_article(&describe_owned_player_zone_filter(player, filter))
                        .to_string();
                let noun = pluralize_noun_phrase(&described);
                let count_text = small_number_word(*count)
                    .unwrap_or_else(|| count.to_string());
                return format!(
                    "{} {} {} or more {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    count_text,
                    noun
                );
            }
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            if *count == 1
                && let Some(choice) =
                    describe_single_basic_land_subtype_choice(&described_filter)
            {
                return format!(
                    "{} {} {}",
                    subject,
                    player_verb(&subject, "control", "controls"),
                    choice
                );
            }
            let described = strip_indefinite_article(&described_filter.description()).to_string();
            let noun = pluralize_noun_phrase(&described);
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} {} or more {}",
                subject,
                player_verb(&subject, "control", "controls"),
                count_text,
                noun
            )
        }
        Condition::VoteOptionGetsMoreVotes(option) => {
            format!("{} gets more votes", option.to_ascii_lowercase())
        }
        Condition::SecretChoicesMatch => "the choices match".to_string(),
        Condition::VoteOptionGetsMoreVotesOrTied(option) => format!(
            "{} gets more votes or the vote is tied",
            option.to_ascii_lowercase()
        ),
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let subject = describe_player_filter(player);
            if *count == 0 {
                let mut described_filter = filter.clone();
                if described_filter
                    .controller
                    .as_ref()
                    .is_some_and(|controller| controller == player)
                {
                    described_filter.controller = None;
                }
                let described = described_filter.description();
                let object_text =
                    pluralize_noun_phrase(strip_indefinite_article(&described)).to_string();
                return format!(
                    "{} {} no {}",
                    subject,
                    player_verb(&subject, "control", "controls"),
                    object_text
                );
            }
            if is_owned_player_zone(filter.zone) {
                let described =
                    strip_leading_article(&describe_owned_player_zone_filter(player, filter))
                        .to_string();
                let noun = if *count == 1 {
                    described
                } else {
                    pluralize_noun_phrase(&described)
                };
                let count_text = small_number_word(*count)
                    .unwrap_or_else(|| count.to_string());
                return format!(
                    "{} {} exactly {} {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    count_text,
                    noun
                );
            }
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            let described = strip_indefinite_article(&described_filter.description()).to_string();
            let noun = if *count == 1 {
                described
            } else {
                pluralize_noun_phrase(&described)
            };
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} exactly {} {}",
                subject,
                player_verb(&subject, "control", "controls"),
                count_text,
                noun
            )
        }
        Condition::PlayerHasAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let subject = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            let described = strip_indefinite_article(&described_filter.description()).to_string();
            let noun = pluralize_noun_phrase(&described);
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} {} or more {} with different powers",
                subject,
                player_verb(&subject, "control", "controls"),
                count_text,
                noun
            )
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            let subject = describe_player_filter(player);
            let verb = player_verb(&subject, "control", "controls");
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!(
                "there are {} or more basic land types among lands {} {}",
                count_text, subject, verb
            )
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!(
                "there are {} or more card types among cards in {}",
                count_text,
                describe_card_type_graveyard_scope(player)
            )
        }
        Condition::PlayerControlsMost { player, filter } => {
            let controller = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|filter_controller| filter_controller == player)
            {
                described_filter.controller = None;
            }
            let mut subject = strip_indefinite_article(&described_filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            format!(
                "{} {} the most {}",
                controller,
                player_verb(&controller, "control", "controls"),
                subject
            )
        }
        Condition::PlayerControlsMoreThanEachOtherPlayer { player, filter } => {
            let controller = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|filter_controller| filter_controller == player)
            {
                described_filter.controller = None;
            }
            let mut subject = strip_indefinite_article(&described_filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            format!(
                "{} {} more {} than each other player",
                controller,
                player_verb(&controller, "control", "controls"),
                subject
            )
        }
        Condition::PlayerControlsMoreThanYou { player, filter } => {
            let controller = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|filter_controller| filter_controller == player)
            {
                described_filter.controller = None;
            }
            let mut subject = strip_indefinite_article(&described_filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            format!(
                "{} {} more {} than you",
                controller,
                player_verb(&controller, "control", "controls"),
                subject
            )
        }
        Condition::AnOpponentControlsMoreThanPlayer { player, filter } => {
            let compared_player = describe_player_filter(player);
            let described_filter = filter.clone();
            let mut subject = strip_indefinite_article(&described_filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            if compared_player == "you" {
                format!("an opponent controls more {subject} than you do")
            } else {
                format!("an opponent controls more {subject} than {compared_player} does")
            }
        }
        Condition::AnOpponentHasFewerThanPlayer { player, filter } => {
            let compared_player = describe_player_filter(player);
            let subject = if filter.zone == Some(Zone::Graveyard)
                && filter.card_types == [CardType::Creature]
                && filter.subtypes.is_empty()
            {
                "creature cards in their graveyard".to_string()
            } else {
                let mut subject = strip_indefinite_article(&filter.description()).to_string();
                if !subject.ends_with('s') {
                    subject.push('s');
                }
                subject
            };
            if compared_player == "you" {
                format!("an opponent has fewer {subject} than you do")
            } else {
                format!("an opponent has fewer {subject} than {compared_player} does")
            }
        }
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            let subject = if *player == PlayerFilter::You {
                "your".to_string()
            } else {
                format!("{}'s", describe_player_filter(player))
            };
            format!(
                "{subject} life total is less than or equal to half {} starting life total",
                describe_possessive_player_filter(player)
            )
        }
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            let subject = if *player == PlayerFilter::You {
                "your".to_string()
            } else {
                format!("{}'s", describe_player_filter(player))
            };
            format!(
                "{subject} life total is less than half {} starting life total",
                describe_possessive_player_filter(player)
            )
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            if *player == PlayerFilter::Opponent {
                "you have more life than an opponent".to_string()
            } else {
                format!("{} has less life than you", describe_player_filter(player))
            }
        }
        Condition::PlayerHasMoreLifeThanYou { player } => {
            format!("{} has more life than you", describe_player_filter(player))
        }
        Condition::PlayerHasNoOpponentWithMoreLifeThan { player } => {
            format!(
                "no opponent has more life than {}",
                describe_player_filter(player)
            )
        }
        Condition::PlayerHasMoreLifeThanEachOtherPlayer { player } => {
            if *player == PlayerFilter::You {
                return "you have more life than each opponent".to_string();
            }
            format!(
                "{} has more life than each other player",
                describe_player_filter(player)
            )
        }
        Condition::PlayerIsMonarch { player } => {
            format!("{} is the monarch", describe_player_filter(player))
        }
        Condition::PlayerHasInitiative { player } => {
            let subject = describe_player_filter(player);
            format!("{} {} the initiative", subject, player_verb(&subject, "have", "has"))
        }
        Condition::PlayerHasCitysBlessing { player } => {
            let subject = describe_player_filter(player);
            format!(
                "{} {} the city's blessing",
                subject,
                player_verb(&subject, "have", "has")
            )
        }
        Condition::PlayerCommittedCrimeThisTurn { player } => {
            format!("{} committed a crime this turn", describe_player_filter(player))
        }
        Condition::PlayerRolledResultThisTurn { player, result } => {
            format!("{} rolled a {result} this turn", describe_player_filter(player))
        }
        Condition::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => match dungeon_name {
            Some(name) => format!("{} completed {}", describe_player_filter(player), name),
            None => format!("{} completed a dungeon", describe_player_filter(player)),
        },
        Condition::LifeTotalOrLess(n) => format!("your life total is {n} or less"),
        Condition::LifeTotalOrGreater(n) => format!("your life total is {n} or greater"),
        Condition::CardsInHandOrMore(n) => {
            let count = number_word(*n).unwrap_or_else(|| n.to_string());
            format!("you have {count} or more cards in hand")
        }
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let subject = describe_player_filter(player);
            let count_text = number_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} {} {} or more cards in hand",
                subject,
                player_verb(&subject, "have", "has"),
                count_text
            )
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let subject = describe_player_filter(player);
            if *count == 0 {
                return format!(
                    "{} {} no cards in hand",
                    subject,
                    player_verb(&subject, "have", "has")
                );
            }
            let count_text = number_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} {} {} or fewer cards in hand",
                subject,
                player_verb(&subject, "have", "has"),
                count_text
            )
        }
        Condition::PlayerCardsInHandAtTurnStartOrMore { player, count } => {
            let subject = describe_player_filter(player);
            if *count == 1 {
                return format!(
                    "{} had a card in hand at the beginning of this turn",
                    subject
                );
            }
            let count_text = number_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} had {} or more cards in hand at the beginning of this turn",
                subject, count_text
            )
        }
        Condition::PlayerCardsInHandAtTurnStartOrFewer { player, count } => {
            let subject = describe_player_filter(player);
            if *count == 0 {
                return format!(
                    "{} had no cards in hand at the beginning of this turn",
                    subject
                );
            }
            let count_text = number_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} had {} or fewer cards in hand at the beginning of this turn",
                subject, count_text
            )
        }
        Condition::PlayerHasMoreCardsInHandThanYou { player } => {
            format!(
                "{} has more cards in hand than you",
                describe_player_filter(player)
            )
        }
        Condition::PlayerHasMoreCardsInHandThanEachOtherPlayer { player } => {
            format!(
                "{} has more cards in hand than each other player",
                describe_player_filter(player)
            )
        }
        Condition::PlayerHasPoisonCountersOrMore { player, count } => {
            format!(
                "{} has {} or more poison counters",
                describe_player_filter(player),
                count
            )
        }
        Condition::YouHaveCardInHandMatching(filter) => {
            let object_text = with_indefinite_article(&filter.description());
            format!("you have {object_text} in hand")
        }
        Condition::YourTurn => "it's your turn".to_string(),
        Condition::YourFirstTurnsOfTheGameOrFewer(3) => {
            "it is your first, second, or third turn of the game".to_string()
        }
        Condition::YourFirstTurnsOfTheGameOrFewer(count) => {
            format!("it is one of your first {count} turns of the game")
        }
        Condition::CreatureDiedThisTurn => "one or more creatures died this turn".to_string(),
        Condition::CreatureDiedThisTurnOrMore(count) => {
            format!("{count} or more creatures died this turn")
        }
        Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } => {
            let mut object_filter = victim.clone();
            object_filter.zone = None;
            let object_description = object_filter.description();
            let object = strip_leading_article(&object_description);
            let subject = if *count <= 1 {
                with_indefinite_article(object)
            } else {
                let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
                format!("{count_text} or more {}", pluralize_noun_phrase(object))
            };
            let source = match damager {
                DamagedBySource::ThisCreature => "this creature",
                DamagedBySource::EquippedCreature => "equipped creature",
                DamagedBySource::EnchantedCreature => "enchanted creature",
            };
            format!("{subject} dealt damage by {source} this turn died")
        }
        Condition::CreatureCardPutIntoYourGraveyardThisTurn => {
            "a creature card was put into your graveyard from anywhere this turn".to_string()
        }
        Condition::CastSpellThisTurn => "a spell was cast this turn".to_string(),
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let subject = describe_player_filter(player);
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} cast {} or more spells this turn",
                subject,
                player_verb(&subject, "have", "has"),
                count_text
            )
        }
        Condition::AttackedThisTurn => "you attacked this turn".to_string(),
        Condition::AttackedWithNOrMoreCreaturesThisTurn(count) => format!(
            "you attacked with {} or more creatures this turn",
            number_word(*count as i32).unwrap_or_else(|| count.to_string())
        ),
        Condition::OpponentLostLifeThisTurn => "an opponent lost life this turn".to_string(),
        Condition::AnyPlayerLostLifeThisTurnOrMore { count } => {
            format!("a player lost {count} or more life this turn")
        }
        Condition::OpponentWasDealtDamageThisTurn => {
            "an opponent was dealt damage this turn".to_string()
        }
        Condition::PermanentLeftBattlefieldThisTurn => {
            "a permanent left the battlefield this turn".to_string()
        }
        Condition::NonlandPermanentLeftBattlefieldThisTurn => {
            "a nonland permanent left the battlefield this turn".to_string()
        }
        Condition::SpellWasWarpedThisTurn => "a spell was warped this turn".to_string(),
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn => {
            "a permanent left the battlefield under your control this turn".to_string()
        }
        Condition::ObjectEnteredBattlefieldThisTurn(filter) => {
            let mut object_filter = filter.clone();
            object_filter.zone = None;
            let controller = object_filter.controller.take();
            let object = with_indefinite_article(strip_leading_article(&object_filter.description()));
            if let Some(controller) = controller {
                format!(
                    "{object} entered the battlefield under {} control this turn",
                    describe_possessive_player_filter(&controller)
                )
            } else {
                format!("{object} entered the battlefield this turn")
            }
        }
        Condition::ObjectEnteredBattlefieldLastTurn(filter) => {
            let mut object_filter = filter.clone();
            object_filter.zone = None;
            let controller = object_filter.controller.take();
            let object = with_indefinite_article(strip_leading_article(&object_filter.description()));
            if let Some(controller) = controller {
                format!(
                    "{object} entered the battlefield under {} control last turn",
                    describe_possessive_player_filter(&controller)
                )
            } else {
                format!("{object} entered the battlefield last turn")
            }
        }
        Condition::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter) => {
            let mut object_filter = filter.clone();
            object_filter.zone = None;
            let destination = match object_filter.owner.take() {
                Some(PlayerFilter::You) => "your graveyard",
                _ => "a graveyard",
            };
            let object = with_indefinite_article(strip_leading_article(&object_filter.description()))
                .replace(" you control", " you controlled");
            format!("{object} was put into {destination} from the battlefield this turn")
        }
        Condition::SourceWasCast => "you cast it".to_string(),
        Condition::ThisSpellEscaped => "this spell escaped".to_string(),
        Condition::ThisSpellWasCastFromZone(zone) => {
            if *zone == Zone::Hand {
                return "you cast it from your hand".to_string();
            }
            let zone_text = match zone {
                Zone::Graveyard => "a graveyard".to_string(),
                _ => format!("the {}", zone.name()),
            };
            format!("this spell was cast from {zone_text}")
        }
        Condition::ThisSpellWasCastFromNonHand => {
            "this spell was cast from anywhere other than your hand".to_string()
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            format!(
                "{} tapped a land for mana this turn",
                describe_player_filter(player)
            )
        }
        Condition::PlayerGainedLifeThisTurnOrMore { player, count } => {
            if *count <= 1 {
                format!("{} gained life this turn", describe_player_filter(player))
            } else {
                format!(
                    "{} gained {} or more life this turn",
                    describe_player_filter(player),
                    count
                )
            }
        }
        Condition::SourceIsRingBearer { player } => format!(
            "this creature is {} Ring-bearer",
            describe_possessive_player_filter(player)
        ),
        Condition::PlayerRingTemptedThisGameOrMore { player, count } => {
            let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "the Ring has tempted {} {count_text} or more times this game",
                describe_player_filter(player)
            )
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            format!(
                "{} had a land enter the battlefield under {} control this turn",
                describe_player_filter(player),
                describe_possessive_player_filter(player)
            )
        }
        Condition::PlayerDescendedThisTurn { player } => {
            format!("{} descended this turn", describe_player_filter(player))
        }
        Condition::NoSpellsWereCastLastTurn => "no spells were cast last turn".to_string(),
        Condition::ItIsNight => "it's night".to_string(),
        Condition::FirstCombatPhaseOfTurn => "it's the first combat phase of the turn".to_string(),
        Condition::SpellsWereCastLastTurnOrMore(count) => {
            let count_text = small_number_word(*count)
                .unwrap_or_else(|| count.to_string());
            format!("{count_text} or more spells were cast last turn")
        }
        Condition::TargetIsTapped => "the target is tapped".to_string(),
        Condition::TargetIsBlocked => "the target is blocked".to_string(),
        Condition::TargetWasKicked => "the target spell was kicked".to_string(),
        Condition::ThisSpellWasKicked => "this spell was kicked".to_string(),
        Condition::ThisSpellPaidLabel(label) => {
            let display_label = label.display_label();
            if display_label.eq_ignore_ascii_case("gift")
                || display_label.to_ascii_lowercase().starts_with("gift ")
            {
                return "the gift was promised".to_string();
            }
            if display_label.eq_ignore_ascii_case("bargain") {
                return "this spell was bargained".to_string();
            }
            if let Some(cost) = label.strip_prefix("Kicker ") {
                return format!("it was kicked with its {cost} kicker");
            }
            if label.eq_ignore_ascii_case("tribute") {
                return "tribute was paid".to_string();
            }
            if label.eq_ignore_ascii_case("CastDuringYourMainPhase") {
                return "you cast this spell during your main phase".to_string();
            }
            format!("this spell's {} cost was paid", label.to_ascii_lowercase())
        }
        Condition::YouHaveFullParty => "you have a full party".to_string(),
        Condition::TargetSpellCastOrderThisTurn(2) => {
            "the target spell was the second spell cast this turn".to_string()
        }
        Condition::TargetSpellCastOrderThisTurn(order) => {
            format!("the target spell was spell number {order} cast this turn")
        }
        Condition::TargetSpellControllerIsPoisoned => {
            "the target spell's controller is poisoned".to_string()
        }
        Condition::TargetSpellManaSpentToCastAtLeast { amount, symbol } => {
            let amount_text = small_number_word(*amount).unwrap_or_else(|| amount.to_string());
            if let Some(symbol) = symbol {
                format!(
                    "at least {amount_text} {} mana was spent to cast the target spell",
                    describe_mana_symbol(*symbol)
                )
            } else {
                format!("at least {amount_text} mana was spent to cast the target spell")
            }
        }
        Condition::TriggeringSpellManaSpentToCastAtLeast { amount, symbol } => {
            let amount_text = small_number_word(*amount).unwrap_or_else(|| amount.to_string());
            if let Some(symbol) = symbol {
                if *amount == 1 {
                    format!("{} was spent to cast it", describe_mana_symbol(*symbol))
                } else {
                    format!(
                        "at least {amount_text} {} mana was spent to cast it",
                        describe_mana_symbol(*symbol)
                    )
                }
            } else {
                format!("at least {amount_text} mana was spent to cast it")
            }
        }
        Condition::ColoredManaSpentToCastThisSpellAtLeast(amount) => {
            if *amount == 1 {
                "colored mana was spent to cast this spell".to_string()
            } else {
                format!("at least {amount} colored mana was spent to cast this spell")
            }
        }
        Condition::TriggeringSpellColoredManaSpentToCastAtLeast(amount) => {
            if *amount == 1 {
                "colored mana was spent to cast it".to_string()
            } else {
                format!("at least {amount} colored mana was spent to cast it")
            }
        }
        Condition::YouControlMoreCreaturesThanTargetSpellController => {
            "you control more creatures than the target spell's controller".to_string()
        }
        Condition::TargetHasGreatestPowerAmongCreatures => {
            "the target creature has the greatest power among creatures on the battlefield"
                .to_string()
        }
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {
            "the target's mana value is less than or equal to the number of colors of mana spent to cast this spell".to_string()
        }
        Condition::EnchantedPermanentAttackedThisTurn => {
            "enchanted creature attacked this turn".to_string()
        }
        Condition::SourceIsTapped => "this source is tapped".to_string(),
        Condition::SourceIsSaddled => "this source is saddled".to_string(),
        Condition::SourceCrewedByExactly { count, filter } => {
            let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
            let filter_text = if *count == 1 {
                filter.description()
            } else {
                pluralize_noun_phrase(&filter.description())
            };
            format!(
                "this source was crewed by exactly {count_text} {filter_text}"
            )
        }
        Condition::SourceDevouredCreaturesOrMore(count) => {
            if *count == 1 {
                "this source devoured a creature".to_string()
            } else {
                format!("this source devoured {count} or more creatures")
            }
        }
        Condition::SourceIsFaceDown => "this source is transformed".to_string(),
        Condition::SourceMatches(filter) => {
            let mut entered_this_turn = ObjectFilter::creature();
            entered_this_turn.entered_battlefield_this_turn = true;
            if *filter == entered_this_turn {
                return "this creature entered the battlefield this turn".to_string();
            }
            let mut was_dealt_damage = ObjectFilter::creature();
            was_dealt_damage.was_dealt_damage_this_turn = true;
            if *filter == was_dealt_damage {
                return "this creature was dealt damage this turn".to_string();
            }
            let mut dealt_damage_to_opponent = ObjectFilter::creature();
            dealt_damage_to_opponent.dealt_damage_to_player_this_turn =
                Some(PlayerFilter::Opponent);
            if *filter == dealt_damage_to_opponent {
                return "this creature dealt damage to an opponent this turn".to_string();
            }
            if let Some(text) = describe_source_matches_keyword_condition(filter) {
                return text;
            }
            let desc = filter.description();
            let stripped = strip_leading_article(&desc).to_ascii_lowercase();
            if stripped == "permanent" {
                "this source is a permanent".to_string()
            } else {
                format!("this permanent is {}", ensure_indefinite_article(&desc))
            }
        }
        Condition::AttachedToSourceMatches(filter) => {
            let mut enchanted_creature_power = ObjectFilter::creature();
            enchanted_creature_power.power =
                Some(crate::filter::Comparison::GreaterThanOrEqual(4));
            if *filter == enchanted_creature_power {
                return "enchanted creature's power is 4 or greater".to_string();
            }
            if filter.subtypes.contains(&Subtype::Equipment)
                && filter
                    .attached_to_object
                    .as_deref()
                    .is_some_and(|attached_to| *attached_to == ObjectFilter::creature())
            {
                return "enchanted Equipment is attached to a creature".to_string();
            }
            let desc = filter.description();
            format!(
                "the permanent this source is attached to is {}",
                ensure_indefinite_article(&desc)
            )
        }
        Condition::MatchingObjectAttachedToMatchingObject {
            attachment,
            attached_to,
        } => format!(
            "{} is attached to {}",
            with_indefinite_article(&attachment.description()),
            with_indefinite_article(&attached_to.description())
        ),
        Condition::SourceHasNoCounter(counter_type) => {
            format!("there are no {} counters on it", counter_type.description())
        }
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
            surface,
        } => match surface {
            crate::effect::SourceCounterThresholdSurface::ThereAreOn(source) => {
                let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
                format!(
                    "there are {count_text} or more {} counters on {}",
                    counter_type.description(),
                    source.display_text()
                )
            }
            crate::effect::SourceCounterThresholdSurface::SourceHas => {
                if *count == 1 {
                    if *counter_type == crate::object::CounterType::Luck {
                        format!(
                            "this source has {} on it",
                            with_indefinite_article(&format!(
                                "{} counter",
                                counter_type.description()
                            ))
                        )
                    } else {
                        format!(
                            "this source has one or more {} counters on it",
                            counter_type.description()
                        )
                    }
                } else {
                    format!(
                        "this source has {count} or more {} counters on it",
                        counter_type.description()
                    )
                }
            }
        },
        Condition::SourceHasCountersAtLeast(count) => {
            let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
            format!("there are {count_text} or more counters on it")
        }
        Condition::SourcePowerAtLeast(min_power) => {
            format!("this has power {min_power} or greater")
        }
        Condition::TargetIsAttacking => "the target is attacking".to_string(),
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let amount_text = small_number_word(*amount).unwrap_or_else(|| amount.to_string());
            if let Some(symbol) = symbol {
                if *amount == 1 {
                    format!("{} was spent to cast this spell", describe_mana_symbol(*symbol))
                } else {
                    format!(
                        "at least {amount_text} {} mana was spent to cast this spell",
                        describe_mana_symbol(*symbol)
                    )
                }
            } else {
                format!("at least {amount_text} mana was spent to cast this spell")
            }
        }
        Condition::SnowManaOfAnySpellColorSpentToCastThisSpell => {
            "{S} of any of that spell's colors was spent to cast it".to_string()
        }
        Condition::SameColorManaSpentToCastThisSpellAtLeast(amount) => {
            let amount_text = small_number_word(*amount)
                .unwrap_or_else(|| amount.to_string());
            format!("at least {amount_text} mana of the same color was spent to cast it")
        }
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(amount) => {
            let amount_text = small_number_word(*amount)
                .unwrap_or_else(|| amount.to_string());
            format!("{} or more colors of mana were spent to cast this spell", amount_text)
        }
        Condition::YouControlCommander => "you control a commander".to_string(),
        Condition::TargetObjectsHaveDifferentColorSets => {
            "either target object is a color the other isn't".to_string()
        }
        Condition::TargetMatches(filter) => {
            let desc = filter.description();
            let stripped = strip_leading_article(&desc).to_ascii_lowercase();
            if stripped == "land" {
                "it's a land card".to_string()
            } else if stripped == "creature" {
                "it's a creature".to_string()
            } else {
                format!("the target is {}", ensure_indefinite_article(&desc))
            }
        }
        Condition::TargetIsSoulbondPaired => {
            "the target is paired with another creature".to_string()
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            let mut same_name_comparison_set = filter.clone();
            let before = same_name_comparison_set.tagged_constraints.len();
            same_name_comparison_set
                .tagged_constraints
                .retain(|constraint| {
                    !(constraint.tag.as_str() == "__it__"
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::SameNameAsTagged)
                });
            if same_name_comparison_set.tagged_constraints.len() != before {
                let comparison = same_name_comparison_set.description();
                return format!("it has the same name as {comparison}");
            }
            let desc = filter.description();
            if filter.shares_creature_type_with_source
                && filter.zone.is_none()
                && filter.controller.is_none()
                && filter.owner.is_none()
                && filter.tagged_constraints.is_empty()
            {
                return "it shares a creature type with this creature".to_string();
            }
            if crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled")
                && filter.zone == Some(Zone::Exile)
            {
                return "any of those cards remain exiled".to_string();
            }
            if tag.as_str().starts_with("exiled_")
                && filter.zone.is_none()
                && filter.controller.is_none()
                && filter.owner.is_none()
                && filter.card_types.len() == 1
                && filter.all_card_types.is_empty()
                && filter.excluded_card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.excluded_subtypes.is_empty()
                && filter.supertypes.is_empty()
                && filter.excluded_supertypes.is_empty()
                && filter.colors.is_none()
                && filter.excluded_colors.is_empty()
                && !filter.colorless
                && !filter.multicolored
                && !filter.monocolored
                && filter.all_colors.is_none()
                && filter.exactly_two_colors.is_none()
                && !filter.historic
                && !filter.nonhistoric
                && !filter.token
                && !filter.nontoken
                && filter.face_down.is_none()
                && !filter.other
                && !filter.tapped
                && !filter.untapped
                && !filter.attacking
                && !filter.nonattacking
                && !filter.blocking
                && !filter.nonblocking
                && !filter.blocked
                && !filter.unblocked
                && !filter.entered_since_your_last_turn_ended
                && filter.power.is_none()
                && filter.toughness.is_none()
                && filter.mana_value.is_none()
                && filter.mana_value_eq_counters_on_source.is_none()
                && !filter.has_mana_cost
                && !filter.has_tap_activated_ability
                && !filter.no_abilities
                && !filter.no_x_in_cost
                && !filter.has_x_in_cost
                && filter.with_counter.is_none()
                && filter.without_counter.is_none()
                && filter.name.is_none()
                && filter.excluded_name.is_none()
                && filter.alternative_cast.is_none()
                && filter.static_abilities.is_empty()
                && filter.excluded_static_abilities.is_empty()
                && filter.ability_markers.is_empty()
                && filter.excluded_ability_markers.is_empty()
                && !filter.is_commander
                && !filter.noncommander
                && filter.tagged_constraints.is_empty()
                && filter.specific.is_none()
                && filter.any_of.is_empty()
                && !filter.source
            {
                let card_type = describe_card_type_word_local(filter.card_types[0]);
                return format!("at least one {card_type} card was exiled this way");
            }
            if let Some(condition) = describe_attached_object_color_condition(tag, filter) {
                return condition;
            }
            if let Some(condition) = describe_attached_object_type_condition(tag, filter) {
                return condition;
            }
            if let Some(condition) = describe_sacrifice_cost_object_condition(tag, filter) {
                return condition;
            }
            if tag.as_str().starts_with("countered_")
                && strip_leading_article(&desc)
                    .eq_ignore_ascii_case("permanent")
            {
                return "a permanent's ability is countered this way".to_string();
            }
            if let Some(action) = this_way_action_from_tag(tag)
                && action != "put"
            {
                let object = describe_player_tagged_object_text(tag, filter);
                return if action == "died" {
                    format!("{object} died this way")
                } else {
                    format!("{object} was {action} this way")
                };
            }
            if is_implicit_reference_tag(tag.as_str()) {
                // Keep implicit tags oracle-like: use pronouns rather than exposing tag keys.
                if tag.as_str() == "triggering" && is_aura_only_filter(filter) {
                    return "that enchantment is an Aura".to_string();
                }
                let subject = if matches!(tag.as_str(), "triggering" | "damaged") {
                    "that object"
                } else {
                    "it"
                };
                let card_context = is_generated_internal_tag(tag.as_str())
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("revealed_");
                let is_clause = |noun_phrase: &str| {
                    let phrase = with_indefinite_article(noun_phrase);
                    if subject == "it" {
                        format!("it's {phrase}")
                    } else {
                        format!("{subject} is {phrase}")
                    }
                };

                if let Some(state_clause) =
                    describe_implicit_tagged_object_state_condition(subject, filter)
                {
                    return state_clause;
                }
                if tag.as_str() == "triggering"
                    && let Some(origin_clause) =
                        describe_triggering_graveyard_origin_condition(subject, filter)
                {
                    return origin_clause;
                }
                if let Some(any_of_clause) =
                    describe_implicit_tagged_object_any_of_condition(tag, filter)
                {
                    return any_of_clause;
                }
                if let Some(quality_clause) =
                    describe_implicit_tagged_object_quality_condition(subject, filter)
                {
                    return quality_clause;
                }
                if let Some(pt_clause) =
                    describe_implicit_tagged_object_pt_condition(subject, filter)
                {
                    return pt_clause;
                }

                if !filter.all_card_types.is_empty()
                    && filter.card_types.is_empty()
                    && simple_type_identity_condition_filter(filter)
                {
                    let noun_phrase = if tagged_condition_is_known_card_reference(tag) {
                        format!("{desc} card")
                    } else {
                        desc.clone()
                    };
                    return is_clause(&noun_phrase);
                }

                if card_context
                    && !filter.card_types.is_empty()
                    && filter.zone.is_none()
                    && filter.controller.is_none()
                    && filter.owner.is_none()
                    && !filter.single_graveyard
                    && filter.targets_player.is_none()
                    && filter.targets_object.is_none()
                    && !filter.targets_any_of
                    && filter.all_card_types.is_empty()
                    && filter.excluded_card_types.is_empty()
                    && filter.subtypes.is_empty()
                    && !filter.type_or_subtype_union
                    && filter.excluded_subtypes.is_empty()
                    && filter.supertypes.is_empty()
                    && filter.excluded_supertypes.is_empty()
                    && filter.colors.is_none()
                    && filter.excluded_colors.is_empty()
                    && !filter.colorless
                    && !filter.multicolored
                    && !filter.monocolored
                    && filter.all_colors.is_none()
                    && filter.exactly_two_colors.is_none()
                    && !filter.historic
                    && !filter.nonhistoric
                    && !filter.token
                    && !filter.nontoken
                    && filter.face_down.is_none()
                    && !filter.other
                    && !filter.tapped
                    && !filter.untapped
                    && !filter.attacking
                    && !filter.nonattacking
                    && !filter.blocking
                    && !filter.nonblocking
                    && !filter.blocked
                    && !filter.unblocked
                    && !filter.entered_since_your_last_turn_ended
                    && filter.power.is_none()
                    && filter.toughness.is_none()
                    && filter.mana_value.is_none()
                    && filter.mana_value_eq_counters_on_source.is_none()
                    && !filter.has_mana_cost
                    && !filter.has_tap_activated_ability
                    && !filter.no_abilities
                    && !filter.no_x_in_cost
                    && !filter.has_x_in_cost
                    && filter.with_counter.is_none()
                    && filter.without_counter.is_none()
                    && filter.name.is_none()
                    && filter.excluded_name.is_none()
                    && filter.alternative_cast.is_none()
                    && filter.static_abilities.is_empty()
                    && filter.excluded_static_abilities.is_empty()
                    && filter.ability_markers.is_empty()
                    && filter.excluded_ability_markers.is_empty()
                    && !filter.is_commander
                    && !filter.noncommander
                    && filter.tagged_constraints.is_empty()
                    && filter.specific.is_none()
                    && filter.any_of.is_empty()
                    && !filter.source
                {
                    let words = filter
                        .card_types
                        .iter()
                        .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                        .collect::<Vec<_>>();
                    let noun_phrase = format!("{} card", join_with_or(&words));
                    return is_clause(&noun_phrase);
                }

                let stripped = strip_leading_article(&desc).to_ascii_lowercase();
                if matches!(filter.controller, Some(PlayerFilter::Opponent))
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Creature
                    && filter.zone.is_none()
                    && filter.owner.is_none()
                    && filter.subtypes.is_empty()
                    && filter.excluded_card_types.is_empty()
                    && filter.excluded_subtypes.is_empty()
                    && filter.colors.is_none()
                    && filter.tagged_constraints.is_empty()
                    && filter.any_of.is_empty()
                    && !filter.source
                {
                    return "an opponent controls that creature".to_string();
                }
                if filter.power.is_some()
                    && filter.zone.is_none()
                    && filter.controller.is_none()
                    && filter.owner.is_none()
                    && filter.single_graveyard == false
                    && filter.card_types.is_empty()
                    && filter.all_card_types.is_empty()
                    && filter.excluded_card_types.is_empty()
                    && filter.subtypes.is_empty()
                    && filter.excluded_subtypes.is_empty()
                    && filter.supertypes.is_empty()
                    && filter.excluded_supertypes.is_empty()
                    && filter.colors.is_none()
                    && filter.excluded_colors.is_empty()
                    && filter.toughness.is_none()
                    && filter.total_power_toughness.is_none()
                    && filter.mana_value.is_none()
                    && filter.name.is_none()
                    && filter.tagged_constraints.is_empty()
                    && filter.any_of.is_empty()
                    && !filter.source
                {
                    let comparison = match filter.power.as_ref().unwrap() {
                        ironsmith_core::FilterComparison::GreaterThan(n) => {
                            format!("is greater than {n}")
                        }
                        ironsmith_core::FilterComparison::GreaterThanOrEqual(n) => {
                            format!("is {n} or greater")
                        }
                        ironsmith_core::FilterComparison::Equal(n) => format!("is {n}"),
                        ironsmith_core::FilterComparison::LessThan(n) => {
                            format!("is less than {n}")
                        }
                        ironsmith_core::FilterComparison::LessThanOrEqual(n) => {
                            format!("is {n} or less")
                        }
                        ironsmith_core::FilterComparison::NotEqual(n) => format!("is not {n}"),
                        ironsmith_core::FilterComparison::LessThanExpr(value) => {
                            format!("is less than {}", describe_value(value))
                        }
                        ironsmith_core::FilterComparison::LessThanOrEqualExpr(value) => {
                            format!("is less than or equal to {}", describe_value(value))
                        }
                        ironsmith_core::FilterComparison::GreaterThanExpr(value) => {
                            format!("is greater than {}", describe_value(value))
                        }
                        ironsmith_core::FilterComparison::GreaterThanOrEqualExpr(value) => {
                            format!("is greater than or equal to {}", describe_value(value))
                        }
                        ironsmith_core::FilterComparison::EqualExpr(value) => {
                            format!("is equal to {}", describe_value(value))
                        }
                        ironsmith_core::FilterComparison::NotEqualExpr(value) => {
                            format!("is not equal to {}", describe_value(value))
                        }
                        other => format!("matches {:?}", other).to_ascii_lowercase(),
                    };
                    let possessive = if subject == "it" {
                        "its"
                    } else {
                        "that object's"
                    };
                    return format!("{possessive} power {comparison}");
                }
                let mut without_mana_value = filter.clone();
                without_mana_value.mana_value = None;
                if card_context
                    && without_mana_value == ObjectFilter::permanent_card()
                    && let Some((_, rest)) = stripped.split_once(" with mana value ")
                {
                    return if subject == "it" {
                        format!("it's a permanent card with mana value {}", rest.trim())
                    } else {
                        format!(
                            "{subject} is a permanent card with mana value {}",
                            rest.trim()
                        )
                    };
                }
                if let Some((_, rest)) = stripped.split_once(" with mana value ") {
                    let possessive = if subject == "it" {
                        "its"
                    } else {
                        "that object's"
                    };
                    return format!("{possessive} mana value is {}", rest.trim());
                }
                if stripped == "land" {
                    let noun = if card_context { "land card" } else { "land" };
                    return is_clause(noun);
                }
                if stripped == "permanent" {
                    return if subject == "it" {
                        "it's a permanent".to_string()
                    } else {
                        "that object is a permanent".to_string()
                    };
                }
                if stripped == "creature" {
                    let noun = if card_context {
                        "creature card"
                    } else {
                        "creature"
                    };
                    return is_clause(noun);
                }
                if matches!(
                    stripped.as_str(),
                    "enchanted permanent"
                        | "enchanted creature"
                        | "enchanted enchantment"
                        | "enchanted artifact"
                        | "equipped permanent"
                        | "equipped creature"
                ) && let Some((quality, _)) = stripped.split_once(' ')
                {
                    return if subject == "it" {
                        format!("it's {quality}")
                    } else {
                        format!("{subject} is {quality}")
                    };
                }
                if filter.subtypes.len() == 1
                    && filter.zone.is_none()
                    && filter.controller.is_none()
                    && filter.owner.is_none()
                    && !filter.single_graveyard
                    && filter.card_types.is_empty()
                    && filter.all_card_types.is_empty()
                    && filter.excluded_card_types.is_empty()
                    && filter.excluded_subtypes.is_empty()
                    && filter.supertypes.is_empty()
                    && filter.excluded_supertypes.is_empty()
                    && filter.colors.is_none()
                    && filter.excluded_colors.is_empty()
                    && !filter.colorless
                    && !filter.multicolored
                    && !filter.monocolored
                    && filter.all_colors.is_none()
                    && filter.exactly_two_colors.is_none()
                    && filter.power.is_none()
                    && filter.toughness.is_none()
                    && filter.total_power_toughness.is_none()
                    && filter.mana_value.is_none()
                    && filter.with_counter.is_none()
                    && filter.without_counter.is_none()
                    && filter.name.is_none()
                    && filter.excluded_name.is_none()
                    && filter.tagged_constraints.is_empty()
                    && filter.any_of.is_empty()
                    && !filter.source
                {
                    let subtype = filter.subtypes[0].to_string();
                    return is_clause(&subtype);
                }
                return describe_implicit_tagged_object_fallback_condition(subject, &desc);
                }
                format!("the tagged object '{}' matches {desc}", tag.as_str())
            }
        Condition::TaggedObjectMatchedLastKnown(tag, filter) => {
            describe_last_known_tagged_object_condition(tag, filter)
        }
        Condition::TaggedObjectIsTopOfLibrary { tag, .. } => {
            if is_implicit_reference_tag(tag.as_str()) {
                "it remains on top of its owner's library".to_string()
            } else {
                format!(
                    "the tagged object '{}' remains on top of its owner's library",
                    tag.as_str()
                )
            }
        }
        Condition::StableObjectIsTopOfLibrary { .. } => {
            "that card remains on top of its owner's library".to_string()
        }
        Condition::TaggedObjectWasCast(tag) => {
            if is_implicit_reference_tag(tag.as_str()) {
                "it was cast".to_string()
            } else {
                format!("the tagged object '{}' was cast", tag.as_str())
            }
        }
        Condition::TaggedObjectIsSoulbondPaired(tag) => {
            if is_implicit_reference_tag(tag.as_str()) {
                "it's paired with another creature".to_string()
            } else {
                format!(
                    "the tagged object '{}' is paired with another creature",
                    tag.as_str()
                )
            }
        }
        Condition::PlayerTaggedObjectMatches { player, tag, filter } => {
            if let Some(action) = tag_action_from_name(tag.as_str()) {
                let object_text = describe_player_tagged_object_text(tag, filter);
                let destination = if action == "put" && filter.zone == Some(Zone::Battlefield) {
                    " onto the battlefield"
                } else {
                    ""
                };
                format!(
                    "{} {} {}{} this way",
                    describe_player_filter(player),
                    action,
                    object_text,
                    destination
                )
            } else {
                format!(
                    "{} had the tagged object '{}' matching {}",
                    describe_player_filter(player),
                    tag.as_str(),
                    filter.description()
                )
            }
        }
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            if let Some(action) = tag_action_from_name(tag.as_str()) {
                format!("{} {} it this way", describe_player_filter(player), action)
            } else if is_implicit_reference_tag(tag.as_str()) {
                // "If it entered under your control, ..." (Hallowed Respite,
                // Phelia, Exuberant Shepherd).
                format!(
                    "it entered under {} control",
                    describe_possessive_player_filter(player)
                )
            } else {
                format!(
                    "{} had the tagged object '{}' enter the battlefield under their control this turn",
                    describe_player_filter(player),
                    tag.as_str()
                )
            }
        }
        Condition::PlayerOwnsCardNamedInZones { player, name, zones } => {
            let subject = describe_player_filter(player);
            let possessive = describe_possessive_player_filter(player);
            let mut zone_phrases = Vec::new();
            for zone in zones {
                match zone {
                    Zone::Exile => zone_phrases.push("in exile".to_string()),
                    Zone::Hand => zone_phrases.push(format!("in {possessive} hand")),
                    Zone::Graveyard => zone_phrases.push(format!("in {possessive} graveyard")),
                    Zone::Library => zone_phrases.push(format!("in {possessive} library")),
                    Zone::Battlefield => zone_phrases.push("on the battlefield".to_string()),
                    Zone::Stack => zone_phrases.push("on the stack".to_string()),
                    Zone::Command => zone_phrases.push("in the command zone".to_string()),
                    Zone::OutsideGame => zone_phrases.push("outside the game".to_string()),
                }
            }
            let zones_text = join_with_and(&zone_phrases);
            format!(
                "{} {} a card named {} {}",
                subject,
                player_verb(&subject, "own", "owns"),
                name,
                zones_text
            )
        }
        Condition::FirstTimeThisTurn => "this is the first time this ability triggered this turn"
            .to_string(),
        Condition::SourceFirstCrewedThisTurn => {
            "this is the first time this source was crewed this turn".to_string()
        }
        Condition::ThisAbilityResolvedThisTurnExactly(count) => format!(
            "this is the {} time this ability has resolved this turn",
            ordinal_number_word(*count)
        ),
        Condition::MaxTimesEachTurn(limit) => {
            format!("this ability has triggered fewer than {limit} times this turn")
        }
        Condition::DoThisMaxTimesEachTurn(limit) => {
            format!("this effect has been used fewer than {limit} times this turn")
        }
        Condition::TriggeringObjectWasEnchanted => "the triggering object was enchanted".to_string(),
        Condition::TriggeringObjectHadToAttackThisCombat => {
            "that creature had to attack this combat".to_string()
        }
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => format!(
            "the triggering object had {min_count} or more {} counters",
            counter_type.description()
        ),
        Condition::ControlCreaturesTotalPowerAtLeast(power) => format!(
            "creatures you control have total power {power} or greater"
        ),
        Condition::CardInYourGraveyard { card_types, subtypes } => {
            if card_types.is_empty() && subtypes.is_empty() {
                "there is a card in your graveyard".to_string()
            } else if subtypes.is_empty() {
                let types = card_types
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                format!("there is a {} card in your graveyard", join_with_or(&types))
            } else if card_types.is_empty() {
                let types = subtypes
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                format!("there is an {} card in your graveyard", join_with_or(&types))
            } else {
                let card_types = card_types
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                let subtypes = subtypes
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                format!(
                    "there is a {} {} card in your graveyard",
                    join_with_or(&subtypes),
                    join_with_or(&card_types)
                )
            }
        }
        Condition::SourceIsInZone(zone) => match zone {
            Zone::Hand => "this card is in your hand".to_string(),
            Zone::Graveyard => "this card is in your graveyard".to_string(),
            Zone::Library => "this card is in your library".to_string(),
            Zone::Exile => "this card is in exile".to_string(),
            Zone::Command => "this card is in the command zone".to_string(),
            Zone::OutsideGame => "this card is outside the game".to_string(),
            Zone::Battlefield => "this object is on the battlefield".to_string(),
            Zone::Stack => "this object is on the stack".to_string(),
        },
        Condition::ActivationTiming(timing) => {
            let label = match timing {
                crate::ability::ActivationTiming::AnyTime => "any time",
                crate::ability::ActivationTiming::SorcerySpeed => "sorcery speed",
                crate::ability::ActivationTiming::DuringCombat => "during combat",
                crate::ability::ActivationTiming::OncePerTurn => "once per turn",
                crate::ability::ActivationTiming::DuringYourTurn => "during your turn",
                crate::ability::ActivationTiming::DuringOpponentsTurn => "during opponents' turns",
            };
            format!("timing restriction: {label}")
        }
        Condition::MaxActivationsPerTurn(limit) => {
            format!("this ability has been activated fewer than {limit} times this turn")
        }
        Condition::SourceIsEquipped => "this permanent is equipped".to_string(),
        Condition::SourceIsEnchanted => "this permanent is enchanted".to_string(),
        Condition::SourceIsMonstrous => "this permanent is monstrous".to_string(),
        Condition::EnchantedPermanentIsCreature => {
            "enchanted permanent is a creature".to_string()
        }
        Condition::EnchantedPermanentIsLand => "enchanted permanent is a land".to_string(),
        Condition::EnchantedPermanentIsEquipment => {
            "enchanted permanent is an equipment".to_string()
        }
        Condition::EnchantedPermanentIsVehicle => {
            "enchanted permanent is a vehicle".to_string()
        }
        Condition::EquippedCreatureTapped => "equipped creature is tapped".to_string(),
        Condition::EquippedCreatureUntapped => "equipped creature is untapped".to_string(),
        Condition::EquippedCreatureAttacking => "equipped creature is attacking".to_string(),
        Condition::CountComparison { display, .. } => display
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "count comparison".to_string()),
        Condition::CountParity { display, even, .. } => display
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("count is {}", if *even { "even" } else { "odd" })),
        Condition::ValueComparison {
            left,
            operator,
            right,
        } => {
            if let (
                Value::ManaFromSourceSpentToCastThisSpell {
                    source_filter,
                    include_source_noun,
                },
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(1),
            ) = (left.unhinted(), operator, right.unhinted())
            {
                let mut source = source_filter.description();
                if *include_source_noun {
                    source.push_str(" source");
                }
                return format!("mana from {source} was spent to cast it");
            }
            if let Value::TurnHistoryCount(query) = left.unhinted()
                && let Some(rendered) =
                    describe_turn_history_value_comparison(query, *operator, right)
            {
                return rendered;
            }
            if let Some(rendered) =
                describe_phase_step_value_comparison(left, *operator, right)
            {
                return rendered;
            }
            if let (
                Value::Count(filter),
                crate::effect::ValueComparisonOperator::Equal,
                Value::Fixed(0),
            ) = (left, operator, right)
                && *filter == ObjectFilter::creature().in_zone(Zone::Battlefield)
            {
                return "no creatures are on the battlefield".to_string();
            }
            if let Some(rendered) = describe_happily_value_comparison(left, *operator, right) {
                return rendered;
            }
            if let (
                Value::Speed(PlayerFilter::You),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(4),
            ) = (left, operator, right)
            {
                return "you have max speed".to_string();
            }
            if let (
                Value::LifeLostThisTurn(player),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
            {
                return format!(
                    "{} lost {} or more life this turn",
                    describe_player_filter(player),
                    count
                );
            }
            if let (
                Value::CreaturesDiedThisTurnControlledBy(player),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(1),
            ) = (left, operator, right)
            {
                return format!(
                    "a creature died under {} control this turn",
                    describe_possessive_player_filter(player)
                );
            }
            if let (
                Value::LifeTotal(player),
                crate::effect::ValueComparisonOperator::Equal,
                Value::Fixed(count),
            ) = (left, operator, right)
            {
                let subject = describe_player_filter(player);
                return format!(
                    "{} {} exactly {count} life",
                    subject,
                    player_verb(&subject, "have", "has")
                );
            }
            if let (
                Value::LifeTotal(player),
                crate::effect::ValueComparisonOperator::LessThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
            {
                let subject = describe_player_filter(player);
                return format!(
                    "{} {} {count} or less life",
                    subject,
                    player_verb(&subject, "have", "has")
                );
            }
            if let (
                Value::CardsInGraveyard(player),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
            {
                let subject = describe_player_filter(player);
                let count_text = small_number_word(*count as u32)
                    .unwrap_or_else(|| count.to_string());
                let graveyard = match player {
                    PlayerFilter::You => "your graveyard".to_string(),
                    PlayerFilter::Opponent | PlayerFilter::Any | PlayerFilter::NotYou => {
                        "their graveyard".to_string()
                    }
                    _ => format!(
                        "{} graveyard",
                        describe_possessive_graveyard_owner_filter(player)
                    ),
                };
                return format!(
                    "{} {} {} or more cards in {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    count_text,
                    graveyard
                );
            }
            if let (
                Value::MaxCardsDrawnThisTurn(player),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
                && *count >= 0
            {
                let count_text =
                    small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
                let subject = match player {
                    PlayerFilter::You => "you've".to_string(),
                    PlayerFilter::Opponent | PlayerFilter::NotYou => "an opponent has".to_string(),
                    PlayerFilter::Any => "a player has".to_string(),
                    _ => {
                        let described = describe_player_filter(player);
                        format!("{} {}", described, player_verb(&described, "have", "has"))
                    }
                };
                return format!("{subject} drawn {count_text} or more cards this turn");
            }
            if let (
                Value::CountersOn(spec, Some(counter_type)),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
                && *count >= 0
            {
                let count_text =
                    small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
                return format!(
                    "{} has {} or more {} counters on it",
                    describe_choose_spec(spec),
                    count_text,
                    counter_type.description()
                );
            }
            if let (
                Value::Count(filter),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
                && is_source_exiled_count_filter(filter)
            {
                let count_text = small_number_word(*count as u32)
                    .unwrap_or_else(|| count.to_string());
                return format!("{count_text} or more cards have been exiled with this permanent");
            }
            if let (
                Value::Count(filter),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
                && filter.zone == Some(Zone::Graveyard)
            {
                let count_text = small_number_word(*count as u32)
                    .unwrap_or_else(|| count.to_string());
                let subject = describe_count_filter_value_subject(filter);
                return format!("there are {} or more {}", count_text, subject);
            }
            if let (
                Value::Count(filter),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(count),
            ) = (left, operator, right)
                && filter.zone == Some(Zone::Battlefield)
            {
                let count_text = small_number_word(*count as u32)
                    .unwrap_or_else(|| count.to_string());
                if let Some(counter) = filter.with_counter {
                    let mut subject_filter = filter.clone();
                    subject_filter.with_counter = None;
                    subject_filter.zone = None;
                    let subject =
                        strip_indefinite_article(&subject_filter.description()).to_string();
                    let noun = pluralize_relative_object_phrase(&subject);
                    return format!(
                        "{count_text} or more {noun} have {} on them",
                        describe_counter_constraint_phrase(counter)
                    );
                }
                let subject = strip_indefinite_article(&filter.description()).to_string();
                let noun = pluralize_relative_object_phrase(&subject);
                return format!("there are {} or more {} on the battlefield", count_text, noun);
            }
            if let (
                Value::Count(filter),
                crate::effect::ValueComparisonOperator::GreaterThan,
                Value::Fixed(0),
            ) = (left, operator, right)
                && filter.zone == Some(Zone::Battlefield)
            {
                let mut described_filter = filter.clone();
                described_filter.zone = None;
                let object_text = described_filter.description();
                let object_text = if object_text.starts_with("a ")
                    || object_text.starts_with("an ")
                    || object_text.starts_with("the ")
                {
                    object_text
                } else {
                    with_indefinite_article(&object_text)
                };
                return format!("{object_text} is on the battlefield");
            }
            if let (
                Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source: false,
                },
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                Value::Fixed(1),
            ) = (left, operator, right)
            {
                let subject = describe_player_filter(player);
                return format!(
                    "{} {} cast {} this turn",
                    subject,
                    player_verb(&subject, "have", "has"),
                    describe_spell_cast_condition_object(filter)
                );
            }
            let operator_text = match operator {
                crate::effect::ValueComparisonOperator::GreaterThan => "is greater than",
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                    "is greater than or equal to"
                }
                crate::effect::ValueComparisonOperator::Equal => "is equal to",
                crate::effect::ValueComparisonOperator::LessThan => "is less than",
                crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                    "is less than or equal to"
                }
                crate::effect::ValueComparisonOperator::NotEqual => "is not equal to",
            };
            format!(
                "{} {} {}",
                describe_value(left),
                operator_text,
                describe_value(right)
            )
        }
        Condition::OwnsCardExiledWithCounter(counter) => format!(
            "you own a card in exile with a {} counter on it",
            counter.description()
        ),
        Condition::SourceAttackedThisTurn => "this creature attacked this turn".to_string(),
        Condition::SourceSuspected => "this creature is suspected".to_string(),
        Condition::SourceDealtCombatDamageToPlayerThisTurn => {
            "it dealt combat damage to a player this turn".to_string()
        }
        Condition::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype } => {
            let player_text = match player {
                PlayerFilter::Any => "a player".to_string(),
                PlayerFilter::Opponent => "an opponent".to_string(),
                _ => describe_player_filter(player),
            };
            format!(
                "{player_text} was dealt combat damage by a {subtype} this turn"
            )
        }
        Condition::SourceCameUnderYourControlThisTurn => {
            "this creature came under your control this turn".to_string()
        }
        Condition::SourceAttackedOrBlockedThisTurn => {
            "this creature attacked or blocked this turn".to_string()
        }
        Condition::SourceChosenOption(option) => {
            format!("the chosen option is {}", option)
        }
        Condition::SourceIsUntapped => "this source is untapped".to_string(),
        Condition::SourceIsAttacking => "this source is attacking".to_string(),
        Condition::SourceIsBlocking => "this source is blocking".to_string(),
        Condition::SourceIsSoulbondPaired => {
            "this creature is paired with another creature".to_string()
        }
        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => {
            format!("{player:?}'s graveyard has {count} or more cards")
        }
        Condition::XValueAtLeast(min) => format!("X is {min} or more"),
        Condition::Custom(id) => match *id {
            "you_would_proliferate" => "you would proliferate".to_string(),
            "opponent_would_proliferate" => "an opponent would proliferate".to_string(),
            "player_would_proliferate" => "a player would proliferate".to_string(),
            "opponent_would_begin_extra_turn" => {
                "an opponent would begin an extra turn".to_string()
            }
            "player_would_begin_extra_turn" => "a player would begin an extra turn".to_string(),
            _ => format!("custom condition {id}"),
        },
        Condition::Not(inner) => {
            if matches!(
                inner.as_ref(),
                Condition::TargetObjectsHaveDifferentColorSets
            ) {
                "the target objects have the same color set".to_string()
            } else if let Condition::TargetSpellManaSpentToCastAtLeast {
                amount: 1,
                symbol: None,
            } = inner.as_ref()
            {
                "no mana was spent to cast the target spell".to_string()
            } else if let Condition::TriggeringSpellManaSpentToCastAtLeast {
                amount: 1,
                symbol: None,
            } = inner.as_ref()
            {
                "no mana was spent to cast it".to_string()
            } else if let Condition::ColoredManaSpentToCastThisSpellAtLeast(1) = inner.as_ref() {
                "no colored mana was spent to cast this spell".to_string()
            } else if let Condition::TriggeringSpellColoredManaSpentToCastAtLeast(1) =
                inner.as_ref()
            {
                "no colored mana was spent to cast it".to_string()
            } else if let Condition::SourceIsTapped = inner.as_ref() {
                "this source is untapped".to_string()
            } else if let Condition::YourTurn = inner.as_ref() {
                "it is not your turn".to_string()
            } else if let Condition::PermanentLeftBattlefieldThisTurn = inner.as_ref() {
                "no permanents left the battlefield this turn".to_string()
            } else if let Condition::NonlandPermanentLeftBattlefieldThisTurn = inner.as_ref() {
                "no nonland permanents left the battlefield this turn".to_string()
            } else if let Condition::SpellWasWarpedThisTurn = inner.as_ref() {
                "no spells were warped this turn".to_string()
            } else if let Condition::SourceMatches(filter) = inner.as_ref() {
                let mut entered_this_turn = ObjectFilter::creature();
                entered_this_turn.entered_battlefield_this_turn = true;
                if *filter == entered_this_turn {
                    "this creature didn't enter the battlefield this turn".to_string()
                } else if *filter == ObjectFilter::creature() {
                    "this permanent isn't a creature".to_string()
                } else {
                    format!("not ({})", describe_condition(inner))
                }
            } else if let Condition::ThisSpellWasKicked = inner.as_ref() {
                "this creature wasn't kicked".to_string()
            } else if let Condition::CreatureDiedThisTurn = inner.as_ref() {
                "no creatures died this turn".to_string()
            } else if let Condition::PlayerIsMonarch {
                player: PlayerFilter::Any,
            } = inner.as_ref()
            {
                "there is no monarch".to_string()
            } else if let Condition::SourceAttackedThisTurn = inner.as_ref() {
                "this creature didn't attack this turn".to_string()
            } else if let Condition::AttackedThisTurn = inner.as_ref() {
                "you didn't attack with a creature this turn".to_string()
            } else if let Condition::CardsInHandOrMore(1) = inner.as_ref() {
                "you have no cards in hand".to_string()
            } else if let Condition::ThisSpellPaidLabel(label) = inner.as_ref()
                && label.display_label().eq_ignore_ascii_case("tribute")
            {
                "tribute wasn't paid".to_string()
            } else if let Condition::ThisSpellPaidLabel(label) = inner.as_ref() {
                // Render the negation as a flat clause (no parentheses): the generic
                // `not (...)` fallback collides with the reminder-text paren stripping
                // in debug_safe and would silently drop the condition.
                format!(
                    "this spell's {} cost wasn't paid",
                    label.display_label().to_ascii_lowercase()
                )
            } else if let Condition::TaggedObjectMatchedLastKnown(tag, filter) = inner.as_ref() {
                let positive = describe_last_known_tagged_object_condition(tag, filter);
                if let Some(rest) = positive.strip_prefix("it was ") {
                    format!("it wasn't {rest}")
                } else if let Some((before, after)) = positive.split_once(" was ") {
                    format!("{before} wasn't {after}")
                } else if let Some((before, after)) = positive.split_once(" were ") {
                    format!("{before} weren't {after}")
                } else {
                    format!("not ({positive})")
                }
            } else if let Condition::PlayerControls { player, filter } = inner.as_ref()
                && *player == PlayerFilter::You
                && filter.excluded_colors.count() == 1
            {
                let color = crate::color::Color::ALL
                    .into_iter()
                    .find(|color| filter.excluded_colors.contains(*color))
                    .expect("single excluded color");
                let mut described_filter = filter.clone();
                described_filter.controller = None;
                described_filter.excluded_colors = crate::color::ColorSet::new();
                let description = described_filter.description();
                let objects = pluralize_relative_object_phrase(strip_indefinite_article(
                    &description,
                ));
                format!("all {objects} you control are {}", color.name())
            } else if let Condition::PlayerControls { player, filter } = inner.as_ref() {
                if let Some(text) =
                    describe_player_controls_only_implicit_tagged_object(player, filter, true)
                {
                    return text;
                }
                let subject = describe_player_filter(player);
                let mut described_filter = filter.clone();
                if described_filter
                    .controller
                    .as_ref()
                    .is_some_and(|controller| controller == player)
                {
                    described_filter.controller = None;
                }
                let described = described_filter.description();
                let mut object_text = strip_indefinite_article(&described).to_string();
                if let Some(rest) = object_text.strip_prefix("another ") {
                    // "You control no other permanents" is substantially closer to oracle text than
                    // the ungrammatical "You control no another permanent".
                    object_text = format!("other {}", pluralize_noun_phrase(rest));
                } else {
                    object_text = pluralize_noun_phrase(&object_text);
                }
                let references_tagged_object =
                    described_filter.tagged_constraints.iter().any(|constraint| {
                        matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                    });
                if references_tagged_object {
                    return format!(
                        "{} {} neither {}",
                        subject,
                        player_verb(&subject, "control", "controls"),
                        object_text
                    );
                }
                format!(
                    "{} {} no {}",
                    subject,
                    player_verb(&subject, "control", "controls"),
                    object_text
                )
            } else if let Condition::ValueComparison {
                left:
                    Value::SpellsCastThisTurnMatching {
                        player,
                        filter,
                        exclude_source: false,
                    },
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            } = inner.as_ref()
            {
                let object_text = describe_spell_cast_condition_object(filter);
                if *player == PlayerFilter::Any {
                    format!("no player has cast {object_text} this turn")
                } else {
                    let subject = describe_player_filter(player);
                    format!(
                        "{} {} cast {} this turn",
                        subject,
                        player_verb(&subject, "haven't", "hasn't"),
                        object_text
                    )
                }
            } else {
                format!("not ({})", describe_condition(inner))
            }
        }
        Condition::And(left, right) => {
            if let Some(control_types_condition) =
                describe_you_control_two_card_types_condition(left, right)
            {
                return control_types_condition;
            }
            if let Some(spell_cast_condition) = describe_both_spell_cast_condition(left, right) {
                return spell_cast_condition;
            }
            if let Some(counter_spell_gate) = describe_missing_counter_spell_cast_gate(left, right)
            {
                return counter_spell_gate;
            }
            if let Some(source_activity_condition) =
                describe_source_neither_attacked_nor_entered_condition(left, right)
            {
                return source_activity_condition;
            }
            if let Some(exploit_condition) =
                describe_source_exploited_triggering_condition(left, right)
            {
                return exploit_condition;
            }
            if let Some(exiled_counter_condition) =
                describe_source_exiled_with_counter_condition(left, right)
            {
                return exiled_counter_condition;
            }
            // Avoid parentheses here: the semantic comparison pipeline strips parentheticals,
            // and these are just internal grouping markers, not oracle reminder text.
            format!("{} and {}", describe_condition(left), describe_condition(right))
        }
        Condition::Or(left, right) => {
            if let (
                Condition::ValueComparison {
                    left: Value::CardsInHand(left_player),
                    operator: crate::effect::ValueComparisonOperator::Equal,
                    right: Value::Fixed(left_count),
                },
                Condition::ValueComparison {
                    left: Value::CardsInHand(right_player),
                    operator: crate::effect::ValueComparisonOperator::Equal,
                    right: Value::Fixed(right_count),
                },
            ) = (left.as_ref(), right.as_ref())
                && left_player == right_player
                && *left_count >= 0
                && *right_count >= 0
            {
                let subject = describe_player_filter(left_player);
                let left_count =
                    number_word(*left_count).unwrap_or_else(|| left_count.to_string());
                let right_count =
                    number_word(*right_count).unwrap_or_else(|| right_count.to_string());
                return format!(
                    "{} {} exactly {} or exactly {} cards in hand",
                    subject,
                    player_verb(&subject, "have", "has"),
                    left_count,
                    right_count,
                );
            }
            let is_you_life_change = |condition: &Condition, gained: bool| {
                matches!(
                    condition,
                    Condition::ValueComparison {
                        left: Value::LifeGainedThisTurn(PlayerFilter::You),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(1),
                    } if gained
                ) || matches!(
                    condition,
                    Condition::ValueComparison {
                        left: Value::LifeLostThisTurn(PlayerFilter::You),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(1),
                    } if !gained
                )
            };
            if (is_you_life_change(left, true) && is_you_life_change(right, false))
                || (is_you_life_change(right, true) && is_you_life_change(left, false))
            {
                return "you gained or lost life this turn".to_string();
            }
            if let Some(initiative_gate) = describe_you_or_attacked_player_initiative_condition(left, right)
            {
                return initiative_gate;
            }
            if let Some(origin_gate) =
                describe_triggering_graveyard_origin_or_condition(left, right)
            {
                return origin_gate;
            }
            format!("{} or {}", describe_condition(left), describe_condition(right))
        }
    }
}

fn describe_last_known_tagged_object_condition(tag: &TagKey, filter: &ObjectFilter) -> String {
    if filter.has_nonbasic_land_type {
        return "that land had a nonbasic land type".to_string();
    }
    // Last-known predicates retain an explicit pronoun subject. Action tags
    // describe how that object reached its last-known state, but must not
    // replace `it was` with a new event-subject sentence.
    if this_way_action_from_tag(tag).is_some() {
        return format!(
            "it was {}",
            ensure_indefinite_article(&filter.description())
        );
    }
    if tag.as_str().starts_with("countered_")
        && filter.zone == Some(Zone::Stack)
        && matches!(
            filter.stack_kind,
            None | Some(crate::filter::StackObjectKind::Spell)
        )
    {
        return format!(
            "it was {}",
            ensure_indefinite_article(&filter.description())
        );
    }
    let current = describe_condition(&Condition::TaggedObjectMatches(tag.clone(), filter.clone()));
    if let Some(rest) = current.strip_prefix("that object is ") {
        return format!("it was {rest}");
    }
    if let Some(rest) = current.strip_prefix("that object are ") {
        return format!("it was {rest}");
    }
    if let Some(rest) = current.strip_prefix("that object's ") {
        let current = format!("its {rest}");
        if let Some((before, after)) = current.split_once(" is ") {
            return format!("{before} was {after}");
        }
        if let Some((before, after)) = current.split_once(" are ") {
            return format!("{before} were {after}");
        }
        return current;
    }
    if let Some(rest) = current.strip_prefix("it's ") {
        return format!("it was {rest}");
    }
    if let Some(rest) = current.strip_prefix("it is ") {
        return format!("it was {rest}");
    }
    current
}

pub(crate) fn describe_implicit_tagged_object_state_condition(
    subject: &str,
    filter: &ObjectFilter,
) -> Option<String> {
    let mut base = filter.clone();
    base.attacking = false;
    base.nonattacking = false;
    base.blocking = false;
    base.nonblocking = false;
    base.blocked = false;
    base.unblocked = false;
    base.tapped = false;
    base.untapped = false;
    if base != ObjectFilter::default()
        && base != ObjectFilter::permanent()
        && base != ObjectFilter::creature()
    {
        return None;
    }

    if filter.tapped {
        return Some(if subject == "it" {
            "it's tapped".to_string()
        } else {
            format!("{subject} is tapped")
        });
    }
    if filter.untapped {
        return Some(if subject == "it" {
            "it's untapped".to_string()
        } else {
            format!("{subject} is untapped")
        });
    }
    if filter.attacking {
        return Some("it was attacking".to_string());
    }
    if filter.nonattacking {
        return Some(if subject == "it" {
            "it wasn't attacking".to_string()
        } else {
            "that object wasn't attacking".to_string()
        });
    }
    if filter.blocking {
        return Some(if subject == "it" {
            "it was blocking".to_string()
        } else {
            "that object was blocking".to_string()
        });
    }
    if filter.nonblocking {
        return Some(if subject == "it" {
            "it wasn't blocking".to_string()
        } else {
            "that object wasn't blocking".to_string()
        });
    }
    if filter.blocked {
        return Some(if subject == "it" {
            "it was blocked this turn".to_string()
        } else {
            "that object was blocked this turn".to_string()
        });
    }
    if filter.unblocked {
        return Some(if subject == "it" {
            "it was unblocked".to_string()
        } else {
            "that object was unblocked".to_string()
        });
    }

    None
}

pub(crate) fn is_implicit_object_identity_filter(filter: &ObjectFilter) -> bool {
    filter == &ObjectFilter::default()
        || filter == &ObjectFilter::permanent()
        || filter == &ObjectFilter::permanent_card()
        || filter == &ObjectFilter::creature()
}

pub(crate) fn describe_triggering_graveyard_origin_condition(
    subject: &str,
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.len() != 2 {
        return None;
    }

    let branches = filter
        .any_of
        .iter()
        .map(triggering_graveyard_origin_filter_branch)
        .collect::<Option<Vec<_>>>()?;
    describe_triggering_graveyard_origin_from_branches(subject, &branches)
}

pub(crate) fn triggering_graveyard_origin_filter_branch(
    filter: &ObjectFilter,
) -> Option<(Option<PlayerFilter>, Option<PlayerFilter>)> {
    let mut base = filter.clone();
    let owner = base.owner.take();
    let cast_by = base.cast_by.take();
    is_implicit_object_identity_filter(&base).then_some((cast_by, owner))
}

pub(crate) fn describe_triggering_graveyard_origin_or_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    fn branch(condition: &Condition) -> Option<(Option<PlayerFilter>, Option<PlayerFilter>)> {
        let Condition::TaggedObjectMatches(tag, filter) = condition else {
            return None;
        };
        (tag.as_str() == "triggering")
            .then(|| triggering_graveyard_origin_filter_branch(filter))
            .flatten()
    }

    let branches = vec![branch(left)?, branch(right)?];
    describe_triggering_graveyard_origin_from_branches("that object", &branches)
}

pub(crate) fn describe_triggering_graveyard_origin_from_branches(
    subject: &str,
    branches: &[(Option<PlayerFilter>, Option<PlayerFilter>)],
) -> Option<String> {
    let entered = branches.iter().find(|(cast_by, _)| cast_by.is_none())?;
    let cast = branches.iter().find(|(cast_by, _)| cast_by.is_some())?;
    let caster = cast.0.as_ref()?;
    let entered_origin = describe_graveyard_origin_phrase(entered.1.as_ref());
    let cast_origin = describe_graveyard_origin_phrase(cast.1.as_ref().or(entered.1.as_ref()));
    let caster = describe_player_filter(caster);

    Some(format!(
        "{subject} entered from {entered_origin} or {caster} cast it from {cast_origin}"
    ))
}

pub(crate) fn describe_graveyard_origin_phrase(owner: Option<&PlayerFilter>) -> String {
    match owner {
        None | Some(PlayerFilter::Any) => "a graveyard".to_string(),
        Some(PlayerFilter::You) => "your graveyard".to_string(),
        Some(PlayerFilter::Opponent) => "an opponent's graveyard".to_string(),
        Some(PlayerFilter::NotYou) => "a graveyard other than yours".to_string(),
        Some(player) => format!("{}'s graveyard", describe_player_filter(player)),
    }
}

pub(crate) fn describe_implicit_tagged_object_any_of_condition(
    tag: &crate::TagKey,
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.is_empty() {
        return None;
    }

    let clauses = filter
        .any_of
        .iter()
        .map(|branch| {
            describe_condition(&Condition::TaggedObjectMatches(tag.clone(), branch.clone()))
        })
        .collect::<Vec<_>>();
    Some(join_with_or(&clauses))
}

pub(crate) fn describe_implicit_tagged_object_quality_condition(
    subject: &str,
    filter: &ObjectFilter,
) -> Option<String> {
    let mut base = filter.clone();
    base.historic = false;
    base.nonhistoric = false;
    base.entered_battlefield_this_turn = false;
    base.entered_battlefield_controller = None;
    if !is_implicit_object_identity_filter(&base) {
        return None;
    }

    if filter.historic {
        return Some(if subject == "it" {
            "it was historic".to_string()
        } else {
            "that object was historic".to_string()
        });
    }
    if filter.nonhistoric {
        return Some(if subject == "it" {
            "it wasn't historic".to_string()
        } else {
            "that object wasn't historic".to_string()
        });
    }
    if filter.entered_battlefield_this_turn || filter.entered_battlefield_controller.is_some() {
        let entered_subject = if subject == "it" { "it" } else { "that object" };
        if let Some(controller) = &filter.entered_battlefield_controller {
            let controller = describe_player_filter(controller);
            return Some(format!(
                "{entered_subject} entered under {controller}'s control this turn"
            ));
        }
        return Some(format!("{entered_subject} entered this turn"));
    }

    None
}

pub(crate) fn describe_implicit_tagged_object_pt_condition(
    subject: &str,
    filter: &ObjectFilter,
) -> Option<String> {
    let mut base = filter.clone();
    base.power = None;
    base.power_parity = None;
    base.power_reference = ironsmith_core::PtReference::default();
    base.power_relative_to_source = None;
    base.power_greater_than_base_power = false;
    base.power_toughness_relation = None;
    base.toughness = None;
    base.toughness_reference = ironsmith_core::PtReference::default();
    base.total_power_toughness = None;
    if !is_implicit_object_identity_filter(&base) {
        return None;
    }

    let possessive = if subject == "it" {
        "its"
    } else {
        "that object's"
    };

    if let (Some(power), Some(toughness)) = (&filter.power, &filter.toughness)
        && let (
            ironsmith_core::FilterComparison::Equal(power),
            ironsmith_core::FilterComparison::Equal(toughness),
        ) = (power, toughness)
        && filter.power_reference == filter.toughness_reference
    {
        let label = match filter.power_reference {
            ironsmith_core::PtReference::Effective => "power and toughness",
            ironsmith_core::PtReference::Base => "base power and toughness",
        };
        return Some(format!("{possessive} {label} are {power}/{toughness}"));
    }

    if let Some(power) = &filter.power {
        let label = match filter.power_reference {
            ironsmith_core::PtReference::Effective => "power",
            ironsmith_core::PtReference::Base => "base power",
        };
        return Some(format!(
            "{possessive} {label} {}",
            describe_filter_comparison_clause(power)
        ));
    }
    if filter.power_greater_than_base_power {
        return Some(format!("{possessive} power is greater than its base power"));
    }
    if let Some(relation) = filter.power_toughness_relation {
        return Some(match relation {
            ironsmith_core::PowerToughnessRelation::PowerGreaterThanToughness => {
                format!("{possessive} power is greater than its toughness")
            }
            ironsmith_core::PowerToughnessRelation::ToughnessGreaterThanPower => {
                format!("{possessive} toughness is greater than its power")
            }
        });
    }
    if let Some(relation) = filter.power_relative_to_source {
        return Some(match relation {
            ironsmith_core::SourcePowerRelation::LessThanSource => {
                format!("{possessive} power is less than this creature's power")
            }
        });
    }
    if let Some(toughness) = &filter.toughness {
        let label = match filter.toughness_reference {
            ironsmith_core::PtReference::Effective => "toughness",
            ironsmith_core::PtReference::Base => "base toughness",
        };
        return Some(format!(
            "{possessive} {label} {}",
            describe_filter_comparison_clause(toughness)
        ));
    }
    if let Some(total) = &filter.total_power_toughness {
        return Some(format!(
            "{possessive} total power and toughness {}",
            describe_filter_comparison_clause(total)
        ));
    }

    None
}

pub(crate) fn describe_filter_comparison_clause(
    comparison: &ironsmith_core::FilterComparison,
) -> String {
    match comparison {
        ironsmith_core::FilterComparison::GreaterThan(n) => format!("is greater than {n}"),
        ironsmith_core::FilterComparison::GreaterThanOrEqual(n) => format!("is {n} or greater"),
        ironsmith_core::FilterComparison::Equal(n) => format!("is {n}"),
        ironsmith_core::FilterComparison::LessThan(n) => format!("is less than {n}"),
        ironsmith_core::FilterComparison::LessThanOrEqual(n) => format!("is {n} or less"),
        ironsmith_core::FilterComparison::NotEqual(n) => format!("is not {n}"),
        ironsmith_core::FilterComparison::OneOf(values) => {
            let values = values.iter().map(i32::to_string).collect::<Vec<_>>();
            format!("is {}", join_with_or(&values))
        }
        ironsmith_core::FilterComparison::LessThanExpr(value) => {
            format!("is less than {}", describe_value(value))
        }
        ironsmith_core::FilterComparison::LessThanOrEqualExpr(value) => {
            format!("is less than or equal to {}", describe_value(value))
        }
        ironsmith_core::FilterComparison::GreaterThanExpr(value) => {
            format!("is greater than {}", describe_value(value))
        }
        ironsmith_core::FilterComparison::GreaterThanOrEqualExpr(value) => {
            format!("is greater than or equal to {}", describe_value(value))
        }
        ironsmith_core::FilterComparison::EqualExpr(value) => {
            format!("is equal to {}", describe_value(value))
        }
        ironsmith_core::FilterComparison::NotEqualExpr(value) => {
            format!("is not equal to {}", describe_value(value))
        }
    }
}

pub(crate) fn describe_implicit_tagged_object_fallback_condition(
    subject: &str,
    desc: &str,
) -> String {
    if desc.contains("object-predicate-debug") {
        return if subject == "it" {
            "it has the stated quality".to_string()
        } else {
            "that object has the stated quality".to_string()
        };
    }
    let phrase = ensure_indefinite_article(desc);
    if subject == "it" {
        format!("it's {phrase}")
    } else {
        format!("{subject} is {phrase}")
    }
}

pub(crate) fn tagged_condition_is_known_card_reference(tag: &crate::TagKey) -> bool {
    let tag = tag.as_str();
    tag.starts_with("exiled_")
        || tag.starts_with("revealed_")
        || crate::cards::is_sentence_helper_tag(tag, "exiled")
        || crate::cards::is_sentence_helper_tag(tag, "revealed")
}

pub(crate) fn simple_type_identity_condition_filter(filter: &ObjectFilter) -> bool {
    let mut stripped = filter.clone();
    stripped.card_types.clear();
    stripped.all_card_types.clear();
    stripped == ObjectFilter::default()
}

pub(crate) fn describe_you_or_attacked_player_initiative_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    fn is_player_initiative(condition: &Condition, player: PlayerFilter) -> bool {
        matches!(
            condition,
            Condition::PlayerHasInitiative {
                player: condition_player
            } if *condition_player == player
        )
    }

    let matches_pair = (is_player_initiative(left, PlayerFilter::You)
        && is_player_initiative(right, PlayerFilter::Defending))
        || (is_player_initiative(left, PlayerFilter::Defending)
            && is_player_initiative(right, PlayerFilter::You));
    if !matches_pair {
        return None;
    }

    Some("you or a player you're attacking has the initiative".to_string())
}

pub(crate) fn describe_you_control_two_card_types_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    fn controlled_single_card_type(condition: &Condition) -> Option<crate::types::CardType> {
        let Condition::PlayerControls { player, filter } = condition else {
            return None;
        };
        if *player != PlayerFilter::You
            || filter.card_types.len() != 1
            || filter.zone.is_some()
            || filter.controller.is_some()
            || filter.owner.is_some()
            || !filter.all_card_types.is_empty()
            || !filter.excluded_card_types.is_empty()
            || !filter.subtypes.is_empty()
            || !filter.supertypes.is_empty()
            || filter.colors.is_some()
            || filter.required_colors.is_some()
            || filter.sticker.is_some()
            || filter.power.is_some()
            || filter.toughness.is_some()
        {
            return None;
        }
        Some(filter.card_types[0])
    }

    let left_type = controlled_single_card_type(left)?;
    let right_type = controlled_single_card_type(right)?;
    if left_type == right_type {
        return None;
    }

    let left_text = with_indefinite_article(&left_type.to_string().to_ascii_lowercase());
    let right_text = with_indefinite_article(&right_type.to_string().to_ascii_lowercase());
    Some(format!("you control {left_text} and {right_text}"))
}

pub(crate) fn describe_source_exploited_triggering_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    fn is_exploited_triggering(condition: &Condition) -> bool {
        let Condition::TaggedObjectMatches(tag, filter) = condition else {
            return false;
        };
        tag.as_str() == crate::tag::EXPLOITED_TAG
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == "triggering"
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
    }

    fn is_exploiter_source(condition: &Condition) -> bool {
        let Condition::TaggedObjectMatches(tag, filter) = condition else {
            return false;
        };
        tag.as_str() == crate::tag::EXPLOITER_TAG && filter.source
    }

    if (is_exploited_triggering(left) && is_exploiter_source(right))
        || (is_exploited_triggering(right) && is_exploiter_source(left))
    {
        Some("it exploited that creature".to_string())
    } else {
        None
    }
}

pub(crate) fn compact_repeated_process_once_surface(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let without_period = trimmed.trim_end_matches('.');

    if let Some(first_pass) = compact_repeated_unless_joined_with_and_loses(without_period) {
        return Some(format!(
            "{}. Repeat this process once.",
            normalize_repeated_process_first_pass_surface(&first_pass)
        ));
    }

    if let Some((left, right)) = without_period.split_once(", then ") {
        let first_pass = left.trim();
        if !first_pass.is_empty()
            && first_pass.eq_ignore_ascii_case(right.trim())
            && first_pass.to_ascii_lowercase().contains(" unless ")
        {
            return Some(format!(
                "{}. Repeat this process once.",
                normalize_repeated_process_first_pass_surface(first_pass)
            ));
        }
    }

    let sentences = without_period
        .split(". ")
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() < 2 || sentences.len() % 2 != 0 {
        return None;
    }

    let half = sentences.len() / 2;
    if sentences[..half] != sentences[half..] {
        return None;
    }

    let first_pass = sentences[..half].join(". ");
    if half == 1 && !first_pass.to_ascii_lowercase().contains(" unless ") {
        return None;
    }

    Some(format!(
        "{}. Repeat this process once.",
        normalize_repeated_process_first_pass_surface(&first_pass)
    ))
}

pub(crate) fn compact_repeated_unless_joined_with_and_loses(line: &str) -> Option<String> {
    let (first_pass, repeated_tail) = line.split_once(" and loses ")?;
    if !first_pass.to_ascii_lowercase().contains(" unless ") {
        return None;
    }
    let (_, first_tail) = first_pass.split_once(" loses ")?;
    first_tail
        .eq_ignore_ascii_case(repeated_tail.trim())
        .then(|| first_pass.trim().to_string())
}

pub(crate) fn normalize_repeated_process_first_pass_surface(first_pass: &str) -> String {
    first_pass
        .replace(" unless target opponent ", " unless that player ")
        .replace(" unless target player ", " unless that player ")
}

pub(crate) fn compact_until_next_turn_token_copy_haste_surface(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix(
        "Until your next turn, at the beginning of combat on your turn, exile target ",
    )?;
    let (target, rest) = rest.split_once(
        " from your graveyard, create a token that's a copy of that card, with base power and toughness 1/1, then it gains haste",
    )?;
    if !rest.trim().trim_end_matches('.').is_empty() {
        return None;
    }

    let target = target
        .trim()
        .replace("white or black or red", "red, white, or black");
    Some(format!(
        "At the beginning of combat on your turn, exile target {target} from your graveyard. Create a token that's a copy of that card, except it's 1/1. It gains haste until your next turn."
    ))
}

pub(crate) fn describe_source_neither_attacked_nor_entered_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    fn is_not_source_attacked(condition: &Condition) -> bool {
        matches!(condition, Condition::Not(inner) if matches!(inner.as_ref(), Condition::SourceAttackedThisTurn))
    }

    fn is_not_source_came_under_your_control(condition: &Condition) -> bool {
        matches!(
            condition,
            Condition::Not(inner)
                if matches!(
                    inner.as_ref(),
                    Condition::SourceCameUnderYourControlThisTurn
                )
        )
    }

    if (is_not_source_attacked(left) && is_not_source_came_under_your_control(right))
        || (is_not_source_attacked(right) && is_not_source_came_under_your_control(left))
    {
        Some("this creature didn't attack or come under your control this turn".to_string())
    } else {
        None
    }
}

pub(crate) fn describe_source_exiled_with_counter_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    fn parts<'a>(first: &'a Condition, second: &'a Condition) -> Option<(&'a CounterType, u32)> {
        if !matches!(first, Condition::SourceIsInZone(Zone::Exile)) {
            return None;
        }
        let Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
            ..
        } = second
        else {
            return None;
        };
        Some((counter_type, *count))
    }

    let (counter_type, count) = parts(left, right).or_else(|| parts(right, left))?;
    if *counter_type == CounterType::Time && count >= 1 {
        return Some("this card is suspended".to_string());
    }
    Some(format!(
        "this card is exiled with {} on it",
        describe_put_counter_phrase(&Value::Fixed(count as i32), *counter_type)
    ))
}

#[cfg(test)]
mod greatest_power_control_tests {
    use super::*;

    #[test]
    fn renders_control_of_every_global_greatest_power_creature() {
        let global_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
        let mut greatest = global_creatures.clone();
        greatest.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestPower(global_creatures),
        )));
        let controlled = greatest.clone().controlled_by(PlayerFilter::You);
        let condition = Condition::ValueComparison {
            left: Value::Count(controlled),
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: Value::Count(greatest),
        };

        assert_eq!(
            describe_condition(&condition),
            "you control each creature on the battlefield with the greatest power"
        );
    }
}
