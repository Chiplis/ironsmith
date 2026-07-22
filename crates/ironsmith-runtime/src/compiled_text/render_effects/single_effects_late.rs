use super::*;

pub(super) fn describe_remove_counter_phrase(
    count: &Value,
    counter_type: CounterType,
    target: &ChooseSpec,
) -> String {
    let leaves_one_matching_counter = match count.unhinted() {
        Value::Add(base, offset) if matches!(offset.unhinted(), Value::Fixed(-1)) => {
            match base.unhinted() {
                Value::CountersOnSource(found_counter) => {
                    *found_counter == counter_type && matches!(target.base(), ChooseSpec::Source)
                }
                Value::CountersOn(counter_source, Some(found_counter)) => {
                    *found_counter == counter_type && counter_source.unhinted() == target.unhinted()
                }
                _ => false,
            }
        }
        _ => false,
    };
    if leaves_one_matching_counter {
        return format!(
            "all but one {} counter",
            describe_counter_type(counter_type)
        );
    }

    let removes_all_matching_counters = match count {
        Value::CountersOnSource(found_counter) => {
            *found_counter == counter_type && matches!(target.base(), ChooseSpec::Source)
        }
        Value::CountersOn(counter_source, Some(found_counter)) => {
            *found_counter == counter_type && counter_source.unhinted() == target.unhinted()
        }
        _ => false,
    };

    if removes_all_matching_counters {
        format!("all {} counters", describe_counter_type(counter_type))
    } else {
        describe_put_counter_phrase(count, counter_type)
    }
}

pub(super) fn prevent_next_time_target_source_text(spec: &ChooseSpec) -> String {
    match spec.base() {
        ChooseSpec::Object(filter) => prevent_next_time_tagged_source_text(filter)
            .unwrap_or_else(|| describe_choose_spec(spec)),
        ChooseSpec::Tagged(_) => "that source".to_string(),
        _ => describe_choose_spec(spec),
    }
}

pub(super) fn prevent_next_time_tagged_source_text(filter: &ObjectFilter) -> Option<String> {
    let has_tagged_source = filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        )
    });
    if !has_tagged_source {
        return None;
    }

    if filter.card_types.contains(&CardType::Creature) {
        Some("that creature".to_string())
    } else if let Some(card_type) = filter.card_types.first() {
        Some(format!("that {}", card_type.name()))
    } else {
        Some("that source".to_string())
    }
}

pub(super) fn singularize_plural_object_phrase(phrase: &str) -> String {
    phrase
        .split_whitespace()
        .map(|word| {
            let (core, suffix) = word
                .strip_suffix(',')
                .map(|core| (core, ","))
                .unwrap_or((word, ""));
            if matches!(core, "and" | "or") {
                return word.to_string();
            }
            // This helper is used for the object phrase before appending a
            // controller tail.  `controls` is a verb here, not a plural
            // noun, so stripping its final `s` produces "they control" in
            // the wrong grammatical context ("that player control").
            if core == "controls" {
                return word.to_string();
            }
            let singular = if let Some(stem) = core.strip_suffix("ies") {
                format!("{stem}y")
            } else if let Some(stem) = core.strip_suffix('s') {
                if core.ends_with("ss") {
                    core.to_string()
                } else {
                    stem.to_string()
                }
            } else {
                core.to_string()
            };
            format!("{singular}{suffix}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn describe_dynamic_count_tap(tap: &crate::effects::TapEffect) -> Option<String> {
    let ChooseSpec::WithCountValue(inner, count, count_value) = &tap.target else {
        return None;
    };
    if !count.is_dynamic_x() || count.is_up_to_dynamic_x() || count.is_random() {
        return None;
    }
    let (counter_type, source_text) = match count_value {
        Value::CountersOnSource(counter_type) => (*counter_type, "this permanent".to_string()),
        Value::CountersOn(spec, Some(counter_type)) => (*counter_type, describe_choose_spec(spec)),
        _ => return None,
    };
    let ChooseSpec::Object(filter) = inner.base() else {
        return None;
    };

    let description = filter.description();
    let (prefix, object_phrase, control_tail) = if filter.controller == Some(PlayerFilter::Active) {
        let stripped = description
            .strip_prefix("the active player's ")
            .or_else(|| description.strip_prefix("active player's "))
            .unwrap_or_else(|| strip_leading_article(&description));
        ("That player taps", stripped, " they control")
    } else {
        ("Tap", strip_leading_article(&description), "")
    };
    let object_phrase = singularize_plural_object_phrase(object_phrase);
    Some(format!(
        "{prefix} {}{control_tail} for each {} counter on {source_text}",
        with_indefinite_article(&object_phrase),
        describe_counter_type(counter_type)
    ))
}

pub(super) fn describe_villainous_choice(
    villainous: &crate::effects::VillainousChoiceEffect,
) -> String {
    let player = villainous
        .player_surface
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| describe_player_filter(&villainous.player));
    let modes = villainous
        .modes
        .iter()
        .map(|mode| {
            let rendered = mode.source_text.trim();
            if rendered.is_empty() {
                describe_effect_list(&mode.effects)
            } else {
                rendered.trim_end_matches('.').to_string()
            }
        })
        .collect::<Vec<_>>();

    match modes.as_slice() {
        [first, second] => {
            format!(
                "{player} faces a villainous choice — {}, or {}",
                capitalize_first(first),
                lowercase_first(second)
            )
        }
        [] => format!("{player} faces a villainous choice"),
        _ => format!(
            "{player} faces a villainous choice — {}",
            capitalize_first(&modes.join(", or "))
        ),
    }
}

fn discard_count_covers_entire_hand(discard: &crate::effects::DiscardEffect) -> bool {
    if discard.any_number || discard.random {
        return false;
    }
    if discard
        .count
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand)
    {
        return true;
    }

    match (&discard.count, discard.card_filter.as_ref()) {
        (Value::CardsInHand(owner), None) => owner == &discard.player,
        (Value::Count(count_filter), Some(card_filter)) if count_filter == card_filter => {
            if count_filter.zone != Some(Zone::Hand)
                || count_filter.owner.as_ref() != Some(&discard.player)
            {
                return false;
            }

            let mut unconstrained = count_filter.clone();
            unconstrained.zone = None;
            unconstrained.owner = None;
            unconstrained == ObjectFilter::default()
        }
        _ => false,
    }
}

fn effect_is_single_draw_instruction(effect: &Effect) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    if effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some()
    {
        return true;
    }
    let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() else {
        return false;
    };
    let [inner] = may.effects.as_slice() else {
        return false;
    };
    effect_is_single_draw_instruction(inner)
}

fn describe_next_turn_upkeep_delayed_instruction(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
    delayed_text: &str,
) -> String {
    let effects = schedule.effects.flattened_default_effects();
    if let [effect] = effects
        && effect_is_single_draw_instruction(effect)
    {
        let draw_text = delayed_text
            .strip_prefix("you draw ")
            .map(|remainder| format!("Draw {remainder}"))
            .unwrap_or_else(|| capitalize_first(delayed_text));
        return format!("{draw_text} at the beginning of the next turn's upkeep");
    }
    format!("At the beginning of the next turn's upkeep, {delayed_text}")
}

fn describe_restart_game(restart: &crate::effects::RestartGameEffect) -> String {
    let Some(spec) = &restart.cards_left_in_exile else {
        return "Restart the game".to_string();
    };

    let objects = if let ChooseSpec::All(filter) = spec.base() {
        let mut residual = filter.clone();
        residual.zone = None;
        residual.card_types.clear();
        residual
            .excluded_subtypes
            .retain(|subtype| *subtype != Subtype::Aura);
        residual.tagged_constraints.retain(|constraint| {
            constraint.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
                || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        });
        residual.set_explicit_card_noun(false);
        if card_types_are_permanent_card_types(&filter.card_types)
            && filter.excluded_subtypes.contains(&Subtype::Aura)
            && residual == ObjectFilter::default()
        {
            "all non-Aura permanent cards".to_string()
        } else {
            let mut printable = filter.clone();
            printable.zone = None;
            printable.tagged_constraints.retain(|constraint| {
                constraint.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
                    || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            });
            describe_choose_spec(&ChooseSpec::All(printable))
        }
    } else {
        describe_choose_spec(spec)
    };
    let source = restart
        .source_surface
        .as_ref()
        .map(crate::target::SourceReferenceSurface::display_text)
        .unwrap_or_else(|| "this source".to_string());
    format!("Restart the game, leaving in exile {objects} exiled with {source}")
}

fn describe_play_subgame(subgame: &crate::effects::PlaySubgameEffect) -> String {
    let opening = "Players play a Magic subgame, using their libraries as their decks";
    let [continuation] = subgame.nonwinner_effects.as_slice() else {
        if subgame.nonwinner_effects.is_empty() {
            return opening.to_string();
        }
        let body = lowercase_first(
            describe_effect_list(&subgame.nonwinner_effects)
                .trim_end_matches('.')
                .trim(),
        );
        return format!("{opening}. After the subgame, {body} for each player who didn't win it");
    };
    let canonical_half_life = continuation
        .downcast_ref::<crate::effects::LoseLifeEffect>()
        .is_some_and(|loss| {
            loss.amount == Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer)
                && matches!(
                    loss.player.base(),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer)
                )
        });
    if canonical_half_life {
        return format!(
            "{opening}. Each player who doesn't win the subgame loses half their life, rounded up"
        );
    }
    let body = lowercase_first(
        describe_effect_list(&subgame.nonwinner_effects)
            .trim_end_matches('.')
            .trim(),
    );
    format!("{opening}. After the subgame, {body} for each player who didn't win it")
}

pub(crate) fn describe_effect_impl(effect: &Effect) -> String {
    include!("effect_impl/early.rs");
    include!("effect_impl/late.rs")
}

pub(super) fn describe_unattach_all_equipment_from_tagged(spec: &ChooseSpec) -> Option<String> {
    let ChooseSpec::All(filter) = spec.base() else {
        return None;
    };
    if !filter.subtypes.contains(&Subtype::Equipment) {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
    }) {
        return None;
    }
    Some("Unattach all Equipment from them".to_string())
}

pub(super) fn describe_target_then_unattach_all_equipment(
    target_effect: &Effect,
    unattach_effect: &Effect,
) -> Option<String> {
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let unattach = unwrap_basic_tag_wrappers(unattach_effect)
        .downcast_ref::<crate::effects::UnattachObjectsEffect>()?;
    let ChooseSpec::All(filter) = unattach.objects.base() else {
        return None;
    };
    if !filter.subtypes.contains(&Subtype::Equipment) {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *target_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
    }) {
        return None;
    }
    Some(format!(
        "Unattach all Equipment from {}",
        describe_choose_spec(&target_only.target)
    ))
}

pub(crate) fn describe_activation_timing_clause(timing: &ActivationTiming) -> Option<&'static str> {
    match timing {
        ActivationTiming::AnyTime => None,
        ActivationTiming::SorcerySpeed => Some("Activate only as a sorcery"),
        ActivationTiming::DuringCombat => Some("Activate only during combat"),
        ActivationTiming::OncePerTurn => Some("Activate only once each turn"),
        ActivationTiming::DuringYourTurn => Some("Activate only during your turn"),
        ActivationTiming::DuringOpponentsTurn => Some("Activate only during an opponent's turn"),
        ActivationTiming::DuringSourceOwnersUpkeep => {
            Some("Activate only during this card's owner's upkeep")
        }
    }
}

pub(super) fn is_whole_graveyard_exile_filter(filter: &ObjectFilter) -> bool {
    let mut expected = ObjectFilter::default().in_zone(Zone::Graveyard);
    expected.owner = filter.owner.clone();
    filter == &expected
}

pub(super) fn describe_excess_damage_condition_target(target: &ChooseSpec) -> String {
    let described = describe_choose_spec(target);
    if let Some(rest) = described.strip_prefix("target ") {
        format!("that {rest}")
    } else {
        described
    }
}

pub(super) fn excess_damage_condition_target_from_effect(effect: &Effect) -> Option<String> {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(describe_excess_damage_condition_target(&damage.target));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return excess_damage_condition_target_from_effect(&tagged.effect);
    }
    None
}

pub(crate) fn normalize_activation_restriction_clause(raw: &str) -> String {
    let mut clause = raw.trim().trim_end_matches('.').to_string();
    if clause.is_empty() {
        return clause;
    }
    let lower = clause.to_ascii_lowercase();
    if lower == "activate only as a sorcery and only once each turn" {
        return "Activate only once each turn".to_string();
    }
    clause = clause.replace("activate only as sorcery", "activate only as a sorcery");
    clause = clause.replace("activate only once turn", "activate only once each turn");
    if clause.starts_with("activate ") {
        clause = capitalize_first(&clause);
    }
    clause
}

pub(super) fn describe_mana_usage_restriction_clauses_for_activated(
    activated: &crate::ability::ActivatedAbility,
) -> Vec<String> {
    activated
        .mana_usage_restrictions
        .iter()
        .filter_map(|restriction| describe_mana_usage_restriction(restriction, Some(activated)))
        .collect()
}

