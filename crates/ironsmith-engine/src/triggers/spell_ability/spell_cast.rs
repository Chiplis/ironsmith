//! "Whenever [player] casts [spell]" trigger.

use crate::color::{Color, ColorSet};
use crate::events::EventKind;
use crate::events::spells::SpellCastEvent;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt as _;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{
    TriggerContext, TriggerMatcher, current_turn_matches_player_filter,
};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCastTrigger {
    pub filter: Option<ObjectFilter>,
    /// Require another card with the triggering spell's name in the given
    /// zone, owned by a player matching the supplied filter. This is checked
    /// when the spell is cast, so later graveyard changes cannot retroactively
    /// suppress the trigger.
    pub same_name_card_in_zone: Option<(Zone, PlayerFilter)>,
    /// Mana-producing objects whose mana must have been spent to cast the
    /// triggering spell. This is matched when the cast event occurs rather
    /// than re-checked as an intervening-if condition during resolution.
    pub mana_source_filter: Option<ObjectFilter>,
    pub caster: PlayerFilter,
    pub timing: Option<ironsmith_core::TriggerTimingRestriction>,
    pub during_turn: Option<PlayerFilter>,
    pub min_spells_this_turn: Option<u32>,
    pub exact_spells_this_turn: Option<u32>,
    /// Count every spell cast in the turn rather than only spells cast by the
    /// triggering spell's caster. This models passive ordinal surfaces such
    /// as "the fourth spell of a turn is cast."
    pub count_all_spells_this_turn: bool,
    pub from_not_hand: bool,
    pub first_spell_of_game: bool,
}

impl SpellCastTrigger {
    pub fn new(filter: Option<ObjectFilter>, caster: PlayerFilter) -> Self {
        Self {
            filter,
            same_name_card_in_zone: None,
            mana_source_filter: None,
            caster,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            count_all_spells_this_turn: false,
            from_not_hand: false,
            first_spell_of_game: false,
        }
    }

    pub fn qualified(
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        timing: Option<ironsmith_core::TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    ) -> Self {
        Self {
            filter,
            same_name_card_in_zone: None,
            mana_source_filter: None,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            count_all_spells_this_turn: false,
            from_not_hand,
            first_spell_of_game: false,
        }
    }

    pub fn nth_spell_of_turn(spell_number: u32) -> Self {
        Self {
            exact_spells_this_turn: Some(spell_number),
            count_all_spells_this_turn: true,
            ..Self::any_cast_any()
        }
    }

    pub fn with_first_spell_of_game(mut self, first_spell_of_game: bool) -> Self {
        self.first_spell_of_game = first_spell_of_game;
        self
    }

    pub fn with_mana_source_filter(mut self, mana_source_filter: Option<ObjectFilter>) -> Self {
        self.mana_source_filter = mana_source_filter;
        self
    }

    pub fn with_same_name_card_in_zone(mut self, zone: Zone, owner: PlayerFilter) -> Self {
        self.same_name_card_in_zone = Some((zone, owner));
        self
    }

    pub fn you_cast_any() -> Self {
        Self::new(None, PlayerFilter::You)
    }

    pub fn any_cast_any() -> Self {
        Self::new(None, PlayerFilter::Any)
    }
}

