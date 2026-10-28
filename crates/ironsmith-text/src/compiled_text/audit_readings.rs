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
    if except { " except the first one you draw in each of your draw steps" } else { "" }
}

fn times(count: u32) -> String {
    match count {
        1 => "an additional time".to_string(),
        n => format!("{n} additional times"),
    }
}

/// A damage source filter: with no zone it matches spells as well as
/// permanents, so its noun is "source" ("a red source you control").
fn damage_source_objects(source: &ObjectFilter) -> String {
    let description = objects(source);
    let description = if source.zone.is_none() {
        description.replacen("permanent", "source", 1)
    } else {
        description
    };
    article(&description)
}

/// "a source you control would deal noncombat damage to an opponent or a
/// permanent an opponent controls"; `kind` is "", "combat " or "noncombat ".
fn damage_scope(
    source: &ObjectFilter,
    player_filter: &Option<PlayerFilter>,
    object_filter: &Option<ObjectFilter>,
    kind: &str,
) -> String {
    let mut recipients = Vec::new();
    if let Some(p) = player_filter {
        recipients.push(player(p));
    }
    if let Some(o) = object_filter {
        recipients.push(article(&objects(o)));
    }
    let to = if recipients.is_empty() {
        String::new()
    } else {
        format!(" to {}", recipients.join(" or "))
    };
    format!("{} would deal {kind}damage{to}", damage_source_objects(source))
}

fn count_words(count: u32) -> String {
    i32::try_from(count)
        .ok()
        .and_then(number_word)
        .unwrap_or_else(|| count.to_string())
}

fn value_words(value: &crate::effect::Value) -> String {
    match value {
        crate::effect::Value::Fixed(n) => number_word(*n).unwrap_or_else(|| n.to_string()),
        other => describe_value(other),
    }
}

fn damaged_by_subject(source: &ironsmith_core::DamagedBySource) -> &'static str {
    match source {
        ironsmith_core::DamagedBySource::ThisCreature => "this source",
        ironsmith_core::DamagedBySource::EquippedCreature => "equipped creature",
        ironsmith_core::DamagedBySource::EnchantedCreature => "enchanted creature",
    }
}

/// An ability a static adds to its object, read from its own model.
type AbilityModel = ironsmith_core::Ability<
    crate::static_abilities::CompiledStaticAbility,
    crate::triggers::Trigger,
    Effect,
    crate::costs::Cost,
>;

fn ability_model(ability: &AbilityModel) -> String {
    match &ability.kind {
        ironsmith_core::AbilityKind::Static(model) => {
            let text = audit_payload_reading(model).unwrap_or_else(|| {
                crate::static_abilities::StaticAbility::from_model(model.clone()).display()
            });
            let text = text.trim().trim_end_matches('.');
            if merge_passes::is_keyword_phrase(text) {
                text.to_ascii_lowercase()
            } else {
                lowercase_first(text)
            }
        }
        _ => {
            let ability = crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(ability);
            let text = ability_surface_text(&ability);
            let text = text.trim().trim_end_matches('.');
            if merge_passes::is_keyword_phrase(text) {
                text.to_ascii_lowercase()
            } else {
                format!("\"{text}.\"")
            }
        }
    }
}

/// "You may have this creature enter as a copy of a creature you control,
/// except it's a 1/1": the copy choice and every exception it applies.
fn enter_as_copy(spec: &ironsmith_core::EnterAsCopyAsEntersSpec<
    crate::triggers::Trigger,
    Effect,
    crate::costs::Cost,
    ironsmith_core::ThisSpellCostCondition,
>) -> String {
    let source = if spec.copy_source_self {
        "itself".to_string()
    } else if spec.copy_source_enchanted {
        "enchanted permanent".to_string()
    } else if spec.filter.zone == Some(Zone::Battlefield) && spec.filter.controller.is_none() {
        // "any creature on the battlefield": no controller restriction.
        format!("any {} on the battlefield", strip_indefinite_article(&objects(&spec.filter)))
    } else if spec.filter.controller.is_none() && spec.filter.zone.is_some() {
        format!("any {}", strip_indefinite_article(&objects(&spec.filter)))
    } else {
        with_indefinite_article(&objects(&spec.filter))
    };
    let tapped = if spec.enters_tapped_if_chosen { " tapped" } else { "" };
    let mut text = if spec.may {
        format!("You may have this source enter{tapped} as a copy of {source}")
    } else {
        format!("This source enters{tapped} as a copy of {source}")
    };
    let mut exceptions = Vec::new();
    if let Some(name) = &spec.name_override {
        exceptions.push(format!("its name is {name}"));
    }
    if let Some((power, toughness)) = spec.set_base_power_toughness {
        exceptions.push(format!("it's {power}/{toughness}"));
    }
    if spec.set_base_power_toughness_from_self {
        exceptions.push("it has this object's base power and toughness".to_string());
    }
    if !spec.added_colors.is_empty() {
        exceptions.push(format!("it's {} in addition to its other colors", sr(&spec.added_colors)));
    }
    let mut types: Vec<String> = spec.added_supertypes.iter().map(|t| format!("{t:?}").to_ascii_lowercase()).collect();
    types.extend(spec.added_subtypes.iter().map(|t| format!("{t:?}")));
    types.extend(spec.added_card_types.iter().map(|t| format!("{t:?}").to_ascii_lowercase()));
    if !types.is_empty() {
        exceptions.push(format!(
            "it's {} {}",
            with_indefinite_article(&types.join(" ")),
            if spec.removes_other_card_types { "and loses all other card types" } else { "in addition to its other types" }
        ));
    }
    for supertype in &spec.removed_supertypes {
        exceptions.push(format!("it isn't {}", format!("{supertype:?}").to_ascii_lowercase()));
    }
    if !spec.added_abilities.is_empty() {
        let abilities = spec.added_abilities.iter().map(ability_model).collect::<Vec<_>>();
        exceptions.push(format!("it has {}", abilities.join(" and ")));
    }
    for (counter, amount) in &spec.additional_counters {
        exceptions.push(format!(
            "it enters with {} additional {} counter{} on it",
            count_words(*amount),
            describe_counter_type(*counter),
            if *amount == 1 { "" } else { "s" }
        ));
    }
    if !exceptions.is_empty() {
        text.push_str(&format!(", except {}", exceptions.join(" and ")));
    }
    // Rarer fields keep a structural reading so none is silently dropped.
    let mut rest = Vec::new();
    if let Some(filter) = &spec.affected_filter { rest.push(format!("affected {}", sr(filter))); }
    if let Some(until) = &spec.copy_duration { rest.push(format!("until {}", sr(until))); }
    if let Some(pair) = &spec.linked_exile_pair { rest.push(sr(pair)); }
    if !spec.additional_x_counters.is_empty() { rest.push(sr(&spec.additional_x_counters)); }
    if spec.keep_other_source_abilities { rest.push("it keeps this object's other abilities".to_string()); }
    if let Some(filter) = &spec.additional_counters_source_filter { rest.push(format!("counters only if it copies {}", objects(filter))); }
    if let Some(filter) = &spec.added_abilities_source_filter { rest.push(format!("abilities only if it copies {}", objects(filter))); }
    if !spec.conditional_additional_counters.is_empty() { rest.push(sr(&spec.conditional_additional_counters)); }
    for followup in &spec.copy_followups {
        rest.push(ironsmith_core::semantic_audit::humanize_variant(followup));
    }
    if !rest.is_empty() {
        text.push_str(&format!("; {}", rest.join("; ")));
    }
    text
}