pub(super) fn describe_mana_usage_restriction(
    restriction: &crate::ability::ManaUsageRestriction,
    activated: Option<&crate::ability::ActivatedAbility>,
) -> Option<String> {
    match restriction {
        crate::ability::ManaUsageRestriction::CastSpell {
            card_types,
            subtype_requirement,
            restrict_to_matching_spell,
            grant_uncounterable,
            enters_with_counters,
            granted_abilities,
        } => {
            let spell_text = describe_mana_usage_spell_target(card_types, *subtype_requirement)?;
            let use_spent_on_wording = !granted_abilities.is_empty()
                && activated
                    .and_then(activated_mana_output_amount)
                    .unwrap_or(1)
                    > 1;
            let mut line = if *restrict_to_matching_spell {
                format!("Spend this mana only to cast {spell_text}")
            } else if !*grant_uncounterable
                && enters_with_counters.is_empty()
                && granted_abilities.is_empty()
            {
                return None;
            } else if use_spent_on_wording {
                format!("If that mana is spent on {spell_text}")
            } else {
                format!("If this mana is spent to cast {spell_text}")
            };

            let mut bonuses = Vec::new();
            if *grant_uncounterable {
                bonuses.push("that spell can't be countered".to_string());
            }
            bonuses.extend(
                enters_with_counters.iter().map(|(counter_type, count)| {
                    describe_mana_usage_etb_bonus(*counter_type, *count)
                }),
            );
            bonuses.extend(
                granted_abilities
                    .iter()
                    .filter_map(|ability| describe_mana_usage_static_ability_bonus(*ability)),
            );

            if bonuses.is_empty() {
                return Some(line);
            }

            if *restrict_to_matching_spell {
                line.push_str(", and ");
            } else {
                line.push_str(", ");
            }
            line.push_str(&bonuses.join(" and "));
            Some(line)
        }
        crate::ability::ManaUsageRestriction::CastSpellMatching {
            filter,
            restrict_to_matching_spell,
            grant_uncounterable,
            enters_with_counters,
            granted_abilities,
        } => {
            let pluralize_origin_spell = activated
                .and_then(activated_mana_output_amount)
                .is_some_and(|amount| amount > 1);
            let spell_text = describe_mana_usage_spell_filter_target_with_options(
                filter,
                pluralize_origin_spell,
            )?;
            let mut line = if *restrict_to_matching_spell {
                format!("Spend this mana only to cast {spell_text}")
            } else if !*grant_uncounterable
                && enters_with_counters.is_empty()
                && granted_abilities.is_empty()
            {
                return None;
            } else if !granted_abilities.is_empty() && pluralize_origin_spell {
                format!("If that mana is spent on {spell_text}")
            } else {
                format!("If this mana is spent to cast {spell_text}")
            };

            let mut bonuses = Vec::new();
            if *grant_uncounterable {
                bonuses.push("that spell can't be countered".to_string());
            }
            bonuses.extend(
                enters_with_counters.iter().map(|(counter_type, count)| {
                    describe_mana_usage_etb_bonus(*counter_type, *count)
                }),
            );
            bonuses.extend(
                granted_abilities
                    .iter()
                    .filter_map(|ability| describe_mana_usage_static_ability_bonus(*ability)),
            );

            if bonuses.is_empty() {
                return Some(line);
            }

            if *restrict_to_matching_spell {
                line.push_str(", and ");
            } else {
                line.push_str(", ");
            }
            line.push_str(&bonuses.join(" and "));
            Some(line)
        }
        crate::ability::ManaUsageRestriction::CastSpellWithManaBonus {
            filter,
            condition,
            grant_uncounterable,
            enters_with_counters,
            granted_abilities,
            granted_keywords,
        } => {
            let spell_text = describe_mana_usage_spell_filter_target_with_options(filter, false)?;
            let mut line = match condition {
                crate::ability::ManaSpendBonusCondition::IfThisManaIsSpentToCast => {
                    format!("If this mana is spent to cast {spell_text}")
                }
                crate::ability::ManaSpendBonusCondition::IfThatManaIsSpentToCast => {
                    format!("If that mana is spent to cast {spell_text}")
                }
                crate::ability::ManaSpendBonusCondition::IfThisManaIsSpentOn => {
                    format!("If this mana is spent on {spell_text}")
                }
                crate::ability::ManaSpendBonusCondition::IfThatManaIsSpentOn => {
                    format!("If that mana is spent on {spell_text}")
                }
                crate::ability::ManaSpendBonusCondition::WhenYouSpendThisManaToCast => {
                    format!("When you spend this mana to cast {spell_text}")
                }
            };

            let mut bonuses = Vec::new();
            if *grant_uncounterable {
                bonuses.push("that spell can't be countered".to_string());
            }
            bonuses.extend(enters_with_counters.iter().map(|(counter_type, count)| {
                let rendered = describe_mana_usage_etb_bonus(*counter_type, *count);
                if matches!(
                    condition,
                    crate::ability::ManaSpendBonusCondition::WhenYouSpendThisManaToCast
                ) {
                    rendered.replacen("that creature", "it", 1)
                } else {
                    rendered
                }
            }));
            let has_prior_permanent_bonus = !enters_with_counters.is_empty();
            for (ability, duration) in granted_abilities {
                let rendered = match (ability, duration) {
                    (
                        crate::static_abilities::StaticAbilityId::Haste,
                        crate::ability::ManaSpendAbilityGrantDuration::UntilEndOfTurn,
                    ) => "it gains haste until end of turn".to_string(),
                    (
                        crate::static_abilities::StaticAbilityId::Haste,
                        crate::ability::ManaSpendAbilityGrantDuration::UntilYourNextTurn,
                    ) => "it gains haste until your next turn".to_string(),
                    (
                        crate::static_abilities::StaticAbilityId::Hexproof,
                        crate::ability::ManaSpendAbilityGrantDuration::UntilEndOfTurn,
                    ) => "it gains hexproof until end of turn".to_string(),
                    (
                        crate::static_abilities::StaticAbilityId::Hexproof,
                        crate::ability::ManaSpendAbilityGrantDuration::UntilYourNextTurn,
                    ) => "it gains hexproof until your next turn".to_string(),
                    _ => continue,
                };
                if has_prior_permanent_bonus {
                    bonuses.push(
                        rendered
                            .strip_prefix("it ")
                            .unwrap_or(&rendered)
                            .to_string(),
                    );
                } else {
                    bonuses.push(rendered);
                }
            }
            bonuses.extend(granted_keywords.iter().map(|keyword| match keyword {
                crate::ability::ManaSpendGrantedKeyword::Riot => "it gains riot".to_string(),
            }));

            if bonuses.is_empty() {
                return None;
            }
            line.push_str(", ");
            line.push_str(&bonuses.join(" and "));
            Some(line)
        }
        crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
            spell_filter,
            ability_source_filter,
        } => {
            let spell_text =
                describe_mana_usage_spell_filter_target_with_options(spell_filter, false)?;
            let source_text = describe_mana_usage_ability_source_filter(ability_source_filter)?;
            Some(format!(
                "Spend this mana only to cast {spell_text} or activate an ability of {source_text}"
            ))
        }
        crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
            spell_filter,
        } => {
            let spell_text =
                describe_mana_usage_spell_filter_target_with_options(spell_filter, false)?;
            Some(format!(
                "Spend this mana only to cast {spell_text}, unlock a door, or turn a permanent face up"
            ))
        }
        crate::ability::ManaUsageRestriction::ActivateAbility => {
            Some("Spend this mana only to activate abilities".to_string())
        }
        crate::ability::ManaUsageRestriction::PaymentTransaction {
            restriction,
            on_spend,
        } => {
            if let Some(crate::ability::ManaPaymentPredicate::Purpose(
                crate::ability::ManaPaymentPurpose::CumulativeUpkeep,
            )) = restriction
            {
                return Some("Spend this mana only to pay cumulative upkeep costs".to_string());
            }
            if matches!(
                restriction,
                Some(crate::ability::ManaPaymentPredicate::CostContainsX)
            ) {
                return Some("Spend this mana only on costs that contain {X}".to_string());
            }
            let [payload] = on_spend.as_slice() else {
                return None;
            };
            let crate::ability::ManaPaymentPredicate::All(predicates) = &payload.predicate else {
                return None;
            };
            let filter = predicates.iter().find_map(|predicate| match predicate {
                crate::ability::ManaPaymentPredicate::SourceMatches(filter) => Some(filter),
                _ => None,
            })?;
            let mut spell_text =
                describe_mana_usage_spell_filter_target_with_options(filter, false)?;
            if predicates.iter().any(|predicate| {
                matches!(
                    predicate,
                    crate::ability::ManaPaymentPredicate::SharesCreatureTypeWithPayersCommander
                )
            }) {
                spell_text.push_str(" that shares a creature type with your commander");
            }
            let effects = payload.effects.all_effects();
            let [effect] = effects.as_slice() else {
                return None;
            };
            let tail = if let Some(scry) = effect.downcast_ref::<crate::effects::ScryEffect>() {
                match &scry.count {
                    crate::effect::Value::Fixed(1) => "scry 1".to_string(),
                    crate::effect::Value::CommanderCastCount(crate::target::PlayerFilter::You) => {
                        "scry X, where X is the number of times it's been cast from the command zone this game"
                            .to_string()
                    }
                    _ => return None,
                }
            } else if effect
                .downcast_ref::<crate::effects::CopySpellEffect>()
                .is_some()
            {
                "copy that spell and you may choose new targets for the copy".to_string()
            } else {
                return None;
            };
            Some(format!(
                "When that mana is spent to cast {spell_text}, {tail}"
            ))
        }
    }
}

pub(super) fn activated_mana_output_amount(
    activated: &crate::ability::ActivatedAbility,
) -> Option<i32> {
    if let Some(mana_output) = activated.mana_output.as_ref()
        && !mana_output.is_empty()
    {
        return Some(mana_output.len() as i32);
    }

    let mut total = 0;
    let mut found = false;
    for effect in activated.effects.flattened_default_effects() {
        if let Some(add_mana) = effect.downcast_ref::<crate::effects::AddManaEffect>() {
            total += add_mana.mana.len() as i32;
            found = true;
        }
        if let Some(add_any) = effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()
            && let Value::Fixed(amount) = &add_any.amount
            && *amount > 0
        {
            total += *amount;
            found = true;
        }
        if let Some(add_one) = effect.downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
            && let Value::Fixed(amount) = &add_one.amount
            && *amount > 0
        {
            total += *amount;
            found = true;
        }
        if let Some(add_chosen) =
            effect.downcast_ref::<crate::effects::AddManaOfChosenColorEffect>()
            && let Value::Fixed(amount) = &add_chosen.amount
            && *amount > 0
        {
            total += *amount;
            found = true;
        }
    }

    found.then_some(total)
}

pub(super) fn describe_mana_usage_spell_filter_target_with_options(
    filter: &ObjectFilter,
    pluralize_origin_spell: bool,
) -> Option<String> {
    if let Some(special) =
        describe_special_mana_usage_spell_filter_target(filter, pluralize_origin_spell)
    {
        return Some(special);
    }

    let mut described = describe_cast_limit_spell_filter(filter);
    if described.is_empty() {
        return None;
    }
    if pluralize_origin_spell {
        return Some(pluralize_cast_spell_description(&described));
    }
    if described == "spell" {
        return Some("a spell".to_string());
    }
    if let Some(singular) = described.strip_suffix(" spells") {
        described = format!("{singular} spell");
    }
    if described.starts_with("a ")
        || described.starts_with("an ")
        || described.starts_with("the ")
        || described.starts_with("your ")
    {
        return Some(described);
    }
    let article = if matches!(
        described.chars().next().map(|ch| ch.to_ascii_lowercase()),
        Some('a' | 'e' | 'i' | 'o' | 'u')
    ) {
        "an"
    } else {
        "a"
    };
    Some(format!("{article} {described}"))
}

pub(super) fn describe_mana_usage_ability_source_filter(filter: &ObjectFilter) -> Option<String> {
    let mut remainder = filter.clone();
    remainder.card_types.clear();
    remainder.subtypes.clear();
    remainder.supertypes.clear();
    if remainder != ObjectFilter::default() {
        return None;
    }

    let mut descriptors = Vec::new();
    descriptors.extend(
        filter
            .supertypes
            .iter()
            .map(|supertype| supertype.to_string()),
    );
    descriptors.extend(
        filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string()),
    );
    descriptors.extend(filter.subtypes.iter().map(|subtype| subtype.to_string()));

    if descriptors.is_empty() {
        return Some("a source".to_string());
    }

    let described = join_with_or(&descriptors);
    let article = if matches!(
        described.chars().next().map(|ch| ch.to_ascii_lowercase()),
        Some('a' | 'e' | 'i' | 'o' | 'u')
    ) {
        "an"
    } else {
        "a"
    };
    Some(format!("{article} {described} source"))
}

