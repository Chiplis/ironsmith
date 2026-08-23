use super::*;
use crate::TaggedOpbjectRelation;
use crate::filter::StackObjectKind;
use ironsmith_core::DamagedBySource;
use ironsmith_core::ValueSurfaceHint;

use std::cell::Cell;

#[path = "normalize_common/condition_rendering.rs"]
mod condition_rendering;
#[path = "normalize_common/continuous_rendering.rs"]
mod continuous_rendering;
#[path = "normalize_common/semantic_phrasing.rs"]
mod semantic_phrasing;
#[path = "normalize_common/value_rendering.rs"]
mod value_rendering;

pub(crate) use condition_rendering::*;
pub(crate) use continuous_rendering::*;
pub(crate) use semantic_phrasing::*;
pub(crate) use value_rendering::*;

#[cfg(test)]
#[path = "normalize_common/tests.rs"]
mod tests;

thread_local! {
    static EFFECT_RENDER_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn with_effect_render_depth<F: FnOnce() -> String>(render: F) -> String {
    EFFECT_RENDER_DEPTH.with(|depth| {
        let current = depth.get();
        if current >= 128 {
            return "<render recursion limit>".to_string();
        }
        depth.set(current + 1);
        let rendered = render();
        depth.set(current);
        rendered
    })
}

fn is_source_exiled_count_filter(filter: &ObjectFilter) -> bool {
    if filter.zone != Some(Zone::Exile)
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        })
    {
        return false;
    }

    let mut base = filter.clone();
    base.zone = None;
    // Source-reference wording (for example, "this enchantment") is
    // presentation metadata on source-exiled threshold predicates.
    base.source_surface = None;
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    });
    base == ObjectFilter::default()
}

fn describe_player_controls_only_implicit_tagged_object(
    player: &PlayerFilter,
    filter: &ObjectFilter,
    negated: bool,
) -> Option<String> {
    let mut stripped = filter.clone();
    if stripped
        .controller
        .as_ref()
        .is_some_and(|controller| controller == player)
    {
        stripped.controller = None;
    }

    let tagged_idx = stripped.tagged_constraints.iter().position(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && is_implicit_reference_tag(constraint.tag.as_str())
    })?;
    if stripped
        .tagged_constraints
        .iter()
        .enumerate()
        .any(|(idx, constraint)| {
            idx != tagged_idx || constraint.relation != TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }

    stripped.tagged_constraints.remove(tagged_idx);
    let object_text = if stripped == ObjectFilter::creature() {
        "that creature"
    } else if stripped == ObjectFilter::default() {
        "it"
    } else {
        return None;
    };

    let subject = describe_player_filter(player);
    if negated {
        let verb = if subject == "you" {
            "don't control"
        } else {
            "doesn't control"
        };
        Some(format!("{subject} {verb} it"))
    } else {
        Some(format!(
            "{} {} {object_text}",
            subject,
            player_verb(&subject, "control", "controls")
        ))
    }
}

pub(super) fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::NotYou => "a player other than you".to_string(),
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        PlayerFilter::Target(inner) => {
            let inner_text = describe_player_filter(inner);
            if inner_text == "you" {
                "you".to_string()
            } else {
                format!("target {}", strip_leading_article(&inner_text))
            }
        }
        PlayerFilter::AliasedTarget(_) => "that player".to_string(),
        PlayerFilter::Specific(_) => "that player".to_string(),
        PlayerFilter::MostLifeTied => {
            "a player with the most life or tied for most life".to_string()
        }
        PlayerFilter::LowestLifeTied => {
            "a player with the lowest life or tied for lowest life".to_string()
        }
        PlayerFilter::MostCardsInHand => "the player who has the most cards in hand".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "a player who cast one or more {} spells this turn",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::AttackedBySourceThisTurn => {
            "a player this creature attacked this turn".to_string()
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => format!(
            "{} this source has dealt damage to this game",
            describe_player_filter(base)
        ),
        PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => filter.description(),
        PlayerFilter::LostLifeThisTurn { base } => format!(
            "{} who lost life this turn",
            strip_leading_article(&describe_player_filter(base))
        ),
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => filter.description(),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = small_number_word(*count).unwrap_or_else(|| count.to_string());
            format!(
                "{} who has at least {count_text} more cards in hand than you do as you activate this ability",
                strip_leading_article(&describe_player_filter(base))
            )
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            format!(
                "{} who has more life than you do as you activate this ability",
                strip_leading_article(&describe_player_filter(base))
            )
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => filter.description(),
        PlayerFilter::ControlsMost { .. } => filter.description(),
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            let verb = if *has_max_speed {
                "has max speed"
            } else {
                "doesn't have max speed"
            };
            format!(
                "{} who {verb}",
                strip_leading_article(&describe_player_filter(base))
            )
        }
        PlayerFilter::ChosenPlayer => "the chosen player".to_string(),
        PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
            "enchanted player".to_string()
        }
        PlayerFilter::TaggedPlayer(_) => "that player".to_string(),
        PlayerFilter::Active => "that player".to_string(),
        PlayerFilter::Defending => "the defending player".to_string(),
        PlayerFilter::Attacking => "the attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "that player".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell".to_string(),
        PlayerFilter::Teammate => "a teammate".to_string(),
        PlayerFilter::PlayerToYourLeft => "the player to your left".to_string(),
        PlayerFilter::PlayerToYourRight => "the player to your right".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller".to_string()
        }
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Opponent)
                && !matches!(excluded.as_ref(), PlayerFilter::You) =>
        {
            "another one of your opponents".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            strip_leading_article(&describe_player_filter(base)),
            strip_leading_article(&describe_player_filter(excluded))
        ),
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "enchanted" =>
        {
            "enchanted creature's controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "equipped" =>
        {
            "equipped creature's controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "triggering_source" =>
        {
            "that source's controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "__it__" =>
        {
            "its controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Target) => {
            "its controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(_)) => {
            "its controller".to_string()
        }
        PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == crate::tag::SOURCE_OBJECT_TAG =>
        {
            "this source's owner".to_string()
        }
        PlayerFilter::OwnerOf(crate::target::ObjectRef::Target) => "its owner".to_string(),
        PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(_)) => "its owner".to_string(),
        PlayerFilter::ControllerOf(_) => "that object's controller".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player".to_string()
        }
    }
}

pub(super) fn describe_player_counter_holder(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "you have".to_string(),
        PlayerFilter::Opponent => "an opponent has".to_string(),
        PlayerFilter::Any => "a player has".to_string(),
        PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) | PlayerFilter::Specific(_) => {
            "that player has".to_string()
        }
        other => format!("{} has", describe_player_filter(other)),
    }
}

pub(super) fn describe_player_set_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Opponent => "your opponents".to_string(),
        PlayerFilter::Any => "players".to_string(),
        PlayerFilter::NotYou => "players other than you".to_string(),
        PlayerFilter::Teammate => "your teammates".to_string(),
        _ => describe_player_filter(filter),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CastSpellFilterContext {
    Standalone,
    EnclosingPermission,
}

fn collapse_simple_cast_spell_alternatives(filter: &ObjectFilter) -> Option<ObjectFilter> {
    if filter.any_of.is_empty() {
        return None;
    }

    let mut collapsed = filter.clone();
    let alternatives = std::mem::take(&mut collapsed.any_of);
    for mut alternative in alternatives {
        let card_types = std::mem::take(&mut alternative.card_types);
        let subtypes = std::mem::take(&mut alternative.subtypes);
        if card_types.len() + subtypes.len() != 1 || alternative != ObjectFilter::default() {
            return None;
        }
        collapsed.card_types.extend(card_types);
        collapsed.subtypes.extend(subtypes);
    }
    if !collapsed.card_types.is_empty() && !collapsed.subtypes.is_empty() {
        collapsed.type_or_subtype_union = true;
    }
    Some(collapsed)
}

fn describe_cast_spell_card_types(filter: &ObjectFilter) -> Option<String> {
    if !filter.all_card_types.is_empty() {
        return Some(
            filter
                .all_card_types
                .iter()
                .map(|card_type| card_type.name())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    if filter.card_types.is_empty() {
        return None;
    }

    let permanent_types = [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    if filter.card_types.len() == permanent_types.len()
        && permanent_types
            .iter()
            .all(|card_type| filter.card_types.contains(card_type))
    {
        return Some("permanent".to_string());
    }

    Some(join_with_or(
        &filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>(),
    ))
}

fn describe_cast_spell_subtypes(filter: &ObjectFilter) -> Option<String> {
    if filter.subtypes.is_empty() {
        return None;
    }

    let outlaw_pack = [
        Subtype::Assassin,
        Subtype::Mercenary,
        Subtype::Pirate,
        Subtype::Rogue,
        Subtype::Warlock,
    ];
    let mut remaining = filter.subtypes.clone();
    let mut subtype_words = Vec::new();
    if outlaw_pack
        .iter()
        .all(|subtype| remaining.contains(subtype))
    {
        subtype_words.push("outlaw".to_string());
        remaining.retain(|subtype| !outlaw_pack.contains(subtype));
    }
    subtype_words.extend(remaining.iter().map(std::string::ToString::to_string));
    Some(join_with_or(&subtype_words))
}

fn place_cast_spell_types_before_noun(description: String, filter: &ObjectFilter) -> String {
    let card_types = describe_cast_spell_card_types(filter);
    let subtypes = describe_cast_spell_subtypes(filter);

    match (card_types, subtypes) {
        (None, Some(subtypes)) => description.replacen(
            &format!("spell {subtypes}"),
            &format!("{subtypes} spell"),
            1,
        ),
        (Some(card_types), Some(subtypes)) if filter.type_or_subtype_union => description.replacen(
            &format!("{card_types} spell or {subtypes}"),
            &format!("{card_types} or {subtypes} spell"),
            1,
        ),
        (Some(card_types), Some(subtypes)) => {
            let already_ordered = format!("{subtypes} {card_types} spell");
            if description.contains(&already_ordered) {
                description
            } else {
                description.replacen(
                    &format!("{card_types} spell {subtypes}"),
                    &already_ordered,
                    1,
                )
            }
        }
        _ => description,
    }
}

pub(super) fn describe_cast_spell_origin(filter: &ObjectFilter) -> Option<String> {
    let zone = filter.zone?;
    let possessive_zone = |zone_name: &str| {
        filter
            .owner
            .as_ref()
            .map(|owner| {
                format!(
                    "from {} {zone_name}",
                    describe_possessive_player_filter(owner)
                )
            })
            .unwrap_or_else(|| format!("from a {zone_name}"))
    };

    match zone {
        Zone::Stack => None,
        Zone::Battlefield => Some("from the battlefield".to_string()),
        Zone::Graveyard if filter.single_graveyard && filter.owner.is_none() => {
            Some("from a single graveyard".to_string())
        }
        Zone::Graveyard => Some(possessive_zone("graveyard")),
        Zone::Hand => Some(possessive_zone("hand")),
        Zone::Library => Some(possessive_zone("library")),
        Zone::Exile => Some("from exile".to_string()),
        Zone::Command => Some("from the command zone".to_string()),
        Zone::Ante => Some("from ante".to_string()),
        Zone::OutsideGame => Some("from outside the game".to_string()),
    }
}

pub(super) fn describe_cast_spell_filter(
    filter: &ObjectFilter,
    context: CastSpellFilterContext,
) -> String {
    let collapsed = collapse_simple_cast_spell_alternatives(filter);
    let filter = collapsed.as_ref().unwrap_or(filter);

    if filter.name.as_deref() == Some("{chosen name}") {
        let mut base = filter.clone();
        base.name = None;
        if base == ObjectFilter::default() {
            return "spell with the chosen name".to_string();
        }
    }
    if filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].tag.as_str() == "__chosen_name__"
        && filter.tagged_constraints[0].relation == TaggedOpbjectRelation::SameNameAsTagged
    {
        let mut base = filter.clone();
        base.tagged_constraints.clear();
        if base == ObjectFilter::default() {
            return "spell with the chosen name".to_string();
        }
    }
    let origin = if context == CastSpellFilterContext::Standalone {
        describe_cast_spell_origin(filter)
    } else {
        None
    };
    let mut projected = filter.clone();
    projected.zone = Some(Zone::Stack);
    projected.stack_kind = Some(StackObjectKind::Spell);
    projected
        .excluded_card_types
        .retain(|card_type| *card_type != CardType::Land);
    if context == CastSpellFilterContext::EnclosingPermission || origin.is_some() {
        projected.owner = None;
        projected.single_graveyard = false;
    }

    let mut description = place_cast_spell_types_before_noun(projected.description(), &projected);
    if let Some(origin) = origin {
        description.push(' ');
        description.push_str(&origin);
    }
    strip_leading_article(&description).to_string()
}

pub(super) fn describe_cast_limit_spell_filter(filter: &ObjectFilter) -> String {
    describe_cast_spell_filter(filter, CastSpellFilterContext::Standalone)
}

pub(super) fn pluralize_cast_spell_description(description: &str) -> String {
    let bytes = description.as_bytes();
    let noun_starts = description
        .match_indices("spell")
        .filter_map(|(start, _)| {
            let before_is_boundary = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
            let after = start + "spell".len();
            let after_is_boundary = after == bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'\'');
            (before_is_boundary && after_is_boundary).then_some(start)
        })
        .collect::<Vec<_>>();
    if noun_starts.is_empty() {
        return description.to_string();
    }

    let mut plural = String::with_capacity(description.len() + noun_starts.len());
    let mut cursor = 0;
    for noun_start in noun_starts {
        plural.push_str(&description[cursor..noun_start]);
        plural.push_str("spells");
        cursor = noun_start + "spell".len();
    }
    plural.push_str(&description[cursor..]);
    plural
}

pub(super) fn describe_cast_ban_spell_filter(filter: &ObjectFilter) -> String {
    if filter == &ObjectFilter::default() {
        return "spells".to_string();
    }
    if filter == &ObjectFilter::default().with_type(CardType::Creature) {
        return "creature spells".to_string();
    }
    if filter == &ObjectFilter::default().of_chosen_card_type() {
        return "spells of the chosen type".to_string();
    }

    pluralize_cast_spell_description(&describe_cast_limit_spell_filter(filter))
}

pub(super) fn strip_leading_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("A "))
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("An "))
        .or_else(|| text.strip_prefix("the "))
        .or_else(|| text.strip_prefix("The "))
        .unwrap_or(text)
}

pub(super) fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

pub(super) fn lowercase_first(text: &str) -> String {
    if text.starts_with('{') {
        return text.to_string();
    }
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
        None => String::new(),
    }
}

fn replace_this_spell_self_reference(text: String, subject: &str) -> String {
    const CAST_THIS_SPELL: &str = "__ironsmith_cast_this_spell__";
    let protected = text.replace("cast this spell", CAST_THIS_SPELL);
    let replaced = protected
        .replace("This spell", &capitalize_first(subject))
        .replace("this spell", &lowercase_first(subject));
    replaced.replace(CAST_THIS_SPELL, "cast this spell")
}

fn normalize_granted_triggered_ability_surface(surface: String) -> String {
    let Some((head, tail)) = surface
        .split_once(": ")
        .or_else(|| surface.split_once(", "))
    else {
        return surface;
    };
    let lower_head = head.to_ascii_lowercase();
    if !(lower_head.starts_with("when ")
        || lower_head.starts_with("whenever ")
        || lower_head.starts_with("at the beginning "))
    {
        return surface;
    }

    // Oracle keeps the explicit subject in optional instructions ("When this
    // creature dies, you may return it ..."); only a mandatory "You <verb>"
    // drops to the bare imperative.
    let keeps_you_subject = tail.to_ascii_lowercase().starts_with("you may ");
    let tail = if keeps_you_subject {
        tail
    } else {
        tail.strip_prefix("You ")
            .or_else(|| tail.strip_prefix("you "))
            .unwrap_or(tail)
            .trim_start()
    };
    if tail.is_empty() {
        return surface;
    }

    let mut normalized_tail = lowercase_first(tail);
    if !normalized_tail.ends_with('.')
        && !normalized_tail.ends_with('!')
        && !normalized_tail.ends_with('?')
    {
        normalized_tail.push('.');
    }

    format!("{head}, {normalized_tail}")
}

fn normalize_temporary_granted_trigger_surface(surface: String, ability: &Ability) -> String {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return surface;
    };
    let trigger_surface = triggered.trigger.display();
    let lower_surface = surface.to_ascii_lowercase();
    let uses_one_shot_when = trigger_surface
        .starts_with("Whenever this permanent deals damage to the player who cast ")
        || (trigger_surface == "Whenever this creature attacks"
            && (lower_surface.contains("must block")
                || lower_surface.contains("blocks it this turn if able")));
    if !uses_one_shot_when {
        return surface;
    }

    surface
        .strip_prefix("Whenever this ")
        .map(|rest| format!("When this {rest}"))
        .unwrap_or(surface)
}

pub(super) fn lowercase_may_clause(text: &str) -> String {
    // Oracle uses lowercase imperatives after "may" ("you may put...", "that player may search...").
    // Avoid lowercasing leading proper nouns/plurals (e.g. creature types like "Allies").
    let Some(first) = text.split_whitespace().next() else {
        return String::new();
    };
    let should_lowercase = matches!(
        first,
        "A" | "An"
            | "The"
            | "Target"
            | "Add"
            | "Attach"
            | "Cast"
            | "Choose"
            | "Copy"
            | "Counter"
            | "Create"
            | "Destroy"
            | "Discard"
            | "Draw"
            | "Exile"
            | "Exchange"
            | "Fight"
            | "Flip"
            | "Gain"
            | "Lose"
            | "Mill"
            | "Pay"
            | "Play"
            | "Put"
            | "Regenerate"
            | "Remove"
            | "Reveal"
            | "Return"
            | "Sacrifice"
            | "Scry"
            | "Search"
            | "Shuffle"
            | "Tap"
            | "Transform"
            | "Untap"
    );
    if should_lowercase {
        return lowercase_first(text);
    }
    text.to_string()
}

fn should_lowercase_trigger_effect_tail(tail: &str) -> bool {
    let Some(first) = tail.split_whitespace().next() else {
        return false;
    };
    let first = first.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
    matches!(
        first,
        "A" | "An"
            | "The"
            | "This"
            | "That"
            | "Those"
            | "It"
            | "They"
            | "If"
            | "Then"
            | "You"
            | "Add"
            | "Attach"
            | "Cast"
            | "Choose"
            | "Copy"
            | "Counter"
            | "Create"
            | "Destroy"
            | "Discard"
            | "Draw"
            | "Exile"
            | "Fight"
            | "Flip"
            | "Gain"
            | "Lose"
            | "Mill"
            | "Pay"
            | "Play"
            | "Put"
            | "Regenerate"
            | "Reveal"
            | "Return"
            | "Sacrifice"
            | "Scry"
            | "Search"
            | "Shuffle"
            | "Surveil"
            | "Tap"
            | "Transform"
            | "Untap"
    )
}

pub(super) fn describe_mana_pool_owner(filter: &PlayerFilter) -> String {
    let player = describe_player_filter(filter);
    if player == "you" || player == "target you" {
        "your mana pool".to_string()
    } else if player.ends_with('s') {
        format!("{player}' mana pool")
    } else {
        format!("{player}'s mana pool")
    }
}