impl TriggerMatcher for SpellCastTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCast {
            return false;
        }
        let Some(e) = event.downcast::<SpellCastEvent>() else {
            return false;
        };

        // Check caster filter. This goes through the shared player-filter
        // evaluator so attached-player tags and exclusions work the same way
        // they do in object filters.
        let caster_matches = self.caster.matches_player(e.caster, &ctx.filter_ctx);

        if !caster_matches {
            return false;
        }

        if matches!(
            self.timing,
            Some(ironsmith_core::TriggerTimingRestriction::DuringCombat)
        ) && ctx.game.turn.phase != crate::game_state::Phase::Combat
        {
            return false;
        }

        if let Some(turn_filter) = &self.during_turn
            && !current_turn_matches_player_filter(turn_filter, ctx, None)
        {
            return false;
        }

        let cast_count = if self.count_all_spells_this_turn {
            ctx.game
                .turn_store
                .turn_history
                .total_spells_cast_this_turn()
        } else {
            ctx.game
                .turn_store
                .turn_history
                .spells_cast_by_player(e.caster)
        };
        if let Some(exact_spells) = self.exact_spells_this_turn {
            if cast_count != exact_spells {
                return false;
            }
        } else if let Some(min_spells) = self.min_spells_this_turn
            && cast_count < min_spells
        {
            return false;
        }
        if self.first_spell_of_game
            && ctx
                .game
                .player(e.caster)
                .is_none_or(|player| player.spells_cast_this_game != 1)
        {
            return false;
        }
        if self.from_not_hand && e.from_zone == Zone::Hand {
            return false;
        }
        if let Some(source_filter) = &self.mana_source_filter {
            let tag = crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
            let matches_mana_source = ctx
                .game
                .object(e.spell)
                .and_then(|spell| spell.cast_tagged_objects.get(&tag))
                .is_some_and(|snapshots| {
                    snapshots.iter().any(|snapshot| {
                        source_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                    })
                });
            if !matches_mana_source {
                return false;
            }
        }

        // Check spell filter if present.
        let spell_matches = if let Some(ref filter) = self.filter {
            let mut object_filter = filter.clone();

            // "Cast from <zone>" filters refer to the source zone, not the spell's
            // current zone (which is always the stack).
            if let Some(zone) = filter.zone
                && zone != Zone::Stack
            {
                if e.from_zone != zone {
                    return false;
                }
                object_filter.zone = None;
            }

            // SpellCastEvent already guarantees this is a spell being cast, so
            // avoid requiring a stack entry just because `ObjectFilter::spell()`
            // sets `zone=Stack` / `stack_kind=Spell`.
            if object_filter.zone == Some(Zone::Stack) {
                object_filter.zone = None;
            }
            if matches!(
                object_filter.stack_kind,
                Some(crate::filter::StackObjectKind::Spell)
            ) {
                object_filter.stack_kind = None;
            }
            // Real spells can have no mana cost (e.g. suspend cards).
            object_filter.has_mana_cost = false;

            if let Some(obj) = ctx.game.object(e.spell) {
                // Actor-relative spell-origin phrases such as `a player casts
                // a spell from their hand` use IteratedPlayer in the object
                // owner filter. During trigger matching, that participant is
                // the event's caster.
                let mut filter_ctx = ctx.filter_ctx.clone().with_iterated_player(Some(e.caster));
                for (tag, snapshots) in &obj.cast_tagged_objects {
                    let retained = filter_ctx.tagged_objects.entry(tag.clone()).or_default();
                    for snapshot in snapshots {
                        if retained
                            .iter()
                            .all(|existing| existing.stable_id != snapshot.stable_id)
                        {
                            retained.push(snapshot.clone());
                        }
                    }
                }
                object_filter.matches(obj, &filter_ctx, ctx.game)
            } else {
                false
            }
        } else {
            true
        };
        if !spell_matches {
            return false;
        }

        if let Some((zone, owner)) = &self.same_name_card_in_zone {
            let Some(spell) = ctx.game.object(e.spell) else {
                return false;
            };
            let owner_ctx = ctx.filter_ctx.clone().with_iterated_player(Some(e.caster));
            if !ctx
                .game
                .objects_in_zone(*zone)
                .into_iter()
                .any(|candidate| {
                    ctx.game.object(candidate).is_some_and(|candidate| {
                        owner.matches_player(candidate.owner, &owner_ctx)
                            && crate::filter::names_match(&candidate.name, &spell.name)
                    })
                })
            {
                return false;
            }
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::SpellCast])
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        let spell_cast = event.downcast::<SpellCastEvent>()?;
        let filter = self.filter.as_ref()?;
        let stack_entry = ctx
            .game
            .stack
            .iter()
            .find(|entry| entry.object_id == spell_cast.spell)?;

        let (player_filter, object_filter) =
            if filter.targets_only_player.is_some() || filter.targets_only_object.is_some() {
                (
                    filter.targets_only_player.as_ref(),
                    filter.targets_only_object.as_deref(),
                )
            } else {
                (
                    filter.targets_player.as_ref(),
                    filter.targets_object.as_deref(),
                )
            };

        // A bare target-count restriction (for example, "a spell with two
        // targets") binds the event value to every distinct target. A
        // targeting relation binds it only to the players or objects described
        // by that relation, so unrelated targets of the same spell do not
        // inflate "that many".
        if player_filter.is_none() && object_filter.is_none() {
            return filter.target_count.is_some().then(|| {
                stack_entry
                    .targets
                    .iter()
                    .copied()
                    .collect::<std::collections::HashSet<_>>()
                    .len() as i32
            });
        }

        Some(
            stack_entry
                .targets
                .iter()
                .copied()
                .filter(|target| match target {
                    crate::game_state::Target::Player(player) => player_filter
                        .is_some_and(|filter| filter.matches_player(*player, &ctx.filter_ctx)),
                    crate::game_state::Target::Object(object_id) => {
                        object_filter.is_some_and(|filter| {
                            ctx.game.object(*object_id).is_some_and(|object| {
                                filter.matches(object, &ctx.filter_ctx, ctx.game)
                            })
                        })
                    }
                })
                .collect::<std::collections::HashSet<_>>()
                .len() as i32,
        )
    }

    fn display(&self) -> String {
        if self.count_all_spells_this_turn
            && let Some(spell_number) = self.exact_spells_this_turn
        {
            let ordinal =
                ironsmith_core::ordinal_word(spell_number).unwrap_or_else(|| "nth".to_string());
            return format!("Whenever the {ordinal} spell of a turn is cast");
        }
        let caster_text = match &self.caster {
            PlayerFilter::You => "you cast",
            PlayerFilter::Any => "a player casts",
            PlayerFilter::Opponent => "an opponent casts",
            PlayerFilter::Active => "the active player casts",
            PlayerFilter::ChosenPlayer => "the chosen player casts",
            PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
                "enchanted player casts"
            }
            PlayerFilter::TaggedPlayer(_) => "that player casts",
            _ => "someone casts",
        };
        let mut spell_text = self
            .filter
            .as_ref()
            .map(describe_spell_filter)
            .unwrap_or_else(|| "a spell".to_string());
        if let Some(filter) = self
            .filter
            .as_ref()
            .filter(|filter| filter.zone == Some(Zone::Hand))
        {
            let mut spell_filter = filter.clone();
            spell_filter.zone = Some(Zone::Stack);
            spell_filter.owner = None;
            let hand = match filter.owner.as_ref() {
                Some(PlayerFilter::You) => "your hand",
                Some(PlayerFilter::Opponent) => "an opponent's hand",
                Some(PlayerFilter::ChosenPlayer) => "the chosen player's hand",
                Some(PlayerFilter::Specific(_)) | Some(PlayerFilter::TaggedPlayer(_)) => {
                    "that player's hand"
                }
                _ => match self.caster {
                    PlayerFilter::You => "your hand",
                    PlayerFilter::ChosenPlayer
                    | PlayerFilter::Specific(_)
                    | PlayerFilter::TaggedPlayer(_)
                    | PlayerFilter::Any
                    | PlayerFilter::Active
                    | PlayerFilter::Opponent => "their hand",
                    _ => "a hand",
                },
            };
            spell_text = format!("{} from {hand}", describe_spell_filter(&spell_filter));
        }
        let mut suffix = String::new();
        let mut suppress_turn_suffix = false;
        if self.first_spell_of_game && (spell_text == "a spell" || spell_text == "spell") {
            spell_text = match &self.caster {
                PlayerFilter::You => "your first spell of the game".to_string(),
                PlayerFilter::Any | PlayerFilter::Active | PlayerFilter::Opponent => {
                    "their first spell of the game".to_string()
                }
                PlayerFilter::Specific(_) => "that player's first spell of the game".to_string(),
                _ => "their first spell of the game".to_string(),
            };
        } else if self.first_spell_of_game {
            suffix.push_str(" if it's that player's first spell of the game");
        } else if let Some(exact_spells) = self.exact_spells_this_turn {
            let ordinal =
                ironsmith_core::ordinal_word(exact_spells).unwrap_or_else(|| "nth".to_string());
            let exact_spell_turn_suffix = match self.during_turn {
                Some(PlayerFilter::You) => Some("during each of your turns"),
                Some(PlayerFilter::Opponent) => Some("during each opponent's turn"),
                _ => None,
            };
            if spell_text == "a spell" || spell_text == "spell" {
                spell_text = match &self.caster {
                    PlayerFilter::You => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("your {ordinal} spell {turn_suffix}")
                        }
                        None => format!("your {ordinal} spell each turn"),
                    },
                    PlayerFilter::Any => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("their {ordinal} spell {turn_suffix}")
                        }
                        None => format!("their {ordinal} spell each turn"),
                    },
                    PlayerFilter::Active => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("their {ordinal} spell {turn_suffix}")
                        }
                        None => format!("their {ordinal} spell each turn"),
                    },
                    PlayerFilter::Opponent => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("their {ordinal} spell {turn_suffix}")
                        }
                        None => format!("their {ordinal} spell each turn"),
                    },
                    PlayerFilter::Specific(_) => {
                        format!("that player's {ordinal} spell each turn")
                    }
                    _ => format!("the {ordinal} spell each turn"),
                };
            } else {
                let base_spell_text = strip_leading_spell_article(&spell_text);
                spell_text = match &self.caster {
                    PlayerFilter::You => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("your {ordinal} {base_spell_text} {turn_suffix}")
                        }
                        None => format!("your {ordinal} {base_spell_text} each turn"),
                    },
                    PlayerFilter::Any | PlayerFilter::Active => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("their {ordinal} {base_spell_text} {turn_suffix}")
                        }
                        None => format!("their {ordinal} {base_spell_text} each turn"),
                    },
                    PlayerFilter::Opponent => match exact_spell_turn_suffix {
                        Some(turn_suffix) => {
                            suppress_turn_suffix = true;
                            format!("their {ordinal} {base_spell_text} {turn_suffix}")
                        }
                        None => format!("their {ordinal} {base_spell_text} each turn"),
                    },
                    PlayerFilter::Specific(_) => {
                        format!("that player's {ordinal} {base_spell_text} each turn")
                    }
                    _ => format!("the {ordinal} {base_spell_text} each turn"),
                };
            }
        } else if self.min_spells_this_turn == Some(2)
            && (spell_text == "a spell" || spell_text == "spell")
        {
            spell_text = match &self.caster {
                PlayerFilter::You if self.during_turn == Some(PlayerFilter::You) => {
                    suppress_turn_suffix = true;
                    "a spell during your turn other than your first spell that turn".to_string()
                }
                PlayerFilter::You => "a spell other than your first spell each turn".to_string(),
                PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
                    "a spell other than the first spell they cast each turn".to_string()
                }
                PlayerFilter::ChosenPlayer
                | PlayerFilter::TaggedPlayer(_)
                | PlayerFilter::Specific(_) => {
                    "a spell other than that player's first spell each turn".to_string()
                }
                _ => "a spell other than their first spell each turn".to_string(),
            };
        } else if self.min_spells_this_turn == Some(2)
            && matches!(
                self.caster,
                PlayerFilter::Opponent | PlayerFilter::Specific(_)
            )
        {
            spell_text = format!(
                "{spell_text} other than the first {spell_text} that player casts each turn"
            );
        } else if self.min_spells_this_turn == Some(2) {
            suffix.push_str(" as your second spell this turn");
        }
        if let Some(turn_filter) = &self.during_turn {
            let turn_text = match turn_filter {
                PlayerFilter::You => " during your turn",
                PlayerFilter::Opponent => " during an opponent's turn",
                PlayerFilter::Specific(_) => " during that player's turn",
                _ => "",
            };
            if !suppress_turn_suffix {
                suffix.push_str(turn_text);
            }
        }
        if matches!(
            self.timing,
            Some(ironsmith_core::TriggerTimingRestriction::DuringCombat)
        ) {
            suffix.push_str(" during combat");
        }
        if self.from_not_hand {
            suffix.push_str(" from anywhere other than your hand");
        }
        if let Some((zone, owner)) = &self.same_name_card_in_zone {
            let zone_text = match (zone, owner) {
                (Zone::Graveyard, PlayerFilter::You) => "your graveyard",
                (Zone::Graveyard, PlayerFilter::Opponent) => "an opponent's graveyard",
                (Zone::Graveyard, PlayerFilter::Specific(_))
                | (Zone::Graveyard, PlayerFilter::ChosenPlayer)
                | (Zone::Graveyard, PlayerFilter::TaggedPlayer(_)) => "that player's graveyard",
                (Zone::Graveyard, _) => "a graveyard",
                (Zone::Exile, PlayerFilter::You) => "among cards you own in exile",
                (Zone::Exile, _) => "among cards in exile",
                _ => "the specified zone",
            };
            suffix.push_str(" that has the same name as a card in ");
            suffix.push_str(zone_text);
        }
        let mana_source = self
            .mana_source_filter
            .as_ref()
            .map(|filter| format!(" using mana produced by {}", filter.description()))
            .unwrap_or_default();
        format!(
            "Whenever {} {}{}{}",
            caster_text, spell_text, mana_source, suffix
        )
    }
}