pub(super) fn describe_special_mana_usage_spell_filter_target(
    filter: &ObjectFilter,
    pluralize_origin_spell: bool,
) -> Option<String> {
    if filter
        == &ObjectFilter::default()
            .commander()
            .owned_by(PlayerFilter::You)
    {
        return Some("your commander".to_string());
    }
    if filter
        == &ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
    {
        if pluralize_origin_spell {
            return Some("spells from your graveyard".to_string());
        }
        return Some("a spell from your graveyard".to_string());
    }
    if filter == &ObjectFilter::default().in_zone(Zone::Exile) {
        return Some("spells from exile".to_string());
    }
    if filter
        == &ObjectFilter::default()
            .with_static_ability(crate::static_abilities::StaticAbilityId::MakeColorless)
    {
        return Some("a spell with devoid".to_string());
    }

    let mut creature_with_no_abilities =
        ObjectFilter::default().with_type(crate::types::CardType::Creature);
    creature_with_no_abilities.no_abilities = true;
    if filter == &creature_with_no_abilities {
        return Some("creature spells with no abilities".to_string());
    }

    if filter == &ObjectFilter::default().owned_by(PlayerFilter::NotYou) {
        return Some("spells you don't own".to_string());
    }

    if filter == &ObjectFilter::default().monocolored().of_chosen_color() {
        if pluralize_origin_spell {
            return Some("monocolored spells of that color".to_string());
        }
        return Some("a monocolored spell of that color".to_string());
    }

    None
}

pub(super) fn describe_mana_usage_spell_target(
    card_types: &[crate::types::CardType],
    subtype_requirement: Option<crate::ability::ManaUsageSubtypeRequirement>,
) -> Option<String> {
    let [card_type] = card_types else {
        return None;
    };
    let article = match card_type {
        crate::types::CardType::Artifact
        | crate::types::CardType::Enchantment
        | crate::types::CardType::Instant => "an",
        _ => "a",
    };
    let type_text = match card_type {
        crate::types::CardType::Artifact => "artifact",
        crate::types::CardType::Battle => "battle",
        crate::types::CardType::Creature => "creature",
        crate::types::CardType::Enchantment => "enchantment",
        crate::types::CardType::Instant => "instant",
        crate::types::CardType::Kindred => "kindred",
        crate::types::CardType::Land => "land",
        crate::types::CardType::Plane => "plane",
        crate::types::CardType::Phenomenon => "phenomenon",
        crate::types::CardType::Vanguard => "vanguard",
        crate::types::CardType::Scheme => "scheme",
        crate::types::CardType::Conspiracy => "conspiracy",
        crate::types::CardType::Planeswalker => "planeswalker",
        crate::types::CardType::Sorcery => "sorcery",
    };
    let mut text = format!("{article} {type_text} spell");
    if let Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource) =
        subtype_requirement
    {
        text.push_str(" of the chosen type");
    }
    Some(text)
}

pub(super) fn describe_mana_usage_etb_bonus(
    counter_type: crate::object::CounterType,
    count: u32,
) -> String {
    let counter_text = describe_counter_type(counter_type);
    if count == 1 {
        return format!("that creature enters with an additional {counter_text} counter on it");
    }
    let count_text = small_number_word(count).unwrap_or_else(|| count.to_string());
    format!("that creature enters with {count_text} additional {counter_text} counters on it")
}

pub(super) fn describe_mana_usage_static_ability_bonus(
    ability: crate::static_abilities::StaticAbilityId,
) -> Option<String> {
    match ability {
        crate::static_abilities::StaticAbilityId::Haste => {
            Some("it gains haste until end of turn".to_string())
        }
        _ => None,
    }
}

pub(crate) fn collect_activation_restriction_clauses(
    timing: &ActivationTiming,
    additional_restrictions: &[String],
) -> Vec<String> {
    let mut clauses = Vec::new();

    let timing_is_implied_by_presentation = *timing == ActivationTiming::DuringSourceOwnersUpkeep
        && additional_restrictions.iter().any(|restriction| {
            restriction
                .strip_prefix("__ironsmith_activation_label:")
                .is_some_and(|label| label.eq_ignore_ascii_case("Forecast"))
        });
    if !timing_is_implied_by_presentation
        && let Some(timing_clause) = describe_activation_timing_clause(timing)
    {
        let normalized = normalize_activation_restriction_clause(timing_clause);
        push_activation_restriction_clause(&mut clauses, normalized);
    }

    for raw in additional_restrictions {
        if raw.starts_with("__ironsmith_class_level:") {
            continue;
        }
        if raw.starts_with("__ironsmith_level_range:") {
            continue;
        }
        if raw.starts_with(STATION_THRESHOLD_RESTRICTION_PREFIX) {
            continue;
        }
        if raw.starts_with("__ironsmith_activation_label:") {
            continue;
        }
        if raw
            .to_ascii_lowercase()
            .contains("exhaust ability only once")
        {
            continue;
        }
        let normalized = normalize_activation_restriction_clause(raw);
        push_activation_restriction_clause(&mut clauses, normalized);
    }

    clauses
}

pub(crate) fn push_activation_restriction_clause(clauses: &mut Vec<String>, clause: String) {
    if clause.is_empty() {
        return;
    }
    let clause_lower = clause.to_ascii_lowercase();
    let mut remove_indices = Vec::new();
    for (idx, existing) in clauses.iter().enumerate() {
        let existing_lower = existing.to_ascii_lowercase();
        if existing_lower == clause_lower
            || activation_clause_is_more_specific(&existing_lower, &clause_lower)
        {
            return;
        }
        if activation_clause_is_more_specific(&clause_lower, &existing_lower) {
            remove_indices.push(idx);
        }
    }
    for idx in remove_indices.into_iter().rev() {
        clauses.remove(idx);
    }
    clauses.push(clause);
}

pub(crate) fn activation_clause_is_more_specific(candidate: &str, base: &str) -> bool {
    if candidate.len() <= base.len() || !candidate.starts_with(base) {
        return false;
    }
    let tail = candidate[base.len()..].trim_start();
    tail.starts_with(',')
        || tail.starts_with("and ")
        || tail.starts_with("before ")
        || tail.starts_with("after ")
        || tail.starts_with("if ")
        || tail.starts_with("unless ")
}

pub(crate) fn join_activation_restriction_clauses(clauses: &[String]) -> String {
    let mut iter = clauses.iter();
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut line = first.clone();
    for clause in iter {
        if let Some(rest) = clause.strip_prefix("Activate only ") {
            line.push_str(" and ");
            line.push_str(rest);
        } else {
            line.push_str(" and ");
            line.push_str(clause);
        }
    }
    line
}

pub(super) fn append_sentence_clause(line: &mut String, clause: &str) {
    if !line.is_empty() {
        if line.ends_with('.') || line.ends_with('!') || line.ends_with('?') {
            line.push(' ');
        } else {
            line.push_str(". ");
        }
    }
    line.push_str(clause);
}

pub(super) fn append_activation_clause(line: &mut String, clause: &str) {
    let Some(newline_idx) = line.find('\n') else {
        append_sentence_clause(line, clause);
        return;
    };

    let mut header = line[..newline_idx].trim_end().to_string();
    if let Some(stripped) = header.strip_suffix('—') {
        header = stripped.trim_end().to_string();
    }
    append_sentence_clause(&mut header, clause);
    header.push_str(&line[newline_idx..]);
    *line = header;
}

pub(super) fn describe_ward_blight_keyword(cost: &crate::cost::TotalCost) -> Option<String> {
    let [choose_cost, put_cost] = cost.as_all()? else {
        return None;
    };
    let choose = choose_cost
        .effect_ref()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_counters = put_cost
        .effect_ref()?
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if choose.filter != ObjectFilter::creature().you_control()
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || put_counters.counter_type != CounterType::MinusOneMinusOne
        || put_counters.target_count.is_some()
        || put_counters.distributed
    {
        return None;
    }
    let ChooseSpec::Tagged(tag) = &put_counters.target else {
        return None;
    };
    if *tag != choose.tag {
        return None;
    }
    Some(format!(
        "Ward—Blight {}",
        describe_value(&put_counters.amount)
    ))
}

pub(super) fn describe_structured_ward_cost(cost: &crate::cost::TotalCost) -> String {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            let parts = describe_cost_component_parts(costs);
            if parts.is_empty() {
                "Free".to_string()
            } else {
                parts.join(", ")
            }
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => branches
            .iter()
            .map(describe_structured_ward_cost)
            .collect::<Vec<_>>()
            .join(" or "),
    }
}

