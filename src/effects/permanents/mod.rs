//! Permanent state change effects.
//!
//! This module contains effects that modify the state of permanents on the battlefield,
//! such as tapping, untapping, monstrosity, regeneration, and transformation.

mod attach_objects;
mod attach_to;
mod become_basic_land_type_choice;
mod become_color_choice;
mod become_creature_type_choice;
mod crew;
mod earthbend;
mod evolve;
mod flip;
mod grant_object_ability;
mod monstrosity;
mod ninjutsu;
mod phase_out;
mod regenerate;
mod renown;
mod saddle;
mod soulbond_pair;
mod tap;
mod transform;
mod unearth;
mod untap;

pub use attach_objects::AttachObjectsEffect;
pub use attach_to::AttachToEffect;
pub use become_basic_land_type_choice::BecomeBasicLandTypeChoiceEffect;
pub use become_color_choice::BecomeColorChoiceEffect;
pub use become_creature_type_choice::BecomeCreatureTypeChoiceEffect;
pub use crew::CrewCostEffect;
pub use earthbend::EarthbendEffect;
pub use evolve::EvolveEffect;
pub use flip::FlipEffect;
pub use grant_object_ability::GrantObjectAbilityEffect;
pub use monstrosity::MonstrosityEffect;
pub use ninjutsu::{NinjutsuCostEffect, NinjutsuEffect};
pub use phase_out::PhaseOutEffect;
pub use regenerate::RegenerateEffect;
pub use renown::RenownEffect;
pub use saddle::{BecomeSaddledUntilEotEffect, SaddleCostEffect};
pub use soulbond_pair::SoulbondPairEffect;
pub use tap::TapEffect;
pub use transform::TransformEffect;
pub use unearth::UnearthEffect;
pub use untap::UntapEffect;
