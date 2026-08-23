use crate::color::Color;
use crate::costs::PaymentReason;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};
use crate::player::{ManaPool, ManaSpendPolicy};

/// Stable, transaction-local identity for an expanded mana pip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ManaPipId(pub u32);

/// Whether declining a payment is itself a legal choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaymentObligation {
    #[default]
    Required,
    Optional,
}

/// One exact mana-ability choice the player has selected while incrementally
/// constructing a payment.  Unlike `required_sources`, this preserves which
/// ability and mana-output branch was chosen for a multi-ability source. The
/// containing vector is a multiset: repeated entries require repeated legal
/// activations of the same ability.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequiredManaActivation {
    pub source: ObjectId,
    pub ability_index: usize,
    pub color_restriction: Option<Vec<Color>>,
}

/// One exact keyword-payment resource selected during incremental planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequiredAlternativePayment {
    pub source: ObjectId,
    pub kind: ManaPaymentSourceKind,
}

/// User choices that constrain replanning.  These are deliberately expressed
/// as constraints rather than client-authored executable steps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManaPaymentPreferences {
    pub required_sources: Vec<ObjectId>,
    pub required_activations: Vec<RequiredManaActivation>,
    pub required_alternatives: Vec<RequiredAlternativePayment>,
    pub excluded_sources: Vec<ObjectId>,
    pub preserve_sources: Vec<ObjectId>,
    pub prefer_life: bool,
}

impl ManaPaymentPreferences {
    pub fn normalize(&mut self) {
        self.required_sources.sort_unstable();
        self.required_sources.dedup();
        for activation in &mut self.required_activations {
            if let Some(colors) = &mut activation.color_restriction {
                colors.sort_by_key(|color| color_sort_key(*color));
                colors.dedup();
            }
        }
        self.required_activations.sort_by_key(|activation| {
            (
                activation.source,
                activation.ability_index,
                activation
                    .color_restriction
                    .as_deref()
                    .map(color_restriction_sort_key),
            )
        });
        self.required_alternatives.sort_unstable();
        self.required_alternatives.dedup();
        self.excluded_sources.sort_unstable();
        self.excluded_sources.dedup();
        self.preserve_sources.sort_unstable();
        self.preserve_sources.dedup();
    }
}

fn color_sort_key(color: Color) -> u8 {
    match color {
        Color::White => 0,
        Color::Blue => 1,
        Color::Black => 2,
        Color::Red => 3,
        Color::Green => 4,
    }
}

fn color_restriction_sort_key(colors: &[Color]) -> u8 {
    colors.iter().fold(0, |bits, color| {
        bits | match color {
            Color::White => 1 << 0,
            Color::Blue => 1 << 1,
            Color::Black => 1 << 2,
            Color::Red => 1 << 3,
            Color::Green => 1 << 4,
        }
    })
}

/// Everything needed to plan one payment transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaPaymentRequest {
    pub payer: PlayerId,
    pub source: ObjectId,
    pub reason: PaymentReason,
    pub cost: ManaCost,
    pub x_value: u32,
    pub spend_policy: ManaSpendPolicy,
    pub allow_mana_abilities: bool,
    pub allow_life_payment: bool,
    pub allow_black_life: bool,
    pub obligation: PaymentObligation,
    pub preferences: ManaPaymentPreferences,
}

impl ManaPaymentRequest {
    pub fn new(payer: PlayerId, source: ObjectId, reason: PaymentReason, cost: ManaCost) -> Self {
        Self {
            payer,
            source,
            reason,
            cost,
            x_value: 0,
            spend_policy: ManaSpendPolicy::default(),
            allow_mana_abilities: true,
            allow_life_payment: true,
            allow_black_life: false,
            obligation: PaymentObligation::Required,
            preferences: ManaPaymentPreferences::default(),
        }
    }

    pub fn with_x(mut self, x_value: u32) -> Self {
        self.x_value = x_value;
        self
    }

    pub fn with_spend_policy(mut self, spend_policy: ManaSpendPolicy) -> Self {
        self.spend_policy = spend_policy;
        self
    }
}