pub(crate) fn describe_keyword_ability(ability: &Ability) -> Option<String> {
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && triggered.intervening_if.is_none()
        && triggered.presentation_label.is_none()
        && matches!(
            triggered.effects.flattened_default_effects(),
            [effect] if effect
                .downcast_ref::<crate::effects::HauntExileEffect>()
                .is_some()
        )
    {
        return Some("Haunt".to_string());
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(annihilator) = describe_annihilator_keyword(triggered)
    {
        return Some(annihilator);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(myriad) = describe_myriad_keyword(triggered)
    {
        return Some(myriad);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(hideaway) = describe_structural_hideaway_keyword(triggered)
    {
        return Some(hideaway);
    }
    // Equip has a structural Oracle surface for alternative costs. Other
    // keyword recognizers below still operate only on one flat conjunction.
    if let AbilityKind::Activated(activated) = &ability.kind
        && activated.mana_cost.as_one_of().is_some()
    {
        return describe_structural_equip_keyword(activated);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(craft) = describe_structural_craft_keyword(ability, activated)
    {
        return Some(craft);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(cycling) = describe_structural_cycling_keyword(ability, activated)
    {
        return Some(cycling);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(transmute) = describe_structural_transmute_keyword(ability, activated)
    {
        return Some(transmute);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(transfigure) = describe_structural_transfigure_keyword(ability, activated)
    {
        return Some(transfigure);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(scavenge) = describe_structural_scavenge_keyword(ability, activated)
    {
        return Some(scavenge);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(embalm) = describe_structural_embalm_keyword(ability, activated)
    {
        return Some(embalm);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(eternalize) = describe_structural_eternalize_keyword(ability, activated)
    {
        return Some(eternalize);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(reconfigure) = describe_structural_reconfigure_keyword(activated)
    {
        return Some(reconfigure);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(equip) = describe_structural_equip_keyword(activated)
    {
        return Some(equip);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(outlast) = describe_structural_outlast_keyword(activated)
    {
        return Some(outlast);
    }
    if let AbilityKind::Activated(activated) = &ability.kind
        && let Some(crew) = describe_structural_crew_keyword(activated)
    {
        return Some(crew);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(exploit) = describe_structural_exploit_keyword(triggered)
    {
        return Some(exploit);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(extort) = describe_structural_extort_keyword(triggered)
    {
        return Some(extort);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(cumulative_upkeep) = describe_structural_cumulative_upkeep_keyword(triggered)
    {
        return Some(cumulative_upkeep);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(soulshift) = describe_structural_soulshift_keyword(triggered)
    {
        return Some(soulshift);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(fabricate) = describe_structural_fabricate_keyword(triggered)
    {
        return Some(fabricate);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(riot) = describe_structural_riot_keyword(triggered)
    {
        return Some(riot);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(mentor) = describe_structural_mentor_keyword(triggered)
    {
        return Some(mentor);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(battle_cry) = describe_structural_battle_cry_keyword(triggered)
    {
        return Some(battle_cry);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(bushido) = describe_structural_bushido_keyword(triggered)
    {
        return Some(bushido);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(frenzy) = describe_structural_frenzy_keyword(triggered)
    {
        return Some(frenzy);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(dethrone) = describe_structural_dethrone_keyword(triggered)
    {
        return Some(dethrone);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(exalted) = describe_structural_exalted_keyword(triggered)
    {
        return Some(exalted);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(recursive) = describe_structural_persist_or_undying_keyword(triggered)
    {
        return Some(recursive);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(afterlife) = describe_structural_afterlife_keyword(triggered)
    {
        return Some(afterlife);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(afflict) = describe_structural_afflict_keyword(triggered)
    {
        return Some(afflict);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(amplify) = describe_structural_amplify_keyword(triggered)
    {
        return Some(amplify);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(devour) = describe_structural_devour_keyword(triggered)
    {
        return Some(devour);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(prowess) = describe_structural_prowess_keyword(triggered)
    {
        return Some(prowess);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(toxic) = describe_structural_toxic_keyword(triggered)
    {
        return Some(toxic);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(casualty) = describe_structural_casualty_keyword(triggered)
    {
        return Some(casualty);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(storm) = describe_structural_storm_keyword(triggered)
    {
        return Some(storm);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(gravestorm) = describe_structural_gravestorm_keyword(triggered)
    {
        return Some(gravestorm);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(demonstrate) = describe_structural_demonstrate_keyword(triggered)
    {
        return Some(demonstrate);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(soulbond) = describe_structural_soulbond_keyword(triggered)
    {
        return Some(soulbond);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(mobilize) = describe_structural_mobilize_keyword(triggered)
    {
        return Some(mobilize);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(enlist) = describe_structural_enlist_keyword(triggered)
    {
        return Some(enlist);
    }
    if let AbilityKind::Triggered(triggered) = &ability.kind
        && let Some(provoke) = describe_structural_provoke_keyword(triggered)
    {
        return Some(provoke);
    }

    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    if let Some(cost) = static_ability.ward_cost() {
        if let Some(blight) = describe_ward_blight_keyword(cost) {
            return Some(blight);
        }
        let cost_text = describe_structured_ward_cost(cost);
        return Some(if cost.has_non_mana_costs() {
            format!("Ward—{cost_text}")
        } else {
            format!("Ward {cost_text}")
        });
    }
    if let Some(model) = static_ability.compiled_model() {
        let render_turn_face_up_keyword = |keyword: &str, cost: &crate::cost::TotalCost| {
            let separator = if cost.has_non_mana_costs() {
                "—"
            } else {
                " "
            };
            format!("{keyword}{separator}{}", describe_total_cost(cost))
        };
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::Morph(cost) => {
                return Some(render_turn_face_up_keyword("Morph", cost));
            }
            ironsmith_core::StaticAbilityPayload::Megamorph(cost) => {
                return Some(render_turn_face_up_keyword("Megamorph", cost));
            }
            ironsmith_core::StaticAbilityPayload::Disguise(cost) => {
                return Some(render_turn_face_up_keyword("Disguise", cost));
            }
            _ => {}
        }
    }
    let raw_text = static_ability.display();
    let raw_text = raw_text.trim();
    let text = raw_text.to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    if ability.functional_zones.contains(&Zone::Hand)
        && ability.functional_zones.contains(&Zone::Stack)
        && let Some(reduction) = static_ability.cost_reduction()
        && reduction.filter == ObjectFilter::default()
        && reduction.condition.is_none()
        && matches!(
            reduction.reduction,
            Value::CountPlayers(PlayerFilter::Opponent)
        )
    {
        return Some("Undaunted".to_string());
    }
    if words.len() == 4
        && words[0] == "cycling"
        && words[1] == "pay"
        && words[3] == "life"
        && (words[2].parse::<u32>().is_ok() || words[2] == "x")
    {
        return Some(format!("Cycling—Pay {} life", words[2]));
    }
    let is_equip_keyword = words.first().is_some_and(|word| {
        *word == "equip" || word.starts_with("equip—") || word.starts_with("equip-")
    });
    if is_equip_keyword {
        let mut rendered = if raw_text.eq_ignore_ascii_case("equip") {
            "Equip".to_string()
        } else {
            raw_text.to_string()
        };
        if let AbilityKind::Activated(activated) = &ability.kind {
            let mut restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            // Equip implies sorcery-speed by default; only surface extra restrictions.
            restriction_clauses
                .retain(|clause| !clause.eq_ignore_ascii_case("Activate only as a sorcery"));
            if !restriction_clauses.is_empty() {
                rendered.push_str(". ");
                rendered.push_str(&join_activation_restriction_clauses(&restriction_clauses));
            }
        }
        return Some(rendered);
    }
    if words.len() >= 2
        && words[0] == "level"
        && words[1] == "up"
        && !text.starts_with("level up abilities")
    {
        return Some(raw_text.to_string());
    }
    if text == "storm" {
        return Some("Storm".to_string());
    }
    if text == "gravestorm" {
        return Some("Gravestorm".to_string());
    }
    if text == "training" {
        return Some("Training".to_string());
    }
    if text == "battle cry" {
        return Some("Battle cry".to_string());
    }
    if text == "dethrone" {
        return Some("Dethrone".to_string());
    }
    if text == "melee" {
        return Some("Melee".to_string());
    }
    if text == "riot" {
        return Some("Riot".to_string());
    }
    if text == "daybound" {
        return Some("Daybound".to_string());
    }
    if text == "nightbound" {
        return Some("Nightbound".to_string());
    }
    if text == "provoke" {
        return Some("Provoke".to_string());
    }
    if text == "ravenous" && matches!(ability.kind, AbilityKind::Static(_)) {
        return Some("Ravenous".to_string());
    }
    if text == "for mirrodin!" {
        return Some("For Mirrodin!".to_string());
    }
    if text.starts_with("afterlife ")
        || text.starts_with("annihilator ")
        || text.starts_with("fabricate ")
        || text.starts_with("mobilize ")
    {
        return Some(raw_text.to_string());
    }
    if text == "toxic" || text.starts_with("toxic ") {
        return Some(raw_text.to_string());
    }
    let first_cycling_idx = words
        .iter()
        .position(|word| trim_cycling_punctuation(word).ends_with("cycling"));
    let is_cycling_clause = first_cycling_idx.is_some_and(|idx| {
        !words[..idx]
            .iter()
            .any(|word| matches!(*word, "has" | "have"))
    });
    if is_cycling_clause {
        let mut cycling_rendered = Vec::new();
        for (idx, word) in words.iter().enumerate() {
            let keyword = trim_cycling_punctuation(word);
            if !keyword.ends_with("cycling") {
                continue;
            }
            let next = words
                .get(idx + 1)
                .map(|next| trim_cycling_punctuation(next));
            let has_cost = next.is_none_or(is_cycling_cost_word);
            if !has_cost {
                continue;
            }
            let mut chars = keyword.chars();
            let mut base = match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => "Cycling".to_string(),
            };
            if keyword == "landcycling"
                && idx > 0
                && trim_cycling_punctuation(words[idx - 1]) == "basic"
            {
                base = "Basic landcycling".to_string();
            }
            let mut cost_tokens = Vec::new();
            let mut j = idx + 1;
            while let Some(word) = words.get(j).map(|word| trim_cycling_punctuation(word)) {
                if is_cycling_cost_word(word) {
                    cost_tokens.push(word);
                    j += 1;
                } else {
                    break;
                }
            }
            if cost_tokens.is_empty() {
                cycling_rendered.push(base);
            } else {
                let cost = cost_tokens
                    .iter()
                    .map(|word| render_cycling_cost_token(word))
                    .collect::<Vec<_>>()
                    .join("");
                cycling_rendered.push(format!("{} {}", base, cost));
            }
        }
        if !cycling_rendered.is_empty() {
            return Some(cycling_rendered.join(", "));
        }
    }
    if text == "prowess" {
        return Some("Prowess".to_string());
    }
    if text == "exalted" {
        return Some("Exalted".to_string());
    }
    if text == "persist" {
        return Some("Persist".to_string());
    }
    if text == "undying" {
        return Some("Undying".to_string());
    }
    if text.starts_with("bushido ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("frenzy ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("rampage ") {
        return Some(raw_text.to_string());
    }
    if text == "extort" {
        return Some(
            "Extort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)"
                .to_string(),
        );
    }
    if text == "partner" {
        return Some("Partner".to_string());
    }
    if text.starts_with("partner-")
        || text.starts_with("partner\u{2013}")
        || text.starts_with("partner\u{2014}")
    {
        return Some(raw_text.trim_end_matches('.').to_string());
    }
    if text.starts_with("partner with ") {
        return Some(raw_text.to_string());
    }
    if text == "assist" {
        return Some("Assist".to_string());
    }
    if text.starts_with("soulshift ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("scavenge ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("outlast ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("modular ") {
        return Some(raw_text.to_string());
    }
    if text == "modular—sunburst" || text == "modular-sunburst" {
        return Some("Modular—Sunburst".to_string());
    }
    if text.starts_with("graft ") {
        return Some(raw_text.to_string());
    }
    if text == "sunburst" {
        return Some("Sunburst".to_string());
    }
    if text.starts_with("fading ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("vanishing ") {
        return Some(raw_text.to_string());
    }
    None
}

pub(super) fn describe_structural_cycling_keyword(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !ability.functional_zones.contains(&Zone::Hand)
        || !matches!(activated.timing, ActivationTiming::AnyTime)
        || !activated.choices.is_empty()
    {
        return None;
    }
    if !activated
        .mana_cost
        .costs()
        .iter()
        .any(is_discard_this_card_cost)
        || !activated.mana_cost.costs().iter().any(is_cycle_event_cost)
    {
        return None;
    }
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };

    let keywords = if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        if draw.count != Value::Fixed(1) || draw.player != PlayerFilter::You {
            return None;
        }
        vec!["Cycling".to_string()]
    } else if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
        if search.destination != Zone::Hand
            || search.player != PlayerFilter::You
            || search.library_position_from_top.is_some()
        {
            return None;
        }
        cycling_keywords_for_search_filter(&search.filter)
    } else {
        return None;
    };
    if keywords.is_empty() {
        return None;
    }

    let cycling_bookkeeping_cost =
        |cost: &crate::costs::Cost| is_discard_this_card_cost(cost) || is_cycle_event_cost(cost);
    let uses_action_separator = activated
        .mana_cost
        .costs()
        .iter()
        .filter(|cost| !cycling_bookkeeping_cost(cost))
        .any(|cost| cost.mana_cost_ref().is_none() && cost.dynamic_mana_cost_ref().is_none());
    let cost_text = keyword_base_cost_text(activated.mana_cost.costs(), cycling_bookkeeping_cost)?;
    let rendered = keywords
        .into_iter()
        .map(|keyword| {
            if uses_action_separator {
                format!("{keyword}—{cost_text}")
            } else {
                format!("{keyword} {cost_text}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(rendered)
}

pub(super) fn describe_structural_craft_keyword(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !ability.functional_zones.contains(&Zone::Battlefield)
        || !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.choices.is_empty()
        || !activated.mana_cost.costs().iter().any(is_exile_source_cost)
        || !activated.mana_cost.costs().iter().any(is_craft_event_cost)
    {
        return None;
    }

    let material = activated
        .mana_cost
        .costs()
        .iter()
        .find_map(craft_material_cost)?;
    let effects = activated.effects.flattened_default_effects();
    if effects.len() != 2 {
        return None;
    }
    let returns_source = effects[0]
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some_and(|move_to_zone| {
            matches!(move_to_zone.target, ChooseSpec::Source)
                && move_to_zone.zone == Zone::Battlefield
                && matches!(
                    move_to_zone.battlefield_controller,
                    crate::effects::BattlefieldController::Owner
                )
                && move_to_zone.transfer_exiled_with_source_links
        });
    let transforms_source = effects[1]
        .downcast_ref::<crate::effects::TransformEffect>()
        .is_some_and(|transform| matches!(transform.target, ChooseSpec::Source));
    if !returns_source || !transforms_source {
        return None;
    }

    let cost_text = keyword_base_cost_text(activated.mana_cost.costs(), |cost| {
        is_exile_source_cost(cost)
            || is_craft_event_cost(cost)
            || craft_material_cost(cost).is_some()
    })?;
    Some(format!("Craft with {material} {cost_text}"))
}

pub(super) fn craft_material_cost(cost: &crate::costs::Cost) -> Option<String> {
    let exile = cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())?;
    if matches!(exile.spec, ChooseSpec::Source) {
        return None;
    }
    let ChooseSpec::Object(filter) = exile.spec.base() else {
        return None;
    };
    describe_craft_material_filter(filter, exile.spec.count())
}

pub(super) fn describe_craft_material_filter(
    filter: &ObjectFilter,
    count: ChoiceCount,
) -> Option<String> {
    if count == ChoiceCount::exactly(1) && is_craft_artifact_material_filter(filter) {
        return Some("artifact".to_string());
    }
    if count == ChoiceCount::exactly(1) && is_craft_creature_material_filter(filter) {
        return Some("creature".to_string());
    }
    if count == ChoiceCount::at_least(1) && is_craft_one_or_more_material_filter(filter) {
        return Some("one or more".to_string());
    }
    if count == ChoiceCount::at_least(4) && is_craft_red_spell_material_filter(filter) {
        return Some("four or more red instant and/or sorcery cards".to_string());
    }
    None
}

pub(super) fn is_craft_artifact_material_filter(filter: &ObjectFilter) -> bool {
    filter.any_of.len() == 2
        && filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Battlefield)
                && branch.controller == Some(PlayerFilter::You)
                && branch.owner.is_none()
                && branch.other
                && branch.card_types == vec![CardType::Artifact]
        })
        && filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Graveyard)
                && branch.owner == Some(PlayerFilter::You)
                && branch.controller.is_none()
                && branch.other
                && branch.card_types == vec![CardType::Artifact]
        })
}

pub(super) fn is_craft_creature_material_filter(filter: &ObjectFilter) -> bool {
    filter.any_of.len() == 2
        && filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Battlefield)
                && branch.controller == Some(PlayerFilter::You)
                && branch.owner.is_none()
                && !branch.other
                && branch.card_types == vec![CardType::Creature]
        })
        && filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Graveyard)
                && branch.owner == Some(PlayerFilter::You)
                && branch.controller.is_none()
                && !branch.other
                && branch.card_types == vec![CardType::Creature]
        })
}

pub(super) fn is_craft_one_or_more_material_filter(filter: &ObjectFilter) -> bool {
    filter.any_of.len() == 2
        && filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Battlefield)
                && branch.controller == Some(PlayerFilter::You)
                && branch.owner.is_none()
                && branch.other
                && branch.card_types.is_empty()
        })
        && filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Graveyard)
                && branch.owner == Some(PlayerFilter::You)
                && branch.controller.is_none()
                && branch.other
                && branch.card_types.is_empty()
        })
}

pub(super) fn is_craft_red_spell_material_filter(filter: &ObjectFilter) -> bool {
    filter.zone == Some(Zone::Graveyard)
        && filter.owner == Some(PlayerFilter::You)
        && filter.colors == Some(crate::color::ColorSet::RED)
        && filter.card_types.len() == 2
        && filter.card_types.contains(&CardType::Instant)
        && filter.card_types.contains(&CardType::Sorcery)
}

pub(super) fn describe_structural_transmute_keyword(
    _ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.choices.is_empty()
        || !activated
            .mana_cost
            .costs()
            .iter()
            .any(is_discard_this_card_cost)
    {
        return None;
    }
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let search = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()?;
    if search.destination != Zone::Hand
        || search.player != PlayerFilter::You
        || search.reveal
        || search.library_position_from_top.is_some()
    {
        return None;
    }
    if !matches!(
        search.filter.mana_value,
        Some(crate::filter::Comparison::EqualExpr(_))
    ) {
        return None;
    }
    let cost_text = keyword_base_cost_text(activated.mana_cost.costs(), is_discard_this_card_cost)?;
    Some(format!("Transmute {cost_text}"))
}

pub(super) fn describe_structural_transfigure_keyword(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !ability.functional_zones.contains(&Zone::Battlefield)
        || !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.choices.is_empty()
        || !activated
            .mana_cost
            .costs()
            .iter()
            .any(crate::costs::Cost::is_sacrifice_self)
    {
        return None;
    }
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let search = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()?;
    if search.destination != Zone::Battlefield
        || search.player != PlayerFilter::You
        || search.chooser != PlayerFilter::You
        || search.reveal
        || search.library_position_from_top.is_some()
        || search.filter.card_types != vec![CardType::Creature]
        || !matches!(
            search.filter.mana_value.as_ref(),
            Some(crate::filter::Comparison::EqualExpr(value))
                if matches!(value.unhinted(), Value::ManaValueOf(spec)
                    if matches!(spec.base(), ChooseSpec::Source))
        )
    {
        return None;
    }
    let cost_text =
        keyword_base_cost_text(activated.mana_cost.costs(), |cost| cost.is_sacrifice_self())?;
    Some(format!("Transfigure {cost_text}"))
}

pub(super) fn cycling_keywords_for_search_filter(filter: &ObjectFilter) -> Vec<String> {
    if filter.supertypes.contains(&Supertype::Basic)
        && filter.card_types.contains(&CardType::Land)
        && filter.subtypes.is_empty()
    {
        return vec!["Basic landcycling".to_string()];
    }

    let mut keywords = Vec::new();
    for subtype in &filter.subtypes {
        keywords.push(format!("{subtype}cycling"));
    }
    if keywords.is_empty() {
        for card_type in &filter.card_types {
            keywords.push(format!("{card_type:?}cycling"));
        }
    }
    if keywords.is_empty() {
        keywords.push("Cycling".to_string());
    }
    keywords
}

pub(super) fn describe_structural_equip_keyword(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !matches!(activated.timing, ActivationTiming::SorcerySpeed) {
        return None;
    }
    if !(activated.choices.is_empty()
        || (activated.choices.len() == 1 && is_target_creature_you_control(&activated.choices[0])))
    {
        return None;
    }
    if activated.effects.segments.len() != 1
        || !activated.effects.segments[0].self_replacements.is_empty()
        || activated.effects.segments[0].default_effects.len() != 1
    {
        return None;
    }
    let attach = activated.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::AttachToEffect>()?;
    if !is_target_creature_you_control(&attach.target) {
        return None;
    }

    let qualifier = equip_target_qualifier_text(&attach.target);
    if let Some(branches) = activated.mana_cost.as_one_of() {
        let keyword = qualifier
            .map(|qualifier| format!("Equip {qualifier}"))
            .unwrap_or_else(|| "Equip".to_string());
        let has_non_mana_branch = branches.iter().any(|branch| branch.has_non_mana_costs());
        let branches = branches
            .iter()
            .map(|branch| {
                let described = describe_total_cost(branch);
                if has_non_mana_branch && !branch.has_non_mana_costs() {
                    format!("pay {described}")
                } else if has_non_mana_branch {
                    lowercase_first(&described)
                } else {
                    described
                }
            })
            .collect::<Vec<_>>()
            .join(" or ");
        let mut rendered = if has_non_mana_branch {
            format!("{keyword}—{}", capitalize_first(&branches))
        } else {
            format!("{keyword} {branches}")
        };

        let mut restriction_clauses = collect_activation_restriction_clauses(
            &activated.timing,
            &activated.additional_restrictions,
        );
        restriction_clauses
            .retain(|clause| !clause.eq_ignore_ascii_case("Activate only as a sorcery"));
        if !restriction_clauses.is_empty() {
            rendered.push_str(". ");
            rendered.push_str(&join_activation_restriction_clauses(&restriction_clauses));
        }
        return Some(rendered);
    }

    let cost = describe_cost_list(activated.mana_cost.costs());
    let mut rendered = if cost.trim().is_empty() || cost.eq_ignore_ascii_case("Free") {
        "Equip {0}".to_string()
    } else if let Some(qualifier) = qualifier {
        format!("Equip {qualifier} {cost}")
    } else {
        format!("Equip {cost}")
    };

    let mut restriction_clauses = collect_activation_restriction_clauses(
        &activated.timing,
        &activated.additional_restrictions,
    );
    restriction_clauses.retain(|clause| !clause.eq_ignore_ascii_case("Activate only as a sorcery"));
    if !restriction_clauses.is_empty() {
        rendered.push_str(". ");
        rendered.push_str(&join_activation_restriction_clauses(&restriction_clauses));
    }

    Some(rendered)
}

pub(super) fn describe_structural_reconfigure_keyword(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !matches!(activated.timing, ActivationTiming::SorcerySpeed) || !activated.choices.is_empty()
    {
        return None;
    }
    if activated.effects.segments.len() != 1
        || !activated.effects.segments[0].self_replacements.is_empty()
        || activated.effects.segments[0].default_effects.len() != 1
    {
        return None;
    }
    let reconfigure = activated.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ReconfigureEffect>()?;
    if !is_target_creature_you_control(&reconfigure.target) {
        return None;
    }

    let cost = describe_cost_list(activated.mana_cost.costs());
    if cost.trim().is_empty() || cost.eq_ignore_ascii_case("Free") {
        Some("Reconfigure {0}".to_string())
    } else {
        Some(format!("Reconfigure {cost}"))
    }
}

pub(super) fn describe_structural_outlast_keyword(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.choices.is_empty()
        || !activated.mana_cost.costs().iter().any(|cost| {
            cost.effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
                .is_some_and(|tap| matches!(tap.target, ChooseSpec::Source))
        })
    {
        return None;
    }
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let counters = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if counters.counter_type != CounterType::PlusOnePlusOne
        || counters.amount != Value::Fixed(1)
        || !matches!(counters.target, ChooseSpec::Source)
    {
        return None;
    }
    let cost_text = keyword_base_cost_text(activated.mana_cost.costs(), |cost| {
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
            .is_some_and(|tap| matches!(tap.target, ChooseSpec::Source))
    })?;
    Some(format!("Outlast {cost_text}"))
}

pub(crate) fn describe_structural_crew_keyword(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !matches!(activated.timing, ActivationTiming::AnyTime) || !activated.choices.is_empty() {
        return None;
    }
    let crew_power = activated.mana_cost.costs().iter().find_map(|cost| {
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::CrewCostEffect>())
            .map(|crew| crew.required_power)
    })?;
    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let continuous = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(continuous.target, crate::continuous::EffectTarget::Source)
        || continuous.until != Until::EndOfTurn
        || continuous.modification
            != Some(crate::continuous::Modification::AddCardTypes(vec![
                CardType::Creature,
            ]))
        || !continuous.additional_modifications.is_empty()
    {
        return None;
    }
    Some(format!("Crew {crew_power}"))
}

pub(super) fn describe_structural_exploit_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [with_id_effect, if_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) || may.effects.len() != 1 {
        return None;
    }
    let sacrifice = may.effects[0].downcast_ref::<crate::effects::SacrificeEffect>()?;
    if sacrifice.player != PlayerFilter::You
        || sacrifice.count != Value::Fixed(1)
        || sacrifice.filter != ObjectFilter::creature()
    {
        return None;
    }

    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }
    let emit = if_effect.then[0].downcast_ref::<crate::effects::EmitKeywordActionEffect>()?;
    (emit.action == crate::events::KeywordActionKind::Exploit && emit.amount == 1)
        .then(|| "Exploit".to_string())
}

pub(super) fn describe_structural_extort_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let spell_cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    if spell_cast.caster != PlayerFilter::You
        || spell_cast.filter.is_some()
        || spell_cast.during_turn.is_some()
        || spell_cast.min_spells_this_turn.is_some()
        || spell_cast.exact_spells_this_turn.is_some()
        || spell_cast.from_not_hand
    {
        return None;
    }
    let rendered =
        super::ast_render::describe_resolution_program(&triggered.effects).to_ascii_lowercase();
    if rendered.contains("you may pay {w/b}")
        && rendered.contains("each opponent")
        && rendered.contains("loses 1 life")
        && (rendered.contains("you gain x life")
            || rendered.contains("you gain that much life")
            || rendered.contains("you gain 1 life"))
    {
        return Some(
            "Extort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)"
                .to_string(),
        );
    }
    None
}

pub(super) fn describe_structural_cumulative_upkeep_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let upkeep = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::You || !triggered.choices.is_empty() {
        return None;
    }
    let [put_age, cumulative] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let put_age = put_age.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put_age.counter_type != CounterType::Age
        || put_age.amount != Value::Fixed(1)
        || !matches!(put_age.target, ChooseSpec::Source)
    {
        return None;
    }
    let cumulative = cumulative.downcast_ref::<crate::effects::CumulativeUpkeepEffect>()?;
    if cumulative.player != PlayerFilter::You {
        return None;
    }
    let payment = cumulative_upkeep_payment_text(&cumulative.payment)?;
    if payment.starts_with('{') {
        Some(format!("Cumulative upkeep {payment}"))
    } else {
        Some(format!("Cumulative upkeep—{payment}"))
    }
}

pub(super) fn cumulative_upkeep_payment_text(payment: &[Effect]) -> Option<String> {
    if let Some(text) = cumulative_upkeep_chosen_put_counters_text(payment) {
        return Some(text);
    }
    if let Some(text) = cumulative_upkeep_chosen_sacrifice_text(payment) {
        return Some(text);
    }

    let mut parts = Vec::new();
    for effect in payment {
        if let Some(pay_mana) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
            parts.push(pay_mana.cost.to_oracle());
        } else if let Some(pay_life) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
            if matches!(
                pay_life.player,
                ChooseSpec::SourceController | ChooseSpec::SourceOwner
            ) || matches!(pay_life.player, ChooseSpec::Player(PlayerFilter::You))
            {
                parts.push(format!("Pay {} life", describe_value(&pay_life.amount)));
            }
        } else if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
            if matches!(
                lose_life.player,
                ChooseSpec::SourceController | ChooseSpec::SourceOwner
            ) || matches!(lose_life.player, ChooseSpec::Player(PlayerFilter::You))
            {
                parts.push(format!("Pay {} life", describe_value(&lose_life.amount)));
            }
        } else if let Some(put_counters) =
            effect.downcast_ref::<crate::effects::PutCountersEffect>()
        {
            parts.push(cumulative_upkeep_put_counters_text(put_counters)?);
        } else if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
            parts.push(cumulative_upkeep_sacrifice_text(
                &sacrifice.filter,
                &sacrifice.count,
            )?);
        } else if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        {
            parts.push(cumulative_upkeep_move_to_zone_text(move_to_zone)?);
        } else if let Some(apply_continuous) =
            effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
        {
            parts.push(describe_apply_continuous_effect(apply_continuous)?);
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" and "))
    }
}

pub(super) fn cumulative_upkeep_move_to_zone_text(
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if move_to_zone.zone != Zone::Library || move_to_zone.to_top {
        return None;
    }
    let filter = match move_to_zone.target.base() {
        ChooseSpec::Object(filter) => filter,
        _ => return None,
    };
    if filter.zone != Some(Zone::Graveyard) {
        return None;
    }
    let count = move_to_zone.target.count();
    if count.min == 0 || count.max != Some(count.min) {
        return None;
    }

    let count_text = ironsmith_core::cardinal_word(count.min as usize as u32)?;
    if count.min == 1 {
        Some("Put a card from a single graveyard on the bottom of its owner's library".to_string())
    } else {
        Some(format!(
            "Put {count_text} cards from a single graveyard on the bottom of their owner's library"
        ))
    }
}

pub(super) fn cumulative_upkeep_chosen_put_counters_text(payment: &[Effect]) -> Option<String> {
    let [choose, put_counters] = payment else {
        return None;
    };
    let choose = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_counters = put_counters.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(&put_counters.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }
    let counter_text = cumulative_counter_phrase(put_counters)?;
    let mut object_text = choose.filter.description();
    for prefix in ["target ", "a ", "an "] {
        if let Some(stripped) = object_text.strip_prefix(prefix) {
            object_text = stripped.to_string();
            break;
        }
    }
    if let Some(stripped) = object_text.strip_prefix("opponent's ") {
        object_text = format!("{stripped} an opponent controls");
    } else if let Some(stripped) = object_text.strip_prefix("your ") {
        object_text = format!("{stripped} you control");
    }
    Some(format!("Put {counter_text} on a {object_text}"))
}

pub(super) fn cumulative_upkeep_chosen_sacrifice_text(payment: &[Effect]) -> Option<String> {
    let [choose, sacrifice] = payment else {
        return None;
    };
    let choose = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice.downcast_ref::<crate::effects::SacrificeEffect>()?;
    if sacrifice.player != PlayerFilter::You || !matches!(sacrifice.count, Value::Fixed(1)) {
        return None;
    }
    if !sacrifice
        .filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == choose.tag.as_str())
    {
        return None;
    }
    cumulative_upkeep_sacrifice_text(&choose.filter, &Value::Fixed(1))
}

pub(super) fn cumulative_upkeep_sacrifice_text(
    filter: &ObjectFilter,
    count: &Value,
) -> Option<String> {
    let Value::Fixed(count) = count else {
        return None;
    };
    if *count <= 0 {
        return None;
    }
    let mut description = filter.description();
    if let Some(stripped) = description.strip_suffix(" you control") {
        description = stripped.to_string();
    }
    if *count == 1 {
        let article = if description.starts_with("a ")
            || description.starts_with("an ")
            || description.starts_with("another ")
            || description.starts_with("target ")
            || description.starts_with("this ")
        {
            ""
        } else {
            "a "
        };
        Some(format!("Sacrifice {article}{description}"))
    } else {
        Some(format!("Sacrifice {count} {description}"))
    }
}

pub(super) fn cumulative_counter_phrase(
    put_counters: &crate::effects::PutCountersEffect,
) -> Option<String> {
    if put_counters.distributed || put_counters.target_count.is_some() {
        return None;
    }
    let Value::Fixed(amount) = put_counters.amount else {
        return None;
    };
    if amount <= 0 {
        return None;
    }
    let counter = put_counters.counter_type.description();
    let noun = if amount == 1 { "counter" } else { "counters" };
    let amount_text = if amount == 1 {
        let article = match counter.chars().next().map(|ch| ch.to_ascii_lowercase()) {
            Some('a' | 'e' | 'i' | 'o' | 'u') => "an".to_string(),
            _ => "a".to_string(),
        };
        format!("{article} {counter}")
    } else {
        format!("{} {counter}", describe_value(&Value::Fixed(amount)))
    };
    Some(format!("{amount_text} {noun}"))
}

pub(super) fn cumulative_upkeep_put_counters_text(
    put_counters: &crate::effects::PutCountersEffect,
) -> Option<String> {
    if !matches!(put_counters.target, ChooseSpec::Source) {
        return None;
    }
    let counter_text = cumulative_counter_phrase(put_counters)?;
    Some(format!("Put {counter_text} on this creature"))
}

pub(super) fn describe_structural_riot_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let choose = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose.modes.len() != 2
        || choose.min != Value::Fixed(1)
        || choose.max != Value::Fixed(1)
        || choose.allow_repeat
    {
        return None;
    }
    let has_counter_mode = choose.modes.iter().any(
        |mode| matches!(mode.effects.as_slice(), [effect] if is_plus_one_counter_on_source(effect)),
    );
    let has_haste_mode = choose
        .modes
        .iter()
        .any(|mode| matches!(mode.effects.as_slice(), [effect] if is_source_permanent_haste_grant(effect)));
    (has_counter_mode && has_haste_mode).then(|| "Riot".to_string())
}

pub(super) fn describe_structural_fabricate_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let choose = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose.modes.len() != 2
        || choose.min != Value::Fixed(1)
        || choose.max != Value::Fixed(1)
        || choose.allow_repeat
    {
        return None;
    }

    let mut counter_amount = None;
    let mut token_amount = None;
    for mode in &choose.modes {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
            if put.counter_type != CounterType::PlusOnePlusOne
                || !matches!(put.target, ChooseSpec::Source)
                || put.target_count.is_some()
                || put.distributed
            {
                return None;
            }
            let Value::Fixed(amount) = put.amount else {
                return None;
            };
            counter_amount = Some(amount);
            continue;
        }
        if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
            if !is_fabricate_servo_token(create) {
                return None;
            }
            let Value::Fixed(amount) = create.count else {
                return None;
            };
            token_amount = Some(amount);
            continue;
        }
        return None;
    }

    let amount = counter_amount?;
    if amount <= 0 || token_amount != Some(amount) {
        return None;
    }
    Some(format!("Fabricate {amount}"))
}

pub(super) fn is_fabricate_servo_token(create: &crate::effects::CreateTokenEffect) -> bool {
    if create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || create.suppress_aura_attachment_choice
        || create.enters_tapped
        || create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
    {
        return false;
    }

    let token = &create.token;
    token.card.is_token
        && token.card.name == "Servo"
        && token.card.color_indicator.is_none()
        && token.card.card_types == [CardType::Artifact, CardType::Creature]
        && token.card.subtypes == [Subtype::Servo]
        && matches!(
            token.card.power_toughness,
            Some(crate::card::PowerToughness {
                power: crate::card::PtValue::Fixed(1),
                toughness: crate::card::PtValue::Fixed(1),
            })
        )
        && token.abilities.is_empty()
}

pub(super) fn endure_spirit_token_size(
    create: &crate::effects::CreateTokenEffect,
) -> Option<Value> {
    if create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || create.suppress_aura_attachment_choice
        || create.enters_tapped
        || create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
        || create.count != Value::Fixed(1)
    {
        return None;
    }

    let token = &create.token;
    if !token.card.is_token
        || token.card.name != "Spirit"
        || token.card.color_indicator != Some(crate::color::ColorSet::WHITE)
        || token.card.card_types != [CardType::Creature]
        || token.card.subtypes != [Subtype::Spirit]
        || !token.abilities.is_empty()
    {
        return None;
    }

    match token.card.power_toughness {
        Some(crate::card::PowerToughness {
            power: crate::card::PtValue::Fixed(power),
            toughness: crate::card::PtValue::Fixed(toughness),
        }) if power == toughness => Some(Value::Fixed(power)),
        _ => None,
    }
}

pub(super) fn describe_structural_afflict_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !matches!(
        triggered.presentation_label.as_ref(),
        Some(PresentationLabel::Keyword(PresentationKeyword::Afflict(_)))
    ) {
        return None;
    }
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisBecomesBlockedTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let lose = effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if !matches!(lose.player, ChooseSpec::Player(PlayerFilter::Defending)) {
        return None;
    }
    let Value::Fixed(amount) = lose.amount else {
        return None;
    };
    (amount > 0).then(|| format!("Afflict {amount}"))
}

pub(super) fn describe_structural_devour_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !matches!(
        triggered.presentation_label.as_ref(),
        Some(PresentationLabel::Keyword(PresentationKeyword::Devour(_)))
    ) {
        return None;
    }
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let devour = effect.downcast_ref::<crate::effects::DevourEffect>()?;
    Some(format!("Devour {}", devour.multiplier))
}

pub(super) fn describe_structural_amplify_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !matches!(
        triggered.presentation_label.as_ref(),
        Some(PresentationLabel::Keyword(PresentationKeyword::Amplify(_)))
    ) {
        return None;
    }
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let amplify = effect.downcast_ref::<crate::effects::AmplifyEffect>()?;
    Some(format!("Amplify {}", amplify.amount))
}

pub(super) fn describe_structural_mentor_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !trigger_is_this_attacks(&triggered.trigger)
        || triggered.choices.len() != 1
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::PlusOnePlusOne
        || put.amount != Value::Fixed(1)
        || put.target_count.is_some()
        || put.distributed
        || !is_mentor_target(&put.target)
    {
        return None;
    }
    if !triggered.choices.iter().any(is_mentor_target) {
        return None;
    }
    Some("Mentor".to_string())
}

pub(super) fn is_mentor_target(target: &ChooseSpec) -> bool {
    let ChooseSpec::Target(target) = target else {
        return false;
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        return false;
    };
    filter.zone == Some(Zone::Battlefield)
        && filter.card_types == [CardType::Creature]
        && filter.attacking
        && filter.power_relative_to_source
            == Some(crate::filter::SourcePowerRelation::LessThanSource)
}

pub(super) fn describe_structural_battle_cry_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_attacks(&triggered.trigger)
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let each = effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if each.filter.zone != Some(Zone::Battlefield)
        || each.filter.controller != Some(PlayerFilter::You)
        || each.filter.card_types != [CardType::Creature]
        || !each.filter.other
        || !each.filter.attacking
    {
        return None;
    }
    let [pump] = each.effects.as_slice() else {
        return None;
    };
    let pump = pump.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()?;
    if matches!(pump.target, ChooseSpec::Iterated)
        && pump.power == Value::Fixed(1)
        && pump.toughness == Value::Fixed(0)
        && matches!(pump.duration, Until::EndOfTurn)
    {
        Some("Battle cry".to_string())
    } else {
        None
    }
}

pub(super) fn describe_structural_bushido_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_blocks_or_becomes_blocked(&triggered.trigger)
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let pump = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()?;
    let Value::Fixed(power) = pump.power else {
        return None;
    };
    if power <= 0
        || pump.toughness != Value::Fixed(power)
        || !matches!(pump.target, ChooseSpec::Source)
        || !matches!(pump.duration, Until::EndOfTurn)
    {
        return None;
    }
    Some(format!("Bushido {power}"))
}

