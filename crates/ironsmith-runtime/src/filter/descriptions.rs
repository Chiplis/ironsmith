use super::*;

pub(super) fn describe_filter_union_list(
    mut parts: Vec<String>,
    connective: ObjectFilterUnionConnective,
    serial_or: bool,
) -> String {
    match parts.as_slice() {
        [] => return String::new(),
        [single] => return single.clone(),
        [first, second] => {
            let joiner = match connective {
                ObjectFilterUnionConnective::Or => "or",
                ObjectFilterUnionConnective::AndOr => "and/or",
            };
            return format!("{first} {joiner} {second}");
        }
        _ => {}
    }
    if connective == ObjectFilterUnionConnective::Or && !serial_or {
        return parts.join(" or ");
    }
    let last = parts.pop().expect("union list has at least three parts");
    let joiner = match connective {
        ObjectFilterUnionConnective::Or => "or",
        ObjectFilterUnionConnective::AndOr => "and/or",
    };
    format!("{}, {joiner} {last}", parts.join(", "))
}

#[allow(dead_code)]
pub(super) fn describe_simple_any_of_keyword_clause(
    any_of: &[ObjectFilter],
    connective: ObjectFilterUnionConnective,
) -> Option<String> {
    if any_of.len() < 2 {
        return None;
    }

    let mut labels = Vec::new();
    for filter in any_of {
        if !filter.any_of.is_empty() {
            return None;
        }

        let mut stripped = filter.clone();
        stripped.static_abilities.clear();
        stripped.excluded_static_abilities.clear();
        stripped.ability_markers.clear();
        stripped.excluded_ability_markers.clear();
        if stripped != ObjectFilter::default() {
            return None;
        }

        if filter.static_abilities.len() == 1 && filter.ability_markers.is_empty() {
            let label = describe_filter_static_ability(filter.static_abilities[0])?;
            labels.push(label.to_string());
            continue;
        }
        if filter.ability_markers.len() == 1 && filter.static_abilities.is_empty() {
            labels.push(filter.ability_markers[0].to_ascii_lowercase());
            continue;
        }

        return None;
    }

    Some(describe_filter_union_list(labels, connective, false))
}

pub(super) fn plus_minus_counter_delta(
    counters: &std::collections::HashMap<CounterType, u32>,
) -> i32 {
    let plus = counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0) as i32;
    let minus = counters
        .get(&CounterType::MinusOneMinusOne)
        .copied()
        .unwrap_or(0) as i32;
    plus - minus
}

pub(super) fn object_base_power_for_filter(object: &Object) -> Option<i32> {
    if let Some(power) = object.power() {
        return Some(power - plus_minus_counter_delta(&object.counters));
    }
    object.base_power.as_ref().map(|pt| pt.base_value())
}

pub(super) fn object_base_toughness_for_filter(object: &Object) -> Option<i32> {
    if let Some(toughness) = object.toughness() {
        return Some(toughness - plus_minus_counter_delta(&object.counters));
    }
    object.base_toughness.as_ref().map(|pt| pt.base_value())
}

pub(super) fn resolve_object_power_for_filter(
    object: &Object,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_power_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                game.calculated_power(object.id).or_else(|| object.power())
            } else {
                object.power()
            }
        }
    }
}

pub(super) fn resolve_layered_object_power_for_filter(
    object: &Object,
    chars: Option<&CalculatedCharacteristics>,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_power_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                chars
                    .and_then(|chars| chars.power)
                    .or_else(|| game.calculated_power(object.id))
                    .or_else(|| object.power())
            } else {
                object.power()
            }
        }
    }
}

pub(super) fn resolve_layered_object_toughness_for_filter(
    object: &Object,
    chars: Option<&CalculatedCharacteristics>,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_toughness_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                chars
                    .and_then(|chars| chars.toughness)
                    .or_else(|| game.calculated_toughness(object.id))
                    .or_else(|| object.toughness())
            } else {
                object.toughness()
            }
        }
    }
}

pub(super) fn snapshot_base_power_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
) -> Option<i32> {
    if let Some(power) = snapshot.power {
        return Some(power - plus_minus_counter_delta(&snapshot.counters));
    }
    snapshot.base_power
}

pub(super) fn snapshot_base_toughness_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
) -> Option<i32> {
    if let Some(toughness) = snapshot.toughness {
        return Some(toughness - plus_minus_counter_delta(&snapshot.counters));
    }
    snapshot.base_toughness
}

