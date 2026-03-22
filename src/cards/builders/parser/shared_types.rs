use std::ops::{Deref, DerefMut};

use crate::effect::EffectId;
use crate::filter::PlayerFilter;

use super::reference_model::ReferenceEnv;

const SENTENCE_HELPER_TAG_PREFIX: &str = "__sentence_helper_";

#[derive(Debug, Clone)]
pub(crate) enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedLine {
    pub(crate) original: String,
    pub(crate) normalized: String,
    pub(crate) char_map: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct LineInfo {
    pub(crate) line_index: usize,
    pub(crate) raw_line: String,
    pub(crate) normalized: NormalizedLine,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IdGenContext {
    pub(crate) next_effect_id: u32,
    pub(crate) next_tag_id: u32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LoweringFrame {
    pub(crate) last_effect_id: Option<EffectId>,
    pub(crate) last_object_tag: Option<String>,
    pub(crate) last_player_filter: Option<PlayerFilter>,
    pub(crate) recent_player_choice_tags: Vec<String>,
    pub(crate) iterated_player: bool,
    pub(crate) auto_tag_object_targets: bool,
    pub(crate) force_auto_tag_object_targets: bool,
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompileContext {
    pub(crate) next_effect_id: u32,
    pub(crate) next_tag_id: u32,
}

impl CompileContext {
    pub(crate) fn new() -> Self {
        Self::from_id_gen(IdGenContext::default())
    }

    pub(crate) fn from_id_gen(id_gen: IdGenContext) -> Self {
        Self {
            next_effect_id: id_gen.next_effect_id,
            next_tag_id: id_gen.next_tag_id,
        }
    }

    pub(crate) fn id_gen_context(&self) -> IdGenContext {
        IdGenContext {
            next_effect_id: self.next_effect_id,
            next_tag_id: self.next_tag_id,
        }
    }

    pub(crate) fn apply_id_gen_context(&mut self, id_gen: IdGenContext) {
        self.next_effect_id = id_gen.next_effect_id;
        self.next_tag_id = id_gen.next_tag_id;
    }

    pub(crate) fn next_effect_id(&mut self) -> EffectId {
        let id = EffectId(self.next_effect_id);
        self.next_effect_id += 1;
        id
    }

    pub(crate) fn next_tag(&mut self, prefix: &str) -> String {
        let tag = if matches!(prefix, "exiled" | "looked" | "chosen" | "revealed") {
            format!(
                "{SENTENCE_HELPER_TAG_PREFIX}{prefix}_l0_s0_e{}",
                self.next_tag_id
            )
        } else {
            format!("{prefix}_{}", self.next_tag_id)
        };
        self.next_tag_id += 1;
        tag
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EffectLoweringContext {
    ids: CompileContext,
    frame: LoweringFrame,
}

impl Deref for EffectLoweringContext {
    type Target = LoweringFrame;

    fn deref(&self) -> &Self::Target {
        &self.frame
    }
}

impl DerefMut for EffectLoweringContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.frame
    }
}

impl EffectLoweringContext {
    pub(crate) fn new() -> Self {
        Self {
            ids: CompileContext::new(),
            frame: LoweringFrame::default(),
        }
    }

    pub(crate) fn from_parts(id_gen: IdGenContext, frame: LoweringFrame) -> Self {
        Self {
            ids: CompileContext::from_id_gen(id_gen),
            frame,
        }
    }

    pub(crate) fn id_gen_context(&self) -> IdGenContext {
        self.ids.id_gen_context()
    }

    pub(crate) fn apply_id_gen_context(&mut self, id_gen: IdGenContext) {
        self.ids.apply_id_gen_context(id_gen);
    }

    pub(crate) fn lowering_frame(&self) -> LoweringFrame {
        self.frame.clone()
    }

    pub(crate) fn reference_env(&self) -> ReferenceEnv {
        ReferenceEnv::from_lowering_frame(&self.frame)
    }

    pub(crate) fn apply_reference_env(&mut self, env: &ReferenceEnv) {
        self.apply_reference_frame(env.to_lowering_frame(false, false));
    }

    pub(crate) fn apply_reference_frame(&mut self, frame: LoweringFrame) {
        self.last_effect_id = frame.last_effect_id;
        self.last_object_tag = frame.last_object_tag;
        self.last_player_filter = frame.last_player_filter;
        self.iterated_player = frame.iterated_player;
        self.allow_life_event_value = frame.allow_life_event_value;
        self.bind_unbound_x_to_last_effect = frame.bind_unbound_x_to_last_effect;
    }

    pub(crate) fn apply_lowering_frame(&mut self, frame: LoweringFrame) {
        self.frame = frame;
    }

    pub(crate) fn next_effect_id(&mut self) -> EffectId {
        self.ids.next_effect_id()
    }

    pub(crate) fn next_tag(&mut self, prefix: &str) -> String {
        self.ids.next_tag(prefix)
    }
}
