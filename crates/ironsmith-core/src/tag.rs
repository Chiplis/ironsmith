//! Tagging primitives for cross-effect composition.
//!
//! Tags are dynamic keys used to pass references (objects, players, counts)
//! between effects during the same spell/ability resolution.

#[path = "tag/tag_walk.rs"]
mod tag_walk;

pub use ironsmith_tag_walk_derive::TagKeyWalk;
pub use tag_walk::{TagKeyWalk, tag_keys_of};

use std::borrow::Borrow;
use std::fmt;

/// Runtime tag for cards linked as "exiled with this source object".
pub const SOURCE_EXILED_TAG: &str = "__source_exiled__";

/// Exact retained result of an instruction or cost that exiles its source.
/// A changed/prevented action retains its receipt-result/original incarnation;
/// permission consumers still require that object actually to be in exile.
/// Never widened to the source's other linked exile objects.
pub const SOURCE_EXILED_SELF_TAG: &str = "__source_exiled_self__";

/// Exact public-zone successor created by the original self-exile cost action.
/// Captured before replacement additions; prevention and hidden arrivals bind
/// an empty set. Stack admission must never repoint this completed receipt.
pub const SOURCE_COST_PUBLIC_ARRIVAL_TAG: &str = "__source_cost_public_arrival__";

/// Runtime tag for only the cards the current resolution exiled with its
/// source. Filter contexts widen [`SOURCE_EXILED_TAG`] to every linked card,
/// so "each other card exiled with ~" excludes the just-exiled card through
/// this resolution-scoped identity.
pub const SOURCE_EXILED_THIS_RESOLUTION_TAG: &str = "__source_exiled_this_resolution__";

/// Runtime tag for the permanents the source sacrificed to its devour ability
/// as it entered ("the number of Goblins it devoured", CR 702.82b). Evaluated
/// from the source's recorded devour snapshots rather than captured.
pub const SOURCE_DEVOURED_TAG: &str = "__source_devoured__";

/// Exact creature chosen by one completed Ring-temptation action. This is
/// event evidence, not a dynamically resolved current-bearer reference.
pub const RING_BEARER_CHOSEN_TAG: &str = "__ring_bearer_chosen__";

/// Runtime tag, recorded on a token when it is created, for the objects the
/// creating ability exiled to pay its cost, the ability's source included
/// ("all triggered abilities of the exiled cards", The Book of Vile
/// Darkness).
pub const COST_EXILED_TAG: &str = "__cost_exiled__";

/// Runtime tag for cards in exile that "you" (the filter context's player)
/// exiled: cards linked as exiled by a source that player controls, or that
/// its owner controlled when it left the battlefield ("cards you exiled").
/// Evaluated directly from the exile links rather than captured.
pub const EXILED_BY_YOU_TAG: &str = "__exiled_by_you__";

/// Runtime tag for the card on top of the filter context player's library
/// ("that card" after "as long as the top card of your library is ...").
/// Evaluated from the live library (CR 401.1), never captured.
pub const TOP_OF_YOUR_LIBRARY_TAG: &str = "__top_of_your_library__";

/// Runtime tag for the objects chosen in earlier rounds of the enclosing
/// repeated process ("can't choose a card already chosen for <this>"). The
/// process owns it: after each completed round it appends that round's choice
/// before the next round chooses (Forgotten Lore).
pub const PRIOR_PROCESS_CHOICES_TAG: &str = "__prior_process_choices__";

/// The exact new object created by a zone-change replacement before its
/// replacement follow-up effects execute.
pub const ZONE_REPLACEMENT_OBJECT_TAG: &str = "__zone_replacement_object__";
/// The card(s) an "Exile a card from your hand" activation cost exiled,
/// published by cost payment to the ability ("the card exiled this way").
pub const COST_EXILED_FROM_HAND_TAG: &str = "__cost_exiled_from_hand__";

/// Runtime tag for a card explicitly referenced later as "the exiled card".
pub const PRIOR_EXILED_CARD_TAG: &str = "__prior_exiled_card__";

/// Object set produced by reveal-hand effects in the current resolution.
///
/// Keeping this in the shared model lets compiler reference analysis and
/// runtime execution agree on the same typed result-set identity.
pub const REVEALED_THIS_WAY_TAG: &str = "__revealed_this_way__";

