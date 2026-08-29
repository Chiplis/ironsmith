use crate::ChooseSpec;
use crate::color::ColorSet;
use crate::effect::EffectId;
use crate::filter::Comparison;
use crate::model::ast::EffectAst;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use crate::{PlayerFilter, TagKey};
use std::sync::Arc;

use super::facts::LoweringFrame;

#[derive(Debug, Clone, PartialEq)]
pub enum RefState<T> {
    Known(T),
    Unknown,
    Ambiguous,
}

/// A stable object target introduced by an earlier declaration in the same
/// resolving instruction. Reference resolution retains every independently
/// declared slot so a later definite description can select the unique slot
/// whose typed filter it refines.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTargetBinding {
    pub tag: TagKey,
    pub discriminator: ObjectTargetDiscriminator,
}

/// Typed characteristics that distinguish independently declared object
/// target slots. Presentation-only filter fields are intentionally absent.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectTargetDiscriminator {
    zone: Option<Zone>,
    controller: Option<PlayerFilter>,
    owner: Option<PlayerFilter>,
    card_types: Vec<CardType>,
    all_card_types: Vec<CardType>,
    excluded_card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    all_subtypes: Vec<Subtype>,
    excluded_subtypes: Vec<Subtype>,
    supertypes: Vec<Supertype>,
    excluded_supertypes: Vec<Supertype>,
    colors: Option<ColorSet>,
    required_colors: Option<ColorSet>,
    colorless: bool,
    multicolored: bool,
    monocolored: bool,
    token: bool,
    nontoken: bool,
    tapped: bool,
    untapped: bool,
    attacking: bool,
    blocking: bool,
    power: Option<Comparison>,
    toughness: Option<Comparison>,
    mana_value: Option<Comparison>,
    name: Option<String>,
}

impl ObjectTargetDiscriminator {
    pub fn from_filter(filter: &ObjectFilter) -> Self {
        Self {
            zone: filter.zone,
            controller: filter.controller.clone(),
            owner: filter.owner.clone(),
            card_types: filter.card_types.clone(),
            all_card_types: filter.all_card_types.clone(),
            excluded_card_types: filter.excluded_card_types.clone(),
            subtypes: filter.subtypes.clone(),
            all_subtypes: filter.all_subtypes.clone(),
            excluded_subtypes: filter.excluded_subtypes.clone(),
            supertypes: filter.supertypes.clone(),
            excluded_supertypes: filter.excluded_supertypes.clone(),
            colors: filter.colors,
            required_colors: filter.required_colors,
            colorless: filter.colorless,
            multicolored: filter.multicolored,
            monocolored: filter.monocolored,
            token: filter.token,
            nontoken: filter.nontoken,
            tapped: filter.tapped,
            untapped: filter.untapped,
            attacking: filter.attacking,
            blocking: filter.blocking,
            power: filter.power.clone(),
            toughness: filter.toughness.clone(),
            mana_value: filter.mana_value.clone(),
            name: filter.name.clone(),
        }
    }

    pub fn matches_filter(&self, filter: &ObjectFilter) -> bool {
        self == &Self::from_filter(filter)
    }
}

impl ObjectTargetBinding {
    pub fn new(tag: TagKey, filter: &ObjectFilter) -> Self {
        Self {
            tag,
            discriminator: ObjectTargetDiscriminator::from_filter(filter),
        }
    }
}