fn cant_attack_unless(
    condition: &ironsmith_core::CantAttackUnlessConditionSpec,
) -> String {
    use ironsmith_core::{
        AttackCostCondition as Cost, AttackingGroupAttackCondition as Group,
        CantAttackUnlessConditionSpec as Spec, DefendingPlayerAttackCondition as Defender,
    };
    let (verb, clause) = match condition {
        Spec::SourceCondition(source) => ("attack", describe_condition(source)),
        Spec::ControllerControlsMoreThanDefendingPlayer(filter) => (
            "attack",
            format!("you control more {} than defending player", objects(filter)),
        ),
        Spec::BattlefieldCountAtLeast { filter, count } => (
            "attack",
            format!("there are {count} or more {} on the battlefield", objects(filter)),
        ),
        Spec::ControllerGraveyardHasCardsAtLeast(count) => (
            "attack",
            format!("there are {count} or more cards in your graveyard"),
        ),
        Spec::DefendingPlayerCondition(condition) => ("attack", match condition {
            Defender::Controls(filter) => format!("defending player controls {}", objects(filter)),
            Defender::ControlsEnchantmentOrEnchantedPermanent => {
                "defending player controls an enchantment or an enchanted permanent".to_string()
            }
            Defender::HasCardsInGraveyardOrMore(count) => {
                format!("defending player has {count} or more cards in their graveyard")
            }
            Defender::IsMonarch => "defending player is the monarch".to_string(),
            Defender::IsPoisoned => "defending player is poisoned".to_string(),
        }),
        Spec::OpponentWasDealtDamageThisTurn => {
            ("attack", "an opponent has been dealt damage this turn".to_string())
        }
        Spec::AttackingGroupCondition(group) => match group {
            Group::AtLeastNOtherCreaturesAttack(count) => {
                ("attack", format!("at least {count} other creatures attack"))
            }
            Group::BlackOrGreenCreatureAlsoAttacks => {
                ("attack", "a black or green creature also attacks".to_string())
            }
            Group::CreatureWithGreaterPowerAlsoAttacks => {
                ("attack", "a creature with greater power also attacks".to_string())
            }
            Group::AtLeastNOtherCreaturesBlock(count) => {
                ("block", format!("at least {count} other creatures block"))
            }
            Group::CreatureWithGreaterPowerAlsoBlocks => {
                ("block", "a creature with greater power also blocks".to_string())
            }
        },
        Spec::AttackCost(cost) => ("attack", match cost {
            Cost::PayGenericPerSourceCounter { counter_type, amount_per_counter } => format!(
                "you pay {{{amount_per_counter}}} for each {} counter on it",
                describe_counter_type(*counter_type)
            ),
            Cost::ReturnPermanentsToOwnersHand { filter, count } => {
                format!("you return {count} {} to their owner's hand", objects(filter))
            }
            Cost::SacrificePermanents { filter, count } => {
                format!("you sacrifice {count} {}", objects(filter))
            }
        }),
    };
    format!("This source can't {verb} unless {clause}")
}

/// "When you do, ..." for a reflexive trigger linked to an optional action.
fn when_you_do<T>(trigger: &ironsmith_core::TriggeredAbility<T, Effect>) -> String {
    format!(
        "When you do, {}{}",
        trigger.intervening_if.as_ref().map_or(String::new(), |condition| format!("if {}, ", describe_condition(condition))),
        effects(trigger.effects.flattened_default_effects())
    )
}

fn possessive(who: &PlayerFilter) -> String {
    match player(who).as_str() {
        "you" => "your".to_string(),
        other => format!("{other}'s"),
    }
}

fn multiple(factor: u32) -> String {
    match factor {
        2 => "twice".to_string(),
        3 => "three times".to_string(),
        n => format!("{n} times"),
    }
}

/// "an artifact or creature entering": a trigger event as the subject of
/// "causes ... to trigger".
fn trigger_gerund(trigger: &crate::triggers::Trigger) -> String {
    let text = trigger.display();
    let event = ["Whenever ", "When ", "At "]
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix))
        .unwrap_or(&text)
        .trim_end_matches('.')
        .to_string();
    let mut words: Vec<String> = event.split(' ').map(str::to_string).collect();
    if let Some(verb) = words.iter_mut().find(|word| {
        matches!(word.as_str(), "enters" | "attacks" | "dies" | "blocks" | "becomes" | "leaves" | "is" | "deals")
    }) {
        *verb = match verb.as_str() {
            "dies" => "dying".to_string(),
            "is" => "being".to_string(),
            other => format!("{}ing", other.trim_end_matches('s')),
        };
    }
    words.join(" ").replacen("entering the battlefield", "entering", 1)
}

fn mana_symbol_name(symbol: &ironsmith_core::ManaSymbol) -> String {
    format!("{symbol:?}").to_ascii_lowercase()
}