fn indefinite_article_for(text: &str) -> &'static str {
    match text
        .trim_start()
        .chars()
        .next()
        .map(|ch| ch.to_ascii_lowercase())
    {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn join_with_or(parts: &[String]) -> String {
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

fn describe_simple_spell_characteristic_union(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.is_empty() || filter.subtypes.is_empty() {
        return None;
    }
    if !matches!(
        filter.stack_kind,
        None | Some(crate::filter::StackObjectKind::Spell)
    ) {
        return None;
    }

    // Keep this compact surface limited to a plain spell-characteristic
    // filter. More qualified filters still need the general description path
    // so controller, color, mana-value, and other restrictions are retained.
    let mut simple_filter = ObjectFilter::spell();
    // Spell-cast matchers already establish that the event object is a spell,
    // so parser-produced filters may validly omit the redundant stack kind.
    simple_filter.stack_kind = filter.stack_kind;
    simple_filter.card_types = filter.card_types.clone();
    simple_filter.subtypes = filter.subtypes.clone();
    simple_filter.type_or_subtype_union = filter.type_or_subtype_union;
    simple_filter.has_mana_cost = filter.has_mana_cost;
    simple_filter.union_surface = filter.union_surface;
    if *filter != simple_filter {
        return None;
    }

    if !filter.type_or_subtype_union
        && filter.card_types.as_slice() == [crate::types::CardType::Creature]
        && filter.subtypes.as_slice() == [crate::types::Subtype::Adventure]
    {
        return Some("a creature spell that has an Adventure".to_string());
    }

    let subtype_names = filter
        .subtypes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let phrase = if filter.type_or_subtype_union {
        let mut alternatives = filter
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        alternatives.extend(subtype_names);
        format!("{} spell", join_with_or(&alternatives))
    } else if filter.card_types.len() == 1 {
        format!(
            "{} {} spell",
            join_with_or(&subtype_names),
            filter.card_types[0].name()
        )
    } else {
        return None;
    };

    Some(format!("{} {phrase}", indefinite_article_for(&phrase)))
}

fn describe_single_creature_target_excluding_source(filter: &ObjectFilter) -> Option<String> {
    if filter.targets_only_player.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.targets_only_any_of
        || filter.target_count != Some(crate::effect::ChoiceCount::exactly(1))
    {
        return None;
    }
    let target = filter.targets_only_object.as_deref()?;
    let source_surface = target.source_surface.as_ref()?;
    if !target.other || target.source {
        return None;
    }

    let mut plain_target = target.clone();
    plain_target.other = false;
    plain_target.source_surface = None;
    if !matches!(plain_target.zone, None | Some(Zone::Battlefield)) {
        return None;
    }
    let mut exact_creature = ObjectFilter::default();
    exact_creature.zone = plain_target.zone;
    exact_creature.card_types = vec![crate::types::CardType::Creature];
    if plain_target != exact_creature {
        return None;
    }

    // Prove the outer object is otherwise an unqualified spell. This keeps
    // the authored single-target/source-relative surface from hiding color,
    // controller, timing, or other targeting restrictions.
    let mut plain_spell = filter.clone();
    plain_spell.targets_only_object = None;
    plain_spell.target_count = None;
    if describe_spell_filter(&plain_spell) != "a spell" {
        return None;
    }

    Some(format!(
        "a spell that targets only a single creature other than {}",
        source_surface.display_text()
    ))
}

fn describe_spell_filter(filter: &ObjectFilter) -> String {
    if let Some(description) = describe_single_creature_target_excluding_source(filter) {
        return description;
    }
    if filter.has_phyrexian_mana_symbol {
        let mut base_filter = filter.clone();
        base_filter.has_phyrexian_mana_symbol = false;
        let mut base = describe_spell_filter(&base_filter);
        if base == "spell" {
            base = "a spell".to_string();
        }
        return format!("{base} with {{H}} in its mana cost");
    }
    let linked_spell = filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].tag.as_str() == "__it__"
        && filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject;
    if linked_spell {
        let mut plain = filter.clone();
        plain.tagged_constraints.clear();
        plain.zone = None;
        plain.stack_kind = None;
        plain.has_mana_cost = false;
        if plain == ObjectFilter::default() {
            return "that spell".to_string();
        }
    }

    if filter.has_x_in_cost {
        let mut base_filter = filter.clone();
        base_filter.has_x_in_cost = false;
        let mut base = describe_spell_filter(&base_filter);
        if !base.contains("{X}") {
            if matches!(base.as_str(), "spell" | "a spell") {
                base.push_str(" with {X} in its mana cost");
            } else {
                base.push_str(" with a mana cost that contains {X}");
            }
        }
        return base;
    }

    if filter.targets_player.is_some() || filter.targets_object.is_some() {
        let mut base_filter = filter.clone();
        let targets_player = base_filter.targets_player.take();
        let targets_object = base_filter.targets_object.take();
        let one_or_more_targets = base_filter
            .target_count
            .is_some_and(|count| count.min == 1 && count.max.is_none());
        base_filter.target_count = None;

        let mut base_text = describe_spell_filter(&base_filter);
        if base_text == "spell" {
            base_text = "a spell".to_string();
        } else if !base_text.to_ascii_lowercase().contains("spell") {
            base_text.push_str(" spell");
        }

        let mut target_parts = Vec::new();
        if let Some(player_filter) = targets_player {
            let player_text = match player_filter {
                PlayerFilter::You => "you".to_string(),
                PlayerFilter::NotYou => "a player other than you".to_string(),
                PlayerFilter::Opponent => "an opponent".to_string(),
                PlayerFilter::Any => "a player".to_string(),
                PlayerFilter::Specific(_) => "that player".to_string(),
                PlayerFilter::MostLifeTied => {
                    "a player with the most life or tied for most life".to_string()
                }
                PlayerFilter::LowestLifeTied => {
                    "a player with the lowest life or tied for lowest life".to_string()
                }
                PlayerFilter::MostCardsInHand => {
                    "the player who has the most cards in hand".to_string()
                }
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
                | PlayerFilter::HasMoreLifeThanYou { .. }
                | PlayerFilter::OpponentWithMoreControlledObjectsThan { .. }
                | PlayerFilter::ControlsMost { .. }
                | PlayerFilter::MaxSpeed { .. } => player_filter.description(),
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "a player who cast one or more {} spells this turn",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::AttackedBySourceThisTurn => player_filter.description(),
                PlayerFilter::WasDealtDamageBySourceThisGame { .. } => player_filter.description(),
                PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                    player_filter.description()
                }
                PlayerFilter::LostLifeThisTurn { .. } => player_filter.description(),
                PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
                    player_filter.description()
                }
                PlayerFilter::ChosenPlayer => "the chosen player".to_string(),
                PlayerFilter::TaggedPlayer(_) => "that player".to_string(),
                PlayerFilter::Teammate => "a teammate".to_string(),
                PlayerFilter::PlayerToYourLeft => "the player to your left".to_string(),
                PlayerFilter::PlayerToYourRight => "the player to your right".to_string(),
                PlayerFilter::Active => "the active player".to_string(),
                PlayerFilter::Defending => "the defending player".to_string(),
                PlayerFilter::Attacking => "an attacking player".to_string(),
                PlayerFilter::DamagedPlayer => "the damaged player".to_string(),
                PlayerFilter::EffectController => "the player who cast this spell".to_string(),
                PlayerFilter::IteratedPlayer => "that player".to_string(),
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    "that player or that object's controller".to_string()
                }
                PlayerFilter::Excluding { base, excluded } => format!(
                    "{} other than {}",
                    base.description(),
                    excluded.description()
                ),
                PlayerFilter::Target(inner) => match inner.as_ref() {
                    PlayerFilter::You => "you".to_string(),
                    PlayerFilter::NotYou => "a player other than you".to_string(),
                    PlayerFilter::Opponent => "an opponent".to_string(),
                    PlayerFilter::Any => "a player".to_string(),
                    _ => "target player".to_string(),
                },
                PlayerFilter::AliasedTarget(_) => "that player".to_string(),
                PlayerFilter::ControllerOf(_) => "that object's controller".to_string(),
                PlayerFilter::OwnerOf(_) => "that object's owner".to_string(),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    "that player".to_string()
                }
            };
            target_parts.push(if one_or_more_targets {
                pluralize_target_description(&player_text)
            } else {
                player_text
            });
        }
        if let Some(object_filter) = targets_object {
            let mut object_desc = object_filter.description();
            if object_desc == "this source" {
                object_desc = "this creature".to_string();
            } else if object_desc == "that source" {
                object_desc = "that creature".to_string();
            }
            target_parts.push(if one_or_more_targets {
                pluralize_target_description(&object_desc)
            } else {
                object_desc
            });
        }

        if !target_parts.is_empty() {
            let targets = if target_parts.len() == 2 {
                format!("{} and {}", target_parts[0], target_parts[1])
            } else {
                target_parts[0].clone()
            };
            let count_prefix = if one_or_more_targets {
                "one or more "
            } else {
                ""
            };
            return format!("{base_text} that targets {count_prefix}{targets}");
        }
        return base_text;
    }

    if let Some(required_colors) = filter.required_colors {
        let mut base_filter = filter.clone();
        base_filter.required_colors = None;
        let base_text = describe_spell_filter(&base_filter);
        let color_names = ordered_color_names(required_colors);
        if color_names.len() >= 2 {
            return format!("{base_text} that's both {}", color_names.join(" and "));
        }
    }

    // A cast trigger's zone is the spell's origin, while the event itself
    // already proves that the object is a spell. Render the characteristics
    // in stack context, then append the typed hand owner as an origin clause.
    // This avoids malformed surfaces such as "legendary card in your hand
    // spell" without discarding either the legendary restriction or origin.
    if filter.zone == Some(Zone::Hand) {
        let hand = match filter.owner.as_ref() {
            Some(PlayerFilter::You) => "your hand".to_string(),
            Some(PlayerFilter::Opponent) => "an opponent's hand".to_string(),
            Some(PlayerFilter::Specific(_)) | Some(PlayerFilter::TaggedPlayer(_)) => {
                "that player's hand".to_string()
            }
            Some(PlayerFilter::ChosenPlayer) => "the chosen player's hand".to_string(),
            _ => "a hand".to_string(),
        };
        let mut spell_filter = filter.clone();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.owner = None;
        return format!("{} from {hand}", describe_spell_filter(&spell_filter));
    }

    if filter.zone == Some(Zone::Graveyard) {
        let owner_text = match filter.owner.as_ref().unwrap_or(&PlayerFilter::Any) {
            PlayerFilter::You => "your",
            PlayerFilter::Opponent => "an opponent's",
            _ => "a",
        };
        if owner_text == "a" {
            return "a spell from a graveyard".to_string();
        }
        return format!("a spell from {owner_text} graveyard");
    }
    if filter.zone == Some(Zone::Exile) {
        return "a spell from exile".to_string();
    }
    if filter.card_types.is_empty()
        && filter
            .excluded_card_types
            .contains(&crate::types::CardType::Creature)
        && filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
    {
        return "a noncreature spell".to_string();
    }
    if let Some(description) = describe_simple_spell_characteristic_union(filter) {
        return description;
    }
    let mut subtype_only_spell_filter = ObjectFilter::default();
    subtype_only_spell_filter.zone = Some(Zone::Stack);
    subtype_only_spell_filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    subtype_only_spell_filter.subtypes = filter.subtypes.clone();
    subtype_only_spell_filter.has_mana_cost = filter.has_mana_cost;
    if !filter.subtypes.is_empty() && *filter == subtype_only_spell_filter {
        let subtypes = filter
            .subtypes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let subtype_text = join_with_or(&subtypes);
        return format!(
            "{} {subtype_text} spell",
            indefinite_article_for(&subtype_text)
        );
    }

    let fallback = filter.description();
    if filter.zone == Some(Zone::Stack) {
        match fallback.as_str() {
            "you own"
            | "you don't own"
            | "an opponent owns"
            | "a player owns"
            | "active player owns"
            | "defending player owns"
            | "attacking player owns"
            | "damaged player owns"
            | "a teammate owns"
            | "that player owns"
            | "they own" => return format!("a spell {fallback}"),
            _ => {}
        }
    }
    if fallback == "permanent" {
        "a spell".to_string()
    } else if fallback.to_ascii_lowercase().contains("spell") {
        if fallback.to_ascii_lowercase().contains("spells") {
            fallback
        } else {
            match fallback.split_whitespace().next() {
                Some("a" | "an" | "the" | "another" | "target") => fallback,
                _ => format!("{} {fallback}", indefinite_article_for(&fallback)),
            }
        }
    } else {
        format!("{fallback} spell")
    }
}

