#[path = "chain_splitting/recognition.rs"]
mod recognition;
#[path = "chain_splitting/split_rules.rs"]
mod split_rules;
#[path = "chain_splitting/verbs.rs"]
mod verbs;

pub(crate) use recognition::{
    has_extended_effect_head_tokens, is_token_creation_context_tokens, preserve_and_reason,
    starts_with_inline_token_rules_tail_tokens, starts_with_player_may_tokens,
    strip_leading_instead_tokens,
};
pub(crate) use split_rules::{
    has_authored_comma_then_surface_tokens, has_explicit_comma_then_boundary_tokens,
    split_effect_chain_on_and_tokens, split_segments_on_comma_effect_head_tokens,
    split_segments_on_comma_then_tokens,
};
pub(crate) use verbs::{find_chain_verb_tokens, find_chain_verb_words};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChainVerbKind {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Reveal,
    Look,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Unattach,
    Attach,
    Untap,
    Unlock,
    Scry,
    Discard,
    Transform,
    Convert,
    Flip,
    Roll,
    Regenerate,
    Heal,
    Mill,
    Get,
    Remove,
    Return,
    Exchange,
    Become,
    Switch,
    Skip,
    Surveil,
    Incubate,
    Shuffle,
    Reorder,
    Reverse,
    Pay,
    Take,
    Detain,
    Assign,
    Goad,
    Suspect,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChainVerbMatch {
    pub(crate) kind: ChainVerbKind,
    pub(crate) word_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AndPreservation {
    ColorPair,
    TokenRules,
    AttachmentList,
    SharedPlayerMay,
    PutRemainder,
    StepAndPhase,
    ExchangeZones,
    CardTypeList,
    PowerToughnessAxis,
    QuotedAbility,
    SharedSubject,
}