/// "You may spend mana as though it were mana of any color to cast
/// planeswalker spells."
fn mana_spend_permission(permission: &ironsmith_core::ManaSpendPermission) -> String {
    use ironsmith_core::{value_model::ManaSpendMode as Mode, ManaSpendScope as Scope};
    let subject = match &permission.player {
        PlayerFilter::You => "You may spend".to_string(),
        PlayerFilter::Any => "Players may spend".to_string(),
        other => format!("{} may spend", capitalize_first(&player(other))),
    };
    let mana = if let Some((from, to)) = &permission.mana_symbol_as {
        format!("{} mana as though it were {} mana", mana_symbol_name(from), mana_symbol_name(to))
    } else {
        let spent = match (&permission.any_color_mana_symbol, permission.mode) {
            (Some(symbol), _) => format!("{} mana", mana_symbol_name(symbol)),
            (None, Mode::ColorlessAsAnyColor) => "colorless mana".to_string(),
            (None, _) => "mana".to_string(),
        };
        let as_though = match permission.mode {
            Mode::AnyType => " as though it were mana of any type",
            Mode::Normal if permission.any_color_mana_symbol.is_none() => "",
            Mode::Normal => " as though it were mana of any color",
            Mode::AnyColor | Mode::ColorlessAsAnyColor => " as though it were mana of any color",
        };
        format!("{spent}{as_though}")
    };
    let mana = match &permission.mana_source_filter {
        Some(filter) => format!("{mana} produced by {}", objects(filter)),
        None => mana,
    };
    let scope = match &permission.scope {
        Scope::AllCosts => String::new(),
        Scope::ActivationCostsOf(filter) => format!(" to activate abilities of {}", objects(filter)),
        Scope::CastingSpellsMatching(filter) => format!(
            " to cast {} spells",
            strip_indefinite_article(&objects(filter))
        ),
        Scope::CastingSpellsWithStableIds(ids) => format!(" to cast {}", sr(ids)),
        Scope::OneSpellCast => " to pay the mana cost of one spell this turn".to_string(),
    };
    let mut text = format!("{subject} {mana}{scope}");
    if permission.other_mana_only_as_colorless {
        text.push_str(". You may spend other mana only as though it were colorless mana");
    }
    text
}

fn zone_phrase(zone: &Zone) -> &'static str {
    match zone {
        Zone::Battlefield => "the battlefield",
        Zone::Graveyard => "a graveyard",
        Zone::Hand => "a hand",
        Zone::Library => "a library",
        Zone::Exile => "exile",
        Zone::Stack => "the stack",
        other => other.name(),
    }
}

/// An object phrase with an article unless it already has a determiner.
fn article(phrase: &str) -> String {
    let determined = ["this ", "that ", "the ", "a ", "an ", "another ", "each ", "enchanted ", "equipped ", "target "]
        .iter()
        .any(|determiner| phrase.starts_with(determiner));
    if determined { phrase.to_string() } else { with_indefinite_article(phrase) }
}

fn damage_source(source: &Option<ObjectFilter>) -> String {
    source
        .as_ref()
        .map_or("a source".to_string(), damage_source_objects)
}

/// "a source would deal damage to you": the event a replacement watches.
fn replaced_event(event: &ironsmith_core::ReplacedEventSpec) -> String {
    use ironsmith_core::ReplacedEventSpec as E;
    let combat = |combat_only: bool| if combat_only { "combat " } else { "" };
    match event {
        E::DamageToPlayer { player: who, source_filter, combat_only } => format!(
            "{} would deal {}damage to {}",
            damage_source(source_filter),
            combat(*combat_only),
            player(who)
        ),
        E::DamageToObject { target, source_filter, combat_only } => format!(
            "{} would deal {}damage to {}",
            damage_source(source_filter),
            combat(*combat_only),
            article(&objects(target))
        ),
        E::LifeGain { player: who, from_own_spell_or_ability } => format!(
            "{} would gain life{}",
            player(who),
            if *from_own_spell_or_ability { " from a spell or ability they control" } else { "" }
        ),
        E::LifeLoss { player: who } => format!("{} would lose life", player(who)),
        E::Destroy { target } => format!("{} would be destroyed", article(&objects(target))),
        E::ZoneChange { object, from, to } => {
            let object = article(&objects(object));
            match (from, to) {
                (None, Some(Zone::Battlefield)) => format!("{object} would enter the battlefield"),
                (Some(Zone::Battlefield), Some(Zone::Graveyard)) => format!("{object} would die"),
                (Some(from), Some(to)) => format!(
                    "{object} would be put into {} from {}",
                    zone_phrase(to),
                    zone_phrase(from)
                ),
                (None, Some(to)) => format!("{object} would be put into {}", zone_phrase(to)),
                (Some(from), None) => format!("{object} would leave {}", zone_phrase(from)),
                (None, None) => format!("{object} would change zones"),
            }
        }
        E::DrawInstruction { player: who, minimum } => match minimum {
            0 | 1 => format!("{} would draw a card", player(who)),
            n => format!("{} would draw {} or more cards", player(who), count_words(*n)),
        },
        E::Untap { object, during_controllers_untap_step } => format!(
            "{} would untap{}",
            article(&objects(object)),
            if *during_controllers_untap_step { " during its controller's untap step" } else { "" }
        ),
        E::LifeReducedToZeroOrLess { player: who } => {
            format!("{} life total would be reduced to 0 or less", possessive(who))
        }
        E::SourceDestructionRegenerates => "this source would be destroyed".to_string(),
        other => format!("{} would happen", sr(other)),
    }
}

/// "If a source would deal 4 or more damage to a permanent or player, it
/// deals 3 damage instead."
fn amount_replacement(
    event: &ironsmith_core::AmountEventSpec,
    modifier: &ironsmith_core::AmountModifierSpec,
    optional: bool,
) -> String {
    use ironsmith_core::{AmountEventSpec as E, AmountModifierSpec as M};
    // `quantity` is "much damage" or "many cards"; `noun` is "damage"/"cards".
    let amount = |quantity: &str, noun: &str| match modifier {
        M::Multiply(factor) => format!("{} that {quantity}", multiple(*factor)),
        M::Add(n) => format!("that {quantity} plus {n}"),
        M::Subtract(n) => format!("that {quantity} minus {n}"),
        M::SetTo(n) => format!("{n} {noun}"),
        M::Half { round_up } => format!("half that {noun}, rounded {}", if *round_up { "up" } else { "down" }),
        M::AllButOne => format!("all but one of that {noun}"),
    };
    match event {
        E::Damage { source_filter, player: who, object, combat_only, minimum } => {
            let mut recipients = Vec::new();
            if let Some(object) = object {
                recipients.push(article(&objects(object)));
            }
            if let Some(who) = who {
                recipients.push(player(who));
            }
            let to = if recipients.is_empty() { String::new() } else { format!(" to {}", recipients.join(" or ")) };
            format!(
                "If {} would deal {}{}damage{to}, {}it deals {} instead",
                damage_source(source_filter),
                minimum.map_or(String::new(), |n| format!("{n} or more ")),
                if *combat_only { "combat " } else { "" },
                may(optional),
                amount("much damage", "damage")
            )
        }
        E::KeywordAction { action, performer } => {
            let verb = action.infinitive();
            let verb = verb.strip_suffix(" cards").unwrap_or(verb);
            let subject = if matches!(performer, PlayerFilter::You) { "you" } else { "they" };
            format!(
                "If {} would {verb} one or more cards, {subject} {}{verb} {} instead",
                player(performer),
                if optional { "may " } else { "" },
                amount("many cards", "cards")
            )
        }
        other => format!("If {} would happen, {}{} its amount instead", sr(other), may(optional), sr(modifier)),
    }
}