pub(super) fn resolve_snapshot_power_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    reference: PtReference,
) -> Option<i32> {
    match reference {
        PtReference::Effective => snapshot.power,
        PtReference::Base => snapshot_base_power_for_filter(snapshot),
    }
}

pub(super) fn resolve_snapshot_toughness_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    reference: PtReference,
) -> Option<i32> {
    match reference {
        PtReference::Effective => snapshot.toughness,
        PtReference::Base => snapshot_base_toughness_for_filter(snapshot),
    }
}

pub(super) fn attacking_defending_player_for_object(
    object_id: ObjectId,
    game: &crate::game_state::GameState,
) -> Option<PlayerId> {
    let combat = game.combat.as_ref()?;
    let target = crate::combat_state::get_attack_target(combat, object_id)?;
    match target {
        crate::combat_state::AttackTarget::Player(player_id) => Some(*player_id),
        crate::combat_state::AttackTarget::Planeswalker(planeswalker_id) => game
            .object(*planeswalker_id)
            .map(|object| game.controller_of(object)),
        crate::combat_state::AttackTarget::Battle(battle_id) => game.battle_protector(*battle_id),
    }
}

pub(super) fn attacking_player_for_object(
    object_id: ObjectId,
    game: &crate::game_state::GameState,
) -> Option<PlayerId> {
    let combat = game.combat.as_ref()?;
    match crate::combat_state::get_attack_target(combat, object_id)? {
        crate::combat_state::AttackTarget::Player(player_id) => Some(*player_id),
        crate::combat_state::AttackTarget::Planeswalker(_) => None,
        crate::combat_state::AttackTarget::Battle(_) => None,
    }
}

#[allow(dead_code)]
pub(super) fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "a player's".to_string(),
        PlayerFilter::You => "your".to_string(),
        PlayerFilter::NotYou => "a non-you player's".to_string(),
        PlayerFilter::Opponent => "an opponent's".to_string(),
        PlayerFilter::Teammate => "a teammate's".to_string(),
        PlayerFilter::Active => "the active player's".to_string(),
        PlayerFilter::Defending => "the defending player's".to_string(),
        PlayerFilter::Attacking => "an attacking player's".to_string(),
        PlayerFilter::DamagedPlayer => "that player's".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell's".to_string(),
        PlayerFilter::Specific(_) => "that player's".to_string(),
        PlayerFilter::MostLifeTied => "the chosen player's".to_string(),
        PlayerFilter::LowestLifeTied => "the chosen player's".to_string(),
        PlayerFilter::MostCardsInHand => "the player with the most cards in hand's".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "a player who cast one or more {} spells this turn's",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
            format!("{}'s", describe_player_filter(filter))
        }
        PlayerFilter::HasMoreLifeThanYou { .. } => format!("{}'s", describe_player_filter(filter)),
        PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
            format!("{}'s", describe_player_filter(filter))
        }
        PlayerFilter::MaxSpeed { .. } => format!("{}'s", describe_player_filter(filter)),
        PlayerFilter::ChosenPlayer => "the chosen player's".to_string(),
        PlayerFilter::TaggedPlayer(_) => "that player's".to_string(),
        PlayerFilter::IteratedPlayer => "that player's".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller's".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_possessive_player_filter(base),
            describe_possessive_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => {
            let base = match inner.as_ref() {
                PlayerFilter::Any => "target player".to_string(),
                other => format!("target {}", describe_player_filter(other)),
            };
            format!("{base}'s")
        }
        PlayerFilter::AliasedTarget(_) => "that player's".to_string(),
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)) if tag.as_str() == "enchanted" => {
            "enchanted permanent's controller's".to_string()
        }
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)) if tag.as_str() == "equipped" => {
            "equipped creature's controller's".to_string()
        }
        PlayerFilter::ControllerOf(ObjectRef::Tagged(_) | ObjectRef::Target) => {
            "its controller's".to_string()
        }
        PlayerFilter::OwnerOf(ObjectRef::Tagged(_) | ObjectRef::Target) => {
            "its owner's".to_string()
        }
        PlayerFilter::ControllerOf(_) => "that object's controller's".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner's".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player's".to_string()
        }
    }
}