/// A single mana ability activation selected by a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedManaActivation {
    pub source: ObjectId,
    pub ability_index: usize,
    /// Restriction supplied to existing mana-choice effects during execution.
    pub color_restriction: Option<Vec<Color>>,
    pub expected_mana: ManaPool,
    /// Exact payer pool after this step in the selected plan order.
    pub expected_pool_after: ManaPool,
    /// Number of distinct colored outputs inferred for source-preservation scoring.
    pub flexibility: usize,
    pub undo_safe: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ManaPaymentSourceKind {
    ManaAbility,
    Convoke,
    Improvise,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaPaymentSourceOption {
    pub source: ObjectId,
    pub kinds: Vec<ManaPaymentSourceKind>,
}

/// One exact mana-ability action that an incremental payment client may offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaPaymentActivationOption {
    pub source: ObjectId,
    pub ability_index: usize,
    pub color_restriction: Option<Vec<Color>>,
    pub expected_mana: ManaPool,
    pub repeatable: bool,
}

/// How a displayed pip is expected to be paid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedPipPayment {
    Mana(ManaSymbol),
    Life(u32),
    Convoke(ObjectId),
    Improvise(ObjectId),
    Assist {
        player: PlayerId,
        symbol: ManaSymbol,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPipAllocation {
    pub pip: ManaPipId,
    pub printed_index: usize,
    pub alternatives: Vec<ManaSymbol>,
    pub payment: PlannedPipPayment,
}

/// Reasons that confirmation deserves an explicit warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaPaymentWarning {
    PaysLife(u32),
    UsesNonUndoSafeSource(ObjectId),
    UsesPreservedSource(ObjectId),
    ProducesExcessMana(u32),
    RequiresManualChoices,
}

/// Lexicographic score. Lower values are preferred in every field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManaPaymentScore {
    pub irreversible_cost: u32,
    pub life_paid: u32,
    pub preserved_sources_used: u32,
    pub excess_mana: u32,
    pub flexible_sources_used: u32,
    pub source_count: u32,
}

/// A complete, engine-produced proposal for paying a mana cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaPaymentPlan {
    pub id: u64,
    pub request_hash: u64,
    pub mana_ability_steps: Vec<PlannedManaActivation>,
    pub allocations: Vec<PlannedPipAllocation>,
    /// The portion still paid from mana/life after keyword alternatives.
    pub mana_cost_after_alternatives: ManaCost,
    pub pool_before: ManaPool,
    pub expected_pool_after_activations: ManaPool,
    pub expected_pool_after_payment: ManaPool,
    pub life_to_pay: u32,
    pub score: ManaPaymentScore,
    pub warnings: Vec<ManaPaymentWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaPaymentFailure {
    MissingPlayer,
    NoLegalPlan,
    SearchLimitReached,
    ConflictingPreferences,
    StalePlan,
    ExecutionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaPaymentExecution {
    Paid,
    PendingDecision,
}

/// A client response to an authoritative payment proposal.
///
/// The client never sends executable activation or spending steps. It may
/// accept the selected server plan, ask the server to replan with constraints,
/// or cancel the enclosing cast/activation transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaPaymentResponse {
    Confirm { plan_id: u64, request_hash: u64 },
    Replan { preferences: ManaPaymentPreferences },
    Cancel,
}

/// Resumable engine-side state for one proposed payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingManaPayment {
    pub request: ManaPaymentRequest,
    pub plan: ManaPaymentPlan,
    pub next_activation: usize,
    /// False while the UI is showing the first legal plan and a better
    /// bounded-search result is still being computed.
    pub planning_complete: bool,
}

impl PendingManaPayment {
    pub fn new(request: ManaPaymentRequest, plan: ManaPaymentPlan) -> Self {
        Self {
            request,
            plan,
            next_activation: 0,
            planning_complete: true,
        }
    }

    pub fn provisional(request: ManaPaymentRequest, plan: ManaPaymentPlan) -> Self {
        Self {
            request,
            plan,
            next_activation: 0,
            planning_complete: false,
        }
    }
}
