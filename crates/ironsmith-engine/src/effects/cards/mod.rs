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
mod miracle_draw;
mod put_tagged_remainder_on_library_bottom;
mod rearrange_looked_cards_in_library;
mod reorder_top_planar_deck;
mod reveal;
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
pub(crate) use miracle_draw::{draw_cards_with_miracle_window, miracle_reveal_event};
pub use put_tagged_remainder_on_library_bottom::PutTaggedRemainderOnLibraryBottomEffect;
pub use rearrange_looked_cards_in_library::RearrangeLookedCardsInLibraryEffect;
pub use reorder_top_planar_deck::ReorderTopPlanarDeckEffect;
pub use reveal_from_hand::{RevealFromHandEffect, RevealSourceFromHandEffect};
pub(crate) use reveal_from_hand::{is_exact_reveal_selection, legal_reveal_from_hand_cards};
pub use reveal_tagged::RevealTaggedEffect;
pub use reveal_top::RevealTopEffect;
pub use scry::{EachPlayerScryEffect, FatesealEffect, ScryEffect};
pub use search_library::SearchLibraryEffect;
pub use search_library_slots::{SearchLibrarySlot, SearchLibrarySlotsEffect};
pub use shuffle_graveyard_into_library::ShuffleGraveyardIntoLibraryEffect;
pub use shuffle_hand_and_graveyard_into_library::ShuffleHandAndGraveyardIntoLibraryEffect;
pub use shuffle_library::ShuffleLibraryEffect;
pub use surveil::SurveilEffect;

pub(crate) use draw_cards::{
    AutomaticDrawRevealCandidate, HiddenDrawRevealMode, automatic_draw_reveal_boolean_context,
    automatic_draw_reveal_candidate_for_pending, automatic_reveal_events_for_draw,
    collect_automatic_draw_reveal_candidates, draw_observation, emit_automatic_draw_reveal_event,
    execute_scoped_draw_replacement_effects, hidden_automatic_draw_reveal_description,
    pending_hidden_automatic_draw_reveal,
};

// Internal receipt consumers share the same discard observation/commit boundary.
pub(crate) use discard::{commit_selected_discard_batch, prepare_selected_discard_batch};
pub(crate) use discard_hand::discard_hand_cards;

pub(crate) use draw_cards::execute_turn_draw_proposal;

pub(crate) use reveal::{
    public_reveal_observation, public_reveal_view, reveal_objects, reveal_objects_with_outputs,
};

mod library_arrangement;
pub(crate) use library_arrangement::{
    LibraryCardPosition, arrange_library_cards, position_library_card,
};

mod look;
pub(crate) use library_arrangement::{
    execute_library_instruction_atomically, order_library_cards_top_to_bottom,
};
pub(crate) use look::{look_at_cards, look_at_cards_with_outputs};

pub(crate) use shuffle_library::{
    commit_library_shuffle, shuffle_library, shuffle_library_action, shuffle_library_with_outputs,
};

pub(crate) use draw_cards::{
    PreparedDrawInstruction, execute_prepared_draw_instruction, prepare_draw_instruction,
};
