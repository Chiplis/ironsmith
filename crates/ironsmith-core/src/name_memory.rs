//! Noncopiable names noted for an exact object incarnation.
use crate::tag::TagKeyWalk;
#[cfg_attr(feature="serde",derive(serde::Serialize,serde::Deserialize))]
#[derive(Debug,Clone,PartialEq,TagKeyWalk)]
pub struct NoteObjectNamesEffect {
    #[cfg_attr(feature="serde",serde(default,skip_serializing_if="Option::is_none"))]
    pub pair:Option<crate::LinkedExilePair>,
    pub object:crate::ChooseSpec,
    /// An exact source, explicit object, or previously bound object reference.
    pub for_object:crate::ChooseSpec,
}
