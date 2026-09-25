//! Dungeon cards (CR 309).
//!
//! Dungeons begin outside the game (CR 309.2) and are never deck cards, so
//! the host registers their compiled definitions here, the same artifacts the
//! compiler bakes for ordinary cards. A dungeon's room abilities are compiled
//! triggered abilities ("When you move your venture marker into this room,
//! [effect]", CR 309.4c) whose trigger carries the room's printed name and the
//! rooms its arrows lead to; the room graph is read from them.

use std::sync::{Arc, OnceLock, RwLock};

use crate::ability::{AbilityKind, TriggeredAbility};
use crate::cards::CardDefinition;
use crate::types::CardType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveDungeonProgress {
    pub dungeon_name: String,
    pub room_name: String,
}

impl ActiveDungeonProgress {
    pub fn new(dungeon_name: impl Into<String>, room_name: impl Into<String>) -> Self {
        Self {
            dungeon_name: dungeon_name.into(),
            room_name: room_name.into(),
        }
    }
}

/// One room of a dungeon card: its name (CR 309.4b), the rooms its arrows
/// point to (CR 309.5a), and its room ability (CR 309.4c).
#[derive(Debug, Clone)]
pub struct DungeonRoom {
    pub name: String,
    pub leads_to: Vec<String>,
    pub ability: TriggeredAbility,
}

/// A dungeon card read from its compiled definition.
#[derive(Debug, Clone)]
pub struct DungeonDefinition {
    pub name: String,
    /// Rooms in printed order; the first is the topmost room (CR 309.4a).
    pub rooms: Vec<DungeonRoom>,
    /// "You can't enter this dungeon unless you 'venture into [quality]'"
    /// (CR 701.49d), e.g. `Some("Undercity")`.
    pub entry_restriction: Option<String>,
}

impl DungeonDefinition {
    /// Read a dungeon from a compiled card definition, validating its room
    /// graph: every arrow must name a room of this dungeon and exactly one
    /// room (the bottommost) has no arrows.
    pub fn from_card_definition(definition: &CardDefinition) -> Result<Self, String> {
        let name = definition.card.name.clone();
        if !definition.card.card_types.contains(&CardType::Dungeon) {
            return Err(format!("{name} is not a dungeon card"));
        }
        let mut rooms = Vec::new();
        let mut entry_restriction = None;
        for ability in &definition.abilities {
            match &ability.kind {
                AbilityKind::Triggered(triggered) => {
                    let Some(room) = triggered.trigger.as_dungeon_room() else {
                        return Err(format!(
                            "{name} has a triggered ability that is not a room ability"
                        ));
                    };
                    rooms.push(DungeonRoom {
                        name: room.room.clone(),
                        leads_to: room.leads_to.clone(),
                        ability: triggered.clone(),
                    });
                }
                AbilityKind::Static(static_ability) => {
                    let Some(quality) = static_ability.dungeon_entry_quality() else {
                        return Err(format!(
                            "{name} has an unsupported dungeon ability: {}",
                            static_ability.display()
                        ));
                    };
                    entry_restriction = Some(quality.to_string());
                }
                _ => return Err(format!("{name} has an unsupported dungeon ability")),
            }
        }
        if rooms.is_empty() {
            return Err(format!("{name} has no rooms"));
        }
        for room in &rooms {
            if let Some(missing) = room.leads_to.iter().find(|next| {
                !rooms
                    .iter()
                    .any(|candidate| candidate.name.eq_ignore_ascii_case(next))
            }) {
                return Err(format!(
                    "{name}: room {} leads to unknown room {missing}",
                    room.name
                ));
            }
        }
        if rooms.iter().filter(|room| room.leads_to.is_empty()).count() != 1 {
            return Err(format!("{name} must have exactly one bottommost room"));
        }
        Ok(Self {
            name,
            rooms,
            entry_restriction,
        })
    }

    pub fn first_room(&self) -> &DungeonRoom {
        &self.rooms[0]
    }

    pub fn room(&self, room_name: &str) -> Option<&DungeonRoom> {
        self.rooms
            .iter()
            .find(|room| room.name.eq_ignore_ascii_case(room_name))
    }

    /// Whether "venture into the dungeon" may bring this dungeon in, or
    /// "venture into [quality]" when `quality` is given (CR 701.49a, d).
    fn can_enter_by(&self, quality: Option<&str>) -> bool {
        match quality {
            None => self.entry_restriction.is_none(),
            Some(quality) => {
                self.name.eq_ignore_ascii_case(quality)
                    || self
                        .entry_restriction
                        .as_deref()
                        .is_some_and(|restriction| restriction.eq_ignore_ascii_case(quality))
            }
        }
    }
}

fn dungeon_catalog() -> &'static RwLock<Vec<Arc<DungeonDefinition>>> {
    static CATALOG: OnceLock<RwLock<Vec<Arc<DungeonDefinition>>>> = OnceLock::new();
    CATALOG.get_or_init(|| RwLock::new(Vec::new()))
}

/// Register a compiled dungeon card. Registering a dungeon again replaces
/// the earlier definition of the same name.
pub fn register_dungeon_definition(definition: &CardDefinition) -> Result<(), String> {
    let dungeon = Arc::new(DungeonDefinition::from_card_definition(definition)?);
    let mut catalog = dungeon_catalog()
        .write()
        .map_err(|_| "dungeon catalog is poisoned".to_string())?;
    catalog.retain(|existing| !existing.name.eq_ignore_ascii_case(&dungeon.name));
    catalog.push(dungeon);
    // Players choose among dungeons by index, so every session must list
    // them in the same order regardless of registration order.
    catalog.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(())
}

pub fn clear_registered_dungeons() {
    if let Ok(mut catalog) = dungeon_catalog().write() {
        catalog.clear();
    }
}

pub fn is_dungeon_definition(definition: &CardDefinition) -> bool {
    definition.card.card_types.contains(&CardType::Dungeon)
}

pub fn lookup_dungeon(name: &str) -> Option<Arc<DungeonDefinition>> {
    dungeon_catalog().read().ok().and_then(|catalog| {
        catalog
            .iter()
            .find(|dungeon| dungeon.name.eq_ignore_ascii_case(name))
            .cloned()
    })
}

pub fn registered_dungeon_names() -> Vec<String> {
    dungeon_catalog()
        .read()
        .map(|catalog| catalog.iter().map(|dungeon| dungeon.name.clone()).collect())
        .unwrap_or_default()
}

/// Dungeons a player may choose when venturing into the dungeon, or into
/// `quality` (CR 701.49a, d).
pub fn venture_dungeon_names(quality: Option<&str>) -> Vec<String> {
    dungeon_catalog()
        .read()
        .map(|catalog| {
            catalog
                .iter()
                .filter(|dungeon| dungeon.can_enter_by(quality))
                .map(|dungeon| dungeon.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub fn first_room_name(dungeon_name: &str) -> Option<String> {
    lookup_dungeon(dungeon_name).map(|dungeon| dungeon.first_room().name.clone())
}

pub fn next_room_names(dungeon_name: &str, room_name: &str) -> Option<Vec<String>> {
    lookup_dungeon(dungeon_name)?
        .room(room_name)
        .map(|room| room.leads_to.clone())
}

/// The room ability printed in `room_name` of `dungeon_name` (CR 309.4c).
pub(crate) fn room_ability(dungeon_name: &str, room_name: &str) -> Option<TriggeredAbility> {
    lookup_dungeon(dungeon_name)?
        .room(room_name)
        .map(|room| room.ability.clone())
}