pub(crate) fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "player".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::NotYou => "player other than you".to_string(),
        PlayerFilter::Opponent => "opponent".to_string(),
        PlayerFilter::Teammate => "teammate".to_string(),
        PlayerFilter::Active => "active player".to_string(),
        PlayerFilter::Defending => "defending player".to_string(),
        PlayerFilter::Attacking => "attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "that player".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell".to_string(),
        PlayerFilter::Specific(_) => "player".to_string(),
        PlayerFilter::MostLifeTied => "player with the most life or tied for most life".to_string(),
        PlayerFilter::LowestLifeTied => {
            "player with the lowest life or tied for lowest life".to_string()
        }
        PlayerFilter::MostCardsInHand => "the player who has the most cards in hand".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "player who cast one or more {} spells this turn",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = count.to_string();
            format!(
                "{} who has at least {count_text} more cards in hand than you do as you activate this ability",
                describe_player_filter(base)
            )
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            format!(
                "{} who has more life than you do as you activate this ability",
                describe_player_filter(base)
            )
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => filter.description(),
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            let verb = if *has_max_speed {
                "has max speed"
            } else {
                "doesn't have max speed"
            };
            format!("{} who {verb}", describe_player_filter(base))
        }
        PlayerFilter::ChosenPlayer => "chosen player".to_string(),
        PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
            "enchanted player".to_string()
        }
        PlayerFilter::TaggedPlayer(_) => "that player".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_player_filter(base),
            describe_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => format!("target {}", describe_player_filter(inner)),
        PlayerFilter::AliasedTarget(_) => "that player".to_string(),
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)) if tag.as_str() == "enchanted" => {
            "enchanted permanent's controller".to_string()
        }
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)) if tag.as_str() == "equipped" => {
            "equipped creature's controller".to_string()
        }
        PlayerFilter::ControllerOf(_) => "controller".to_string(),
        PlayerFilter::OwnerOf(_) => "owner".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player".to_string()
        }
    }
}

#[allow(dead_code)]
pub(super) fn describe_card_type_word(card_type: CardType) -> &'static str {
    card_type.name()
}

#[allow(dead_code)]
pub(super) fn describe_card_type_list(
    card_types: &[CardType],
    connective: ObjectFilterUnionConnective,
) -> String {
    describe_filter_union_list(
        card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect(),
        connective,
        true,
    )
}

#[allow(dead_code)]
pub(super) fn describe_card_type_source_phrase(
    card_types: &[CardType],
    connective: ObjectFilterUnionConnective,
) -> String {
    let types = describe_card_type_list(card_types, connective);
    if types.is_empty() {
        return "a source".to_string();
    }
    let article = if types
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {types}")
}

#[allow(dead_code)]
pub(super) fn describe_stack_object_kind(kind: StackObjectKind) -> &'static str {
    match kind {
        StackObjectKind::Spell => "spell",
        StackObjectKind::Ability => "ability",
        StackObjectKind::ActivatedAbility => "activated ability",
        StackObjectKind::TriggeredAbility => "triggered ability",
        StackObjectKind::SpellOrAbility => "spell or ability",
    }
}

pub(super) fn alternative_cast_matches_kind(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    kind: AlternativeCastKind,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    match (kind, method) {
        (AlternativeCastKind::Blitz, AlternativeCastingMethod::Blitz { .. }) => true,
        (AlternativeCastKind::Dash, AlternativeCastingMethod::Dash { .. }) => true,
        (AlternativeCastKind::Flashback, AlternativeCastingMethod::Flashback { .. }) => true,
        (AlternativeCastKind::JumpStart, AlternativeCastingMethod::JumpStart { .. }) => true,
        (AlternativeCastKind::Escape, AlternativeCastingMethod::Escape { .. }) => true,
        (AlternativeCastKind::Madness, AlternativeCastingMethod::Madness { .. }) => true,
        (AlternativeCastKind::Miracle, AlternativeCastingMethod::Miracle { .. }) => true,
        (AlternativeCastKind::Suspend, AlternativeCastingMethod::Suspend { .. }) => true,
        _ => false,
    }
}