pub(super) fn describe_structural_frenzy_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksAndIsntBlockedTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let pump = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()?;
    let Value::Fixed(power) = pump.power else {
        return None;
    };
    if power <= 0
        || pump.toughness != Value::Fixed(0)
        || !matches!(pump.target, ChooseSpec::Source)
        || !matches!(pump.duration, Until::EndOfTurn)
    {
        return None;
    }
    Some(format!("Frenzy {power}"))
}

pub(super) fn describe_structural_dethrone_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisAttacksPlayerWithMostLifeTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    is_plus_one_counter_on_source(effect).then(|| "Dethrone".to_string())
}

pub(super) fn trigger_is_this_attacks(trigger: &crate::triggers::Trigger) -> bool {
    trigger
        .downcast_ref::<crate::triggers::ThisAttacksTrigger>()
        .is_some()
        || trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
            .is_some()
        || trigger.display() == "Whenever this creature attacks"
}

pub(super) fn describe_structural_enlist_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_attacks(&triggered.trigger)
    {
        return None;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [tag_attacker, choose_creature, tap_creature, pump_attacker] = may.effects.as_slice()
    else {
        return None;
    };

    let tag_attacker = tag_attacker.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if tag_attacker.tag.as_str() != "enlist_attacker" {
        return None;
    }

    let choose_creature = choose_creature.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_creature.chooser != PlayerFilter::You
        || choose_creature.tag.as_str() != "enlisted_creature"
        || choose_creature.count.min != 1
        || choose_creature.count.max != Some(1)
        || choose_creature.count.dynamic_x
        || choose_creature.count.up_to_x
        || choose_creature.count.random
        || choose_creature.filter.controller != Some(PlayerFilter::You)
        || !choose_creature
            .filter
            .card_types
            .contains(&CardType::Creature)
        || !choose_creature.filter.other
        || !choose_creature.filter.nonattacking
    {
        return None;
    }

    let tap_creature = tap_creature.downcast_ref::<crate::effects::TapEffect>()?;
    if !matches!(&tap_creature.target, ChooseSpec::Tagged(tag) if tag.as_str() == "enlisted_creature")
    {
        return None;
    }

    let pump_attacker =
        pump_attacker.downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()?;
    if !matches!(&pump_attacker.target, ChooseSpec::Tagged(tag) if tag.as_str() == "enlist_attacker")
        || pump_attacker.power_per != 1
        || pump_attacker.toughness_per != 0
        || pump_attacker.duration != Until::EndOfTurn
        || !matches!(
            &pump_attacker.count,
            Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == "enlisted_creature")
        )
    {
        return None;
    }

    Some("Enlist".to_string())
}