/// A companion's deck-building condition (CR 702.139).
fn companion_condition(condition: &ironsmith_core::CompanionDeckCondition) -> String {
    use ironsmith_core::CompanionDeckCondition as C;
    match condition {
        C::OnlyManaValueParity { even, lands_are_exempt } => format!(
            "Your starting deck contains only cards with {} mana values{}",
            if *even { "even" } else { "odd" },
            if *lands_are_exempt { " and land cards" } else { "" }
        ),
        C::NoRepeatedManaSymbols => {
            "No card in your starting deck has more than one of the same mana symbol in its mana cost".to_string()
        }
        C::CreatureSubtypes(subtypes) => format!(
            "Each creature card in your starting deck is a {} card",
            or_list(&subtypes.iter().map(|subtype| format!("{subtype:?}")).collect::<Vec<_>>())
        ),
        C::NonlandManaValueAtLeast(n) => format!(
            "Your starting deck contains only cards with mana value {n} or greater and land cards"
        ),
        C::PermanentManaValueAtMost(n) => {
            format!("Each permanent card in your starting deck has mana value {n} or less")
        }
        C::UniqueNonlandNames => "Each nonland card in your starting deck has a different name".to_string(),
        C::SharedNonlandCardType => "Each nonland card in your starting deck shares a card type".to_string(),
        C::CardsAboveMinimumDeckSize(n) => format!(
            "Your starting deck contains at least {} cards more than the minimum deck size",
            count_words(*n as u32)
        ),
        C::PermanentsHaveActivatedAbility => {
            "Each permanent card in your starting deck has an activated ability".to_string()
        }
    }
}