pub(super) fn object_has_alternative_cast_kind(
    object: &Object,
    kind: AlternativeCastKind,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
) -> bool {
    if object
        .alternative_casts
        .iter()
        .any(|method| alternative_cast_matches_kind(method, kind))
    {
        return true;
    }

    // Include temporary grants (e.g., Snapcaster Mage granting flashback).
    let Some(player) = ctx.you else {
        return false;
    };
    game.effect_store
        .grant_registry
        .granted_alternative_casts_for_card(game, object.id, object.zone, player)
        .iter()
        .any(|grant| alternative_cast_matches_kind(&grant.method, kind))
}

pub(super) fn object_has_static_ability_id(object: &Object, ability_id: StaticAbilityId) -> bool {
    use crate::ability::AbilityKind;

    let has_regular = object.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            static_ability.id() == ability_id
        } else {
            false
        }
    });
    if has_regular {
        return true;
    }

    object
        .level_granted_abilities()
        .iter()
        .any(|ability| ability.id() == ability_id)
}

pub(super) fn object_has_ability_marker(object: &Object, marker: &str) -> bool {
    if marker.trim().eq_ignore_ascii_case("disturb")
        && object.alternative_casts.iter().any(|method| {
            matches!(
                method,
                crate::alternative_cast::AlternativeCastingMethod::Disturb { .. }
            )
        })
    {
        return true;
    }
    if abilities_have_marker(&object.abilities, marker) {
        return true;
    }

    object.level_granted_abilities().iter().any(|ability| {
        matches!(
            ability.id(),
            StaticAbilityId::KeywordMarker | StaticAbilityId::KeywordText
        ) && ability.display().eq_ignore_ascii_case(marker)
    })
}

pub(super) fn object_has_tap_activated_ability(object: &Object) -> bool {
    abilities_have_tap_activated_ability(&object.abilities)
}

pub(super) fn abilities_have_marker(abilities: &[crate::ability::Ability], marker: &str) -> bool {
    use crate::ability::AbilityKind;

    let normalized_marker = marker.trim().to_ascii_lowercase();
    if matches!(
        normalized_marker.as_str(),
        "mana ability" | "mana abilities"
    ) {
        return abilities_have_mana_ability(abilities);
    }
    if normalized_marker == "cycling" && abilities.iter().any(ability_is_structural_cycling) {
        return true;
    }
    if normalized_marker == "craft" && abilities.iter().any(ability_is_structural_craft) {
        return true;
    }

    abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            matches!(
                static_ability.id(),
                StaticAbilityId::KeywordMarker | StaticAbilityId::KeywordText
            ) && static_ability.display().eq_ignore_ascii_case(marker)
        } else {
            false
        }
    }) || abilities
        .iter()
        .any(|ability| ability_text_has_marker(ability, marker))
}

pub(super) fn abilities_have_mana_ability(abilities: &[crate::ability::Ability]) -> bool {
    abilities.iter().any(|ability| ability.is_mana_ability())
}

pub(super) fn abilities_have_tap_activated_ability(abilities: &[crate::ability::Ability]) -> bool {
    use crate::ability::AbilityKind;

    abilities.iter().any(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => activated.has_tap_cost(),
        _ => false,
    })
}

pub(super) fn snapshot_has_static_ability_id(
    snapshot: &crate::snapshot::ObjectSnapshot,
    ability_id: StaticAbilityId,
) -> bool {
    snapshot.has_static_ability_id(ability_id)
}

pub(super) fn snapshot_has_ability_marker(
    snapshot: &crate::snapshot::ObjectSnapshot,
    marker: &str,
) -> bool {
    use crate::ability::AbilityKind;

    let normalized_marker = marker.trim().to_ascii_lowercase();
    if matches!(
        normalized_marker.as_str(),
        "mana ability" | "mana abilities"
    ) {
        return snapshot_has_mana_ability(snapshot);
    }
    if normalized_marker == "cycling"
        && snapshot.abilities.iter().any(ability_is_structural_cycling)
    {
        return true;
    }
    if normalized_marker == "craft" && snapshot.abilities.iter().any(ability_is_structural_craft) {
        return true;
    }

    snapshot.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind
            && matches!(
                static_ability.id(),
                StaticAbilityId::KeywordMarker | StaticAbilityId::KeywordText
            )
            && static_ability.display().eq_ignore_ascii_case(marker)
        {
            return true;
        }
        ability_text_has_marker(ability, marker)
    })
}