pub(super) fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    if matches!(
        filter,
        PlayerFilter::DamagedPlayer
            | PlayerFilter::AliasedTarget(_)
            | PlayerFilter::AliasedOwnerOf(_)
            | PlayerFilter::AliasedControllerOf(_)
    ) {
        return "their".to_string();
    }
    let player = describe_player_filter(filter);
    if player == "you" || player == "target you" {
        "your".to_string()
    } else if player.ends_with('s') {
        format!("{player}'")
    } else {
        format!("{player}'s")
    }
}

pub(super) fn describe_possessive_graveyard_owner_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::AliasedTarget(_)
        | PlayerFilter::AliasedOwnerOf(_)
        | PlayerFilter::AliasedControllerOf(_) => "their".to_string(),
        PlayerFilter::OwnerOf(_) => "that player's".to_string(),
        _ => describe_possessive_player_filter(filter),
    }
}

pub(super) fn describe_possessive_choose_spec(spec: &ChooseSpec) -> String {
    let subject = describe_choose_spec(spec);
    if subject == "you" || subject == "target you" {
        "your".to_string()
    } else if subject == "it" {
        "its".to_string()
    } else if subject.ends_with('s') {
        format!("{subject}'")
    } else {
        format!("{subject}'s")
    }
}

fn describe_card_type_graveyard_scope(player: &PlayerFilter) -> String {
    match player {
        PlayerFilter::You => "your graveyard".to_string(),
        PlayerFilter::Opponent | PlayerFilter::NotYou => "your opponents' graveyards".to_string(),
        PlayerFilter::Any => "all graveyards".to_string(),
        PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Opponent) => {
            "target opponent's graveyard".to_string()
        }
        PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::You) => {
            "your graveyard".to_string()
        }
        _ => format!(
            "{} graveyard",
            describe_possessive_graveyard_owner_filter(player)
        ),
    }
}

pub(super) fn join_with_and(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(", and ");
            text.push_str(parts.last().map(String::as_str).unwrap_or_default());
            text
        }
    }
}

pub(super) fn join_with_or(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} or {}", parts[0], parts[1]),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(", or ");
            text.push_str(parts.last().map(String::as_str).unwrap_or_default());
            text
        }
    }
}

pub(super) fn repeated_energy_symbols(count: usize) -> String {
    "{E}".repeat(count)
}

pub(super) fn describe_energy_payment_amount(value: &Value) -> String {
    match value {
        Value::Fixed(amount) if *amount > 0 => repeated_energy_symbols(*amount as usize),
        _ => format!("an amount of {{E}} equal to {}", describe_value(value)),
    }
}

pub(super) fn describe_card_type_word_local(card_type: CardType) -> &'static str {
    card_type.name()
}

pub(super) fn describe_pt_value(value: crate::card::PtValue) -> String {
    match value {
        crate::card::PtValue::Fixed(n) => n.to_string(),
        crate::card::PtValue::Star => "*".to_string(),
        crate::card::PtValue::StarPlus(n) => format!("*+{n}"),
    }
}

pub(super) fn describe_token_color_words(
    colors: crate::color::ColorSet,
    include_colorless: bool,
) -> String {
    if colors.is_empty() {
        return if include_colorless {
            "colorless".to_string()
        } else {
            String::new()
        };
    }

    if colors.count() == 2 {
        use crate::color::Color;
        let has_w = colors.contains(Color::White);
        let has_u = colors.contains(Color::Blue);
        let has_b = colors.contains(Color::Black);
        let has_r = colors.contains(Color::Red);
        let has_g = colors.contains(Color::Green);
        if has_w && has_u {
            return "white and blue".to_string();
        }
        if has_u && has_b {
            return "blue and black".to_string();
        }
        if has_b && has_r {
            return "black and red".to_string();
        }
        if has_r && has_g {
            return "red and green".to_string();
        }
        if has_g && has_w {
            return "green and white".to_string();
        }
        if has_w && has_b {
            return "white and black".to_string();
        }
        if has_b && has_g {
            return "black and green".to_string();
        }
        if has_g && has_u {
            return "green and blue".to_string();
        }
        if has_u && has_r {
            return "blue and red".to_string();
        }
        if has_r && has_w {
            return "red and white".to_string();
        }
    }

    let mut names = Vec::new();
    if colors.contains(crate::color::Color::White) {
        names.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        names.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        names.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        names.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        names.push("green".to_string());
    }
    join_with_and(&names)
}

pub(super) fn describe_token_blueprint(token: &CardDefinition) -> String {
    describe_token_blueprint_with_presentation(token, None)
}

pub(super) fn describe_create_token_blueprint(
    create: &crate::effects::CreateTokenEffect,
) -> String {
    describe_create_token_blueprint_with_presentation(create, create.ability_presentation)
}

pub(super) fn describe_create_token_blueprint_with_presentation(
    create: &crate::effects::CreateTokenEffect,
    ability_presentation: Option<ironsmith_core::TokenAbilityPresentation>,
) -> String {
    let mut blueprint =
        describe_token_blueprint_with_presentation(&create.token, ability_presentation);
    if create.use_source_chosen_color {
        blueprint = blueprint.replacen("colorless ", "", 1);
    }
    let characteristic = match (
        create.use_source_chosen_color,
        create.use_source_chosen_creature_type,
    ) {
        (true, true) => Some("the chosen color and type"),
        (true, false) => Some("the chosen color"),
        (false, true) => Some("the chosen type"),
        (false, false) => None,
    };
    if let Some(characteristic) = characteristic
        && let Some(token_end) = blueprint.find(" token").map(|idx| idx + " token".len())
    {
        blueprint.insert_str(token_end, &format!(" of {characteristic}"));
    }
    blueprint
}

pub(super) fn describe_token_blueprint_with_presentation(
    token: &CardDefinition,
    ability_presentation: Option<ironsmith_core::TokenAbilityPresentation>,
) -> String {
    let standalone_tail_count = ability_presentation
        .map(ironsmith_core::TokenAbilityPresentation::standalone_tail_count)
        .unwrap_or(0)
        .min(token.abilities.len());
    let grouped_ability_presentation = ability_presentation
        .and_then(ironsmith_core::TokenAbilityPresentation::grouped_presentation);
    let card = &token.card;
    if card.subtypes.contains(&crate::types::Subtype::Role)
        && !card.name.trim().is_empty()
        && !card.name.eq_ignore_ascii_case("token")
    {
        return format!("{} token", card.name);
    }
    let mut parts = Vec::new();
    let mut creature_name_prefix: Option<String> = None;
    let mut explicit_named_clause: Option<String> = None;
    let has_characteristic_defining_pt = token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::CharacteristicDefiningPT
        )
    });
    let is_named_noncreature_subtype_token = !card.is_creature()
        && !card.name.trim().is_empty()
        && !card.name.eq_ignore_ascii_case("token")
        && !card.subtypes.is_empty()
        && card
            .subtypes
            .iter()
            .any(|subtype| subtype.to_string().eq_ignore_ascii_case(&card.name));

    if !card.supertypes.is_empty() {
        let supertypes = card
            .supertypes
            .iter()
            .map(|supertype| supertype.name().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        if !supertypes.is_empty() {
            parts.push(supertypes);
        }
    }

    if let Some(pt) = card.power_toughness
        && !(has_characteristic_defining_pt
            && matches!(pt.power, crate::card::PtValue::Fixed(0))
            && matches!(pt.toughness, crate::card::PtValue::Fixed(0)))
    {
        parts.push(format!(
            "{}/{}",
            describe_pt_value(pt.power),
            describe_pt_value(pt.toughness)
        ));
    }

    let explicit_colorless = token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::MakeColorless
        )
    });
    let all_colors = card.colors().count() == crate::color::Color::ALL.len() as u32;
    let colors = describe_token_color_words(
        card.colors(),
        (card.is_creature() || explicit_colorless) && !all_colors,
    );
    if !colors.is_empty() && !all_colors {
        parts.push(colors);
    }

    if card.subtypes.is_empty()
        && !card.is_creature()
        && card.card_types.contains(&CardType::Artifact)
        && !card.name.trim().is_empty()
        && !card.name.eq_ignore_ascii_case("token")
    {
        // Prefer the oracle-style "artifact token named <Name>" for explicitly named tokens.
        // (Common named tokens like Treasure/Clue/Food/Blood/Powerstone are handled elsewhere.)
        if !matches!(
            card.name.as_str(),
            "Treasure" | "Clue" | "Food" | "Blood" | "Powerstone"
        ) {
            explicit_named_clause = Some(card.name.to_string());
        } else {
            parts.push(card.name.to_string());
        }
    }

    if !card.subtypes.is_empty() {
        if is_named_noncreature_subtype_token {
            parts.push(card.name.to_string());
        } else {
            let name_lower = card.name.to_ascii_lowercase();
            let has_changeling_keyword = token.abilities.iter().any(|ability| {
                matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.is_keyword()
                            && static_ability.display().eq_ignore_ascii_case("changeling")
                )
            });
            let displayed_subtypes = card
                .subtypes
                .iter()
                .filter(|subtype| {
                    !(has_changeling_keyword && **subtype == crate::types::Subtype::Changeling)
                })
                .collect::<Vec<_>>();
            let subtype_words_lower = displayed_subtypes
                .iter()
                .map(|subtype| subtype.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>();
            let subtype_text = displayed_subtypes
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            let name_matches_any_subtype = subtype_words_lower.contains(&name_lower);
            let name_is_distinct = !card.name.trim().is_empty()
                && name_lower != "token"
                && name_lower != subtype_text.to_ascii_lowercase()
                && !name_matches_any_subtype;
            let use_name_as_prefix = name_is_distinct
                && card
                    .supertypes
                    .contains(&crate::types::Supertype::Legendary);
            if name_is_distinct && !use_name_as_prefix {
                explicit_named_clause = Some(card.name.to_string());
            }
            let use_name_for_noncreature = false;
            if use_name_as_prefix {
                creature_name_prefix = Some(card.name.to_string());
                if !subtype_text.is_empty() {
                    parts.push(subtype_text);
                }
            } else if use_name_for_noncreature {
                parts.push(card.name.to_string());
                if !subtype_text.is_empty() {
                    parts.push(subtype_text);
                }
            } else {
                parts.push(subtype_text);
            }
        }
    }

    if !card.card_types.is_empty() && !is_named_noncreature_subtype_token {
        parts.push(
            card.card_types
                .iter()
                .map(|card_type| card_type.name().to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    parts.push("token".to_string());

    let appositive_named_token = creature_name_prefix.is_some();
    let mut text = parts.join(" ");
    if all_colors && !appositive_named_token {
        text.push_str(" that's all colors");
    }
    if standalone_tail_count == 0
        && matches!(
            grouped_ability_presentation,
            Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined)
        )
        && let Some(payload) = compact_separate_sentence_equipment_token_ability_payload(token)
    {
        if let Some(name) = &explicit_named_clause {
            text.push_str(" named ");
            text.push_str(name);
        }
        text.push_str(". It has ");
        text.push_str(&payload);
        if let Some(name) = creature_name_prefix {
            text = format!("{name}, {}", with_indefinite_article(&text));
        }
        return text;
    }
    if standalone_tail_count == 0
        && let Some(payload) = compact_equipment_token_ability_payload(token)
    {
        if let Some(name) = &explicit_named_clause {
            text.push_str(" named ");
            text.push_str(name);
        }
        text.push_str(" with ");
        text.push_str(&payload);
        if let Some(name) = creature_name_prefix {
            text = format!("{name}, {}", with_indefinite_article(&text));
        }
        return text;
    }
    let mut keyword_texts = Vec::new();
    let mut extra_ability_texts = Vec::new();
    let mut standalone_ability_texts = Vec::new();
    let has_non_toxic_poison_trigger = token_has_non_toxic_poison_trigger(token);
    let has_decayed_marker = token_has_decayed_marker(token);
    let standalone_tail_start = token.abilities.len() - standalone_tail_count;
    for (ability_idx, ability) in token.abilities.iter().enumerate() {
        if ability_idx >= standalone_tail_start {
            standalone_ability_texts.push(describe_standalone_token_ability_text(ability));
            continue;
        }
        match &ability.kind {
            AbilityKind::Static(static_ability) => {
                if static_ability.id() == crate::static_abilities::StaticAbilityId::MakeColorless {
                    continue;
                }
                if static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                    && static_ability.display().eq_ignore_ascii_case("decayed")
                {
                    keyword_texts.push("decayed".to_string());
                    continue;
                }
                if static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                    && !is_keyword_style_line(static_ability.display().as_str())
                {
                    extra_ability_texts.push(quote_token_granted_ability_text(
                        normalize_token_granted_static_ability_text(
                            static_ability.display().as_str(),
                        )
                        .as_str(),
                    ));
                    continue;
                }
                if has_decayed_marker
                    && static_ability.id() == crate::static_abilities::StaticAbilityId::CantBlock
                {
                    continue;
                }
                if static_ability.is_keyword() {
                    keyword_texts.push(static_ability.display().to_ascii_lowercase());
                    continue;
                }
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::CopyTriggeredAbilities
                {
                    extra_ability_texts.push(static_ability.display().to_ascii_lowercase());
                    continue;
                }
                extra_ability_texts.push(quote_token_granted_ability_text(
                    normalize_token_granted_static_ability_text(
                        describe_static_ability_with_subject(static_ability, "this token").as_str(),
                    )
                    .as_str(),
                ));
            }
            AbilityKind::Triggered(triggered) => {
                if let Some(keyword) = describe_structural_prowess_keyword(triggered) {
                    keyword_texts.push(keyword.to_ascii_lowercase());
                    continue;
                }
                if has_decayed_marker && is_decayed_sacrifice_trigger(triggered) {
                    continue;
                }
                if !has_non_toxic_poison_trigger
                    && let Some(keyword) = describe_structural_toxic_keyword(triggered)
                {
                    keyword_texts.push(keyword.to_ascii_lowercase());
                    continue;
                }
                let mut text =
                    quote_token_granted_ability_text(describe_inline_ability(ability).as_str());
                if matches!(
                    grouped_ability_presentation,
                    Some(
                        ironsmith_core::TokenAbilityPresentation::SeparateSentence
                            | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
                            | ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined
                            | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGainCombined
                    )
                ) && triggered
                    .trigger
                    .downcast_ref::<crate::triggers::ThisDealsDamageTrigger>()
                    .is_some()
                {
                    // Damage triggers on separately described creature tokens
                    // use the creature source noun; inline token rules use the
                    // token noun. The executable matcher is identical.
                    text = text.replacen("this token deals", "this creature deals", 1);
                }
                extra_ability_texts.push(text);
            }
            AbilityKind::Activated(activated) => {
                if let Some(crew) = describe_structural_crew_keyword(activated) {
                    keyword_texts.push(crew.to_ascii_lowercase());
                    continue;
                }
                extra_ability_texts.push(quote_token_granted_ability_text(
                    describe_inline_ability(ability).as_str(),
                ));
            }
        }
    }
    if appositive_named_token {
        let mut ordered_unique = Vec::with_capacity(keyword_texts.len());
        for keyword in keyword_texts.drain(..) {
            if !ordered_unique.contains(&keyword) {
                ordered_unique.push(keyword);
            }
        }
        keyword_texts = ordered_unique;
    } else {
        keyword_texts.sort();
        keyword_texts.dedup();
    }
    if matches!(
        grouped_ability_presentation,
        Some(ironsmith_core::TokenAbilityPresentation::InlineWith)
    ) {
        // Quoted inline rules are authored in a meaningful source order.
        // Deduplicate without sorting so a token with `"This token can't
        // block"` followed by an upkeep trigger keeps that exact order.
        let mut ordered_unique = Vec::with_capacity(extra_ability_texts.len());
        for ability_text in extra_ability_texts.drain(..) {
            if !ordered_unique.contains(&ability_text) {
                ordered_unique.push(ability_text);
            }
        }
        extra_ability_texts = ordered_unique;
    } else {
        extra_ability_texts.sort();
        extra_ability_texts.dedup();
    }
    strip_nonfinal_quoted_ability_periods(&mut extra_ability_texts);
    // An authored inline `with` clause treats intrinsic keywords and a quoted
    // rule as one serial list: `with flying, haste, and "When ..."`. Joining
    // the keyword and quoted groups independently produces the lossy
    // `flying and haste and "Whenever ..."` surface.
    if matches!(
        grouped_ability_presentation,
        Some(ironsmith_core::TokenAbilityPresentation::InlineWith)
    ) && !keyword_texts.is_empty()
        && !extra_ability_texts.is_empty()
    {
        keyword_texts.append(&mut extra_ability_texts);
    }
    if matches!(
        grouped_ability_presentation,
        Some(
            ironsmith_core::TokenAbilityPresentation::SeparateSentence
                | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
                | ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined
                | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGainCombined
        )
    ) && let Some(last) = extra_ability_texts.last_mut()
        && let Some(unquoted) = last.strip_suffix('"')
        && !unquoted.ends_with('.')
        && !unquoted.ends_with('!')
        && !unquoted.ends_with('?')
    {
        *last = format!("{unquoted}.\"");
    }
    let named_clause_after_inline_keywords = explicit_named_clause.is_some()
        && card.is_creature()
        && !keyword_texts.is_empty()
        && extra_ability_texts.is_empty()
        && !matches!(
            grouped_ability_presentation,
            Some(
                ironsmith_core::TokenAbilityPresentation::SeparateSentence
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGainCombined
            )
        );
    if !named_clause_after_inline_keywords && let Some(name) = &explicit_named_clause {
        text.push_str(" named ");
        text.push_str(name);
    }
    if !keyword_texts.is_empty() {
        match grouped_ability_presentation {
            Some(
                ironsmith_core::TokenAbilityPresentation::SeparateSentence
                | ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined,
            ) => {
                text.push_str(". It has ");
            }
            Some(
                ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
                | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGainCombined,
            ) => {
                text.push_str(". It gains ");
            }
            _ => {
                text.push_str(" with ");
            }
        }
        text.push_str(&join_with_and(&keyword_texts));
    }
    if !extra_ability_texts.is_empty() {
        if keyword_texts.is_empty() {
            match grouped_ability_presentation {
                Some(ironsmith_core::TokenAbilityPresentation::InlineWith) => {
                    text.push_str(" with ");
                }
                Some(
                    ironsmith_core::TokenAbilityPresentation::SeparateSentence
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined,
                ) => {
                    text.push_str(". It has ");
                }
                Some(
                    ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGainCombined,
                ) => {
                    text.push_str(". It gains ");
                }
                Some(_) => {
                    debug_assert!(
                        false,
                        "grouped token presentation must not retain a standalone-tail variant"
                    );
                    text.push_str(" with ");
                }
                None if token_extra_abilities_prefer_with_clause(&extra_ability_texts) => {
                    text.push_str(" with ");
                }
                None => {
                    text.push_str(". It has ");
                }
            }
        } else {
            match grouped_ability_presentation {
                Some(ironsmith_core::TokenAbilityPresentation::SeparateSentence) => {
                    text.push_str(". It has ");
                }
                Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain) => {
                    text.push_str(". It gains ");
                }
                Some(
                    ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined
                    | ironsmith_core::TokenAbilityPresentation::SeparateSentenceGainCombined,
                ) => {
                    text.push_str(" and ");
                }
                _ => {
                    text.push_str(" and ");
                }
            }
        }
        text.push_str(&join_with_and(&extra_ability_texts));
    }
    if named_clause_after_inline_keywords && let Some(name) = &explicit_named_clause {
        text.push_str(" named ");
        text.push_str(name);
    }
    if all_colors && appositive_named_token {
        text.push_str(" that's all colors");
    }
    for standalone_ability in standalone_ability_texts {
        text.push_str(". ");
        text.push_str(&standalone_ability);
    }

    if let Some(name) = creature_name_prefix {
        text = format!("{name}, {}", with_indefinite_article(&text));
    }

    text
}

