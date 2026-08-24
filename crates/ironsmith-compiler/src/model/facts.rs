use std::ops::{Deref, DerefMut};

use crate::effect::EffectId;
use crate::filter::PlayerFilter;
// This model is still consumed by the legacy runtime-backend pipeline.  Keep
// its source-token payload on that pipeline's lexer type until the lexer
// extraction itself is complete; mixing it with the new public front-end
// token creates two nominally distinct token graphs throughout lowering.
use crate::lexer::OwnedLexToken;
use crate::zone::Zone;

use super::ast::PredicateAst;
use super::reference_state::ReferenceEnv;

const SENTENCE_HELPER_TAG_PREFIX: &str = "__sentence_helper_";

#[derive(Debug, Clone)]
pub enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    FirstPrintedSet(String),
    AttractionLights(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

#[derive(Debug, Clone)]
pub struct NormalizedLine {
    pub original: String,
    pub normalized: String,
    pub char_map: Vec<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineSemanticFacts {
    pub static_ability: StaticLineSemanticFacts,
    pub statement: StatementLineSemanticFacts,
    pub triggered_ability: TriggeredLineSemanticFacts,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatementLineSemanticFacts {
    pub instead_followup: InsteadFollowupFacts,
    pub trailing_instead_if_predicate: Option<PredicateAst>,
    pub replacement_surfaces: Vec<StatementReplacementSurfaceKind>,
    pub as_enters_effect_program: Option<AsEntersEffectProgramFacts>,
    pub as_transforms_effect_program: Option<AsTransformsEffectProgramFacts>,
    pub presentation_label: Option<crate::ability::PresentationLabel>,
    pub creature_type_choice_buff: bool,
    pub leading_condition_intro: Option<StatementConditionIntro>,
    pub repeatable_instant_timing_payment_until_end_of_turn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsEntersEffectProgramFacts {
    pub subject: String,
    pub also_turns_face_up: bool,
    pub turns_face_up_only: bool,
    pub uses_enters_with_counter_surface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsTransformsEffectProgramFacts {
    pub subject: String,
    pub destination: String,
}

impl StatementLineSemanticFacts {
    pub fn has_replacement_surface(&self, expected: StatementReplacementSurfaceKind) -> bool {
        self.replacement_surfaces.contains(&expected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsteadFollowupFacts {
    pub semantics: crate::cards::builders::InsteadSemantics,
    pub conditional_intro: bool,
    pub leading_instead_surface: bool,
}

impl Default for InsteadFollowupFacts {
    fn default() -> Self {
        Self {
            semantics: crate::cards::builders::InsteadSemantics::NonReplacement,
            conditional_intro: false,
            leading_instead_surface: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementConditionIntro {
    If,
    Unless,
    AsLongAs,
    ForAsLongAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementReplacementSurfaceKind {
    BargainedReturnToBattlefield,
    KickedCountOverride,
    KickedMultiZoneToBattlefield,
    ClashWinTopOfLibrary,
    MorbidSearchToBattlefield,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StaticLineSemanticFacts {
    pub explicit_functional_zones: Option<Vec<Zone>>,
    pub references_this_ability_cost: bool,
    pub this_spell_cost: Option<ThisSpellCostFacts>,
    pub presentation_label: Option<crate::ability::PresentationLabel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThisSpellCostFacts {
    pub reduction_cap: Option<i32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriggeredLineSemanticFacts {
    pub compiler_ability: Option<super::CompilerTriggeredAbilityAst>,
    pub intro_surface: Option<super::ast::TriggerIntroSurfaceAst>,
    pub presentation_label: Option<crate::ability::PresentationLabel>,
    pub functional_zones: TriggerFunctionalZoneFacts,
    pub becomes_tapped_during_your_turn: bool,
    pub frequency: TriggerFrequencyFacts,
    pub leading_unless_surface: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TriggerFunctionalZoneFacts {
    pub explicit_zone: Option<Zone>,
    pub returns_self_from_graveyard: bool,
    pub discards_this_card: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TriggerFrequencyFacts {
    pub first_time_each_or_this_turn: bool,
    pub first_time_during_each_of_your_turns: bool,
    pub becomes_crewed: bool,
    pub do_this_limit_each_turn: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LineInfo {
    pub line_index: usize,
    pub display_line_index: usize,
    pub raw_line: String,
    /// Tokens for the original source line, retained by the front end so later
    /// stages never need to lex presentation text again.
    pub source_tokens: Vec<OwnedLexToken>,
    pub normalized: NormalizedLine,
    pub semantic_facts: LineSemanticFacts,
}

#[derive(Debug, Clone, Default)]
pub struct IdGenContext {
    pub next_effect_id: u32,
    pub next_tag_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct LoweringFrame {
    pub last_effect_id: Option<EffectId>,
    pub last_library_search_effect_id: Option<EffectId>,
    pub last_object_tag: Option<String>,
    pub last_it_choice_is_set: bool,
    /// Parse-time tag aliases bound by `SnapshotLastObjectTag`, mapping a
    /// stable parse-time placeholder tag to the concrete runtime tag that was
    /// in `last_object_tag` at snapshot time. Consulted during tag/filter
    /// resolution so composed effects can reference an earlier looked-at pool
    /// even after a later `ChooseObjects` clobbers `last_object_tag`.
    pub snapshot_tag_aliases: Vec<(String, String)>,
    pub last_revealed_tag: Option<String>,
    pub last_revealed_zone: Option<Zone>,
    pub last_revealed_player_filter: Option<PlayerFilter>,
    pub last_exiled_collection_tag: Option<String>,
    /// True when the most recent exile/choose that bound an exiled-collection
    /// tag set aside more than one card (a dynamic or fixed >1 count). Drives
    /// "those exiled cards" (plural) vs "that card" cast-permission wording.
    pub last_exiled_collection_is_plural: bool,
    pub last_player_filter: Option<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub recent_player_choice_tags: Vec<String>,
    pub iterated_player: bool,
    /// True while lowering the body of an object iteration. Kept separate from
    /// `iterated_player` so `__it__` can lower to `ChooseSpec::Iterated`
    /// without rebinding an outer "that player" antecedent.
    pub iterated_object: bool,
    pub auto_tag_object_targets: bool,
    pub force_auto_tag_object_targets: bool,
    pub allow_life_event_value: bool,
    pub bind_unbound_x_to_last_effect: bool,
}

#[derive(Debug, Clone)]
pub struct CompileContext {
    pub next_effect_id: u32,
    pub next_tag_id: u32,
}

impl Default for CompileContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CompileContext {
    pub fn new() -> Self {
        Self::from_id_gen(IdGenContext::default())
    }

    pub fn from_id_gen(id_gen: IdGenContext) -> Self {
        Self {
            next_effect_id: id_gen.next_effect_id,
            next_tag_id: id_gen.next_tag_id,
        }
    }

    pub fn id_gen_context(&self) -> IdGenContext {
        IdGenContext {
            next_effect_id: self.next_effect_id,
            next_tag_id: self.next_tag_id,
        }
    }

    pub fn apply_id_gen_context(&mut self, id_gen: IdGenContext) {
        self.next_effect_id = id_gen.next_effect_id;
        self.next_tag_id = id_gen.next_tag_id;
    }

    pub fn next_effect_id(&mut self) -> EffectId {
        let id = EffectId(self.next_effect_id);
        self.next_effect_id += 1;
        id
    }

    pub fn next_tag(&mut self, prefix: &str) -> String {
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
pub struct EffectLoweringContext {
    ids: CompileContext,
    frame: LoweringFrame,
    reserved_object_result_tag: Option<String>,
}

impl Default for EffectLoweringContext {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn new() -> Self {
        Self {
            ids: CompileContext::new(),
            frame: LoweringFrame::default(),
            reserved_object_result_tag: None,
        }
    }

    pub fn from_parts(id_gen: IdGenContext, frame: LoweringFrame) -> Self {
        Self {
            ids: CompileContext::from_id_gen(id_gen),
            frame,
            reserved_object_result_tag: None,
        }
    }

    pub fn id_gen_context(&self) -> IdGenContext {
        self.ids.id_gen_context()
    }

    pub fn apply_id_gen_context(&mut self, id_gen: IdGenContext) {
        self.ids.apply_id_gen_context(id_gen);
    }

    pub fn lowering_frame(&self) -> LoweringFrame {
        self.frame.clone()
    }

    pub fn reference_env(&self) -> ReferenceEnv {
        ReferenceEnv::from_lowering_frame(&self.frame)
    }

    pub fn apply_reference_env(&mut self, env: &ReferenceEnv) {
        self.apply_reference_frame(env.to_lowering_frame(false, false));
    }

    pub fn apply_reference_frame(&mut self, frame: LoweringFrame) {
        self.last_effect_id = frame.last_effect_id;
        self.last_library_search_effect_id = frame.last_library_search_effect_id;
        self.last_object_tag = frame.last_object_tag;
        self.snapshot_tag_aliases = frame.snapshot_tag_aliases;
        self.last_it_choice_is_set = frame.last_it_choice_is_set;
        self.last_exiled_collection_tag = frame.last_exiled_collection_tag;
        self.last_player_filter = frame.last_player_filter;
        self.source_object_antecedent = frame.source_object_antecedent;
        self.iterated_player = frame.iterated_player;
        self.iterated_object = frame.iterated_object;
        self.allow_life_event_value = frame.allow_life_event_value;
        self.bind_unbound_x_to_last_effect = frame.bind_unbound_x_to_last_effect;
    }

    pub fn apply_lowering_frame(&mut self, frame: LoweringFrame) {
        self.frame = frame;
    }

    pub fn next_effect_id(&mut self) -> EffectId {
        self.ids.next_effect_id()
    }

    pub fn next_tag(&mut self, prefix: &str) -> String {
        self.ids.next_tag(prefix)
    }

    pub fn reserve_object_result_tag(&mut self, tag: Option<String>) {
        self.reserved_object_result_tag = tag;
    }

    pub fn take_reserved_object_result_tag(&mut self, prefix: &str) -> Option<String> {
        let prefix = format!("{prefix}_");
        self.reserved_object_result_tag
            .as_ref()
            .is_some_and(|tag| tag.starts_with(&prefix))
            .then(|| self.reserved_object_result_tag.take())
            .flatten()
    }
}
