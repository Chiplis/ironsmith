//! Permanent state change effects.
//!
//! This module contains effects that modify the state of permanents on the battlefield,
//! such as tapping, untapping, monstrosity, regeneration, and transformation.

mod attach_objects;
mod attach_to;
mod crew;
mod earthbend;
mod grant_object_ability;
mod monstrosity;
mod regenerate;
mod tap;
mod transform;
mod untap;

pub use attach_objects::AttachObjectsEffect;
pub use attach_to::AttachToEffect;
pub use crew::CrewCostEffect;
pub use earthbend::EarthbendEffect;
pub use grant_object_ability::GrantObjectAbilityEffect;
pub use monstrosity::MonstrosityEffect;
pub use regenerate::RegenerateEffect;
pub use tap::TapEffect;
pub use transform::TransformEffect;
pub use untap::UntapEffect;