/// Object set produced by a private look at a player's hand in the current
/// resolution ("look at target opponent's hand and exile those cards").
///
/// Distinct from [`REVEALED_THIS_WAY_TAG`]: a look shows the cards only to the
/// looking player, so this set is resolution-local bookkeeping and never marks
/// the cards publicly revealed.
pub const LOOKED_AT_HAND_TAG: &str = "__looked_at_hand__";

/// Snapshots of the spells cast this turn, gathered as the comparison set of
/// a same-name check ("a spell with the same name as a spell that was cast
/// this turn").
pub const SPELLS_CAST_THIS_TURN_TAG: &str = "__spells_cast_this_turn__";

/// Runtime tag for the resolving spell or ability's source object.
///
/// This gives object-relative player filters (for example, "this artifact's
/// owner") the same snapshot-backed representation as other tagged-object
/// references without inventing a separate player-filter primitive.
pub const SOURCE_OBJECT_TAG: &str = "__source_object__";

/// The actual pre-payment creature snapshot sacrificed for this source's
/// Emerge alternative cost. Imported only by that incarnation's ETB event.
pub const SOURCE_EMERGE_SACRIFICE_TAG: &str = "__source_emerge_sacrifice__";

/// A sacrifice cost's announced object set and its completed original action
/// are different references when a replacement changes the payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SacrificeCostTag {
    Selected(usize),
    OriginalResult(usize),
}

impl SacrificeCostTag {
    pub fn parse(tag: &TagKey) -> Option<Self> {
        if let Some(ordinal) = tag.as_str().strip_prefix("sacrifice_cost_") {
            return ordinal.parse().ok().map(Self::Selected);
        }
        tag.as_str().strip_prefix("__original_sacrifice_cost_")?
            .parse().ok().map(Self::OriginalResult)
    }

    pub fn key(self) -> TagKey {
        match self {
            Self::Selected(ordinal) => TagKey::new(format!("sacrifice_cost_{ordinal}")),
            Self::OriginalResult(ordinal) => TagKey::new(format!("__original_sacrifice_cost_{ordinal}")),
        }
    }

    pub fn original_result_key(self) -> TagKey {
        match self { Self::Selected(ordinal) | Self::OriginalResult(ordinal) => Self::OriginalResult(ordinal).key() }
    }
}

/// Runtime player tag for the opponent a resolving clash was performed with
/// (CR 701.30a). "Clash with an opponent. ... Otherwise, that player ..."
/// refers back to this player.
pub const CLASH_OPPONENT_TAG: &str = "__clash_opponent__";

/// Runtime tag for the object whose effect granted the resolving ability to
/// its source (CR 113.3, 613.1f): the Equipment or Aura in `Equipped creature
/// has "... Return Trusty Boomerang to its owner's hand."`. Captured when an
/// effect-granted ability is activated or triggers; absent for printed
/// abilities.
pub const GRANTING_SOURCE_TAG: &str = "__granting_source__";

/// Player targets captured when a delayed trigger is registered.
///
/// A delayed trigger may both wait for and later affect a player chosen by
/// the resolving spell or ability. The ordinary target list is local to that
/// resolution, so the delayed registration preserves those players under
/// this system tag.
pub const DELAYED_TARGET_PLAYERS_TAG: &str = "__delayed_target_players__";
/// The player chosen for "up to N target cards from a player's graveyard"
/// when no card was targeted; "that player" names them (Lodestone Bauble).
pub const TARGET_GRAVEYARD_PLAYER_TAG: &str = "__target_graveyard_player__";
/// The single player a permanent's own entering trigger targeted, exposed to
/// that permanent's linked leaves-the-battlefield trigger as "that player"
/// (CR 607.2a).
pub const LINKED_TRIGGER_PLAYER_TAG: &str = "__linked_trigger_player__";

/// The controller the triggering event names beyond its trigger player, such
/// as the controller of the spell or ability that caused a discard ("When a
/// spell or ability an opponent controls causes you to discard this card,
/// that player ..."). Populated from the event when a triggered ability
/// resolves.
pub const TRIGGERING_EVENT_CONTROLLER_TAG: &str = "__triggering_event_controller__";
pub const TRIGGERING_EVENT_CAUSE_CONTROLLER_TAG: &str = "__triggering_event_cause_controller__";

/// The object selected by an authored "the chosen object" choice.
///
/// Resolution-local reference analysis may alias this key to a concrete
/// effect tag. When a later ability on the same source refers to the choice,
/// runtime filter contexts populate this canonical key from persistent source
/// memory instead.
pub const CHOSEN_OBJECTS_TAG: &str = "__chosen_objects__";

