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

/// The gate `parse_first_draw_each_turn_replacement_line` builds: no card
/// drawn this turn yet, or (with the draw-step exception) on your turn at
/// most the excepted draw-step card.
fn is_first_draw_each_turn_gate(
    condition: &crate::effect::Condition,
    except_first_of_draw_step: bool,
) -> bool {
    use crate::effect::Condition as C;
    let drawn_compare = |condition: &C, expected: crate::effect::ValueComparisonOperator, amount: Value| {
        matches!(condition, C::ValueComparison { left, operator, right }
            if left.unhinted() == &Value::MaxCardsDrawnThisTurn(PlayerFilter::You)
                && *operator == expected
                && right.unhinted() == &amount)
    };
    let none_drawn =
        |condition: &C| drawn_compare(condition, crate::effect::ValueComparisonOperator::Equal, Value::Fixed(0));
    if !except_first_of_draw_step {
        return none_drawn(condition);
    }
    let C::Or(first, second) = condition else {
        return false;
    };
    let C::And(turn, at_most_one) = second.as_ref() else {
        return false;
    };
    none_drawn(first)
        && matches!(turn.as_ref(), C::YourTurn)
        && drawn_compare(
            at_most_one,
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            Value::Fixed(1),
        )
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
    // "a Giant source you control" (Calamity Bearer): a subtype-only dealer
    // in no zone is any source of that subtype.
    let description = if source.zone.is_none()
        && source.card_types.is_empty()
        && !source.subtypes.is_empty()
        && !description.contains("source")
        && let Some(at) = description.find(" you control")
    {
        format!("{} source{}", &description[..at], &description[at..])
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
    } else if matches!(player_filter, Some(PlayerFilter::Any))
        && object_filter.as_ref().is_some_and(|filter| *filter == ObjectFilter::default())
    {
        // Any player and any object, with no zone: every damage recipient,
        // so no recipient is named (Fire Servant).
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
            let mut ability = crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(ability);
            // Mana-output activated abilities store their executable output
            // outside the ordinary effect list. Materialize that typed output
            // only in the rendering copy so loyalty mana grants retain it.
            if let AbilityKind::Activated(activated) = &mut ability.kind
                && activated.effects.is_empty()
                && activated.mana_usage_restrictions.is_empty()
                && let Some(mana) = &activated.mana_output
            {
                activated.effects = crate::resolution::ResolutionProgram::from_effects(
                    vec![Effect::add_mana(mana.clone())],
                );
                activated.mana_output = None;
            }
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

/// Render a granted ability with its receiving object's proven type. This
/// context ends at the ability boundary; the granting source keeps its type.
fn ability_model_for_recipient(ability: &AbilityModel, recipient: &ObjectFilter) -> String {
    let runtime = crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(ability);
    let subject = super::render_effects::granted_ability_self_subject_for_filter(recipient);
    let text = super::render_effects::describe_inline_ability_with_self_subject(&runtime, subject);
    let text = super::render_effects::normalize_ability_self_reference_surface(&text, subject);
    let text = text.trim().trim_end_matches('.');
    if merge_passes::is_keyword_phrase(text) {
        text.to_ascii_lowercase()
    } else if matches!(&ability.kind, ironsmith_core::AbilityKind::Static(_)) {
        lowercase_first(text)
    } else {
        format!("\"{text}.\"")
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
    // "except it isn't legendary, is an artifact in addition to its other
    // types, and has myriad" (Auton Soldier): with type additions present,
    // the removed supertype is printed first.
    let removed_supertypes_lead = !types.is_empty();
    if removed_supertypes_lead {
        for supertype in &spec.removed_supertypes {
            exceptions.push(format!("it isn't {}", format!("{supertype:?}").to_ascii_lowercase()));
        }
    }
    if !types.is_empty() {
        // Supertypes are adjectives: "it's legendary and snow in addition to
        // its other types" (Moritte of the Frost), "it's legendary" (Sakashima).
        let supertypes_only = spec.added_subtypes.is_empty() && spec.added_card_types.is_empty();
        exceptions.push(format!(
            "it's {} {}",
            if supertypes_only { types.join(" and ") } else { with_indefinite_article(&types.join(" ")) },
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
    if !removed_supertypes_lead {
        for supertype in &spec.removed_supertypes {
            exceptions.push(format!("it isn't {}", format!("{supertype:?}").to_ascii_lowercase()));
        }
    }
    // "if it's a creature, it enters with two additional +1/+1 counters on it
    // and has changeling" (Moritte of the Frost): counters and abilities that
    // share one copied-kind gate state it once.
    let shared_copy_gate = spec.additional_counters_source_filter.as_ref().filter(|filter| {
        spec.added_abilities_source_filter.as_ref() == Some(*filter)
            && !spec.added_abilities.is_empty()
            && !spec.additional_counters.is_empty()
            && spec.additional_x_counters.is_empty()
    });
    let mut gated_abilities: Option<String> = None;
    if !spec.added_abilities.is_empty() {
        // The entering object becomes a copy of something matching the
        // filter, so an added rule's self-reference is that object's type
        // ("you may exile this enchantment", Estrid's Invocation).
        let recipient = (spec.affected_filter.is_none()
            && !spec.copy_source_self
            && !spec.copy_source_enchanted
            && !matches!(
                super::render_effects::granted_ability_self_subject_for_filter(&spec.filter),
                "this creature" | "this permanent"
            ))
        .then_some(&spec.filter);
        let abilities = spec
            .added_abilities
            .iter()
            .map(|ability| match recipient {
                Some(filter) if !matches!(&ability.kind, ironsmith_core::AbilityKind::Static(_)) => {
                    ability_model_for_recipient(ability, filter)
                }
                _ => ability_model(ability),
            })
            .collect::<Vec<_>>();
        let joined = abilities.join(" and ");
        if shared_copy_gate.is_some() {
            gated_abilities = Some(joined);
        } else {
            exceptions.push(format!("it has {joined}"));
        }
    }
    // "except it enters with a shield counter on it if you control that
    // creature" (Undercover Operative): the counters' copied-object gate.
    let copies_gate = spec
        .additional_counters_source_filter
        .as_ref()
        .map_or(String::new(), |filter| format!(" if it copies {}", objects(filter)));
    if let Some(gate) = shared_copy_gate {
        let counters = spec
            .additional_counters
            .iter()
            .map(|(counter, amount)| {
                format!(
                    "{} additional {} counter{}",
                    if *amount == 1 { "an".to_string() } else { count_words(*amount) },
                    describe_counter_type(*counter),
                    if *amount == 1 { "" } else { "s" }
                )
            })
            .collect::<Vec<_>>();
        exceptions.push(format!(
            "if it's {}, it enters with {} on it and has {}",
            with_indefinite_article(&objects(gate)),
            counters.join(" and "),
            gated_abilities.take().unwrap_or_default()
        ));
    }
    for (counter, amount) in spec
        .additional_counters
        .iter()
        .filter(|_| shared_copy_gate.is_none())
    {
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
            Followup::RemoveManaCost => exceptions.push("it has no mana cost".into()),
            Followup::ConditionalCharacteristics{entering_filter,remove_mana_cost,set_base_power_toughness} => {
                let mut parts=Vec::new();
                if *remove_mana_cost {parts.push("it has no mana cost".to_string());}
                if let Some((power,toughness))=set_base_power_toughness {parts.push(format!("it has base power and toughness {power}/{toughness}"));}
                exceptions.push(format!("{} if it's {}",parts.join(" and "),objects(entering_filter)));
            }
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
    if shared_copy_gate.is_none() && let Some(filter) = &spec.added_abilities_source_filter { rest.push(format!("abilities only if it copies {}", objects(filter))); }
    let mut sentences = Vec::new();
    for followup in &spec.copy_followups {
        sentences.push(match followup {
            Followup::RetainOwnColors | Followup::RetainEmbalmCharacteristics
            | Followup::RemoveManaCost | Followup::ConditionalCharacteristics{..} => continue,
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
            Group::CreatureOfColorsAlsoAttacks(colors) => {
                let names = ironsmith_core::Color::ALL.into_iter().filter(|c| colors.contains(*c)).map(|c| c.name()).collect::<Vec<_>>().join(" or ");
                ("attack", format!("a {names} creature also attacks"))
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
            // The executor offers only permanents you control
            // (`non_mana_attack_cost_program`), so the reading names that
            // restriction even when the stored filter leaves it implicit.
            Cost::ReturnPermanentsToOwnersHand { filter, count } if *count == 1 => {
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter = filter.you_control();
                }
                format!("you return {} to its owner's hand", article(&objects(&filter)))
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

/// Alternative price grants carry a separate cast-origin restriction and a
/// shared use limit. Both must survive audit rendering.
/// "Once each turn, you may pay {0} rather than pay the mana cost for a spell
/// you cast with mana value X or less, where X is the number of time counters
/// on this enchantment" (As Foretold): a use-limited alternative price for any
/// spell you cast, bounded by a dynamic mana value.
fn audit_limited_dynamic_mana_value_price_grant(grant: &crate::grant::GrantSpec) -> Option<String> {
    let ironsmith_core::Grantable::AlternativePrice { costs, origin: None } = &grant.grantable else {
        return None;
    };
    if costs.is_empty()
        || costs.iter().any(|cost| cost.mana_cost_ref().is_none())
        || grant.beneficiary != PlayerFilter::You
        || !grant.additional_zones.is_empty() || grant.max_plays.is_some()
        || !grant.cast_this_way_grants.is_empty() || !grant.permanent_this_way_grants.is_empty()
        || !grant.on_use_effects.is_empty() || grant.instant_timing || grant.land_play_companion
        || grant.top_card_only || grant.may_look_at_top || grant.may_look_at_linked_exile
        || grant.requires_linked_exile_pair || grant.linked_exile_pair.is_some()
        || grant.linked_exile_class_level.is_some() || grant.cast_this_way_filter.is_some()
        || grant.cast_mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }
    let lead = match grant.usage_limit {
        Some(ironsmith_core::GrantUsageLimit::OnceEachTurn) => "Once each turn, ",
        Some(ironsmith_core::GrantUsageLimit::OnceDuringEachOfYourTurns) => "Once during each of your turns, ",
        _ => return None,
    };
    let Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) = &grant.filter.mana_value else {
        return None;
    };
    if matches!(limit.unhinted(), Value::Fixed(_)) {
        return None;
    }
    let mut rest = grant.filter.clone();
    rest.mana_value = None;
    rest.zone = None;
    if rest.owner == Some(PlayerFilter::You) {
        rest.owner = None;
    }
    rest.excluded_card_types.retain(|card_type| *card_type != CardType::Land);
    rest.union_surface = Default::default();
    if rest != ObjectFilter::default() {
        return None;
    }
    let price = costs.iter().map(|cost| cost.display()).collect::<Vec<_>>().join(" and ");
    Some(format!(
        "{lead}you may pay {price} rather than pay the mana cost for a spell you cast with mana value X or less, where X is {}",
        describe_value(limit)
    ))
}

fn audit_alternative_price_grant(grant: &crate::grant::GrantSpec) -> Option<String> {
    let ironsmith_core::Grantable::AlternativePrice { costs, origin } = &grant.grantable else {
        return None;
    };
    // This compact price form describes an explicit casting origin and
    // ordinary mana payment; other cost grammars keep their own reader.
    if origin.is_none() || costs.iter().any(|cost| cost.mana_cost_ref().is_none()) {
        return None;
    }
    if !grant.additional_zones.is_empty() || grant.max_plays.is_some()
        || !grant.cast_this_way_grants.is_empty() || !grant.permanent_this_way_grants.is_empty()
        || !grant.on_use_effects.is_empty() || grant.instant_timing || grant.land_play_companion
        || grant.top_card_only || grant.may_look_at_top || grant.may_look_at_linked_exile
        || grant.requires_linked_exile_pair || grant.linked_exile_pair.is_some()
        || grant.linked_exile_class_level.is_some() || grant.cast_this_way_filter.is_some()
        || grant.cast_mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }
    let lead = match grant.usage_limit {
        None => "",
        Some(ironsmith_core::GrantUsageLimit::OnceEachTurn) => "Once each turn, ",
        Some(ironsmith_core::GrantUsageLimit::OnceDuringEachOfYourTurns) => "Once during each of your turns, ",
        Some(ironsmith_core::GrantUsageLimit::DuringYourTurns) => "During your turn, ",
    };
    let spell = super::normalize_common::describe_cast_spell_filter(
        &grant.filter, super::normalize_common::CastSpellFilterContext::Standalone,
    );
    let who = player(&grant.beneficiary);
    let subject = with_indefinite_article(&spell);
    let origin = origin.map(|zone| match zone {
        Zone::Exile => " from exile".to_string(),
        Zone::Hand => format!(" from {} hand", possessive(&grant.beneficiary)),
        Zone::Graveyard => format!(" from {} graveyard", possessive(&grant.beneficiary)),
        other => format!(" from {}", zone_phrase(&other)),
    }).unwrap_or_default();
    let price = costs.iter().map(|cost| cost.display()).collect::<Vec<_>>().join(" and ");
    let body = if costs.is_empty() {
        format!("{who} may cast {subject}{origin} without paying its mana cost")
    } else {
        format!("{who} may pay {price} rather than pay the mana cost for {subject} {who} cast{origin}")
    };
    Some(capitalize_first(&format!("{lead}{body}")))
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
    // "you may play a land or cast a permanent spell from among cards in your
    // graveyard that were put there from your library this turn" (Kagha,
    // Shadow Archdruid): a land arm and a nonland-permanent arm, gated on
    // the card's graveyard provenance.
    if filter.entered_graveyard_from_library_this_turn
        && grant.zone == Zone::Graveyard
        && own == Some(PlayerFilter::You)
        && let [land, permanent] = filter.any_of.as_slice()
        && land.card_types.as_slice() == [CardType::Land]
        && permanent.excluded_card_types.as_slice() == [CardType::Land]
        && permanent.card_types.as_slice()
            == [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Planeswalker,
                CardType::Battle,
            ]
        && grant.cast_this_way_grants.is_empty()
        && grant.permanent_this_way_grants.is_empty()
    {
        return Some(capitalize_first(&format!(
            "{lead}{who} may play a land or cast a permanent spell from among cards in your graveyard that were put there from your library this turn"
        )));
    }
    // "Once during each of your turns, you may cast a spell from among cards
    // in your graveyard that were milled this turn" (Raul, Trouble Shooter):
    // a use-limited nonland cast gated on the card's mill provenance.
    if filter.milled_into_graveyard_this_turn
        && grant.zone == Zone::Graveyard
        && own == Some(PlayerFilter::You)
        && grant.usage_limit.is_some()
        && filter.any_of.is_empty()
        && filter.excluded_card_types.as_slice() == [CardType::Land]
        && grant.cast_this_way_grants.is_empty()
        && grant.permanent_this_way_grants.is_empty()
        && !grant.instant_timing
        && !grant.land_play_companion
        && grant.max_plays.is_none()
        && matches!(grant.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::Normal)
        && {
            let mut rest = filter.clone();
            rest.milled_into_graveyard_this_turn = false;
            rest.excluded_card_types.clear();
            rest == ObjectFilter::default()
        }
    {
        return Some(capitalize_first(&format!(
            "{lead}{who} may cast a spell from among cards in your graveyard that were milled this turn"
        )));
    }
    // "you may cast a creature spell from among cards in your graveyard that
    // were put there from anywhere other than the battlefield this turn"
    // (Banon, the Returners' Leader): the provenance gate is part of the
    // permitted card set.
    if filter.entered_graveyard_not_from_battlefield_this_turn
        && grant.zone == Zone::Graveyard
        && own == Some(PlayerFilter::You)
        && filter.card_types.as_slice() == [CardType::Creature]
        && grant.cast_this_way_grants.is_empty()
        && grant.permanent_this_way_grants.is_empty()
        && grant.max_plays.is_none()
        && {
            let mut rest = filter.clone();
            rest.entered_graveyard_not_from_battlefield_this_turn = false;
            rest.card_types.clear();
            rest.excluded_card_types.retain(|card_type| *card_type != CardType::Land);
            rest.union_surface = Default::default();
            rest == ObjectFilter::default()
        }
    {
        return Some(capitalize_first(&format!(
            "{lead}{who} may cast a creature spell from among cards in your graveyard that were put there from anywhere other than the battlefield this turn"
        )));
    }
    // "You may play lands and cast spells from your graveyard" (Yawgmoth's
    // Agenda), "play lands and cast Insect spells" (Zask): a land arm and a
    // nonland arm are a land play plus a spell cast.
    let land_play_and_cast = match filter.any_of.as_slice() {
        [land, nonland]
            if land.card_types.as_slice() == [CardType::Land]
                && nonland.excluded_card_types.as_slice() == [CardType::Land]
                && nonland.card_types.is_empty() =>
        {
            let mut bare_land = land.clone();
            bare_land.union_surface = Default::default();
            let mut expected_land = ObjectFilter::default();
            expected_land.card_types = vec![CardType::Land];
            let mut rest = nonland.clone();
            rest.excluded_card_types.clear();
            rest.union_surface = Default::default();
            let subtypes = std::mem::take(&mut rest.subtypes);
            let mut rest_filter = filter.clone();
            rest_filter.any_of.clear();
            (bare_land == expected_land
                && rest == ObjectFilter::default()
                && rest_filter == ObjectFilter::default())
            .then(|| {
                let spells = if subtypes.is_empty() {
                    "spells".to_string()
                } else {
                    format!(
                        "{} spells",
                        subtypes.iter().map(|subtype| subtype.to_string()).collect::<Vec<_>>().join(" ")
                    )
                };
                format!("lands and cast {spells}")
            })
        }
        _ => None,
    };
    let cards = objects(&filter);
    let cards = pluralize_noun_phrase(strip_indefinite_article(&cards));
    let cards = if cards.is_empty() || cards == "permanents" { "cards".to_string() } else { cards };
    let cards = land_play_and_cast.unwrap_or(cards);
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

/// "Creatures your opponents control with hexproof can be the targets of
/// spells and abilities you control as though they didn't have hexproof":
/// read from the protected players and objects, the controller of the
/// sources allowed to target, and the ignored ability.
pub(crate) fn targeting_as_though_no_ability(
    spec: &ironsmith_core::static_ability_model::TargetingAsThoughNoAbilitySpec,
) -> String {
    let ability = ironsmith_core::semantic_audit::humanize_variant(&spec.ignored_ability);
    let protected = spec.objects.as_ref().map(set_subject);
    let (subject, singular) = match (&spec.players, protected) {
        (Some(players), Some((objects, _, _))) => (
            format!("{} and {}", capitalize_first(&player(players)), lowercase_first(&objects)),
            false,
        ),
        (Some(players), None) => (capitalize_first(&player(players)), false),
        (None, Some((objects, verb, _))) => (objects, verb == "is"),
        (None, None) => ("Permanents".to_string(), false),
    };
    let sources = match &spec.sources_controlled_by {
        PlayerFilter::Any => String::new(),
        PlayerFilter::You => " you control".to_string(),
        other => format!(" controlled by {}", player(other)),
    };
    let (target, pronoun) = if singular { ("target", "it") } else { ("targets", "they") };
    format!(
        "{subject} can be the {target} of spells and abilities{sources} as though {pronoun} didn't have {ability}"
    )
}

fn mana_symbol_name(symbol: &ironsmith_core::ManaSymbol) -> String {
    format!("{symbol:?}").to_ascii_lowercase()
}

/// "You may spend mana as though it were mana of any color to cast
/// planeswalker spells."
pub(crate) fn mana_spend_permission(permission: &ironsmith_core::ManaSpendPermission) -> String {
    mana_spend_permission_for_scope(permission, &mana_spend_scope(&permission.scope))
}

/// Render a proven scope separately so equal typed permissions can share a
/// sentence without consulting their retained display strings.
pub(super) fn mana_spend_scope(scope: &ironsmith_core::ManaSpendScope) -> String {
    use ironsmith_core::ManaSpendScope as Scope;
    match scope {
        Scope::AllCosts => String::new(),
        Scope::ActivationCostsOf(filter) => {
            let holder = objects(filter);
            if holder.starts_with("this ") && holder.split(' ').count() == 2 {
                format!(" to pay the activation costs of {holder}'s abilities")
            } else {
                let (holder, _, _) = set_subject(filter);
                format!(" to activate abilities of {}", lowercase_first(&holder))
            }
        }
        Scope::CastingSpellsMatching(filter) => {
            // Exile ownership is an eligibility restriction, not a possessive
            // zone name. Keep the entire card pool: projecting it to the stack
            // used to discard its owner and nested branches' origin zones.
            if filter.zone == Some(Zone::Exile)
                && filter.any_of.iter().all(|branch| branch.zone.is_none())
            {
                // Exile is a shared zone. Project only its zone to the spell
                // subject so ownership stays an eligibility qualifier.
                let mut spells = filter.clone();
                spells.zone = None;
                let spell = super::normalize_common::describe_cast_spell_filter(
                    &spells, super::normalize_common::CastSpellFilterContext::Standalone,
                );
                return format!(" to cast {} from exile", super::normalize_common::pluralize_cast_spell_description(&spell));
            }
            if filter.zone == Some(Zone::Exile)
                || filter.any_of.iter().any(|branch| branch.zone == Some(Zone::Exile))
            {
                let cards = describe_revealed_selection_with_cards(&objects(filter));
                let cards = pluralize_relative_object_phrase(strip_indefinite_article(&cards));
                return format!(" to cast spells from among {cards}");
            }
            let spell = super::normalize_common::describe_cast_spell_filter(
                filter,
                super::normalize_common::CastSpellFilterContext::Standalone,
            );
            format!(" to cast {}", super::normalize_common::pluralize_cast_spell_description(&spell))
        }
        Scope::CastingSpellsWithStableIds(ids) => format!(" to cast {}", sr(ids)),
        Scope::OneSpellCast => " to pay the mana cost of one spell this turn".to_string(),
        Scope::ActivationCostsOfAbilityKind { filter, kind } => {
            let kind = match kind {
                ironsmith_core::ActivatedAbilityKindSelector::Keyword(keyword) => keyword
                    .official_keyword()
                    .unwrap_or("keyword")
                    .to_string(),
                ironsmith_core::ActivatedAbilityKindSelector::Loyalty => "loyalty".to_string(),
            };
            if filter.source {
                format!(" to pay the activation costs of this object's {kind} abilities")
            } else if *filter == ironsmith_core::ObjectFilter::default() {
                format!(" to pay the activation costs of {kind} abilities")
            } else {
                let (holder, _, _) = set_subject(filter);
                format!(
                    " to pay the activation costs of {kind} abilities of {}",
                    lowercase_first(&holder)
                )
            }
        }
    }
}

pub(super) fn mana_spend_permission_for_scope(
    permission: &ironsmith_core::ManaSpendPermission,
    scope_text: &str,
) -> String {
    use ironsmith_core::value_model::ManaSpendMode as Mode;
    let subject = match &permission.player {
        PlayerFilter::You => "You may spend".to_string(),
        PlayerFilter::Any => "Players may spend".to_string(),
        other => format!("{} may spend", capitalize_first(&player(other))),
    };
    let (spent, as_though) = if let Some((from, to)) = &permission.mana_symbol_as {
        (format!("{} mana", mana_symbol_name(from)), format!(" as though it were {} mana", mana_symbol_name(to)))
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
        (spent, as_though.to_string())
    };
    let source = match &permission.mana_source_filter {
        Some(filter) => {
            let source = damage_source_objects(filter);
            format!(" from {}", pluralize_relative_object_phrase(strip_indefinite_article(&source)))
        }
        None => String::new(),
    };
    let mut text = format!("{subject} {spent}{source}{as_though}{scope_text}");
    if permission.other_mana_only_as_colorless {
        text.push_str(". You may spend other mana only as though it were colorless mana");
    }
    text
}

/// "If a land is tapped for mana, it produces {B} instead of any other type":
/// a mana production rewrite read from its source, provenance, input,
/// output and quantity.
pub(crate) fn mana_output_rewrite(rule: &ironsmith_core::ManaOutputRewrite) -> String {
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
        In::Colored => "any other color".to_string(),
        // The replaced type is named by its color ("instead of white mana",
        // Quarum Trench Gnomes); the produced symbol keeps its symbol.
        In::Symbol(ironsmith_core::ManaSymbol::White) => "white mana".to_string(),
        In::Symbol(ironsmith_core::ManaSymbol::Blue) => "blue mana".to_string(),
        In::Symbol(ironsmith_core::ManaSymbol::Black) => "black mana".to_string(),
        In::Symbol(ironsmith_core::ManaSymbol::Red) => "red mana".to_string(),
        In::Symbol(ironsmith_core::ManaSymbol::Green) => "green mana".to_string(),
        In::Symbol(input) => symbol(&input),
    };
    let amount = match rule.quantity {
        Qty::Preserve => String::new(),
        Qty::Exact(1) => " and amount".to_string(),
        Qty::Exact(n) => format!(" and amount, {} times", count_words(n)),
    };
    // "spells and abilities you control that would add colored mana instead
    // add that much white mana" (False Dawn): the authored producer-side form
    // of a colored-input rewrite to one colored symbol.
    if rule.surface == ironsmith_core::ManaOutputRewriteSurface::SpellsAndAbilitiesWouldAdd
        && rule.input == In::Colored
        && let (Some(who), Out::Symbol(output)) = (&rule.controller, &rule.output)
        && let Some(color) = match output {
            ironsmith_core::ManaSymbol::White => Some("white"),
            ironsmith_core::ManaSymbol::Blue => Some("blue"),
            ironsmith_core::ManaSymbol::Black => Some("black"),
            ironsmith_core::ManaSymbol::Red => Some("red"),
            ironsmith_core::ManaSymbol::Green => Some("green"),
            _ => None,
        }
    {
        return format!(
            "Spells and abilities {} control that would add colored mana instead add that much {color} mana",
            player(who)
        );
    }
    let source = objects(&rule.source_filter);
    match &rule.output {
        output @ (Out::ByBasicLandType(_) | Out::ByLandTypeClauses(_)) => {
            let pairs = output.land_type_clauses().unwrap().into_iter()
                .filter_map(|(land, output)| output.map(|output| {
                    let suffix = if land == ironsmith_core::Subtype::Plains { "" } else { "s" };
                    format!("{}{suffix} produce {}", land.display_name(), symbol(&output))
                }))
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
                Out::ChooseColor if matches!(rule.quantity, Qty::Exact(1)) => {
                    "one mana of a color of your choice".to_string()
                }
                Out::ChooseColor => "mana of a color of your choice".to_string(),
                Out::ByBasicLandType(_) | Out::ByLandTypeClauses(_) => unreachable!(),
            };
            let head = match (&rule.controller, rule.tapped_for_mana) {
                // "if you tap a land you control for mana" (Deep Water,
                // Harvest Mage): the tapping player leads.
                (Some(who), true) => format!(
                    "If {} {} {} for mana",
                    player(who),
                    if matches!(who, PlayerFilter::You) { "tap" } else { "taps" },
                    article(&source)
                ),
                // "if a player taps a nonbasic land for mana" (Pale Moon).
                (None, true)
                    if rule.surface == ironsmith_core::ManaOutputRewriteSurface::PlayerTaps
                        || rule.surface
                            == ironsmith_core::ManaOutputRewriteSurface::PlayerTapsNamedSource =>
                {
                    format!("If a player taps {} for mana", article(&source))
                }
                _ => format!("If {} {event}", article(&source)),
            };
            // "that Mountain produces" (Chaos Moon): the tapped source named again.
            let producer = if rule.surface
                == ironsmith_core::ManaOutputRewriteSurface::PlayerTapsNamedSource
                && rule.controller.is_none()
                && rule.tapped_for_mana
            {
                format!("that {}", strip_indefinite_article(&source))
            } else {
                "it".to_string()
            };
            format!("{head}, {producer} produces {produced} instead of {replaced}{amount}")
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
    // "As long as an opponent controls a creature with flying not named
    // Escaped Shapeshifter": an opponent-controlled holder names the opponent
    // as the controlling subject; the tested quality qualifies the noun ahead
    // of any name exclusion.
    let opponent_holder = (spec.filter.controller == Some(PlayerFilter::Opponent))
        .then(|| {
            let mut controlled = spec.filter.clone();
            controlled.controller = None;
            let full = objects(&controlled);
            let mut unnamed = controlled;
            unnamed.excluded_name = None;
            unnamed.excluded_name_surface = Default::default();
            let noun = objects(&unnamed);
            full.strip_prefix(noun.as_str()).map(|tail| {
                format!(
                    "an opponent controls {} with {first_quality}{tail}",
                    with_indefinite_article(&noun)
                )
            })
        })
        .flatten();
    let mut text = match opponent_holder {
        Some(clause) => format!("As long as {clause}, this source has {first}"),
        None => format!("As long as {holder} has {first_quality}, this source has {first}"),
    };
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
            // "If a card would be put into enchanted player's graveyard from
            // anywhere" (Wheel of Sun and Moon): a zone-unrestricted physical
            // card is a card, its owner names the graveyard it goes to, and no
            // origin zone means from anywhere.
            if object.physical_card
                && object.zone.is_none()
                && from.is_none()
                && *to == Some(Zone::Graveyard)
            {
                let mut card = object.clone();
                let graveyard = card
                    .owner
                    .take()
                    .map_or("a graveyard".to_string(), |owner| format!("{} graveyard", possessive(&owner)));
                return format!(
                    "{} would be put into {graveyard} from anywhere",
                    article(&card.description_with_default_noun("card"))
                );
            }
            let self_reference = object.source;
            let object = article(&objects(object));
            match (from, to) {
                (None, Some(Zone::Battlefield)) => format!("{object} would enter the battlefield"),
                // CR 700.4: only a creature dies; another named permanent kind
                // is put into a graveyard from the battlefield.
                (Some(Zone::Battlefield), Some(Zone::Graveyard))
                    if object.contains("creature")
                        || (self_reference
                            && !matches!(
                                object.as_str(),
                                "this artifact" | "this enchantment" | "this land"
                            )) =>
                {
                    format!("{object} would die")
                }
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
            // "You may look at an additional two cards each time you surveil"
            // (Enhanced Surveillance).
            if matches!(verb, "surveil" | "scry")
                && matches!(performer, PlayerFilter::You)
                && optional
                && let M::Add(n) = modifier
                && *n > 0
            {
                return format!(
                    "You may look at an additional {} card{} each time you {verb}",
                    count_words(*n),
                    if *n == 1 { "" } else { "s" }
                );
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
        P::OptionalTextFragments(spec) => if spec.casting_payment.is_some() {
            "For each time you pay this spell's optional additional cost, remove one bracketed text fragment from it".to_string()
        } else { String::new() },
        P::NamedActionPaymentAlternative { label, effects: body, .. } => format!("You may pay the cost \"{}\" rather than the cost of performing the {label} action.", effects(body)),
        // "You can't cast this spell unless an opponent lost life this turn"
        // (Rakdos, Lord of Riots): the authored prohibition surface.
        P::ThisSpellCastRestriction { kind, .. }
            if kind.cant_cast_unless_surface && kind.timing.is_none() && kind.condition.is_some() =>
        {
            format!(
                "You can't cast this spell unless {}",
                describe_condition(kind.condition.as_ref().expect("condition checked"))
            )
        }
        // "You can't cast Rock Jockey if you've played a land this turn": the
        // authored prohibition while the negated condition holds.
        P::ThisSpellCastRestriction { kind, .. }
            if kind.cant_cast_if_surface
                && kind.timing.is_none()
                && matches!(kind.condition, Some(crate::ConditionExpr::Not(_))) =>
        {
            match kind.condition.as_ref() {
                Some(crate::ConditionExpr::Not(inner)) => format!(
                    "You can't cast this spell if {}",
                    match inner.as_ref() {
                        crate::ConditionExpr::TurnHistory(
                            ironsmith_core::TurnHistoryCondition::PlayerPlayedLandThisTurn(PlayerFilter::You),
                        ) => "you've played a land this turn".to_string(),
                        other => describe_condition(other),
                    }
                ),
                _ => String::new(),
            }
        }
        P::NamedActionTokenAbilityGrant { display, .. } => display.clone(),
        P::ActivateAbilitiesAsThoughZone { display, .. } => display.clone(),
        P::PhaseBoundaryPlayerAction { boundary, label, condition } => {
            let boundary = match boundary {
                ironsmith_core::static_ability_model::PlayerActionBoundary::EndStep => "end step",
                ironsmith_core::static_ability_model::PlayerActionBoundary::BeginCombat => "combat",
            };
            format!("As your {boundary} begins, if {}, you perform the {label} action once rather than once for each permanent with this ability", describe_condition(condition))
        },
        P::GrantOptionalSpellCost { filter, price, reference, effects: body, timing, generic_reduction, .. } => {
            let price = match price {
                ironsmith_core::static_ability_model::GrantedOptionalCostPrice::Fixed(cost) => cost.display(),
                ironsmith_core::static_ability_model::GrantedOptionalCostPrice::GenericManaValue => "mana equal to this spell's mana value".into(),
            };
            let consequence = match timing {
                ironsmith_core::static_ability_model::GrantedOptionalCostTiming::DuringCasting => "If you do, ",
                ironsmith_core::static_ability_model::GrantedOptionalCostTiming::CastTrigger => "When you cast that spell, if you paid this additional cost, ",
            };
            let reduction = generic_reduction.as_ref().map(|amount| format!(". That spell costs {} less to cast", value_words(amount))).unwrap_or_default();
            format!("As an additional cost to cast {}, you may pay {} for {}. {}{}{}", objects(filter), price, reference.display_label(), consequence, effects(body), reduction)
        }
        P::OptionalCostPaymentAlternative { filter, reference, replacement } => format!(
            "You may pay {} rather than pay the {} cost for {}",
            replacement.display(), reference.custom_keyword.clone().unwrap_or_else(||reference.display_label()), objects(filter)
        ),
        P::Grants(_) => crate::static_abilities::StaticAbility::from_model(model.clone())
            .grant_spec()
            .map(|grant| {
                if let Some(evoke) = audit_granted_hand_evoke(&grant) {
                    return evoke;
                }
                if let Some(price) = audit_alternative_price_grant(&grant) {
                    return price;
                }
                if let Some(price) = audit_limited_dynamic_mana_value_price_grant(&grant) {
                    return price;
                }
                if let Some(free) = audit_source_exiled_free_cast_grant(&grant) {
                    return free;
                }
                if let Some(free) = audit_limited_hand_free_cast_grant(&grant) {
                    return free;
                }
                if let Some(play) = audit_active_player_source_exiled_play_refill(&grant) {
                    return play;
                }
                if let Some(play) = audit_library_historic_play_with_reflexive(&grant) {
                    return play;
                }
                // "you may cast a spell from among cards in your graveyard
                // that were milled this turn" (Raul): the provenance-gated
                // reading, even when the grant has its own display.
                if (grant.filter.milled_into_graveyard_this_turn
                    || grant.filter.entered_graveyard_not_from_battlefield_this_turn)
                    && let Some(milled) = audit_play_from_grant(&grant)
                {
                    return milled;
                }
                let display = grant.display();
                // A play permission with no prose reading reads from its
                // typed origin, filter, beneficiary and use limit.
                if display.ends_with(" have play from zone") {
                    audit_play_from_grant(&grant).unwrap_or(display)
                } else {
                    let riders = audit_permanent_this_way_riders(&grant);
                    if riders.is_empty() || display.contains(" this way") {
                        display
                    } else {
                        format!("{}{riders}", display.trim_end_matches('.'))
                    }
                }
            })
            .unwrap_or_else(|| ironsmith_core::semantic_audit::unverified_marker(
                "GrantSpec", model, &[])),
        // "Other nontoken artifact creatures and Vehicles you control have
        // modular 1" (Blaster, Combat DJ): a granted bundle that is exactly
        // one counter keyword's abilities reads as that keyword.
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none()
                && granted_counter_keyword(&grant.ability, &grant.additional_abilities).is_some() =>
        {
            let keyword = granted_counter_keyword(&grant.ability, &grant.additional_abilities)
                .unwrap_or_default();
            let (subject, verb, _) = match conjunctive_controlled_set_subject(&grant.filter) {
                Some(subject) => (subject, "are", "their"),
                None => set_subject(&grant.filter),
            };
            format!("{subject} {} {keyword}", if verb == "is" { "has" } else { "have" })
        }
        // "The first spell you cast during each of your turns has assist"
        // (Yume, Chronicler of Valor): the your-turn gate scopes the ordinal.
        P::GrantObjectAbilityForFilter(grant)
            if matches!(grant.condition, Some(crate::effect::Condition::YourTurn))
                && grant.additional_abilities.is_empty()
                && grant.filter.first_spell_cast_each_turn
                && grant.filter.cast_by == Some(PlayerFilter::You)
                && grant.filter.zone == Some(Zone::Stack)
                && grant.filter.card_types.is_empty()
                && grant.filter.subtypes.is_empty()
                && matches!(&grant.ability.kind, ironsmith_core::AbilityKind::Static(_))
                && merge_passes::is_keyword_phrase(&ability_model(&grant.ability)) =>
        {
            format!(
                "The first spell you cast during each of your turns has {}",
                ability_model(&grant.ability).to_ascii_lowercase()
            )
        }
        // "Each Sliver card in each player's hand has slivercycling {3}"
        // (Homing Sliver): an ability granted to the matching cards in every
        // player's hand (no owner or controller scope on the hand).
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && grant.filter.zone == Some(Zone::Hand)
                && grant.filter.owner.is_none()
                && grant.filter.controller.is_none()
                && !grant.filter.source
                && grant.set_quantifier_surface == Some(ironsmith_core::SetQuantifierSurface::Each) =>
        {
            let mut cards = grant.filter.clone();
            cards.zone = None;
            let description = objects(&cards);
            let noun = strip_indefinite_article(&description).trim().to_string();
            let granted = ability_model_for_recipient(&grant.ability, &grant.filter);
            format!("Each {noun} in each player's hand has {granted}")
        }
        // "Artifacts you control have "{2}, Sacrifice this artifact: ..."" (Food
        // Fight): the granted ability's self-reference is the recipient's type.
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && matches!(&grant.ability.kind,
                    ironsmith_core::AbilityKind::Activated(activated) if !activated.is_loyalty_ability)
                && matches!(grant.filter.card_types.as_slice(),
                    [CardType::Artifact | CardType::Enchantment | CardType::Land])
                && !grant.filter.source =>
        {
            let (subject, verb, _) = set_subject(&grant.filter);
            // The authored "All lands have ..." quantifier is kept.
            let subject = if grant.set_quantifier_surface == Some(ironsmith_core::SetQuantifierSurface::All)
                && !subject.starts_with("All ")
            {
                format!("All {}", lowercase_first(&subject))
            } else {
                subject
            };
            format!("{subject} {} {}", if verb == "is" { "has" } else { "have" },
                ability_model_for_recipient(&grant.ability, &grant.filter))
        }
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && matches!(&grant.ability.kind,
                    ironsmith_core::AbilityKind::Activated(activated)
                        if activated.is_loyalty_ability && activated.mana_output.is_some() && activated.effects.is_empty()
                            && activated.mana_usage_restrictions.is_empty()) =>
        {
            let (subject, verb, _) = set_subject(&grant.filter);
            format!("{subject} {} {}", if verb == "is" { "has" } else { "have" }, ability_model(&grant.ability))
        }
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && matches!(&grant.ability.kind,
                    ironsmith_core::AbilityKind::Static(ability)
                        if ability.id == Some(crate::static_abilities::StaticAbilityId::MustAttack)
                            && matches!(ability.payload, ironsmith_core::StaticAbilityPayload::None)) =>
        {
            let (mut subject, verb, _) = set_subject(&grant.filter);
            if verb == "are"
                && let Some(controller @ PlayerFilter::TaggedPlayer(_)) = &grant.filter.controller
            {
                // A tagged player's controller clause is postpositive;
                // pluralize the object head, not its trailing "controls".
                let mut recipients = grant.filter.clone();
                recipients.controller = None;
                subject = format!("{} {} controls", set_subject(&recipients).0, player(controller));
            }
            format!("{subject} {} each combat if able", if verb == "is" { "attacks" } else { "attack" })
        }
        // A granted rules restriction on "this creature" belongs to the
        // recipient (Infiltrator's Magemark): its self-reference names the
        // receiving object, never the granting Aura or Equipment.
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && matches!(&grant.ability.kind,
                    ironsmith_core::AbilityKind::Static(ability)
                        if ability.id == Some(crate::static_abilities::StaticAbilityId::RuleRestriction)) =>
        {
            let (subject, verb, _) = set_subject(&grant.filter);
            format!(
                "{subject} {} {}",
                if verb == "is" { "has" } else { "have" },
                ability_model_for_recipient(&grant.ability, &grant.filter)
            )
        }
        // "Cleric creatures have vigilance" (Akroma's Devoted): an
        // unquantified, controller-less subtype set reads in the plural; the
        // "All" quantifier belongs only to an authored "All Sliver creatures"
        // (`set_quantifier_surface`).
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && grant.set_quantifier_surface.is_none()
                && !grant.filter.subtypes.is_empty()
                && matches!(grant.filter.card_types.as_slice(), [] | [CardType::Creature])
                && grant.filter.controller.is_none()
                && grant.filter.zone.is_none_or(|zone| zone == Zone::Battlefield)
                && !grant.filter.other && !grant.filter.source
                && matches!(&grant.ability.kind, ironsmith_core::AbilityKind::Static(_))
                && merge_passes::is_keyword_phrase(&ability_model(&grant.ability)) =>
        {
            let (subject, verb, _) = set_subject(&grant.filter);
            format!("{subject} {} {}", if verb == "is" { "has" } else { "have" }, ability_model(&grant.ability))
        }
        // "Other Dogs you control have unleash" (Tesak): the granted keyword
        // (CR 702.98a) names itself.
        P::GrantObjectAbilityForFilter(grant)
            if grant.condition.is_none() && grant.additional_abilities.is_empty()
                && matches!(&grant.ability.kind,
                    ironsmith_core::AbilityKind::Static(ability)
                        if ability.id == Some(crate::static_abilities::StaticAbilityId::Unleash)
                            && matches!(ability.payload, ironsmith_core::StaticAbilityPayload::None)) =>
        {
            let (subject, verb, _) = set_subject(&grant.filter);
            format!("{subject} {} unleash", if verb == "is" { "has" } else { "have" })
        }
        // "Each noncreature, non-Equipment artifact is an Equipment with equip
        // {X} and "Equipped creature gets +X/+0," where X is that artifact's
        // mana value." (Bludgeon Brawl): the equip price and the equipped
        // creature's bonus both read the recipient's own mana value.
        P::AddSubtypesAndGrantAbilities { filter, subtypes, abilities, .. }
            if subtypes.as_slice() == [crate::types::Subtype::Equipment]
                && filter.card_types.as_slice() == [CardType::Artifact]
                && matches!(abilities.as_slice(), [equip, equipped]
                    if matches!(&equip.kind, ironsmith_core::AbilityKind::Activated(activated)
                        if activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Equip))
                    && matches!(&equipped.kind, ironsmith_core::AbilityKind::Static(ability)
                        if matches!(&ability.payload, P::Anthem(anthem)
                            if anthem.toughness == ironsmith_core::AnthemValue::Fixed(0)
                                && matches!(anthem.power, ironsmith_core::AnthemValue::Dynamic(Value::ManaValueOf(_)))))) =>
        {
            format!(
                "Each {} is an Equipment with equip {{X}} and \"Equipped creature gets +X/+0,\" where X is that artifact's mana value",
                strip_indefinite_article(&objects(filter))
            )
        }
        P::AddSubtypesAndGrantAbilities { filter, subtypes, abilities, .. } => format!(
            "Each {} is {} in addition to its other types and has {}",
            strip_indefinite_article(&objects(filter)),
            with_indefinite_article(
                &subtypes.iter().map(|subtype| format!("{subtype:?}")).collect::<Vec<_>>().join(" ")
            ),
            abilities
                .iter()
                .map(|ability| ability_model_for_recipient(ability, filter))
                .collect::<Vec<_>>()
                .join(" and ")
        ),
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
            // A described quantity ("the discarded card's mana value") is
            // printed as X with a closing where-clause (Dralnu's Pet).
            let described_count = value_words(count);
            let where_x = (!matches!(count, crate::effect::Value::Fixed(_)) && described_count.contains(' '))
                .then_some(described_count.clone());
            let counters = format!(
                "{} {} counter{} on it",
                if where_x.is_some() { "X".to_string() } else if plural { described_count } else { "a".to_string() },
                describe_counter_type(*counter),
                if plural { "s" } else { "" }
            );
            if let Some(where_x) = where_x
                && !matches!(condition, crate::ConditionExpr::ThisSpellEscaped)
            {
                let mut withs = added_abilities
                    .iter()
                    .map(|ability| ability_model(ability))
                    .collect::<Vec<_>>();
                withs.push(counters);
                return Some(format!(
                    "If {}, this source enters with {}, where X is {where_x}",
                    describe_condition(condition),
                    withs.join(" and with ")
                ));
            }
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
            "{} can't attack {} unless {} {}{}",
            if attackers.source { "This source".to_string() } else { set_subject(attackers).0 },
            if *planeswalkers_only {
                "planeswalkers you control"
            } else if *covers_planeswalkers {
                "you or planeswalkers you control"
            } else {
                "you"
            },
            if attackers.source { "you pay" } else { "their controller pays" },
            cost.display(),
            if attackers.source { "" } else { " for each of those creatures" }
        ),
        // "Enchanted creature can't be blocked unless defending player pays
        // {3} for each creature they control that's blocking it" (Awesome
        // Presence): a per-blocker tax on blocking the creature this source is
        // attached to.
        P::BlockCost {
            blockers,
            blocker_is_attached_to_source: false,
            attackers,
            cost,
            also_attack_cost: false,
            attack_only: false,
            ..
        } if *blockers == ObjectFilter::creature()
            && attackers.with_attached_object.as_ref().is_some_and(|attached| attached.source)
            && {
                let mut host = attackers.clone();
                host.with_attached_object = None;
                host == ObjectFilter::creature()
            } =>
        {
            format!(
                "Enchanted creature can't be blocked unless defending player pays {} for each creature they control that's blocking it",
                cost.display()
            )
        }
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
            // "This creature can't attack or block unless you tap an untapped
            // creature you control ..." (Hollow Warrior): an effect-only cost
            // reads its own payment verb.
            let effect_only_cost = matches!(cost.kind(),
                ironsmith_core::TotalCostKind::All(costs)
                    if !costs.is_empty() && costs.iter().all(|cost| cost.effect_ref().is_some()));
            if blockers.source
                && effect_only_cost
                && let ironsmith_core::TotalCostKind::All(costs) = cost.kind()
                && let [only] = costs.as_slice()
                && let Some(effect) = only.effect_ref()
                && let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
                && let [choose, tap] = sequence.effects.as_slice()
                && let Some(choose) = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()
                && let Some(tap) = tap.downcast_ref::<crate::effects::TapEffect>()
                && matches!(tap.target.base(), crate::target::ChooseSpec::Tagged(tag) if *tag == choose.tag)
                && choose.count.is_single()
                && choose.filter.nonattacking
                && choose.filter.nonblocking
            {
                let mut tapped = choose.filter.clone();
                tapped.nonattacking = false;
                tapped.nonblocking = false;
                return Some(format!(
                    "{} can't {action} unless you tap {} not declared as an attacking or blocking creature this combat",
                    capitalize_first(&subject),
                    article(&objects(&tapped))
                ));
            }
            if blockers.source && effect_only_cost {
                let payment = lowercase_first(describe_total_cost(cost).trim().trim_end_matches('.'));
                if !payment.is_empty() {
                    return Some(format!(
                        "{} can't {action} unless you {payment}",
                        capitalize_first(&subject)
                    ));
                }
            }
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
        P::TriggerClassEventChoice { class, official_keyword, event, sources, include_event_source, destination_subject, maximum, optional, automatic, caused_by_event } => {
            format!("At the event {}, {}select {} {} triggered program{} from {}{} and trigger the selected programs due to {}",
                event.display(), if *automatic {"automatically "} else if *optional {"you may "} else {""}, maximum, if *official_keyword {format!("official keyword {class}")} else {class.clone()},
                if *maximum==1 {""} else {"s"}, sources.as_ref().map(objects).unwrap_or_default(),
                if *include_event_source {if *destination_subject {" or the event arrival"} else {" or the event source"}} else {""},
                if *caused_by_event {"the observed event"} else {"the selection"})
        }
        P::IntrinsicImmediateAction { cost, timing, source_zone, player: actor, condition, shared_turn_group, effects: program, once } => {
            let timing = match timing {
                ironsmith_core::static_ability_model::ImmediateActionTiming::Priority => "Any time you have priority",
                ironsmith_core::static_ability_model::ImmediateActionTiming::Sorcery => "Any time you could cast a sorcery",
                ironsmith_core::static_ability_model::ImmediateActionTiming::Attacking => "Any time you have priority and this permanent is attacking",
            };
            let zone = if *source_zone == Zone::Battlefield { String::new() }
                else { format!(" Take this action only while this card is in your {}.", source_zone.name()) };
            let pronoun = if *actor == PlayerFilter::You { "you" } else { "they" };
            let actor = if *actor == PlayerFilter::You { "you".to_string() } else { player(actor) };
            let condition = condition.as_ref().map(|condition| format!(" Take this action only if {}.", describe_condition(condition))).unwrap_or_default();
            let group = shared_turn_group.as_ref().map(|group| format!(" Actions of this permanent in group \"{group}\" share one use each turn.")).unwrap_or_default();
            format!("{timing}, {actor} may pay {} to take a special action. If {pronoun} do, {}.{}{zone}{condition}{group}",
                cost.display(), lowercase_first(&effects(program)),
                if *once { " Take this action only once." } else { "" })
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
        // "Locus of Enlightenment has each activated ability of the exiled
        // cards used to craft it" (CR 702.167c): the craft materials stay
        // linked to the crafted permanent.
        P::CopyActivatedAbilities(spec)
            if spec.filter.has_craft_material_reference()
                && !spec.only_loyalty
                && spec.counter.is_none()
                && !spec.exclude_source_name
                && !spec.exclude_mana_abilities
                && !spec.exclude_loyalty_abilities =>
        {
            let mut text =
                "This source has each activated ability of the exiled cards used to craft it".to_string();
            if spec.force_once_each_turn {
                text.push_str(". You may activate each of those abilities only once each turn");
            }
            text
        }
        P::CopyActivatedAbilities(spec) => {
            // The object chosen as this permanent entered is one object, not
            // a set: "the chosen permanent" (Scheming Fence).
            let chosen_object = spec.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == "__chosen_objects__"
            });
            let mut text = if chosen_object {
                format!(
                    "This source has all {}abilities of {}",
                    if spec.only_loyalty { "loyalty " } else { "activated " },
                    objects(&spec.filter)
                )
            } else {
                format!(
                    "This source has all {}abilities of all {}",
                    if spec.only_loyalty { "loyalty " } else { "activated " },
                    pluralize_relative_object_phrase(strip_indefinite_article(&objects(&spec.filter)))
                )
            };
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
            if spec.exclude_loyalty_abilities {
                text.push_str(" except loyalty abilities");
            }
            // A card in exile or a graveyard is never this battlefield object.
            // Excluding this object is inert (it already has its own
            // abilities); Oracle names the battlefield scope instead. The
            // "all other ..." form keeps the marker for the line pass that
            // reads it as "on the battlefield".
            if spec.exclude_source_id
                && !spec.exclude_source_name
                && spec.filter.controller != Some(PlayerFilter::Opponent)
                && matches!(spec.filter.zone, None | Some(Zone::Battlefield))
            {
                if spec.filter.other {
                    text.push_str(" other than this object");
                } else if spec.filter.zone == Some(Zone::Battlefield)
                    && spec.filter.controller.is_none()
                    && spec.filter.owner.is_none()
                    && spec.filter.tagged_constraints.is_empty()
                    && spec.filter.with_counter.is_none()
                    && spec.counter.is_none()
                {
                    text.push_str(" on the battlefield");
                }
            }
            if spec.force_once_each_turn {
                text.push_str(". You may activate each of those abilities only once each turn");
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
            "If {} would search a library, that player searches the top {} {} of that library instead",
            player(searcher),
            count_words(*count),
            if *count == 1 { "card" } else { "cards" }
        ),
        P::PlayerProtectionFrom { player: who, source_filter, .. } => format!(
            "{} {} protection from {}",
            capitalize_first(&player(who)),
            if matches!(who, PlayerFilter::You) { "have" } else { "has" },
            // CR 702.16j: everything an opponent controls is "each of your
            // opponents" (Absolute Virtue, Cliffside Rescuer).
            if source_filter.controller == Some(PlayerFilter::Opponent) && {
                let mut bare = source_filter.clone();
                bare.controller = None;
                bare == ObjectFilter::default()
            } {
                "each of your opponents".to_string()
            } else if let Some(quality) = source_filter.protection_chosen_card_type_quality() {
                // "protection from the chosen card type" (Serra's Emissary).
                quality.to_string()
            } else {
                objects(source_filter)
            }
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
        P::AdditionalSpellTargets { filter, amount } => format!("As you choose targets for {}, choose up to {amount} additional targets",objects(filter)),
        P::AttachedIsCopyOfSource { retain_original_types } => {
            let tail = if *retain_original_types { " except for its card types, supertypes, and subtypes" } else { "" };
            format!("The permanent this is attached to is a copy of this permanent{tail}")
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
            // "Each creature you control is the chosen type" (Xenograft).
            let (subject, is, its) =
                if filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Each) {
                    (format!("Each {}", strip_indefinite_article(&objects(filter))), "is", "its")
                } else {
                    set_subject(filter)
                };
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
        P::AddChosenColor { filter, .. }
            if filter.global_characteristic_domain_surface()
                == Some(
                    ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents,
                ) =>
        {
            "All cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors".to_string()
        }
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
                // "a triggered ability of this creature or an emblem you own"
                // (The Masamune): the source arm plus your command-zone
                // objects, which are the emblems you own.
                let mut command = ObjectFilter::default();
                command.zone = Some(Zone::Command);
                command.owner = Some(PlayerFilter::You);
                let mut outer = filter.clone();
                outer.any_of.clear();
                if outer == ObjectFilter::default()
                    && filter.any_of.as_slice() == [ObjectFilter::source(), command]
                {
                    return "a triggered ability of this creature or an emblem you own".to_string();
                }
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
        P::SuppressMatchingTriggeredAbilities { source_filter, event_matcher, ward_only, .. } => {
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
                        Some(nouns) => {
                            let explicit_battlefield = trigger
                                .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
                                .is_some_and(|zone_change| {
                                    zone_change.object_filter.has_entered_battlefield_explicit_surface()
                                });
                            let entering = if explicit_battlefield { "entering the battlefield" } else { "entering" };
                            format!("{} {entering} don't cause {abilities} to trigger", capitalize_first(&nouns))
                        }
                        None => format!("{} doesn't cause {abilities} to trigger", capitalize_first(&gerund)),
                    }
                }
                None if *ward_only => format!("Ward {abilities} don't trigger"),
                None => format!("Triggered {abilities} don't trigger"),
            }
        }
        P::AdditionalLifeCostPerTarget(life) => {
            format!("This spell costs an additional {life} life to cast for each target")
        }
        // CR 903.8: the {2} commander tax per previous cast from the command
        // zone is replaced by a life payment.
        P::CommanderTaxLifeSubstitution { life_per_previous_cast } => format!(
            "Rather than pay {{2}} for each previous time you've cast this spell from the command zone this game, pay {life_per_previous_cast} life that many times"
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
            if *reveal_opponents_hands
                && *require_nonland_from_revealed_opponents
                && !*opponent_also_chooses
                && !*exclude_basic_land_names
            {
                text.push_str(
                    "each opponent reveals their hand. You choose the name of a nonland card revealed this way",
                );
                return Some(text);
            }
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
        // "choose two abilities from among first strike, vigilance, and
        // lifelink" (Greymond): every option is one combination of the list.
        P::ChooseNamedOptionAsEnters { options, at_random: false, each_player: false, .. }
            if listed_combination_options(options).is_some() =>
        {
            let (items, picks) = listed_combination_options(options).unwrap_or_default();
            let count = number_word(picks as i32).unwrap_or_else(|| picks.to_string());
            let list = match items.as_slice() {
                [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
                [] => String::new(),
            };
            format!("As this permanent enters, choose {count} abilities from among {list}")
        }
        P::ChooseNamedOptionAsEnters { options, at_random, each_player, .. } => format!(
            "As this permanent enters, {} {}{}",
            if *each_player { "each player chooses" } else { "choose" },
            or_list(options),
            if *at_random { " at random" } else { "" }
        ),
        P::ChoosePowerToughnessAsEntersOrTurnsFaceUp { options, at_random, enters_only, .. } => {
            let options = options
                .iter()
                .map(|option| {
                    let mut text = format!("a {}/{}", option.power, option.toughness);
                    // "a 1/6 Wall artifact creature" (Primal Clay): the added
                    // subtype leads the type words.
                    for subtype in &option.added_subtypes {
                        text.push_str(&format!(" {subtype:?}"));
                    }
                    if let Some(types) = &option.card_types {
                        let names = types
                            .iter()
                            .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
                            .collect::<Vec<_>>();
                        text.push_str(&format!(" {}", names.join(" ")));
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
                    if !option.added_subtypes.is_empty() {
                        text.push_str(" in addition to its other types");
                    }
                    text
                })
                .collect::<Vec<_>>();
            // "As this creature enters, flip a coin. If the coin comes up
            // heads, this creature enters as a 5/2 creature with haste. If it
            // comes up tails, ..." (Molten Sentry): two equally likely entry
            // characteristics are a coin flip (CR 705).
            if *at_random
                && *enters_only
                && let [heads, tails] = options.as_slice()
            {
                return Some(format!(
                    "As this source enters, flip a coin. If the coin comes up heads, this source enters as {heads}. If it comes up tails, this source enters as {tails}"
                ));
            }
            format!(
                "As this source {}, it becomes {}{}",
                if *enters_only { "enters" } else { "enters or is turned face up" },
                if *at_random { "one of these at random: " } else { "your choice of " },
                or_list(&options)
            )
        }
        P::EnterAsCopyAsEnters { spec, .. } => enter_as_copy(spec),
        P::CastAnnouncementProgram {program} => format!("As you cast this spell, {}",effects(program.flattened_default_effects())),
        P::SpellResolutionDestinationProgram { origin_zone, destination, follow_up, multiplicity } => {
            let occurrence=match multiplicity {
                ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::PerInstance=>String::new(),
                ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::OncePerDeclaredKey{key}=>format!(" once for the resolution recipe named {key:?}"),
            };
            let condition = origin_zone.map(|zone| format!("If this spell was cast from your {}, ", zone_phrase(&zone))).unwrap_or_default();
            let movement = if *destination == Zone::Exile { "exile it".to_string() }
                else { format!("put it into your {}", zone_phrase(destination)) };
            format!("{condition}instead of putting it into your graveyard as it resolves, {movement}{occurrence} and {}", effects(follow_up.flattened_default_effects()))
        }
        P::AsEntersEffectProgram { program, also_turns_face_up, turns_face_up_only, transforms_into, transform_characteristics, uses_enters_with_counter_surface, entry_instead_surface, .. } => {
            if let Some(adjustment) = transform_characteristics {
                use ironsmith_core::static_ability_model::TransformCharacteristicAdjustment as Adjustment;
                let body = match adjustment {
                    Adjustment::CounterKind(kind) => format!("counters on it become {} counters instead", kind.description()),
                    Adjustment::ColorsFromNamedActivationPayment(label) => format!("for each color of mana spent to {label} it, it becomes that color"),
                };
                return Some(format!("As this permanent transforms into this face, {body}"));
            }
            let event = if *turns_face_up_only {
                "is turned face up"
            } else if *also_turns_face_up {
                "enters or is turned face up"
            } else {
                "enters"
            };
            // "As this creature enters, roll X d6. It enters with a number of
            // +1/+1 counters on it equal to the total of those results"
            // (Neverwinter Hydra): the counted value is the summed roll.
            if !*turns_face_up_only
                && !*also_turns_face_up
                && transforms_into.is_none()
                && let [roll_effect, put_effect] = program.flattened_default_effects()
                && let Some(with_id) = roll_effect.downcast_ref::<crate::effects::WithIdEffect>()
                && with_id
                    .effect
                    .downcast_ref::<crate::effects::RollDiceChooseResultEffect>()
                    .is_some_and(|roll| roll.total)
                && let Some(put) = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()
                && matches!(put.target.base(), crate::target::ChooseSpec::Source)
                && matches!(put.amount.unhinted(), crate::effect::Value::EffectValue(id) if *id == with_id.id)
            {
                let roll = effects(std::slice::from_ref(roll_effect));
                return Some(format!(
                    "As this permanent enters, {}. It enters with a number of {} counters on it equal to the total of those results",
                    roll.trim().trim_end_matches('.'),
                    describe_counter_type(put.counter_type)
                ));
            }
            // "As this creature enters, remove all counters from all
            // permanents. This creature enters with a +1/+1 counter on it for
            // each counter removed this way." (Thief of Blood)
            if *uses_enters_with_counter_surface
                && !*turns_face_up_only
                && !*also_turns_face_up
                && transforms_into.is_none()
                && let [prefix @ .., put_effect] = program.flattened_default_effects()
                && !prefix.is_empty()
                && let Some(put) = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()
                && matches!(put.target.base(), crate::target::ChooseSpec::Source)
                && counters_removed_this_way_multiplier(&put.amount) == Some(1)
            {
                return Some(format!(
                    "As this permanent enters, {}. This permanent enters with {} on it for each counter removed this way",
                    effects(prefix),
                    with_indefinite_article(&format!("{} counter", describe_counter_type(put.counter_type)))
                ));
            }
            // "As Mimeoplasm enters, exile up to X creature cards from your
            // graveyard. It enters with three +1/+1 counters on it for each
            // creature card exiled this way." The counters scale with the
            // cards this source's entry exiled, so they are part of its entry.
            if *uses_enters_with_counter_surface
                && !*turns_face_up_only
                && !*also_turns_face_up
                && transforms_into.is_none()
                && let [exile_effect, put_effect] = program.flattened_default_effects()
                && exile_effect.downcast_ref::<crate::effects::ExileEffect>().is_some()
                && let Some(put) = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()
                && matches!(put.target.base(), crate::target::ChooseSpec::Source)
                && let Value::CountScaled(filter, multiplier) = put.amount.unhinted()
                && *multiplier > 0
                && filter.zone == Some(Zone::Exile)
                && filter.card_types.as_slice() == [CardType::Creature]
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                })
            {
                let mut residual = filter.clone();
                residual.zone = None;
                residual.card_types.clear();
                residual.tagged_constraints.retain(|constraint| {
                    !(constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
                });
                residual.set_explicit_card_noun(false);
                residual.set_explicit_card_type_noun(None);
                if residual == ObjectFilter::default() {
                    let amount = u32::try_from(*multiplier)
                        .ok()
                        .and_then(ironsmith_core::cardinal_word)
                        .unwrap_or_else(|| multiplier.to_string());
                    let exiled = effects(std::slice::from_ref(exile_effect));
                    return Some(format!(
                        "As this permanent enters, {}. It enters with {amount} {} counters on it for each creature card exiled this way",
                        exiled.trim().trim_end_matches('.'),
                        describe_counter_type(put.counter_type)
                    ));
                }
            }
            // "As this creature enters, exile X creature cards from your
            // graveyard. If you can't, put this creature into its owner's
            // graveyard instead of onto the battlefield. For each creature card
            // exiled this way, this creature enters with a +2/+0, +1/+1, or
            // +0/+2 counter on it." The full-count check gates the counter
            // choices; a short exile redirects the entering source.
            if *uses_enters_with_counter_surface
                && !*turns_face_up_only
                && !*also_turns_face_up
                && transforms_into.is_none()
                && let Some(text) =
                    audit_exile_or_redirect_entry_with_counter_choice(program.flattened_default_effects())
            {
                return Some(text);
            }
            // CR 614.1c: "If this land would enter, instead sacrifice ...,
            // then put this land onto the battlefield" (Sheltered Valley).
            if *entry_instead_surface
                && !*turns_face_up_only
                && !*also_turns_face_up
                && transforms_into.is_none()
                && !program.flattened_default_effects().is_empty()
            {
                return Some(format!(
                    "If this permanent would enter, instead {}, then put this permanent onto the battlefield",
                    effects(program.flattened_default_effects())
                ));
            }
            // "As Shimatsu enters, sacrifice any number of permanents.
            // Shimatsu enters with that many +1/+1 counters on it": the
            // counters the entering permanent gets from an earlier step's
            // result are part of its entry, not a later placement.
            if !*turns_face_up_only
                && !*also_turns_face_up
                && transforms_into.is_none()
                && let [prefix @ .., put_effect] = program.flattened_default_effects()
                && let Some(producer) = prefix
                    .last()
                    .and_then(|effect| effect.downcast_ref::<crate::effects::WithIdEffect>())
                && let Some(put) = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()
                && matches!(put.target.base(), crate::target::ChooseSpec::Source)
                && matches!(put.amount.unhinted(), crate::effect::Value::EffectValue(id) if *id == producer.id)
            {
                let steps = effects(prefix);
                let steps = steps.trim().trim_end_matches('.');
                let steps = steps.strip_prefix("you choose ").map_or(steps.to_string(), |rest| format!("choose {rest}"));
                return Some(format!(
                    "As this permanent enters, {}. This permanent enters with that many {} counters on it",
                    lowercase_first(&steps),
                    describe_counter_type(put.counter_type)
                ));
            }
            // The imperative "choose" is the controller's choice.
            let body = match program.flattened_default_effects() {
                // Two independent as-enters choices by you are one
                // coordinated instruction: "choose a color and an opponent"
                // (Call to Arms).
                [first, second]
                    if first
                        .downcast_ref::<crate::effects::ChooseColorEffect>()
                        .is_some_and(|color| color.chooser == PlayerFilter::You && color.among.is_none())
                        && second
                            .downcast_ref::<crate::effects::ChoosePlayerEffect>()
                            .is_some_and(|choice| choice.chooser == PlayerFilter::You && !choice.random) =>
                {
                    let choice = |effect: &Effect| -> String {
                        let text = effects(std::slice::from_ref(effect));
                        let rest = text
                            .strip_prefix("you choose ")
                            .or_else(|| text.strip_prefix("choose "))
                            .map(str::to_string);
                        rest.unwrap_or(text)
                    };
                    format!("choose {} and {}", choice(first), choice(second))
                }
                list => effects(list),
            };
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
        // "If a red source you control would deal an amount of noncombat
        // damage less than Ojer Axonil's power to an opponent, that source
        // deals damage equal to Ojer Axonil's power instead." The floor sits
        // between the damage amount and its recipient.
        P::MinimumDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, floor, noncombat_only, ..
        } => {
            let kind = if *noncombat_only { "an amount of noncombat " } else { "an amount of " };
            let scoped = damage_scope(source_filter, target_player_filter, target_object_filter, kind);
            let unscoped = damage_scope(source_filter, &None, &None, kind);
            let recipient = scoped.strip_prefix(unscoped.as_str()).unwrap_or("");
            let floor = describe_value(floor);
            format!(
                "If {unscoped} less than {floor}{recipient}, that source deals damage equal to {floor} instead"
            )
        }
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
        // "Double all damage that sources you control of the chosen type
        // would deal" (Collective Inferno): the authored imperative over a
        // class of dealers, with no recipient restriction.
        P::DoubleDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, factor,
            combat_only: false, noncombat_only: false, double_all_damage_surface: true, ..
        } if matches!(*factor, 2 | 3)
            && damage_scope(source_filter, target_player_filter, target_object_filter, "")
                .ends_with(" would deal damage") =>
        {
            let dealers = damage_source_objects(source_filter);
            // "Double all damage that creature sources you control would
            // deal" (Absorbing Man and Titania): an unzoned creature dealer
            // is a creature source in any zone, not a creature permanent.
            let dealers = if source_filter.zone.is_none()
                && source_filter.card_types.as_slice() == [CardType::Creature]
            {
                dealers.replacen("creature", "creature source", 1)
            } else {
                dealers
            };
            let dealers = pluralize_relative_object_phrase(strip_indefinite_article(&dealers));
            let verb = if *factor == 2 { "Double" } else { "Triple" };
            format!("{verb} all damage that {dealers} would deal")
        }
        P::DoubleDamageAmountReplacement {
            source_filter, target_player_filter, target_object_filter, factor, combat_only, noncombat_only, ..
        } => format!(
            "If {}, it deals {} instead",
            damage_scope(source_filter, target_player_filter, target_object_filter,
                if *combat_only { "combat " } else if *noncombat_only { "noncombat " } else { "" }),
            match *factor { 2 => "double that damage".to_string(), 3 => "triple that damage".to_string(), n => format!("{} that much damage", multiple(n)) }
        ),
        // "If this creature would deal combat damage to a creature, prevent
        // that damage and that creature's owner shuffles it into their
        // library" (Weeping Angel): the shield's scope, then its follow-up.
        P::DamagePreventionWithFollowUp { source_filter, target_filter, combat_only, effects: body, .. } => format!(
            "If {} would deal {}damage to {}, prevent that damage and {}",
            damage_source_objects(source_filter),
            match combat_only { Some(true) => "combat ", Some(false) => "noncombat ", None => "" },
            article(&objects(target_filter)),
            effects(body),
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
                    (true, true) => "permanent or player".to_string(),
                    (false, true) => "player".to_string(),
                    // "on that artifact" / "on that creature": a recipient of
                    // one card type is named by it.
                    _ => match filter.card_types.as_slice() {
                        [card_type] if filter.all_card_types.is_empty() => card_type.name().to_string(),
                        _ => "permanent".to_string(),
                    },
                };
                let counters = counter_type.map_or("of each of those kinds of counters".to_string(), |c| {
                    format!("{} counters", describe_counter_type(c))
                });
                return Some(format!(
                    "If you would put {kind} counters on {}, put {} that many {counters} on that {noun} instead",
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
        // "If you would get one or more counters, you get that many plus one
        // of each of those kinds of counters instead" (Winding Constrictor):
        // a player placement replacement applies to that player only; the
        // runtime ignores the object filter when `player_filter` is set.
        P::AddCountersPlacementReplacement {
            player_filter: Some(player_filter), counter_type, additional, cause: None, ..
        } if *additional > 0 => {
            let who = player(player_filter);
            let get = if who == "you" { "get" } else { "gets" };
            let (kind, added) = match counter_type {
                Some(counter) => {
                    let kind = describe_counter_type(*counter);
                    (
                        format!("{kind} "),
                        format!(
                            "{} {kind} counter{}",
                            count_words(*additional as u32),
                            if *additional == 1 { "" } else { "s" }
                        ),
                    )
                }
                None => (
                    String::new(),
                    format!("{} of each of those kinds of counters", count_words(*additional as u32)),
                ),
            };
            format!(
                "If {who} would get one or more {kind}counters, {who} {get} that many plus {added} instead"
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
        // "If you would get one or more poison counters, instead you get one
        // poison counter and you can't get additional poison counters this
        // turn": the first counters of the turn are capped at one.
        P::PlayerCounterPerTurnLimitReplacement { player_filter: PlayerFilter::You, counter_type, maximum: 1, .. } => {
            let kind = describe_counter_type(*counter_type);
            format!(
                "If you would get one or more {kind} counters, instead you get one {kind} counter and you can't get additional {kind} counters this turn"
            )
        }
        P::PlayerCounterPerTurnLimitReplacement { player_filter, counter_type, maximum, .. } => format!(
            "{} can't get more than {maximum} {} counters each turn",
            capitalize_first(&player(player_filter)),
            describe_counter_type(*counter_type)
        ),
        P::DoubleTokenCreationReplacement { controller: PlayerFilter::Any, .. } =>
            "If one or more tokens would be created, twice that many of those tokens are created instead".to_string(),
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
        P::NamedActionReplacement { label, source_filter, performer_filter, object_requirement, replacement_effects, optional, .. } => {
            let subject = match performer_filter {
                Some(performer) if *source_filter == ObjectFilter::default() => player(performer),
                Some(performer) => format!("{} with {}", player(performer), article(&objects(source_filter))),
                None => article(&objects(source_filter)),
            };
            let subjects=object_requirement.as_ref().map(|requirement|format!(" with at least {} {}",requirement.minimum,objects(&requirement.filter))).unwrap_or_default();
            format!("If {subject} would perform the {label} action{subjects}, {}instead {}", may(*optional), effects(replacement_effects))
        }
        P::KeywordActionReplacement { action, source_filter, performer_filter: None, replacement_effects, optional, .. } => {
            let effects_text = effects(replacement_effects);
            let effects_text = effects_text.replacen(". It ", ", then it ", 1);
            // "If you would proliferate, proliferate twice instead": an
            // untyped source you control is your own player action.
            if *source_filter
                == (ObjectFilter::from(ironsmith_core::filter_model::ObjectFilterFields {
                    controller: Some(PlayerFilter::You),
                    ..ironsmith_core::filter_model::ObjectFilterFields::from(ObjectFilter::default())
                }))
            {
                return Some(format!(
                    "If you would {}, {}{effects_text} instead",
                    sr(action).to_ascii_lowercase(),
                    may(*optional),
                ));
            }
            // "If a Rigger you control would assemble a Contraption, it
            // assembles two Contraptions instead": the replacement repeats
            // the replaced action a number of times.
            if let [repeat] = replacement_effects.as_slice()
                && let Some(emit) = repeat.downcast_ref::<crate::effects::EmitKeywordActionEffect>()
                && emit.action == *action
                && emit.amount > 1
                && *action == ironsmith_core::KeywordActionKind::AssembleContraption
            {
                return Some(format!(
                    "If {} would {}, {}it assembles {} Contraptions instead",
                    article(&objects(source_filter)),
                    action.infinitive(),
                    may(*optional),
                    count_words(emit.amount),
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
            // "If you would planeswalk, instead look at ..., then planeswalk"
            // (Susan Foreman): the substitute program ends by performing the
            // replaced action once.
            if !*optional
                && let [rest @ .., last] = replacement_effects.as_slice()
                && !rest.is_empty()
                && let Some(emit) = last.downcast_ref::<crate::effects::EmitKeywordActionEffect>()
                && emit.action == *action
                && emit.amount == 1
                && emit.object_tags.is_empty()
            {
                return Some(format!(
                    "If {subject} would {}, instead {}, then {}",
                    action.infinitive(),
                    effects(rest),
                    action.infinitive()
                ));
            }
            format!(
                "If {subject} would {}, {}{} instead",
                action.infinitive(),
                may(*optional),
                effects(replacement_effects)
            )
        }
        // "The first time you would draw a card each turn except the first
        // card you draw during each of your draw steps, you draw four cards
        // instead" (Reed Richards): the draw-count gate the first-draw
        // replacement builds reads as the printed first-draw window.
        P::ConditionalDrawReplacement { condition, replacement_effects, optional: false, except_first_of_draw_step, .. }
            if is_first_draw_each_turn_gate(condition, *except_first_of_draw_step) =>
        {
            let except = if *except_first_of_draw_step {
                " except the first card you draw during each of your draw steps"
            } else {
                ""
            };
            // "instead look at the top two cards of your library. Put one of
            // them into your graveyard and the other back on top of your
            // library. Then draw a card." (Scion of Halaster): the substitute
            // program's closing draw is the replaced drawer's, who is you
            // under this gate.
            if let [prefix @ .., last] = replacement_effects.as_slice()
                && prefix.len() > 1
                && let Some(draw) = last.downcast_ref::<crate::effects::DrawCardsEffect>()
                && draw.player == PlayerFilter::IteratedPlayer
            {
                let mut own_draw = draw.clone();
                own_draw.player = PlayerFilter::You;
                format!(
                    "The first time you would draw a card each turn{except}, instead {}. Then {}",
                    effects(prefix),
                    effects(&[Effect::new(own_draw)])
                )
            } else if replacement_effects.len() > 1 {
                format!(
                    "The first time you would draw a card each turn{except}, instead {}",
                    effects(replacement_effects)
                )
            } else {
                format!(
                    "The first time you would draw a card each turn{except}, {} instead",
                    effects(replacement_effects)
                )
            }
        }
        P::ConditionalDrawReplacement { condition, replacement_effects, optional, except_first_of_draw_step, .. } => {
            // A timing window reads as the window ("during your draw step").
            let condition = describe_condition(condition);
            let when = match condition.strip_prefix("timing restriction: ") {
                Some(window) => format!(" {window}"),
                None => format!(" while {condition}"),
            };
            // "instead return a creature card from your graveyard to the
            // battlefield. If you can't, you lose the game." (Out of the
            // Tombs): the final step is a didn't-happen fallback on the
            // replacement step just before it.
            let fallback = match replacement_effects.as_slice() {
                [prefix @ .., last] if !prefix.is_empty() => last
                    .downcast_ref::<crate::effects::IfEffect>()
                    .filter(|fallback| {
                        fallback.predicate == EffectPredicate::DidNotHappen
                            && fallback.else_.is_empty()
                            && !fallback.then.is_empty()
                            && !fallback.per_player_result
                            && prefix
                                .last()
                                .and_then(|step| step.downcast_ref::<crate::effects::WithIdEffect>())
                                .is_some_and(|step| step.id == fallback.condition)
                    })
                    .map(|fallback| (prefix, fallback)),
                _ => None,
            };
            // "instead you may skip that draw. If you do, until your next
            // turn, you can't be attacked ..." (Island Sanctuary): a lone
            // restriction is the consequence of the skipped draw.
            let skip_then_consequence = *optional
                && matches!(
                    replacement_effects.as_slice(),
                    [only] if only.downcast_ref::<crate::effects::CantEffect>().is_some()
                );
            if skip_then_consequence {
                format!(
                    "If you would draw a card{}{when}, instead you may skip that draw. If you do, {}",
                    except_first(*except_first_of_draw_step),
                    effects(replacement_effects)
                )
            } else if let Some((prefix, fallback)) = fallback {
                format!(
                    "If you would draw a card{}{when}, {}instead {}. If you can't, {}",
                    except_first(*except_first_of_draw_step),
                    may(*optional),
                    effects(prefix),
                    effects(&fallback.then)
                )
            } else if matches!(
                replacement_effects.as_slice(),
                [only] if only.downcast_ref::<crate::effects::SearchLibraryEffect>().is_some()
            ) {
                // "you may instead search your library for a card, put that
                // card into your hand, then shuffle" (Archmage Ascension): a
                // multi-step search replacement leads with "instead".
                format!(
                    "If you would draw a card{}{when}, {}instead {}",
                    except_first(*except_first_of_draw_step),
                    may(*optional),
                    effects(replacement_effects)
                )
            } else if let Some(body) = audit_chosen_kind_reveal_until(replacement_effects) {
                format!(
                    "If you would draw a card{}{when}, {}instead {body}",
                    except_first(*except_first_of_draw_step),
                    may(*optional)
                )
            } else {
            format!(
                "If you would draw a card{}{when}, {}{} instead",
                except_first(*except_first_of_draw_step),
                may(*optional),
                effects(replacement_effects)
            )
            }
        }
        P::CreateOneOfEachTokenReplacement { kinds, .. } => {
            // The executor matches exactly these token kinds under your
            // control and adds the other kinds once per matched token.
            let kinds = kinds.iter().map(|kind| match kind {
                ironsmith_core::AdditionalTokenKind::Clue => "Clue".to_string(),
                ironsmith_core::AdditionalTokenKind::Food => "Food".to_string(),
                ironsmith_core::AdditionalTokenKind::Treasure => "Treasure".to_string(),
                ironsmith_core::AdditionalTokenKind::Squirrel => "Squirrel".to_string(),
            }).collect::<Vec<_>>();
            format!("If you would create {} token, instead create one of each", with_indefinite_article(&or_list(&kinds)))
        },
        // The drawer skips the draw and you draw a card instead.
        P::RedirectDrawReplacement { drawer, except_first_of_draw_step, .. } => format!(
            "If {} would draw a card{}, instead that player skips that draw and you draw a card",
            player(drawer),
            except_first_for(*except_first_of_draw_step, drawer)
        ),
        // "If a player would draw a card, they reveal it instead. Then any
        // other player may pay 2 life. ..." (Zur's Weirding): another drawer
        // revealing the top card of their own library reveals the card the
        // draw would take; only that reveal replaces the draw, and the rest
        // follows it.
        P::DrawReplacementWithEffects {
            drawer,
            except_first_of_draw_step,
            replacement_effects,
            cycling_source: None,
            ..
        } if !matches!(drawer, PlayerFilter::You)
            && replacement_effects.len() > 1
            && replacement_effects[0]
                .downcast_ref::<crate::effects::RevealTopEffect>()
                .is_some_and(|reveal| reveal.player == PlayerFilter::IteratedPlayer) =>
        {
            format!(
                "If {} would draw a card{}, they reveal it instead. Then {}",
                player(drawer),
                except_first_for(*except_first_of_draw_step, drawer),
                effects(&replacement_effects[1..])
            )
        }
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
        // CR 609.4b any-type permissions printed as their own line.
        P::ManaSpendPermission { permission, .. }
            if permission.mode == ironsmith_core::value_model::ManaSpendMode::AnyType
                && permission.mana_source_filter.is_none()
                && permission.any_color_mana_symbol.is_none()
                && permission.mana_symbol_as.is_none()
                && !permission.other_mana_only_as_colorless
                && match (&permission.player, &permission.scope) {
                    // "You can spend mana of any type to cast creature
                    // spells" (Vizier of the Menagerie).
                    (PlayerFilter::You, ironsmith_core::ManaSpendScope::CastingSpellsMatching(filter)) => {
                        filter.zone.is_none()
                            && filter.any_of.iter().all(|branch| branch.zone.is_none())
                    }
                    // "Mana of any type can be spent to activate Sharkey's
                    // abilities" (Sharkey, Tyrant of the Shire).
                    (PlayerFilter::Any, ironsmith_core::ManaSpendScope::ActivationCostsOf(filter)) => {
                        filter.source
                    }
                    _ => false,
                } =>
        {
            match &permission.scope {
                ironsmith_core::ManaSpendScope::ActivationCostsOf(filter) => format!(
                    "Mana of any type can be spent to activate {}'s abilities",
                    objects(filter)
                ),
                scope => format!(
                    "You can spend mana of any type{}",
                    mana_spend_scope(scope)
                ),
            }
        }
        P::ManaSpendPermission { permission, .. } => mana_spend_permission(permission),
        P::TargetingAsThoughNoAbility(spec) => targeting_as_though_no_ability(spec),
        P::PreventDamageToSelfPutCountersInstead { counter_type, .. } => format!(
            "If damage would be dealt to this permanent, put that many {} counters on it instead",
            describe_counter_type(*counter_type)
        ),
        P::HealDamageBeforeDamageToSelf { .. } => {
            "If damage would be dealt to this permanent, instead that damage is dealt, but all other damage already dealt to it is healed".into()
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
            "If {} would deal {}damage to {}, put that many {} counters on {} instead",
            damage_source_objects(source_filter),
            match combat_only { Some(true) => "combat ", Some(false) => "noncombat ", None => "" },
            objects(target_filter),
            describe_counter_type(*counter_type),
            if target_filter.card_types == [CardType::Creature] { "that creature" } else { "that permanent" }
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
        P::AddLifeGainReplacement { player: who, additional, loss, .. } => {
            let verb=if *loss { "lose" } else { "gain" };
            let quantity=if *loss { "that much life" } else { "that much" };
            format!("If {} would {verb} life, they {verb} {quantity} plus {additional} instead", player(who))
        },
        P::TokenCreationTemplates { controller, token_filter, templates, mode, choose_one, optional, surface, .. } => {
            // AppendOnce adds one of each template to the event; the other
            // modes add (AppendForEach) or substitute (ReplaceEach) one per
            // matching token created.
            use ironsmith_core::TokenCreationTemplateMode as Mode;
            let created = pluralize_relative_object_phrase(strip_indefinite_article(&objects(token_filter)));
            // "you may instead choose a creature other than Esix and create
            // that many tokens that are copies of that creature": the copied
            // object is a non-target choice made by the replacement.
            if let [template] = templates.as_slice()
                && matches!(mode, Mode::ReplaceEach)
                && *optional
                && !*choose_one
                && matches!(controller, PlayerFilter::You)
                && let Some(chosen) = audit_chosen_other_token_copy_template(template)
            {
                return Some(format!(
                    "If you would create one or more {created}, you may instead choose {chosen} other than this permanent and create that many tokens that are copies of that creature"
                ));
            }
            let per_token = !matches!(mode, Mode::AppendOnce);
            let added = templates
                .iter()
                .map(|template| {
                    let text = effects(std::slice::from_ref(template));
                    let text = text.strip_prefix("create ").unwrap_or(&text).to_string();
                    if !per_token {
                        // "those tokens plus an additional Map token"
                        // (Worldwalker Helm): the authored "additional".
                        if surface.additional_word {
                            return with_indefinite_article(&format!(
                                "additional {}",
                                strip_indefinite_article(&text)
                            ));
                        }
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
            let added = match (*choose_one, *optional) {
                // "you may instead create that many X or that many Y": the
                // optional choice already covers which one.
                (true, true) => or_list(&added),
                (true, false) => format!("your choice of {}", or_list(&added)),
                (false, _) => added.join(" and "),
            };
            let result = match mode {
                Mode::ReplaceEach => added,
                Mode::AppendOnce | Mode::AppendForEach => format!("those tokens plus {added}"),
            };
            // "If you would create a Fish token, create a 3/3 blue Shark
            // creature token instead" (Fisher's Talent) / "If you would
            // create one or more artifact tokens, instead create those tokens
            // plus an additional Map token" (Worldwalker Helm): the authored
            // active creation clause.
            if surface.active && !*optional && matches!(controller, PlayerFilter::You) {
                let source = if surface.singular_source {
                    with_indefinite_article(strip_indefinite_article(&objects(token_filter)))
                } else {
                    format!("one or more {created}")
                };
                // "If you would create a Food token, instead create a Food
                // token and a Treasure token" (Bilbo): one replaced token
                // reads as single tokens, not "that many".
                let result = if surface.singular_source && matches!(mode, Mode::ReplaceEach) {
                    let singles = templates
                        .iter()
                        .map(|template| {
                            let text = effects(std::slice::from_ref(template));
                            text.strip_prefix("create ").unwrap_or(&text).to_string()
                        })
                        .collect::<Vec<_>>();
                    if *choose_one {
                        format!("your choice of {}", or_list(&singles))
                    } else {
                        singles.join(" and ")
                    }
                } else {
                    result
                };
                return Some(if surface.leading_instead {
                    format!("If you would create {source}, instead create {result}")
                } else {
                    format!("If you would create {source}, create {result} instead")
                });
            }
            if *optional && matches!(controller, PlayerFilter::You) {
                format!("If you would create one or more {created}, you may instead create {result}")
            } else if *optional {
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
        P::GenericManaTapPayment { spells, activated_abilities, exclude_source, name, .. } => {
            let mut programs = Vec::new();
            if *spells { programs.push("For each generic mana in this spell's total cost, you may tap an untapped creature you control rather than pay that mana".to_string()); }
            if *activated_abilities { programs.push(format!("For each generic mana in the total cost of this permanent's abilities, you may tap {} untapped creature you control rather than pay that mana", if *exclude_source {"another"} else {"an"})); }
            if let Some(name)=name { programs.push(format!("This generic tap payment is named {name}")); }
            programs.join(". ")
        }
        P::GenericManaTapPaymentProhibition { name } => format!("This creature can't be tapped to pay the generic tap payment named {name}"),
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
        P::FirstCoinBatchHeadsWin { player: who } => {
            let you = matches!(who, PlayerFilter::You);
            format!(
                "The first time {} {} one or more coins each turn, those coins come up heads and {} {} those flips",
                player(who),
                if you { "flip" } else { "flips" },
                if you { "you" } else { "that player" },
                if you { "win" } else { "wins" }
            )
        }
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
            event: ironsmith_core::ReplacedEventSpec::LifeGain { player: who, from_own_spell_or_ability: false },
            replacement_effects, optional: false, ..
        } if replacement_effects.is_empty() => format!(
            "If {} would gain life, that player gains no life instead", player(who)),
        P::EventReplacementWithEffects {
            event: ironsmith_core::ReplacedEventSpec::SourceDestructionRegenerates,
            replacement_effects,
            ..
        } if replacement_effects.is_empty() => "If this source would be destroyed, regenerate it".to_string(),
        P::EventReplacementWithEffects {
            event: ironsmith_core::ReplacedEventSpec::DamageToPlayer {
                player: who,
                source_filter: None,
                combat_only,
            },
            replacement_effects,
            optional,
            ..
        } => format!(
            "If {}damage would be dealt to {}, {}{} instead",
            if *combat_only { "combat " } else { "" },
            player(who),
            may(*optional),
            effects(replacement_effects)
        ),
        // "If your life total would be reduced to 0 or less, instead transform
        // this creature and your life total becomes 3. Then if this creature
        // didn't transform this way, you lose the game." (Enduring Angel): the
        // transform and the life-total change are one compound substitute; the
        // checked follow-up is its own sentence.
        P::EventReplacementWithEffects { event, replacement_effects, optional: false, .. }
            if matches!(event, ironsmith_core::ReplacedEventSpec::LifeReducedToZeroOrLess { .. })
                && replacement_effects.len() >= 3
                && super::render_effects::unwrap_basic_tag_wrappers(&replacement_effects[0])
                    .downcast_ref::<crate::effects::TransformEffect>()
                    .is_some()
                && super::render_effects::unwrap_basic_tag_wrappers(&replacement_effects[1])
                    .downcast_ref::<crate::effects::SetLifeTotalEffect>()
                    .is_some() =>
        {
            let rest = audit_untransformed_follow_up(&replacement_effects[0], &replacement_effects[2..])
                .unwrap_or_else(|| effects(&replacement_effects[2..]));
            let rest = if rest.starts_with("then ") { rest } else { format!("then {rest}") };
            format!(
                "If {}, instead {} and {}. {}",
                replaced_event(event),
                effects(&replacement_effects[..1]),
                effects(&replacement_effects[1..2]),
                capitalize_first(&rest)
            )
        }
        // "If an opponent who controls at least as many lands as you do would
        // put a land onto the battlefield, ..." (Land Equilibrium).
        P::EventReplacementWithEffects {
            event: ironsmith_core::ReplacedEventSpec::ZoneChange {
                object,
                from: None,
                to: Some(Zone::Battlefield),
            },
            replacement_effects,
            optional: false,
            ..
        } if audit_parity_gated_entry_then_sacrifice(object, replacement_effects).is_some() => {
            audit_parity_gated_entry_then_sacrifice(object, replacement_effects).unwrap_or_default()
        }
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
            format!("Unlock costs you pay cost {{{generic_reduction}}} less")
        }
        P::SourceHasFullTextOfTopGraveyardCard { filter, abilities, .. } => {
            let mut card = filter.clone();
            card.zone = None;
            card.set_explicit_card_noun(true);
            let card = with_indefinite_article(strip_indefinite_article(&objects(&card)).trim());
            let granted = abilities.iter().map(ability_model).collect::<Vec<_>>();
            let rider = if granted.is_empty() {
                String::new()
            } else {
                format!(" and has the text {}", granted.join(" and "))
            };
            format!(
                "As long as the top card of your graveyard is {card}, this permanent has the full text of that card{rider}"
            )
        }
        P::RevealActionOffer { label, source_zones, max_per_player_turn, arrival_zone, effects: program, .. } => {
            let zones = if source_zones.len() == 2 && source_zones.contains(&Zone::Hand) && source_zones.contains(&Zone::Library) {
                "a hidden zone".to_string()
            } else { source_zones.iter().map(zone_phrase).collect::<Vec<_>>().join(" or ") };
            let limit = max_per_player_turn.map_or(String::new(), |maximum| if maximum == 1 { " once each turn".into() } else { format!(" at most {maximum} times each turn") });
            format!("As you reveal this card from {zones}, you may perform the {label} action{limit} ending in {arrival_zone} as follows: [{}]", effects(program))
        },
        P::CopySourceSelectionReplacement {player:actor,replacement,..} => {
            let mut operand=replacement;while let Some(child)=operand.transparent_child_effect(){operand=child;}
            let copy=operand.downcast_ref::<crate::effects::CopyFromDefinitionEffect>()?;
            format!("If {} would copy a spell or cast a copy, they copy the card named {} instead of anything else",player(actor),copy.definition.card.name)
        },
        P::CardCreationSelectionReplacement {operation_label,player:actor,replacement,..} => {
            let mut template=replacement;
            while let Some(tagged)=template.downcast_ref::<crate::effects::TaggedEffect>(){template=&tagged.effect;}
            let Some(create)=template.downcast_ref::<crate::effects::CreateCardsEffect>() else{return None;};
            let ironsmith_core::CardCreationSelection::Named(name)=&create.selection else{return None;};
            format!("If {} would create real cards{}, use the supplied card definition named {} instead",player(actor),operation_label.as_ref().map_or(String::new(),|label|format!(" as part of the named {label} action")),name)
        },
        P::MaySkipDrawStepWithFollowup { if_you_do, .. } => format!(
            "If you would begin your draw step, you may skip that step instead. If you do, {}",
            effects(if_you_do)
        ),
        P::PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn { player: who, .. } => format!(
            "{} may sacrifice a permanent to ignore this permanent's effect until end of turn",
            capitalize_first(&player(who))
        ),
        // CR 601.2c: only this spell's own target announcement is limited;
        // the resolution recheck ignores it (Enthralling Hold, Dream Leash).
        P::CastTargetRestriction { forbidden, .. } => format!(
            "You can't choose {} as this spell's target as you cast it",
            article(&objects(forbidden))
        ),
        P::EntersAttachedToChosenObject { filter, .. } => format!(
            "As this enters, choose {} it could be attached to. If you do, it enters attached to that {}",
            article(&objects(filter)),
            if filter.card_types.as_slice() == [CardType::Creature] { "creature" } else { "permanent" }
        ),
        P::AdditionalSacrificePerManaSymbol { activated_abilities, symbol, sacrifice, .. } => {
            // The payer is implied by the quoted cost ("Sacrifice a Swamp").
            let mut sacrificed = sacrifice.clone();
            sacrificed.controller = None;
            let (subject, verb, costs) = if *activated_abilities {
                ("Activated abilities", "activate", "activation costs")
            } else {
                ("Spells", "cast", "mana costs")
            };
            format!(
                "{subject} cost an additional \"Sacrifice {}\" to {verb} for each {} mana symbol in their {costs}",
                objects(&sacrificed),
                mana_symbol_name(symbol)
            )
        }
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
        // "The first time you would create one or more tokens each turn, you
        // may instead create ..." (Moonlit Meditation; Mirrormind Crown adds
        // "As long as this Equipment is attached to a creature").
        P::Conditional { ability, condition, .. }
            if matches!(
                &ability.payload,
                P::TokenCreationTemplates { controller: PlayerFilter::You, optional: true, .. }
            ) && (audit_no_token_created_this_turn(condition)
                || matches!(condition, crate::ConditionExpr::And(left, right)
                    if audit_no_token_created_this_turn(left) || audit_no_token_created_this_turn(right)))
                && audit_payload_reading(ability)
                    .is_some_and(|inner| inner.starts_with("If you would create one or more tokens, ")) =>
        {
            let inner = audit_payload_reading(ability).unwrap_or_default();
            let body = inner.strip_prefix("If you would create one or more tokens, ").unwrap_or(&inner);
            let first = format!("the first time you would create one or more tokens each turn, {body}");
            match condition {
                crate::ConditionExpr::And(left, right) => {
                    let other = if audit_no_token_created_this_turn(left) { right } else { left };
                    // "The first time you would create one or more tokens
                    // during each of your turns" (Esix, Fractal Bloom).
                    if matches!(other.as_ref(), crate::ConditionExpr::YourTurn) {
                        format!(
                            "The first time you would create one or more tokens during each of your turns, {body}"
                        )
                    } else {
                        format!("As long as {}, {first}", describe_condition(other))
                    }
                }
                _ => capitalize_first(&first),
            }
        }
        // "Solved — Creatures you control get +1/+0." (Case of the Gateway
        // Express): a Case's solved static keeps its printed label.
        P::Conditional { ability, condition, .. }
            if matches!(condition, crate::ConditionExpr::SourceCaseSolved)
                && (ability.id == Some(crate::static_abilities::StaticAbilityId::Anthem)
                    || matches!(&ability.payload, P::TokenCreationTemplates { .. })) =>
        {
            let inner = audit_payload_reading(ability).unwrap_or_else(|| {
                crate::static_abilities::StaticAbility::from_model(ability.as_ref().clone()).display()
            });
            format!("Solved — {}", capitalize_first(inner.trim().trim_end_matches('.')))
        }
        // "As long as enchanted creature isn't legendary, it doesn't untap
        // during its controller's untap step" (Stunning Strike): a negated
        // attached-object supertype gate on a restriction of that object.
        P::Conditional { ability, condition: crate::ConditionExpr::Not(negated), .. }
            if audit_attached_supertype_exclusion(negated).is_some()
                && audit_attached_restriction_subject_rest(ability).is_some() =>
        {
            let excluded = audit_attached_supertype_exclusion(negated).unwrap_or_default();
            let (subject, rest) = audit_attached_restriction_subject_rest(ability).unwrap_or_default();
            format!("As long as {subject} isn't {excluded}, it {rest}")
        }
        P::Conditional {
            ability,
            condition: crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(filter),
                comparison: ironsmith_core::Comparison::Equal(n),
                ..
            },
            ..
        } if ability.id == Some(crate::static_abilities::StaticAbilityId::LegendRuleDoesntApply)
            && matches!(ability.payload, P::None)
            && filter.name.is_some()
            && filter.zone == Some(Zone::Battlefield)
            && filter.controller.is_none() =>
        {
            let amount = ironsmith_core::cardinal_word(*n as u32).unwrap_or_else(|| n.to_string());
            format!(
                "If there are exactly {amount} {} on the battlefield, the \"legend rule\" doesn't apply to them",
                pluralize_noun_phrase(strip_indefinite_article(&objects(filter)))
            )
        }
        // "As long as this Equipment is attached to a creature, your
        // opponents can't cast spells during your turn" (Conqueror's Flail):
        // the attachment gate leads; the your-turn half is the ban's window.
        P::Conditional { ability, condition: crate::ConditionExpr::And(attached, timing), .. }
            if ability.id == Some(crate::static_abilities::StaticAbilityId::OpponentsCantCastSpells)
                && matches!(ability.payload, P::None)
                && matches!(timing.as_ref(),
                    crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringYourTurn))
                && matches!(attached.as_ref(), crate::ConditionExpr::AttachedToSourceMatches(filter)
                    if { let mut bare = filter.clone(); bare.zone = Some(Zone::Battlefield); bare == ObjectFilter::creature() }) =>
        {
            "As long as this source is attached to a creature, your opponents can't cast spells during your turn"
                .to_string()
        }
        // "You may cast this card from your graveyard if a non-Zombie creature
        // died this turn. If you do, it enters with a +1/+1 counter on it"
        // (Undead Sprinter): the gate qualifies the permission itself, before
        // its cast rider.
        P::Conditional { ability, condition, .. }
            if matches!(&ability.payload,
                P::Grants(spec) if matches!(spec.grantable, ironsmith_core::Grantable::PlayFrom))
                && capitalize_first(audit_conditional_inner_reading(ability).trim())
                    .starts_with("You may cast this card from ") =>
        {
            let inner = capitalize_first(audit_conditional_inner_reading(ability).trim());
            let inner = inner.trim_end_matches('.');
            let condition = describe_condition(condition);
            let gate = condition.strip_prefix("as long as ").unwrap_or(&condition);
            match inner.split_once(". ") {
                Some((permission, rider)) => format!("{permission} if {gate}. {rider}"),
                None => format!("{inner} if {gate}"),
            }
        }
        // "Enchanted creature doesn't untap during your untap step if this
        // Aura has a pupa counter on it" (Cocoon): an attached object's untap
        // restriction gated to your turn applies during your untap step.
        P::Conditional { ability, condition: crate::ConditionExpr::And(left, right), .. }
            if matches!(
                &ability.payload,
                P::RuleRestriction {
                    restriction: crate::effect::Restriction::Untap(_),
                    additional_restrictions,
                    ..
                } if additional_restrictions.is_empty()
            ) && (matches!(left.as_ref(), crate::ConditionExpr::YourTurn)
                != matches!(right.as_ref(), crate::ConditionExpr::YourTurn))
                && audit_attached_restriction_subject_rest(ability).is_some() =>
        {
            let (subject, _) = audit_attached_restriction_subject_rest(ability).unwrap_or_default();
            let gate = if matches!(left.as_ref(), crate::ConditionExpr::YourTurn) { right } else { left };
            format!(
                "{} doesn't untap during your untap step as long as {}",
                capitalize_first(&subject),
                describe_condition(gate)
            )
        }
        // "You can't play lands if this creature was cast this turn" (Rock
        // Jockey): a rule gated on the source's own cast this turn reads with
        // "if" — a past cast is not a lasting state.
        P::Conditional { ability, condition: condition @ crate::ConditionExpr::And(left, right), .. }
            if matches!(&ability.payload, P::RuleRestriction { .. })
                && matches!(
                    left.as_ref(),
                    crate::ConditionExpr::TurnHistory(
                        ironsmith_core::TurnHistoryCondition::SourceWasCast { .. }
                    )
                )
                && matches!(
                    right.as_ref(),
                    crate::ConditionExpr::TurnHistory(
                        ironsmith_core::TurnHistoryCondition::SourceEnteredBattlefieldThisTurn { .. }
                    )
                ) =>
        {
            let inner = audit_payload_reading(ability).unwrap_or_else(|| {
                crate::static_abilities::StaticAbility::from_model(ability.as_ref().clone()).display()
            });
            format!("{} if {}", inner.trim().trim_end_matches('.'), describe_condition(condition))
        }
        P::Conditional { ability, condition, .. } => {
            let inner = audit_payload_reading(ability).unwrap_or_else(|| {
                crate::static_abilities::StaticAbility::from_model(ability.as_ref().clone()).display()
            });
            // "During your main phase, each card in your graveyard has beam me
            // up. ..." (Scotty, Miracle Worker): the main-phase gate leads the
            // rule it scopes.
            if matches!(condition, crate::ConditionExpr::SourceControllersMainPhase) {
                return Some(format!(
                    "During your main phase, {}",
                    lowercase_first(inner.trim().trim_end_matches('.'))
                ));
            }
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
                // "As long as you control your commander, activated abilities
                // of cards in your graveyard cost {2} less to activate. This
                // effect can't reduce ..." (Convergence of Dominion): a gate
                // over a multi-sentence rule scopes all of it, so it leads.
                Some(rest)
                    if trimmed.contains(". ")
                        && !trimmed.contains(" as long as ")
                        && !trimmed.contains('"') =>
                {
                    format!("As long as {rest}, {}", lowercase_first(trimmed.trim_end_matches('.')))
                }
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
            "As you create your deck, circle {} of the colors below",
            count_words(*count)
        ),
        P::CommanderDeckAllowance { allowance, .. } => allowance.describe(),
        P::Toxic { amount } => format!("Toxic {amount}"),
        // CR 123.8: a power/toughness sticker row and its ticket cost.
        P::TicketPowerToughnessSticker { tickets, power, toughness } => format!(
            "{} \u{2014} {power}/{toughness}",
            ironsmith_core::static_ability_model::ticket_cost_symbols(*tickets)
        ),
        P::MustAttackDraftNotedPlayers { card_name } => format!(
            "This creature attacks a player you noted for cards named {card_name} each combat if able"
        ),
        // CR 905: typed draft-time instructions read in printed order; the
        // card's own name is resolved by the audit assembly ("~").
        P::DraftRule { rule } if rule.actions != 0 && !rule.unmodeled => {
            let mut parts = Vec::new();
            if rule.face_up {
                parts.push("Draft this card face up.".to_string());
            }
            if rule.revealed
                && !rule
                    .draft_actions()
                    .any(|action| action == ironsmith_core::DraftAction::RevealAndNoteDraftedCount)
            {
                parts.push("Reveal this card as you draft it.".to_string());
            }
            for action in rule.draft_actions() {
                parts.push(action.reading("~"));
                // "... you may draft an additional card from that booster
                // pack. If you do, put this card into that booster pack."
                if rule.exchange_pick && action == ironsmith_core::DraftAction::DraftAdditionalCard {
                    parts.push("If you do, put this card into that booster pack.".to_string());
                }
            }
            parts.join(" ").trim_end_matches('.').to_string()
        }
        P::DraftRule { rule } => {
            let mut parts = Vec::new();
            if rule.face_up {
                parts.push("draft this card face up".to_string());
            }
            // "Reveal this card as you draft it and note the player who
            // passed it to you." (Cogwork Tracker): one instruction.
            if rule.revealed && rule.notes_passing_player {
                parts.push(
                    "reveal this card as you draft it and note the player who passed it to you"
                        .to_string(),
                );
            } else {
                if rule.revealed {
                    parts.push("this card is revealed as it's drafted".to_string());
                }
                if rule.notes_passing_player {
                    parts.push("its drafter notes the player who passed it".to_string());
                }
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
                M::NoteExiledCardManaValues => "Note the mana value of each card as it's put into exile".into(),
                M::ClassTriggerChoiceAdjustment { action,class,.. } => format!("As you {action}, you may choose another {class} ability to trigger."),
                M::ClassTriggerMultiplication { class,once_each_turn,.. } => format!("{}If you would trigger a {class} ability of a permanent you control, you may have that ability trigger twice instead.",if *once_each_turn { "Once each turn, " } else { "" }),
                M::PermanentCountGrantsDesignation { minimum, historic_only, enduring_story, tapped_only, named_designation, history_requirements } => if !history_requirements.is_empty() { format!("If you {}, you are in your {} for the rest of the game.", history_requirements.iter().map(|requirement| requirement.clause()).collect::<Vec<_>>().join(", "), named_designation.as_deref().unwrap_or("designation")) } else { format!("If you control {minimum} or more {}, you {} for the rest of the game.", if *historic_only { "artifacts, legendaries, and/or Sagas" } else if *tapped_only { "tapped permanents" } else { "permanents" }, if let Some(name) = named_designation { format!("are in your {name}") } else if *enduring_story { "have an enduring story".into() } else { "get the city's blessing".into() }) },
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
                M::DamageCausesRecipientControllerLifeLoss => "Damage dealt to this creature also causes its controller to lose that much life.".to_string(),
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
                M::PrecombatLoreProgression { stop_at_final_chapter } => format!("As your precombat main phase begins, put a lore counter on this permanent as a turn-based action{}.", if *stop_at_final_chapter { ", unless it has at least as many lore counters as its final chapter number" } else { "" }),
                M::FinalChapterResolutionRemovesCounters { counter_name } => format!("As this Saga's final chapter ability resolves, remove all {counter_name} counters from it."),
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
                M::ContributesMultipliedPower {
                    stations,
                    stations_spacecraft_only,
                    crews,
                    saddles,
                    multiplier,
                } => {
                    let mut actions = Vec::new();
                    if *stations {
                        actions.push(if *stations_spacecraft_only {
                            "stations Spacecraft"
                        } else {
                            "stations permanents"
                        });
                    }
                    if *saddles {
                        actions.push("saddles Mounts");
                    }
                    if *crews {
                        actions.push("crews Vehicles");
                    }
                    let actions = match actions.as_slice() {
                        [] => String::new(),
                        [only] => (*only).to_string(),
                        [init @ .., last] => format!("{} and {last}", init.join(", ")),
                    };
                    let multiplied = match multiplier {
                        2 => "doubled".to_string(),
                        3 => "tripled".to_string(),
                        4 => "quadrupled".to_string(),
                        other => format!("multiplied by {other}"),
                    };
                    format!("This creature {actions} as though its power were {multiplied}")
                }
                M::LoyaltyCounterPaysCrewCost => {
                    "You may remove a loyalty counter from a planeswalker you control rather than pay this Vehicle's crew cost".to_string()
                }
            }
        }
        // CR 103.2 pregame action over the drafted pool (Caller of the
        // Untamed, Arcane Savant, Volatile Chimera): read from the typed
        // card types and count bounds.
        P::PregameAction {
            kind: ironsmith_core::PregameActionKind::ExileDraftedCardsBeforeShuffle(spec),
            ..
        } => {
            let types = spec
                .card_types
                .iter()
                .map(|card_type| card_type.name().to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" or ");
            let noun = if types.is_empty() { "card".to_string() } else { format!("{types} card") };
            let (amount, plural) = match (spec.minimum, spec.maximum) {
                (1, Some(1)) => (with_indefinite_article(&noun), false),
                (minimum, None) => (format!("{} or more {noun}s", count_words(minimum)), true),
                (minimum, Some(maximum)) if minimum == maximum => {
                    (format!("{} {noun}s", count_words(minimum)), true)
                }
                (minimum, Some(maximum)) => (
                    format!("between {} and {} {noun}s", count_words(minimum), count_words(maximum)),
                    true,
                ),
            };
            let verb = if plural { "aren't" } else { "isn't" };
            format!(
                "Before you shuffle your deck to start the game, you may reveal this card from your deck and exile {amount} you drafted that {verb} in your deck"
            )
        }
        // The pregame color choice is the commander's (CR 903.3): every
        // printing of this kind gates it on "if this is your commander".
        P::PregameAction { kind: ironsmith_core::PregameActionKind::ChooseColor, .. } => {
            "If this is your commander, choose a color before the game begins".to_string()
        }
        _ => return None,
    })
}

/// "you didn't create a token this turn": no token created by you so far.
fn audit_no_token_created_this_turn(condition: &crate::ConditionExpr) -> bool {
    let crate::ConditionExpr::ValueComparison { left, operator, right } = condition else {
        return false;
    };
    let Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::TokensCreatedMatching { player, filter }) =
        left.unhinted()
    else {
        return false;
    };
    let mut bare = filter.clone();
    bare.zone = None;
    bare.token = false;
    *player == PlayerFilter::You
        && *operator == crate::effect::ValueComparisonOperator::Equal
        && right.unhinted() == &Value::Fixed(0)
        && bare == ObjectFilter::default()
}

/// "When you cast an Equipment spell this way, it gains "When this Equipment
/// enters, attach it to target creature you control."" (Galea, Kindler of
/// Hope): a permission's recipient rider read from its typed holder filter
/// and granted ability.
/// "Elemental permanent spells you cast from your hand gain evoke {4} as you
/// cast them" (Ashling, the Limitless): a granted evoke alternative cost
/// (CR 702.74a) over permanent spells cast from your hand. The evoked
/// permanent's sacrifice is the keyword's own rider, so it is not restated.
fn audit_granted_hand_evoke(grant: &crate::grant::GrantSpec) -> Option<String> {
    let ironsmith_core::Grantable::AlternativeCast(method) = &grant.grantable else {
        return None;
    };
    if method.name() != "Evoke"
        || method.cast_from_zone() != Zone::Hand
        || grant.zone != Zone::Hand
        || !grant.additional_zones.is_empty()
        || grant.permanent_this_way_grants.len() != 1
        || !grant.cast_this_way_grants.is_empty()
        || grant.filter.owner != Some(PlayerFilter::You)
        || grant.filter.excluded_card_types.as_slice() != [CardType::Land]
        || grant.filter.subtypes.is_empty()
        || ![CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Planeswalker]
            .iter()
            .all(|card_type| grant.filter.card_types.contains(card_type))
    {
        return None;
    }
    let cost = method.total_cost()?.mana_cost()?.to_oracle();
    let kinds = grant
        .filter
        .subtypes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!(
        "{kinds} permanent spells you cast from your hand gain evoke {cost} as you cast them"
    ))
}

fn audit_permanent_this_way_riders(grant: &crate::grant::GrantSpec) -> String {
    let mut text = String::new();
    for granted in &grant.permanent_this_way_grants {
        let Some(model) = granted.compiled_model() else { continue };
        let P::GrantObjectAbilityForFilter(rider) = &model.payload else { continue };
        if rider.condition.is_some() || !rider.additional_abilities.is_empty() {
            continue;
        }
        let noun = rider
            .filter
            .subtypes
            .iter()
            .map(ToString::to_string)
            .chain(rider.filter.card_types.iter().map(|card_type| card_type.to_string().to_ascii_lowercase()))
            .collect::<Vec<_>>()
            .join(" ");
        let spell = if noun.is_empty() { "spell".to_string() } else { format!("{noun} spell") };
        let ability = ability_model_for_recipient(&rider.ability, &rider.filter);
        text.push_str(&format!(
            ". When you cast {} this way, it gains {ability}",
            with_indefinite_article(&spell)
        ));
    }
    text
}

/// "Once during each of your turns, you may play a land or cast a spell from
/// among the cards exiled with this permanent without paying its mana cost"
/// (Hauken's Insight): a free cast over the source's linked exile, read from
/// the typed origin, free method, source-exiled link, use limit and land
/// companion.
fn audit_source_exiled_free_cast_grant(grant: &crate::grant::GrantSpec) -> Option<String> {
    let ironsmith_core::Grantable::AlternativeCast(method) = &grant.grantable else {
        return None;
    };
    if grant.zone != Zone::Exile
        || method.cast_from_zone() != Zone::Exile
        || method.mana_cost().is_some()
        || !method.non_mana_costs().is_empty()
        || method.cast_condition().is_some()
        || !grant.additional_zones.is_empty()
        || !grant.cast_this_way_grants.is_empty()
        || !grant.permanent_this_way_grants.is_empty()
    {
        return None;
    }
    let mut spell = grant.filter.clone();
    let linked = spell.tagged_constraints.len();
    // "from among cards exiled with Maralen this turn": the linked exile
    // narrowed to the cards you exiled with this source this turn.
    let this_turn = spell.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == ironsmith_core::tag::SOURCE_EXILED_BY_YOU_THIS_TURN_TAG
    });
    spell.tagged_constraints.retain(|constraint| {
        constraint.tag.as_str() != ironsmith_core::tag::SOURCE_EXILED_TAG
            && constraint.tag.as_str() != ironsmith_core::tag::SOURCE_EXILED_BY_YOU_THIS_TURN_TAG
    });
    if linked != 1 + usize::from(this_turn) || !spell.tagged_constraints.is_empty() {
        return None;
    }
    spell.zone = None;
    spell.excluded_card_types.retain(|card_type| *card_type != CardType::Land);
    let spell = if spell == ObjectFilter::default() {
        "a spell".to_string()
    } else {
        with_indefinite_article(&super::normalize_common::describe_cast_spell_filter(
            &spell,
            super::normalize_common::CastSpellFilterContext::Standalone,
        ))
    };
    let lead = match grant.usage_limit {
        None => "",
        Some(ironsmith_core::GrantUsageLimit::OnceEachTurn) => "Once each turn, ",
        Some(ironsmith_core::GrantUsageLimit::OnceDuringEachOfYourTurns) => "Once during each of your turns, ",
        Some(ironsmith_core::GrantUsageLimit::DuringYourTurns) => "During your turn, ",
    };
    let land = if grant.land_play_companion { "play a land or " } else { "" };
    Some(capitalize_first(&format!(
        "{lead}{} may {land}cast {spell} from among {} without paying its mana cost",
        player(&grant.beneficiary),
        if this_turn {
            "cards exiled with this permanent this turn"
        } else {
            "the cards exiled with this permanent"
        }
    )))
}


/// "Once during each of your turns, you may cast an instant or sorcery spell
/// from your hand without paying its mana cost" (Zaffai and the Tempests);
/// "... cast a spell from your hand or the top of your library ..." (One with
/// the Multiverse): a use-limited free cast of one of your own cards. The
/// unlimited form ("You may cast spells from your hand without paying their
/// mana costs", Omniscience) keeps its plural reading.
/// "Once each turn, you may play a historic land or cast a historic spell
/// from the top of your library. When you do, create a Food token." (The
/// Fourth Doctor): a use-limited play permission over a historic land branch
/// and a historic nonland branch, with an effect each use performs.
fn audit_library_historic_play_with_reflexive(grant: &crate::grant::GrantSpec) -> Option<String> {
    if !matches!(grant.grantable, ironsmith_core::Grantable::PlayFrom)
        || grant.zone != Zone::Library
        || !matches!(grant.usage_limit, Some(ironsmith_core::GrantUsageLimit::OnceEachTurn))
        || grant.beneficiary != PlayerFilter::You
        || grant.on_use_effects.is_empty()
        || !grant.additional_zones.is_empty()
        || !grant.cast_this_way_grants.is_empty()
        || !grant.permanent_this_way_grants.is_empty()
        || grant.max_plays.is_some()
    {
        return None;
    }
    let [first, second] = grant.filter.any_of.as_slice() else {
        return None;
    };
    let bare = |branch: &ObjectFilter| {
        let mut rest = branch.clone();
        rest.historic = false;
        rest.zone = None;
        rest.union_surface = Default::default();
        rest
    };
    let land = |branch: &ObjectFilter| {
        let mut rest = bare(branch);
        branch.historic
            && rest.card_types.as_slice() == [CardType::Land]
            && {
                rest.card_types.clear();
                rest == ObjectFilter::default()
            }
    };
    let nonland = |branch: &ObjectFilter| {
        let mut rest = bare(branch);
        branch.historic
            && rest.excluded_card_types.as_slice() == [CardType::Land]
            && {
                rest.excluded_card_types.clear();
                rest == ObjectFilter::default()
            }
    };
    if !((land(first) && nonland(second)) || (land(second) && nonland(first))) {
        return None;
    }
    Some(format!(
        "Once each turn, you may play a historic land or cast a historic spell from the top of your library. When you do, {}",
        effects(&grant.on_use_effects)
    ))
}

fn audit_limited_hand_free_cast_grant(grant: &crate::grant::GrantSpec) -> Option<String> {
    let ironsmith_core::Grantable::AlternativeCast(method) = &grant.grantable else {
        return None;
    };
    let lead = match grant.usage_limit {
        None => return None,
        Some(ironsmith_core::GrantUsageLimit::OnceEachTurn) => "Once each turn, ",
        Some(ironsmith_core::GrantUsageLimit::OnceDuringEachOfYourTurns) => "Once during each of your turns, ",
        Some(ironsmith_core::GrantUsageLimit::DuringYourTurns) => return None,
    };
    if grant.zone != Zone::Hand
        || method.cast_from_zone() != Zone::Hand
        || method.mana_cost().is_some()
        || !method.non_mana_costs().is_empty()
        || method.cast_condition().is_some()
        || grant.beneficiary != PlayerFilter::You
        || grant.max_plays.is_some()
        || !grant.cast_this_way_grants.is_empty()
        || !grant.permanent_this_way_grants.is_empty()
        || !grant.on_use_effects.is_empty()
        || grant.instant_timing
        || grant.land_play_companion
        || grant.cast_this_way_filter.is_some()
        || grant.cast_mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }
    let origin = match grant.additional_zones.as_slice() {
        [] => "from your hand",
        [Zone::Library] if grant.top_card_only => "from your hand or the top of your library",
        _ => return None,
    };
    let mut spell = grant.filter.clone();
    spell.zone = None;
    if spell.owner == Some(PlayerFilter::You) {
        spell.owner = None;
    }
    spell.excluded_card_types.retain(|card_type| *card_type != CardType::Land);
    let spell = if spell == ObjectFilter::default() {
        "a spell".to_string()
    } else {
        with_indefinite_article(&super::normalize_common::describe_cast_spell_filter(
            &spell,
            super::normalize_common::CastSpellFilterContext::Standalone,
        ))
    };
    Some(format!("{lead}you may cast {spell} {origin} without paying its mana cost"))
}

/// "legendary" for an attached-object match that names only one supertype.
fn audit_attached_supertype_exclusion(condition: &crate::ConditionExpr) -> Option<String> {
    let crate::ConditionExpr::AttachedToSourceMatches(filter) = condition else {
        return None;
    };
    let [supertype] = filter.supertypes.as_slice() else {
        return None;
    };
    let mut rest = filter.clone();
    rest.supertypes.clear();
    rest.zone = None;
    (rest == ObjectFilter::default()).then(|| supertype.name().to_ascii_lowercase())
}

/// The attached subject ("enchanted creature") and the rest of a single
/// rule restriction's typed reading ("doesn't untap during ...").
fn audit_attached_restriction_subject_rest(
    ability: &crate::static_abilities::CompiledStaticAbility,
) -> Option<(String, String)> {
    let P::RuleRestriction { restriction, additional_restrictions, .. } = &ability.payload else {
        return None;
    };
    if !additional_restrictions.is_empty() {
        return None;
    }
    let reading = lowercase_first(audit_rule_restriction_reading(restriction).trim());
    let reading = reading.trim_end_matches('.');
    ["enchanted creature", "enchanted permanent", "equipped creature"]
        .iter()
        .find_map(|subject| {
            let rest = reading.strip_prefix(subject)?.strip_prefix(' ')?;
            (!rest.is_empty()).then(|| (subject.to_string(), rest.to_string()))
        })
}

/// "choose land or nonland and reveal cards from the top of your library
/// until you reveal a card of the chosen kind. Put that card into your hand
/// and put all other cards revealed this way on the bottom of your library
/// in any order" (Abundance, Pavel Chekov): a land / nonland choice whose
/// two modes each reveal to the first match of that kind, put it into your
/// hand, and put the rest on the bottom in one shared order.
fn audit_chosen_kind_reveal_until(replacement: &[Effect]) -> Option<String> {
    let [choice] = replacement else { return None; };
    let choice = choice.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if !choice.common_prefix_effects.is_empty() {
        return None;
    }
    let [land_mode, nonland_mode] = choice.modes.as_slice() else { return None; };
    let mode_order = |mode_effects: &[Effect], land_kind: bool| -> Option<ironsmith_core::LibraryBottomOrder> {
        let [consult, keep, rest] = mode_effects else { return None; };
        let consult = consult.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
        let keep = keep.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        let rest = rest.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
        let kind_matches = if land_kind {
            consult.filter.card_types.as_slice() == [CardType::Land]
                && consult.filter.excluded_card_types.is_empty()
        } else {
            consult.filter.excluded_card_types.as_slice() == [CardType::Land]
                && consult.filter.card_types.is_empty()
        };
        (kind_matches
            && consult.player == PlayerFilter::You
            && consult.mode == crate::effects::consult_helpers::LibraryConsultMode::Reveal
            && matches!(consult.stop_rule, crate::effects::ConsultTopOfLibraryStopRule::FirstMatch)
            && consult.max_exposed.is_none()
            && keep.zone == Zone::Hand
            && matches!(keep.target.base(), crate::target::ChooseSpec::Tagged(_))
            && rest.tag == consult.all_tag
            && rest.keep_tagged.as_ref() == Some(&consult.match_tag)
            && rest.player == PlayerFilter::You)
            .then_some(rest.order)
    };
    let order = mode_order(land_mode.effects.as_slice(), true)?;
    if mode_order(nonland_mode.effects.as_slice(), false)? != order {
        return None;
    }
    Some(format!(
        "choose land or nonland and reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and put all other cards revealed this way on the bottom of your library in {} order",
        match order {
            ironsmith_core::LibraryBottomOrder::Random => "a random",
            ironsmith_core::LibraryBottomOrder::ChooserChooses => "any",
        }
    ))
}

/// "then if this creature didn't transform this way, you lose the game"
/// (Enduring Angel): an empty receipt of the substitute's own tagged
/// transform names that transform, not a count of objects.
fn audit_untransformed_follow_up(transform: &Effect, rest: &[Effect]) -> Option<String> {
    let tagged = transform.downcast_ref::<crate::effects::TaggedEffect>()?;
    let transformed = tagged.effect.downcast_ref::<crate::effects::TransformEffect>()?;
    let [follow_up] = rest else {
        return None;
    };
    let conditional = follow_up.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.is_empty() {
        return None;
    }
    let crate::effect::Condition::ValueComparison {
        left,
        operator: crate::effect::ValueComparisonOperator::Equal,
        right,
    } = &conditional.condition
    else {
        return None;
    };
    if !matches!(right.unhinted(), crate::effect::Value::Fixed(0)) {
        return None;
    }
    let crate::effect::Value::TaggedSnapshotCount { tag, .. } = left.unhinted() else {
        return None;
    };
    if *tag != tagged.tag {
        return None;
    }
    Some(format!(
        "then if {} didn't transform this way, {}",
        describe_transform_target(&transformed.target),
        effects(&conditional.if_true)
    ))
}

/// The reading of a conditional static's inner ability: its payload reading,
/// or the engine display when the family has none (as the generic
/// `Conditional` arm reads it).
fn audit_conditional_inner_reading(ability: &crate::static_abilities::CompiledStaticAbility) -> String {
    audit_payload_reading(ability).unwrap_or_else(|| {
        crate::static_abilities::StaticAbility::from_model(ability.clone()).display()
    })
}

/// The chosen object of a token-copy replacement template that copies one
/// non-target "other" creature: "a creature" (read with "other than this
/// permanent"). Any entry-state or count rider declines.
fn audit_chosen_other_token_copy_template(template: &Effect) -> Option<String> {
    let copy = template.downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    let crate::target::ChooseSpec::WithCount(inner, count) = &copy.target else {
        return None;
    };
    let crate::target::ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if count.min != 1
        || count.max != Some(1)
        || !matches!(copy.count.unhinted(), Value::Fixed(1))
        || copy.enters_tapped
        || copy.has_haste
        || copy.enters_attacking
        || !filter.other
        || filter.card_types.as_slice() != [CardType::Creature]
        || !filter.tagged_constraints.is_empty()
    {
        return None;
    }
    let mut chosen = filter.clone();
    chosen.other = false;
    Some(with_indefinite_article(strip_indefinite_article(&objects(&chosen))))
}

/// "If Ugin's Nexus would be put into a graveyard from the battlefield,
/// instead exile it and ...": a replacement of the source's own
/// battlefield-to-graveyard move on a card that is not a creature. CR 700.4
/// keeps "dies" for creatures, so the zone move is named. The reading keeps
/// the generic replacement shape and the "a this" self-subject that the audit
/// assembly resolves to the card's own subject.
pub(crate) fn audit_noncreature_self_graveyard_replacement(
    model: &crate::static_abilities::CompiledStaticAbility,
    card_types: &[CardType],
) -> Option<String> {
    if card_types.is_empty() || card_types.contains(&CardType::Creature) {
        return None;
    }
    let P::EventReplacementWithEffects {
        event:
            ironsmith_core::ReplacedEventSpec::ZoneChange {
                object,
                from: Some(Zone::Battlefield),
                to: Some(Zone::Graveyard),
            },
        replacement_effects,
        optional,
        ..
    } = &model.payload
    else {
        return None;
    };
    if !object.source || replacement_effects.is_empty() {
        return None;
    }
    let subject = article(&objects(object));
    if subject.contains("creature") {
        return None;
    }
    Some(format!(
        "If {subject} would be put into a graveyard from the battlefield, {}{} instead",
        may(*optional),
        effects(replacement_effects)
    ))
}

/// The program of an exile-X-as-enters cost whose shortfall redirects the
/// entering source to its owner's graveyard and whose full exile gives the
/// source one counter of a chosen kind per exiled card (Frankenstein's
/// Monster): `[Tagged(Exile X), Conditional(count == X, [ForEachTagged
/// [ChooseMode(put one counter of kind K on this)]], [RedirectEnteringSource
/// (Graveyard)])]`.
fn audit_exile_or_redirect_entry_with_counter_choice(program_effects: &[Effect]) -> Option<String> {
    let [exile_effect, conditional_effect] = program_effects else {
        return None;
    };
    let mut exile_inner: &Effect = exile_effect;
    while let Some(tagged) = exile_inner.downcast_ref::<crate::effects::TaggedEffect>() {
        exile_inner = &*tagged.effect;
    }
    let exile = exile_inner.downcast_ref::<crate::effects::ExileEffect>()?;
    let crate::target::ChooseSpec::Object(exiled_filter) = exile.spec.base() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [redirect] = conditional.if_false.as_slice() else {
        return None;
    };
    if redirect
        .downcast_ref::<crate::effects::RedirectEnteringSourceEffect>()?
        .destination
        != Zone::Graveyard
    {
        return None;
    }
    let [for_each] = conditional.if_true.as_slice() else {
        return None;
    };
    let for_each = for_each.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [choice] = for_each.effects.as_slice() else {
        return None;
    };
    let choice = choice.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let kinds = choice
        .modes
        .iter()
        .map(|mode| {
            let [put] = mode.effects.as_slice() else {
                return None;
            };
            let put = put.downcast_ref::<crate::effects::PutCountersEffect>()?;
            (matches!(put.target.base(), crate::target::ChooseSpec::Source)
                && matches!(put.amount.unhinted(), Value::Fixed(1)))
            .then(|| describe_counter_type(put.counter_type))
        })
        .collect::<Option<Vec<_>>>()?;
    let kinds = match kinds.as_slice() {
        [first, second] => format!("{first} or {second}"),
        [rest @ .., last] if rest.len() >= 2 => format!("{}, or {last}", rest.join(", ")),
        _ => return None,
    };
    let noun = if exiled_filter.card_types.as_slice() == [CardType::Creature] {
        "creature card"
    } else {
        "card"
    };
    let exiled = effects(std::slice::from_ref(exile_effect));
    Some(format!(
        "As this permanent enters, {}. If you can't, put this permanent into its owner's graveyard instead of onto the battlefield. For each {noun} exiled this way, this permanent enters with {} on it",
        exiled.trim().trim_end_matches('.'),
        with_indefinite_article(&format!("{kinds} counter"))
    ))
}

/// "choose two abilities from among first strike, vigilance, and lifelink"
/// (Greymond, Avacyn's Stalwart): the parser records each chosen combination
/// as one named option ("first strike and vigilance", ...). The listed set
/// and the count are read back only when the options are exactly every
/// combination of that set, in listed order.
fn listed_combination_options(options: &[String]) -> Option<(Vec<String>, usize)> {
    fn extend(
        start: usize,
        count: usize,
        picks: usize,
        current: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == picks {
            out.push(current.clone());
            return;
        }
        for index in start..count {
            current.push(index);
            extend(index + 1, count, picks, current, out);
            current.pop();
        }
    }
    let parts = options
        .iter()
        .map(|option| option.split(" and ").collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let picks = parts.first()?.len();
    if picks < 2
        || parts.iter().any(|option| {
            option.len() != picks || option.iter().any(|item| item.is_empty() || item.contains(','))
        })
    {
        return None;
    }
    let mut items: Vec<String> = Vec::new();
    for option in &parts {
        for item in option {
            if !items.iter().any(|known| known == item) {
                items.push(item.to_string());
            }
        }
    }
    if items.len() <= picks {
        return None;
    }
    let mut combinations = Vec::new();
    extend(0, items.len(), picks, &mut Vec::new(), &mut combinations);
    let expected = combinations
        .iter()
        .map(|combination| {
            combination
                .iter()
                .map(|index| items[*index].as_str())
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .collect::<Vec<_>>();
    (expected.as_slice() == options).then_some((items, picks))
}

/// "If an opponent who controls at least as many lands as you do would put a
/// land onto the battlefield, that player instead puts that land onto the
/// battlefield then sacrifices a land of their choice." (Land Equilibrium):
/// the entry of a land an opponent owns is replaced, while that player
/// controls at least as many of those permanents as you, by the same entry
/// followed by that player's sacrifice of one they control; otherwise the
/// entry happens unchanged.
fn audit_parity_gated_entry_then_sacrifice(
    object: &ObjectFilter,
    replacement: &[Effect],
) -> Option<String> {
    use crate::effect::Value;
    if object.owner != Some(PlayerFilter::Opponent) || object.zone.is_some() {
        return None;
    }
    let [gate] = replacement else {
        return None;
    };
    let gate = gate.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::ValueComparison {
        left,
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right,
    } = &gate.condition
    else {
        return None;
    };
    let (Value::Count(theirs), Value::Count(yours)) = (left.unhinted(), right.unhinted()) else {
        return None;
    };
    let mut counted = object.clone();
    counted.owner = None;
    let counts_controlled_by = |filter: &ObjectFilter, player: PlayerFilter| {
        let mut expected = counted.clone();
        expected.zone = Some(Zone::Battlefield);
        expected.controller = Some(player);
        *filter == expected
    };
    if counted == ObjectFilter::default()
        || !counts_controlled_by(theirs, PlayerFilter::IteratedPlayer)
        || !counts_controlled_by(yours, PlayerFilter::You)
    {
        return None;
    }
    let puts_it_onto_battlefield = |effect: &Effect| {
        effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|movement| {
                movement.zone == Zone::Battlefield
                    && matches!(movement.target.base(), crate::target::ChooseSpec::Tagged(_))
            })
    };
    let [otherwise] = gate.if_false.as_slice() else {
        return None;
    };
    let [instead] = gate.if_true.as_slice() else {
        return None;
    };
    let instead = instead.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [put, choose, sacrifice] = instead.effects.as_slice() else {
        return None;
    };
    let choose = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()?;
    if !puts_it_onto_battlefield(otherwise)
        || !puts_it_onto_battlefield(put)
        || choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count.random
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || sacrifice.player != PlayerFilter::IteratedPlayer
        || !matches!(sacrifice.count.unhinted(), Value::Fixed(1))
        || !sacrifice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == choose.tag)
    {
        return None;
    }
    let mut sacrificed = choose.filter.clone();
    sacrificed.zone = None;
    sacrificed.controller = None;
    let noun = objects(&counted);
    let noun = strip_indefinite_article(&noun).to_string();
    let sacrificed = objects(&sacrificed);
    Some(format!(
        "If an opponent who controls at least as many {} as you do would put {} onto the battlefield, that player instead puts that {noun} onto the battlefield then sacrifices {} of their choice",
        pluralize_noun_phrase(&noun),
        with_indefinite_article(&noun),
        with_indefinite_article(strip_indefinite_article(&sacrificed)),
    ))
}

/// "During each player's turn, that player may play a land or cast a spell
/// from among cards exiled with this enchantment, and they may spend mana as
/// though it were mana of any color to cast that spell. When they do, exile
/// the top card of their library." (Share the Spoils): the active player's
/// once-a-turn play from the source's linked exile, its spending rider, and
/// the effect run when the permission is used.
fn audit_active_player_source_exiled_play_refill(grant: &crate::grant::GrantSpec) -> Option<String> {
    if !matches!(grant.grantable, ironsmith_core::Grantable::PlayFrom)
        || grant.zone != Zone::Exile
        || grant.beneficiary != PlayerFilter::Active
        || grant.usage_limit != Some(ironsmith_core::GrantUsageLimit::OnceEachTurn)
        || grant.cast_mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::AnyColor
        || !grant.additional_zones.is_empty()
        || !grant.cast_this_way_grants.is_empty()
        || !grant.permanent_this_way_grants.is_empty()
        || grant.max_plays.is_some()
        || grant.may_look_at_linked_exile
        || grant.source_exiled_surface.is_some()
    {
        return None;
    }
    let mut pool = grant.filter.clone();
    if pool.zone != Some(Zone::Exile) {
        return None;
    }
    pool.zone = None;
    let [constraint] = pool.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || constraint.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
    {
        return None;
    }
    pool.tagged_constraints.clear();
    if pool != ObjectFilter::default() {
        return None;
    }
    let [refill] = grant.on_use_effects.as_slice() else {
        return None;
    };
    let refill = refill.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if refill.player != PlayerFilter::You
        || refill.face_down
        || !matches!(refill.count.unhinted(), crate::effect::Value::Fixed(1))
    {
        return None;
    }
    Some(
        "During each player's turn, that player may play a land or cast a spell from among cards exiled with this permanent, and they may spend mana as though it were mana of any color to cast that spell. When they do, exile the top card of their library"
            .to_string(),
    )
}


/// A granted ability bundle that is exactly one counter keyword's printed
/// abilities ("This creature enters with a +1/+1 counter on it" plus the
/// modular dies trigger) reads as the keyword ("modular 1").
fn granted_counter_keyword(ability: &AbilityModel, additional: &[AbilityModel]) -> Option<String> {
    if additional.is_empty() {
        return None;
    }
    let abilities = std::iter::once(ability)
        .chain(additional.iter())
        .map(|ability| crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(ability))
        .collect::<Vec<_>>();
    let (keyword, consumed) = super::ast_render::describe_structural_counter_keyword_bundle(&abilities)?;
    (consumed == abilities.len()).then(|| keyword.to_ascii_lowercase())
}

/// "Other nontoken artifact creatures and Vehicles you control" (Blaster,
/// Combat DJ): a conjunctive two-branch set whose `other` and controller are
/// shared names that scope once, around the joined branch nouns.
fn conjunctive_controlled_set_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() != 2
        || !filter.has_conjunctive_set_surface()
        || filter.controller != Some(PlayerFilter::You)
    {
        return None;
    }
    let mut shell = filter.clone();
    shell.any_of.clear();
    shell.other = false;
    shell.controller = None;
    shell.zone = None;
    shell.union_surface = Default::default();
    if shell != ObjectFilter::default() {
        return None;
    }
    let branches = filter
        .any_of
        .iter()
        .map(|branch| {
            if branch.controller.is_some() || branch.owner.is_some() || branch.other || !branch.any_of.is_empty() {
                return None;
            }
            Some(pluralize_relative_object_phrase(strip_indefinite_article(&objects(branch))))
        })
        .collect::<Option<Vec<_>>>()?;
    let [first, second] = branches.as_slice() else {
        return None;
    };
    Some(if filter.other {
        format!("Other {first} and {second} you control")
    } else {
        format!("{} and {second} you control", capitalize_first(first))
    })
}
