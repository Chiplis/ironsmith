//! Choose retained triggered-ability occurrences by an externally invoked class.
use crate::tag::TagKeyWalk;
#[cfg_attr(feature="serde",derive(serde::Serialize,serde::Deserialize))]
#[derive(Debug,Clone,PartialEq,TagKeyWalk)]
pub struct TriggerAbilitiesByClassEffect {
    pub class: String,
    pub sources: crate::ObjectFilter,
    /// An explicit source selection (including real targets) keeps its own
    /// declaration; legacy finite source filters remain unchanged.
    #[cfg_attr(feature="serde",serde(default))]
    pub source_selector: Option<crate::ChooseSpec>,
    /// Choose up to this many distinct occurrences, with no target selection.
    pub maximum: u32,
    /// Zero preserves the original optional Harmony-style finite choice.
    #[cfg_attr(feature="serde",serde(default))]
    pub minimum: u32,
    /// The selector may be someone other than the sources' controller.
    #[cfg_attr(feature="serde",serde(default))]
    pub chooser: Option<crate::PlayerFilter>,
    /// Optional surrounding named action; does not itself publish that action.
    #[cfg_attr(feature="serde",serde(default))]
    pub action_name: Option<String>,
}

#[cfg(all(test,feature="serde"))]
mod compatibility_tests {
 #[test]
 fn original_optional_class_program_artifact_defaults_remain_unchanged() {
  let old=serde_json::json!({"class":"echo bell","sources":crate::ObjectFilter::default(),"maximum":1,"action_name":null});
  let restored:super::TriggerAbilitiesByClassEffect=serde_json::from_value(old).unwrap();
  assert_eq!(restored.minimum,0);assert_eq!(restored.chooser,None);assert_eq!(restored.maximum,1);
 }
}
