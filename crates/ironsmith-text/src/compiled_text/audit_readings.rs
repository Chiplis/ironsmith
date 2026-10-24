//! Structural readings of static payload families for the semantic audit.
//!
//! Each reading is built only from a payload's executable fields, never from
//! its `display` or other retained prose, so a mutated model always reads
//! differently and a line can never quote the printed card. Nested values
//! without an English describer use the compact structural reading from
//! `ironsmith_core::semantic_audit`.

use super::*;
use ironsmith_core::StaticAbilityPayload as P;

/// Compact structural reading of a nested value without an English reader.
fn sr(value: &dyn std::fmt::Debug) -> String {
    ironsmith_core::semantic_audit::structural_reading(value, &[])
}

fn player(filter: &PlayerFilter) -> String {
    describe_player_filter(filter)
}

fn objects(filter: &ObjectFilter) -> String {
    filter.description()
}

fn effects(list: &[Effect]) -> String {
    let text = describe_effect_list(list);
    let text = text.trim().trim_end_matches('.');
    if text.is_empty() { "do nothing".to_string() } else { lowercase_first(text) }
}

fn may(optional: bool) -> &'static str {
    if optional { "you may " } else { "" }
}

fn except_first(except: bool) -> &'static str {
    if except { " except the first one in their draw step" } else { "" }
}

fn times(count: u32) -> String {
    match count {
        1 => "an additional time".to_string(),
        n => format!("{n} additional times"),
    }
}

fn damage_scope(
    source: &ObjectFilter,
    player_filter: &Option<PlayerFilter>,
    object_filter: &Option<ObjectFilter>,
) -> String {
    let mut recipients = Vec::new();
    if let Some(p) = player_filter {
        recipients.push(player(p));
    }
    if let Some(o) = object_filter {
        recipients.push(objects(o));
    }
    let to = if recipients.is_empty() {
        String::new()
    } else {
        format!(" to {}", recipients.join(" or "))
    };
    format!("{} would deal damage{to}", objects(source))
}

