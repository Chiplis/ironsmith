//! Card-related effects.
//!
//! This module contains effects that manipulate cards in zones,
//! such as milling, shuffling libraries, drawing cards, discarding, etc.

mod clash;
mod connive;
mod consult_top_of_library;
mod discard;
mod discard_hand;
mod draw_cards;
mod draw_for_each_tagged_matching;
mod exile_top;
mod exile_until_match;
mod imprint;
mod learn;
mod look_at_hand;
mod look_at_objects;
mod look_at_top;
mod mill;
mod move_top_source_exiled_to_zone;
mod put_tagged_remainder_on_library_bottom;
mod rearrange_looked_cards_in_library;
mod reveal_from_hand;
mod reveal_tagged;
mod reveal_top;
mod scry;
mod search_library;
mod search_library_slots;
pub(crate) mod search_overrides;
mod shuffle_graveyard_into_library;
mod shuffle_hand_and_graveyard_into_library;
mod shuffle_library;
mod shuffle_source_exiled_pile;
mod surveil;

pub use clash::{ClashEffect, ClashOpponentMode};
pub use connive::ConniveEffect;
pub use consult_top_of_library::{ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule};
pub use discard::DiscardEffect;
pub use discard_hand::DiscardHandEffect;
pub use draw_cards::DrawCardsEffect;
pub use draw_for_each_tagged_matching::DrawForEachTaggedMatchingEffect;
pub use exile_top::ExileTopOfLibraryEffect;
pub use exile_until_match::ExileUntilMatchEffect;
pub use imprint::ImprintFromHandEffect;
pub use learn::LearnEffect;
pub use look_at_hand::LookAtHandEffect;
pub use look_at_objects::LookAtObjectsEffect;
pub use look_at_top::LookAtTopCardsEffect;
pub use mill::MillEffect;
pub use move_top_source_exiled_to_zone::MoveTopSourceExiledToZoneEffect;
pub use put_tagged_remainder_on_library_bottom::PutTaggedRemainderOnLibraryBottomEffect;
pub use rearrange_looked_cards_in_library::RearrangeLookedCardsInLibraryEffect;
pub use reveal_from_hand::{RevealFromHandEffect, RevealSourceFromHandEffect};
pub use reveal_tagged::RevealTaggedEffect;
pub use reveal_top::RevealTopEffect;
pub use scry::{EachPlayerScryEffect, FatesealEffect, ScryEffect};
pub use search_library::SearchLibraryEffect;
pub use search_library_slots::{SearchLibrarySlot, SearchLibrarySlotsEffect};
pub use shuffle_graveyard_into_library::ShuffleGraveyardIntoLibraryEffect;
pub use shuffle_hand_and_graveyard_into_library::ShuffleHandAndGraveyardIntoLibraryEffect;
pub use shuffle_library::ShuffleLibraryEffect;
pub use shuffle_source_exiled_pile::ShuffleSourceExiledPileEffect;
pub use surveil::SurveilEffect;

pub(crate) use draw_cards::{
    AutomaticDrawRevealCandidate, automatic_draw_reveal_boolean_context,
    automatic_reveal_events_for_draw, collect_automatic_draw_reveal_candidates,
    emit_automatic_draw_reveal_event,
};