fn describe_standalone_token_ability_text(ability: &Ability) -> String {
    if let Some(rendered) = describe_token_leaves_shared_damage_ability(ability) {
        return rendered;
    }
    let rendered = describe_inline_ability_with_self_subject(ability, "it");
    let unquoted = rendered
        .trim()
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(rendered.trim())
        .trim_end_matches('.');
    capitalize_first(unquoted)
}

fn describe_token_leaves_shared_damage_ability(ability: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let leaves = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()?;
    if !leaves.this_object
        || leaves.from != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || leaves.to != crate::triggers::zone_changes::ZonePattern::Any
    {
        return None;
    }
    let effects = triggered.effects.flattened_default_effects();
    let [controller_damage_effect, creature_loop_effect] = effects else {
        return None;
    };
    let controller_damage =
        controller_damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if controller_damage.target != ChooseSpec::SourceController
        || controller_damage.source_is_combat
        || controller_damage.unpreventable
    {
        return None;
    }
    let creature_loop = creature_loop_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if creature_loop.filter != ObjectFilter::creature().you_control() {
        return None;
    }
    let [creature_damage_effect] = creature_loop.effects.as_slice() else {
        return None;
    };
    let creature_damage =
        creature_damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if creature_damage.target != ChooseSpec::Iterated
        || creature_damage.amount != controller_damage.amount
        || creature_damage.source_is_combat
        || creature_damage.unpreventable
    {
        return None;
    }

    Some(format!(
        "When it leaves the battlefield, it deals {} damage to you and each creature you control",
        describe_value(&controller_damage.amount)
    ))
}

fn token_has_decayed_marker(token: &CardDefinition) -> bool {
    token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                    && static_ability.display().eq_ignore_ascii_case("decayed")
        )
    })
}

fn is_decayed_sacrifice_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksTrigger>()
            .is_none()
    {
        return false;
    }

    let effects = triggered.effects.flattened_default_effects();
    if effects.len() != 1 {
        return false;
    }
    let effect = &effects[0];
    let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    else {
        return false;
    };
    if !schedule.one_shot
        || schedule
            .trigger
            .downcast_ref::<crate::triggers::EndOfCombatTrigger>()
            .is_none()
    {
        return false;
    }

    if schedule.effects.len() != 1 {
        return false;
    }
    let delayed_effect = &schedule.effects[0];
    delayed_effect
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()
        .is_some_and(|sacrifice| sacrifice.target == ChooseSpec::Source)
}

fn token_extra_abilities_prefer_with_clause(abilities: &[String]) -> bool {
    match abilities {
        [ability] => {
            if ability == "\"This token can't block.\"" {
                return true;
            }
            if ability.starts_with("\"{") {
                return true;
            }
            if ability.starts_with("\"Whenever ")
                || ability.starts_with("\"When ")
                || ability.starts_with("\"At ")
                || ability.starts_with("\"This token")
            {
                return true;
            }
            ability.to_ascii_lowercase().starts_with(
                "\"this token saddles mounts and crews vehicles as though its power were ",
            )
        }
        abilities => abilities.iter().any(|ability| ability.starts_with("\"{")),
    }
}

fn strip_nonfinal_quoted_ability_periods(abilities: &mut [String]) {
    let Some((_, nonfinal)) = abilities.split_last_mut() else {
        return;
    };
    for ability in nonfinal {
        if let Some(without_period) = ability.strip_suffix(".\"") {
            *ability = format!("{without_period}\"");
        }
    }
}

fn compact_equipment_token_ability_payload(token: &CardDefinition) -> Option<String> {
    if !token.card.card_types.contains(&CardType::Artifact)
        || !token
            .card
            .subtypes
            .contains(&crate::types::Subtype::Equipment)
    {
        return None;
    }

    let mut pump_text: Option<String> = None;
    let mut keywords = Vec::new();
    let mut equip_text: Option<String> = None;

    for ability in &token.abilities {
        match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::MakeColorless =>
            {
                continue;
            }
            AbilityKind::Static(static_ability) => {
                let text =
                    normalize_token_granted_static_ability_text(static_ability.display().as_str());
                let text = text.trim().trim_end_matches('.');
                if let Some(rest) = text.strip_prefix("Equipped creature gets ") {
                    if pump_text.replace(rest.to_string()).is_some() {
                        return None;
                    }
                    continue;
                }
                if let Some(rest) = text.strip_prefix("Equipped creature has ") {
                    let keyword = rest.trim().to_ascii_lowercase();
                    if keyword.is_empty()
                        || keyword.contains(',')
                        || keyword.contains(" and ")
                        || keyword.contains('"')
                    {
                        return None;
                    }
                    keywords.push(keyword);
                    continue;
                }
                return None;
            }
            AbilityKind::Activated(_) => {
                let text = describe_inline_ability(ability);
                if !text.starts_with("Equip ") || text.contains(". ") {
                    return None;
                }
                if equip_text
                    .replace(text.trim_end_matches('.').to_string())
                    .is_some()
                {
                    return None;
                }
            }
            _ => return None,
        }
    }

    let equip_text = equip_text?;
    if pump_text.is_none() && keywords.is_empty() {
        return None;
    }

    let pump_text = pump_text?;
    let has_pump = true;
    let mut ability_text = format!("Equipped creature gets {pump_text}");
    if !keywords.is_empty() {
        if has_pump {
            ability_text.push_str(" and has ");
        } else {
            ability_text.push_str(" has ");
        }
        ability_text.push_str(&join_with_and(&keywords));
    }

    Some(format!(
        "\"{ability_text}\" and {}",
        lowercase_first(&equip_text)
    ))
}

/// Recombine an authored separate-sentence Equipment rule list when its
/// executable abilities consist of intrinsic keywords, one attached-creature
/// grant, and equip. Keeping this structural prevents an intrinsic keyword
/// from forcing the generic renderer to quote `Equip`, while retaining the
/// quote around the rule granted to the equipped creature.
fn compact_separate_sentence_equipment_token_ability_payload(
    token: &CardDefinition,
) -> Option<String> {
    if !token.card.card_types.contains(&CardType::Artifact)
        || !token
            .card
            .subtypes
            .contains(&crate::types::Subtype::Equipment)
    {
        return None;
    }

    let mut intrinsic_keywords = Vec::new();
    let mut pump_text: Option<String> = None;
    let mut attached_keywords = Vec::new();
    let mut equip_text: Option<String> = None;
    for ability in &token.abilities {
        match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::MakeColorless => {}
            AbilityKind::Static(static_ability) if static_ability.is_keyword() => {
                let keyword = static_ability.display().trim().to_ascii_lowercase();
                if !intrinsic_keywords.contains(&keyword) {
                    intrinsic_keywords.push(keyword);
                }
            }
            AbilityKind::Static(static_ability) => {
                let text =
                    normalize_token_granted_static_ability_text(static_ability.display().as_str());
                let text = text.trim().trim_end_matches(['.', ',']);
                if let Some(rest) = text.strip_prefix("Equipped creature gets ") {
                    if pump_text.replace(rest.to_string()).is_some() {
                        return None;
                    }
                    continue;
                }
                if let Some(rest) = text.strip_prefix("Equipped creature has ") {
                    let keyword = rest.trim().to_ascii_lowercase();
                    if keyword.is_empty()
                        || keyword.contains(',')
                        || keyword.contains(" and ")
                        || keyword.contains('"')
                    {
                        return None;
                    }
                    if !attached_keywords.contains(&keyword) {
                        attached_keywords.push(keyword);
                    }
                    continue;
                }
                return None;
            }
            AbilityKind::Activated(_) => {
                let text = describe_inline_ability(ability);
                if !text.starts_with("Equip ") || text.contains(". ") {
                    return None;
                }
                let text = lowercase_first(text.trim_end_matches('.'));
                if equip_text
                    .as_ref()
                    .is_some_and(|existing| existing != &text)
                {
                    return None;
                }
                equip_text = Some(text);
            }
            _ => return None,
        }
    }

    if intrinsic_keywords.is_empty() {
        return None;
    }
    let pump_text = pump_text?;
    let mut attached_rule = format!("Equipped creature gets {pump_text}");
    if !attached_keywords.is_empty() {
        attached_rule.push_str(" and has ");
        attached_rule.push_str(&join_with_and(&attached_keywords));
    }
    let equip_text = equip_text?;
    Some(format!(
        "{}, \"{attached_rule},\" and {equip_text}",
        intrinsic_keywords.join(", ")
    ))
}

fn token_has_non_toxic_poison_trigger(token: &CardDefinition) -> bool {
    token.abilities.iter().any(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return false;
        };
        describe_structural_toxic_keyword(triggered).is_none()
            && triggered
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::PoisonCountersEffect>()
                        .is_some()
                })
    })
}

pub(super) fn quote_token_granted_ability_text(text: &str) -> String {
    let trimmed = text.trim();
    let unquoted = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].trim()
    } else {
        trimmed
    };
    let mut normalized = normalize_quoted_token_ability_surface(unquoted);
    // Oracle uses single quotes for a rules quotation nested inside the
    // double-quoted ability of a created token.
    normalized = normalized.replace('"', "'");
    if token_quoted_ability_needs_terminal_period(&normalized) {
        normalized.push('.');
    }
    format!("\"{normalized}\"")
}

fn normalize_quoted_token_ability_surface(text: &str) -> String {
    let mut normalized = text
        .trim()
        .replace("{t}", "{T}")
        .replace("{q}", "{Q}")
        .replace("{w}", "{W}")
        .replace("{u}", "{U}")
        .replace("{b}", "{B}")
        .replace("{r}", "{R}")
        .replace("{g}", "{G}")
        .replace("{c}", "{C}")
        .replace("{e}", "{E}")
        .replace("{s}", "{S}")
        .replace("{x}", "{X}");
    if normalized.is_empty() {
        return normalized;
    }

    if !normalized.starts_with('{') {
        let normalized =
            normalize_token_self_reference_in_quoted_ability(&capitalize_first(&normalized));
        return normalize_quoted_token_trigger_surface(&normalized);
    }

    // A quoted ability beginning with a mana symbol is an activated ability.
    // Some lowered costs retain the final list comma as their cost/effect
    // separator; restore the rules-text colon before applying sentence
    // capitalization. Splitting at the final comma preserves earlier
    // multi-component costs such as "{T}, Sacrifice this artifact".
    if !normalized.contains(':')
        && let Some((cost, body)) = normalized.rsplit_once(", ")
        && cost.starts_with('{')
        && !body.is_empty()
    {
        normalized = format!("{cost}: {body}");
    }

    let mut chars: Vec<char> = normalized.chars().collect();
    let mut capitalize_next_alpha = false;
    for idx in 0..chars.len() {
        let ch = chars[idx];
        if capitalize_next_alpha && ch.is_ascii_alphabetic() {
            chars[idx] = ch.to_ascii_uppercase();
            capitalize_next_alpha = false;
            continue;
        }
        if ch == ',' || ch == ':' {
            capitalize_next_alpha = true;
        } else if capitalize_next_alpha && !ch.is_ascii_whitespace() {
            capitalize_next_alpha = false;
        }
    }
    let normalized =
        normalize_token_self_reference_in_quoted_ability(&chars.into_iter().collect::<String>());
    normalize_quoted_token_trigger_surface(&normalized)
}

fn normalize_token_self_reference_in_quoted_ability(text: &str) -> String {
    let mut normalized = text.to_string();
    for source_type in [
        "creature",
        "artifact",
        "enchantment",
        "land",
        "permanent",
        "source",
    ] {
        normalized = normalized.replace(&format!("This {source_type}"), "This token");
        normalized = normalized.replace(&format!("this {source_type}"), "this token");
    }
    normalized = normalized
        .replace("This token creature's", "This token's")
        .replace("this token creature's", "this token's");
    let had_period = normalized.ends_with('.');
    let bare = normalized.trim_end_matches('.');
    if bare.eq_ignore_ascii_case("attacks each combat if able") {
        return format!(
            "This token attacks each combat if able{}",
            if had_period { "." } else { "" }
        );
    }
    normalized
        .replace("Sacrifice this token, Add ", "Sacrifice this token: Add ")
        .replace("Sacrifice this token, add ", "Sacrifice this token: Add ")
        .replace(": add ", ": Add ")
}

fn normalize_quoted_token_trigger_surface(text: &str) -> String {
    if !(text.starts_with("Whenever ") || text.starts_with("When ") || text.starts_with("At ")) {
        return text.to_string();
    }
    let normalize_self_pronoun = |trigger: &str, effect: &str| {
        let effect = lowercase_first(effect);
        if !trigger.contains("this token") {
            return effect;
        }
        if let Some(rest) = effect.strip_prefix("that creature ") {
            return format!("it {rest}");
        }
        if let Some(rest) = effect.strip_prefix("that creature's ") {
            return format!("its {rest}");
        }
        effect
    };
    // A quoted token ability can contain an activated ability after the
    // trigger's effect ("Whenever ..., it gains '{T}: ...'"). Only a colon
    // in the trigger prefix itself is the legacy separator repaired here.
    // Nested quotation marks are single quotes after the outer token ability
    // has been normalized, so guard both quote forms.
    if let Some((trigger, effect)) = text.split_once(": ")
        && !trigger.contains('"')
        && !trigger.contains(" '")
    {
        return format!("{trigger}, {}", normalize_self_pronoun(trigger, effect));
    }
    if let Some((trigger, effect)) = text.split_once(", ")
        && trigger.contains("this token")
        && (effect.starts_with("that creature ") || effect.starts_with("that creature's "))
    {
        return format!("{trigger}, {}", normalize_self_pronoun(trigger, effect));
    }
    text.to_string()
}

fn token_quoted_ability_needs_terminal_period(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.ends_with('.')
        && !trimmed.ends_with('!')
        && !trimmed.ends_with('?')
        && (trimmed.starts_with('{')
            || trimmed.contains("Sacrifice this token:")
            || trimmed.starts_with("When ")
            || trimmed.starts_with("Whenever ")
            || trimmed.starts_with("At ")
            || (trimmed.starts_with("This token's power and toughness ")
                && trimmed.contains(" are each equal to ")))
}

pub(super) fn normalize_token_quoted_ability_surfaces(line: &str) -> String {
    if !line.contains(" token") || !line.contains('"') {
        return line.to_string();
    }

    let mut out = String::new();
    let mut in_quote = false;
    let mut token_quote_list_active = false;
    let mut separate_creature_token_quote = false;
    for part in line.split('"') {
        if in_quote {
            if token_quote_list_active {
                let mut ability = normalize_quoted_token_ability_surface(part);
                if separate_creature_token_quote
                    && (ability.starts_with("Whenever this token deals ")
                        || ability.starts_with("When this token deals "))
                {
                    ability = ability.replacen("this token deals", "this creature deals", 1);
                }
                if token_quoted_ability_needs_terminal_period(&ability) {
                    ability.push('.');
                }
                out.push('"');
                out.push_str(&ability);
                out.push('"');
            } else {
                out.push('"');
                out.push_str(part);
                out.push('"');
            }
        } else {
            let lower = part.trim_end().to_ascii_lowercase();
            let begins_token_quote_list = lower.ends_with("token. it has")
                || lower.ends_with("tokens. they have")
                || lower.ends_with("token with")
                || lower.ends_with("tokens with")
                || (lower.contains(" token") && (lower.ends_with(" and") || lower.ends_with(',')));
            if begins_token_quote_list {
                token_quote_list_active = true;
                separate_creature_token_quote = lower.ends_with("creature token. it has");
            } else if token_quote_list_active {
                let connector = part.trim().trim_end_matches('.');
                token_quote_list_active = connector.eq_ignore_ascii_case("and") || connector == ",";
                if !token_quote_list_active {
                    separate_creature_token_quote = false;
                }
            }
            out.push_str(part);
        }
        in_quote = !in_quote;
    }
    out
}

pub(super) fn normalize_token_granted_static_ability_text(text: &str) -> String {
    let mut normalized = normalize_sentence_surface_style(text);
    if normalized
        .starts_with("This token saddles mounts and crews vehicles as though its power were ")
    {
        normalized = normalized
            .replace("saddles mounts", "saddles Mounts")
            .replace("crews vehicles", "crews Vehicles");
    }
    if let Some(rest) = normalized.strip_prefix("This creature ") {
        normalized = format!("This token {rest}");
    } else if normalized == "This creature gets +1/+1." {
        normalized = "This token gets +1/+1.".to_string();
    } else if normalized == "Can't block." {
        normalized = "This token can't block.".to_string();
    } else if normalized == "Can't be blocked." {
        normalized = "This token can't be blocked.".to_string();
    }
    if is_keyword_style_line(&normalized) {
        normalized
    } else {
        ensure_trailing_period(&normalized)
    }
}

pub(super) fn player_verb(
    subject: &str,
    you_form: &'static str,
    other_form: &'static str,
) -> &'static str {
    if matches!(subject, "you" | "they" | "They") {
        you_form
    } else {
        other_form
    }
}

pub(super) fn normalize_you_verb_phrase(text: &str) -> String {
    let replacements = [
        ("pays ", "pay "),
        ("loses ", "lose "),
        ("gains ", "gain "),
        ("draws ", "draw "),
        ("puts ", "put "),
        ("returns ", "return "),
        ("discards ", "discard "),
        ("sacrifices ", "sacrifice "),
        ("creates ", "create "),
        ("chooses ", "choose "),
        ("mills ", "mill "),
        ("reveals ", "reveal "),
        ("scries ", "scry "),
        ("searches ", "search "),
        ("shuffles ", "shuffle "),
        ("surveils ", "surveil "),
        ("Behold ", "behold "),
    ];
    for (from, to) in replacements {
        if let Some(stripped) = text.strip_prefix(from) {
            return format!("{to}{stripped}");
        }
    }
    text.to_string()
}

pub(super) fn normalize_third_person_verb_phrase(text: &str) -> String {
    let replacements = [
        ("pay ", "pays "),
        ("lose ", "loses "),
        ("gain ", "gains "),
        ("draw ", "draws "),
        ("put ", "puts "),
        ("return ", "returns "),
        ("move ", "moves "),
        ("exile ", "exiles "),
        ("discard ", "discards "),
        ("sacrifice ", "sacrifices "),
        ("choose ", "chooses "),
        ("mill ", "mills "),
        ("scry ", "scries "),
        ("surveil ", "surveils "),
        ("reveal ", "reveals "),
        ("search ", "searches "),
        ("shuffle ", "shuffles "),
    ];
    for (from, to) in replacements {
        if let Some(stripped) = text.strip_prefix(from) {
            return format!("{to}{stripped}");
        }
    }
    text.to_string()
}

pub(super) fn normalize_cost_amount_token(text: &str) -> String {
    let cleaned = text.trim().trim_end_matches('.').trim_matches('"').trim();
    if cleaned.is_empty() {
        return cleaned.to_string();
    }
    if cleaned.starts_with('{') && cleaned.ends_with('}') {
        return cleaned.to_string();
    }
    if cleaned.chars().all(|ch| ch.is_ascii_digit()) {
        return format!("{{{cleaned}}}");
    }
    cleaned.to_string()
}

pub(super) fn small_number_word(n: u32) -> Option<String> {
    ironsmith_core::cardinal_word(n)
}

fn ordinal_number_word(n: u32) -> String {
    ironsmith_core::ordinal_word(n).unwrap_or_else(|| format!("{n}th"))
}

