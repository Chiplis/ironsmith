//! A shared, authored resolution label; independent of source identity or token creation.
use crate::tag::TagKeyWalk;
#[cfg_attr(feature="serde",derive(serde::Serialize,serde::Deserialize))]
#[derive(Debug,Clone,PartialEq,TagKeyWalk)]
pub struct FirstResolutionEachTurnEffect<E> {
    pub label: String,
    pub effects: Vec<E>,
}