pub(super) fn ability_is_structural_cycling(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    if !ability.functional_zones.contains(&Zone::Hand)
        || !matches!(activated.timing, crate::ability::ActivationTiming::AnyTime)
    {
        return false;
    }
    let costs = activated.mana_cost.costs();
    costs.iter().any(cost_is_discard_this_card) && costs.iter().any(cost_is_cycle_keyword_action)
}

pub(super) fn ability_is_structural_craft(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    if !ability.functional_zones.contains(&Zone::Battlefield)
        || !matches!(
            activated.timing,
            crate::ability::ActivationTiming::SorcerySpeed
        )
    {
        return false;
    }
    let costs = activated.mana_cost.costs();
    costs.iter().any(cost_is_exile_this_source) && costs.iter().any(cost_is_craft_keyword_action)
}

pub(super) fn cost_is_discard_this_card(cost: &crate::costs::Cost) -> bool {
    let Some(discard) = cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
    else {
        return false;
    };
    discard.count == crate::effect::Value::Fixed(1)
        && discard.player == PlayerFilter::You
        && !discard.random
        && discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| filter.source && filter.zone == Some(Zone::Hand))
}

pub(super) fn cost_is_exile_this_source(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
        .is_some_and(|exile| matches!(exile.spec, ChooseSpec::Source) && !exile.face_down)
}

pub(super) fn cost_is_cycle_keyword_action(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>())
        .is_some_and(|emit| {
            emit.action == crate::events::KeywordActionKind::Cycle && emit.amount == 1
        })
}

pub(super) fn cost_is_craft_keyword_action(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>())
        .is_some_and(|emit| {
            emit.action == crate::events::KeywordActionKind::Craft && emit.amount == 1
        })
}

pub(super) fn snapshot_has_mana_ability(snapshot: &crate::snapshot::ObjectSnapshot) -> bool {
    snapshot
        .abilities
        .iter()
        .any(|ability| ability.is_mana_ability())
}

pub(super) fn ability_text_has_marker(ability: &crate::ability::Ability, marker: &str) -> bool {
    let marker = marker.trim().to_ascii_lowercase();
    if marker.is_empty() {
        return false;
    }
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    let text = static_ability.display();

    let words = text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '\'')))
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return false;
    }

    if marker == "cycling" {
        if !ability.functional_zones.contains(&crate::zone::Zone::Hand) {
            return false;
        }
        return words
            .iter()
            .any(|word| word == "cycling" || word.ends_with("cycling"));
    }

    let marker_words = marker
        .split_whitespace()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if marker_words.is_empty() {
        return false;
    }
    if marker_words.len() == 1 {
        return words.iter().any(|word| word == &marker_words[0]);
    }

    words.windows(marker_words.len()).any(|window| {
        window
            .iter()
            .zip(marker_words.iter())
            .all(|(word, marker_word)| word == marker_word)
    })
}

pub(super) fn snapshot_has_tap_activated_ability(
    snapshot: &crate::snapshot::ObjectSnapshot,
) -> bool {
    use crate::ability::AbilityKind;
    snapshot
        .abilities
        .iter()
        .any(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.has_tap_cost(),
            _ => false,
        })
}

#[allow(dead_code)]
pub(super) fn describe_counter_constraint(constraint: CounterConstraint, plural: bool) -> String {
    match constraint {
        CounterConstraint::Any if plural => "counters".to_string(),
        CounterConstraint::Any => "a counter".to_string(),
        CounterConstraint::Typed(counter_type) if plural => {
            format!("{} counters", counter_type.description())
        }
        CounterConstraint::Typed(counter_type) => {
            format!("a {} counter", counter_type.description())
        }
    }
}

#[allow(dead_code)]
pub(super) fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Blitz => "blitz",
        AlternativeCastKind::Dash => "dash",
        AlternativeCastKind::Flashback => "flashback",
        AlternativeCastKind::JumpStart => "jump-start",
        AlternativeCastKind::Escape => "escape",
        AlternativeCastKind::Madness => "madness",
        AlternativeCastKind::Miracle => "miracle",
        AlternativeCastKind::Suspend => "suspend",
    }
}

#[allow(dead_code)]
pub(super) fn describe_filter_static_ability(ability_id: StaticAbilityId) -> Option<&'static str> {
    use StaticAbilityId::*;
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
        Cascade => Some("cascade"),
        _ => None,
    }
}