pub(super) fn describe_structural_provoke_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !trigger_is_this_attacks(&triggered.trigger)
        || triggered.choices.len() != 1
    {
        return None;
    }
    let ChooseSpec::Target(target) = &triggered.choices[0] else {
        return None;
    };
    let ChooseSpec::Object(choice_filter) = target.as_ref() else {
        return None;
    };
    if choice_filter.controller != Some(PlayerFilter::Defending)
        || choice_filter.card_types != vec![CardType::Creature]
    {
        return None;
    }

    let flattened = triggered.effects.flattened_default_effects();
    let effects = if flattened.is_empty() && triggered.effects.segments.len() == 1 {
        triggered.effects.segments[0].default_effects.as_slice()
    } else {
        flattened
    };
    let Some(untap_effect) = effects.first() else {
        return None;
    };
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if !matches!(untap.target, ChooseSpec::Target(_)) {
        return None;
    }

    effects
        .iter()
        .skip(1)
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .any(|apply| {
            apply.until == Until::EndOfCombat
                && matches!(apply.target, crate::continuous::EffectTarget::Source)
                && matches!(apply.target_spec.as_ref(), Some(ChooseSpec::Target(_)))
                && matches!(
                    &apply.modification,
                    Some(crate::continuous::Modification::AddAbility(ability))
                        if ability.id() == crate::static_abilities::StaticAbilityId::MustBlock
                )
        })
        .then(|| "Provoke".to_string())
}

pub(super) fn describe_structural_soulshift_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some() || triggered.choices.len() != 1 {
        return None;
    }

    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()?;
    if !zone_change.this_object
        || zone_change.from
            != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || zone_change.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        || zone_change.player != crate::triggers::zone_changes::PlayerRelation::Any
        || zone_change.count_mode != crate::triggers::zone_changes::CountMode::Each
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let return_effect = effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_effect.random {
        return None;
    }

    let amount = soulshift_target_amount_text(&return_effect.target)?;
    if soulshift_target_amount_text(&triggered.choices[0]) != Some(amount.clone()) {
        return None;
    }
    Some(format!("Soulshift {amount}"))
}