pub fn join_object_target_bindings(
    left: &Arc<Vec<ObjectTargetBinding>>,
    right: &Arc<Vec<ObjectTargetBinding>>,
) -> Arc<Vec<ObjectTargetBinding>> {
    Arc::new(
        left.iter()
            .filter(|binding| right.contains(binding))
            .cloned()
            .collect(),
    )
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceFrame {
    pub last_effect_id: Option<EffectId>,
    pub last_library_search_effect_id: Option<EffectId>,
    pub last_object_tag: Option<String>,
    pub recent_object_target_bindings: Arc<Vec<ObjectTargetBinding>>,
    pub snapshot_tag_aliases: Vec<(String, String)>,
    pub last_it_choice_is_set: bool,
    pub last_player_filter: Option<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub recent_player_choice_tags: Vec<String>,
    pub iterated_player: bool,
    pub iterated_object: bool,
    pub auto_tag_object_targets: bool,
    pub force_auto_tag_object_targets: bool,
    pub allow_life_event_value: bool,
    pub bind_unbound_x_to_last_effect: bool,
}

impl ReferenceFrame {
    pub fn from_lowering_frame(frame: &LoweringFrame) -> Self {
        Self {
            last_effect_id: frame.last_effect_id,
            last_library_search_effect_id: frame.last_library_search_effect_id,
            last_object_tag: frame.last_object_tag.clone(),
            recent_object_target_bindings: Arc::default(),
            snapshot_tag_aliases: frame.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: frame.last_it_choice_is_set,
            last_player_filter: frame.last_player_filter.clone(),
            source_object_antecedent: frame.source_object_antecedent,
            recent_player_choice_tags: frame.recent_player_choice_tags.clone(),
            iterated_player: frame.iterated_player,
            iterated_object: frame.iterated_object,
            auto_tag_object_targets: frame.auto_tag_object_targets,
            force_auto_tag_object_targets: frame.force_auto_tag_object_targets,
            allow_life_event_value: frame.allow_life_event_value,
            bind_unbound_x_to_last_effect: frame.bind_unbound_x_to_last_effect,
        }
    }

    pub fn to_lowering_frame(&self) -> LoweringFrame {
        LoweringFrame {
            last_effect_id: self.last_effect_id,
            last_library_search_effect_id: self.last_library_search_effect_id,
            last_object_tag: self.last_object_tag.clone(),
            snapshot_tag_aliases: self.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: self.last_it_choice_is_set,
            last_revealed_tag: None,
            last_revealed_zone: None,
            last_revealed_player_filter: None,
            last_exiled_collection_tag: None,
            last_exiled_collection_is_plural: false,
            last_player_filter: self.last_player_filter.clone(),
            source_object_antecedent: self.source_object_antecedent,
            recent_player_choice_tags: self.recent_player_choice_tags.clone(),
            iterated_player: self.iterated_player,
            iterated_object: self.iterated_object,
            auto_tag_object_targets: self.auto_tag_object_targets,
            force_auto_tag_object_targets: self.force_auto_tag_object_targets,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }
}

impl<T: Clone + PartialEq> RefState<T> {
    pub fn from_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Known(value),
            None => Self::Unknown,
        }
    }

    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown | Self::Ambiguous => None,
        }
    }

    pub fn join(left: &Self, right: &Self) -> Self {
        match (left, right) {
            (Self::Known(left), Self::Known(right)) if left == right => Self::Known(left.clone()),
            (Self::Unknown, Self::Unknown) => Self::Unknown,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            (Self::Known(_), Self::Known(_)) => Self::Ambiguous,
            (Self::Known(_), Self::Unknown) | (Self::Unknown, Self::Known(_)) => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReferenceImports {
    pub last_object_tag: Option<TagKey>,
    pub recent_object_target_bindings: Arc<Vec<ObjectTargetBinding>>,
    /// Stable parse-time aliases already bound by the enclosing reference
    /// context. Nested lowering must import these alongside `last_object_tag`;
    /// otherwise compiling a conditional branch can erase an alias before the
    /// condition itself is lowered.
    pub snapshot_tag_aliases: Vec<(String, String)>,
    pub last_it_choice_is_set: bool,
    pub iterated_object: bool,
    pub last_player_filter: Option<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub last_effect_id: Option<EffectId>,
    pub last_library_search_effect_id: Option<EffectId>,
}

impl ReferenceImports {
    pub fn is_empty(&self) -> bool {
        self.last_object_tag.is_none()
            && self.recent_object_target_bindings.is_empty()
            && self.snapshot_tag_aliases.is_empty()
            && !self.last_it_choice_is_set
            && !self.iterated_object
            && self.last_player_filter.is_none()
            && !self.source_object_antecedent
            && self.last_effect_id.is_none()
            && self.last_library_search_effect_id.is_none()
    }

    pub fn with_last_object_tag(tag: impl Into<TagKey>) -> Self {
        Self {
            last_object_tag: Some(tag.into()),
            last_it_choice_is_set: false,
            iterated_object: false,
            ..Default::default()
        }
    }

    pub fn from_frame(frame: &ReferenceFrame) -> Self {
        Self {
            last_object_tag: frame.last_object_tag.as_ref().map(TagKey::from),
            recent_object_target_bindings: frame.recent_object_target_bindings.clone(),
            snapshot_tag_aliases: frame.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: frame.last_it_choice_is_set,
            iterated_object: frame.iterated_object,
            last_player_filter: frame.last_player_filter.clone(),
            source_object_antecedent: frame.source_object_antecedent,
            last_effect_id: frame.last_effect_id,
            last_library_search_effect_id: frame.last_library_search_effect_id,
        }
    }

    pub fn from_lowering_frame(frame: &LoweringFrame) -> Self {
        Self::from_frame(&ReferenceFrame::from_lowering_frame(frame))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceEnv {
    pub last_object_tag: RefState<TagKey>,
    pub recent_object_target_bindings: Arc<Vec<ObjectTargetBinding>>,
    /// Parse-time tag aliases bound by `SnapshotLastObjectTag`, mapping a stable
    /// parse-time placeholder tag to the concrete tag captured from
    /// `last_object_tag` at snapshot time. Survives later `last_object_tag`
    /// clobbers so composed effects can still reference an earlier looked pool.
    pub snapshot_tag_aliases: Vec<(String, String)>,
    pub last_it_choice_is_set: bool,
    pub last_player_filter: RefState<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub last_effect_id: RefState<EffectId>,
    pub last_library_search_effect_id: RefState<EffectId>,
    pub iterated_player: bool,
    pub iterated_object: bool,
    pub allow_life_event_value: bool,
    pub bind_unbound_x_to_last_effect: bool,
}

impl Default for ReferenceEnv {
    fn default() -> Self {
        Self {
            last_object_tag: RefState::Unknown,
            recent_object_target_bindings: Arc::default(),
            snapshot_tag_aliases: Vec::new(),
            last_it_choice_is_set: false,
            last_player_filter: RefState::Unknown,
            source_object_antecedent: false,
            last_effect_id: RefState::Unknown,
            last_library_search_effect_id: RefState::Unknown,
            iterated_player: false,
            iterated_object: false,
            allow_life_event_value: false,
            bind_unbound_x_to_last_effect: false,
        }
    }
}

impl ReferenceEnv {
    pub fn from_imports(
        imports: &ReferenceImports,
        iterated_player: bool,
        allow_life_event_value: bool,
        bind_unbound_x_to_last_effect: bool,
        initial_last_effect_id: Option<EffectId>,
    ) -> Self {
        Self {
            last_object_tag: RefState::from_option(imports.last_object_tag.clone()),
            recent_object_target_bindings: imports.recent_object_target_bindings.clone(),
            snapshot_tag_aliases: imports.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: imports.last_it_choice_is_set,
            last_player_filter: RefState::from_option(imports.last_player_filter.clone()),
            source_object_antecedent: imports.source_object_antecedent,
            last_effect_id: RefState::from_option(
                imports.last_effect_id.or(initial_last_effect_id),
            ),
            last_library_search_effect_id: RefState::from_option(
                imports.last_library_search_effect_id,
            ),
            iterated_player,
            iterated_object: imports.iterated_object,
            allow_life_event_value,
            bind_unbound_x_to_last_effect,
        }
    }

    pub fn from_frame(frame: &ReferenceFrame) -> Self {
        Self {
            last_object_tag: RefState::from_option(
                frame.last_object_tag.as_ref().map(TagKey::from),
            ),
            recent_object_target_bindings: frame.recent_object_target_bindings.clone(),
            snapshot_tag_aliases: frame.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: frame.last_it_choice_is_set,
            last_player_filter: RefState::from_option(frame.last_player_filter.clone()),
            source_object_antecedent: frame.source_object_antecedent,
            last_effect_id: RefState::from_option(frame.last_effect_id),
            last_library_search_effect_id: RefState::from_option(
                frame.last_library_search_effect_id,
            ),
            iterated_player: frame.iterated_player,
            iterated_object: frame.iterated_object,
            allow_life_event_value: frame.allow_life_event_value,
            bind_unbound_x_to_last_effect: frame.bind_unbound_x_to_last_effect,
        }
    }

    pub fn from_lowering_frame(frame: &LoweringFrame) -> Self {
        Self::from_frame(&ReferenceFrame::from_lowering_frame(frame))
    }

    pub fn to_frame(
        &self,
        auto_tag_object_targets: bool,
        force_auto_tag_object_targets: bool,
    ) -> ReferenceFrame {
        ReferenceFrame {
            last_effect_id: self.last_effect_id.clone().into_option(),
            last_library_search_effect_id: self.last_library_search_effect_id.clone().into_option(),
            last_object_tag: self
                .last_object_tag
                .clone()
                .into_option()
                .map(|tag| tag.as_str().to_string()),
            recent_object_target_bindings: self.recent_object_target_bindings.clone(),
            snapshot_tag_aliases: self.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: self.last_it_choice_is_set,
            last_player_filter: self.last_player_filter.clone().into_option(),
            source_object_antecedent: self.source_object_antecedent,
            recent_player_choice_tags: Vec::new(),
            iterated_player: self.iterated_player,
            iterated_object: self.iterated_object,
            auto_tag_object_targets: auto_tag_object_targets || force_auto_tag_object_targets,
            force_auto_tag_object_targets,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }

    pub fn to_lowering_frame(
        &self,
        auto_tag_object_targets: bool,
        force_auto_tag_object_targets: bool,
    ) -> LoweringFrame {
        self.to_frame(auto_tag_object_targets, force_auto_tag_object_targets)
            .to_lowering_frame()
    }

    pub fn known_last_object_tag(&self) -> Option<&TagKey> {
        match &self.last_object_tag {
            RefState::Known(tag) => Some(tag),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }

    pub fn known_last_player_filter(&self) -> Option<&PlayerFilter> {
        match &self.last_player_filter {
            RefState::Known(filter) => Some(filter),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }

    pub fn has_source_object_antecedent(&self) -> bool {
        self.source_object_antecedent
    }

    pub fn known_last_effect_id(&self) -> Option<EffectId> {
        match self.last_effect_id {
            RefState::Known(id) => Some(id),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceExports {
    pub last_object_tag: RefState<TagKey>,
    pub recent_object_target_bindings: Arc<Vec<ObjectTargetBinding>>,
    pub last_it_choice_is_set: bool,
    pub last_player_filter: RefState<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub last_effect_id: RefState<EffectId>,
    pub last_library_search_effect_id: RefState<EffectId>,
    pub iterated_player: bool,
}

impl Default for ReferenceExports {
    fn default() -> Self {
        Self {
            last_object_tag: RefState::Unknown,
            recent_object_target_bindings: Arc::default(),
            last_it_choice_is_set: false,
            last_player_filter: RefState::Unknown,
            source_object_antecedent: false,
            last_effect_id: RefState::Unknown,
            last_library_search_effect_id: RefState::Unknown,
            iterated_player: false,
        }
    }
}

impl ReferenceExports {
    pub fn from_env(env: &ReferenceEnv) -> Self {
        Self {
            last_object_tag: env.last_object_tag.clone(),
            recent_object_target_bindings: env.recent_object_target_bindings.clone(),
            last_it_choice_is_set: env.last_it_choice_is_set,
            last_player_filter: env.last_player_filter.clone(),
            source_object_antecedent: env.source_object_antecedent,
            last_effect_id: env.last_effect_id.clone(),
            last_library_search_effect_id: env.last_library_search_effect_id.clone(),
            iterated_player: env.iterated_player,
        }
    }

    pub fn join(left: &Self, right: &Self) -> Self {
        Self {
            last_object_tag: RefState::join(&left.last_object_tag, &right.last_object_tag),
            recent_object_target_bindings: join_object_target_bindings(
                &left.recent_object_target_bindings,
                &right.recent_object_target_bindings,
            ),
            last_it_choice_is_set: left.last_it_choice_is_set && right.last_it_choice_is_set,
            last_player_filter: RefState::join(&left.last_player_filter, &right.last_player_filter),
            source_object_antecedent: left.source_object_antecedent
                && right.source_object_antecedent,
            last_effect_id: RefState::join(&left.last_effect_id, &right.last_effect_id),
            last_library_search_effect_id: RefState::join(
                &left.last_library_search_effect_id,
                &right.last_library_search_effect_id,
            ),
            iterated_player: left.iterated_player && right.iterated_player,
        }
    }

    pub fn to_imports(&self) -> ReferenceImports {
        ReferenceImports {
            last_object_tag: self.last_object_tag.clone().into_option(),
            recent_object_target_bindings: self.recent_object_target_bindings.clone(),
            snapshot_tag_aliases: Vec::new(),
            last_it_choice_is_set: self.last_it_choice_is_set,
            iterated_object: false,
            last_player_filter: self.last_player_filter.clone().into_option(),
            source_object_antecedent: self.source_object_antecedent,
            last_effect_id: self.last_effect_id.clone().into_option(),
            last_library_search_effect_id: self.last_library_search_effect_id.clone().into_option(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoweredEffects {
    pub effects: crate::resolution::ResolutionProgram,
    pub choices: Vec<ChooseSpec>,
    pub exports: ReferenceExports,
}

#[derive(Debug, Clone)]
pub struct AnnotatedEffect {
    pub effect: EffectAst,
    pub in_env: ReferenceEnv,
    pub out_env: ReferenceEnv,
    pub assigned_effect_id: Option<EffectId>,
    pub auto_tag_object_targets: bool,
}

#[derive(Debug, Clone)]
pub struct AnnotatedEffectSequence {
    pub effects: Vec<AnnotatedEffect>,
    pub final_env: ReferenceEnv,
}