/// The structural audit reading of a static payload, or `None` when this
/// family has no reader (the engine display then reports it explicitly).
pub(crate) fn audit_payload_reading(
    model: &crate::static_abilities::CompiledStaticAbility,
) -> Option<String> {
    Some(match &model.payload {
        P::Grants(_) => crate::static_abilities::StaticAbility::from_model(model.clone())
            .grant_spec()
            .map(|grant| grant.display())
            .unwrap_or_else(|| ironsmith_core::semantic_audit::unverified_marker(
                "GrantSpec", model, &[])),
        P::None if model.id == Some(crate::static_abilities::StaticAbilityId::StaticEffectsContinueUntilEndOfTurnAfterLeaving) =>
            "If this permanent leaves the battlefield, its static effects continue until end of turn".to_string(),
        P::None if matches!(model.id, Some(
            crate::static_abilities::StaticAbilityId::DraftRuleText
            | crate::static_abilities::StaticAbilityId::KeywordMarker
            | crate::static_abilities::StaticAbilityId::DeckConstructionRuleText
            | crate::static_abilities::StaticAbilityId::MustAttackDraftNotedPlayers
            | crate::static_abilities::StaticAbilityId::Toxic
            | crate::static_abilities::StaticAbilityId::CountAsCardNamedForSpellEffect
            | crate::static_abilities::StaticAbilityId::KeywordText
            | crate::static_abilities::StaticAbilityId::KeywordFallbackText
            | crate::static_abilities::StaticAbilityId::RuleFallbackText
            | crate::static_abilities::StaticAbilityId::UnsupportedParserLine
        )) => ironsmith_core::semantic_audit::unverified_marker(
            "LabelDependentStatic", model, &[]),
        // These families' ordinary displays can return the entire authored
        // sentence. Include every typed scope and cost in the audit reading.
        P::AttackCost { .. } | P::BlockCost { .. }
        | P::CantAttackUnlessCondition { .. }
        | P::ExertAttack { .. } | P::EnlistAttack { .. }
        | P::ModifyDamageAmountReplacement { .. }
        | P::ExileWouldDieInstead { .. }
        | P::EntersWithCountersIfCondition { .. }
        | P::EntersWithCounterChoice { .. }
        | P::ActorCountersAddition { .. }
        | P::EchoCostAlternative { .. }
        | P::ChooseBasicLandTypesAsEnters { .. } => sr(&model.payload),
        P::CountersRemainAcrossZoneChanges { excluded_destinations, .. } => {
            let mut text = "Counters on this permanent remain on it as it changes zones".to_string();
            if !excluded_destinations.is_empty() {
                text.push_str(&format!(", except to {}", sr(excluded_destinations)));
            }
            text
        }
        P::AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn { cost, .. } => format!(
            "Any player may pay {} to ignore this permanent's effect until end of turn",
            cost.to_oracle()
        ),
        P::Companion(condition) => format!("Companion — deck condition: {}", sr(condition)),
        P::CopyActivatedAbilities(spec) => format!("This has the activated abilities of {}", sr(spec)),
        P::CopyStaticAbilityVariants(spec) => format!("This has the static abilities of {}", sr(spec)),
        P::CopyTriggeredAbilities(spec) => format!("This has the triggered abilities of {}", sr(spec)),
        P::CostIncreaseLife { filter, amount, .. } => format!(
            "{} cost an additional {amount} life to cast",
            capitalize_first(&objects(filter))
        ),
        P::ActivateAbilitiesAsThoughHaste { filter, .. } => format!(
            "Abilities of {} can be activated as though they had haste",
            objects(filter)
        ),
        P::LoyaltyAbilitiesAnyTime { filter } => format!(
            "Loyalty abilities of {} can be activated any time you could cast an instant",
            objects(filter)
        ),
        P::ThisSpellXMaximum { maximum, .. } => format!("X is at most {}", describe_value(maximum)),
        P::ThisSpellXMinimum { minimum, .. } => format!("X is at least {}", describe_value(minimum)),
        P::DieRollResultAdjustment(adjustment) => format!("Die roll results are adjusted: {}", sr(adjustment)),
        P::BandsWithOther(filter) => format!("Bands with other {}", objects(filter)),
        P::CounterLimit { counter_type, maximum, .. } => format!(
            "This can't have more than {maximum} {} counters on it",
            describe_counter_type(*counter_type)
        ),
        P::UntapDuringEachOtherPlayersUntapStep { filter, opponents_only, .. } => format!(
            "Untap {} during each {} untap step",
            objects(filter),
            if *opponents_only { "opponent's" } else { "other player's" }
        ),
        P::UntapStepLimit { player: who, filter, max, .. } => format!(
            "{} can't untap more than {max} {} during their untap step",
            capitalize_first(&player(who)),
            objects(filter)
        ),
        P::UntapStepChosenCardTypeOnly { player: who, card_types, .. } => format!(
            "During {}'s untap step, only permanents of the chosen card type among {} untap",
            player(who),
            sr(card_types)
        ),
        P::SearchLimitedToTopCards { searcher, count, .. } => format!(
            "If {} would search a library, they search only its top {count} cards",
            player(searcher)
        ),
        P::PlayerProtectionFrom { player: who, source_filter, .. } => format!(
            "{} has protection from {}",
            capitalize_first(&player(who)),
            objects(source_filter)
        ),
        P::ControlAttachedPermanent(_) => "You control the permanent this is attached to".into(),
        P::ChooseBasicLandTypeAsEnters(_) => "As this permanent enters, choose a basic land type".into(),
        P::ChooseLandTypeAsEnters(_) => "As this permanent enters, choose a land type".into(),
        P::ChooseCreatureTypeAsEnters(_) => "As this permanent enters, choose a creature type".into(),
        P::EnchantedLandIsChosenType(_) => "The land this is attached to is the chosen land type".into(),
        P::SourceLandIsChosenType(_) => "This land is the chosen land type".into(),
        P::AttachedIsCopyOfChosenObject(_) => {
            "The permanent this is attached to is a copy of the chosen object".into()
        }
        P::NoteLifeTotalAsEnters(_) => "As this permanent enters, note your life total".into(),
        P::DiscardHandAsEnters(_) => "As this permanent enters, discard your hand".into(),
        P::Enchant(filter) => format!("Enchant {}", sr(filter)),
        P::FlashIfTargetsMatching(filter) => format!(
            "You may cast this spell as though it had flash if it targets {}",
            objects(filter)
        ),
        P::FlashIfCastChoice(choice) => format!(
            "You may cast this spell as though it had flash if {}",
            sr(choice)
        ),
        P::AddChosenCreatureType { filter, replace_existing, .. } => format!(
            "{} is the chosen creature type{}",
            capitalize_first(&objects(filter)),
            if *replace_existing { "" } else { " in addition to its other types" }
        ),
        P::AddChosenBasicLandType { filter, .. } => format!(
            "{} is the chosen basic land type in addition to its other types",
            capitalize_first(&objects(filter))
        ),
        P::SourceHasAbilityStickerAbilitiesOf { filter, .. } => format!(
            "This has the abilities of ability stickers on {}",
            objects(filter)
        ),
        P::AddChosenColor { filter, .. } => format!(
            "{} is the chosen color in addition to its other colors",
            capitalize_first(&objects(filter))
        ),
        P::SetChosenColor { filter, .. } => {
            format!("{} is the chosen color", capitalize_first(&objects(filter)))
        }
        P::DuplicateMatchingTriggeredAbilities { source_filter, event_matcher, count, .. } => format!(
            "Triggered abilities of {} triggered by {} trigger {}",
            source_filter.as_ref().map_or("any source".to_string(), objects),
            event_matcher.as_ref().map_or("any event".to_string(), |m| sr(m)),
            times(*count)
        ),
        P::SuppressMatchingTriggeredAbilities { source_filter, event_matcher, .. } => format!(
            "Triggered abilities of {} triggered by {} don't trigger",
            source_filter.as_ref().map_or("any source".to_string(), objects),
            event_matcher.as_ref().map_or("any event".to_string(), |m| sr(m))
        ),
        P::AdditionalLifeCostPerTarget(life) => {
            format!("This spell costs an additional {life} life to cast for each target")
        }
        P::CommanderTaxLifeSubstitution { life_per_previous_cast } => format!(
            "You may pay {life_per_previous_cast} life for each previous cast instead of commander tax"
        ),
        P::ChooseColorAsEnters { excluded, count, .. } => format!(
            "As this permanent enters, choose {} color{}{}",
            count,
            if *count == 1 { "" } else { "s" },
            excluded.map_or(String::new(), |color| format!(" other than {}", color.name()))
        ),
        P::ChoosePlayerAsEnters { filter, count, .. } => format!(
            "As this permanent enters, choose {count} {}",
            player(filter)
        ),
        P::RevealFromHandAsEnters { filter, count, optional, .. } => format!(
            "As this permanent enters, {}reveal {} {} from your hand",
            may(*optional),
            sr(count),
            objects(filter)
        ),
        P::ChooseCardNameAsEnters {
            reveal_opponents_hands,
            require_nonland_from_revealed_opponents,
            opponent_also_chooses,
            exclude_basic_land_names,
            ..
        } => {
            let mut text = "As this permanent enters, ".to_string();
            if *reveal_opponents_hands {
                text.push_str("your opponents reveal their hands and ");
            }
            text.push_str("you choose a card name");
            if *require_nonland_from_revealed_opponents {
                text.push_str(" of a nonland card revealed this way");
            }
            if *exclude_basic_land_names {
                text.push_str(" other than a basic land card name");
            }
            if *opponent_also_chooses {
                text.push_str("; an opponent also chooses a card name");
            }
            text
        }
        P::ChooseNamedOptionAsEnters { options, at_random, each_player, .. } => format!(
            "As this permanent enters, {}{} one of: {}",
            if *each_player { "each player chooses" } else { "choose" },
            if *at_random { " at random" } else { "" },
            options.join(", ")
        ),
        P::ChoosePowerToughnessAsEntersOrTurnsFaceUp { options, at_random, .. } => format!(
            "As this permanent enters or is turned face up, choose{} its base power and toughness among {}",
            if *at_random { " at random" } else { "" },
            sr(options)
        ),
        P::EnterAsCopyAsEnters { spec, .. } => {
            format!("You may have this permanent enter as a copy of {}", sr(spec))
        }
        P::AsEntersEffectProgram { program, also_turns_face_up, turns_face_up_only, transforms_into, .. } => {
            let event = if *turns_face_up_only {
                "is turned face up"
            } else if *also_turns_face_up {
                "enters or is turned face up"
            } else {
                "enters"
            };
            let mut text = format!(
                "As this permanent {event}, {}",
                effects(program.flattened_default_effects())
            );
            if let Some(face) = transforms_into {
                text.push_str(&format!("; it transforms into {}", sr(face)));
            }
            text
        }
        P::RedirectDamageToSourceController { source_filter, target_player_filter, .. } => format!(
            "If {} would deal damage to {}, it deals that damage to its controller instead",
            objects(source_filter),
            player(target_player_filter)
        ),
        P::MinimumDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, floor, noncombat_only, ..
        } => format!(
            "If {}{}, it deals at least {} instead",
            damage_scope(source_filter, target_player_filter, target_object_filter),
            if *noncombat_only { " as noncombat damage" } else { "" },
            describe_value(floor)
        ),
        P::DoubleDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, factor, combat_only, noncombat_only, ..
        } => format!(
            "If {}{}, it deals {factor} times that much damage instead",
            damage_scope(source_filter, target_player_filter, target_object_filter),
            if *combat_only { " as combat damage" } else if *noncombat_only { " as noncombat damage" } else { "" }
        ),
        P::PreventHalfDamageReplacement {
            source_filter, target_player_filter, target_object_filter, round_up, ..
        } => format!(
            "If {}, prevent half that damage, rounded {}",
            damage_scope(source_filter, target_player_filter, target_object_filter),
            if *round_up { "up" } else { "down" }
        ),
        P::DoubleCountersReplacement {
            filter, player_filter, counter_type, actor, includes_permanents, halve, effect_only, ..
        } => {
            let kind = counter_type.map_or("one or more".to_string(), |c| describe_counter_type(c));
            let mut recipients = Vec::new();
            if *includes_permanents {
                recipients.push(objects(filter));
            }
            if let Some(p) = player_filter {
                recipients.push(player(p));
            }
            format!(
                "If {}{} {kind} counters would be put on {}, {} that many instead",
                actor.as_ref().map_or(String::new(), |a| format!("{} would put ", player(a))),
                if *effect_only { " by an effect" } else { "" },
                recipients.join(" or "),
                if *halve { "half" } else { "twice" }
            )
        }
        P::AddCountersPlacementReplacement { filter, player_filter, counter_type, additional, cause, .. } => format!(
            "If {} counters would be put on {}{}{}, that many plus {additional} are put instead",
            counter_type.map_or("one or more".to_string(), describe_counter_type),
            objects(filter),
            player_filter.as_ref().map_or(String::new(), |p| format!(" or {}", player(p))),
            cause.as_ref().map_or(String::new(), |c| format!(" by {}", sr(c)))
        ),
        P::PlayerCounterPerTurnLimitReplacement { player_filter, counter_type, maximum, .. } => format!(
            "{} can't get more than {maximum} {} counters each turn",
            capitalize_first(&player(player_filter)),
            describe_counter_type(*counter_type)
        ),
        P::DoubleTokenCreationReplacement { controller, .. } => format!(
            "If one or more tokens would be created under {}'s control, twice that many are created instead",
            player(controller)
        ),
        P::MultiplyTokenCreationReplacement { controller, token_filter, factor, .. } => format!(
            "If one or more {} would be created under {}'s control, {factor} times that many are created instead",
            token_filter.as_ref().map_or("tokens".to_string(), objects),
            player(controller)
        ),
        P::AddTokenCreationReplacement { controller, token_filter, additional_token, additional, per_created, .. } => format!(
            "If one or more {} would be created under {}'s control, also create {additional} {}{}",
            objects(token_filter),
            player(controller),
            sr(additional_token),
            if *per_created { " for each one created" } else { "" }
        ),
        P::KeywordActionReplacement { action, source_filter, performer_filter, replacement_effects, optional, .. } => format!(
            "If {} would {} from {}, {}{} instead",
            performer_filter.as_ref().map_or("a player".to_string(), player),
            sr(action),
            objects(source_filter),
            may(*optional),
            effects(replacement_effects)
        ),
        P::ConditionalDrawReplacement { condition, replacement_effects, optional, except_first_of_draw_step, .. } => format!(
            "If you would draw a card{} while {}, {}{} instead",
            except_first(*except_first_of_draw_step),
            describe_condition(condition),
            may(*optional),
            effects(replacement_effects)
        ),
        P::CreateOneOfEachTokenReplacement { kinds, .. } => format!(
            "If one or more tokens would be created, create one of each of {} instead",
            sr(kinds)
        ),
        P::RedirectDrawReplacement { drawer, except_first_of_draw_step, .. } => format!(
            "If {} would draw a card{}, you draw it instead",
            player(drawer),
            except_first(*except_first_of_draw_step)
        ),
        P::DrawReplacementWithEffects { drawer, except_first_of_draw_step, replacement_effects, cycling_source, .. } => format!(
            "If {} would draw a card{}{}, {} instead",
            player(drawer),
            except_first(*except_first_of_draw_step),
            cycling_source.as_ref().map_or(String::new(), |s| format!(" by cycling {}", objects(s))),
            effects(replacement_effects)
        ),
        P::DrawExtraCardsReplacement { extra, except_first_of_draw_step, per_instruction, .. } => format!(
            "If you would draw a card{}, draw {extra} additional card{}{} instead",
            except_first(*except_first_of_draw_step),
            if *extra == 1 { "" } else { "s" },
            if *per_instruction { " per instruction" } else { "" }
        ),
        P::LoseGameReplacement { replacement_effects, optional, .. } => format!(
            "If you would lose the game, {}{} instead",
            may(*optional),
            effects(replacement_effects)
        ),
        P::DrawReplacementRevealTopMatchingToHandRestBottom { count, filter, order, .. } => format!(
            "If you would draw a card, reveal the top {count} cards of your library, put {} among them into your hand, and the rest on the bottom {}",
            objects(filter),
            sr(order)
        ),
        P::RedirectWouldEnter { filter, not_cast, destination, or_no_mana_spent, .. } => format!(
            "If {}{}{} would enter, put it into {} instead",
            objects(filter),
            if *not_cast { " that wasn't cast" } else { "" },
            if *or_no_mana_spent { " or had no mana spent to cast it" } else { "" },
            sr(destination)
        ),
        P::ManaProductionReplacement { source_filter, minimum_amount, replacement_mana, .. } => format!(
            "If {} would produce {minimum_amount} or more mana, it produces {} instead",
            objects(source_filter),
            replacement_mana.iter().map(|symbol| describe_mana_symbol(*symbol)).collect::<Vec<_>>().join(" ")
        ),
        P::ManaProductionMultiplierReplacement { source_filter, factor, .. } => format!(
            "If {} would produce mana, it produces {factor} times that much instead",
            objects(source_filter)
        ),
        P::DoubleLifeChangeReplacement { player: who, loss, .. } => format!(
            "If {} would {} life, they {} twice that much instead",
            player(who),
            if *loss { "lose" } else { "gain" },
            if *loss { "lose" } else { "gain" }
        ),
        P::ManaSpendPermission { permission, .. } => format!("Mana may be spent as follows: {}", sr(permission)),
        P::PreventDamageToSelfPutCountersInstead { counter_type, .. } => format!(
            "If damage would be dealt to this permanent, prevent it and put that many {} counters on it",
            describe_counter_type(*counter_type)
        ),
        P::HealDamageBeforeDamageToSelf { .. } => {
            "Before damage is dealt to this permanent, remove the damage already marked on it".into()
        }
        P::PreventConstrainedDamageToSelfPutCountersInstead { counter_type, source_filter, combat_only, .. } => format!(
            "If {}damage{} would be dealt to this permanent, prevent it and put that many {} counters on it",
            match combat_only { Some(true) => "combat ", Some(false) => "noncombat ", None => "" },
            source_filter.as_ref().map_or(String::new(), |s| format!(" from {}", objects(s))),
            describe_counter_type(*counter_type)
        ),
        P::PreventDamageToYouFromSourceFilter { amount, source_filter, .. } => format!(
            "Prevent {amount} of the damage that would be dealt to you by {}",
            objects(source_filter)
        ),
        P::ReplaceDamageWithCountersInstead { counter_type, source_filter, target_filter, combat_only, .. } => format!(
            "If {} would deal {}damage to {}, put that many {} counters on it instead",
            objects(source_filter),
            match combat_only { Some(true) => "combat ", Some(false) => "noncombat ", None => "" },
            objects(target_filter),
            describe_counter_type(*counter_type)
        ),
        P::NativeAlternativeCastFromZone { zone, method } => {
            format!("You may cast this card from {} using {}", sr(zone), sr(method))
        }
        P::AlternativeCastFromZoneForFilter { filter, zone, method } => format!(
            "You may cast {} from {} using {}",
            objects(filter),
            sr(zone),
            sr(method)
        ),
        P::AddLifeGainReplacement { player: who, additional, .. } => format!(
            "If {} would gain life, they gain that much plus {additional} instead",
            player(who)
        ),
        P::TokenCreationTemplates { controller, token_filter, templates, mode, choose_one, optional, .. } => format!(
            "If {} would be created under {}'s control, {}{}{} using {}",
            objects(token_filter),
            player(controller),
            may(*optional),
            if *choose_one { "choose one and " } else { "" },
            sr(mode),
            effects(templates)
        ),
        P::SpellManaSpendingRestriction(restriction) => {
            format!("Mana spent on this spell is restricted: {}", sr(restriction))
        }
        P::ManaProductionRewrite { rule, .. } => format!("Mana production is rewritten: {}", sr(rule)),
        P::ExtraDieIgnoreLowest { player: who, additional } => format!(
            "If {} would roll a die, they roll {additional} more and ignore the lowest",
            player(who)
        ),
        P::ExtraCoinIgnoreOne { player: who } => format!(
            "If {} would flip a coin, they flip an additional coin and ignore one",
            player(who)
        ),
        P::FirstCoinBatchHeadsWin { player: who } => format!(
            "{} wins the first coin flip each batch",
            capitalize_first(&player(who))
        ),
        P::ExtraPlanarDieIgnoreOne { player: who } => format!(
            "If {} would roll the planar die, they roll it twice and ignore one",
            player(who)
        ),
        P::LookAtSourceExiledCards { pair, .. } => format!(
            "You may look at cards exiled with this{}",
            pair.as_ref().map_or(String::new(), |p| format!(", linked as {}", sr(p)))
        ),
        P::EventReplacementWithEffects { event, replacement_effects, optional, .. } => format!(
            "If {} would happen, {}{} instead",
            sr(event),
            may(*optional),
            effects(replacement_effects)
        ),
        P::EventAmountReplacement { event, modifier, optional, .. } => format!(
            "If {} would happen, {}{} its amount instead",
            sr(event),
            may(*optional),
            sr(modifier)
        ),
        P::DevotionIncrease { amount } => format!("Your devotion is increased by {amount}"),
        P::MorphCostIncrease { generic_increase } => {
            format!("Morph costs cost {{{generic_increase}}} more")
        }
        P::UnlockCostReduction { generic_reduction } => {
            format!("Unlock costs cost {{{generic_reduction}}} less")
        }
        P::SourceHasFullTextOfTopGraveyardCard { filter, .. } => format!(
            "This has the full text of the top {} of your graveyard",
            objects(filter)
        ),
        P::MaySkipDrawStepWithFollowup { if_you_do, .. } => format!(
            "You may skip your draw step; if you do, {}",
            effects(if_you_do)
        ),
        P::PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn { player: who, .. } => format!(
            "{} may sacrifice a permanent to ignore this permanent's effect until end of turn",
            capitalize_first(&player(who))
        ),
        P::CastTargetRestriction { forbidden, .. } => {
            format!("Spells and abilities can't target {}", objects(forbidden))
        }
        P::EntersAttachedToChosenObject { filter, .. } => format!(
            "This enters attached to a chosen {}",
            objects(filter)
        ),
        P::AdditionalSacrificePerManaSymbol { activated_abilities, symbol, sacrifice, .. } => format!(
            "For each {} in the cost of {}, sacrifice {} as an additional cost",
            describe_mana_symbol(*symbol),
            if *activated_abilities { "activated abilities" } else { "spells" },
            objects(sacrifice)
        ),
        P::ControllerCounterExilePermission { permission, .. } => {
            format!("Its controller may exile countered cards: {}", sr(permission))
        }
        P::AttackingPlayerChoosesBlocks { .. } => {
            "The attacking player chooses how creatures block".into()
        }
        P::AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn { cost, .. } => format!(
            "The controller of the permanent this is attached to may pay {} to ignore this effect until end of turn",
            cost.display()
        ),
        P::FirstChosenLandTypeBecomesSecond { filter, .. } => format!(
            "{} of the first chosen land type are the second chosen land type",
            capitalize_first(&objects(filter))
        ),
        P::RevealCardsDrawn { player: who } => {
            format!("{} reveals each card they draw", capitalize_first(&player(who)))
        }
        P::DamageBetweenChosenPlayers { factor, .. } => {
            format!("Damage dealt between the chosen players is multiplied by {factor}")
        }
        P::AddSubtypesAndGrantAbilities { filter, subtypes, abilities, .. } => format!(
            "{} is {} in addition to its other types and has {}",
            capitalize_first(&objects(filter)),
            sr(subtypes),
            abilities
                .iter()
                .map(|ability| sr(ability))
                .collect::<Vec<_>>()
                .join(" and ")
        ),
        P::Conditional { ability, condition, .. } => format!(
            "{} as long as {}",
            audit_payload_reading(ability).unwrap_or_else(|| {
                crate::static_abilities::StaticAbility::from_model(ability.as_ref().clone()).display()
            }),
            describe_condition(condition)
        ),
        P::ForetellSpecialActionModifier { generic_reduction, any_players_turn, .. } => format!(
            "Foretelling costs {{{generic_reduction}}} less{}",
            if *any_players_turn { " and is possible during any player's turn" } else { "" }
        ),
        P::CommanderPairing { partner_with, group, choose_background } => {
            if let Some(name) = partner_with {
                format!("Partner with {name}")
            } else if *choose_background {
                "Choose a Background".to_string()
            } else if let Some(group) = group {
                format!("Partner within the group {group}")
            } else {
                "Partner".to_string()
            }
        }
        P::DeckConstructionCopyLimit { maximum } => match maximum {
            None => "A deck can have any number of copies of this card".to_string(),
            Some(limit) => format!("A deck can have up to {limit} copies of this card"),
        },
        _ => return None,
    })
}
