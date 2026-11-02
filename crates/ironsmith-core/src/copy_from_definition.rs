//! A supplied, compiled card identity used by an ordinary copy operation.
use crate::tag::TagKeyWalk;
#[cfg_attr(feature="serde",derive(serde::Serialize,serde::Deserialize))]
#[derive(Debug,Clone,PartialEq,TagKeyWalk)]
pub struct CopyFromDefinitionEffect<D> {
    pub definition: D,
    pub count: crate::Value,
    pub player: crate::PlayerFilter,
}
