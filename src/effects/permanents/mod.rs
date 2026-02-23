//! Permanent state change effects.
//!
//! This module contains effects that modify the state of permanents on the battlefield,
//! such as tapping, untapping, monstrosity, regeneration, and transformation.

mod attach_objects;
mod attach_to;
mod crew;
mod earthbend;
mod evolve;
mod grant_object_ability;
mod monstrosity;
mod regenerate;
mod renown;
mod saddle;
mod tap;
mod training;
mod transform;
mod untap;

pub use attach_objects::AttachObjectsEffect;
pub use attach_to::AttachToEffect;
pub use crew::CrewCostEffect;
pub use earthbend::EarthbendEffect;
pub use evolve::EvolveEffect;
pub use grant_object_ability::GrantObjectAbilityEffect;
pub use monstrosity::MonstrosityEffect;
pub use regenerate::RegenerateEffect;
pub use renown::RenownEffect;
pub use saddle::{BecomeSaddledUntilEotEffect, SaddleCostEffect};
pub use tap::TapEffect;
pub use training::TrainingEffect;
pub use transform::TransformEffect;
pub use untap::UntapEffect;
