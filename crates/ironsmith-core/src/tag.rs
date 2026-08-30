//! Tagging primitives for cross-effect composition.
//!
//! Tags are dynamic keys used to pass references (objects, players, counts)
//! between effects during the same spell/ability resolution.

use std::borrow::Borrow;
use std::fmt;

/// Runtime tag for cards linked as "exiled with this source object".
pub const SOURCE_EXILED_TAG: &str = "__source_exiled__";

/// The exact new object created by a zone-change replacement before its
/// replacement follow-up effects execute.
pub const ZONE_REPLACEMENT_OBJECT_TAG: &str = "__zone_replacement_object__";

/// Runtime tag for a card explicitly referenced later as "the exiled card".
pub const PRIOR_EXILED_CARD_TAG: &str = "__prior_exiled_card__";

/// Object set produced by reveal-hand effects in the current resolution.
///
/// Keeping this in the shared model lets compiler reference analysis and
/// runtime execution agree on the same typed result-set identity.
pub const REVEALED_THIS_WAY_TAG: &str = "__revealed_this_way__";

/// Runtime tag for the resolving spell or ability's source object.
///
/// This gives object-relative player filters (for example, "this artifact's
/// owner") the same snapshot-backed representation as other tagged-object
/// references without inventing a separate player-filter primitive.
pub const SOURCE_OBJECT_TAG: &str = "__source_object__";

/// Player targets captured when a delayed trigger is registered.
///
/// A delayed trigger may both wait for and later affect a player chosen by
/// the resolving spell or ability. The ordinary target list is local to that
/// resolution, so the delayed registration preserves those players under
/// this system tag.
pub const DELAYED_TARGET_PLAYERS_TAG: &str = "__delayed_target_players__";

/// The object selected by an authored "the chosen object" choice.
///
/// Resolution-local reference analysis may alias this key to a concrete
/// effect tag. When a later ability on the same source refers to the choice,
/// runtime filter contexts populate this canonical key from persistent source
/// memory instead.
pub const CHOSEN_OBJECTS_TAG: &str = "__chosen_objects__";

/// One source snapshot per mana unit spent to cast the current spell.
pub const MANA_SOURCES_SPENT_TO_CAST_TAG: &str = "__mana_sources_spent_to_cast__";
/// The spell or ability whose transaction consumed one concrete mana unit.
pub const MANA_PAID_OBJECT_TAG: &str = "__mana_paid_object__";

/// The color shared by the most permanents on the battlefield.
///
/// This is a derived characteristic rather than a resolution-local result
/// set, so runtime filter contexts compute it on demand under this canonical
/// key instead of a tag written by an earlier effect.
pub const MOST_COMMON_PERMANENT_COLOR_TAG: &str = "most_common_permanent_color";

/// Runtime tag for the creature sacrificed to an exploit action.
pub const EXPLOITED_TAG: &str = "exploited";

/// Runtime tag for the object whose exploit action sacrificed another object.
pub const EXPLOITER_TAG: &str = "exploiter";

/// Runtime tag for cards seen by a surveil action this turn.
pub const SURVEILLED_THIS_TURN_TAG: &str = "__surveilled_this_turn__";

/// Runtime action-event tag for the card put into a graveyard while
/// performing the manifest-dread keyword action.
pub const MANIFEST_DREAD_GRAVEYARD_TAG: &str = "__manifest_dread_graveyard__";

/// The complete set of attackers captured by a group attack trigger.
pub const ATTACKING_GROUP_TAG: &str = "__attacking_group__";

/// The complete set of sources captured by a one-or-more combat-damage
/// trigger.
///
/// This preserves the individual source controllers for follow-ups such as
/// "the controller of those creatures," even though the trigger itself is
/// coalesced into one simultaneous damage-batch event.
pub const COMBAT_DAMAGE_GROUP_TAG: &str = "__combat_damage_group__";

/// The complete set of objects captured by a one-or-more zone-change trigger.
///
/// The snapshots are the matched objects' last-known information, so aggregate
/// values in the triggered ability remain stable after those objects leave
/// their original zone.
pub const ZONE_CHANGE_GROUP_TAG: &str = "__zone_change_group__";

/// The player who currently holds the initiative designation.
///
/// Runtime filter contexts populate this system tag from game state so typed
/// player references can follow the designation as it changes hands.
pub const INITIATIVE_HOLDER_TAG: &str = "__initiative_holder__";

/// Snapshots processed before the current object in an ordered iteration.
pub const PREVIOUS_ITERATED_OBJECTS_TAG: &str = "__previous_iterated_objects__";

/// Modified creatures controlled by the caster when the current spell was cast.
///
/// This preserves the cast-time set for effects whose value is defined by
/// "modified creatures you controlled as you cast this spell", rather than
/// accidentally recounting the battlefield when the spell resolves.
pub const CAST_MODIFIED_CREATURES_TAG: &str = "__cast_modified_creatures__";

/// Objects controlled by the caster when the current spell was cast.
///
/// This preserves the cast-time set for aggregate values such as "the
/// greatest power among creatures you controlled as you cast this spell".
pub const CAST_CONTROLLED_OBJECTS_TAG: &str = "__cast_controlled_objects__";

/// Dynamic tag key used by the tagging system.
///
/// Using an owned key instead of `&'static str` enables tags built at runtime
/// while keeping convenient string-based APIs.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TagKey(String);

impl TagKey {
    /// Create a new tag key from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the tag key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for TagKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for TagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TagKey").field(&self.0).finish()
    }
}

impl fmt::Display for TagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Borrow<str> for TagKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for TagKey {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl From<String> for TagKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&String> for TagKey {
    fn from(value: &String) -> Self {
        Self::new(value.clone())
    }
}