fn or_list(options: &[String]) -> String {
    match options {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} or {second}"),
        [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
    }
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
        P::CantAttackUnlessCondition { condition, .. } => cant_attack_unless(condition),
        P::ModifyDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, delta, dynamic_delta, noncombat_only, ..
        } => format!(
            "If {}, it deals that much damage {} instead",
            damage_scope(source_filter, target_player_filter, target_object_filter,
                if *noncombat_only { "noncombat " } else { "" }),
            match dynamic_delta {
                Some(value) => format!("plus {}", describe_value(value)),
                None if *delta < 0 => format!("minus {}", -delta),
                None => format!("plus {delta}"),
            }
        ),
        P::ExileWouldDieInstead {
            filter, damaged_by, damager_filter, exile_with_counters, follow_up_effects, ..
        } => {
            let mut subject = objects(filter);
            if let Some(source) = damaged_by {
                subject.push_str(&format!(" dealt damage by {} this turn", damaged_by_subject(source)));
            }
            if let Some(damager) = damager_filter {
                subject.push_str(&format!(" dealt damage by {} this turn", objects(damager)));
            }
            let mut text = format!("If {subject} would die, exile it instead");
            for (counter, amount) in exile_with_counters {
                text.push_str(&format!(
                    " with {} {} counter{} on it",
                    count_words(*amount),
                    describe_counter_type(*counter),
                    if *amount == 1 { "" } else { "s" }
                ));
            }
            if !follow_up_effects.is_empty() {
                text.push_str(&format!(". {}", capitalize_first(&effects(follow_up_effects))));
            }
            text
        }
        P::EntersWithCountersIfCondition { counter, count, condition, added_abilities, .. } => {
            let plural = !matches!(count, crate::effect::Value::Fixed(1));
            let counters = format!(
                "{} {} counter{} on it",
                if plural { value_words(count) } else { "a".to_string() },
                describe_counter_type(*counter),
                if plural { "s" } else { "" }
            );
            // "This creature escapes with ..." is the escape-only entry.
            let mut text = if matches!(condition, crate::ConditionExpr::ThisSpellEscaped) {
                format!("This source escapes with {counters}")
            } else {
                format!("If {}, this source enters with {counters}", describe_condition(condition))
            };
            for ability in added_abilities {
                text.push_str(&format!(" and with {}", ability_model(ability)));
            }
            text
        }
        P::ExertAttack { only_if_not_exerted_this_turn, linked_trigger, .. } => {
            let mut text = format!(
                "You may exert this source as it attacks{}",
                if *only_if_not_exerted_this_turn { " if it wasn't exerted this turn" } else { "" }
            );
            if let Some(trigger) = linked_trigger {
                text.push_str(&format!(". {}", when_you_do(trigger)));
            }
            text
        }
        P::EnlistAttack { linked_trigger, .. } => format!(
            "As this source attacks, you may tap a nonattacking creature you control without summoning sickness. {}",
            when_you_do(linked_trigger)
        ),
        P::AttackCost { attackers, covers_planeswalkers, cost, planeswalkers_only, .. } => format!(
            "{} can't attack {} unless {} {}",
            if attackers.source { "This source".to_string() } else { capitalize_first(&objects(attackers)) },
            if *planeswalkers_only {
                "planeswalkers you control"
            } else if *covers_planeswalkers {
                "you or planeswalkers you control"
            } else {
                "you"
            },
            if attackers.source { "you pay" } else { "their controller pays" },
            cost.display()
        ),
        P::BlockCost {
            blockers, blocker_is_attached_to_source, attackers, cost, also_attack_cost, attack_only, ..
        } => {
            let subject = if blockers.source {
                "this source".to_string()
            } else if *blocker_is_attached_to_source {
                format!("{} this is attached to", objects(blockers))
            } else {
                objects(blockers)
            };
            // Blocking any attacking creature needs no attacker qualifier.
            let blocked = objects(attackers);
            let blocked = if matches!(blocked.as_str(), "creature" | "a creature") {
                String::new()
            } else {
                format!(" {blocked}")
            };
            let action = if *attack_only {
                "attack".to_string()
            } else {
                format!("{}block{blocked}", if *also_attack_cost { "attack or " } else { "" })
            };
            format!(
                "{} can't {action} unless {} {}",
                capitalize_first(&subject),
                if blockers.source { "you pay" } else { "its controller pays" },
                cost.display()
            )
        }
        P::EntersWithCounterChoice { counter_types, count, condition, kinds, .. } => {
            let amount = match count {
                crate::effect::Value::Fixed(1) => "a".to_string(),
                other => value_words(other),
            };
            let choices = counter_types
                .iter()
                .map(|counter| format!("{amount} {} counter", describe_counter_type(*counter)))
                .collect::<Vec<_>>();
            let choice = if *kinds > 1 {
                format!("your choice of {} different counters from among {}", count_words(*kinds), or_list(&choices))
            } else {
                format!("your choice of {}", or_list(&choices))
            };
            match condition {
                Some(crate::ConditionExpr::ThisSpellEscaped) => format!("This source escapes with {choice} on it"),
                Some(condition) => format!("If {}, this source enters with {choice} on it", describe_condition(condition)),
                None => format!("This source enters with {choice} on it"),
            }
        }
        P::ActorCountersAddition { .. }
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
        P::Companion(condition) => if model.id == Some(ironsmith_core::StaticAbilityId::Companion) {
            format!("Companion \u{2014} {}", companion_condition(condition))
        } else {
            ironsmith_core::semantic_audit::unverified_marker("Companion", model, &[])
        },
        P::CopyActivatedAbilities(spec) => {
            let mut text = format!(
                "This source has all {}abilities of all {}",
                if spec.only_loyalty { "loyalty " } else { "activated " },
                pluralize_relative_object_phrase(strip_indefinite_article(&objects(&spec.filter)))
            );
            if let Some(counter) = spec.counter {
                text.push_str(&format!(" with a {} counter on them", describe_counter_type(counter)));
            }
            if spec.exclude_source_name {
                text.push_str(" except those with this object's name");
            }
            if spec.exclude_mana_abilities {
                text.push_str(" except mana abilities");
            }
            if spec.exclude_source_id && spec.filter.controller != Some(PlayerFilter::Opponent) {
                text.push_str(" other than this object");
            }
            if spec.force_once_each_turn {
                text.push_str(". Each of those abilities can be activated only once each turn");
            }
            text
        }
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
        P::ThisSpellXMaximum { maximum, definition, .. } => match definition {
            Some(value) => format!("X is defined as {}", describe_value(value)),
            None => format!("X is at most {}", describe_value(maximum)),
        },
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
        P::Enchant(filter) => format!("Enchant {}", describe_enchant_filter(filter)),
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
        P::DuplicateMatchingTriggeredAbilities { source_filter, event_matcher, count, .. } => {
            let abilities = source_filter.as_ref().map_or("a triggered ability".to_string(), |filter| {
                format!("a triggered ability of {}", with_indefinite_article(&objects(filter)))
            });
            match event_matcher {
                Some(trigger) => format!(
                    "If {} causes {abilities} to trigger, that ability triggers {}",
                    trigger_gerund(trigger),
                    times(*count)
                ),
                None => format!("If {abilities} triggers, that ability triggers {}", times(*count)),
            }
        }
        P::SuppressMatchingTriggeredAbilities { source_filter, event_matcher, .. } => {
            let abilities = source_filter.as_ref().map_or("abilities".to_string(), |filter| {
                format!("abilities of {}", pluralize_relative_object_phrase(&strip_indefinite_article(&objects(filter))))
            });
            match event_matcher {
                Some(trigger) => format!("{} doesn't cause {abilities} to trigger", capitalize_first(&trigger_gerund(trigger))),
                None => format!("Triggered {abilities} don't trigger"),
            }
        }
        P::AdditionalLifeCostPerTarget(life) => {
            format!("This spell costs an additional {life} life to cast for each target")
        }
        P::CommanderTaxLifeSubstitution { life_per_previous_cast } => format!(
            "You may pay {life_per_previous_cast} life for each previous cast instead of commander tax"
        ),
        P::ChooseColorAsEnters { excluded, count, .. } => format!(
            "As this permanent enters, choose {} color{}{}",
            if *count == 1 { "a".to_string() } else { count_words(*count) },
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
            "As this permanent enters, {} {}{}",
            if *each_player { "each player chooses" } else { "choose" },
            or_list(options),
            if *at_random { " at random" } else { "" }
        ),
        P::ChoosePowerToughnessAsEntersOrTurnsFaceUp { options, at_random, .. } => {
            let options = options
                .iter()
                .map(|option| {
                    let mut text = format!("{}/{}", option.power, option.toughness);
                    if let Some(types) = &option.card_types {
                        text.push_str(&format!(" {}", sr(types)));
                    }
                    for subtype in &option.added_subtypes {
                        text.push_str(&format!(" {subtype:?}"));
                    }
                    let abilities = option
                        .abilities
                        .iter()
                        .map(|ability| {
                            let text = audit_payload_reading(ability).unwrap_or_else(|| {
                                crate::static_abilities::StaticAbility::from_model(ability.clone()).display()
                            });
                            lowercase_first(text.trim().trim_end_matches('.'))
                        })
                        .collect::<Vec<_>>();
                    if !abilities.is_empty() {
                        text.push_str(&format!(" with {}", abilities.join(" and ")));
                    }
                    text
                })
                .collect::<Vec<_>>();
            format!(
                "As this source enters or is turned face up, it becomes {}{}",
                if *at_random { "one of these at random: " } else { "your choice of " },
                or_list(&options)
            )
        }
        P::EnterAsCopyAsEnters { spec, .. } => enter_as_copy(spec),
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
            "If {}, it deals at least {} instead",
            damage_scope(source_filter, target_player_filter, target_object_filter,
                if *noncombat_only { "noncombat " } else { "" }),
            describe_value(floor)
        ),
        P::DoubleDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, factor, combat_only, noncombat_only, ..
        } => format!(
            "If {}, it deals {} instead",
            damage_scope(source_filter, target_player_filter, target_object_filter,
                if *combat_only { "combat " } else if *noncombat_only { "noncombat " } else { "" }),
            if *factor == 2 { "double that damage".to_string() } else { format!("{} that much damage", multiple(*factor)) }
        ),
        P::PreventHalfDamageReplacement {
            source_filter, target_player_filter, target_object_filter, round_up, ..
        } => format!(
            "If {}, prevent half that damage, rounded {}",
            damage_scope(source_filter, target_player_filter, target_object_filter, ""),
            if *round_up { "up" } else { "down" }
        ),
        P::DoubleCountersReplacement {
            filter, player_filter, counter_type, actor, includes_permanents, halve, effect_only, ..
        } => {
            let kind = counter_type.map_or("one or more".to_string(), |c| describe_counter_type(c));
            // The object filter applies unless only a player filter is set.
            let mut recipients = Vec::new();
            if *includes_permanents || player_filter.is_none() {
                recipients.push(with_indefinite_article(&objects(filter)));
            }
            if let Some(p) = player_filter {
                recipients.push(player(p));
            }
            let putter = match actor {
                Some(a) => player(a),
                None if *effect_only => "an effect".to_string(),
                None => "a player or effect".to_string(),
            };
            let kind = if counter_type.is_some() { format!("one or more {kind}") } else { kind };
            if matches!(actor, Some(PlayerFilter::You)) {
                let noun = match (*includes_permanents || player_filter.is_none(), player_filter.is_some()) {
                    (true, true) => "permanent or player",
                    (false, true) => "player",
                    _ => "permanent",
                };
                return Some(format!(
                    "If you would put {kind} counters on {}, put {} that many of each of those kinds of counters on that {noun} instead",
                    recipients.join(" or "),
                    if *halve { "half" } else { "twice" }
                ));
            }
            format!(
                "If {putter} would put {kind} counters on {}, it puts {} that many of those counters on that {} instead",
                recipients.join(" or "),
                if *halve { "half" } else { "twice" },
                if player_filter.is_some() && !*includes_permanents { "player" } else { "permanent" }
            )
        }
        P::AddCountersPlacementReplacement { filter, player_filter, counter_type, additional, cause, .. } => format!(
            "If one or more {}counters would be put on {}{}{}, that many plus {} {} put on that permanent instead",
            counter_type.map_or(String::new(), |c| format!("{} ", describe_counter_type(c))),
            article(&objects(filter)),
            player_filter.as_ref().map_or(String::new(), |p| format!(" or {}", player(p))),
            cause.as_ref().map_or(String::new(), |c| format!(" by {}", sr(c))),
            count_words(*additional as u32),
            if counter_type.is_some() {
                format!("{} counter{} are", describe_counter_type(counter_type.unwrap()), if *additional == 1 { "" } else { "s" })
            } else {
                "of each of those kinds of counters are".to_string()
            }
        ),
        P::PlayerCounterPerTurnLimitReplacement { player_filter, counter_type, maximum, .. } => format!(
            "{} can't get more than {maximum} {} counters each turn",
            capitalize_first(&player(player_filter)),
            describe_counter_type(*counter_type)
        ),
        P::DoubleTokenCreationReplacement { controller, .. } => format!(
            "If an effect would create one or more tokens under {} control, it creates twice that many of those tokens instead",
            possessive(controller)
        ),
        P::MultiplyTokenCreationReplacement { controller, token_filter, factor, .. } => format!(
            "If an effect would create one or more {} under {} control, it creates {} that many of those tokens instead",
            token_filter.as_ref().map_or("tokens".to_string(), |f| pluralize_relative_object_phrase(&strip_indefinite_article(&objects(f)))),
            possessive(controller),
            multiple(*factor)
        ),
        P::AddTokenCreationReplacement { controller, token_filter, additional_token, additional, per_created, .. } => format!(
            "If one or more {} would be created under {} control, also create {additional} {}{}",
            objects(token_filter),
            possessive(controller),
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
            "If you would draw a card{}, draw {} cards{} instead",
            except_first(*except_first_of_draw_step),
            count_words(extra + 1),
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
        P::RedirectWouldEnter { filter, not_cast, destination, or_no_mana_spent, .. } => {
            let destination = match format!("{destination:?}").as_str() {
                "Exile" => "exile it".to_string(),
                "Graveyard" => "put it into its owner's graveyard".to_string(),
                "Hand" => "return it to its owner's hand".to_string(),
                _ => format!("put it into {}", sr(destination)),
            };
            format!(
                "If {} would enter{}{}, {destination} instead",
                article(&objects(filter)),
                if *not_cast { " and it wasn't cast" } else { "" },
                if *or_no_mana_spent { " or no mana was spent to cast it" } else { "" },
            )
        }
        P::ManaProductionReplacement { source_filter, minimum_amount, replacement_mana, .. } => format!(
            "If {} would produce {minimum_amount} or more mana, it produces {} instead",
            objects(source_filter),
            replacement_mana.iter().map(|symbol| describe_mana_symbol(*symbol)).collect::<Vec<_>>().join(" ")
        ),
        P::ManaProductionMultiplierReplacement { source_filter, factor, .. } => format!(
            "If you tap {} for mana, it produces {} as much of that mana instead",
            with_indefinite_article(&objects(source_filter)),
            multiple(*factor)
        ),
        P::DoubleLifeChangeReplacement { player: who, loss, .. } => {
            let verb = if *loss { "lose" } else { "gain" };
            let (subject, verb_after) = match who {
                PlayerFilter::You => ("you", verb.to_string()),
                _ => ("they", verb.to_string()),
            };
            format!(
                "If {} would {verb} life, {subject} {verb_after} twice that much life instead",
                player(who)
            )
        }
        P::ManaSpendPermission { permission, .. } => mana_spend_permission(permission),
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
            "If {} would deal damage to you, prevent {amount} of that damage",
            damage_source_objects(source_filter)
        ),
        P::ReplaceDamageWithCountersInstead { counter_type, source_filter, target_filter, combat_only, .. } => format!(
            "If {} would deal {}damage to {}, put that many {} counters on it instead",
            objects(source_filter),
            match combat_only { Some(true) => "combat ", Some(false) => "noncombat ", None => "" },
            objects(target_filter),
            describe_counter_type(*counter_type)
        ),
        P::NativeAlternativeCastFromZone { zone, method } => {
            // The card is cast from its owner's zone: "from your graveyard
            // using its blitz ability".
            let from = match zone {
                Zone::Graveyard => "your graveyard".to_string(),
                Zone::Hand => "your hand".to_string(),
                Zone::Library => "your library".to_string(),
                Zone::Exile => "exile".to_string(),
                other => sr(other),
            };
            format!(
                "You may cast this card from {from} using its {} ability",
                sr(method).to_ascii_lowercase()
            )
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
            "If {} would be created under {} control, {}{}{} using {}",
            objects(token_filter),
            possessive(controller),
            may(*optional),
            if *choose_one { "choose one and " } else { "" },
            sr(mode),
            effects(templates)
        ),
        P::SpellManaSpendingRestriction(ironsmith_core::mana::ManaSpendingRestriction::OnX {
            colors,
            maximum_per_color: None,
        }) if !colors.is_empty() => {
            use ironsmith_core::color::Color;
            let names = [Color::White, Color::Blue, Color::Black, Color::Red, Color::Green]
                .into_iter()
                .filter(|color| colors.contains(*color))
                .map(|color| color.name().to_string())
                .collect::<Vec<_>>();
            format!("Spend only {} mana on X", or_list(&names))
        }
        P::SpellManaSpendingRestriction(ironsmith_core::mana::ManaSpendingRestriction::NoMana) => {
            "You can't spend mana to cast this spell".to_string()
        }
        P::SpellManaSpendingRestriction(restriction) => {
            format!("Mana spent on this spell is restricted: {}", sr(restriction))
        }
        P::ManaProductionRewrite { rule, .. } => format!("Mana production is rewritten: {}", sr(rule)),
        P::ExtraDieIgnoreLowest { player: who, additional } => format!(
            "If {} would roll a die, they roll {additional} more and ignore the lowest",
            player(who)
        ),
        P::ExtraCoinIgnoreOne { player: who } => format!(
            "If {} would flip a coin, instead {} two coins and ignore one",
            player(who),
            if matches!(who, PlayerFilter::You) { "flip" } else { "they flip" }
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
        P::EventReplacementWithEffects {
            event: ironsmith_core::ReplacedEventSpec::SourceDestructionRegenerates,
            replacement_effects,
            ..
        } if replacement_effects.is_empty() => "If this source would be destroyed, regenerate it".to_string(),
        P::EventReplacementWithEffects { event, replacement_effects, optional, .. } => format!(
            "If {}, {}{} instead",
            replaced_event(event),
            may(*optional),
            effects(replacement_effects)
        ),
        P::EventAmountReplacement { event, modifier, optional, .. } => amount_replacement(event, modifier, *optional),
        P::GoadMatching { filter } => {
            let description = objects(filter);
            if ["enchanted ", "equipped ", "this ", "that ", "the ", "target "]
                .iter()
                .any(|prefix| description.starts_with(prefix))
            {
                format!("{} is goaded", capitalize_first(&description))
            } else {
                format!(
                    "{} are goaded",
                    capitalize_first(&pluralize_relative_object_phrase(strip_indefinite_article(&description)))
                )
            }
        }
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
        P::Conditional { ability, condition, .. } => {
            let inner = audit_payload_reading(ability).unwrap_or_else(|| {
                crate::static_abilities::StaticAbility::from_model(ability.as_ref().clone()).display()
            });
            let condition = describe_condition(condition);
            let trimmed = inner.trim();
            // A replacement or prevention instruction is a whole sentence;
            // its condition leads ("As long as this creature is untapped,
            // all damage ... is dealt to this creature instead").
            let sentence = trimmed.ends_with('.')
                || ["If ", "Prevent ", "All damage ", "All combat damage "]
                    .iter()
                    .any(|head| trimmed.starts_with(head));
            match condition.strip_prefix("as long as ").or(Some(condition.as_str())) {
                Some(rest) if sentence && !trimmed.contains(" as long as ") => format!(
                    "As long as {rest}, {}",
                    lowercase_first(trimmed.trim_end_matches('.'))
                ),
                _ => format!("{inner} as long as {condition}"),
            }
        }
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
                format!("Partner\u{2014}{}", capitalize_first(group))
            } else {
                "Partner".to_string()
            }
        }
        P::DeckConstructionCopyLimit { maximum } => match maximum {
            None => "A deck can have any number of copies of this card".to_string(),
            Some(limit) => format!(
                "A deck can have up to {} copies of this card",
                ironsmith_core::cardinal_word(*limit).unwrap_or_else(|| limit.to_string())
            ),
        },
        P::DeckConstructionColorChoice { count } => format!(
            "As its owner creates their deck, they choose {count} different colors for this card"
        ),
        P::Toxic { amount } => format!("Toxic {amount}"),
        P::MustAttackDraftNotedPlayers { card_name } => format!(
            "This creature attacks a player you noted for cards named {card_name} each combat if able"
        ),
        P::DraftRule { rule } => {
            let mut parts = Vec::new();
            if rule.face_up {
                parts.push("draft this card face up".to_string());
            }
            if rule.revealed {
                parts.push("this card is revealed as it's drafted".to_string());
            }
            if rule.notes_passing_player {
                parts.push("its drafter notes the player who passed it".to_string());
            }
            if rule.chosen_colors > 0 {
                parts.push(format!("{} colors are chosen as it's drafted", rule.chosen_colors));
            }
            if rule.exchange_pick {
                parts.push("its drafter may draft an additional card by putting this card into that booster pack".to_string());
            }
            if parts.is_empty() || rule.unmodeled {
                return Some(ironsmith_core::semantic_audit::unverified_marker(
                    "LabelDependentStatic", model, &[]));
            }
            let mut text = parts.join("; ");
            if let Some(first) = text.get_mut(..1) {
                first.make_ascii_uppercase();
            }
            text
        }
        P::KeywordMarkerMechanic { mechanic } => {
            use ironsmith_core::KeywordMarkerMechanic as M;
            let action = |saddles: bool| if saddles { "saddles Mounts and crews Vehicles" } else { "crews Vehicles" };
            match mechanic {
                M::AttackingDoesNotCauseTap => "Attacking doesn't cause this creature to tap".to_string(),
                M::CastAtInstantTiming => "You may cast this spell any time you could cast an instant".to_string(),
                M::DamageIsLethalToCreatures => "Any amount of damage this deals to a creature is enough to destroy it.".to_string(),
                M::CombatDamageAddsPoison { amount } => format!("Players dealt combat damage by this creature also get {amount} poison counters."),
                M::OverflowCombatDamage { through_planeswalkers } => if *through_planeswalkers {
                    "This creature can deal excess combat damage to the controller of the planeswalker it's attacking.".to_string()
                } else {
                    "This creature can deal excess combat damage to the player or planeswalker it's attacking.".to_string()
                },
                M::ExileGraveyardCardsForGenericMana => "Each card you exile from your graveyard while casting this spell pays for {1}.".into(),
                M::TapCreaturesForMana => "Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.".into(),
                M::DestroyAttachmentInstead => "If enchanted permanent would be destroyed, instead remove all damage from it and destroy this Aura.".into(),
                M::AttachWhileCreature => "This permanent can be attached while it's a creature.".into(),
                M::PermanentCountGrantsDesignation { minimum, historic_only, enduring_story } => format!("If you control {minimum} or more {}, you {} for the rest of the game.", if *historic_only { "artifacts, legendaries, and/or Sagas" } else { "permanents" }, if *enduring_story { "have an enduring story" } else { "get the city's blessing" }),
                M::ExileOnStackExitIfCastFrom { zone } => format!("If this spell was cast from your {}, exile it instead of putting it anywhere else any time it would leave the stack.", format!("{zone:?}").to_ascii_lowercase()),
                M::TapArtifactsForGenericMana => "Each artifact you tap after you're done activating mana abilities pays for {1}.".into(),
                M::PhaseDuringUntap => "This phases in or out before you untap during each of your untap steps.".to_string(),
                M::DestructionImmune => "This permanent cannot be destroyed.".to_string(),
                M::DamageAsCounters { poison_players } => if *poison_players {
                    "This deals damage to creatures in the form of -1/-1 counters and to players in the form of poison counters.".to_string()
                } else {
                    "This deals damage to creatures in the form of -1/-1 counters.".to_string()
                },
                M::DamageGrantsControllerLife => "Damage dealt by this creature also causes you to gain that much life.".to_string(),
                M::EarlyCombatDamage { also_regular } => if *also_regular {
                    "This creature deals both first-strike and regular combat damage".to_string()
                } else { "This creature deals combat damage before creatures without first strike".to_string() },
                M::EntryCountersFromSpentColors => "If this permanent is entering as a creature, ignoring type-changing effects that would affect it, it enters with a +1/+1 counter on it for each color of mana spent to cast it. Otherwise, it enters with a charge counter on it for each color of mana spent to cast it.".into(),
                M::EntryCountersAndFollowingAbilities { amount } => format!("When this creature enters, put {amount} +1/+1 counters on target creature. If that's another creature, it gains the following abilities until end of turn."),
                M::Sunburst => "Sunburst".to_string(),
                M::Compleated => "Compleated".to_string(),
                M::DayNightTransformDefinition { day_face } => if *day_face { "If it's neither day nor night, it becomes day. If it's night, this permanent enters transformed. As it becomes night, if this permanent is front face up, transform it. This permanent can't transform except due to this ability. If a player casts no spells during their own turn, it becomes night next turn.".into() } else { "If it's neither day nor night and no permanent establishes day, it becomes night. As it becomes day, if this permanent is back face up, transform it. This permanent can't transform except due to this ability. If a player casts at least two spells during their own turn, it becomes day next turn.".into() },
                M::CombatBandsAndDamageChoice => "As you declare attackers, you may group one or more attacking creatures with this ability and up to one other attacking creature into a band. Each creature can belong to only one band, and all members must attack the same player, planeswalker, or battle. The band lasts for the rest of combat even if this ability is lost; creatures removed from combat leave the band. Members don't share abilities. A creature blocking any member blocks all members, and an effect that blocks one member blocks the whole band. If this creature is blocking an attacker, you choose how that attacker divides its combat damage among its blockers. If this creature is blocked, you choose how its blockers divide their combat damage among the creatures they block.".into(),
                M::AssignCreatureSectors => "As a state-based action, each creature without a sector designation is assigned alpha, beta, or gamma sector by its controller. Players who don't control a permanent with this ability assign first, then the other players assign, in active-player order within each group. A permanent keeps its sector until no player controls a permanent with this ability or an ability whose source has this ability. Sector designations aren't copiable.".into(),
                M::InitializeControllerSpeed => "If you have no speed while you control this permanent, your speed becomes 1 as a state-based action. Whenever one or more opponents lose life during your turn, if your speed is less than 4, your speed increases by 1. This inherent ability has no source and triggers only once each turn. Your speed remains after this permanent leaves the battlefield. Max speed is 4.".into(),
                M::ChooseSagaEntryChapter => "As this Saga enters, choose a number between one and its final chapter number. It enters with the chosen number of lore counters. Chapter abilities of this Saga can't trigger the turn it entered unless it has exactly the number of lore counters specified in that chapter's symbol.".into(),
                M::AnotherPlayerPaysGeneric => "As you cast this spell, before activating mana abilities, you may choose another player. That player may activate mana abilities before you do and may pay any amount of the generic mana in this spell's total cost before you pay the rest.".into(),
                M::EntryLoyaltyReductionPerLifePip => "For each Phyrexian mana symbol paid with life, this planeswalker enters with two fewer loyalty counters.".into(),
                M::Epic => "Epic".to_string(),
                M::RecurringCopyOnFirstNameResolution => "If this is the first time a spell you control with this spell's name has resolved this game, at the beginning of each of your precombat main phases for the rest of the game, create a copy of this object in exile. You may cast the copy without paying its mana cost. Exile this spell.".into(),
                M::RevealSharedTypesForEntryCounters { amount } => format!("As this creature enters, reveal any number of cards from your hand that share a creature type with it. This creature enters with {amount} +1/+1 counters on it for each card revealed this way."),
                M::CastBothSplitHalvesFromHand => "You may cast one or both halves of this card from your hand. If you cast both, combine their characteristics and mana costs into one spell, then follow the left half's instructions before the right half's instructions.".into(),
                M::LinkOnGraveyardArrival => "On a permanent, this ability means \"When this permanent is put into a graveyard from the battlefield, exile it haunting target creature.\" On an instant or sorcery spell, this ability means \"When this spell is put into a graveyard during its resolution, exile it haunting target creature.\"".into(),
                M::Paradigm => "Paradigm".to_string(),
                M::AssignsCombatDamageAsThoughPowerPositive => {
                    "This creature assigns combat damage as though its power were positive".to_string()
                }
                M::StationsUsingToughness => {
                    "This creature stations permanents using its toughness rather than its power".to_string()
                }
                M::ContributesToughness { saddles } => format!(
                    "This creature {} using its toughness rather than its power", action(*saddles)
                ),
                M::ContributesExtraPower { saddles, amount } => format!(
                    "This creature {} as though its power were {amount} greater", action(*saddles)
                ),
                M::LoyaltyCounterPaysCrewCost => {
                    "You may remove a loyalty counter from a planeswalker you control rather than pay this Vehicle's crew cost".to_string()
                }
            }
        }
        _ => return None,
    })
}
