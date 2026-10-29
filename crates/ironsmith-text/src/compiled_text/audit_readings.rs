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

/// `except_first` for a drawer other than you: "the first one they draw in
/// each of their draw steps".
fn except_first_for(except: bool, drawer: &PlayerFilter) -> &'static str {
    match (except, drawer) {
        (true, PlayerFilter::You) | (false, _) => except_first(except),
        (true, _) => " except the first one they draw in each of their draw steps",
    }
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
    } else if recipients == ["a player", "a permanent"] {
        // Oracle prints the any-recipient pair "to a permanent or player"
        // (Furnace of Rath, City on Fire, Fiery Emancipation).
        " to a permanent or player".to_string()
    } else {
        format!(" to {}", recipients.join(" or "))
    };
    let source = damage_source_objects(source);
    // The one object an Equipment or Aura is attached to is "equipped
    // creature" / "enchanted creature" (Inquisitor's Flail).
    let source = match source.as_str() {
        "a creature with this Equipment attached to it" => "equipped creature".to_string(),
        "a creature with this Aura attached to it" => "enchanted creature".to_string(),
        _ => source,
    };
    format!("{source} would deal {kind}damage{to}")
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
    // "Creatures you control enter as a copy of this creature" (Essence of
    // the Wild): the entering objects are the affected ones, not the source.
    let source = match (&spec.affected_filter, spec.copy_source_self) {
        (Some(_), true) => "this source".to_string(),
        _ => source,
    };
    let mut text = if let Some(affected) = &spec.affected_filter {
        let entering = capitalize_first(&pluralize_relative_object_phrase(strip_indefinite_article(&objects(affected))));
        if spec.may {
            format!("You may have {entering} enter{tapped} as a copy of {source}")
        } else {
            format!("{entering} enter{tapped} as a copy of {source}")
        }
    } else if let Some(pair) = &spec.linked_exile_pair {
        // "As The Mimeoplasm enters, you may exile two creature cards from
        // graveyards. If you do, it enters as a copy of one of those cards
        // with a number of additional +1/+1 counters on it equal to the power
        // of the other card."
        format!(
            "As this source enters, {}exile two {}. If you do, it enters{tapped} as a copy of one of those cards with a number of additional {} counters on it equal to the power of the other card",
            if spec.may { "you may " } else { "" },
            pluralize_relative_object_phrase(strip_indefinite_article(&objects(&spec.filter))),
            describe_counter_type(pair.counter_type)
        )
    } else if spec.may {
        format!("You may have this source enter{tapped} as a copy of {source}")
    } else {
        format!("This source enters{tapped} as a copy of {source}")
    };
    // "As this artifact enters, you may have it become a copy of any
    // creature on the battlefield until end of turn" (Cursed Mirror): a
    // timed copy is a "become", read with its duration.
    let copy_until_end_of_turn = matches!(spec.copy_duration, Some(ironsmith_core::Until::EndOfTurn))
        && spec.affected_filter.is_none()
        && spec.linked_exile_pair.is_none();
    if copy_until_end_of_turn {
        text = if spec.may {
            format!("As this source enters, you may have it become a copy of {source}{tapped} until end of turn")
        } else {
            format!("As this source enters, it becomes a copy of {source}{tapped} until end of turn")
        };
    }
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
        let colors = crate::color::Color::ALL
            .into_iter()
            .filter(|color| spec.added_colors.contains(*color))
            .map(|color| color.name().to_string())
            .collect::<Vec<_>>();
        exceptions.push(format!("it's {} in addition to its other colors", colors.join(" and ")));
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
    // "it enters with an additional loyalty counter on it if it's a
    // planeswalker" (Spark Double): an exception per copied kind.
    for conditional in &spec.conditional_additional_counters {
        exceptions.push(format!(
            "it enters with {} additional {} counter{} on it if it's {}",
            if conditional.count == 1 { "an".to_string() } else { count_words(conditional.count) },
            describe_counter_type(conditional.counter_type),
            if conditional.count == 1 { "" } else { "s" },
            with_indefinite_article(&objects(&conditional.source_filter))
        ));
    }
    for supertype in &spec.removed_supertypes {
        exceptions.push(format!("it isn't {}", format!("{supertype:?}").to_ascii_lowercase()));
    }
    if !spec.added_abilities.is_empty() {
        let abilities = spec.added_abilities.iter().map(ability_model).collect::<Vec<_>>();
        exceptions.push(format!("it has {}", abilities.join(" and ")));
    }
    // "except it enters with a shield counter on it if you control that
    // creature" (Undercover Operative): the counters' copied-object gate.
    let copies_gate = spec
        .additional_counters_source_filter
        .as_ref()
        .map_or(String::new(), |filter| format!(" if it copies {}", objects(filter)));
    for (counter, amount) in &spec.additional_counters {
        exceptions.push(format!(
            "it enters with {} additional {} counter{} on it{copies_gate}",
            if *amount == 1 { "an".to_string() } else { count_words(*amount) },
            describe_counter_type(*counter),
            if *amount == 1 { "" } else { "s" }
        ));
    }
    // "except it enters with X additional +1/+1 counters on it" (Altered Ego)
    for counter in &spec.additional_x_counters {
        exceptions.push(format!(
            "it enters with X additional {} counters on it{copies_gate}",
            describe_counter_type(*counter)
        ));
    }
    // Copy exceptions applied as the copiable values are set (CR 707.9b,
    // 702.128a).
    use ironsmith_core::EnterAsCopyFollowup as Followup;
    for followup in &spec.copy_followups {
        match followup {
            Followup::RetainOwnColors => exceptions.push("it doesn't copy that creature's color".to_string()),
            Followup::RetainEmbalmCharacteristics => exceptions.push(
                "if this creature was embalmed, the token has no mana cost, it's white, and it's a Zombie in addition to its other types"
                    .to_string(),
            ),
            _ => {}
        }
    }
    if spec.keep_other_source_abilities {
        exceptions.push("it has this creature's other abilities".to_string());
    }
    if !exceptions.is_empty() {
        text.push_str(&format!(", except {}", exceptions.join(" and ")));
    }
    // Rarer fields keep a structural reading so none is silently dropped.
    let mut rest = Vec::new();

    if let Some(until) = &spec.copy_duration
        && !copy_until_end_of_turn
    {
        rest.push(format!("until {}", sr(until)));
    }

    // A copied-object counter gate with no counters to gate keeps its
    // structural reading.
    if spec.additional_counters.is_empty()
        && spec.additional_x_counters.is_empty()
        && let Some(filter) = &spec.additional_counters_source_filter
    {
        rest.push(format!("counters only if it copies {}", objects(filter)));
    }
    if let Some(filter) = &spec.added_abilities_source_filter { rest.push(format!("abilities only if it copies {}", objects(filter))); }
    let mut sentences = Vec::new();
    for followup in &spec.copy_followups {
        sentences.push(match followup {
            Followup::RetainOwnColors | Followup::RetainEmbalmCharacteristics => continue,
            Followup::ExileCopiedObject => "When you do, exile that card".to_string(),
            Followup::GainsHasteUntilEndOfTurn => "If you do, it gains haste until end of turn".to_string(),
            Followup::TapCopiedObjectFrozenWhileYouControlSource => {
                "When you do, tap the copied creature and it doesn't untap during its controller's untap step for as long as you control this creature".to_string()
            }
            Followup::CountersFromCopiedPermanent => {
                "If you do, it enters with additional counters on it equal to the same number and kinds of counters the copied permanent has on it".to_string()
            }
            #[allow(unreachable_patterns)]
            other => {
                rest.push(ironsmith_core::semantic_audit::humanize_variant(other));
                continue;
            }
        });
    }
    if !rest.is_empty() {
        text.push_str(&format!("; {}", rest.join("; ")));
    }
    for sentence in sentences {
        text.push_str(&format!(". {sentence}"));
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
        // "unless there is a Mountain on the battlefield" (Glacial Crasher).
        Spec::BattlefieldCountAtLeast { filter, count } if *count <= 1 => (
            "attack",
            format!("there is {} on the battlefield", article(&objects(filter))),
        ),
        Spec::BattlefieldCountAtLeast { filter, count } => (
            "attack",
            format!(
                "there are {} or more {} on the battlefield",
                count_words(*count as u32),
                pluralize_relative_object_phrase(strip_indefinite_article(&objects(filter)))
            ),
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
            Cost::ReturnPermanentsToOwnersHand { filter, count } if *count == 1 => {
                format!("you return {} to its owner's hand", article(&objects(filter)))
            }
            Cost::ReturnPermanentsToOwnersHand { filter, count } => format!(
                "you return {} {} to their owners' hands",
                count_words(*count),
                pluralize_relative_object_phrase(strip_indefinite_article(&objects(filter)))
            ),
            Cost::SacrificePermanents { filter, count } if *count == 1 => {
                format!("you sacrifice {}", article(&objects(filter)))
            }
            Cost::SacrificePermanents { filter, count } => format!(
                "you sacrifice {} {}",
                count_words(*count),
                pluralize_relative_object_phrase(strip_indefinite_article(&objects(filter)))
            ),
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

/// "Once during each of your turns, you may play lands and permanent spells
/// with mana value 3 or less from your graveyard": a play permission read
/// from its typed fields.
fn audit_play_from_grant(grant: &crate::grant::GrantSpec) -> Option<String> {
    if matches!(grant.grantable, ironsmith_core::Grantable::PlayFrom) && !grant.additional_zones.is_empty() {
        return audit_play_from_multi_zone_grant_r5a(grant);
    }
    if !matches!(grant.grantable, ironsmith_core::Grantable::PlayFrom) || !grant.additional_zones.is_empty() {
        return None;
    }
    let lead = match grant.usage_limit {
        None => "",
        Some(ironsmith_core::GrantUsageLimit::OnceEachTurn) => "Once each turn, ",
        Some(ironsmith_core::GrantUsageLimit::OnceDuringEachOfYourTurns) => "Once during each of your turns, ",
        Some(ironsmith_core::GrantUsageLimit::DuringYourTurns) => "During your turn, ",
    };
    let who = player(&grant.beneficiary);
    let mut filter = grant.filter.clone();
    filter.zone = None;
    let own = filter.owner.take();
    let cards = objects(&filter);
    let cards = pluralize_noun_phrase(strip_indefinite_article(&cards));
    let cards = if cards.is_empty() || cards == "permanents" { "cards".to_string() } else { cards };
    let whose = match own {
        Some(PlayerFilter::You) => "your ".to_string(),
        Some(other) => format!("{} ", possessive(&other)),
        None => String::new(),
    };
    let origin = match grant.zone {
        Zone::Graveyard if whose.is_empty() => "a graveyard".to_string(),
        Zone::Graveyard => format!("{whose}graveyard"),
        Zone::Library if grant.top_card_only => format!("the top of {}library", if whose.is_empty() { "your ".to_string() } else { whose.clone() }),
        Zone::Exile => "exile".to_string(),
        Zone::Hand => format!("{whose}hand"),
        other => sr(&other),
    };
    let mut text = format!("{lead}{who} may play {cards} from {origin}");
    match grant.cast_mana_spend_mode {
        ironsmith_core::value_model::ManaSpendMode::Normal => {}
        ironsmith_core::value_model::ManaSpendMode::AnyColor => {
            text.push_str(", and you may spend mana as though it were mana of any color to cast those spells")
        }
        ironsmith_core::value_model::ManaSpendMode::AnyType => {
            text.push_str(", and mana of any type can be spent to cast those spells")
        }
        ref other => text.push_str(&format!(", spending mana as {}", sr(other))),
    }
    if grant.instant_timing {
        text.push_str(". If you cast a spell this way, you may cast it as though it had flash");
    }
    for granted in grant.cast_this_way_grants.iter().chain(&grant.permanent_this_way_grants) {
        let reading = granted
            .compiled_model()
            .and_then(audit_payload_reading)
            .unwrap_or_else(|| granted.display());
        let reading = reading.trim().trim_end_matches('.');
        if !reading.is_empty() {
            text.push_str(&format!(". If you do, {}", lowercase_first(reading)));
        }
    }
    Some(capitalize_first(&text))
}

/// "Once during each of your turns, you may play a permanent card you own
/// from outside the game or from your graveyard": a play permission over
/// several origin zones, read from its typed zones, filter, owner and limit.
fn audit_play_from_multi_zone_grant_r5a(grant: &crate::grant::GrantSpec) -> Option<String> {
    if grant.top_card_only
        || grant.instant_timing
        || !grant.cast_this_way_grants.is_empty()
        || !grant.permanent_this_way_grants.is_empty()
        || !matches!(grant.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::Normal)
    {
        return None;
    }
    let lead = match grant.usage_limit {
        None => "",
        Some(ironsmith_core::GrantUsageLimit::OnceEachTurn) => "Once each turn, ",
        Some(ironsmith_core::GrantUsageLimit::OnceDuringEachOfYourTurns) => "Once during each of your turns, ",
        Some(ironsmith_core::GrantUsageLimit::DuringYourTurns) => "During your turn, ",
    };
    let who = player(&grant.beneficiary);
    let mut filter = grant.filter.clone();
    filter.zone = None;
    let own = filter.owner.take();
    let cards = strip_indefinite_article(&objects(&filter)).to_string();
    let cards = if cards.is_empty() {
        "card".to_string()
    } else if cards == "permanent" {
        "permanent card".to_string()
    } else {
        cards
    };
    let (owned, whose) = match own {
        Some(PlayerFilter::You) => (" you own", "your ".to_string()),
        Some(_) => return None,
        None => ("", String::new()),
    };
    let mut origins = Vec::new();
    for zone in std::iter::once(&grant.zone).chain(grant.additional_zones.iter()) {
        let origin = match zone {
            Zone::OutsideGame => "outside the game".to_string(),
            Zone::Graveyard if whose.is_empty() => "a graveyard".to_string(),
            Zone::Graveyard => format!("{whose}graveyard"),
            Zone::Hand if !whose.is_empty() => format!("{whose}hand"),
            Zone::Exile => "exile".to_string(),
            _ => return None,
        };
        origins.push(format!("from {origin}"));
    }
    let noun = if grant.usage_limit.is_some() { article(&cards) } else { pluralize_noun_phrase(&cards) };
    Some(capitalize_first(&format!("{lead}{who} may play {noun}{owned} {}", or_list(&origins))))
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
        // "to pay the activation costs of this artifact's abilities"
        // (Manascape Refractor, Quicksilver Elemental): the source's own
        // abilities read through its activation costs.
        Scope::ActivationCostsOf(filter) => {
            let holder = objects(filter);
            if holder.starts_with("this ") && holder.split(' ').count() == 2 {
                format!(" to pay the activation costs of {holder}'s abilities")
            } else {
                format!(" to activate abilities of {holder}")
            }
        }
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

/// "If a land is tapped for mana, it produces {B} instead of any other type":
/// a mana production rewrite read from its source, provenance, input,
/// output and quantity.
fn mana_output_rewrite(rule: &ironsmith_core::ManaOutputRewrite) -> String {
    use ironsmith_core::{ManaRewriteInput as In, ManaRewriteOutput as Out, ManaRewriteQuantity as Qty};
    let by = rule
        .controller
        .as_ref()
        .map_or(String::new(), |who| format!(" by {}", player(who)));
    let event = if rule.tapped_for_mana {
        format!("is tapped for mana{by}")
    } else {
        format!("would produce mana{by}")
    };
    let symbol = |symbol: &ironsmith_core::ManaSymbol| match symbol {
        ironsmith_core::ManaSymbol::Colorless => "colorless mana".to_string(),
        other => describe_mana_symbol(*other),
    };
    let replaced = match rule.input {
        In::Any => "any other type".to_string(),
        In::Colored => "each colored mana".to_string(),
        In::Symbol(input) => symbol(&input),
    };
    let amount = match rule.quantity {
        Qty::Preserve => String::new(),
        Qty::Exact(1) => " and amount".to_string(),
        Qty::Exact(n) => format!(" and amount, {} times", count_words(n)),
    };
    let source = objects(&rule.source_filter);
    match &rule.output {
        Out::ByBasicLandType(mapping) => {
            let types = ["Plains", "Islands", "Swamps", "Mountains", "Forests"];
            let pairs = types
                .iter()
                .zip(mapping.iter())
                .filter_map(|(land, output)| output.as_ref().map(|output| format!("{land} produce {}", symbol(output))))
                .collect::<Vec<_>>();
            let pairs = match pairs.len() {
                0 => String::new(),
                1 => pairs[0].clone(),
                n => format!("{}, and {}", pairs[..n - 1].join(", "), pairs[n - 1]),
            };
            let head = if source == "land" && by.is_empty() && rule.tapped_for_mana {
                "If tapped for mana".to_string()
            } else {
                format!("If {} {event}", article(&source))
            };
            format!("{head}, {pairs} instead of {replaced}{amount}")
        }
        output => {
            let produced = match output {
                Out::Symbol(output) => symbol(output),
                Out::ChosenColor => "mana of the chosen color".to_string(),
                Out::ChooseColor => "mana of a color of your choice".to_string(),
                Out::ByBasicLandType(_) => unreachable!(),
            };
            format!(
                "If {} {event}, it produces {produced} instead of {replaced}{amount}",
                article(&source)
            )
        }
    }
}

/// "After you roll a die, you may pay 1 life. If you do, increase or decrease
/// the result by 1": a die-roll adjustment read from its player, costs,
/// amount, reroll / exchange mode, die size and frequency.
fn die_roll_adjustment(adjustment: &ironsmith_core::static_ability_model::DieRollResultAdjustment) -> String {
    let who = player(&adjustment.player);
    let you = who == "you";
    let roll = if you { "roll" } else { "rolls" };
    let sides = adjustment
        .sides
        .map_or(String::new(), |sides| format!("{}-sided ", count_words(sides)));
    let mut costs = Vec::new();
    if let Some(mana) = &adjustment.mana_cost {
        costs.push(format!("pay {}", mana.to_oracle()));
    }
    if adjustment.life_cost > 0 {
        costs.push(format!("pay {} life", adjustment.life_cost));
    }
    if let Some(counter) = &adjustment.source_counter_cost {
        costs.push(format!(
            "remove {} from this source",
            with_indefinite_article(&format!("{} counter", describe_counter_type(*counter)))
        ));
    }
    let cost = costs.join(" and ");
    let once = if adjustment.once_each_turn { ". Do this only once each turn" } else { "" };
    let mut text = if adjustment.exchange_with_source_base_pt {
        format!(
            "If {who} would roll one or more {sides}dice, instead {} them and {who} may exchange one result with this source's base power or base toughness",
            if you { "roll" } else { "they roll" }
        )
    } else if adjustment.reroll {
        let pay = if cost.is_empty() { String::new() } else { format!("{cost} to ") };
        format!(
            "{}{who} may {pay}reroll one or more {sides}dice {who} rolled",
            if adjustment.once_each_turn { "Once each turn, " } else { "" }
        )
    } else if cost.is_empty() {
        format!(
            "After {who} {roll} a {sides}die, {who} may increase or decrease the result by {}{once}",
            adjustment.amount
        )
    } else {
        format!(
            "After {who} {roll} a {sides}die, {who} may {cost}. If {} do, increase or decrease the result by {}{once}",
            if you { "you" } else { "they" },
            adjustment.amount
        )
    };
    // Fields no phrasing above reads keep their structure.
    if (adjustment.exchange_with_source_base_pt || adjustment.reroll) && adjustment.amount != 0 {
        text.push_str(&format!(" and adjust the result by {}", adjustment.amount));
    }
    if adjustment.exchange_with_source_base_pt && (!cost.is_empty() || adjustment.once_each_turn) {
        text.push_str(&format!("; cost {}{once}", if cost.is_empty() { "none" } else { &cost }));
    }
    capitalize_first(&text)
}

/// "As long as an opponent controls a creature with flying, this creature has
/// flying. The same is true for first strike and trample."
fn copy_static_ability_variants(spec: &ironsmith_core::CopyStaticAbilityVariants) -> String {
    use ironsmith_core::StaticAbilityVariantSelector as S;
    let names = spec
        .selectors
        .iter()
        .map(|selector| match selector {
            S::Any(id) => ironsmith_core::semantic_audit::humanize_variant(id),
            S::ProtectionFromColor => "protection from any color".to_string(),
        })
        .collect::<Vec<_>>();
    let Some((first, rest)) = names.split_first() else {
        return format!("This source has no static abilities of {}", objects(&spec.filter));
    };
    let holder = with_indefinite_article(&objects(&spec.filter));
    let first_quality = if first == "protection from any color" { "protection from a color" } else { first.as_str() };
    let mut text = format!("As long as {holder} has {first_quality}, this source has {first}");
    if !rest.is_empty() {
        let list = match rest.len() {
            1 => rest[0].clone(),
            2 => format!("{} and {}", rest[0], rest[1]),
            n => format!("{}, and {}", rest[..n - 1].join(", "), rest[n - 1]),
        };
        text.push_str(&format!(". The same is true for {list}"));
    }
    if !spec.exclude_source_id {
        text.push_str(". This object's own abilities count");
    }
    text
}

/// "a modular triggered ability", "a spell or ability an opponent
/// controlled": the event cause a replacement or history requires.
fn cause_phrase(cause: &ironsmith_core::CauseFilter) -> String {
    use ironsmith_core::CauseAbilityKindFilter as K;
    let kind = match &cause.ability_kind {
        None => None,
        Some(K::ModularTrigger) => Some("a modular triggered ability".to_string()),
        Some(K::ChampionTrigger) => Some("a champion triggered ability".to_string()),
        Some(K::Triggered) => Some("a triggered ability".to_string()),
        Some(K::Activated) => Some("an activated ability".to_string()),
        Some(K::ActivatedKeywordCost(keyword)) => Some(format!(
            "the cost of {}",
            with_indefinite_article(&format!("{} ability", ironsmith_core::semantic_audit::humanize_variant(keyword)))
        )),
    };
    match kind {
        Some(kind)
            if cause.cause_type.is_none() && cause.source_filter.is_none() && cause.controller_filter.is_none() =>
        {
            kind
        }
        Some(kind) => format!("{kind}, as {}", describe_history_cause(cause)),
        None => describe_history_cause(cause),
    }
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
        E::LifeGain { player: PlayerFilter::Any, from_own_spell_or_ability: true } => {
            "a spell or ability would cause its controller to gain life".to_string()
        }
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
        // "If you would pay life while your library has at least that many
        // cards in it" (Ashiok, Wicked Manipulator).
        E::LifePayment { player: who, require_library_capacity } => format!(
            "{} would pay life{}",
            player(who),
            if *require_library_capacity {
                format!(" while {} library has at least that many cards in it", possessive(who))
            } else {
                String::new()
            }
        ),
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
            // Copying counts times, not cards: "If you would copy a spell one
            // or more times, you copy it that many times plus one instead".
            let (noun, again) = match verb.strip_suffix(" a spell") {
                Some(base) => ("times", format!("{base} it")),
                None => ("cards", verb.to_string()),
            };
            let changed = amount(&format!("many {noun}"), noun);
            let changed = match modifier {
                M::Add(n) => changed.replace(&format!(" plus {n}"), &format!(" plus {}", count_words(*n))),
                _ => changed,
            };
            // "If you would scry a number of cards, scry that many cards plus
            // one instead" (Kenessos, Priest of Thassa).
            if verb == "scry" && matches!(performer, PlayerFilter::You) && !optional {
                return format!("If you would scry a number of cards, scry {changed} instead");
            }
            format!(
                "If {} would {verb} one or more {noun}, {subject} {}{again} {changed} instead",
                player(performer),
                if optional { "may " } else { "" },
            )
        }
        E::DamageRemovesAllCounters { object, counter_type } => format!(
            "If damage dealt to {} would result in all {} counters on it being removed, {}instead {} are removed",
            article(&objects(object)),
            describe_counter_type(*counter_type),
            may(optional),
            match modifier {
                M::AllButOne => "all but one of those counters".to_string(),
                _ => amount("many counters", "counters"),
            }
        ),
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

/// The subject of a characteristic-setting rule: a definite object reads
/// in the singular ("Enchanted land is ..."), a class of objects in the
/// plural ("Lands you control are ... in addition to their other types").
fn set_subject(filter: &ObjectFilter) -> (String, &'static str, &'static str) {
    let description = objects(filter);
    if ["enchanted ", "equipped ", "this ", "that ", "the ", "target "]
        .iter()
        .any(|determiner| description.starts_with(determiner))
    {
        return (capitalize_first(&description), "is", "its");
    }
    let plural = pluralize_relative_object_phrase(strip_indefinite_article(&description));
    (capitalize_first(&plural), "are", "their")
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
            .map(|grant| {
                let display = grant.display();
                // A play permission with no prose reading reads from its
                // typed origin, filter, beneficiary and use limit.
                if display.ends_with(" have play from zone") {
                    audit_play_from_grant(&grant).unwrap_or(display)
                } else {
                    display
                }
            })
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
            // "If a creature dealt damage by this creature this turn would
            // die" names one creature; the damage history is the damager's
            // controller at the time, so "a source you controlled" (Etching
            // of Kumano) carries no zone.
            let mut subject = with_indefinite_article(&objects(filter));
            if let Some(source) = damaged_by {
                subject.push_str(&format!(" dealt damage by {} this turn", damaged_by_subject(source)));
            }
            if let Some(damager) = damager_filter {
                let mut source = damager.clone();
                source.zone = None;
                // A damager narrowed by its controller alone is any source.
                let any_source = {
                    let mut bare = source.clone();
                    bare.controller = None;
                    bare == ObjectFilter::default()
                };
                let damager_noun = if any_source {
                    objects(&source).replacen("permanent", "source", 1)
                } else {
                    objects(&source)
                };
                subject.push_str(&format!(
                    " dealt damage this turn by {}",
                    with_indefinite_article(&damager_noun)
                        .replace(" you control", " you controlled")
                        .replace(" an opponent controls", " an opponent controlled")
                ));
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
        // "If you would put one or more counters on a creature you control
        // or on yourself, put that many plus one of each of those kinds of
        // counters on that permanent or player instead."
        P::ActorCountersAddition { filter, player_filter, actor, counter_type, additional, .. } => {
            let actor_text = player(actor);
            let counters = counter_type.map_or("counters".to_string(), |counter| {
                format!("{} counters", describe_counter_type(counter))
            });
            let player_target = player_filter.as_ref().map(|who| {
                if who == actor && actor_text == "you" { "yourself".to_string() } else { player(who) }
            });
            let recipients = match &player_target {
                Some(who) => format!("{} or on {who}", article(&objects(filter))),
                None => article(&objects(filter)),
            };
            let put = if actor_text == "you" { "put".to_string() } else { "puts".to_string() };
            let plus = i32::try_from(*additional)
                .ok()
                .and_then(number_word)
                .unwrap_or_else(|| additional.to_string());
            let kinds = if counter_type.is_some() { counters.clone() } else { "each of those kinds of counters".to_string() };
            let of = if counter_type.is_some() { format!("{plus} {kinds}") } else { format!("{plus} of {kinds}") };
            format!(
                "If {actor_text} would put one or more {counters} on {recipients}, {} that many plus {of} on that {} instead",
                if actor_text == "you" { "put".to_string() } else { format!("{actor_text} {put}") },
                if player_target.is_some() { "permanent or player" } else { "permanent" }
            )
        }
        // "You may pay {0} rather than pay the echo cost for permanents you
        // control."
        P::EchoCostAlternative { filter, replacement_mana_cost, .. } => format!(
            "You may pay {} rather than pay the echo cost for {}",
            replacement_mana_cost.to_oracle(),
            pluralize_relative_object_phrase(strip_indefinite_article(&objects(filter)))
        ),
        P::ChooseBasicLandTypesAsEnters { count, .. } => format!(
            "As this permanent enters, choose {} basic land types",
            count_words(*count)
        ),
        P::CountersRemainAcrossZoneChanges { excluded_destinations, .. } => {
            // "Counters remain on Skullbriar as it moves to any zone other
            // than a player's hand or library."
            if excluded_destinations.is_empty() {
                "Counters on this permanent remain on it as it changes zones".to_string()
            } else {
                let zones = excluded_destinations
                    .iter()
                    .map(|zone| match zone {
                        Zone::Command => "command zone".to_string(),
                        other => format!("{other:?}").to_ascii_lowercase(),
                    })
                    .collect::<Vec<_>>();
                format!(
                    "Counters remain on this permanent as it moves to any zone other than a player's {}",
                    zones.join(" or ")
                )
            }
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
            // The filter's own counter requirement already reads "with cage
            // counters on them" (Mairsil, the Pretender).
            if let Some(counter) = spec.counter
                && spec.filter.with_counter.is_none()
            {
                text.push_str(&format!(" with a {} counter on them", describe_counter_type(counter)));
            }
            // Excluding this object's name already excludes this object.
            if spec.exclude_source_name && spec.counter.is_none() && !spec.only_loyalty {
                text = text.replacen(" abilities of all ", " abilities of ", 1);
                text.push_str(" that don't have the same name as this source");
            } else if spec.exclude_source_name {
                text.push_str(" except those with this object's name");
            }
            if spec.exclude_mana_abilities {
                text.push_str(" except mana abilities");
            }
            // A card in exile or a graveyard is never this battlefield object.
            if spec.exclude_source_id
                && !spec.exclude_source_name
                && spec.filter.controller != Some(PlayerFilter::Opponent)
                && matches!(spec.filter.zone, None | Some(Zone::Battlefield))
            {
                text.push_str(" other than this object");
            }
            if spec.force_once_each_turn {
                text.push_str(". Each of those abilities can be activated only once each turn");
            }
            text
        }
        P::CopyStaticAbilityVariants(spec) => copy_static_ability_variants(spec),
        P::CopyTriggeredAbilities(spec) => format!(
            "This has all triggered abilities of all {}{}",
            pluralize_relative_object_phrase(strip_indefinite_article(&objects(&spec.filter))),
            if spec.exclude_source_name { " except those with this object's name" } else { "" }
        ),
        P::CostIncreaseLife { filter, amount, .. } => format!(
            "{} cost an additional {amount} life to cast",
            capitalize_first(&objects(filter))
        ),
        // Only a permanent's controller activates its abilities (CR 602.2),
        // so "you control" makes "you" the activator.
        P::ActivateAbilitiesAsThoughHaste { filter, .. }
            if objects(filter).ends_with(" you control")
                && pluralize_relative_object_phrase(&strip_leading_article(&objects(filter)))
                    .starts_with("creatures ") =>
        {
            format!(
                "You may activate abilities of {} as though those creatures had haste",
                pluralize_relative_object_phrase(&strip_leading_article(&objects(filter)))
            )
        }
        P::ActivateAbilitiesAsThoughHaste { filter, .. } => {
            let (subject, is, _) = set_subject(filter);
            let pronoun = if is == "is" { "it" } else { "those creatures" };
            let subject = lowercase_first(&subject);
            format!("You may activate abilities of {subject} as though {pronoun} had haste")
        }
        P::LoyaltyAbilitiesAnyTime { filter } => format!(
            "You may activate loyalty abilities of {} on any player's turn any time you could cast an instant",
            objects(filter)
        ),
        P::ThisSpellXMaximum { maximum, definition, .. } => match definition {
            Some(value) => format!("X is defined as {}", describe_value(value)),
            None => format!("X can't be greater than {}", describe_value(maximum)),
        },
        P::ThisSpellXMinimum { minimum, .. } => match minimum {
            crate::effect::Value::Fixed(1) => "X can't be 0".to_string(),
            other => format!("X can't be less than {}", describe_value(other)),
        },
        P::DieRollResultAdjustment(adjustment) => die_roll_adjustment(adjustment),
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
            "{} {} protection from {}",
            capitalize_first(&player(who)),
            if matches!(who, PlayerFilter::You) { "have" } else { "has" },
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
            match choice {
                ironsmith_core::static_ability_model::FlashCastChoice::XAtMost(maximum) => {
                    format!("X is {maximum} or less")
                }
                ironsmith_core::static_ability_model::FlashCastChoice::OptionalCostPaid(label) => {
                    describe_condition(&crate::ConditionExpr::ThisSpellPaidLabel(label.clone()))
                }
            }
        ),
        P::AddChosenCreatureType { filter, replace_existing, .. } => {
            let (subject, is, its) = set_subject(filter);
            format!(
                "{subject} {is} the chosen creature type{}",
                if *replace_existing { String::new() } else { format!(" in addition to {its} other types") }
            )
        }
        P::AddChosenBasicLandType { filter, .. } => {
            let (subject, is, its) = set_subject(filter);
            format!("{subject} {is} the chosen basic land type in addition to {its} other types")
        }
        P::SourceHasAbilityStickerAbilitiesOf { filter, .. } => format!(
            "This has the abilities of ability stickers on {}",
            objects(filter)
        ),
        P::AddChosenColor { filter, .. } => {
            let (subject, is, its) = set_subject(filter);
            format!("{subject} {is} the chosen color in addition to {its} other colors")
        }
        P::SetChosenColor { filter, .. } => {
            let (subject, is, _) = set_subject(filter);
            format!("{subject} {is} the chosen color")
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
                Some(trigger) => {
                    let gerund = trigger_gerund(trigger);
                    // "Artifacts and creatures entering don't cause abilities
                    // to trigger": the event's objects read in the plural.
                    let plural = strip_indefinite_article(&gerund)
                        .strip_suffix(" entering")
                        .filter(|nouns| gerund != *nouns && nouns.split(" or ").all(|noun| !noun.contains(' ')))
                        .map(|nouns| {
                            nouns.split(" or ").map(pluralize_noun_phrase).collect::<Vec<_>>().join(" and ")
                        });
                    match plural {
                        Some(nouns) => format!("{} entering don't cause {abilities} to trigger", capitalize_first(&nouns)),
                        None => format!("{} doesn't cause {abilities} to trigger", capitalize_first(&gerund)),
                    }
                }
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
        P::ChoosePlayerAsEnters { filter, count, .. } => match (count, filter) {
            (1, _) => format!("As this permanent enters, choose {}", player(filter)),
            (n, PlayerFilter::Any) => format!("As this permanent enters, choose {} players", count_words(*n)),
            (n, PlayerFilter::Opponent) => format!("As this permanent enters, choose {} opponents", count_words(*n)),
            (n, _) => format!("As this permanent enters, choose {n} {}", player(filter)),
        },
        P::RevealFromHandAsEnters { filter, count, optional, .. } => {
            // "you may reveal any number of other artifact cards", "a Dragon
            // card": the count reads as words when it is a plain range.
            let mut hand_filter = filter.clone();
            if hand_filter.zone == Some(Zone::Hand) {
                hand_filter.zone = None;
            }
            let described = objects(&hand_filter);
            let noun = strip_indefinite_article(&described).to_string();
            let noun = match noun.strip_prefix("another ") {
                Some(rest) if count.max != Some(1) => format!("other {rest}"),
                _ => noun,
            };
            let simple = !count.dynamic_x && !count.up_to_x && !count.random;
            let quantity = match (count.min, count.max) {
                (0, None) if simple => format!(
                    "any number of {}",
                    pluralize_relative_object_phrase(&noun)
                ),
                (1, Some(1)) if simple => article(&described),
                (0, Some(1)) if simple => format!("up to one {noun}"),
                (min, Some(max)) if simple && min == max => format!(
                    "{} {}",
                    count_words(min as u32),
                    pluralize_relative_object_phrase(&noun)
                ),
                _ => format!("{} {described}", sr(count)),
            };
            format!(
                "As this permanent enters, {}reveal {quantity} from your hand",
                may(*optional)
            )
        }
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
            // "choose a card name"; with an opponent, both choose.
            text.push_str(if *opponent_also_chooses {
                "you and an opponent each choose a card name"
            } else if *reveal_opponents_hands {
                "you choose a card name"
            } else {
                "choose a card name"
            });
            if *require_nonland_from_revealed_opponents {
                text.push_str(" of a nonland card revealed this way");
            }
            if *exclude_basic_land_names {
                text.push_str(" other than a basic land card name");
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
                    let mut text = format!("a {}/{}", option.power, option.toughness);
                    if let Some(types) = &option.card_types {
                        let names = types
                            .iter()
                            .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
                            .collect::<Vec<_>>();
                        text.push_str(&format!(" {}", names.join(" ")));
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
            // The imperative "choose" is the controller's choice.
            let body = effects(program.flattened_default_effects());
            let body = match body.strip_prefix("you choose ") {
                Some(rest) => format!("choose {rest}"),
                None => body,
            };
            // An "as this transforms into <this face>" program (CR 712.x)
            // runs as the permanent becomes the face that carries it, which
            // the card-level pass names ("transforms into Shinryu, ...").
            if transforms_into.is_some() && !*turns_face_up_only && !*also_turns_face_up {
                return Some(format!("As this permanent transforms into this face, {body}"));
            }
            let mut text = format!("As this permanent {event}, {body}");
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
        // "Double all damage Wolverine would deal", "Double all damage
        // equipped creature would deal": one specific source doubling all
        // of its damage to any recipient.
        P::DoubleDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, factor: 2, combat_only: false, noncombat_only: false, ..
        } if {
            let source = damage_source_objects(source_filter);
            let head = if source.starts_with("this ") { source.clone() } else { "equipped creature".to_string() };
            (source.starts_with("this ") || source == "a creature with this Equipment attached to it")
                && matches!(
                    damage_scope(source_filter, target_player_filter, target_object_filter, "")
                        .strip_prefix(head.as_str()),
                    Some(
                        " would deal damage"
                            | " would deal damage to a player or a permanent"
                            | " would deal damage to a permanent or player"
                    )
                )
        } => {
            let source = damage_source_objects(source_filter);
            let source = if source.starts_with("this ") { source } else { "equipped creature".to_string() };
            format!("Double all damage {source} would deal")
        }
        P::DoubleDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, factor, combat_only, noncombat_only, ..
        } => format!(
            "If {}, it deals {} instead",
            damage_scope(source_filter, target_player_filter, target_object_filter,
                if *combat_only { "combat " } else if *noncombat_only { "noncombat " } else { "" }),
            match *factor { 2 => "double that damage".to_string(), 3 => "triple that damage".to_string(), n => format!("{} that much damage", multiple(n)) }
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
            let factor = if *halve { "half" } else { "twice" };
            // "If you would get one or more {E}, you get twice that many {E}
            // instead" (CR 107.14: energy counters are {E}).
            if actor.is_none()
                && !*effect_only
                && matches!(counter_type, Some(ironsmith_core::CounterType::Energy))
                && matches!(player_filter, Some(PlayerFilter::You))
                && !*includes_permanents
            {
                return Some(format!("If you would get one or more {{E}}, you get {factor} that many {{E}} instead"));
            }
            // With no actor the replacement watches the counters, whoever
            // puts them: "If one or more +1/+1 counters would be put on a
            // creature you control, twice that many +1/+1 counters are put
            // on it instead."
            if actor.is_none() && !*effect_only {
                let noun = counter_type.map_or("of each of those kinds of counters".to_string(), |c| {
                    format!("{} counters", describe_counter_type(c))
                });
                return Some(format!(
                    "If {kind} counters would be put on {}, {factor} that many {noun} are put on {} instead",
                    recipients.join(" or "),
                    if player_filter.is_some() && !*includes_permanents { "that player" } else { "it" }
                ));
            }
            format!(
                "If {putter} would put {kind} counters on {}, it puts {factor} that many of those counters on that {} instead",
                recipients.join(" or "),
                if player_filter.is_some() && !*includes_permanents { "player" } else { "permanent" }
            )
        }
        // "that many -1/-1 counters minus one are put on it instead" (Vizier
        // of Remedies): a negative addition removes from the placed count.
        P::AddCountersPlacementReplacement { filter, player_filter: None, counter_type: Some(counter_type), additional, cause: None, .. }
            if *additional < 0 =>
        {
            let kind = describe_counter_type(*counter_type);
            format!(
                "If one or more {kind} counters would be put on {}, that many {kind} counters minus {} are put on it instead",
                article(&objects(filter)),
                count_words(additional.unsigned_abs() as u32)
            )
        }
        P::AddCountersPlacementReplacement { filter, player_filter, counter_type, additional, cause, .. } => format!(
            "If one or more {}counters would be put on {}{}{}, that many plus {} {} put on that permanent instead",
            counter_type.map_or(String::new(), |c| format!("{} ", describe_counter_type(c))),
            article(&objects(filter)),
            player_filter.as_ref().map_or(String::new(), |p| format!(" or {}", player(p))),
            cause.as_ref().map_or(String::new(), |c| format!(" by {}", cause_phrase(c))),
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
        // Doubling Season, Parallel Lives and Anointed Procession print the
        // active form; the newer passive form (Adrix and Nev) is the same
        // replacement.
        P::DoubleTokenCreationReplacement { controller, .. } => format!(
            "If an effect would create one or more tokens under {} control, it creates twice that many of those tokens instead",
            possessive(controller)
        ),
        P::MultiplyTokenCreationReplacement { controller, token_filter, factor, .. } => format!(
            "If one or more {} would be created under {} control, {} that many of those tokens are created instead",
            token_filter.as_ref().map_or("tokens".to_string(), |f| pluralize_relative_object_phrase(&strip_indefinite_article(&objects(f)))),
            possessive(controller),
            multiple(*factor)
        ),
        P::AddTokenCreationReplacement { controller, token_filter, additional_token, additional, per_created, .. } => {
            use ironsmith_core::AdditionalTokenKind as Kind;
            let token = match additional_token {
                Kind::Treasure => "Treasure token",
                Kind::Food => "Food token",
                Kind::Clue => "Clue token",
                Kind::Squirrel => "1/1 green Squirrel creature token",
            };
            let added = match (*per_created, *additional) {
                (true, 1) => format!("that many {token}s"),
                (true, n) => format!("{} times that many {token}s", count_words(n.max(0) as u32)),
                (false, 1) => with_indefinite_article(token),
                (false, n) => format!("{} {token}s", count_words(n.max(0) as u32)),
            };
            // "If you would create one or more Treasure tokens, instead create
            // those tokens plus an additional Treasure token" (Xorn, Academy
            // Manufactor's printed Treasure form).
            if matches!(additional_token, Kind::Treasure)
                && !*per_created
                && *additional == 1
                && matches!(controller, PlayerFilter::You)
            {
                return Some(format!(
                    "If you would create one or more {}, instead create those tokens plus an additional {token}",
                    pluralize_relative_object_phrase(strip_indefinite_article(&objects(token_filter)))
                ));
            }
            format!(
                "If one or more {} would be created under {} control, those tokens plus {added} are created instead",
                pluralize_relative_object_phrase(strip_indefinite_article(&objects(token_filter))),
                possessive(controller)
            )
        }
        // "If a creature you control would explore, instead you scry 1, then
        // that creature explores."
        P::KeywordActionReplacement { action, source_filter, performer_filter: None, replacement_effects, optional, .. } => {
            let effects_text = effects(replacement_effects);
            let effects_text = effects_text.replacen(". It ", ", then it ", 1);
            // "If you would proliferate, proliferate twice instead": an
            // untyped source you control is your own player action.
            if *source_filter
                == (ObjectFilter {
                    controller: Some(PlayerFilter::You),
                    ..ObjectFilter::default()
                })
            {
                return Some(format!(
                    "If you would {}, {}{effects_text} instead",
                    sr(action).to_ascii_lowercase(),
                    may(*optional),
                ));
            }
            format!(
                "If {} would {}, {}instead {effects_text}",
                article(&objects(source_filter)),
                sr(action).to_ascii_lowercase(),
                may(*optional),
            )
        }
        // The matcher checks the performing player when one is named, and
        // otherwise the object performing the action ("If a Rigger you
        // control would assemble a Contraption"); an unrestricted source
        // with a performer is any instance of that player's action.
        P::KeywordActionReplacement { action, source_filter, performer_filter, replacement_effects, optional, .. } => {
            let subject = match performer_filter {
                Some(performer) if *source_filter == ObjectFilter::default() => player(performer),
                Some(performer) => format!("{} with {}", player(performer), article(&objects(source_filter))),
                None => article(&objects(source_filter)),
            };
            format!(
                "If {subject} would {}, {}{} instead",
                action.infinitive(),
                may(*optional),
                effects(replacement_effects)
            )
        }
        P::ConditionalDrawReplacement { condition, replacement_effects, optional, except_first_of_draw_step, .. } => {
            // A timing window reads as the window ("during your draw step").
            let condition = describe_condition(condition);
            let when = match condition.strip_prefix("timing restriction: ") {
                Some(window) => format!(" {window}"),
                None => format!(" while {condition}"),
            };
            format!(
                "If you would draw a card{}{when}, {}{} instead",
                except_first(*except_first_of_draw_step),
                may(*optional),
                effects(replacement_effects)
            )
        }
        P::CreateOneOfEachTokenReplacement { kinds, .. } => format!(
            "If one or more tokens would be created, create one of each of {} instead",
            sr(kinds)
        ),
        // The drawer skips the draw and you draw a card instead.
        P::RedirectDrawReplacement { drawer, except_first_of_draw_step, .. } => format!(
            "If {} would draw a card{}, instead that player skips that draw and you draw a card",
            player(drawer),
            except_first_for(*except_first_of_draw_step, drawer)
        ),
        P::DrawReplacementWithEffects { drawer, except_first_of_draw_step, replacement_effects, cycling_source, .. } => format!(
            "If {} would draw a card{}{}, {} instead",
            player(drawer),
            except_first_for(*except_first_of_draw_step, drawer),
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
        // "If you would draw a card, instead reveal the top three cards of
        // your library. Put all creature cards revealed this way into your
        // hand and the rest on the bottom of your library in any order."
        // (Sages of the Anima)
        P::DrawReplacementRevealTopMatchingToHandRestBottom { count, filter, order, .. } => format!(
            "If you would draw a card, instead reveal the top {} cards of your library. Put all {} revealed this way into your hand and the rest on the bottom of your library in {} order",
            count_words(*count),
            {
                let noun = objects(filter);
                let noun = strip_indefinite_article(&noun);
                if noun.contains("card") { pluralize_relative_object_phrase(noun) } else { format!("{noun} cards") }
            },
            match order {
                ironsmith_core::LibraryBottomOrder::Random => "a random",
                ironsmith_core::LibraryBottomOrder::ChooserChooses => "any",
            }
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
        // "If a land would produce two or more mana, it produces {C} instead
        // of any other type and amount" (Damping Sphere): the replacement
        // mana is all that is produced.
        P::ManaProductionReplacement { source_filter, minimum_amount, replacement_mana, .. } => format!(
            "If {} would produce {} or more mana, it produces {} instead of any other type and amount",
            with_indefinite_article(&objects(source_filter)),
            count_words(*minimum_amount),
            replacement_mana.iter().map(|symbol| describe_mana_symbol(*symbol)).collect::<Vec<_>>().join("")
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
            // Oracle: "prevent that damage. Put a +1/+1 counter on this
            // creature for each 1 damage prevented this way" (Stormwild Capridor).
            "If {}damage{} would be dealt to this permanent, prevent that damage. Put a {} counter on this permanent for each 1 damage prevented this way",
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
        // "You may cast creature spells from your graveyard using their sneak
        // abilities." (Ninja Teen)
        P::AlternativeCastFromZoneForFilter { filter, zone, method } => {
            let from = match zone {
                Zone::Graveyard => "your graveyard".to_string(),
                Zone::Hand => "your hand".to_string(),
                Zone::Library => "your library".to_string(),
                Zone::Exile => "exile".to_string(),
                other => sr(other),
            };
            let described = objects(filter);
            let noun = strip_indefinite_article(&described);
            let noun = noun.strip_suffix(" card").unwrap_or(noun);
            format!(
                "You may cast {noun} spells from {from} using their {} abilities",
                sr(method).to_ascii_lowercase()
            )
        }
        P::AddLifeGainReplacement { player: who, additional, .. } => format!(
            "If {} would gain life, they gain that much plus {additional} instead",
            player(who)
        ),
        P::TokenCreationTemplates { controller, token_filter, templates, mode, choose_one, optional, .. } => {
            // AppendOnce adds one of each template to the event; the other
            // modes add (AppendForEach) or substitute (ReplaceEach) one per
            // matching token created.
            use ironsmith_core::TokenCreationTemplateMode as Mode;
            let created = pluralize_relative_object_phrase(strip_indefinite_article(&objects(token_filter)));
            let per_token = !matches!(mode, Mode::AppendOnce);
            let added = templates
                .iter()
                .map(|template| {
                    let text = effects(std::slice::from_ref(template));
                    let text = text.strip_prefix("create ").unwrap_or(&text).to_string();
                    if !per_token {
                        return text;
                    }
                    let plural = text
                        .strip_prefix("a ")
                        .or_else(|| text.strip_prefix("an "))
                        .unwrap_or(&text)
                        .replacen("token that's a copy of ", "tokens that are copies of ", 1);
                    let plural = if plural.contains("tokens") { plural } else { plural.replacen(" token", " tokens", 1) };
                    format!("that many {plural}")
                })
                .collect::<Vec<_>>();
            let added = if *choose_one { format!("your choice of {}", or_list(&added)) } else { added.join(" and ") };
            let result = match mode {
                Mode::ReplaceEach => added,
                Mode::AppendOnce | Mode::AppendForEach => format!("those tokens plus {added}"),
            };
            if *optional {
                format!(
                    "If one or more {created} would be created under {} control, you may instead create {result}",
                    possessive(controller)
                )
            } else {
                format!(
                    "If one or more {created} would be created under {} control, {result} are created instead",
                    possessive(controller)
                )
            }
        }
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
        // "Spend only mana produced by basic lands to cast this spell."
        P::SpellManaSpendingRestriction(ironsmith_core::mana::ManaSpendingRestriction::ProducedBy(
            producer,
        )) => format!("Spend only mana produced by {} to cast this spell", producer.description()),
        // "Spend only colored mana on X. No more than one mana of each color
        // may be spent this way."
        P::SpellManaSpendingRestriction(ironsmith_core::mana::ManaSpendingRestriction::OnX {
            colors,
            maximum_per_color: Some(maximum),
        }) if !colors.is_empty() => {
            use ironsmith_core::color::Color;
            let mana = if Color::ALL.into_iter().all(|color| colors.contains(color)) {
                "colored".to_string()
            } else {
                or_list(
                    &Color::ALL
                        .into_iter()
                        .filter(|color| colors.contains(*color))
                        .map(|color| color.name().to_string())
                        .collect::<Vec<_>>(),
                )
            };
            format!(
                "Spend only {mana} mana on X. No more than {} mana of each color may be spent this way",
                count_words(*maximum)
            )
        }
        P::SpellManaSpendingRestriction(restriction) => {
            format!("Mana spent on this spell is restricted: {}", sr(restriction))
        }
        P::ManaProductionRewrite { rule, .. } => mana_output_rewrite(rule),
        P::ExtraDieIgnoreLowest { player: who, additional } => format!(
            "If {} would roll one or more dice, instead {} that many dice plus {} and ignore the lowest roll",
            player(who),
            if matches!(who, PlayerFilter::You) { "roll" } else { "they roll" },
            count_words(*additional)
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
            "If {} would roll one or more planar dice, instead {} that many planar dice plus one and ignore one",
            player(who),
            if matches!(who, PlayerFilter::You) { "roll" } else { "they roll" }
        ),
        // The pair is an opaque identity of the exiling ability (a digest),
        // which names no other card set than "exiled with this".
        P::LookAtSourceExiledCards { pair, .. } => format!(
            "You may look at cards exiled with {}",
            if pair.is_some() { "this source" } else { "this" }
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
        // The increase applies to every devotion count (CR 700.5).
        P::DevotionIncrease { amount } => format!(
            "Your devotion to each color and each combination of colors is increased by {}",
            i32::try_from(*amount).ok().and_then(number_word).unwrap_or_else(|| amount.to_string())
        ),
        P::MorphCostIncrease { generic_increase } => {
            format!("All morph costs cost {{{generic_increase}}} more")
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
        // "If a spell or ability you control would counter a spell, instead
        // exile that spell and you may play that card without paying its
        // mana cost."
        P::ControllerCounterExilePermission { permission, .. } => format!(
            "If a spell or ability you control would counter {}, instead exile that spell and you may {} that card without paying its mana cost",
            match permission.gate {
                ironsmith_core::effect::CounterExileGate::AnySpell => "a spell",
                ironsmith_core::effect::CounterExileGate::PermanentSpell => "a permanent spell",
            },
            if permission.allow_land { "play" } else { "cast" }
        ),
        P::AttackingPlayerChoosesBlocks { .. } => {
            "The attacking player chooses how creatures block".into()
        }
        P::AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn { cost, .. } => format!(
            "The controller of the permanent this is attached to may pay {} to ignore this effect until end of turn",
            cost.display()
        ),
        P::FirstChosenLandTypeBecomesSecond { filter, .. } => format!(
            "{} of the first chosen type are the second chosen type",
            capitalize_first(&pluralize_relative_object_phrase(strip_indefinite_article(&objects(filter))))
        ),
        P::RevealCardsDrawn { player: who } => {
            format!("{} reveals each card they draw", capitalize_first(&player(who)))
        }
        P::DamageBetweenChosenPlayers { factor, .. } => format!(
            "If a source controlled by one of the chosen players would deal damage to the other chosen player or a permanent that player controls, that source deals {} to that player or permanent instead",
            match *factor { 2 => "double that damage".to_string(), 3 => "triple that damage".to_string(), n => format!("{} that much damage", multiple(n)) }
        ),
        P::AddSubtypesAndGrantAbilities { filter, subtypes, abilities, .. } => format!(
            "Each {} is {} in addition to its other types and has {}",
            strip_indefinite_article(&objects(filter)),
            with_indefinite_article(
                &subtypes.iter().map(|subtype| format!("{subtype:?}")).collect::<Vec<_>>().join(" ")
            ),
            abilities
                .iter()
                .map(ability_model)
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
            // A conditional Flash keyword is a casting permission (CR
            // 702.8a): "You may cast this spell as though it had flash if
            // you control a green or blue permanent".
            if ability.id == Some(crate::static_abilities::StaticAbilityId::Flash) {
                let rest = condition.strip_prefix("as long as ").unwrap_or(&condition);
                return Some(format!("You may cast this spell as though it had flash if {rest}"));
            }
            match condition.strip_prefix("as long as ").or(Some(condition.as_str())) {
                // "During your turn, prevent all damage ...".
                Some("it's your turn") if sentence && !trimmed.contains(" as long as ") => format!(
                    "During your turn, {}",
                    lowercase_first(trimmed.trim_end_matches('.'))
                ),
                Some(rest) if sentence && !trimmed.contains(" as long as ") => format!(
                    "As long as {rest}, {}",
                    lowercase_first(trimmed.trim_end_matches('.'))
                ),
                _ => format!("{inner} as long as {condition}"),
            }
        }
        P::ForetellSpecialActionModifier { generic_reduction, any_players_turn, first_each_turn, plot } => format!(
            "{} {{{generic_reduction}}} less{}",
            match (*plot, *first_each_turn) {
                (true, false) => "Plotting cards from your hand costs",
                (true, true) => "The first card you plot each turn costs",
                (false, true) => "The first card you foretell each turn costs",
                (false, false) => "Foretelling costs",
            },
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
            Some(limit) => format!("A deck can have up to {limit} copies of this card"),
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