fn pluralize_target_description(description: &str) -> String {
    let description = description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(description);
    if description.contains(" or ") {
        return description
            .split(" or ")
            .map(pluralize_target_description)
            .collect::<Vec<_>>()
            .join(" or ");
    }
    for suffix in [
        " you control",
        " an opponent controls",
        " they control",
        " you own",
        " an opponent owns",
        " they own",
    ] {
        if let Some(head) = description.strip_suffix(suffix) {
            return format!("{}{suffix}", pluralize_target_description(head));
        }
    }
    let Some((head, noun)) = description.rsplit_once(' ') else {
        return pluralize_target_word(description);
    };
    format!("{head} {}", pluralize_target_word(noun))
}

fn pluralize_target_word(word: &str) -> String {
    if word.ends_with('s') {
        word.to_string()
    } else if let Some(stem) = word.strip_suffix('y') {
        format!("{stem}ies")
    } else if word.ends_with("ch") || word.ends_with("sh") || word.ends_with('x') {
        format!("{word}es")
    } else {
        format!("{word}s")
    }
}

fn ordered_color_names(colors: ColorSet) -> Vec<&'static str> {
    const CONVENTIONAL_PAIRS: [(Color, Color); 10] = [
        (Color::White, Color::Blue),
        (Color::Blue, Color::Black),
        (Color::Black, Color::Red),
        (Color::Red, Color::Green),
        (Color::Green, Color::White),
        (Color::White, Color::Black),
        (Color::Blue, Color::Red),
        (Color::Black, Color::Green),
        (Color::Red, Color::White),
        (Color::Green, Color::Blue),
    ];

    if colors.count() == 2
        && let Some((first, second)) = CONVENTIONAL_PAIRS
            .into_iter()
            .find(|(first, second)| colors.contains(*first) && colors.contains(*second))
    {
        return vec![first.name(), second.name()];
    }

    Color::ALL
        .into_iter()
        .filter(|color| colors.contains(*color))
        .map(Color::name)
        .collect()
}