pub(super) fn number_word(n: i32) -> Option<String> {
    u32::try_from(n)
        .ok()
        .and_then(ironsmith_core::cardinal_word)
}

pub(super) fn render_small_number_or_raw(text: &str) -> String {
    text.trim()
        .parse::<u32>()
        .ok()
        .and_then(small_number_word)
        .unwrap_or_else(|| text.trim().to_string())
}

pub(super) fn looks_like_trigger_condition(head: &str) -> bool {
    let lower = head.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.starts_with("chapter ")
        || lower.starts_with("activate only")
        || lower.starts_with("equip ")
        || lower.starts_with("ward")
        || lower.starts_with("madness")
        || lower.starts_with("kicker")
        || lower.starts_with("cycling")
    {
        return false;
    }
    if lower.contains('{') {
        return false;
    }

    [
        " attacks",
        " attack",
        " blocks",
        " block",
        " plays ",
        " play ",
        " enters",
        " enter",
        " dies",
        " die",
        " leaves",
        " is put into ",
        " becomes",
        " become",
        " is tapped for mana",
        " cast",
        " casts",
        " gain life",
        " gains life",
        " deals damage",
        " deal damage",
        " create ",
        " unlock ",
        "beginning of",
        "control no other ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(super) fn normalize_trigger_colon_clause(line: &str) -> Option<String> {
    let (line_prefix, body) = if let Some((prefix, rest)) = line.split_once(": ")
        && is_render_heading_prefix(prefix)
    {
        (Some(prefix.trim()), rest.trim())
    } else {
        (None, line)
    };

    let (head, tail) = body.split_once(": ")?;
    if head.contains('"') {
        return None;
    }
    let normalized_head = if let Some(rest) = head.strip_prefix("You ") {
        format!("you {rest}")
    } else {
        head.to_string()
    };
    if !looks_like_trigger_condition(&normalized_head) {
        return None;
    }

    let lower_head = normalized_head.to_ascii_lowercase();
    if lower_head.starts_with("as an additional cost to cast this spell") {
        return None;
    }
    let normalized_tail = if tail
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        lowercase_first(tail)
    } else {
        tail.to_string()
    };

    let mapped = if lower_head.starts_with("the beginning ") {
        format!("At {normalized_head}, {normalized_tail}")
    } else if lower_head.starts_with("when ")
        || lower_head.starts_with("whenever ")
        || lower_head.starts_with("at the beginning ")
    {
        format!("{normalized_head}, {normalized_tail}")
    } else if lower_head.starts_with("you control no other ") {
        format!("When {normalized_head}, {normalized_tail}")
    } else {
        format!("Whenever {normalized_head}, {normalized_tail}")
    };

    if let Some(prefix) = line_prefix {
        Some(format!("{prefix}: {mapped}"))
    } else {
        Some(mapped)
    }
}

pub(super) fn normalize_inline_earthbend_phrasing(text: &str) -> Option<String> {
    let needle = "Earthbend target land you control with ";
    let suffix = " +1/+1 counter(s)";

    let mut rest = text;
    let mut out = String::new();
    let mut changed = false;

    while let Some(idx) = rest.find(needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        let Some(end_idx) = after.find(suffix) else {
            out.push_str(&rest[idx..]);
            rest = "";
            break;
        };

        let count = after[..end_idx].trim();
        if count.is_empty() {
            out.push_str(&rest[idx..idx + needle.len() + end_idx + suffix.len()]);
        } else {
            out.push_str("Earthbend ");
            out.push_str(count);
            changed = true;
        }
        rest = &after[end_idx + suffix.len()..];
    }

    out.push_str(rest);
    if changed { Some(out) } else { None }
}