pub(super) fn soulshift_target_amount_text(spec: &ChooseSpec) -> Option<String> {
    let ChooseSpec::WithCount(inner, count) = spec else {
        return None;
    };
    if count.min != 0 || count.max != Some(1) || count.dynamic_x || count.up_to_x || count.random {
        return None;
    }
    let ChooseSpec::Target(target) = inner.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        return None;
    };
    if filter.zone != Some(Zone::Graveyard)
        || filter.owner != Some(PlayerFilter::You)
        || !filter.subtypes.contains(&Subtype::Spirit)
    {
        return None;
    }
    match &filter.mana_value {
        Some(crate::filter::Comparison::LessThanOrEqual(amount)) if *amount >= 0 => {
            Some(amount.to_string())
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => {
            describe_where_x_basis(value).map(|basis| format!("X, where X is {basis}"))
        }
        _ => None,
    }
}

pub(super) fn trigger_is_this_enters_battlefield(trigger: &crate::triggers::Trigger) -> bool {
    trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .is_some_and(|zone_change| {
            zone_change.this_object
                && zone_change.to.matches(Zone::Battlefield)
                && zone_change.cause_filter.is_none()
        })
}

pub(super) fn trigger_is_this_dies(trigger: &crate::triggers::Trigger) -> bool {
    trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .is_some_and(|zone_change| {
            zone_change.this_object
                && zone_change.from.matches(Zone::Battlefield)
                && zone_change.to.matches(Zone::Graveyard)
                && zone_change.cause_filter.is_none()
        })
}

pub(super) fn trigger_is_state_based(trigger: &crate::triggers::Trigger) -> bool {
    trigger
        .downcast_ref::<crate::triggers::StateTrigger>()
        .is_some()
}

pub(super) fn retain_state_trigger_residual_condition(
    trigger: &crate::triggers::Trigger,
    condition: Option<Condition>,
) -> Option<Condition> {
    if !trigger_is_state_based(trigger) {
        return condition;
    }

    let condition = condition?;
    let trigger_surface = trigger.display().to_ascii_lowercase();
    let mut conjuncts = Vec::new();
    flatten_condition_and_expr(&condition, &mut conjuncts);

    let mut found_embedded_state = false;
    let residual = conjuncts
        .into_iter()
        .filter(|conjunct| {
            let described = describe_condition(conjunct).to_ascii_lowercase();
            let embedded = !described.trim().is_empty()
                && trigger_surface.contains(described.trim().trim_end_matches('.'));
            found_embedded_state |= embedded;
            !embedded
        })
        .collect::<Vec<_>>();

    found_embedded_state
        .then(|| fold_condition_exprs(residual))
        .flatten()
}

pub(super) fn trigger_is_this_blocks_or_becomes_blocked(
    trigger: &crate::triggers::Trigger,
) -> bool {
    let Some(or_trigger) = trigger.downcast_ref::<crate::triggers::OrTrigger>() else {
        return false;
    };
    if or_trigger.triggers.len() != 2 {
        return false;
    }
    let has_blocks = or_trigger.triggers.iter().any(|trigger| {
        trigger
            .downcast_ref::<crate::triggers::ThisBlocksTrigger>()
            .is_some()
    });
    let has_becomes_blocked = or_trigger.triggers.iter().any(|trigger| {
        trigger
            .downcast_ref::<crate::triggers::ThisBecomesBlockedTrigger>()
            .is_some()
    });
    has_blocks && has_becomes_blocked
}

pub(super) fn is_plus_one_counter_on_source(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .is_some_and(|put| {
            put.counter_type == CounterType::PlusOnePlusOne
                && put.amount == Value::Fixed(1)
                && matches!(put.target, ChooseSpec::Source)
                && put.target_count.is_none()
                && !put.distributed
        })
}

pub(super) fn is_source_permanent_haste_grant(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .is_some_and(|apply| {
            matches!(apply.target_spec.as_ref(), Some(ChooseSpec::Source))
                && matches!(apply.until, Until::Forever)
                && apply_continuous_adds_static_ability(
                    apply,
                    crate::static_abilities::StaticAbilityId::Haste,
                )
        })
}

pub(super) fn apply_continuous_adds_static_ability(
    apply: &crate::effects::ApplyContinuousEffect,
    id: crate::static_abilities::StaticAbilityId,
) -> bool {
    let mut saw = false;
    let mut visit = |modification: &crate::continuous::Modification| match modification {
        crate::continuous::Modification::AddAbility(ability) => {
            saw |= ability.id() == id;
            true
        }
        crate::continuous::Modification::AddAbilityGeneric(ability) => match &ability.kind {
            AbilityKind::Static(static_ability) => {
                saw |= static_ability.id() == id;
                true
            }
            _ => false,
        },
        _ => false,
    };
    if let Some(modification) = &apply.modification
        && !visit(modification)
    {
        return false;
    }
    for modification in &apply.additional_modifications {
        if !visit(modification) {
            return false;
        }
    }
    saw
}

pub(super) fn describe_structural_exalted_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.trigger.display() != "Whenever a creature you control attacks alone"
        || !triggered
            .trigger
            .downcast_ref::<crate::triggers::AttacksAloneTrigger>()
            .is_some()
    {
        return None;
    }
    let [tag, pump] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tag = tag.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if tag.tag.as_str() != "exalted_attacker" {
        return None;
    }
    let pump = pump.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()?;
    if pump.power == Value::Fixed(1)
        && pump.toughness == Value::Fixed(1)
        && matches!(pump.duration, Until::EndOfTurn)
        && matches!(&pump.target, ChooseSpec::Tagged(tag) if tag.as_str() == "exalted_attacker")
    {
        Some("Exalted".to_string())
    } else {
        None
    }
}

pub(crate) fn describe_structural_prowess_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.trigger.display() != "Whenever you cast a noncreature spell"
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::SpellCastTrigger>()
            .is_none()
    {
        return None;
    }
    let [pump] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let pump = pump.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()?;
    if pump.power == Value::Fixed(1)
        && pump.toughness == Value::Fixed(1)
        && matches!(pump.duration, Until::EndOfTurn)
        && matches!(pump.target, ChooseSpec::Source)
    {
        Some("Prowess".to_string())
    } else {
        None
    }
}

pub(super) fn describe_structural_persist_or_undying_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !triggered.choices.is_empty() || triggered.trigger.display() != "When this creature dies" {
        return None;
    }
    let counter_type = match triggered.intervening_if.as_ref()? {
        Condition::Not(condition) => match condition.as_ref() {
            Condition::TriggeringObjectHadCounters {
                counter_type,
                min_count: 1,
            } => *counter_type,
            _ => return None,
        },
        _ => return None,
    };
    let [tag, _choose, _move, counters] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tag = tag.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let expected_tag = match counter_type {
        CounterType::MinusOneMinusOne => "persist_trigger",
        CounterType::PlusOnePlusOne => "undying_trigger",
        _ => return None,
    };
    if tag.tag.as_str() != expected_tag {
        return None;
    }
    let counters = counters.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put] = counters.effects.as_slice() else {
        return None;
    };
    let put = put.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != counter_type
        || put.amount != Value::Fixed(1)
        || !matches!(put.target, ChooseSpec::Iterated)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }
    match counter_type {
        CounterType::MinusOneMinusOne => Some("Persist".to_string()),
        CounterType::PlusOnePlusOne => Some("Undying".to_string()),
        _ => None,
    }
}

pub(super) fn describe_structural_afterlife_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.trigger.display() != "When this creature dies"
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let create = effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || create.suppress_aura_attachment_choice
        || create.enters_tapped
        || create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
    {
        return None;
    }
    let Value::Fixed(amount) = create.count else {
        return None;
    };
    if amount <= 0 {
        return None;
    }
    let token = &create.token;
    if !token.card.is_token
        || token.card.name != "Spirit"
        || token.card.color_indicator
            != Some(crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLACK))
        || token.card.card_types != [CardType::Creature]
        || token.card.subtypes != [Subtype::Spirit]
        || !matches!(
            token.card.power_toughness,
            Some(crate::card::PowerToughness {
                power: crate::card::PtValue::Fixed(1),
                toughness: crate::card::PtValue::Fixed(1),
            })
        )
        || token.abilities.len() != 1
        || !matches!(
            token.abilities[0].kind,
            AbilityKind::Static(ref static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::Flying
        )
    {
        return None;
    }
    Some(format!("Afterlife {amount}"))
}

pub(crate) fn describe_structural_toxic_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let keyword = match triggered.presentation_label.as_ref() {
        Some(PresentationLabel::Keyword(PresentationKeyword::Toxic(_))) => "Toxic",
        Some(PresentationLabel::Keyword(PresentationKeyword::Poisonous(_))) => "Poisonous",
        _ => return None,
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let poison = effect.downcast_ref::<crate::effects::PoisonCountersEffect>()?;
    if poison.player != PlayerFilter::DamagedPlayer {
        return None;
    }
    let Value::Fixed(amount) = poison.count else {
        return None;
    };
    (amount > 0).then(|| format!("{keyword} {amount}"))
}

pub(super) fn describe_structural_storm_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
            .is_none()
    {
        return None;
    }

    let [copy, choose_targets] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let copy = copy.downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy_spell = copy
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if !matches!(copy_spell.target, ChooseSpec::Source)
        || copy_spell.count != Value::SpellsCastBeforeThisTurn(PlayerFilter::You)
        || copy_spell.copier != PlayerFilter::You
        || !copy_spell.removed_supertypes.is_empty()
    {
        return None;
    }
    let choose_targets = choose_targets.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if choose_targets.from_effect != copy.id
        || !choose_targets.may
        || choose_targets.chooser.is_some()
    {
        return None;
    }
    Some("Storm".to_string())
}

pub(super) fn describe_structural_gravestorm_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
            .is_none()
    {
        return None;
    }

    let [copy, choose_targets] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let copy = copy.downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy_spell = copy
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if !matches!(copy_spell.target, ChooseSpec::Source)
        || copy_spell.count
            != Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Died(
                crate::target::ObjectFilter::default(),
            ))
        || copy_spell.copier != PlayerFilter::You
        || !copy_spell.removed_supertypes.is_empty()
    {
        return None;
    }
    let choose_targets = choose_targets.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if choose_targets.from_effect != copy.id
        || !choose_targets.may
        || choose_targets.chooser.is_some()
    {
        return None;
    }
    Some("Gravestorm".to_string())
}

pub(super) fn describe_structural_demonstrate_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
            .is_none()
    {
        return None;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [
        copy_you,
        choose_opponent,
        copy_opponent,
        retarget_you,
        retarget_opponent,
    ] = may.effects.as_slice()
    else {
        return None;
    };

    let copy_you = copy_you.downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy_you_spell = copy_you
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy_you.id != crate::effect::EffectId(0)
        || !matches!(copy_you_spell.target, ChooseSpec::Source)
        || copy_you_spell.count != Value::Fixed(1)
        || copy_you_spell.copier != PlayerFilter::You
        || !copy_you_spell.removed_supertypes.is_empty()
    {
        return None;
    }

    let choose_opponent = choose_opponent.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    if choose_opponent.chooser != PlayerFilter::You
        || choose_opponent.filter != PlayerFilter::Opponent
    {
        return None;
    }
    let opponent = PlayerFilter::TaggedPlayer(choose_opponent.tag.clone());

    let copy_opponent = copy_opponent.downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy_opponent_spell = copy_opponent
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy_opponent.id != crate::effect::EffectId(1)
        || !matches!(copy_opponent_spell.target, ChooseSpec::Source)
        || copy_opponent_spell.count != Value::Fixed(1)
        || copy_opponent_spell.copier != opponent
        || !copy_opponent_spell.removed_supertypes.is_empty()
    {
        return None;
    }

    let retarget_you = retarget_you.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if retarget_you.from_effect != copy_you.id
        || !retarget_you.may
        || retarget_you.chooser.as_ref() != Some(&PlayerFilter::You)
    {
        return None;
    }
    let retarget_opponent =
        retarget_opponent.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if retarget_opponent.from_effect != copy_opponent.id
        || !retarget_opponent.may
        || retarget_opponent.chooser.as_ref() != Some(&opponent)
    {
        return None;
    }

    Some("Demonstrate".to_string())
}

pub(super) fn describe_structural_soulbond_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
        return None;
    }
    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !matches!(zone_change.from, crate::triggers::ZonePattern::Any)
        || !matches!(
            zone_change.to,
            crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        )
        || !matches!(zone_change.player, crate::triggers::PlayerRelation::Any)
        || zone_change.cause_filter.is_some()
        || !matches!(
            zone_change.count_mode,
            crate::triggers::zone_changes::CountMode::Each
        )
        || zone_change.this_object
    {
        return None;
    }

    let mut expected_filter = ObjectFilter::default();
    expected_filter.zone = Some(Zone::Battlefield);
    expected_filter.card_types.push(CardType::Creature);
    expected_filter.controller = Some(PlayerFilter::You);
    if zone_change.object_filter != expected_filter {
        return None;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    effect
        .downcast_ref::<crate::effects::SoulbondPairEffect>()
        .is_some()
        .then(|| "Soulbond".to_string())
}