/// "both creatures" in a "<subject> blocks or becomes blocked by a creature"
/// trigger: the subject half of the pair. Lowering rebinds it to the source
/// or to the attached (equipped/enchanted) creature the trigger watches.
pub const BLOCK_PAIR_SUBJECT_TAG: &str = "__block_pair_subject__";

/// The attacking creature an attack cost is being charged for (CR 508.1d),
/// bound while that cost's dynamic amount is resolved.
pub const TAXED_ATTACKER_TAG: &str = "__taxed_attacker__";

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

/// The two exact participants of a matched attachment transition. The
/// recipient is the permanent "that creature/permanent" in its body.
pub const TRIGGER_ATTACHMENT_TAG: &str = "__trigger_attachment__";
pub const TRIGGER_ATTACHMENT_RECIPIENT_TAG: &str = "__trigger_attachment_recipient__";

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
/// Matched objects in one simultaneous tap/untap instruction.
pub const TAP_STATE_GROUP_TAG: &str = "__tap_state_group__";
/// Exact participants of one matched simultaneous phasing transition.
pub const PHASING_GROUP_TAG: &str = "__phasing_group__";
/// Frozen actor and directly attacked player of one declared attack pair.
pub const ATTACK_DECLARATION_ACTOR_TAG: &str = "__attack_declaration_actor__";
pub const ATTACK_DECLARATION_DEFENDER_TAG: &str = "__attack_declaration_defender__";
/// Controller when the triggering completed damage was dealt, not the source's current controller.
pub const DAMAGE_SOURCE_CONTROLLER_TAG: &str = "__damage_source_controller__";
/// Live controllers attacking the event's frozen defender when an effect
/// constructs its filter context (CR 508.6), not the declaration's old actors.
pub const CURRENT_PLAYERS_ATTACKING_EVENT_DEFENDER_TAG: &str = "__current_players_attacking_event_defender__";

/// The players a source chose ("As this enters, choose two players", Sower of
/// Discord). Runtime filter contexts populate this system tag from the
/// source's recorded player choices, so "one of the chosen players" is
/// `PlayerFilter::TaggedPlayer` of this key.
pub const SOURCE_CHOSEN_PLAYERS_TAG: &str = "__source_chosen_players__";

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

/// The card-global reference keys the compiler and runtime agree on by name.
/// A parse binds each of them once, in the document's symbol scope.
pub const WELL_KNOWN_TAGS: &[&str] = &[
    SOURCE_EXILED_TAG,
    SOURCE_EXILED_SELF_TAG,
    SOURCE_EXILED_THIS_RESOLUTION_TAG,
    EXILED_BY_YOU_TAG,
    ZONE_REPLACEMENT_OBJECT_TAG,
    PRIOR_EXILED_CARD_TAG,
    REVEALED_THIS_WAY_TAG,
    LOOKED_AT_HAND_TAG,
    SOURCE_OBJECT_TAG,
    DELAYED_TARGET_PLAYERS_TAG,
    CHOSEN_OBJECTS_TAG,
    MANA_SOURCES_SPENT_TO_CAST_TAG,
    MANA_PAID_OBJECT_TAG,
    MOST_COMMON_PERMANENT_COLOR_TAG,
    EXPLOITED_TAG,
    EXPLOITER_TAG,
    SURVEILLED_THIS_TURN_TAG,
    MANIFEST_DREAD_GRAVEYARD_TAG,
    ATTACKING_GROUP_TAG,
    COMBAT_DAMAGE_GROUP_TAG,
    ZONE_CHANGE_GROUP_TAG,
    TAP_STATE_GROUP_TAG,
    PHASING_GROUP_TAG,
    ATTACK_DECLARATION_ACTOR_TAG,
    ATTACK_DECLARATION_DEFENDER_TAG,
    DAMAGE_SOURCE_CONTROLLER_TAG,
    CURRENT_PLAYERS_ATTACKING_EVENT_DEFENDER_TAG,
    INITIATIVE_HOLDER_TAG,
    PREVIOUS_ITERATED_OBJECTS_TAG,
    CAST_MODIFIED_CREATURES_TAG,
    CAST_CONTROLLED_OBJECTS_TAG,
    SOURCE_COST_PUBLIC_ARRIVAL_TAG,
    SOURCE_EMERGE_SACRIFICE_TAG,
];

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