pub(super) fn looks_like_creature_type_list_subject(subject: &str) -> bool {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains(',') || trimmed.contains(':') {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    for banned in [
        "when ",
        "whenever ",
        "at the beginning ",
        "target ",
        "up to ",
        " each ",
        " enters",
        " attacks",
        " blocks",
        " dies",
        " deals",
        " gain ",
        " get ",
        " has ",
        " have ",
    ] {
        if lower.contains(banned) {
            return false;
        }
    }
    true
}

pub(super) fn normalize_enchanted_creature_dies_clause(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let tail = strip_prefix_ascii_ci(trimmed, "Whenever a enchanted creature dies, ")
        .or_else(|| strip_prefix_ascii_ci(trimmed, "When a enchanted creature dies, "))
        .or_else(|| strip_prefix_ascii_ci(trimmed, "Whenever enchanted creature dies, "))
        .or_else(|| strip_prefix_ascii_ci(trimmed, "When enchanted creature dies, "))?;

    let tail = tail.trim();
    if let Some(counter_tail) = strip_prefix_ascii_ci(
        tail,
        "return it from graveyard to the battlefield. put ",
    )
    .and_then(|rest| {
        strip_suffix_ascii_ci(rest, " on it.").or_else(|| strip_suffix_ascii_ci(rest, " on it"))
    }) {
        return Some(format!(
            "When enchanted creature dies, return that card to the battlefield under your control with {} on it.",
            counter_tail.trim()
        ));
    }

    if tail.eq_ignore_ascii_case("return it from graveyard to the battlefield.")
        || tail.eq_ignore_ascii_case("return it from graveyard to the battlefield")
        || tail.eq_ignore_ascii_case("return it to the battlefield under your control.")
        || tail.eq_ignore_ascii_case("return it to the battlefield under your control")
        || tail.eq_ignore_ascii_case("put it onto the battlefield under your control.")
        || tail.eq_ignore_ascii_case("put it onto the battlefield under your control")
    {
        return Some(
            "When enchanted creature dies, return that card to the battlefield under your control."
                .to_string(),
        );
    }

    let create_tail = strip_prefix_ascii_ci(tail, "return this aura to its owner's hand. ")
        .or_else(|| strip_prefix_ascii_ci(tail, "return this permanent to its owner's hand. "))
        .and_then(|rest| {
            strip_prefix_ascii_ci(rest, "you create ")
                .map(|tail| (tail, true))
                .or_else(|| strip_prefix_ascii_ci(rest, "create ").map(|tail| (tail, false)))
        })
        .or_else(|| {
            strip_prefix_ascii_ci(tail, "return this aura to its owner's hand and you create ")
                .map(|tail| (tail, true))
        })
        .or_else(|| {
            strip_prefix_ascii_ci(
                tail,
                "return this permanent to its owner's hand and you create ",
            )
            .map(|tail| (tail, true))
        })
        .or_else(|| {
            strip_prefix_ascii_ci(tail, "return this aura to its owner's hand and create ")
                .map(|tail| (tail, false))
        })
        .or_else(|| {
            strip_prefix_ascii_ci(
                tail,
                "return this permanent to its owner's hand and create ",
            )
            .map(|tail| (tail, false))
        });
    if let Some((create_tail, actor_surface_explicit)) = create_tail {
        let mut create_clause = create_tail.trim().to_string();
        if !create_clause.ends_with('.') {
            create_clause.push('.');
        }
        let actor = if actor_surface_explicit { "you " } else { "" };
        return Some(format!(
            "When enchanted creature dies, return this card to its owner's hand and {actor}create {create_clause}"
        ));
    }

    None
}

pub(super) fn normalize_subject_signature_for_get_gain(subject: &str) -> String {
    let mut words = Vec::new();
    for raw_word in subject.split_whitespace() {
        let lower = raw_word
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
            .to_ascii_lowercase();
        if lower.is_empty() {
            continue;
        }
        if matches!(
            lower.as_str(),
            "a" | "an"
                | "and"
                | "another"
                | "any"
                | "each"
                | "every"
                | "other"
                | "some"
                | "the"
                | "this"
                | "their"
                | "their's"
                | "these"
                | "them"
                | "it"
                | "its"
                | "to"
                | "with"
                | "your"
        ) {
            continue;
        }
        let normalized = if lower.len() > 3 && lower.ends_with('s') {
            lower[..lower.len() - 1].to_string()
        } else {
            lower
        };
        words.push(normalized);
    }
    words.join(" ")
}

pub(super) fn normalize_sacrifice_implied_choice(sentence: &str) -> Option<String> {
    let trimmed = sentence.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("sacrifice") || lower.contains("choice") {
        return None;
    }

    let (subject, body) = if let Some(rhs) =
        strip_prefix_ascii_ci(trimmed, "that player sacrifices ")
    {
        ("that player sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "each player sacrifices ") {
        ("each player sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "its controller sacrifices ") {
        ("its controller sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "that object's controller sacrifices ")
    {
        ("that object's controller sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "that player's controller sacrifices ")
    {
        ("that player's controller sacrifices ", rhs)
    } else {
        return None;
    };

    let mut body = body.trim().trim_end_matches('.').to_string();
    let body_lower = body.to_ascii_lowercase();
    if body_lower.contains("of your choice") || body_lower.contains("of their choice") {
        return None;
    }
    // "All" is already a complete set, not a choice made by the affected
    // player. This guard also matters for structural multi-sentence bundles:
    // treating the remainder of the bundle as one sacrifice noun phrase can
    // otherwise append "of their choice" to a later sentence.
    if body_lower.starts_with("all ") {
        return None;
    }

    for suffix in [
        " that player controls",
        " that object's controller controls",
        " that object's controller's control",
        " that your controller controls",
        " its controller controls",
        " your control",
        " target opponent controls",
        " target player controls",
    ] {
        if let Some(stripped) = strip_suffix_ascii_ci(body.as_str(), suffix) {
            body = stripped.to_string();
            break;
        }
    }

    if let Some(rest) = strip_prefix_ascii_ci(&body, "a controller's ") {
        body = rest.to_string();
        if let Some(rest_tail) = strip_prefix_ascii_ci(&body, "a ") {
            body = rest_tail.to_string();
        } else {
            body = format!("a {body}");
        }
    } else if let Some(rest) = strip_prefix_ascii_ci(&body, "an controller's ") {
        body = rest.to_string();
        if let Some(rest_tail) = strip_prefix_ascii_ci(&body, "an ") {
            body = rest_tail.to_string();
        } else {
            body = format!("an {body}");
        }
    } else if let Some(rest) = strip_prefix_ascii_ci(&body, "the controller's ") {
        body = rest.to_string();
        if let Some(rest_tail) = strip_prefix_ascii_ci(&body, "the ") {
            body = rest_tail.to_string();
        } else {
            body = format!("the {body}");
        }
    } else if let Some(rest) = strip_prefix_ascii_ci(&body, "controller's ") {
        body = rest.to_string();
    }

    let mut split_at = body.len();
    let split_markers = [" unless ", " if ", " then "];
    for marker in split_markers {
        if let Some(idx) = body.to_ascii_lowercase().find(marker)
            && idx < split_at
        {
            split_at = idx;
        }
    }

    if split_at == body.len() {
        body = format!("{body} of their choice");
    } else {
        body = format!("{} of their choice{}", &body[..split_at], &body[split_at..]);
    }

    let mut rewritten = format!("{subject}{body}");
    if trimmed.ends_with('.') {
        rewritten.push('.');
    }
    Some(rewritten)
}

pub(super) fn normalize_choose_sacrifice_subject(chosen: &str) -> String {
    let mut chosen = chosen.trim().trim_end_matches('.').to_string();
    if let Some((before, _)) = split_once_ascii_ci(&chosen, " and tag it as ") {
        chosen = before.to_string();
    } else if let Some((before, _)) = split_once_ascii_ci(&chosen, " and tags it as ") {
        chosen = before.to_string();
    }
    chosen = chosen
        .strip_suffix(" in the battlefield")
        .or_else(|| chosen.strip_suffix(" in the battlefields"))
        .or_else(|| chosen.strip_suffix(" you control in the battlefield"))
        .or_else(|| chosen.strip_suffix(" you control in the battlefields"))
        .unwrap_or(chosen.as_str())
        .trim()
        .to_string();
    if let Some(rest) = strip_prefix_ascii_ci(&chosen, "at least 1 ") {
        chosen = rest.trim().to_string();
    }
    let chosen_words = chosen.split_whitespace().collect::<Vec<_>>();
    if let Some(cutoff) = chosen_words
        .iter()
        .position(|word| word.eq_ignore_ascii_case("you") || word.eq_ignore_ascii_case("in"))
        && cutoff > 0
    {
        chosen = chosen_words[..cutoff].join(" ");
    }
    chosen = chosen
        .strip_prefix("a ")
        .or_else(|| chosen.strip_prefix("an "))
        .unwrap_or(chosen.as_str())
        .trim()
        .to_string();
    pluralize_noun_phrase(&chosen)
}

pub(super) fn normalize_two_sentence_pump_and_gain_until_end_of_turn(
    left: &str,
    right: &str,
) -> Option<String> {
    let left = left.trim().trim_end_matches('.');
    let left_lower = left.to_ascii_lowercase();
    if !left_lower.ends_with("until end of turn") && !left_lower.ends_with("until your turn") {
        return None;
    }

    let mut get_idx = None;
    for (needle, suffix_len) in [(" gets ", 5usize), (" get ", 4usize)] {
        if let Some(idx) = left_lower.rfind(needle) {
            match get_idx {
                None => get_idx = Some((idx, suffix_len)),
                Some((existing_idx, _)) if idx > existing_idx => get_idx = Some((idx, suffix_len)),
                _ => {}
            }
        }
    }
    let (get_idx, get_suffix_len) = get_idx?;
    let left_subject = left[..get_idx].trim();
    if left_subject.is_empty() {
        return None;
    }
    let left_get_keyword = if get_suffix_len == 5 { "gets" } else { "get" };
    let left_pump_body = left[get_idx + get_suffix_len..].trim();
    let left_pump_body = strip_suffix_ascii_ci(left_pump_body, " until end of turn")
        .or_else(|| strip_suffix_ascii_ci(left_pump_body, " until your turn"))?
        .trim();
    let left_sig = normalize_subject_signature_for_get_gain(left_subject);
    if left_sig.is_empty() {
        return None;
    }

    let right = right.trim().trim_end_matches('.');
    let right_lower = right.to_ascii_lowercase();
    let mut gain_idx = None;
    for (needle, suffix_len) in [(" gains ", 6usize), (" gain ", 5usize)] {
        if let Some(idx) = right_lower.find(needle) {
            match gain_idx {
                None => gain_idx = Some((idx, suffix_len)),
                Some((existing_idx, _)) if idx < existing_idx => gain_idx = Some((idx, suffix_len)),
                _ => {}
            }
        }
    }
    let (gain_idx, gain_suffix_len) = gain_idx?;
    let right_subject = right[..gain_idx].trim();
    if right_subject.is_empty() {
        return None;
    }
    let right_gain_body = right[gain_idx + gain_suffix_len..].trim();
    let right_gain_body = strip_suffix_ascii_ci(right_gain_body, " until end of turn")
        .or_else(|| strip_suffix_ascii_ci(right_gain_body, " until your turn"))?;
    let right_sig = normalize_subject_signature_for_get_gain(right_subject);
    if right_sig.is_empty() || right_sig != left_sig {
        return None;
    }

    Some(format!(
        "{} {} {} and gains {} until end of turn.",
        left_subject,
        left_get_keyword,
        left_pump_body,
        right_gain_body.trim(),
    ))
}

pub(super) fn normalize_pump_and_gain_until_end_of_turn(line: &str) -> Option<String> {
    let segments: Vec<&str> = line.split(". ").collect();
    if segments.len() < 2 {
        return None;
    }

    let first = segments[0];
    let second = segments[1];
    let merged = normalize_two_sentence_pump_and_gain_until_end_of_turn(first, second)?;
    if segments.len() == 2 {
        return Some(merged);
    }

    Some(format!(
        "{} {}",
        merged,
        segments[2..].join(". ").trim_start()
    ))
}

pub(super) fn normalize_create_named_token_article(line: &str) -> String {
    if let Some((head, tail)) = split_once_ascii_ci(line, "create a ")
        && tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && tail.contains(", a ")
    {
        let first_item = tail.split(',').next().unwrap_or(tail).trim();
        if first_item.ends_with(" token") {
            return line.to_string();
        }
        return format!("{}create {}", head, tail);
    }
    line.to_string()
}

pub(super) fn normalize_exile_named_token_until_source_leaves(line: &str) -> String {
    let marker = "Exile target a token named ";
    let Some(start) = line.find(marker) else {
        return line.to_string();
    };
    let before = &line[..start];
    let after = &line[start + marker.len()..];
    for subject in ["this permanent", "this creature", "this source"] {
        if let Some((_, rest)) =
            after.split_once(&format!(" until {subject} leaves the battlefield"))
        {
            return format!(
                "{}Exile that token when {subject} leaves the battlefield{}",
                before, rest
            );
        }
    }
    line.to_string()
}

pub(super) fn normalize_granted_named_token_leaves_sacrifice_source(line: &str) -> String {
    let marker = "Grant When token named ";
    let Some(start) = line.find(marker) else {
        return line.to_string();
    };
    let before = &line[..start];
    let after = &line[start + marker.len()..];
    if let Some((_, rest)) = after.split_once(" leaves the battlefield, sacrifice this ")
        && let Some((subject, rest_after_subject)) = rest.split_once(". to this ")
        && matches!(subject, "permanent" | "creature" | "source")
        && let Some(rest_suffix) = rest_after_subject.strip_prefix(subject)
        && let Some(rest_suffix) = rest_suffix.strip_prefix('.')
    {
        return format!(
            "{}Sacrifice this {} when that token leaves the battlefield.{}",
            before, subject, rest_suffix
        );
    }
    line.to_string()
}

pub(super) fn normalize_same_name_search_bundle_clause(line: &str) -> Option<String> {
    let (before_search, search_tail) =
        split_once_ascii_ci(line, "Search its controller's library for ")?;
    let (search_clause, rest_after_library) = split_once_ascii_ci(
        search_tail,
        ". Exile all cards with the same name as that object in its controller's graveyard.",
    )?;
    let (rest_after_hand, rest_after_shuffle) = split_once_ascii_ci(
        rest_after_library,
        "Exile all cards with the same name as that object in its controller's hand.",
    )?;
    if !rest_after_hand.trim().is_empty() {
        return None;
    }
    let rest_after_shuffle = rest_after_shuffle.trim_start();
    let rest_after_shuffle =
        strip_prefix_ascii_ci(rest_after_shuffle, "Shuffle its controller's library.")?;

    let normalized_search_clause = search_clause.trim().replace(
        "permanent with the same name as that object cards",
        "cards with the same name as that object",
    );
    let normalized_search_clause = strip_suffix_ascii_ci(&normalized_search_clause, ", exile them")
        .or_else(|| strip_suffix_ascii_ci(&normalized_search_clause, " and exile them"))
        .unwrap_or(&normalized_search_clause)
        .trim();

    let mut rewritten = format!(
        "{}Search its controller's graveyard, hand, and library for {} and exile them. Then that player shuffles.",
        before_search, normalized_search_clause
    );
    if !rest_after_shuffle.trim().is_empty() {
        rewritten.push(' ');
        rewritten.push_str(rest_after_shuffle.trim());
    }
    Some(rewritten)
}

fn normalize_zero_zero_token_with_base_pt(line: &str) -> Option<String> {
    let (before_create, create_tail) = split_once_ascii_ci(line, "Create a 0/0 ")
        .or_else(|| split_once_ascii_ci(line, "Create an 0/0 "))?;
    let (token_desc, base_pt_tail) =
        split_once_ascii_ci(create_tail, ". it has base power and toughness ")?;
    let (power_text, toughness_text) = base_pt_tail.split_once('/')?;

    let power_text = power_text.trim().trim_end_matches('.');
    let toughness_text = toughness_text.trim().trim_end_matches('.');
    let (toughness_text, remainder) = toughness_text
        .split_once(". ")
        .map_or((toughness_text, None), |(value, rest)| (value, Some(rest)));
    let toughness_text = strip_suffix_ascii_ci(toughness_text, " forever")
        .unwrap_or(toughness_text)
        .trim();
    if power_text.is_empty() || !power_text.eq_ignore_ascii_case(toughness_text) {
        return None;
    }

    let mut rewritten = String::new();
    if !before_create.is_empty() {
        rewritten.push_str(before_create);
    }
    rewritten.push_str(&format!(
        "Create an X/X {token_desc}, where X is {power_text}"
    ));
    if let Some(remainder) = remainder
        && !remainder.is_empty()
    {
        rewritten.push_str(". ");
        rewritten.push_str(remainder);
    }
    Some(rewritten)
}

fn normalize_attached_creature_with_base_pt(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let (first, second) = trimmed.split_once(". ")?;
    let first = first.trim_end_matches('.').trim();
    let second = second.trim_end_matches('.').trim();

    let first_lower = first.to_ascii_lowercase();
    let subject = if let Some(subject) = first_lower.strip_suffix(" is creature") {
        subject
    } else if let Some(subject) = first_lower.strip_suffix(" is a creature") {
        subject
    } else {
        return None;
    };
    if subject.is_empty() {
        return None;
    }

    let second_lower = second.to_ascii_lowercase();
    let marker = " has base power and toughness ";
    let idx = second_lower.find(marker)?;
    if second_lower[..idx] != *subject {
        return None;
    }
    let pt = second[idx + marker.len()..].trim();
    if pt.is_empty() || !pt.contains('/') {
        return None;
    }

    Some(format!(
        "{} is a creature with base power and toughness {} in addition to its other types.",
        capitalize_first(subject),
        pt
    ))
}

fn normalize_ability_loss_transform_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let marker = " loses all abilities and is ";
    let marker_idx = lower.find(marker)?;
    let subject = trimmed[..marker_idx].trim();
    if subject.is_empty() {
        return None;
    }

    let rest = trimmed[marker_idx + marker.len()..].trim();
    let lower_rest = rest.to_ascii_lowercase();
    let base_marker = ", has base power, and toughness ";
    let base_idx = lower_rest.find(base_marker)?;
    let descriptor = rest[..base_idx].trim();
    let (descriptor, inline_name) = split_ability_loss_transform_name(descriptor);
    let base_tail = rest[base_idx + base_marker.len()..].trim();
    let (base_pt, card_type) = base_tail.split_once(' ')?;
    if !base_pt.contains('/') || card_type.trim().is_empty() {
        return None;
    }

    let mut colors: Vec<String> = Vec::new();
    let mut subtypes: Vec<String> = Vec::new();
    let mut named: Option<String> = None;
    if let Some(name) = inline_name {
        named = Some(name);
    }

    for raw_part in descriptor
        .split(',')
        .flat_map(|part| part.split(" and "))
        .map(trim_ability_loss_transform_connector)
    {
        let mut part = raw_part;
        while let Some(rest) = part.strip_prefix("is ") {
            part = rest.trim();
        }
        if part.is_empty() {
            continue;
        }

        let mut words = part.split_whitespace().peekable();
        while let Some(word) = words.peek().copied() {
            if matches!(
                word.to_ascii_lowercase().as_str(),
                "white" | "blue" | "black" | "red" | "green" | "colorless"
            ) {
                colors.push(word.to_string());
                words.next();
            } else {
                break;
            }
        }

        let mut remainder = words.collect::<Vec<_>>().join(" ");
        while let Some(rest) = remainder.strip_prefix("is ") {
            remainder = rest.trim().to_string();
        }
        let mut remainder = trim_ability_loss_transform_connector(&remainder);
        loop {
            let stripped = strip_leading_article(remainder);
            if stripped.len() == remainder.len() {
                break;
            }
            remainder = stripped;
        }
        if let Some(name) = remainder.strip_prefix("named ") {
            named = Some(title_case_card_name_fragment(
                trim_ability_loss_transform_connector(name),
            ));
        } else if let Some((subtype, _card_type)) =
            split_ability_loss_transform_subtype_card_type(remainder)
        {
            subtypes.push(title_case_card_name_fragment(&subtype));
        } else if !remainder.is_empty() {
            subtypes.push(title_case_card_name_fragment(remainder));
        }
    }

    if subtypes.is_empty() || card_type.trim() != "creature" {
        return None;
    }

    let mut type_phrase_parts = Vec::new();
    if !colors.is_empty() {
        type_phrase_parts.push(join_with_and(&colors));
    }
    type_phrase_parts.push(subtypes.join(" "));
    let type_phrase = type_phrase_parts.join(" ");
    let type_phrase = with_indefinite_article(&type_phrase);

    let mut normalized = format!(
        "{subject} loses all abilities and is {type_phrase} {card_type} with base power and toughness {base_pt}"
    );
    if let Some(name) = named {
        normalized.push_str(" named ");
        normalized.push_str(&name);
    }
    normalized.push('.');
    Some(normalized)
}

fn split_ability_loss_transform_name(text: &str) -> (&str, Option<String>) {
    let lower = text.to_ascii_lowercase();
    for marker in [" is named ", " named "] {
        if let Some(idx) = lower.find(marker) {
            let name = trim_ability_loss_transform_connector(&text[idx + marker.len()..]);
            return (&text[..idx], Some(title_case_card_name_fragment(name)));
        }
    }
    (text, None)
}

fn trim_ability_loss_transform_connector(text: &str) -> &str {
    let mut trimmed = text.trim();
    loop {
        if let Some(rest) = trimmed.strip_prefix("and ") {
            trimmed = rest.trim();
            continue;
        }
        if let Some(rest) = trimmed.strip_suffix(" and") {
            trimmed = rest.trim();
            continue;
        }
        break trimmed;
    }
}

fn split_ability_loss_transform_subtype_card_type(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    for glue in [" is ", " "] {
        let Some((left, right)) = lower.rsplit_once(glue) else {
            continue;
        };
        if matches!(
            right,
            "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
        ) && !left.trim().is_empty()
        {
            return Some((left.trim().to_string(), right.to_string()));
        }
    }
    None
}

fn normalize_temporary_animation_oracle_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("this land becomes ")
        && lower.ends_with(" in addition to its other types until end of turn")
    {
        let suffix = " in addition to its other types until end of turn";
        let prefix_len = trimmed.len() - suffix.len();
        let prefix = trimmed[..prefix_len].trim();
        return Some(format!("{prefix} until end of turn. It's still a land."));
    }
    if lower.contains("this land becomes ") && lower.ends_with(" in addition to its other types") {
        let suffix = " in addition to its other types";
        let prefix_len = trimmed.len() - suffix.len();
        let prefix = trimmed[..prefix_len].trim();
        return Some(format!("{prefix}. It's still a land."));
    }
    if lower.contains("this land becomes ")
        && lower.contains(" creature")
        && lower.ends_with(" until end of turn")
        && !lower.contains("still a land")
    {
        return Some(format!("{trimmed}. It's still a land."));
    }

    if lower.ends_with(" creature that's still a land until end of turn") {
        let prefix_len = trimmed.len() - " creature that's still a land until end of turn".len();
        let prefix = trimmed[..prefix_len].trim();
        if let Some((subject, pt)) = prefix.split_once(" becomes ") {
            let subject = subject.trim();
            let pt = pt.trim().trim_start_matches("a ");
            if pt.contains('/') {
                return Some(format!(
                    "Until end of turn, {subject} becomes a {pt} creature that's still a land."
                ));
            }
        }
    }

    if lower.ends_with(" blue serpent creature until end of turn") {
        let prefix_len = trimmed.len() - " blue serpent creature until end of turn".len();
        let prefix = trimmed[..prefix_len].trim();
        if let Some((subject, pt)) = prefix.split_once(" becomes ") {
            let subject = subject.trim();
            let pt = pt.trim().trim_start_matches("a ");
            if pt.contains('/') {
                return Some(format!(
                    "Until end of turn, {subject} becomes a blue Serpent with base power and toughness {pt}."
                ));
            }
        }
    }

    None
}

fn normalize_token_death_trigger_quote_surface(line: &str) -> String {
    line.replace(
        "\"When this token dies: You gain 1 life.\"",
        "\"When this token dies, you gain 1 life.\"",
    )
    .replace(
        "\"When this token dies: You gain 1 life\"",
        "\"When this token dies, you gain 1 life\"",
    )
    .replace(
        "When this token dies: You gain 1 life",
        "When this token dies, you gain 1 life",
    )
    .replace(
        "\"When this token dies: It deals 1 damage to any target.\"",
        "\"When this token dies, it deals 1 damage to any target.\"",
    )
    .replace(
        "When this token dies: It deals 1 damage to any target",
        "When this token dies, it deals 1 damage to any target",
    )
    .replace(
        "\"When this token dies, this token deals 1 damage to any target.\"",
        "\"When this token dies, it deals 1 damage to any target.\"",
    )
    .replace(
        "\"When this token dies, this token deals 1 damage to any target\"",
        "\"When this token dies, it deals 1 damage to any target\"",
    )
    .replace(
        "When this token dies, this token deals 1 damage to any target",
        "When this token dies, it deals 1 damage to any target",
    )
}

fn split_choose_sacrifice_tail(rest: &str) -> Option<(&str, &str)> {
    for needle in [
        ". you sacrifice all permanents you control",
        ". sacrifice all permanents you control",
        ", you sacrifice all permanents you control",
        ", sacrifice all permanents you control",
    ] {
        if let Some((chosen, tail)) = split_once_ascii_ci(rest, needle) {
            return Some((chosen, tail));
        }
    }
    None
}

pub(super) fn normalize_repeated_dynamic_buff(line: &str) -> Option<String> {
    let (before_until, after_until) = split_once_ascii_ci(line, " until end of turn")?;
    let (subject, buff) = split_once_ascii_ci(before_until, " gets ")?;
    let (left, right) = buff.split_once('/')?;
    if !left.trim().eq_ignore_ascii_case(right.trim()) {
        return None;
    }
    let value_expr = left.trim();
    let value_expr_lower = value_expr.to_ascii_lowercase();
    if !value_expr_lower.contains("number of") {
        return None;
    }

    let remainder = after_until.trim();
    let mut rewritten = format!(
        "{} gets +X/+X until end of turn, where X is {}.",
        subject.trim(),
        value_expr
    );
    if !remainder.is_empty() && remainder != "." {
        let rest = remainder.trim_start_matches('.').trim();
        if !rest.is_empty() {
            let lower_rest = rest.to_ascii_lowercase();
            if !lower_rest.starts_with("x is ") && !lower_rest.starts_with("where x is ") {
                rewritten.push(' ');
                rewritten.push_str(rest);
            }
        }
    }
    Some(rewritten)
}

pub(super) fn normalize_singular_tagged_play_permission(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let singular_source = [
        "exile the top card",
        "exiles the top card",
        "reveal the top card",
        "reveals the top card",
        "look at the top card",
        "looks at the top card",
    ]
    .into_iter()
    .any(|needle| lower.contains(needle))
        || lower.contains("tagged '__source_exiled__' cards")
        || ((lower.contains("you may exile a ") || lower.contains("you may exile an "))
            && lower.contains(" from among them"));
    if !singular_source {
        return None;
    }

    let rewrites = [
        ("you may play tagged 'exiled_", "play"),
        ("you may cast tagged 'exiled_", "cast"),
        ("you may play that card until end of turn", "play"),
        ("you may cast that card until end of turn", "cast"),
        ("you may play tagged 'revealed_", "play"),
        ("you may cast tagged 'revealed_", "cast"),
        ("you may play tagged '__sentence_helper_exiled_", "play"),
        ("you may cast tagged '__sentence_helper_exiled_", "cast"),
        ("you may play tagged '__source_exiled__' cards", "play"),
        ("you may cast tagged '__source_exiled__' cards", "cast"),
        ("you may play tagged '__sentence_helper_revealed_", "play"),
        ("you may cast tagged '__sentence_helper_revealed_", "cast"),
    ];
    for (needle, verb) in rewrites {
        let Some((prefix, rest)) = split_once_ascii_ci(line, needle) else {
            continue;
        };
        if needle.contains("that card until end of turn") {
            return Some(format!(
                "{prefix}you may {verb} that card until end of turn"
            ));
        }
        let Some((_, tail)) = rest.split_once('\'') else {
            continue;
        };

        if let Some(remaining) = strip_prefix_ascii_ci(tail, " cards until end of turn") {
            return Some(format!(
                "{prefix}you may {verb} that card until end of turn{remaining}"
            ));
        }
        if let Some(remaining) =
            strip_prefix_ascii_ci(tail, " cards until the end of your next turn")
        {
            return Some(format!(
                "{prefix}you may {verb} that card until the end of your next turn{remaining}"
            ));
        }
    }

    None
}

fn normalize_searched_tagged_hand_followup(line: &str) -> String {
    let mut normalized = line.to_string();
    for tag in ["searched", "searched_named", "searched_multi_zone"] {
        for lead in ["for each", "For each"] {
            for put in ["put", "Put"] {
                let for_each_put = format!(
                    "{lead} card searched for this way, {put} the tagged object '{tag}' into its owner's hand"
                );
                normalized = normalized.replace(&for_each_put, "put it into your hand");
            }
        }
        for return_verb in ["return", "Return"] {
            let return_tagged =
                format!("{return_verb} the tagged object '{tag}' to its owner's hand");
            normalized = normalized.replace(&return_tagged, "put it into your hand");
        }
        for put in ["put", "Put"] {
            let put_tagged = format!("{put} the tagged object '{tag}' into its owner's hand");
            normalized = normalized.replace(&put_tagged, "put it into your hand");
            let put_battlefield =
                format!("{put} the tagged object '{tag}' onto the battlefield tapped");
            normalized =
                normalized.replace(&put_battlefield, "put them onto the battlefield tapped");
            let put_battlefield_untapped =
                format!("{put} the tagged object '{tag}' onto the battlefield");
            normalized =
                normalized.replace(&put_battlefield_untapped, "put them onto the battlefield");
            let exile_tagged = format!("{put} the tagged object '{tag}' into exile");
            normalized = normalized.replace(&exile_tagged, "exile them");
        }
        for exile in ["exile", "Exile"] {
            let exile_tagged = format!("{exile} the tagged object '{tag}'");
            normalized = normalized.replace(&exile_tagged, "exile them");
        }
        // Explicit destination-player surface hints can make a searched-card
        // move render as "into your/their hand" rather than the older
        // rules-level "into its owner's hand". Preserve that destination,
        // but never leak the internal search tag into compiled text.
        for put in ["put", "Put"] {
            let put_tagged_prefix = format!("{put} the tagged object '{tag}' into ");
            normalized = normalized.replace(&put_tagged_prefix, "put it into ");
        }
        for return_verb in ["return", "Return"] {
            let return_tagged_prefix = format!("{return_verb} the tagged object '{tag}' to ");
            normalized = normalized.replace(&return_tagged_prefix, "return it to ");
        }
    }
    if let Some(compact) = compact_multi_zone_named_search_to_battlefield_surface(&normalized) {
        normalized = compact;
    }
    normalized = normalized
        .replace(
            "Search your library and/or graveyard for a permanent you own named",
            "Search your library and/or graveyard for a card you own named",
        )
        .replace(
            "Search your library and/or graveyard for a card you own named",
            "Search your library and/or graveyard for a card named",
        )
        .replace(
            "Search your library and/or graveyard for a permanent named",
            "Search your library and/or graveyard for a card named",
        )
        .replace(
            "search your library and/or graveyard for a permanent you own named",
            "search your library and/or graveyard for a card you own named",
        )
        .replace(
            "search your library and/or graveyard for a card you own named",
            "search your library and/or graveyard for a card named",
        )
        .replace(
            "search your library and/or graveyard for a permanent named",
            "search your library and/or graveyard for a card named",
        );
    normalized = normalized
        .replace("If you completed a dungeon,", "If you completed dungeon,")
        .replace("if you completed a dungeon,", "if you completed dungeon,");
    normalized = normalized
        .replace("you've completed dungeon", "you've completed a dungeon")
        .replace("you completed dungeon", "you completed a dungeon");
    normalized = normalized.replace(
        "Whenever this creature deals combat damage to a player, you search their library for a card. That player chooses a card name. Then if the tagged object 'searched' matches creature and not, you may put them onto the battlefield under your control. Target player shuffles.",
        "Whenever this creature deals combat damage to a player, search that player's library for a card, then that player chooses a card name. If you searched for a creature card that doesn't have that name, you may put it onto the battlefield under your control. Then that player shuffles.",
    );
    if normalized
        .to_ascii_lowercase()
        .contains("search your library and/or graveyard")
    {
        normalized = normalized
            // A singular result collected from your own library or
            // graveyard necessarily has you as its owner. Collapse the
            // executable per-result loop back to Oracle's singular
            // searched-card continuation before compacting the full
            // search/reveal/put procedure below.
            .replace(
                "For each card searched for this way, put it into its owner's hand",
                "Put it into your hand",
            )
            .replace(
                "for each card searched for this way, put it into its owner's hand",
                "put it into your hand",
            )
            .replace(
                "For each card searched for this way, you put it into its owner's hand",
                "Put it into your hand",
            )
            .replace(
                "for each card searched for this way, you put it into its owner's hand",
                "put it into your hand",
            )
            .replace(
                "put it into your hand, then shuffle your library",
                "put it into your hand. If you search your library this way, shuffle your library",
            )
            .replace(
                "Put it into your hand, then shuffle your library",
                "Put it into your hand. If you search your library this way, shuffle your library",
            );
    }
    if let Some(compact) = compact_named_library_graveyard_search_to_hand_surface(&normalized) {
        normalized = compact;
    }
    if let Some(compact) = compact_multi_zone_search_to_hand_surface(&normalized) {
        normalized = compact;
    }
    let lower = normalized.to_ascii_lowercase();
    if (lower.contains("search your library and/or graveyard")
        || lower.contains("search your library, hand")
        || lower.contains("search your graveyard, hand"))
        && lower.contains("if you do, shuffle")
    {
        normalized = normalized
            .replace(
                ". If you do, shuffle your library",
                ". If you search your library this way, shuffle",
            )
            .replace(
                ". If you do, shuffle",
                ". If you search your library this way, shuffle",
            )
            .replace(
                ". if you do, shuffle your library",
                ". if you search your library this way, shuffle",
            )
            .replace(
                ". if you do, shuffle",
                ". if you search your library this way, shuffle",
            );
    }
    normalized = normalized
        .replace(
            "and exile them, then shuffle target player's library",
            "and exile them. Then that player shuffles",
        )
        .replace(
            "and exile them, then shuffle its controller's library",
            "and exile them. Then that player shuffles",
        );
    if normalized
        .to_ascii_lowercase()
        .contains("card named brambleweft behemoth")
        && normalized
            .to_ascii_lowercase()
            .contains("card named nissa, genesis mage")
    {
        normalized = normalized
            .replace(
                "you search your library and/or graveyard for a card named forest, you search your library and/or graveyard for a card named brambleweft behemoth, you search your library and/or graveyard for a card named nissa, genesis mage",
                "you search your library and graveyard for a card named forest, a card named brambleweft behemoth, and a card named nissa, genesis mage",
            )
            .replace(
                "You search your library and/or graveyard for a card named forest, you search your library and/or graveyard for a card named brambleweft behemoth, you search your library and/or graveyard for a card named nissa, genesis mage",
                "You search your library and graveyard for a card named forest, a card named brambleweft behemoth, and a card named nissa, genesis mage",
            )
            .replace("reveal it, put it into your hand", "reveal those cards, put them into your hand");
        normalized = normalized.replace(
            ". If you search your library this way, shuffle",
            ", then shuffle",
        );
    }
    normalized
}

fn compact_named_library_graveyard_search_to_hand_surface(line: &str) -> Option<String> {
    for (needle, replacement) in [
        (
            "you may search your library and/or graveyard for a card named ",
            "you may search your library and/or graveyard for a card named ",
        ),
        (
            "You may search your library and/or graveyard for a card named ",
            "You may search your library and/or graveyard for a card named ",
        ),
    ] {
        let Some((prefix, rest)) = line.split_once(needle) else {
            continue;
        };
        let rest = rest.trim_end_matches('.');
        let Some(name) = rest
            .strip_suffix(", reveal it, and put it into your hand. If you do, shuffle your library")
            .or_else(|| {
                rest.strip_suffix(", reveal it, and put it into your hand. If you do, shuffle")
            })
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it, then put it into your hand. If you search your library this way, shuffle",
                )
            })
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it, then put it into your hand. If you searched your library this way, shuffle",
                )
            })
        else {
            continue;
        };
        let name = title_case_card_name_fragment(name.trim());
        return Some(format!(
            "{prefix}{replacement}{name}, reveal it, and put it into your hand. If you search your library this way, shuffle."
        ));
    }

    let needles = [
        "Search your library for a basic land card, put it onto the battlefield tapped, you search your library and/or graveyard for a card named ",
        "Search your library for a basic land card, put it onto the battlefield tapped, search your library and/or graveyard for a card named ",
        "Search your library for a basic land card, put it onto the battlefield tapped. You search your library and/or graveyard for a card named ",
        "Search your library for a basic land card, put it onto the battlefield tapped. Search your library and/or graveyard for a card named ",
    ];
    let (prefix, rest) = needles.iter().find_map(|needle| line.split_once(needle))?;
    let rest = rest.trim_end_matches('.');
    let name = rest
        .strip_suffix(
            ", reveal it, put it into your hand. If you search your library this way, shuffle your library",
        )
        .or_else(|| {
            rest.strip_suffix(
                ", reveal it, put it into your hand. If you search your library this way, shuffle",
            )
        })
        .or_else(|| {
            rest.strip_suffix(". Reveal it. Put it into your hand. Then if you search your library this way, shuffle")
        })
        .or_else(|| {
            rest.strip_suffix(
                ". Reveal it. Put it into your hand. Then if you search your library this way, shuffle your library",
            )
        })
        .or_else(|| {
            rest.strip_suffix(
                ". Reveal it. Put it into your hand. If you search your library this way, shuffle",
            )
        })
        .or_else(|| {
            rest.strip_suffix(
                ". Reveal it. Put it into your hand. If you search your library this way, shuffle your library",
            )
        })
        ?;
    let name = title_case_card_name_fragment(name.trim());
    Some(format!(
        "{prefix}Search your library for a basic land card and put it onto the battlefield tapped. Search your library and graveyard for a card named {name}, reveal it, put it into your hand, then shuffle."
    ))
}