pub(super) fn describe_structural_mobilize_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisAttacksTrigger>()
            .is_none()
    {
        return None;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let create = effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let Value::Fixed(amount) = create.count else {
        return None;
    };
    if amount <= 0
        || create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || !create.enters_tapped
        || !create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || !create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
        || !describe_token_blueprint(&create.token)
            .eq_ignore_ascii_case("1/1 red Warrior creature token")
    {
        return None;
    }
    Some(format!("Mobilize {amount}"))
}

pub(super) fn describe_structural_casualty_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [sacrifice, copy, choose_targets] = may.effects.as_slice() else {
        return None;
    };
    let sacrifice = sacrifice.downcast_ref::<crate::effects::SacrificeEffect>()?;
    if sacrifice.player != PlayerFilter::You || sacrifice.count != Value::Fixed(1) {
        return None;
    }
    let power = match sacrifice.filter.power {
        Some(crate::filter::Comparison::GreaterThanOrEqual(power)) if power >= 0 => power,
        _ => return None,
    };
    if !sacrifice.filter.card_types.contains(&CardType::Creature)
        || sacrifice.filter.controller != Some(PlayerFilter::You)
    {
        return None;
    }
    let copy = copy.downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy_spell = copy
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if !matches!(copy_spell.target, ChooseSpec::Source)
        || copy_spell.count != Value::Fixed(1)
        || copy_spell.copier != PlayerFilter::You
    {
        return None;
    }
    let choose_targets = choose_targets.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if choose_targets.from_effect != copy.id || !choose_targets.may {
        return None;
    }
    Some(format!("Casualty {power}"))
}

pub(super) fn keyword_base_cost_text(
    costs: &[crate::costs::Cost],
    skip: impl Fn(&crate::costs::Cost) -> bool,
) -> Option<String> {
    let included = costs
        .iter()
        .filter(|cost| !skip(cost))
        .cloned()
        .collect::<Vec<_>>();
    let parts = describe_cost_component_parts(&included)
        .into_iter()
        .map(|part| part.trim().trim_end_matches(',').to_string())
        .filter(|part| !part.is_empty() && !part.starts_with("Effect("))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

pub(super) fn is_discard_this_card_cost(cost: &crate::costs::Cost) -> bool {
    let Some(discard) = cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
    else {
        return false;
    };
    discard.count == Value::Fixed(1)
        && discard.player == PlayerFilter::You
        && !discard.random
        && discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| filter.source && filter.zone == Some(Zone::Hand))
}

pub(super) fn is_cycle_event_cost(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>())
        .is_some_and(|emit| {
            emit.action == crate::events::KeywordActionKind::Cycle && emit.amount == 1
        })
}

pub(super) fn is_craft_event_cost(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>())
        .is_some_and(|emit| {
            emit.action == crate::events::KeywordActionKind::Craft && emit.amount == 1
        })
}

pub(super) fn is_exile_source_cost(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
        .is_some_and(|exile| matches!(exile.spec, ChooseSpec::Source) && !exile.face_down)
}

pub(super) fn is_target_creature_spec(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => is_target_creature_spec(inner),
        ChooseSpec::WithCount(inner, count) if count.is_single() => is_target_creature_spec(inner),
        ChooseSpec::Object(filter) => {
            filter.zone == Some(Zone::Battlefield)
                && filter.card_types.contains(&CardType::Creature)
                && filter.controller.is_none()
        }
        _ => false,
    }
}

pub(super) fn describe_structural_scavenge_keyword(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !ability.functional_zones.contains(&Zone::Graveyard)
        || !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
        || activated.choices.len() != 1
        || !is_target_creature_spec(&activated.choices[0])
    {
        return None;
    }

    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::PlusOnePlusOne
        || put.amount != Value::SourcePower
        || !is_target_creature_spec(&put.target)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }

    let costs = activated.mana_cost.costs();
    if !costs.iter().any(is_exile_source_cost) {
        return None;
    }
    let cost = keyword_base_cost_text(costs, is_exile_source_cost)?;
    Some(format!("Scavenge {cost}"))
}

pub(super) fn describe_structural_embalm_keyword(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !ability.functional_zones.contains(&Zone::Graveyard)
        || !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
        || !activated.choices.is_empty()
    {
        return None;
    }

    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let create = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if !matches!(create.target.unhinted(), ChooseSpec::Source)
        || create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || create.set_colors != Some(crate::color::ColorSet::WHITE)
        || !create.added_subtypes.contains(&Subtype::Zombie)
        || !create.clear_mana_cost
    {
        return None;
    }

    let costs = activated.mana_cost.costs();
    if !costs.iter().any(is_exile_source_cost) {
        return None;
    }
    let cost = keyword_base_cost_text(costs, is_exile_source_cost)?;
    Some(format!("Embalm {cost}"))
}

pub(super) fn describe_structural_eternalize_keyword(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if !ability.functional_zones.contains(&Zone::Graveyard)
        || !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
        || !activated.choices.is_empty()
    {
        return None;
    }

    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let create = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if !matches!(create.target.unhinted(), ChooseSpec::Source)
        || create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || create.set_colors != Some(crate::color::ColorSet::BLACK)
        || !create.added_subtypes.contains(&Subtype::Zombie)
        || create.set_base_power_toughness != Some((4, 4))
        || !create.clear_mana_cost
    {
        return None;
    }

    let costs = activated.mana_cost.costs();
    if !costs.iter().any(is_exile_source_cost) {
        return None;
    }
    let cost = keyword_base_cost_text(costs, is_exile_source_cost)?;
    Some(format!("Eternalize {cost}"))
}

pub(super) fn is_target_creature_you_control(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => is_target_creature_you_control(inner),
        ChooseSpec::WithCount(inner, count) if count.is_single() => {
            is_target_creature_you_control(inner)
        }
        ChooseSpec::Object(filter) => {
            filter.zone == Some(Zone::Battlefield)
                && filter.controller == Some(PlayerFilter::You)
                && filter.card_types.contains(&CardType::Creature)
        }
        _ => false,
    }
}

pub(super) fn equip_target_qualifier_text(spec: &ChooseSpec) -> Option<String> {
    match spec {
        ChooseSpec::Target(inner) => equip_target_qualifier_text(inner),
        ChooseSpec::WithCount(inner, count) if count.is_single() => {
            equip_target_qualifier_text(inner)
        }
        ChooseSpec::Object(filter) => {
            if filter.zone != Some(Zone::Battlefield)
                || filter.controller != Some(PlayerFilter::You)
                || !filter.card_types.contains(&CardType::Creature)
            {
                return None;
            }
            if filter.subtypes.len() == 1 {
                return Some(filter.subtypes[0].to_string());
            }
            if filter.subtypes.len() > 1 {
                let names = filter
                    .subtypes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                return Some(names.join(" or "));
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn trim_cycling_punctuation(word: &str) -> &str {
    word.trim_matches(|ch: char| matches!(ch, ',' | '.' | ';'))
}

pub(super) fn normalize_granted_cycling_surface_text(text: &str) -> String {
    let mut normalized_words = Vec::new();
    let mut expecting_cost = false;

    for word in text.split_whitespace() {
        let trimmed = trim_cycling_punctuation(word);
        if expecting_cost && is_cycling_cost_word(trimmed) {
            let rendered_cost = trimmed.to_ascii_uppercase().replace(['{', '}'], "");
            normalized_words.push(word.replacen(trimmed, &rendered_cost, 1));
            continue;
        }

        normalized_words.push(word.to_string());
        expecting_cost = trimmed.ends_with("cycling");
    }

    normalized_words.join(" ")
}

pub(crate) fn render_cycling_cost_token(word: &str) -> String {
    let upper = word.to_ascii_uppercase();
    if upper.starts_with('{') && upper.ends_with('}') {
        upper
    } else {
        format!("{{{upper}}}")
    }
}

pub(crate) fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x'
                )
        })
}

pub(crate) fn choices_are_simple_targets(choices: &[ChooseSpec]) -> bool {
    fn is_simple_target(choice: &ChooseSpec) -> bool {
        match choice {
            ChooseSpec::SurfaceHinted { spec, .. } => is_simple_target(spec),
            ChooseSpec::Target(_)
            | ChooseSpec::AnyTarget
            | ChooseSpec::AnyOtherTarget
            | ChooseSpec::PlayerOrPlaneswalker(_) => true,
            ChooseSpec::WithCount(inner, _) | ChooseSpec::WithCountValue(inner, _, _) => {
                is_simple_target(inner)
            }
            _ => false,
        }
    }

    choices.iter().all(is_simple_target)
}

pub(crate) fn flatten_condition_and_expr(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_condition_and_expr(left, out);
            flatten_condition_and_expr(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

pub(crate) fn fold_condition_exprs(
    conditions: Vec<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        crate::ConditionExpr::And(Box::new(acc), Box::new(next))
    }))
}

#[derive(Clone, Copy)]
pub(crate) enum TriggerFrequencySurface {
    FirstTimeThisTurn,
    AbilityMaxTimesEachTurn(u32),
    DoThisMaxTimesEachTurn(u32),
}

pub(crate) fn split_trigger_intervening_if(
    condition: &crate::ConditionExpr,
) -> (
    Option<crate::ConditionExpr>,
    Option<TriggerFrequencySurface>,
) {
    let mut flat = Vec::new();
    flatten_condition_and_expr(condition, &mut flat);

    let mut non_limit = Vec::new();
    let mut first_time_this_turn = false;
    let mut do_this_max_times_each_turn: Option<u32> = None;
    let mut max_times_each_turn: Option<u32> = None;
    for item in flat {
        match item {
            crate::ConditionExpr::FirstTimeThisTurn => {
                first_time_this_turn = true;
            }
            crate::ConditionExpr::SourceFirstCrewedThisTurn => {
                first_time_this_turn = true;
            }
            crate::ConditionExpr::DoThisMaxTimesEachTurn(limit) => {
                do_this_max_times_each_turn = Some(match do_this_max_times_each_turn {
                    Some(existing) => existing.min(limit),
                    None => limit,
                });
            }
            crate::ConditionExpr::MaxTimesEachTurn(limit) => {
                max_times_each_turn = Some(match max_times_each_turn {
                    Some(existing) => existing.min(limit),
                    None => limit,
                });
            }
            other => non_limit.push(other),
        }
    }

    let frequency = if first_time_this_turn {
        Some(TriggerFrequencySurface::FirstTimeThisTurn)
    } else if let Some(limit) = do_this_max_times_each_turn {
        Some(TriggerFrequencySurface::DoThisMaxTimesEachTurn(limit))
    } else {
        max_times_each_turn.map(TriggerFrequencySurface::AbilityMaxTimesEachTurn)
    };

    (fold_condition_exprs(non_limit), frequency)
}

pub(super) fn remove_presentation_label_chosen_option(
    condition: &crate::ConditionExpr,
    triggered: &crate::ability::TriggeredAbility,
) -> Option<crate::ConditionExpr> {
    match condition {
        crate::ConditionExpr::SourceChosenOption(option) => {
            if presentation_label_matches_chosen_option(triggered, option) {
                None
            } else {
                Some(condition.clone())
            }
        }
        crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::Speed(crate::target::PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(4),
        } if presentation_label_matches_chosen_option(triggered, "Max speed") => None,
        crate::ConditionExpr::And(left, right) => match (
            remove_presentation_label_chosen_option(left, triggered),
            remove_presentation_label_chosen_option(right, triggered),
        ) {
            (Some(left), Some(right)) => {
                Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
            }
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        },
        other => Some(other.clone()),
    }
}

pub(super) fn presentation_label_matches_chosen_option(
    triggered: &crate::ability::TriggeredAbility,
    option: &str,
) -> bool {
    triggered
        .presentation_label
        .as_ref()
        .and_then(PresentationLabel::display_prefix)
        .is_some_and(|label| {
            let label = label
                .trim()
                .trim_start_matches(|ch: char| !ch.is_alphanumeric())
                .trim();
            let label = label.split(['—', '-']).next().unwrap_or(label).trim();
            label.eq_ignore_ascii_case(option)
        })
}

#[cfg(test)]
mod next_turn_draw_surface_tests {
    use super::*;

    fn next_turn_upkeep(effect: Effect) -> Effect {
        Effect::new(
            crate::effects::ScheduleDelayedTriggerEffect::new(
                crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::Any),
                vec![effect],
                true,
                Vec::new(),
                PlayerFilter::You,
            )
            .starting_next_turn(),
        )
    }

    #[test]
    fn single_draw_instruction_places_next_upkeep_timing_last() {
        assert_eq!(
            describe_effect(&next_turn_upkeep(Effect::draw(1))),
            "Draw a card at the beginning of the next turn's upkeep"
        );
    }

    #[test]
    fn other_delayed_instructions_keep_the_timing_prefix() {
        assert_eq!(
            describe_effect(&next_turn_upkeep(Effect::gain_life(1))),
            "At the beginning of the next turn's upkeep, you gain 1 life"
        );
    }

    #[test]
    fn forecast_presentation_suppresses_its_implied_upkeep_clause() {
        assert!(
            collect_activation_restriction_clauses(
                &ActivationTiming::DuringSourceOwnersUpkeep,
                &["__ironsmith_activation_label:Forecast".to_string()],
            )
            .is_empty()
        );
        assert_eq!(
            collect_activation_restriction_clauses(
                &ActivationTiming::DuringSourceOwnersUpkeep,
                &[],
            ),
            vec!["Activate only during this card's owner's upkeep"]
        );
    }
}