#[allow(dead_code)]
pub(super) fn describe_comparison(cmp: &Comparison) -> String {
    fn describe_value_expr(value: &crate::effect::Value) -> String {
        use crate::effect::Value;
        match value {
            Value::SurfaceHinted { value, .. } => describe_value_expr(value),
            Value::Fixed(v) => v.to_string(),
            Value::X => "X".to_string(),
            Value::Count(filter) => format!("the number of {}", filter.description()),
            Value::CountScaled(filter, factor) => {
                format!("{factor} times the number of {}", filter.description())
            }
            Value::LandsEnteredBattlefieldThisTurn(player) => {
                format!(
                    "the number of lands that entered the battlefield under {:?}'s control this turn",
                    player
                )
            }
            Value::ColorsAmong(filter) => {
                format!("the number of colors among {}", filter.description())
            }
            Value::CreatureTypesAmong(filter) => {
                format!(
                    "the number of creature types among {}",
                    filter.description()
                )
            }
            Value::CardTypesAmong(filter) => {
                format!("the number of card types among {}", filter.description())
            }
            Value::GreatestPower(filter) => {
                format!("the greatest power among {}", filter.description())
            }
            Value::GreatestToughness(filter) => {
                format!("the greatest toughness among {}", filter.description())
            }
            Value::GreatestManaValue(filter) => {
                format!("the greatest mana value among {}", filter.description())
            }
            Value::LeastPower(filter) => {
                format!("the least power among {}", filter.description())
            }
            Value::LeastToughness(filter) => {
                format!("the lowest toughness among {}", filter.description())
            }
            Value::LeastManaValue(filter) => {
                format!("the lowest mana value among {}", filter.description())
            }
            Value::DistinctPowers(filter) => {
                format!(
                    "the number of different powers among {}",
                    filter.description()
                )
            }
            Value::CountersOnSource(counter_type) => {
                format!(
                    "the number of {} counters on this",
                    counter_type.description()
                )
            }
            Value::CountersOn(_, Some(counter_type)) => {
                format!("the number of {} counters", counter_type.description())
            }
            Value::CountersOn(_, None) => "the number of counters".to_string(),
            Value::SourcePower => "this creature's power".to_string(),
            Value::SourceToughness => "this creature's toughness".to_string(),
            Value::ManaValueOf(spec) => {
                if let ChooseSpec::Tagged(tag) = spec.base()
                    && tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                {
                    "the exiled spell's mana value".to_string()
                } else {
                    "that card's mana value".to_string()
                }
            }
            Value::UnspentMana(player) => {
                let subject = player.description();
                let verb = if matches!(player, PlayerFilter::You) {
                    "have"
                } else {
                    "has"
                };
                format!("the amount of unspent mana {subject} {verb}")
            }
            Value::EffectValue(_) => "that result".to_string(),
            Value::ColorsOfManaSpentToCastThisSpell => {
                "the number of colors of mana spent to cast this spell".to_string()
            }
            Value::EffectMetric {
                metric: crate::effect::EffectMetric::OtherNumber,
                ..
            } => "the other result".to_string(),
            Value::Add(left, right) => {
                format!(
                    "{} plus {}",
                    describe_value_expr(left),
                    describe_value_expr(right)
                )
            }
            _ => "a dynamic value".to_string(),
        }
    }

    let describe_values = |values: &[i32]| -> String {
        match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        }
    };
    match cmp {
        Comparison::Equal(v) => format!("{v}"),
        Comparison::OneOf(values) => describe_values(values),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
        Comparison::EqualExpr(value) => format!("equal to {}", describe_value_expr(value)),
        Comparison::NotEqualExpr(value) => {
            format!("not equal to {}", describe_value_expr(value))
        }
        Comparison::LessThanExpr(value) => format!("less than {}", describe_value_expr(value)),
        Comparison::LessThanOrEqualExpr(value) => {
            if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ExplicitComparison) {
                format!("less than or equal to {}", describe_value_expr(value))
            } else {
                format!("{} or less", describe_value_expr(value))
            }
        }
        Comparison::GreaterThanExpr(value) => {
            format!("greater than {}", describe_value_expr(value))
        }
        Comparison::GreaterThanOrEqualExpr(value) => {
            if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ExplicitComparison) {
                format!("greater than or equal to {}", describe_value_expr(value))
            } else {
                format!("{} or greater", describe_value_expr(value))
            }
        }
    }
}