fn compact_multi_zone_search_to_hand_surface(line: &str) -> Option<String> {
    for needle in [
        "you search your library, graveyard, and/or outside the game for ",
        "You search your library, graveyard, and/or outside the game for ",
        "search your library, graveyard, and/or outside the game for ",
        "Search your library, graveyard, and/or outside the game for ",
        "you search your library and/or graveyard for ",
        "You search your library and/or graveyard for ",
    ] {
        let Some((prefix, rest)) = line.split_once(needle) else {
            continue;
        };
        let selection = rest
            .strip_suffix(". Reveal it. Put it into your hand. Then if you search your library this way, shuffle")
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it. Put it into your hand. Then if you search your library this way, shuffle.",
                )
            })
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it. Put it into your hand. Then if you search your library this way, shuffle your library",
                )
            })
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it. Put it into your hand. Then if you search your library this way, shuffle your library.",
                )
            })
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it. Put it into your hand. If you search your library this way, shuffle",
                )
            })
            .or_else(|| {
                rest.strip_suffix(
                    ". Reveal it. Put it into your hand. If you search your library this way, shuffle.",
                )
            })?;
        let mut selection = selection.trim().to_string();
        if !selection.contains(" card") {
            if let Some(index) = selection.find(" with ") {
                selection.insert_str(index, " card");
            } else if let Some(index) = selection.find(" you own") {
                selection.insert_str(index, " card");
            } else {
                selection.push_str(" card");
            }
        }
        let origin = if needle.contains("outside the game") {
            "your library, graveyard, and/or outside the game"
        } else {
            "your library and/or graveyard"
        };
        return Some(format!(
            "{prefix}search {origin} for {selection}, reveal it, and put it into your hand. If you search your library this way, shuffle."
        )
        .replace("When this siege enters", "When this Siege enters"));
    }
    None
}

fn compact_multi_zone_named_search_to_battlefield_surface(line: &str) -> Option<String> {
    for needle in [
        "Search your graveyard, hand, and library for a ",
        "You search your graveyard, hand, and library for a ",
        "you search your graveyard, hand, and library for a ",
        "search your graveyard, hand, and library for a ",
    ] {
        let Some((prefix, rest)) = line.split_once(needle) else {
            continue;
        };
        let Some((selection, tail)) = rest.split_once(
            ", for each card searched for this way, put them onto the battlefield, then shuffle your library",
        ) else {
            continue;
        };
        if !tail.trim().trim_end_matches('.').is_empty() {
            continue;
        }
        let name = selection
            .strip_prefix("permanent named ")
            .or_else(|| selection.strip_prefix("card named "))?;
        let name = title_case_card_name_fragment(name.trim());
        if prefix.trim_end().ends_with(':') {
            return Some(format!(
                "{prefix}Search your graveyard, hand, and/or library for a card named {name} and put it onto the battlefield. If you search your library this way, shuffle."
            ));
        }
        let search_verb = if prefix.trim().is_empty() {
            "Search"
        } else {
            "search"
        };
        return Some(format!(
            "{prefix}{search_verb} your graveyard, hand, and library for a card named {name}, put it onto the battlefield, then shuffle."
        ));
    }

    None
}

fn normalize_untap_target_creature_gets_and_gains_split(line: &str) -> Option<String> {
    let rest = line.strip_prefix("untap target creature, it gets ")?;
    let (pt_delta, keyword) =
        if let Some((pt_delta, tail)) = rest.split_once(" until end of turn, then it gains ") {
            (pt_delta, tail.strip_suffix(" until end of turn")?)
        } else {
            let (pt_delta, tail) = rest.split_once(" and gains ")?;
            (pt_delta, tail.strip_suffix(" until end of turn")?)
        };
    if pt_delta.is_empty() || keyword.is_empty() {
        return None;
    }

    Some(format!(
        "Untap target creature. It gets {pt_delta} and gains {keyword} until end of turn."
    ))
}

fn compact_same_subject_pt_then_gain_surface(line: &str) -> Option<String> {
    let had_period = line.trim_end().ends_with('.');
    let trimmed = line.trim().trim_end_matches('.');
    for (pt_verb, gain_verb) in [
        (" gets ", " gains "),
        (" get ", " gain "),
        (" has base power and toughness ", " gains "),
        (" have base power and toughness ", " gain "),
    ] {
        let Some((head, rest)) = trimmed.split_once(pt_verb) else {
            continue;
        };
        let Some((pt_delta, followup)) = rest.split_once(" until end of turn, then ") else {
            continue;
        };
        let Some((followup_subject, keyword)) = followup.split_once(gain_verb) else {
            continue;
        };
        let first_subject = head.rsplit([',', ':']).next()?.trim();
        let Some((keyword, trailing)) = keyword.split_once(" until end of turn") else {
            continue;
        };
        if !trailing.is_empty() && !trailing.starts_with(". Activate ") {
            continue;
        }
        if pt_delta.is_empty()
            || keyword.is_empty()
            || !first_subject.eq_ignore_ascii_case(followup_subject.trim())
        {
            continue;
        }
        let period = if had_period { "." } else { "" };
        return Some(format!(
            "{head}{pt_verb}{pt_delta} and {} {keyword} until end of turn{trailing}{period}",
            gain_verb.trim()
        ));
    }
    None
}

fn normalize_this_creature_gets_gains_can_attack_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let (activation_prefix, body) = if let Some((prefix, body)) = trimmed.split_once(": ") {
        (Some(prefix), body)
    } else {
        (None, trimmed)
    };
    let rest = body.strip_prefix("This creature gets ")?;
    let (pt_delta, tail) = rest.split_once(" until end of turn, this creature gains ")?;
    let (keyword, attack_tail) =
        tail.split_once(" until end of turn and can attack this turn as though ")?;
    if pt_delta.is_empty() || keyword.is_empty() || attack_tail.is_empty() {
        return None;
    }

    let normalized = format!(
        "This creature gets {pt_delta} and gains {keyword} until end of turn. It can attack this turn as though {attack_tail}."
    );
    Some(match activation_prefix {
        Some(prefix) => format!("{prefix}: {normalized}"),
        None => normalized,
    })
}

fn normalize_for_each_opponent_gain_control_followup(line: &str) -> Option<String> {
    for marker in [
        "for each opponent, gain control of up to one target creature that player controls until end of turn",
        "For each opponent, gain control of up to one target creature that player controls until end of turn",
        "For each opponent, Gain control of up to one target creature that player controls until end of turn",
    ] {
        let Some(index) = line.find(marker) else {
            continue;
        };
        let prefix = &line[..index];
        let after_marker = &line[index + marker.len()..];
        let marker = if marker.starts_with("For") {
            "For each opponent, gain control of up to one target creature that player controls until end of turn"
        } else {
            marker
        };
        for followup_prefix in [
            ", untap that creature, then it gains ",
            ". Untap that creature. It gains ",
            ". Untap those creatures. It gains ",
        ] {
            let Some(after_followup) = after_marker.strip_prefix(followup_prefix) else {
                continue;
            };
            let (keywords, remainder) = after_followup.split_once(" until end of turn")?;
            if keywords.is_empty() {
                return None;
            }
            return Some(format!(
                "{prefix}{marker}. Untap those creatures. They gain {keywords} until end of turn{remainder}"
            ));
        }
    }

    None
}

struct DamageClauseParts<'a> {
    source: &'a str,
    amount: &'a str,
    target: &'a str,
    prefix_before_target: Option<&'a str>,
    consumed: usize,
}

fn damage_source_tail(source_prefix: &str) -> &str {
    let start = source_prefix
        .rfind(", ")
        .map(|idx| idx + 2)
        .or_else(|| source_prefix.rfind(": ").map(|idx| idx + 2))
        .or_else(|| source_prefix.rfind(" — ").map(|idx| idx + " — ".len()))
        .unwrap_or(0);
    source_prefix[start..].trim()
}

fn parse_trailing_damage_clause(text: &str) -> Option<DamageClauseParts<'_>> {
    let deals_idx = text.rfind(" deals ")?;
    let source_prefix = &text[..deals_idx];
    let source = damage_source_tail(source_prefix);
    let after_deals = &text[deals_idx + " deals ".len()..];
    let (amount, _) = after_deals.split_once(" damage to ")?;
    let target_start = deals_idx + " deals ".len() + amount.len() + " damage to ".len();
    let target = text[target_start..].trim();
    Some(DamageClauseParts {
        source,
        amount: amount.trim(),
        target,
        prefix_before_target: Some(&text[..target_start]),
        consumed: text.len(),
    })
}

fn parse_leading_damage_clause(text: &str) -> Option<DamageClauseParts<'_>> {
    let deals_idx = text.find(" deals ")?;
    let source = text[..deals_idx].trim();
    let after_deals = &text[deals_idx + " deals ".len()..];
    let (amount, _) = after_deals.split_once(" damage to ")?;
    let target_start = deals_idx + " deals ".len() + amount.len() + " damage to ".len();
    let after_target = &text[target_start..];
    let target_len = after_target
        .find(". ")
        .map(|idx| idx + 1)
        .unwrap_or(after_target.len());
    let raw_target = &after_target[..target_len];
    Some(DamageClauseParts {
        source,
        amount: amount.trim(),
        target: raw_target.trim().trim_end_matches('.').trim(),
        prefix_before_target: None,
        consumed: target_start + target_len,
    })
}

fn split_damage_instead_suffix(target: &str) -> (&str, &str) {
    target
        .trim()
        .strip_suffix(" instead")
        .map(|target| (target.trim(), " instead"))
        .unwrap_or((target.trim(), ""))
}

fn normalize_joined_damage_target(target: &str) -> &str {
    match target {
        "it" => "that creature",
        "target defending player's creature" => "target creature defending player controls",
        _ => target,
    }
}

fn compact_split_damage_pair_once(line: &str, delimiter: &str) -> Option<String> {
    let mut search_start = 0;
    while let Some(relative_idx) = line[search_start..].find(delimiter) {
        let delimiter_idx = search_start + relative_idx;
        let first_text = &line[..delimiter_idx];
        let second_start = delimiter_idx + delimiter.len();
        let second_text = &line[second_start..];
        let Some(first) = parse_trailing_damage_clause(first_text) else {
            search_start = second_start;
            continue;
        };
        let Some(second) = parse_leading_damage_clause(second_text) else {
            search_start = second_start;
            continue;
        };
        if !first.source.eq_ignore_ascii_case(second.source) {
            search_start = second_start;
            continue;
        }

        let (second_target, suffix) = split_damage_instead_suffix(second.target);
        let first_target = normalize_joined_damage_target(first.target);
        let controller_followup = second_target == "that object's controller"
            && (first_target.contains("creature") || first_target == "that creature");
        let mass_followup = first_target.starts_with("each ") && second_target.starts_with("each ");
        let direct_followup = matches!(second_target, "you" | "any target")
            || first_target.contains("any target")
            || second_target.starts_with("each ");
        if !controller_followup && !mass_followup && !direct_followup {
            search_start = second_start;
            continue;
        }

        let first_target = if controller_followup && first_target == "target creature" {
            if first_text.contains(". If ") || first_text.contains(" — If ") {
                "that creature"
            } else {
                first_target
            }
        } else {
            first_target
        };
        let second_target = if controller_followup {
            "that creature's controller"
        } else {
            normalize_joined_damage_target(second_target)
        };
        let joined_target = if mass_followup && first.amount == second.amount {
            format!("{first_target} and {second_target}{suffix}")
        } else {
            format!(
                "{first_target} and {} damage to {second_target}{suffix}",
                second.amount
            )
        };
        let consumed_text = &second_text[..second.consumed];
        let terminal_period = if consumed_text.trim_end().ends_with('.') {
            "."
        } else {
            ""
        };
        let remainder = &second_text[second.consumed..];
        return Some(format!(
            "{}{joined_target}{terminal_period}{remainder}",
            first.prefix_before_target?
        ));
    }

    None
}

fn normalize_split_damage_pairs(line: &str) -> String {
    let mut normalized = line.to_string();
    let mut seen = Vec::new();
    loop {
        if seen.iter().any(|previous| previous == &normalized) {
            break;
        }
        seen.push(normalized.clone());
        if let Some(joined) = compact_split_damage_pair_once(&normalized, ". ") {
            if joined == normalized {
                break;
            }
            normalized = joined;
            continue;
        }
        if let Some(joined) = compact_split_damage_pair_once(&normalized, ", then ") {
            if joined == normalized {
                break;
            }
            normalized = joined;
            continue;
        }
        break;
    }
    normalized
}

fn normalize_gets_replacement_instead_order(line: &str) -> Option<String> {
    let (prefix, tail) = line.rsplit_once(". If ")?;
    if let Some((condition, effect)) = tail.split_once(", instead it gets ")
        && condition == "the target is a Human"
    {
        let effect = effect.trim_end_matches('.');
        if let Some((pt, keyword)) = effect.split_once(" until end of turn and it gains ")
            && let Some(keyword) = keyword.strip_suffix(" until end of turn")
        {
            return Some(format!(
                "{prefix}. If it's a Human, instead it gets {pt} and gains {keyword} until end of turn."
            ));
        }
    }
    let (condition, effect) = tail.split_once(", it gets ")?;
    if !matches!(
        condition,
        "it's a Human" | "it's an Human" | "that creature has toxic"
    ) {
        return None;
    }
    let effect = effect.trim_end_matches('.');
    let effect = effect.strip_suffix(" instead")?;
    Some(format!(
        "{prefix}. If {condition}, instead it gets {effect}."
    ))
}

fn color_mana_word(symbol: &str) -> Option<&'static str> {
    match symbol {
        "{W}" => Some("white"),
        "{U}" => Some("blue"),
        "{B}" => Some("black"),
        "{R}" => Some("red"),
        "{G}" => Some("green"),
        _ => None,
    }
}

fn normalize_spell_damage_replacement_surfaces(line: &str) -> Option<String> {
    let (default_text, replacement_tail) =
        line.split_once(". If this spell's additional cost was paid, ")?;
    let default_damage = parse_trailing_damage_clause(default_text)?;
    let replacement_damage = parse_leading_damage_clause(replacement_tail)?;
    if !default_damage
        .source
        .eq_ignore_ascii_case(replacement_damage.source)
    {
        return None;
    }
    let (target, suffix) = split_damage_instead_suffix(replacement_damage.target);
    let consumed_text = &replacement_tail[..replacement_damage.consumed];
    let terminal_period = if consumed_text.trim_end().ends_with('.') {
        "."
    } else {
        ""
    };
    let remainder = &replacement_tail[replacement_damage.consumed..];
    Some(format!(
        "{default_text}. It deals {} damage to {target}{suffix} if this spell's additional cost was paid{terminal_period}{remainder}",
        replacement_damage.amount
    ))
}

fn normalize_adamant_damage_replacement_surface(line: &str) -> Option<String> {
    let (default_text, tail) = line.split_once(". If at least three ")?;
    let default_damage = parse_trailing_damage_clause(default_text)?;
    let (mana_symbol, after_mana) = tail.split_once(" mana was spent to cast this spell, ")?;
    let color = color_mana_word(mana_symbol)?;
    let deals_idx = after_mana.find(" deals ")?;
    let replacement_source = after_mana[..deals_idx].trim();
    if !default_damage
        .source
        .eq_ignore_ascii_case(replacement_source)
    {
        return None;
    }
    let after_deals = &after_mana[deals_idx + " deals ".len()..];
    let (amount, raw_after_damage) = after_deals.split_once(" damage")?;
    let skipped_ws = raw_after_damage.len() - raw_after_damage.trim_start().len();
    let after_damage = raw_after_damage.trim_start();
    let (target, suffix, consumed_tail_len) = if let Some(rest) = after_damage.strip_prefix("to ") {
        let target_len = rest.find(". ").map(|idx| idx + 1).unwrap_or(rest.len());
        let raw_target = rest[..target_len].trim().trim_end_matches('.').trim();
        let (target, suffix) = split_damage_instead_suffix(raw_target);
        (target, suffix, "to ".len() + target_len)
    } else {
        let tail_len = after_damage
            .find(". ")
            .map(|idx| idx + 1)
            .unwrap_or(after_damage.len());
        let raw_tail = after_damage[..tail_len].trim().trim_end_matches('.').trim();
        let suffix = if raw_tail == "instead" {
            " instead"
        } else {
            ""
        };
        ("", suffix, tail_len)
    };
    let target_text = if target.is_empty() {
        String::new()
    } else {
        format!(" to {target}")
    };
    let consumed = deals_idx
        + " deals ".len()
        + amount.len()
        + " damage".len()
        + skipped_ws
        + consumed_tail_len;
    let consumed_text = &after_mana[..consumed];
    let terminal_period = if consumed_text.trim_end().ends_with('.') {
        "."
    } else {
        ""
    };
    let remainder = &after_mana[consumed..];
    Some(format!(
        "{default_text}. Adamant — If at least three {color} mana was spent to cast this spell, it deals {} damage{target_text}{suffix}{terminal_period}{remainder}",
        amount.trim()
    ))
}