fn strip_leading_spell_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("another "))
        .or_else(|| text.strip_prefix("the "))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::{ObjectId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn test_matches_own_spell() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let spell_id = ObjectId::from_raw(2);

        let trigger = SpellCastTrigger::you_cast_any();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = SpellCastTrigger::you_cast_any();
        assert!(trigger.display().contains("you cast"));
    }

    #[test]
    fn same_name_graveyard_requirement_matches_at_cast_time_and_renders() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::new(), "Ascension Source")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let duplicated = CardBuilder::new(CardId::new(), "Repeated Spark")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&duplicated, alice, Zone::Stack);
        let graveyard_id = game.create_object_from_card(&duplicated, alice, Zone::Graveyard);
        let trigger =
            SpellCastTrigger::new(Some(ObjectFilter::instant_or_sorcery()), PlayerFilter::You)
                .with_same_name_card_in_zone(Zone::Graveyard, PlayerFilter::You);
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever you cast an instant or sorcery spell that has the same name as a card in your graveyard"
        );

        game.move_object_by_effect(graveyard_id, Zone::Exile);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn linked_spell_identity_displays_as_that_spell() {
        let mut filter = ObjectFilter::spell();
        filter.has_mana_cost = true;
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::tag::TagKey::from("__it__"),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);

        assert_eq!(trigger.display(), "Whenever you cast that spell");
    }

    #[test]
    fn phyrexian_mana_spell_filter_renders_and_matches_the_printed_cost() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::new(), "Rage Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let phyrexian = CardBuilder::new(CardId::new(), "Phyrexian Spell")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Red,
                crate::mana::ManaSymbol::Life(2),
            ]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let phyrexian_id = game.create_object_from_card(&phyrexian, alice, Zone::Stack);
        let ordinary = CardBuilder::new(CardId::new(), "Ordinary Spell")
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Red,
            ]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let ordinary_id = game.create_object_from_card(&ordinary, alice, Zone::Stack);

        let mut filter = ObjectFilter::spell();
        filter.has_phyrexian_mana_symbol = true;
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell with {H} in its mana cost"
        );

        let ctx = TriggerContext::for_source(source_id, alice, &game);
        for (spell, expected) in [(phyrexian_id, true), (ordinary_id, false)] {
            let event = TriggerEvent::new_with_provenance(
                SpellCastEvent::new(spell, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            );
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }

    #[test]
    fn display_keeps_shared_creature_type_comparison_on_the_spell() {
        let filter = ObjectFilter::spell()
            .with_type(CardType::Creature)
            .sharing_no_creature_types_with(ObjectFilter::creature().you_control())
            .sharing_no_creature_types_with(
                ObjectFilter::default()
                    .with_type(CardType::Creature)
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            );
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);

        assert_eq!(
            trigger.display(),
            "Whenever you cast a creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard"
        );
    }

    #[test]
    fn nth_spell_of_turn_counts_spells_across_players() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let trigger = SpellCastTrigger::nth_spell_of_turn(2);

        let first = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(ObjectId::from_raw(2), alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&first);
        assert!(!trigger.matches(&first, &TriggerContext::for_source(source_id, alice, &game),));

        let second = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(ObjectId::from_raw(3), bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&second);
        assert!(trigger.matches(
            &second,
            &TriggerContext::for_source(source_id, alice, &game),
        ));
        assert_eq!(
            trigger.display(),
            "Whenever the second spell of a turn is cast"
        );
    }

    #[test]
    fn mana_source_provenance_matches_at_cast_event_time() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::new(), "Barracks of the Thousand")
            .card_types(vec![CardType::Land])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let other_source = CardBuilder::new(CardId::new(), "Other Land")
            .card_types(vec![CardType::Land])
            .build();
        let other_source_id = game.create_object_from_card(&other_source, alice, Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(), "Artifact Spell")
            .card_types(vec![CardType::Artifact])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source_id).unwrap(), &game);
        let other_source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(other_source_id).unwrap(),
            &game,
        );
        let mana_sources_tag =
            crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
        game.object_mut(spell_id)
            .unwrap()
            .cast_tagged_objects
            .insert(mana_sources_tag.clone(), vec![source_snapshot]);

        let trigger = SpellCastTrigger::new(None, PlayerFilter::You).with_mana_source_filter(Some(
            ObjectFilter::source_with_surface(crate::target::SourceReferenceSurface::ShortName(
                "Barracks of the Thousand".to_string(),
            )),
        ));
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell using mana produced by Barracks of the Thousand"
        );
        assert!(trigger.matches(&event, &TriggerContext::for_source(source_id, alice, &game),));

        game.object_mut(spell_id)
            .unwrap()
            .cast_tagged_objects
            .insert(mana_sources_tag, vec![other_source_snapshot]);
        assert!(!trigger.matches(&event, &TriggerContext::for_source(source_id, alice, &game),));
    }

    #[test]
    fn hand_origin_spell_filter_keeps_characteristics_before_spell_noun() {
        let mut filter = ObjectFilter::spell()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You);
        filter.supertypes = vec![crate::types::Supertype::Legendary];
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);

        assert_eq!(
            trigger.display(),
            "Whenever you cast a legendary spell from your hand"
        );
    }

    #[test]
    fn hand_origin_uses_the_casting_players_possessive() {
        let filter = ObjectFilter::spell().in_zone(Zone::Hand);

        assert_eq!(
            SpellCastTrigger::new(Some(filter.clone()), PlayerFilter::Opponent).display(),
            "Whenever an opponent casts a spell from their hand"
        );
        assert_eq!(
            SpellCastTrigger::new(Some(filter), PlayerFilter::Any).display(),
            "Whenever a player casts a spell from their hand"
        );
    }

    #[test]
    fn chosen_player_display_preserves_the_bound_caster() {
        let trigger = SpellCastTrigger::new(None, PlayerFilter::ChosenPlayer);
        assert_eq!(
            trigger.display(),
            "Whenever the chosen player casts a spell"
        );
    }

    #[test]
    fn adventure_creature_spell_uses_rules_characteristic_surface() {
        for stack_kind in [None, Some(crate::filter::StackObjectKind::Spell)] {
            let filter = ObjectFilter {
                zone: Some(Zone::Stack),
                stack_kind,
                card_types: vec![CardType::Creature],
                subtypes: vec![Subtype::Adventure],
                ..Default::default()
            };
            let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);

            assert_eq!(
                trigger.display(),
                "Whenever you cast a creature spell that has an Adventure"
            );
        }
    }

    #[test]
    fn adventure_creature_spell_special_surface_rejects_qualified_near_misses() {
        let filter = ObjectFilter {
            zone: Some(Zone::Stack),
            card_types: vec![CardType::Creature],
            subtypes: vec![Subtype::Adventure],
            ..Default::default()
        };

        let mut owned = filter.clone();
        owned.owner = Some(PlayerFilter::You);
        assert!(describe_simple_spell_characteristic_union(&owned).is_none());

        let mut ability = filter.clone();
        ability.stack_kind = Some(crate::filter::StackObjectKind::Ability);
        assert!(describe_simple_spell_characteristic_union(&ability).is_none());

        let mut union = filter;
        union.type_or_subtype_union = true;
        assert_ne!(
            describe_simple_spell_characteristic_union(&union).as_deref(),
            Some("a creature spell that has an Adventure")
        );
    }

    #[test]
    fn first_spell_of_game_is_tracked_per_player_across_turns() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(1);
        let trigger =
            SpellCastTrigger::new(None, PlayerFilter::Opponent).with_first_spell_of_game(true);

        let first_bob_cast = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(ObjectId::from_raw(2), bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&first_bob_cast);
        assert!(trigger.matches(
            &first_bob_cast,
            &TriggerContext::for_source(source_id, alice, &game),
        ));

        game.turn_store.turn_history.clear_for_new_turn();
        let second_bob_cast = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(ObjectId::from_raw(3), bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&second_bob_cast);
        assert!(!trigger.matches(
            &second_bob_cast,
            &TriggerContext::for_source(source_id, alice, &game),
        ));

        let first_cara_cast = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(ObjectId::from_raw(4), cara, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&first_cara_cast);
        assert!(trigger.matches(
            &first_cara_cast,
            &TriggerContext::for_source(source_id, alice, &game),
        ));
        assert_eq!(
            trigger.display(),
            "Whenever an opponent casts their first spell of the game"
        );
    }

    #[test]
    fn test_display_noncreature_spell_filter() {
        let trigger =
            SpellCastTrigger::new(Some(ObjectFilter::noncreature_spell()), PlayerFilter::You);
        assert_eq!(trigger.display(), "Whenever you cast a noncreature spell");
    }

    #[test]
    fn mixed_card_type_and_subtype_spell_union_renders_and_matches_every_arm() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::new(), "Trigger Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let instant = CardBuilder::new(CardId::new(), "Instant Arm")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Stack);
        let sorcery = CardBuilder::new(CardId::new(), "Sorcery Arm")
            .card_types(vec![CardType::Sorcery])
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Stack);
        let wizard = CardBuilder::new(CardId::new(), "Subtype Arm")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wizard])
            .build();
        let wizard_id = game.create_object_from_card(&wizard, alice, Zone::Stack);
        let unrelated = CardBuilder::new(CardId::new(), "Unrelated Spell")
            .card_types(vec![CardType::Creature])
            .build();
        let unrelated_id = game.create_object_from_card(&unrelated, alice, Zone::Stack);

        let mut filter = ObjectFilter::spell();
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        filter.subtypes = vec![Subtype::Wizard];
        filter.type_or_subtype_union = true;
        filter.has_mana_cost = true;
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);
        assert_eq!(
            trigger.display(),
            "Whenever you cast an instant, sorcery, or Wizard spell"
        );

        let ctx = TriggerContext::for_source(source_id, alice, &game);
        for (spell_id, expected) in [
            (instant_id, true),
            (sorcery_id, true),
            (wizard_id, true),
            (unrelated_id, false),
        ] {
            let event = TriggerEvent::new_with_provenance(
                SpellCastEvent::new(spell_id, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            );
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }

    #[test]
    fn subtype_list_creature_spell_filter_renders_and_matches_every_subtype() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::new(), "Trigger Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

        let mut spell_ids = Vec::new();
        for (name, subtype) in [
            ("Pegasus Arm", Subtype::Pegasus),
            ("Unicorn Arm", Subtype::Unicorn),
            ("Horse Arm", Subtype::Horse),
        ] {
            let card = CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .subtypes(vec![subtype])
                .build();
            spell_ids.push(game.create_object_from_card(&card, alice, Zone::Stack));
        }
        let unrelated = CardBuilder::new(CardId::new(), "Unrelated Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wizard])
            .build();
        let unrelated_id = game.create_object_from_card(&unrelated, alice, Zone::Stack);

        let mut filter = ObjectFilter::spell();
        filter.card_types = vec![CardType::Creature];
        filter.subtypes = vec![Subtype::Pegasus, Subtype::Unicorn, Subtype::Horse];
        filter.has_mana_cost = true;
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);
        assert_eq!(
            trigger.display(),
            "Whenever you cast a Pegasus, Unicorn, or Horse creature spell"
        );

        let ctx = TriggerContext::for_source(source_id, alice, &game);
        for spell_id in spell_ids {
            let event = TriggerEvent::new_with_provenance(
                SpellCastEvent::new(spell_id, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            );
            assert!(trigger.matches(&event, &ctx));
        }
        let unrelated_event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(unrelated_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&unrelated_event, &ctx));
    }

    #[test]
    fn simple_creature_spell_filter_remains_unchanged() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::new(), "Trigger Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let creature = CardBuilder::new(CardId::new(), "Creature Spell")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Stack);
        let instant = CardBuilder::new(CardId::new(), "Instant Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let instant_id = game.create_object_from_card(&instant, alice, Zone::Stack);

        let mut filter = ObjectFilter::spell();
        filter.card_types = vec![CardType::Creature];
        filter.has_mana_cost = true;
        let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);
        assert_eq!(trigger.display(), "Whenever you cast a creature spell");

        let ctx = TriggerContext::for_source(source_id, alice, &game);
        for (spell_id, expected) in [(creature_id, true), (instant_id, false)] {
            let event = TriggerEvent::new_with_provenance(
                SpellCastEvent::new(spell_id, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            );
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }

    #[test]
    fn test_display_spell_requiring_both_colors() {
        for (colors, expected) in [
            (
                ColorSet::RED.with(Color::White),
                "Whenever you cast a spell that's both red and white",
            ),
            (
                ColorSet::WHITE.with(Color::Black),
                "Whenever you cast a spell that's both white and black",
            ),
            (
                ColorSet::GREEN.with(Color::Blue),
                "Whenever you cast a spell that's both green and blue",
            ),
            (
                ColorSet::BLACK.with(Color::Green),
                "Whenever you cast a spell that's both black and green",
            ),
        ] {
            let mut filter = ObjectFilter::spell();
            filter.required_colors = Some(colors);
            let trigger = SpellCastTrigger::new(Some(filter), PlayerFilter::You);
            assert_eq!(trigger.display(), expected);
        }
    }

    #[test]
    fn test_matches_spell_cast_from_graveyard_zone_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let spell = CardBuilder::new(CardId::new(), "Graveyard Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);

        let trigger = SpellCastTrigger::new(
            Some(
                ObjectFilter::spell()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            PlayerFilter::You,
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let from_graveyard = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Graveyard),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&from_graveyard, &ctx));

        let from_hand = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&from_hand, &ctx));
    }

    #[test]
    fn test_display_spell_from_graveyard_filter() {
        let trigger = SpellCastTrigger::new(
            Some(
                ObjectFilter::spell()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            PlayerFilter::You,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell from your graveyard"
        );
    }

    #[test]
    fn test_qualified_second_spell_during_your_turn_display() {
        let trigger = SpellCastTrigger::qualified(
            None,
            PlayerFilter::You,
            None,
            Some(PlayerFilter::You),
            Some(2),
            None,
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell during your turn other than your first spell that turn"
        );
    }

    #[test]
    fn combat_timing_restriction_is_rendered_and_matched() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let spell = CardBuilder::new(CardId::new(), "Combat Timing Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let trigger = SpellCastTrigger::qualified(
            None,
            PlayerFilter::You,
            Some(ironsmith_core::TriggerTimingRestriction::DuringCombat),
            None,
            None,
            None,
            false,
        );
        assert_eq!(trigger.display(), "Whenever you cast a spell during combat");

        game.turn.phase = crate::game_state::Phase::FirstMain;
        assert!(!trigger.matches(&event, &TriggerContext::for_source(source_id, alice, &game)));
        game.turn.phase = crate::game_state::Phase::Combat;
        assert!(trigger.matches(&event, &TriggerContext::for_source(source_id, alice, &game)));
    }

    #[test]
    fn qualified_second_spell_any_player_keeps_typed_ordinal() {
        let trigger = SpellCastTrigger::qualified(
            Some(ObjectFilter::spell().in_zone(Zone::Stack)),
            PlayerFilter::Any,
            None,
            None,
            Some(2),
            None,
            false,
        );
        assert_eq!(trigger.caster, PlayerFilter::Any);
        assert_eq!(trigger.min_spells_this_turn, Some(2));
        assert_eq!(trigger.exact_spells_this_turn, None);
        assert_eq!(
            trigger.filter,
            Some(ObjectFilter::spell().in_zone(Zone::Stack))
        );
    }

    #[test]
    fn test_qualified_third_spell_you_display() {
        let trigger =
            SpellCastTrigger::qualified(None, PlayerFilter::You, None, None, None, Some(3), false);
        assert_eq!(
            trigger.display(),
            "Whenever you cast your third spell each turn"
        );
    }

    #[test]
    fn test_qualified_first_noncreature_spell_opponent_display() {
        let trigger = SpellCastTrigger::qualified(
            Some(ObjectFilter::noncreature_spell()),
            PlayerFilter::Opponent,
            None,
            None,
            None,
            Some(1),
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever an opponent casts their first noncreature spell each turn"
        );
    }

    #[test]
    fn test_qualified_first_spell_from_graveyard_you_display() {
        let trigger = SpellCastTrigger::qualified(
            Some(
                ObjectFilter::spell()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ),
            PlayerFilter::You,
            None,
            None,
            None,
            Some(1),
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast your first spell from your graveyard each turn"
        );
    }

    #[test]
    fn test_display_spell_filter_with_targeted_object_clause() {
        let trigger = SpellCastTrigger::new(
            Some(ObjectFilter::spell().targeting_object(ObjectFilter::source())),
            PlayerFilter::You,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell that targets this creature"
        );
    }

    #[test]
    fn single_creature_target_source_exclusion_keeps_explicit_cardinality_and_alias() {
        for mut target in [
            ObjectFilter::creature(),
            ObjectFilter::default().with_type(CardType::Creature),
        ] {
            target.other = true;
            target.source_surface = Some(crate::target::SourceReferenceSurface::ShortName(
                "Named Source".to_string(),
            ));
            let mut spell = ObjectFilter::spell();
            spell.targets_only_object = Some(Box::new(target));
            spell.target_count = Some(crate::effect::ChoiceCount::exactly(1));
            let trigger = SpellCastTrigger::new(Some(spell), PlayerFilter::Any);

            assert_eq!(
                trigger.display(),
                "Whenever a player casts a spell that targets only a single creature other than Named Source"
            );
        }
    }

    #[test]
    fn source_exclusion_surface_requires_exact_single_plain_creature_target_shape() {
        let mut target = ObjectFilter::creature();
        target.other = true;
        target.source_surface = Some(crate::target::SourceReferenceSurface::ShortName(
            "Named Source".to_string(),
        ));

        let mut non_single = ObjectFilter::spell();
        non_single.targets_only_object = Some(Box::new(target.clone()));
        non_single.target_count = Some(crate::effect::ChoiceCount::at_least(1));
        assert_ne!(
            SpellCastTrigger::new(Some(non_single), PlayerFilter::Any).display(),
            "Whenever a player casts a spell that targets only a single creature other than Named Source"
        );

        let mut controlled_target = target;
        controlled_target.controller = Some(PlayerFilter::You);
        let mut qualified = ObjectFilter::spell();
        qualified.targets_only_object = Some(Box::new(controlled_target));
        qualified.target_count = Some(crate::effect::ChoiceCount::exactly(1));
        assert_ne!(
            SpellCastTrigger::new(Some(qualified), PlayerFilter::Any).display(),
            "Whenever a player casts a spell that targets only a single creature other than Named Source"
        );
    }

    #[test]
    fn event_value_counts_distinct_targets_matching_the_trigger_relation() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = CardBuilder::new(CardId::new(), "Arcee")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(), "Multi-target Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        let creature = CardBuilder::new(CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let own_creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let opposing_creature = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let vehicle = CardBuilder::new(CardId::new(), "Vehicle")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Vehicle])
            .build();
        let own_vehicle = game.create_object_from_card(&vehicle, alice, Zone::Battlefield);
        game.push_to_stack(
            crate::game_state::StackEntry::new(spell_id, alice).with_targets(vec![
                crate::game_state::Target::Object(own_creature),
                crate::game_state::Target::Object(own_creature),
                crate::game_state::Target::Object(own_vehicle),
                crate::game_state::Target::Object(opposing_creature),
                crate::game_state::Target::Player(bob),
            ]),
        );

        let mut target_filter = ObjectFilter::default();
        target_filter.card_types = vec![CardType::Creature];
        target_filter.subtypes = vec![Subtype::Vehicle];
        target_filter.type_or_subtype_union = true;
        target_filter.controller = Some(PlayerFilter::You);
        let trigger = SpellCastTrigger::new(
            Some(
                ObjectFilter::spell()
                    .targeting_object(target_filter)
                    .with_target_count(crate::effect::ChoiceCount::at_least(1)),
            ),
            PlayerFilter::You,
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(spell_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.event_value_amount(&event, &ctx), Some(2));
        assert_eq!(
            trigger.display(),
            "Whenever you cast a spell that targets one or more creatures or Vehicles you control"
        );
    }

    #[test]
    fn test_display_chosen_color_spell_filter() {
        let trigger = SpellCastTrigger::new(
            Some(ObjectFilter::spell().of_chosen_color()),
            PlayerFilter::Any,
        );
        assert_eq!(
            trigger.display(),
            "Whenever a player casts a spell of the chosen color"
        );
    }

    #[test]
    fn test_matches_chosen_color_spell_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = CardBuilder::new(CardId::new(), "Curse Source")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        game.set_chosen_color(source_id, crate::color::Color::Black);

        let black_spell = CardBuilder::new(CardId::new(), "Black Spell")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Black,
            ]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let black_spell_id = game.create_object_from_card(&black_spell, bob, Zone::Stack);

        let red_spell = CardBuilder::new(CardId::new(), "Red Spell")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Red,
            ]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let red_spell_id = game.create_object_from_card(&red_spell, bob, Zone::Stack);

        let trigger = SpellCastTrigger::new(
            Some(ObjectFilter::spell().of_chosen_color()),
            PlayerFilter::Any,
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let black_cast = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(black_spell_id, bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&black_cast, &ctx));

        let red_cast = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(red_spell_id, bob, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&red_cast, &ctx));
    }
}
