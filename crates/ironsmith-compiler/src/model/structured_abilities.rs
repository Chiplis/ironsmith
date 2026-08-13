use crate::effect::Value;
use crate::model::ast::{EffectAst, PredicateAst};
use crate::model::provenance::SemanticProvenance;
use crate::model::symbols::SymbolScopeId;
use crate::model::{
    CompilerActivatedAbilityAst, CompilerStaticAbilityAst, CompilerTotalCost,
    CompilerTriggeredAbilityAst, TargetAst,
};
use crate::target::ObjectFilter;

/// Keyword identity is closed and independent from payload representation.
/// Payload-bearing mechanics use `CompilerKeywordPayloadAst`; callers cannot
/// smuggle an unparsed payload through a display string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CompilerKeywordIdentityAst {
    Flying,
    Menace,
    Banding,
    Hexproof,
    Haste,
    Improvise,
    Convoke,
    Affinity,
    Delve,
    Dredge,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Lifelink,
    Vigilance,
    Trample,
    Reach,
    Defender,
    Decayed,
    Flash,
    Phasing,
    Indestructible,
    Shroud,
    Ward,
    Wither,
    Afflict,
    Afterlife,
    Fabricate,
    Infect,
    Undying,
    Persist,
    Prowess,
    Exalted,
    Cascade,
    Storm,
    Gravestorm,
    Toxic,
    Poisonous,
    BattleCry,
    Dethrone,
    Evolve,
    Ingest,
    Mentor,
    Skulk,
    Training,
    Myriad,
    Riot,
    Unleash,
    Renown,
    Modular,
    Graft,
    Soulbond,
    Soulshift,
    Recover,
    Outlast,
    Scavenge,
    Unearth,
    Embalm,
    Eternalize,
    Emerge,
    Ninjutsu,
    Backup,
    Cipher,
    Dash,
    Blitz,
    Warp,
    Plot,
    Melee,
    Mobilize,
    Suspend,
    Disturb,
    Overload,
    Cleave,
    Awaken,
    Spectacle,
    Foretell,
    Echo,
    CumulativeUpkeep,
    Casualty,
    Demonstrate,
    Conspire,
    Amplify,
    AuraSwap,
    Devour,
    Ravenous,
    Ascend,
    Daybound,
    Nightbound,
    Haunt,
    Provoke,
    Undaunted,
    Enlist,
    Extort,
    Partner,
    StartYourEngines,
    Assist,
    SplitSecond,
    Rebound,
    Sunburst,
    ReadAhead,
    Firebending,
    Fading,
    Vanishing,
    Fear,
    Intimidate,
    Shadow,
    Horsemanship,
    Flanking,
    UmbraArmor,
    Landwalk,
    Bloodthirst,
    Tribute,
    Rampage,
    Bushido,
    Frenzy,
    Changeling,
    Protection,
    Unblockable,
    Devoid,
    Annihilator,
    ForMirrodin,
    LivingWeapon,
    Fuse,
    Prototype,
    Bolster,
    Crew,
    Saddle,
    AdditionalCost,
    AlternativeCost,
    Bestow,
    Bargain,
    Buyback,
    Channel,
    Craft,
    Cycling,
    Equip,
    Escape,
    Flashback,
    Harmonize,
    Kicker,
    Madness,
    Morph,
    Mutate,
    Multikicker,
    Replicate,
    Offspring,
    Reconfigure,
    Reinforce,
    Retrace,
    Squad,
    Splice,
    Transmute,
    Transfigure,
    Entwine,
    Escalate,
    Evoke,
    Epic,
    Gift,
    Exert,
    Exploit,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerKeywordPayloadAst {
    None,
    Value(Value),
    Cost(CompilerTotalCost),
    Filter(ObjectFilter),
    ValueAndCost {
        value: Value,
        cost: CompilerTotalCost,
    },
    FilterAndCost {
        filter: ObjectFilter,
        cost: CompilerTotalCost,
    },
    Effects(Vec<EffectAst>),
    Choice {
        min: Value,
        max: Option<Value>,
        options: Vec<EffectAst>,
    },
    NestedAbilities(Vec<Box<CompilerStructuredAbilityAst>>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerKeywordAbilityAst {
    pub identity: CompilerKeywordIdentityAst,
    pub payload: CompilerKeywordPayloadAst,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ModalSelectionModifierAst {
    None,
    ChooseAllFor(CompilerTotalCost),
    AdditionalModesFor(CompilerTotalCost),
    PointBudget(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerModalSelectionAst {
    pub min: Value,
    pub max: Option<Value>,
    pub same_mode_more_than_once: bool,
    pub mode_must_be_unchosen: bool,
    pub mode_must_be_unchosen_this_turn: bool,
    pub distinct_player_targets_per_mode: bool,
    pub modifier: ModalSelectionModifierAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerModalModeAst {
    pub scope: SymbolScopeId,
    pub label: Option<SemanticProvenance>,
    pub point_cost: Option<u32>,
    pub additional_cost: Option<CompilerTotalCost>,
    pub effects: Vec<EffectAst>,
    pub targets: Vec<TargetAst>,
    pub condition: Option<PredicateAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerModalAbilityAst {
    pub selection: CompilerModalSelectionAst,
    pub prefix_effects: Vec<EffectAst>,
    pub modes: Vec<CompilerModalModeAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LevelBandAst {
    pub min: u32,
    pub max: Option<u32>,
    pub power_toughness: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerLevelBandAst {
    pub band: LevelBandAst,
    pub abilities: Vec<CompilerStructuredAbilityAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerLevelAbilityAst {
    pub bands: Vec<CompilerLevelBandAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerSagaChapterAst {
    pub chapters: Vec<u32>,
    pub effects: Vec<EffectAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerSagaAbilityAst {
    pub read_ahead: bool,
    pub chapters: Vec<CompilerSagaChapterAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerClassLevelAst {
    pub level: u32,
    pub cost: Option<CompilerTotalCost>,
    pub abilities: Vec<CompilerStructuredAbilityAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerClassAbilityAst {
    pub levels: Vec<CompilerClassLevelAst>,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerStructuredAbilityAst {
    Keyword(Box<CompilerKeywordAbilityAst>),
    Static(Box<CompilerStaticAbilityAst>),
    Activated(Box<CompilerActivatedAbilityAst>),
    Triggered(Box<CompilerTriggeredAbilityAst>),
    Modal(Box<CompilerModalAbilityAst>),
    Level(Box<CompilerLevelAbilityAst>),
    Saga(Box<CompilerSagaAbilityAst>),
    Class(Box<CompilerClassAbilityAst>),
}