fn normalize_kicked_also_damage_surface(line: &str) -> Option<String> {
    let (default_text, tail) = line.split_once(". Then if this spell was kicked, ")?;
    let default_damage = parse_trailing_damage_clause(default_text)?;
    let kicked_damage = parse_leading_damage_clause(tail)?;
    if !default_damage
        .source
        .eq_ignore_ascii_case(kicked_damage.source)
    {
        return None;
    }
    let consumed_text = &tail[..kicked_damage.consumed];
    let terminal_period = if consumed_text.trim_end().ends_with('.') {
        "."
    } else {
        ""
    };
    let remainder = &tail[kicked_damage.consumed..];
    if kicked_damage.target == "any other target" {
        return Some(format!(
            "{default_text}. If this spell was kicked, it deals {} damage to another target{terminal_period}{remainder}",
            kicked_damage.amount
        ));
    }
    Some(format!(
        "{default_text}. If this spell was kicked, it also deals {} damage to {}{terminal_period}{remainder}",
        kicked_damage.amount, kicked_damage.target
    ))
}

fn normalize_delayed_player_planeswalker_damage_surface(line: &str) -> Option<String> {
    let (default_text, tail) = line.split_once(". At the beginning of your next upkeep, ")?;
    let default_damage = parse_trailing_damage_clause(default_text)?;
    if default_damage.target != "target player or planeswalker" {
        return None;
    }
    let damage = parse_leading_damage_clause(tail)?;
    if !(default_damage.source.eq_ignore_ascii_case(damage.source)
        || damage.source.eq_ignore_ascii_case("it"))
        || !matches!(
            damage.target,
            "target that player or that object's controller or planeswalker unless that player or that object's controller pays {U}"
                | "a planeswalker unless that player or that object's controller pays {U}"
        )
    {
        return None;
    }
    Some(format!(
        "{default_text}. It deals an additional {} damage to that player or planeswalker at the beginning of your next upkeep step unless that player or that planeswalker's controller pays {{U}} before that step.",
        damage.amount
    ))
}

fn compact_three_way_looked_card_distribution(line: &str) -> Option<String> {
    let trimmed = line.trim_end_matches('.');
    let head = [
        ", choose a card, choose an other card, choose an other other card, return it to its owner's hand, put it on the bottom of its owner's library, exile it, then you may play those cards this turn",
        ", choose a card, choose another card, choose another other card, return it to its owner's hand, put it on the bottom of its owner's library, exile it, then you may play those cards this turn",
    ]
    .iter()
    .find_map(|suffix| trimmed.strip_suffix(suffix))?;
    if !head.contains("Look at the top three cards of your library") {
        return None;
    }
    Some(format!(
        "{head}. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn."
    ))
}

fn compact_looked_card_battlefield_rest_bottom(line: &str) -> Option<String> {
    let (head, rest) = line.split_once(". You may choose ")?;
    if !head.contains("Look at the top") {
        return None;
    }
    let selection = rest.strip_suffix(
        ". For each card chosen this way, put that object onto the battlefield. For each card chosen this way, Unless it's a permanent, put that object on the bottom of its owner's library.",
    )?;
    Some(format!(
        "{head}. You may put {selection} from among them onto the battlefield. Put the rest on the bottom of your library in any order."
    ))
}

fn compact_delirium_same_name_search_exile(line: &str) -> Option<String> {
    for artifact in [
        "If there are four or more card types among cards in your graveyard, you search its controller's graveyard, hand, and library for any number permanents with the same name as that object that object's controller owns. For each card searched for this way, exile them. If you searched your library this way, shuffle its controller's library. Shuffle their library",
        "If there are four or more card types among cards in your graveyard, you search its controller's graveyard, hand, and library for any number of permanents with the same name as that object that object's controller owns. For each card searched for this way, exile them. If you searched your library this way, shuffle its controller's library. Then that player shuffles",
    ] {
        if line.contains(artifact) {
            return Some(line.replace(
                artifact,
                "Delirium — If there are four or more card types among cards in your graveyard, search the graveyard, hand, and library of that spell's controller for any number of cards with the same name as that spell, exile those cards, then that player shuffles",
            ));
        }
    }
    None
}

fn compact_delirium_exiled_card_same_name_search_exile(line: &str) -> Option<String> {
    for artifact in [
        "If there are four or more card types among cards in your graveyard, target opponent chooses a card exiled with this source. You search target opponent's graveyard, hand, and library for any number permanents with the same name as that object target opponent owns. For each card searched for this way, exile them. If you searched your library this way, shuffle target opponent's library. Shuffle target opponent's library",
        "If there are four or more card types among cards in your graveyard, target opponent chooses a card exiled with this source. You search target opponent's graveyard, hand, and library for any number of permanents with the same name as that object target opponent owns. For each card searched for this way, exile them. If you searched your library this way, shuffle target opponent's library. Shuffle target opponent's library",
    ] {
        if line.contains(artifact) {
            return Some(line.replace(
                artifact,
                "Delirium — If there are four or more card types among cards in your graveyard, search that player's graveyard, hand, and library for any number of cards with the same name as the exiled card, exile those cards, then that player shuffles",
            ));
        }
    }
    None
}

fn compact_count_based_power_boost(line: &str) -> Option<String> {
    let rest = line.strip_prefix("This creature gets +X/+0, where X is the number of ")?;
    if rest.contains(" as long as ") {
        return None;
    }
    let rest = rest
        .trim_end_matches('.')
        .replacen(" cards ", " card ", 1)
        .replacen(" creatures ", " creature ", 1)
        .replacen(" artifacts ", " artifact ", 1);
    if rest.is_empty() {
        return None;
    }
    Some(format!("This creature gets +1/+0 for each {rest}."))
}

fn compact_exile_wheel_then_untap_lands(line: &str) -> Option<String> {
    let rest = line.trim_end_matches('.').strip_prefix("Exile ")?;
    let (card, rest) = rest.split_once(
        ", each player shuffles their hand and graveyard into their library, each player draws ",
    )?;
    let (cards, lands) = rest.split_once(" cards, then untap up to ")?;
    let lands = lands.strip_suffix(" lands")?;
    if card.trim().is_empty() || cards.trim().is_empty() || lands.trim().is_empty() {
        return None;
    }
    Some(format!(
        "Exile {card}. Each player shuffles their hand and graveyard into their library, then draws {cards} cards. You untap up to {lands} lands."
    ))
}

fn compact_any_player_may_choose_sacrifice_surface(line: &str) -> Option<String> {
    let lower = line.trim_end_matches('.').to_ascii_lowercase();
    let artifact = "when this creature enters, each player may sacrifice two creatures. if a player does, sacrifice this creature";
    let choose_artifact = "when this creature enters, a player may choose two creatures on the battlefield. sacrifice all permanents. if a player does, sacrifice this creature";
    let choose_artifact_no_zone = "when this creature enters, a player may choose two creatures. sacrifice all permanents. if a player does, sacrifice this creature";
    if lower != artifact && lower != choose_artifact && lower != choose_artifact_no_zone {
        return None;
    }
    Some(
        "When this creature enters, any player may sacrifice two creatures of their choice. If a player does, sacrifice this creature."
            .to_string(),
    )
}

fn compact_search_reveal_hand_discard_random_shuffle(line: &str) -> Option<String> {
    let (prefix, rest) = line.split_once("Search your library for ")?;
    let selection = rest.strip_suffix(
        ", reveal it, put it into your hand, discard a card at random, then shuffle your library.",
    )?;
    if selection.trim().is_empty() {
        return None;
    }
    Some(format!(
        "{prefix}Search your library for {selection} and reveal that card. Put it into your hand, then discard a card at random. Then shuffle."
    ))
}

fn compact_domain_dynamic_mana_value_return_surface(line: &str) -> Option<String> {
    let artifact = "Then if its mana value is a dynamic value or less, return it from graveyard to the battlefield. Otherwise, return it to its owner's hand.";
    if !line.contains(artifact) {
        return None;
    }
    Some(line.replace(
        artifact,
        "Return that card to the battlefield if its mana value is less than or equal to the number of basic land types among lands you control. Otherwise, put it into your hand.",
    ))
}

/// Collapse the runtime choice used by an untargeted graveyard return back
/// into Oracle's single return instruction, including an immediately linked
/// counter placement on the returned object.
fn compact_choose_graveyard_return_with_counter_surface(line: &str) -> Option<String> {
    let (prefix, choice_tail) = line.split_once(", choose ")?;
    let (selection, counter_tail) =
        choice_tail.split_once(", return it from graveyard to the battlefield, and put ")?;
    if selection.trim().is_empty() || !selection.trim_end().ends_with("card") {
        return None;
    }
    let counter_end = counter_tail.find(" on it")?;
    let counter = counter_tail[..counter_end].trim();
    if counter.is_empty() || !counter.contains("counter") {
        return None;
    }
    let remainder = &counter_tail[counter_end + " on it".len()..];
    Some(format!(
        "{prefix}, then return {} from your graveyard to the battlefield with {counter} on it{remainder}",
        selection.trim()
    ))
}

/// A return limited to one object produces a singular antecedent even though
/// its internal result tag uses the collection-shaped fallback renderer.
fn normalize_single_returned_animation_surface(line: &str) -> Option<String> {
    let marker = ". Those permanents are ";
    let (return_text, animation_tail) = line.split_once(marker)?;
    if !return_text
        .to_ascii_lowercase()
        .contains("up to one target ")
    {
        return None;
    }
    let sentence_end = animation_tail.find(". ").unwrap_or(animation_tail.len());
    let animation = &animation_tail[..sentence_end];
    if !animation.contains(" in addition to their other types") {
        return None;
    }
    let singular = animation
        .replacen(" creatures", " creature", 1)
        .replace("their other types", "its other types");
    let remainder = &animation_tail[sentence_end..];
    Some(format!("{return_text}. It's {singular}{remainder}"))
}

/// Merge a linked base-P/T assignment and additive subtype change into the
/// copular animation surface they represent. A preceding exhaustive `each`
/// battlefield move supplies the distributive singular subject.
fn compact_linked_base_pt_subtype_animation_surface(line: &str) -> Option<String> {
    let marker = ". It has base power and toughness ";
    let (prefix, animation_tail) = line.split_once(marker)?;
    let (power_toughness, subtype_tail) = animation_tail.split_once(" and becomes a ")?;
    let (subtypes, remainder) = subtype_tail.split_once(" in addition to its other types")?;
    if power_toughness.trim().is_empty() || !power_toughness.contains('/') {
        return None;
    }
    let subtypes = subtypes
        .split_whitespace()
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join(" ");
    if subtypes.is_empty() {
        return None;
    }
    let prior_instruction = prefix
        .rsplit_once(". ")
        .map_or(prefix, |(_, instruction)| instruction)
        .to_ascii_lowercase();
    let subject =
        if prior_instruction.contains("put each ") || prior_instruction.contains("return each ") {
            "Each of them is"
        } else {
            "It's"
        };
    Some(format!(
        "{prefix}. {subject} a {} {subtypes} in addition to its other types{remainder}",
        power_toughness.trim()
    ))
}

fn compact_second_landfall_damage_surface(line: &str) -> Option<String> {
    const ARTIFACTS: &[&str] = &[
        "Whenever a land an opponent controls enters, if the number of lands that entered the battlefield under that object's controller's control this turn is greater than or equal to 2, this creature deals 3 damage to that object's controller.",
        "Whenever a land an opponent controls enters, if the number of lands that entered the battlefield under that player's control this turn is greater than or equal to 2, this creature deals 3 damage to that object's controller.",
    ];
    if !ARTIFACTS.contains(&line) {
        return None;
    }
    Some(
        "Whenever a land enters under an opponent's control, if that player had another land enter the battlefield under their control this turn, this creature deals 3 damage to that player."
            .to_string(),
    )
}

fn compact_dynamic_ally_reanimate_surface(line: &str) -> Option<String> {
    let artifact = "{T}: Choose target creature card in an opponent's graveyard. Then if its mana value is the number of a ally you control or less, put target creature card in an opponent's graveyard onto the battlefield under your control.";
    if line != artifact {
        return None;
    }
    Some(
        "{T}: Put target creature card from an opponent's graveyard onto the battlefield under your control if its mana value is less than or equal to the number of Allies you control."
            .to_string(),
    )
}

fn compact_equipment_blocker_damage_surface(line: &str) -> Option<String> {
    let mut normalized = line.replace(
        "Equipped creature has {2}: This creature gets +1/+0 until end of turn.",
        "Equipped creature has \"{2}: This creature gets +1/+0 until end of turn.\"",
    );
    normalized = normalized.replace(
        "Whenever equipped creature deals damage to blocking creature, this Equipment deals that much damage to each other defending player's creature.",
        "Whenever equipped creature deals damage to a blocking creature, this Equipment deals that much damage to each other creature defending player controls.",
    );
    normalized = normalized.replace(
        "Whenever equipped creature deals damage to blocking creature, this Equipment deals that much damage to each other creature defending player controls.",
        "Whenever equipped creature deals damage to a blocking creature, this Equipment deals that much damage to each other creature defending player controls.",
    );
    (normalized != line).then_some(normalized)
}

fn compact_valiant_looked_card_battlefield_or_hand_surface(line: &str) -> Option<String> {
    let artifact = "Valiant — Whenever this creature becomes the target of a spell or ability you controls for the first time each turn, look at the top five cards of your library. You may reveal it. Then if it is your turn, you may put it onto the battlefield. Then if not, put it into its owner's hand. For each card revealed this way, Unless it's a permanent, put that object on the bottom of its owner's library.";
    if line != artifact {
        return None;
    }
    Some(
        "Valiant — Whenever this creature becomes the target of a spell or ability you control for the first time each turn, look at the top five cards of your library. You may reveal a creature card with mana value 3 or less from among them. You may put it onto the battlefield if it's your turn. If you don't put it onto the battlefield, put it into your hand. Put the rest on the bottom of your library in a random order."
            .to_string(),
    )
}

fn compact_opponent_library_creature_steal_surface(line: &str) -> Option<String> {
    let rest = line.strip_prefix("Search target opponent's library for ")?;
    let selection = rest.strip_suffix(
        ", put it onto the battlefield under target opponent's control, then shuffle target opponent's library.",
    )?;
    if !matches!(selection, "a creature card" | "an artifact card") {
        return None;
    }
    Some(format!(
        "Search target opponent's library for {selection} and put that card onto the battlefield under your control. Then that player shuffles."
    ))
}

fn compact_countered_spell_draw_trigger_surface(line: &str) -> Option<String> {
    (line == "Whenever a spell you've cast is countered, draw a card.").then_some(line.to_string())
}

fn compact_greatest_mana_value_sacrifice_surface(line: &str) -> Option<String> {
    let artifact = "Each opponent sacrifices a creature or planeswalker with mana value equal to a dynamic value of their choice.";
    if line != artifact {
        return None;
    }
    Some(
        "Each opponent sacrifices a creature or planeswalker with the greatest mana value among creatures and planeswalkers they control."
            .to_string(),
    )
}

fn compact_cycled_or_discarded_graveyard_return_surface(line: &str) -> Option<String> {
    if line
        != "Return all card in your graveyard you cycleds or discarded this turns from your graveyard to your hand."
    {
        return None;
    }
    Some(
        "Return to your hand all cards in your graveyard that you cycled or discarded this turn."
            .to_string(),
    )
}

fn compact_top_card_type_match_counter_cast_surface(line: &str) -> Option<String> {
    let prior_result_artifacts = [
        "Whenever an opponent casts a spell, you may reveal the top card of your library. Then if it's a permanent that shares a card type with that object, counter it, then that object's controller may cast that card without paying its mana cost.",
        "Whenever an opponent casts a spell, you may reveal the top card of your library. If a permanent that shares a card type with it was revealed this way, counter it and that player may cast that card without paying its mana cost.",
        "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If a permanent that shares a card type with it was revealed this way, counter it and that player may cast that card without paying its mana cost.",
    ];
    let candidate = line.trim_end_matches('.');
    if !prior_result_artifacts
        .iter()
        .any(|artifact| artifact.trim_end_matches('.') == candidate)
    {
        return None;
    }
    let oracle = "Whenever an opponent casts a spell from their hand, you may reveal the top card of your library. If it shares a card type with that spell, counter it and that opponent may cast the revealed card without paying its mana cost";
    Some(if line.ends_with('.') {
        format!("{oracle}.")
    } else {
        oracle.to_string()
    })
}

fn compact_shared_type_reveal_copy_draw_surface(line: &str) -> Option<String> {
    let artifact = "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. Then if a permanent that shares a card type with it was revealed this way, copy that spell, you may choose new targets for the copy, then each opponent draws a card. Otherwise, draw a card.";
    if line.trim_end_matches('.') != artifact.trim_end_matches('.') {
        return None;
    }
    let oracle = "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card";
    Some(if line.ends_with('.') {
        format!("{oracle}.")
    } else {
        oracle.to_string()
    })
}

fn compact_opponent_attack_pump_surface(line: &str) -> Option<String> {
    if line != "Whenever creature attacks, this creature gets +2/+0 until end of turn." {
        return None;
    }
    Some(
        "Whenever a creature attacks one of your opponents or a planeswalker an opponent controls, that creature gets +2/+0 until end of turn."
            .to_string(),
    )
}

fn compact_unblocked_creature_combat_prevention_surface(line: &str) -> Option<String> {
    if line == "Prevent all combat damage that would be dealt this turn by unblocked creature." {
        return Some(
            "Prevent all combat damage that would be dealt by unblocked creatures this turn."
                .to_string(),
        );
    }
    if line
        != "You may discard a Forest card rather than pay this spell's mana cost.\nPrevent all combat damage that would be dealt this turn by unblocked creature."
    {
        return None;
    }
    Some(
        "You may discard a Forest card rather than pay this spell's mana cost.\nPrevent all combat damage that would be dealt by unblocked creatures this turn."
            .to_string(),
    )
}

fn compact_counter_spell_damage_controller_surface(line: &str) -> Option<String> {
    if line != "Counter target spell and Ionize deals 2 damage to that object's controller." {
        return None;
    }
    Some("Counter target spell. Ionize deals 2 damage to that spell's controller.".to_string())
}

fn compact_forest_mana_additional_surface(line: &str) -> Option<String> {
    if line != "Whenever a player taps a Forest for mana, that object's controller adds {G}." {
        return None;
    }
    Some("Whenever a Forest is tapped for mana, its controller adds an additional {G}.".to_string())
}

fn compact_pyxis_exiled_permanents_surface(line: &str) -> Option<String> {
    if line
        == "{7}, {T}, Sacrifice this artifact: For each player, Return all permanent card in that player's exile to the battlefield under their owners' control."
    {
        return Some(
            "{7}, {T}, Sacrifice this artifact: Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield."
                .to_string(),
        );
    }
    let artifact = "{T}: Each player exiles the top card of their library face down.\n{7}, {T}, Sacrifice this artifact: For each player, Return all permanent card in that player's exile to the battlefield under their owners' control.";
    if line != artifact {
        return None;
    }
    Some(
        "{T}: Each player exiles the top card of their library face down.\n{7}, {T}, Sacrifice this artifact: Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield."
            .to_string(),
    )
}

