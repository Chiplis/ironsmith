//! Queries over actual damage dealt during the current turn.
use crate::tag::TagKeyWalk;
use crate::{ChooseSpec, ObjectFilter, PlayerFilter};

/// Sources are identities or characteristics at the time damage was dealt.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum DamageHistorySources {
    Any,
    Reference(Box<ChooseSpec>),
    Matching(ObjectFilter),
    /// The source's currently attached object, or its exact source LKI if gone.
    SourceAttachedObject,
}

/// Object references name an exact incarnation, including a departed object
/// retained by the resolving ability. Matching filters inspect event receipts.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum DamageHistoryRecipients {
    Any,
    Reference(Box<ChooseSpec>),
    MatchingObjects(ObjectFilter),
    Players(PlayerFilter),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum DamageHistoryReduction {
    Total,
    LargestSourceTotal,
    DistinctSources,
    /// Largest completed amount from one source to one recipient in one
    /// damage occurrence, preserving replacement-fragment coalescing.
    LargestSourceRecipientOccurrence,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct DamageHistoryQuery {
    pub sources: DamageHistorySources,
    pub recipients: DamageHistoryRecipients,
    /// None includes both combat and noncombat damage.
    pub combat: Option<bool>,
    pub reduction: DamageHistoryReduction,
}

impl DamageHistoryQuery {
    pub fn reference_specs(&self) -> impl Iterator<Item = &ChooseSpec> {
        let source = match &self.sources {
            DamageHistorySources::Reference(spec) => Some(spec.as_ref()),
            _ => None,
        };
        let recipient = match &self.recipients {
            DamageHistoryRecipients::Reference(spec) => Some(spec.as_ref()),
            _ => None,
        };
        source.into_iter().chain(recipient)
    }
    pub fn reference_specs_mut(&mut self) -> impl Iterator<Item = &mut ChooseSpec> {
        let source = match &mut self.sources {
            DamageHistorySources::Reference(spec) => Some(spec.as_mut()),
            _ => None,
        };
        let recipient = match &mut self.recipients {
            DamageHistoryRecipients::Reference(spec) => Some(spec.as_mut()),
            _ => None,
        };
        source.into_iter().chain(recipient)
    }
    pub fn object_filters(&self) -> impl Iterator<Item = &ObjectFilter> {
        let source = match &self.sources {
            DamageHistorySources::Matching(filter) => Some(filter),
            _ => None,
        };
        let recipient = match &self.recipients {
            DamageHistoryRecipients::MatchingObjects(filter) => Some(filter),
            _ => None,
        };
        source.into_iter().chain(recipient)
    }
    pub fn object_filters_mut(&mut self) -> impl Iterator<Item = &mut ObjectFilter> {
        let source = match &mut self.sources {
            DamageHistorySources::Matching(filter) => Some(filter),
            _ => None,
        };
        let recipient = match &mut self.recipients {
            DamageHistoryRecipients::MatchingObjects(filter) => Some(filter),
            _ => None,
        };
        source.into_iter().chain(recipient)
    }
    pub fn player_filter(&self) -> Option<&PlayerFilter> {
        match &self.recipients {
            DamageHistoryRecipients::Players(filter) => Some(filter),
            _ => None,
        }
    }
    pub fn player_filter_mut(&mut self) -> Option<&mut PlayerFilter> {
        match &mut self.recipients {
            DamageHistoryRecipients::Players(filter) => Some(filter),
            _ => None,
        }
    }
}

impl DamageHistoryQuery {
    pub fn describe_with_reference(&self, reference: impl Fn(&ChooseSpec) -> String) -> String {
        let damage = match self.combat {
            Some(true) => "combat damage",
            Some(false) => "noncombat damage",
            None => "damage",
        };
        let source = match &self.sources {
            DamageHistorySources::Any => String::new(),
            DamageHistorySources::Reference(spec) => format!(" by {}", reference(spec)),
            DamageHistorySources::Matching(filter) => format!(
                " by {}",
                filter
                    .description()
                    .replace(" you control", " you controlled")
            ),
            DamageHistorySources::SourceAttachedObject => " by the equipped creature".into(),
        };
        let recipient = match &self.recipients {
            DamageHistoryRecipients::Any => String::new(),
            DamageHistoryRecipients::Reference(spec) => format!(" to {}", reference(spec)),
            DamageHistoryRecipients::MatchingObjects(filter) => {
                format!(" to {}", filter.description())
            }
            DamageHistoryRecipients::Players(players) => format!(" to {}", players.description()),
        };
        match self.reduction {
            DamageHistoryReduction::Total => {
                format!("the amount of {damage} dealt{source}{recipient} this turn")
            }
            DamageHistoryReduction::LargestSourceTotal => format!(
                "the greatest total amount of {damage} dealt{source}{recipient} this turn by any one source"
            ),
            DamageHistoryReduction::LargestSourceRecipientOccurrence => {
                let source = if source.is_empty() {
                    " by a source"
                } else {
                    &source
                };
                let recipient = if recipient.is_empty() {
                    " to a permanent or player"
                } else {
                    &recipient
                };
                format!("the greatest amount of {damage} dealt{source}{recipient} this turn")
            }
            DamageHistoryReduction::DistinctSources => {
                let qualifying = source
                    .strip_prefix(" by ")
                    .map(|source| format!(" matching {source}"))
                    .unwrap_or_default();
                format!(
                    "the number of distinct sources{qualifying} that dealt {damage}{recipient} this turn"
                )
            }
        }
    }
}