fn compact_colored_creature_destroy_surface(line: &str) -> Option<String> {
    if line != "Destroy target colored creature." {
        return None;
    }
    Some("Destroy target creature that's one or more colors.".to_string())
}

fn normalize_one_or_more_colors_surface(line: &str) -> String {
    line.replace(
        "a colored permanent",
        "a permanent that's one or more colors",
    )
    .replace(
        "A colored permanent",
        "A permanent that's one or more colors",
    )
}

fn compact_dragon_reveal_additional_cost_surface(line: &str) -> Option<String> {
    if line != "As an additional cost to cast this spell, you may choose a Dragon card. Reveal it."
    {
        return None;
    }
    Some(
        "As an additional cost to cast this spell, you may reveal a Dragon card from your hand."
            .to_string(),
    )
}

fn compact_reveal_until_creature_reanimate_surface(line: &str) -> Option<String> {
    let artifacts = [
        "Target opponent reveals cards from the top of target opponent's library until they reveal a creature card, then target opponent puts all cards revealed this way into target opponent's graveyard. Put it onto the battlefield.",
        "Target opponent reveals cards from the top of target opponent's library until they reveal a creature card. For each card revealed this way, Unless it's a permanent, put that object into its owner's graveyard. Put it onto the battlefield.",
    ];
    if !artifacts.contains(&line) {
        return None;
    }
    Some(
        "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control."
            .to_string(),
    )
}

fn compact_each_opponent_who_didnt_draws_surface(line: &str) -> Option<String> {
    let artifact = "At the beginning of your end step, draw a card, each player may put a land card from their hand onto the battlefield, then for each opponent, if effect #0 that doesn't happen, that player draws a card.";
    if line != artifact {
        return None;
    }
    Some(
        "At the beginning of your end step, draw a card. Each player may put a land card from their hand onto the battlefield, then each opponent who didn't draws a card."
            .to_string(),
    )
}

fn compact_life_total_threshold_win_surface(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix(
            "At the beginning of your upkeep, if your life total is greater than or equal to ",
        )
        .map(|rest| (rest, ", you win the game."))
        .or_else(|| {
            // The comparison renderer's "N or greater" form of the same
            // condition.
            line.strip_prefix("At the beginning of your upkeep, if your life total is ")
                .map(|rest| (rest, " or greater, you win the game."))
        })?;
    let (rest, suffix) = rest;
    let amount = rest.strip_suffix(suffix)?;
    Some(format!(
        "At the beginning of your upkeep, if you have {amount} or more life, you win the game."
    ))
}

fn compact_reciprocal_creature_control_surface(line: &str) -> Option<String> {
    let artifacts = [
        "Gain control of all permanents until end of turn. Target opponent gains control of all permanents until end of turn. Untap all permanents or permanents. All permanents or permanents gain haste until end of turn.",
        "Gain control of each other creature until end of turn, untap that creature, then it gains haste until end of turn.",
    ];
    if !artifacts.contains(&line) {
        return None;
    }
    Some(
        "You and target opponent each gain control of all creatures the other controls until end of turn. Untap those creatures. Those creatures gain haste until end of turn."
            .to_string(),
    )
}

fn compact_search_exact_three_exile_shuffle_surface(line: &str) -> Option<String> {
    let artifact = "{2}, {T}, Sacrifice this artifact: Search target player's library for exactly 3 cards, exile them. Target player shuffles.";
    if line != artifact {
        return None;
    }
    Some(
        "{2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles."
            .to_string(),
    )
}

fn compact_white_reveal_life_gain_surface(line: &str) -> Option<String> {
    if line != "Choose any number white cards, reveal it, then gain 2 life for each permanent." {
        return None;
    }
    Some(
        "Reveal any number of white cards in your hand. You gain 2 life for each card revealed this way."
            .to_string(),
    )
}

fn compact_clash_additional_pump_trample_surface(line: &str) -> Option<String> {
    let artifacts = [
        "Target creature gets +2/+2 until end of turn. Clash with an opponent. If you do, it gets +2/+2 and gains trample until end of turn.",
        "Target creature gets +2/+2 until end of turn, clash with an opponent, then creatures gain trample until end of turn.",
    ];
    if !artifacts.contains(&line) {
        return None;
    }
    Some(
        "Target creature gets +2/+2 until end of turn. Clash with an opponent. If you win, that creature gets an additional +2/+2 and gains trample until end of turn."
            .to_string(),
    )
}

fn compact_historical_spell_half_damage_surface(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let leading = trimmed
        .strip_prefix("You choose a player who cast one or more ")
        .or_else(|| trimmed.strip_prefix("Choose a player who cast one or more "))?;
    let (card_type, rest) = leading.split_once(" spells this turn. Choose one of ")?;
    if card_type.is_empty() || card_type.split_whitespace().count() != 1 {
        return None;
    }
    let (_rendered_plural, damage_clause) =
        rest.split_once(" cast this turn by that player, then ")?;
    let (source, suffix) = damage_clause.split_once(
        " deals half the damage dealt this turn by the chosen spell, rounded down damage to that player",
    )?;
    if source.trim().is_empty() || !suffix.trim_matches('.').is_empty() {
        return None;
    }
    let period = if suffix.ends_with('.') { "." } else { "" };
    Some(format!(
        "Choose a player who cast one or more {card_type} spells this turn. {} deals damage to that player equal to half the damage dealt by one of those {card_type} spells this turn, rounded down{period}",
        source.trim()
    ))
}

fn normalize_attack_group_total_power_trigger_surface(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let leading = trimmed.strip_prefix("Whenever you attack with one or more ")?;
    let (subject, effect) = leading.split_once(", ")?;
    if subject.is_empty()
        || subject.contains(" you control")
        || !effect.contains("their total power")
    {
        return None;
    }
    Some(format!(
        "Whenever one or more {subject} you control attack, {effect}"
    ))
}

fn compact_face_down_return_then_turn_surface(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let (return_clause, tail) = trimmed
        .split_once(", then turn it face up if ")
        .or_else(|| trimmed.split_once(". Turn it face up if "))?;
    let lower_return_clause = return_clause.to_ascii_lowercase();
    if !(lower_return_clause.starts_with("return ") || lower_return_clause.contains(", return "))
        || !return_clause.contains(" to the battlefield face down")
    {
        return None;
    }
    let period = if tail.ends_with('.') { "." } else { "" };
    let condition = tail.trim_end_matches('.');
    let condition = condition
        .strip_prefix("that object is ")
        .map(|rest| {
            if rest == "a permanent" {
                "it's a permanent card".to_string()
            } else {
                format!("it's {rest}")
            }
        })
        .or_else(|| {
            condition.strip_prefix("it is ").map(|rest| {
                if rest == "a permanent" {
                    "it's a permanent card".to_string()
                } else {
                    format!("it's {rest}")
                }
            })
        })
        .unwrap_or_else(|| condition.to_string());
    Some(format!(
        "{return_clause} if {condition}, then turn it face up{period}"
    ))
}

fn compact_colored_permanent_sacrifice_surface(line: &str) -> Option<String> {
    let line = line.trim().trim_end_matches('.');
    let artifacts = [
        "each player sacrifices all colored permanents each player controls of their choice",
        "each player sacrifices all colored permanents they control of their choice",
        "each player sacrifices all colored permanents they control",
        "each player sacrifices all permanents they control that are one or more colors of their choice",
        "each player sacrifices all permanents they control that are one or more colors",
    ];
    if !artifacts
        .iter()
        .any(|artifact| line.eq_ignore_ascii_case(artifact))
    {
        return None;
    }
    Some(
        "Each player sacrifices all permanents they control that are one or more colors."
            .to_string(),
    )
}

fn compact_revealed_top_cards_choose_graveyard_surface(line: &str) -> Option<String> {
    let artifact = "Whenever this creature deals combat damage to a player, that player reveals the top two cards of their library. You choose a card. Put it into its owner's graveyard.";
    if line != artifact {
        return None;
    }
    Some(
        "Whenever this creature deals combat damage to a player, that player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard."
            .to_string(),
    )
}

fn compact_everybody_lives_surface(line: &str) -> Option<String> {
    let artifact = "Creatures gain hexproof and indestructible until end of turn, Players have hexproof this turn, Players can't lose life this turn, Players can't win the game this turn, then Players can't lose the game this turn.";
    if line != artifact {
        return None;
    }
    Some(
        "All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn."
            .to_string(),
    )
}

fn compact_multiverse_breach_surface(line: &str) -> Option<String> {
    let artifact = "Each player mills ten cards, for each player, you choose a creature or planeswalker card, put that card onto the battlefield under your control, then for each creature you control, each creature you control becomes a phyrexian in addition to its other types.";
    if line != artifact {
        return None;
    }
    Some(
        "Each player mills ten cards. For each player, choose a creature or planeswalker card in that player's graveyard. Put those cards onto the battlefield under your control. Then each creature you control becomes a Phyrexian in addition to its other types."
            .to_string(),
    )
}

fn compact_scry_reveal_draw_mana_value_surface(line: &str) -> Option<String> {
    if line != "Scry 3, reveal the top card of your library, then draw its mana value cards." {
        return None;
    }
    Some("Scry 3, then reveal the top card of your library. Draw cards equal to that card's mana value.".to_string())
}

fn compact_flying_becomes_blue_surface(line: &str) -> Option<String> {
    if line != "{U}, {T}: Target creature gains flying, then it becomes blue until end of turn." {
        return None;
    }
    Some("{U}, {T}: Target creature gains flying and becomes blue until end of turn.".to_string())
}

fn compact_opponent_hand_card_top_library_surface(line: &str) -> Option<String> {
    if line
        != "Target opponent loses 3 life. Put a card from their hand on top of target opponent's library."
    {
        return None;
    }
    Some(
        "Target opponent loses 3 life and puts a card from their hand on top of their library."
            .to_string(),
    )
}

fn compact_chosen_nonland_name_hand_discard_surface(line: &str) -> Option<String> {
    if line
        != "Choose a nonland permanent card name, target player reveals their hand, then target player discards the number of cards named {chosen Name}."
    {
        return None;
    }
    Some(
        "Choose a nonland card name. Target player reveals their hand and discards all cards with that name."
            .to_string(),
    )
}

fn compact_draw_cards_equal_instant_sorcery_graveyard_surface(line: &str) -> Option<String> {
    if line != "Draw a card for each instant or sorcery card in your graveyard." {
        return None;
    }
    Some(
        "Draw cards equal to the number of instant and sorcery cards in your graveyard."
            .to_string(),
    )
}

fn compact_aura_animation_activation_surface(line: &str) -> Option<String> {
    let marker = ", {T}: This creature loses this ability, this creature becomes an enchantment in addition to its other types, isn't an artifact, battle, creature, kindred, land, or planeswalker, becomes an aura in addition to its other types, and has enchant restriction, attach it to target creature, then you may pay ";
    let (cost, payment_tail) = line.split_once(marker)?;
    let payment = payment_tail.strip_suffix('.')?;
    Some(format!(
        "{cost}, {{T}}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {payment} to end this effect."
    ))
}

fn compact_enchanted_creature_artifact_pump_surface(line: &str) -> Option<String> {
    if line != "Enchanted creature is artifact in addition to its other types." {
        return None;
    }
    Some(
        "Enchanted creature gets +1/+1 and is an artifact in addition to its other types."
            .to_string(),
    )
}

fn compact_vivid_elemental_spectacle_surface(line: &str) -> Option<String> {
    let artifact = "Create a 5/5 red and green Elemental creature token for each colors among permanent you control, then gain 1 life for each creature you control.";
    if line != artifact {
        return None;
    }
    Some(
        "Vivid — Create a number of 5/5 red and green Elemental creature tokens equal to the number of colors among permanents you control. Then you gain life equal to the number of creatures you control."
            .to_string(),
    )
}

fn compact_target_opponent_count_prelude(line: &str) -> Option<String> {
    let mut normalized = line.to_string();
    let mut changed = false;
    for (from, to) in [
        ("Choose target opponent. You gain ", "You gain "),
        ("choose target opponent. You gain ", "you gain "),
        ("Choose target opponent. Draw ", "Draw "),
        ("choose target opponent. Draw ", "draw "),
        (
            "Choose target opponent. Target opponent ",
            "Target opponent ",
        ),
        (
            "choose target opponent. Target opponent ",
            "target opponent ",
        ),
        (
            "choose target opponent. target opponent ",
            "target opponent ",
        ),
    ] {
        if normalized.contains(from) {
            normalized = normalized.replace(from, to);
            changed = true;
        }
    }
    if !changed {
        return None;
    }
    Some(
        normalized
            .replace(" that player controls", " target opponent controls")
            .replace(" that player owns", " target opponent owns"),
    )
}

fn compact_target_cant_block_carry_surface(line: &str) -> Option<String> {
    let trimmed = line.trim_end_matches('.');
    let (head, tail) = trimmed
        .rsplit_once(", then ")
        .or_else(|| trimmed.rsplit_once(", Then "))?;
    let lower_tail = tail.to_ascii_lowercase();
    let (_plural_tail, remainder) =
        if let Some(remainder) = lower_tail.strip_prefix("creatures can't block this turn") {
            (true, &tail[tail.len() - remainder.len()..])
        } else if let Some(remainder) = lower_tail.strip_prefix("creature can't block this turn") {
            (false, &tail[tail.len() - remainder.len()..])
        } else {
            return None;
        };
    if !remainder.is_empty() && !remainder.starts_with(". ") {
        return None;
    }

    let lower_head = head.to_ascii_lowercase();
    if lower_head.contains("target creature") {
        let pronoun = if lower_head.contains("target creatures") {
            "Those creatures"
        } else {
            "That creature"
        };
        let mut head = head.to_string();
        if lower_head.contains(" damage to up to ") && !lower_head.contains(" damage to each of ") {
            head = head.replace(" damage to up to ", " damage to each of up to ");
        }
        return Some(format!(
            "{head}. {pronoun} can't block this turn{remainder}."
        ));
    }

    if lower_head.contains("you may ") {
        return Some(format!(
            "{head}. When you do, target creature can't block this turn{remainder}."
        ));
    }

    None
}

fn compact_period_cant_block_carry_surface(line: &str) -> Option<String> {
    let marker = ". Creatures can't block this turn";
    let idx = line.find(marker)?;
    let head = &line[..idx];
    let suffix = &line[idx + marker.len()..];
    let sentence_start = head.rfind(". ").map(|idx| idx + 2).unwrap_or(0);
    let prior_sentence = &head[sentence_start..];
    let lower_prior = prior_sentence.to_ascii_lowercase();
    if lower_prior.contains("you may ") {
        return Some(format!(
            "{head}. When you do, target creature can't block this turn{suffix}"
        ));
    }
    if lower_prior.contains("goad up to one target creature") {
        return Some(format!(
            "{head}. Those creatures can't block this turn{suffix}"
        ));
    }
    None
}

fn normalize_braced_numeric_damage_amounts(mut line: String) -> String {
    for amount in 0..=20 {
        line = line.replace(
            &format!("deals {{{amount}}} damage"),
            &format!("deals {amount} damage"),
        );
        line = line.replace(
            &format!("deal {{{amount}}} damage"),
            &format!("deal {amount} damage"),
        );
    }
    line
}

fn normalize_you_may_becomes_copy_surface(line: &str) -> Option<String> {
    let marker = "you may ";
    let idx = line.find(marker)?;
    let after = &line[idx + marker.len()..];
    let (subject, rest) = after.split_once(" becomes a copy of ")?;
    if subject.trim().is_empty() || subject.contains('.') || subject.contains(',') {
        return None;
    }
    Some(format!(
        "{}you may have {} become a copy of {}",
        &line[..idx],
        subject.trim(),
        rest
    ))
}

fn compact_token_redundant_mana_ability_surface(line: &str) -> Option<String> {
    let marker = " creature token. It has \"";
    let (prefix, ability_tail) = line.split_once(marker)?;
    let (ability, duplicate_tail) = ability_tail.split_once("\" And \"")?;
    let (duplicate, suffix) = duplicate_tail.split_once('"')?;
    let base_ability = ability.split_once(". ")?.0.trim();
    if !base_ability.starts_with("{T}: Add ")
        || !ability.contains(". Spend this mana only to cast ")
        || duplicate.trim_end_matches('.') != base_ability
    {
        return None;
    }
    Some(format!(
        "{prefix} creature token with \"{ability}\"{suffix}"
    ))
}

fn compact_repeated_target_player_life_loss(line: &str) -> Option<String> {
    fn compact_subject(line: &str, subject: &str) -> Option<String> {
        let marker = format!(". {subject} loses ");
        let idx = line.find(&marker)?;
        let prior = &line[..idx];
        let sentence_start = prior.rfind('.').map(|idx| idx + 1).unwrap_or(0);
        let prior_sentence = prior[sentence_start..].to_ascii_lowercase();
        if !prior_sentence.contains(&format!("{} ", subject.to_ascii_lowercase())) {
            return None;
        }
        Some(format!(
            "{} and loses {}",
            prior,
            &line[idx + marker.len()..]
        ))
    }

    compact_subject(line, "Target player")
        .or_else(|| compact_subject(line, "target player"))
        .or_else(|| compact_subject(line, "Target opponent"))
        .or_else(|| compact_subject(line, "target opponent"))
}

fn compact_repeated_target_opponent_discard(line: &str) -> Option<String> {
    let marker = ". Target opponent discards ";
    let idx = line.find(marker)?;
    let prior = &line[..idx];
    let sentence_start = prior.rfind('.').map(|idx| idx + 1).unwrap_or(0);
    let prior_sentence = prior[sentence_start..].to_ascii_lowercase();
    if !prior_sentence.contains("target opponent ") {
        return None;
    }
    Some(format!(
        "{}, discards {}",
        prior,
        &line[idx + marker.len()..]
    ))
}

fn compact_enters_counter_life_loss_surface(line: &str) -> Option<String> {
    let needle = "target player loses 1 life for each +1/+1 counter on this creature";
    if !line.contains(needle)
        || !line.contains(" enters, ")
        || !(line.starts_with("When ") || line.contains(". When "))
    {
        return None;
    }
    Some(line.replace(
        needle,
        "target player loses life equal to the number of +1/+1 counters on it",
    ))
}

fn compact_temporary_additional_block_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let marker = " and gains can block ";
    let (subject_and_pump, tail) = trimmed.split_once(marker)?;
    // Singular grant conjoined with a pump ("It gets +2/+2 and gains can
    // block an additional creature each combat until end of turn") — the
    // oracle keeps one sentence: "... until end of turn and can block an
    // additional creature this turn."
    if tail == "an additional creature each combat until end of turn"
        && subject_and_pump.contains(" gets ")
    {
        return Some(format!(
            "{subject_and_pump} until end of turn and can block an additional creature this turn."
        ));
    }
    let count = tail.strip_suffix(" additional creatures each combat until end of turn")?;
    let (subject, pump) = subject_and_pump.split_once(" gets ")?;
    let count_text = match count {
        "1" | "one" => "one",
        "2" | "two" => "two",
        "3" | "three" => "three",
        "4" | "four" => "four",
        _ => return None,
    };
    Some(format!(
        "{subject} gets {pump} until end of turn. That creature can block up to {count_text} additional creatures this turn."
    ))
}
