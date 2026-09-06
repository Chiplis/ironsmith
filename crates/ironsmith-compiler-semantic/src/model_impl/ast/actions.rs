use ironsmith_compiler_ast::TagRef;
use ironsmith_core::tag::TagKeyWalk;

#[path = "actions/counters.rs"]
mod counters;
pub use counters::*;
#[path = "actions/damage_prevention.rs"]
mod damage_prevention;
pub use damage_prevention::*;
#[path = "actions/grants.rs"]
mod grants;
pub use grants::*;
#[path = "actions/game.rs"]
mod game;
pub use game::*;
#[path = "actions/control.rs"]
mod control;
pub use control::*;
#[path = "actions/tokens.rs"]
mod tokens;
pub use tokens::*;
#[path = "actions/stack.rs"]
mod stack;
pub use stack::*;
#[path = "actions/stat_changes.rs"]
mod stat_changes;
pub use stat_changes::*;
#[path = "actions/damage.rs"]
mod damage;
pub use damage::*;
#[path = "actions/choices.rs"]
mod choices;
pub use choices::*;
#[path = "actions/life_resources.rs"]
mod life_resources;
pub use life_resources::*;
#[path = "actions/random.rs"]
mod random;
pub use random::*;
#[path = "actions/reveal_look.rs"]
mod reveal_look;
pub use reveal_look::*;
#[path = "actions/permanent_state.rs"]
mod permanent_state;
pub use permanent_state::*;
#[path = "actions/zone_moves.rs"]
mod zone_moves;
pub use zone_moves::*;
#[path = "actions/keyword_actions.rs"]
mod keyword_actions;
pub use keyword_actions::*;
#[path = "actions/characteristics.rs"]
mod characteristics;
pub use characteristics::*;
#[path = "actions/turn_structure.rs"]
mod turn_structure;
pub use turn_structure::*;
#[path = "actions/exchanges.rs"]
mod exchanges;
pub use exchanges::*;
#[path = "actions/replacements.rs"]
mod replacements;
pub use replacements::*;
#[path = "actions/library.rs"]
mod library;
pub use library::*;
#[path = "actions/mana.rs"]
mod mana;
pub use mana::*;

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum SubjectVerbRoleAst {
    Actor,
    AffectedPlayer,
    Chooser,
    LibraryOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum DieNoun {
    Die,
    Dice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum DieSurface {
    CompactNotation,
    Sided(DieNoun),
}

impl DieSurface {
    pub fn render(self, sides: u32) -> String {
        match self {
            Self::CompactNotation => format!("d{sides}"),
            Self::Sided(DieNoun::Die) => format!("{sides}-sided die"),
            Self::Sided(DieNoun::Dice) => format!("{sides}-sided dice"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TokenCopySacrificeSubjectSurface {
    Token,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TokenCopySacrificeEndStepSurface {
    The,
    Your,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct TokenCopySacrificeAbilitySurface {
    pub end_step: TokenCopySacrificeEndStepSurface,
    pub subject: TokenCopySacrificeSubjectSurface,
}

impl TokenCopySacrificeAbilitySurface {
    pub fn render(self) -> String {
        let end_step = match self.end_step {
            TokenCopySacrificeEndStepSurface::The => "the end step",
            TokenCopySacrificeEndStepSurface::Your => "your end step",
        };
        let subject = match self.subject {
            TokenCopySacrificeSubjectSurface::Token => "this token",
            TokenCopySacrificeSubjectSurface::Permanent => "this permanent",
        };
        format!("At the beginning of {end_step}, sacrifice {subject}.")
    }
}

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SubjectVerbSubjectAst {
    pub role: SubjectVerbRoleAst,
    pub player: PlayerAst,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ReturnAsAuraAst {
    pub attachment_filter: ObjectFilter,
    pub remove_all_abilities: bool,
    pub granted_abilities: Vec<GrantedAbilityAst>,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct EmblemDescriptionAst {
    pub text: String,
    pub abilities: Vec<EmblemAbilityAst>,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum EmblemAbilityAst {
    Static(Vec<StaticAbilityAst>),
    Activated(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        trigger_limit_condition: Option<PredicateAst>,
    },
}

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum SubjectVerbActionAst {
    /// Game: see [`GameActionAst`].
    Game(GameActionAst),
    /// Control: see [`ControlActionAst`].
    Control(ControlActionAst),
    /// Tokens: see [`TokenActionAst`].
    Tokens(TokenActionAst),
    /// Stack: see [`StackActionAst`].
    Stack(StackActionAst),
    /// StatChanges: see [`StatChangeActionAst`].
    StatChanges(StatChangeActionAst),
    /// Damage: see [`DamageActionAst`].
    Damage(DamageActionAst),
    /// Choices: see [`ChoiceActionAst`].
    Choices(ChoiceActionAst),
    /// LifeResources: see [`LifeResourceActionAst`].
    LifeResources(LifeResourceActionAst),
    /// Random: see [`RandomActionAst`].
    Random(RandomActionAst),
    /// RevealLook: see [`RevealLookActionAst`].
    RevealLook(RevealLookActionAst),
    /// PermanentState: see [`PermanentStateActionAst`].
    PermanentState(PermanentStateActionAst),
    /// ZoneMoves: see [`ZoneMoveActionAst`].
    ZoneMoves(ZoneMoveActionAst),
    /// KeywordActions: see [`KeywordActionAst`].
    KeywordActions(KeywordActionAst),
    /// Characteristics: see [`CharacteristicActionAst`].
    Characteristics(CharacteristicActionAst),
    /// TurnStructure: see [`TurnStructureActionAst`].
    TurnStructure(TurnStructureActionAst),
    /// Exchanges: see [`ExchangeActionAst`].
    Exchanges(ExchangeActionAst),
    /// Replacements: see [`ReplacementActionAst`].
    Replacements(ReplacementActionAst),
    /// Library: see [`LibraryActionAst`].
    Library(LibraryActionAst),
    /// Mana: see [`ManaActionAst`].
    Mana(ManaActionAst),
    /// Grants: see [`GrantActionAst`].
    Grants(GrantActionAst),
    /// DamagePrevention: see [`DamagePreventionActionAst`].
    DamagePrevention(DamagePreventionActionAst),
    /// Counters: see [`CounterActionAst`].
    Counters(CounterActionAst),
    ReorderTopPlanarDeck {
        count: u32,
    },
    TargetOnly {
        target: TargetAst,
        explicit_declaration: bool,
    },
    TagMatchingObjects {
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagRef,
        source_tags: Vec<TagRef>,
    },
    Cant {
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        start: crate::effect::RestrictionStart,
        duration_surface: crate::effect::RestrictionDurationSurface,
        condition: Option<PredicateAst>,
    },
    PutSticker {
        target: TargetAst,
        action: crate::events::KeywordActionKind,
    },
}

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SubjectVerbEffectAst {
    pub subject: SubjectVerbSubjectAst,
    pub action: SubjectVerbActionAst,
}

impl std::fmt::Debug for SubjectVerbRoleAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Actor => "Actor",
            Self::AffectedPlayer => "AffectedPlayer",
            Self::Chooser => "Chooser",
            Self::LibraryOwner => "LibraryOwner",
        };
        f.write_str(label)
    }
}

impl std::fmt::Debug for SubjectVerbSubjectAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectVerbSubject")
            .field("role", &self.role)
            .field("player", &self.player)
            .finish()
    }
}

impl std::fmt::Debug for SubjectVerbActionAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LifeResources(LifeResourceActionAst::Draw { count }) => f.debug_tuple("Draw").field(count).finish(),
            Self::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching { tag, filter }) => f
                .debug_struct("DrawForEachTaggedMatching")
                .field("tag", tag)
                .field("filter", filter)
                .finish(),
            Self::LifeResources(LifeResourceActionAst::LoseLife { amount }) => f.debug_tuple("LoseLife").field(amount).finish(),
            Self::LifeResources(LifeResourceActionAst::PayLife { amount }) => f.debug_tuple("PayLife").field(amount).finish(),
            Self::LifeResources(LifeResourceActionAst::GainLife { amount }) => f.debug_tuple("GainLife").field(amount).finish(),
            Self::RevealLook(RevealLookActionAst::RevealHand) => f.write_str("RevealHand"),
            Self::Library(LibraryActionAst::Mill { count }) => f.debug_tuple("Mill").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Scry { count }) => f.debug_tuple("Scry").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Surveil { count }) => f.debug_tuple("Surveil").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Proliferate { count }) => f.debug_tuple("Proliferate").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Investigate { count }) => f.debug_tuple("Investigate").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Incubate { amount, count }) => f
                .debug_struct("Incubate")
                .field("amount", amount)
                .field("count", count)
                .finish(),
            Self::KeywordActions(KeywordActionAst::Learn) => f.write_str("Learn"),
            Self::KeywordActions(KeywordActionAst::EmitKeywordAction { action, amount }) => f
                .debug_struct("EmitKeywordAction")
                .field("action", action)
                .field("amount", amount)
                .finish(),
            Self::ReorderTopPlanarDeck { count } => {
                f.debug_tuple("ReorderTopPlanarDeck").field(count).finish()
            }
            Self::ZoneMoves(ZoneMoveActionAst::ReturnSourceTransformedFromExile) => {
                f.write_str("ReturnSourceTransformedFromExile")
            }
            Self::KeywordActions(KeywordActionAst::Reconfigure { target }) => f.debug_tuple("Reconfigure").field(target).finish(),
            Self::KeywordActions(KeywordActionAst::CumulativeUpkeep { cost }) => {
                f.debug_tuple("CumulativeUpkeep").field(cost).finish()
            }
            Self::KeywordActions(KeywordActionAst::Casualty { power }) => f.debug_tuple("Casualty").field(power).finish(),
            Self::KeywordActions(KeywordActionAst::Amass { subtype, amount }) => f
                .debug_struct("Amass")
                .field("subtype", subtype)
                .field("amount", amount)
                .finish(),
            Self::KeywordActions(KeywordActionAst::Bolster { amount }) => f.debug_tuple("Bolster").field(amount).finish(),
            Self::KeywordActions(KeywordActionAst::Support { amount }) => f.debug_tuple("Support").field(amount).finish(),
            Self::KeywordActions(KeywordActionAst::Adapt { amount }) => f.debug_tuple("Adapt").field(amount).finish(),
            Self::KeywordActions(KeywordActionAst::Monstrosity { amount }) => f.debug_tuple("Monstrosity").field(amount).finish(),
            Self::KeywordActions(KeywordActionAst::Discover { count }) => f.debug_tuple("Discover").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Fateseal { count }) => f.debug_tuple("Fateseal").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Populate { count, .. }) => f.debug_tuple("Populate").field(count).finish(),
            Self::KeywordActions(KeywordActionAst::Explore { target }) => f.debug_tuple("Explore").field(target).finish(),
            Self::KeywordActions(KeywordActionAst::Endure { target, amount }) => f
                .debug_struct("Endure")
                .field("target", target)
                .field("amount", amount)
                .finish(),
            Self::KeywordActions(KeywordActionAst::Exploit) => f.write_str("Exploit"),
            Self::KeywordActions(KeywordActionAst::Connive { target, count }) => f
                .debug_struct("Connive")
                .field("target", target)
                .field("count", count)
                .finish(),
            Self::KeywordActions(KeywordActionAst::ConniveIterated) => f.write_str("ConniveIterated"),
            Self::KeywordActions(KeywordActionAst::OpenAttraction { reminder }) => f
                .debug_struct("OpenAttraction")
                .field("reminder", reminder)
                .finish(),
            Self::Library(LibraryActionAst::ManifestTopCardOfLibrary) => f.write_str("ManifestTopCardOfLibrary"),
            Self::Library(LibraryActionAst::CloakTopCardOfLibrary) => f.write_str("CloakTopCardOfLibrary"),
            Self::KeywordActions(KeywordActionAst::ManifestCardFromHand) => f.write_str("ManifestCardFromHand"),
            Self::KeywordActions(KeywordActionAst::ManifestDread) => f.write_str("ManifestDread"),
            Self::KeywordActions(KeywordActionAst::Earthbend { counters }) => f.debug_tuple("Earthbend").field(counters).finish(),
            Self::KeywordActions(KeywordActionAst::Behold { subtype, count }) => f
                .debug_struct("Behold")
                .field("subtype", subtype)
                .field("count", count)
                .finish(),
            Self::KeywordActions(KeywordActionAst::Fight {
                creature1,
                creature2,
                mutual_surface,
            }) => f
                .debug_struct("Fight")
                .field("creature1", creature1)
                .field("creature2", creature2)
                .field("mutual_surface", mutual_surface)
                .finish(),
            Self::KeywordActions(KeywordActionAst::FightIterated { creature2 }) => {
                f.debug_tuple("FightIterated").field(creature2).finish()
            }
            Self::KeywordActions(KeywordActionAst::Clash { opponent }) => f.debug_tuple("Clash").field(opponent).finish(),
            Self::Random(RandomActionAst::FlipCoin) => f.write_str("FlipCoin"),
            Self::Random(RandomActionAst::FlipCoinFaceOnly) => f.write_str("FlipCoinFaceOnly"),
            Self::Random(RandomActionAst::RollDie { sides, surface }) => {
                if let Some(surface) = surface {
                    f.debug_struct("RollDie")
                        .field("sides", sides)
                        .field("surface", surface)
                        .finish()
                } else {
                    f.debug_tuple("RollDie").field(sides).finish()
                }
            }
            Self::Random(RandomActionAst::RollDiceChooseResult {
                count,
                sides,
                surface,
            }) => f
                .debug_struct("RollDiceChooseResult")
                .field("count", count)
                .field("sides", sides)
                .field("surface", surface)
                .finish(),
            Self::Library(LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary) => {
                f.write_str("ShuffleHandAndGraveyardIntoLibrary")
            }
            Self::Library(LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary) => {
                f.write_str("ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary")
            }
            Self::Library(LibraryActionAst::ShuffleGraveyardIntoLibrary {
                explicit_all_cards_from,
            }) => f
                .debug_struct("ShuffleGraveyardIntoLibrary")
                .field("explicit_all_cards_from", explicit_all_cards_from)
                .finish(),
            Self::Library(LibraryActionAst::ReorderGraveyard) => f.write_str("ReorderGraveyard"),
            Self::Choices(ChoiceActionAst::ChooseColor) => f.write_str("ChooseColor"),
            Self::Choices(ChoiceActionAst::ChooseCardType { options }) => {
                f.debug_tuple("ChooseCardType").field(options).finish()
            }
            Self::Choices(ChoiceActionAst::ChooseNamedOption { options }) => {
                f.debug_tuple("ChooseNamedOption").field(options).finish()
            }
            Self::Choices(ChoiceActionAst::ChooseCreatureType {
                excluded_subtypes,
                family,
            }) => f
                .debug_struct("ChooseCreatureType")
                .field("excluded_subtypes", excluded_subtypes)
                .field("family", family)
                .finish(),
            Self::Choices(ChoiceActionAst::ChooseLandType { exclude_basic }) => f
                .debug_struct("ChooseLandType")
                .field("exclude_basic", exclude_basic)
                .finish(),
            Self::Choices(ChoiceActionAst::ChooseCardName { filter, tag }) => f
                .debug_struct("ChooseCardName")
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::Choices(ChoiceActionAst::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            }) => f
                .debug_struct("ChoosePlayer")
                .field("filter", filter)
                .field("tag", tag)
                .field("random", random)
                .field("exclude_previous_choices", exclude_previous_choices)
                .finish(),
            Self::LifeResources(LifeResourceActionAst::NoteLifeTotal) => f.write_str("NoteLifeTotal"),
            Self::Choices(ChoiceActionAst::ChooseSpellCastHistory {
                cast_by,
                filter,
                tag,
            }) => f
                .debug_struct("ChooseSpellCastHistory")
                .field("cast_by", cast_by)
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::Mana(ManaActionAst::AddMana { mana }) => f.debug_tuple("AddMana").field(mana).finish(),
            Self::Mana(ManaActionAst::AddManaScaled { mana, amount }) => f
                .debug_struct("AddManaScaled")
                .field("mana", mana)
                .field("amount", amount)
                .finish(),
            Self::Mana(ManaActionAst::AddManaAnyColor {
                amount,
                available_colors,
                distinct_colors,
            }) => f
                .debug_struct("AddManaAnyColor")
                .field("amount", amount)
                .field("available_colors", available_colors)
                .field("distinct_colors", distinct_colors)
                .finish(),
            Self::Mana(ManaActionAst::AddManaAnyOneColor { amount }) => {
                f.debug_tuple("AddManaAnyOneColor").field(amount).finish()
            }
            Self::Mana(ManaActionAst::AddManaChosenColor {
                amount,
                fixed_option,
            }) => f
                .debug_struct("AddManaChosenColor")
                .field("amount", amount)
                .field("fixed_option", fixed_option)
                .finish(),
            Self::Mana(ManaActionAst::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
                mana_type_source,
            }) => f
                .debug_struct("AddManaFromLandCouldProduce")
                .field("amount", amount)
                .field("land_filter", land_filter)
                .field("allow_colorless", allow_colorless)
                .field("same_type", same_type)
                .field("mana_type_source", mana_type_source)
                .finish(),
            Self::Mana(ManaActionAst::AddManaColorsAmong { filter }) => f
                .debug_struct("AddManaColorsAmong")
                .field("filter", filter)
                .finish(),
            Self::Mana(ManaActionAst::AddOneManaAnyColorAmong {
                filter,
                choose_color_of_object_surface,
            }) => f
                .debug_struct("AddOneManaAnyColorAmong")
                .field("filter", filter)
                .field(
                    "choose_color_of_object_surface",
                    choose_color_of_object_surface,
                )
                .finish(),
            Self::Mana(ManaActionAst::AddManaCommanderIdentity { amount }) => f
                .debug_tuple("AddManaCommanderIdentity")
                .field(amount)
                .finish(),
            Self::Exchanges(ExchangeActionAst::ExchangeLifeTotals { player2 }) => {
                f.debug_tuple("ExchangeLifeTotals").field(player2).finish()
            }
            Self::Exchanges(ExchangeActionAst::ExchangeTextBoxes { target }) => {
                f.debug_tuple("ExchangeTextBoxes").field(target).finish()
            }
            Self::Exchanges(ExchangeActionAst::ExchangeZones { zone1, zone2 }) => f
                .debug_struct("ExchangeZones")
                .field("zone1", zone1)
                .field("zone2", zone2)
                .finish(),
            Self::Library(LibraryActionAst::PutRestOnBottomOfLibrary) => f.write_str("PutRestOnBottomOfLibrary"),
            Self::Mana(ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn) => {
                f.write_str("DontLoseThisManaAsStepsAndPhasesEndThisTurn")
            }
            Self::Exchanges(ExchangeActionAst::ExchangeValues {
                left,
                right,
                duration,
            }) => f
                .debug_struct("ExchangeValues")
                .field("left", left)
                .field("right", right)
                .field("duration", duration)
                .finish(),
            Self::Exchanges(ExchangeActionAst::ExchangeControl {
                filter,
                count,
                shared_type,
            }) => f
                .debug_struct("ExchangeControl")
                .field("filter", filter)
                .field("count", count)
                .field("shared_type", shared_type)
                .finish(),
            Self::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            }) => f
                .debug_struct("ExchangeControlHeterogeneous")
                .field("permanent1", permanent1)
                .field("permanent2", permanent2)
                .field("shared_type", shared_type)
                .finish(),
            Self::Control(ControlActionAst::Attach { object, target }) => f
                .debug_struct("Attach")
                .field("object", object)
                .field("target", target)
                .finish(),
            Self::Control(ControlActionAst::Unattach { object }) => {
                f.debug_struct("Unattach").field("object", object).finish()
            }
            Self::Control(ControlActionAst::Enchant { filter }) => f.debug_tuple("Enchant").field(filter).finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { target }) => f
                .debug_tuple("ExileWhenSourceLeaves")
                .field(target)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { target }) => f
                .debug_tuple("SacrificeSourceWhenLeaves")
                .field(target)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement,
                duration,
                optional,
                choice_description,
                counters,
                linked_exile_follow_up,
            }) => f
                .debug_struct("RegisterZoneReplacement")
                .field("target", target)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("library_placement", library_placement)
                .field("duration", duration)
                .field("optional", optional)
                .field("choice_description", choice_description)
                .field("counters", counters)
                .field("linked_exile_follow_up", linked_exile_follow_up)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
                cause_policy,
                link_exiled_to_source,
            }) => f
                .debug_struct("RegisterFutureZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .field("cause_policy", cause_policy)
                .field("link_exiled_to_source", link_exiled_to_source)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterDrawReplacement {
                player,
                replacement_effects,
                duration,
            }) => f
                .debug_struct("RegisterDrawReplacement")
                .field("player", player)
                .field("replacement_effects", replacement_effects)
                .field("duration", duration)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterManaReplacement {
                source_filter,
                replacement_mana,
                mode,
            }) => f
                .debug_struct("RegisterManaReplacement")
                .field("source_filter", source_filter)
                .field("replacement_mana", replacement_mana)
                .field("mode", mode)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterDamagedBySourceZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            }) => f
                .debug_struct("RegisterDamagedBySourceZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterEnterUnderControlReplacement { filter, duration }) => f
                .debug_struct("RegisterEnterUnderControlReplacement")
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterEnterTappedReplacement { filter, duration }) => f
                .debug_struct("RegisterEnterTappedReplacement")
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters {
                filter,
                counter_type,
                count,
            }) => f
                .debug_struct("RegisterNextBatchEnterWithCounters")
                .field("filter", filter)
                .field("counter_type", counter_type)
                .field("count", count)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn) => f.write_str("ExileInsteadOfGraveyardThisTurn"),
            Self::Control(ControlActionAst::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
                this_combat,
            }) => f
                .debug_struct("ControlCombatChoicesThisTurn")
                .field("attackers", attackers)
                .field("blockers", blockers)
                .field("this_combat", this_combat)
                .finish(),
            Self::Control(ControlActionAst::GainControl {
                target,
                duration,
                condition,
                controller_reference,
                source_reference_surface,
            }) => f
                .debug_struct("GainControl")
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .field("controller_reference", controller_reference)
                .field("source_reference_surface", source_reference_surface)
                .finish(),
            Self::RevealLook(RevealLookActionAst::RevealTop) => f.write_str("RevealTop"),
            Self::Library(LibraryActionAst::ExileTopOfLibrary {
                count,
                surface,
                tags,
                accumulated_tags,
                face_down,
            }) => f
                .debug_struct("ExileTopOfLibrary")
                .field("count", count)
                .field("surface", surface)
                .field("tags", tags)
                .field("accumulated_tags", accumulated_tags)
                .field("face_down", face_down)
                .finish(),
            Self::RevealLook(RevealLookActionAst::RevealTagged { tag }) => f.debug_tuple("RevealTagged").field(tag).finish(),
            Self::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak,
                shuffle_before,
            }) => f
                .debug_struct("PutOntoBattlefield")
                .field("target", target)
                .field("tapped", tapped)
                .field("controller", controller)
                .field("cloak", cloak)
                .field("shuffle_before", shuffle_before)
                .finish(),
            Self::RevealLook(RevealLookActionAst::RevealCardsFromHand {
                count,
                count_value,
                tag,
            }) => f
                .debug_struct("RevealCardsFromHand")
                .field("count", count)
                .field("count_value", count_value)
                .field("tag", tag)
                .finish(),
            Self::RevealLook(RevealLookActionAst::LookAtTopCards { count, tag, reveal }) => f
                .debug_struct("LookAtTopCards")
                .field("count", count)
                .field("tag", tag)
                .field("reveal", reveal)
                .finish(),
            Self::RevealLook(RevealLookActionAst::LookAtObjects { filter }) => f
                .debug_struct("LookAtObjects")
                .field("filter", filter)
                .finish(),
            Self::RevealLook(RevealLookActionAst::LookAtTarget { target }) => f.debug_tuple("LookAtTarget").field(target).finish(),
            Self::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, zone }) => f
                .debug_struct("MayMoveToZone")
                .field("target", target)
                .field("zone", zone)
                .finish(),
            Self::TurnStructure(TurnStructureActionAst::AdditionalLandPlays { count, duration }) => f
                .debug_struct("AdditionalLandPlays")
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::Game(GameActionAst::ExtraTurnAfterTurn { anchor }) => {
                f.debug_tuple("ExtraTurnAfterTurn").field(anchor).finish()
            }
            Self::Library(LibraryActionAst::ReorderTopOfLibrary { tag }) => {
                f.debug_tuple("ReorderTopOfLibrary").field(tag).finish()
            }
            Self::Mana(ManaActionAst::AddManaImprintedColors) => f.write_str("AddManaImprintedColors"),
            Self::Library(LibraryActionAst::ShuffleLibrary) => f.write_str("ShuffleLibrary"),
            Self::Library(LibraryActionAst::ShuffleObjectsIntoLibrary {
                target,
                all,
                owner_library_destination,
                possessive_owner_subject,
            }) => f
                .debug_struct("ShuffleObjectsIntoLibrary")
                .field("target", target)
                .field("all", all)
                .field("owner_library_destination", owner_library_destination)
                .field("possessive_owner_subject", possessive_owner_subject)
                .finish(),
            Self::Grants(GrantActionAst::GrantProtectionChoice {
                target,
                chooser,
                allow_colorless,
                allow_artifacts,
                choose_card_type,
            }) => f
                .debug_struct("GrantProtectionChoice")
                .field("target", target)
                .field("chooser", chooser)
                .field("allow_colorless", allow_colorless)
                .field("allow_artifacts", allow_artifacts)
                .field("choose_card_type", choose_card_type)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamage { duration }) => f
                .debug_struct("PreventAllCombatDamage")
                .field("duration", duration)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { source, duration }) => f
                .debug_struct("AssignNoCombatDamage")
                .field("source", source)
                .field("duration", duration)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource {
                duration,
                source,
                source_would_deal_surface,
            }) => f
                .debug_struct("PreventAllCombatDamageFromSource")
                .field("duration", duration)
                .field("source", source)
                .field("source_would_deal_surface", source_would_deal_surface)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target,
            }) => f
                .debug_struct("PreventAllCombatDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
                .field("excluded_source_target", excluded_source_target)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToPlayers { duration }) => f
                .debug_struct("PreventAllCombatDamageToPlayers")
                .field("duration", duration)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToYou { duration }) => f
                .debug_struct("PreventAllCombatDamageToYou")
                .field("duration", duration)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventNextTimeDamage {
                source,
                target,
                reflect_damage_to_source_controller,
                follow_up_effects,
            }) => f
                .debug_struct("PreventNextTimeDamage")
                .field("source", source)
                .field("target", target)
                .field(
                    "reflect_damage_to_source_controller",
                    reflect_damage_to_source_controller,
                )
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::ReplaceNextDamageToTarget {
                target,
                damage_target_tag,
                replacement_effects,
            }) => f
                .debug_struct("ReplaceNextDamageToTarget")
                .field("target", target)
                .field("damage_target_tag", damage_target_tag)
                .field("replacement_effects", replacement_effects)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventDamage {
                amount,
                target,
                duration,
                follow_up_effects,
                ..
            }) => f
                .debug_struct("PreventDamage")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice,
                source_choice_shares_activation_mana_color,
                source_target,
            }) => f
                .debug_struct("PreventAllDamageToTarget")
                .field("target", target)
                .field("duration", duration)
                .field("source_of_your_choice", source_of_your_choice)
                .field(
                    "source_choice_shares_activation_mana_color",
                    source_choice_shares_activation_mana_color,
                )
                .field("source_target", source_target)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                target,
                duration,
                source_filter,
            }) => f
                .debug_struct("PreventAllDamageToTargetFromSourceFilter")
                .field("target", target)
                .field("duration", duration)
                .field("source_filter", source_filter)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                duration,
                source_filter,
            }) => f
                .debug_struct("PreventAllDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                amount,
                target,
                duration,
                counter_type,
            }) => f
                .debug_struct("PreventDamageToTargetPutCounters")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .field("counter_type", counter_type)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::PreventDamageEach {
                amount,
                filter,
                duration,
            }) => f
                .debug_struct("PreventDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::Stack(StackActionAst::CopySpell {
                target,
                target_reference_kind,
                target_reference_pronoun,
                all_matches,
                count,
                count_surface,
                player,
                may_choose_new_targets,
                choose_new_target_singular,
                removed_supertypes,
                set_colors,
                added_card_types,
                added_subtypes,
                set_base_power_toughness,
            }) => f
                .debug_struct("CopySpell")
                .field("target", target)
                .field("target_reference_kind", target_reference_kind)
                .field("target_reference_pronoun", target_reference_pronoun)
                .field("all_matches", all_matches)
                .field("count", count)
                .field("count_surface", count_surface)
                .field("player", player)
                .field("may_choose_new_targets", may_choose_new_targets)
                .field("choose_new_target_singular", choose_new_target_singular)
                .field("removed_supertypes", removed_supertypes)
                .field("set_colors", set_colors)
                .field("added_card_types", added_card_types)
                .field("added_subtypes", added_subtypes)
                .field("set_base_power_toughness", set_base_power_toughness)
                .finish(),
            Self::Stack(StackActionAst::CopySpellForEachTarget {
                target,
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
                removed_supertypes,
            }) => f
                .debug_struct("CopySpellForEachTarget")
                .field("target", target)
                .field("object_filter", object_filter)
                .field("player_filter", player_filter)
                .field("player", player)
                .field("exclude_current_targets", exclude_current_targets)
                .field("removed_supertypes", removed_supertypes)
                .finish(),
            Self::Stack(StackActionAst::ScaleXValue { target, multiplier }) => f
                .debug_struct("ScaleXValue")
                .field("target", target)
                .field("multiplier", multiplier)
                .finish(),
            Self::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
                surface,
            }) => f
                .debug_struct("PutTaggedRemainderOnBottomOfLibrary")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("order", order)
                .field("player", player)
                .field("surface", surface)
                .finish(),
            Self::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag,
                keep_tagged,
                zone,
                surface,
            }) => f
                .debug_struct("PutTaggedRemainderInZone")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("zone", zone)
                .field("surface", surface)
                .finish(),
            Self::Stack(StackActionAst::CastTagged {
                tag,
                player,
                allow_land,
                as_copy,
                copy_cast_reminder_surface,
                copy_instruction_surface,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
            }) => f
                .debug_struct("CastTagged")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("as_copy", as_copy)
                .field("copy_cast_reminder_surface", copy_cast_reminder_surface)
                .field("copy_instruction_surface", copy_instruction_surface)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("additional_mana_cost", additional_mana_cost)
                .field("cost_reduction", cost_reduction)
                .field("mana_spend_mode", mana_spend_mode)
                .finish(),
            Self::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library,
                free_cast_from_current_zone,
                until_source_exiles_another,
                max_plays,
                surface,
            }) => f
                .debug_struct("GrantPlayTaggedUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("while_on_top_of_library", while_on_top_of_library)
                .field("free_cast_from_current_zone", free_cast_from_current_zone)
                .field("until_source_exiles_another", until_source_exiles_another)
                .field("max_plays", max_plays)
                .field("surface", surface)
                .finish(),
            Self::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            }) => f
                .debug_struct("GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .finish(),
            Self::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step,
                max_plays,
            }) => f
                .debug_struct("GrantPlayTaggedUntilYourNextTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("until_next_end_step", until_next_end_step)
                .field("max_plays", max_plays)
                .finish(),
            Self::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source,
                spell_cost_increase,
                lands_enter_tapped,
            }) => f
                .debug_struct("GrantPlayTaggedForAsLongAsExiled")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("filter", filter)
                .field(
                    "during_turns_counter_put_on_source",
                    during_turns_counter_put_on_source,
                )
                .field("spell_cost_increase", spell_cost_increase)
                .field("lands_enter_tapped", lands_enter_tapped)
                .finish(),
            Self::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                surface,
            }) => f
                .debug_struct("GrantPlayTaggedForAsLongAsYouControlSource")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("surface", surface)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                target,
                target_reference_surface,
                from_graveyard_or_exile,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
                top_only,
            }) => f
                .debug_struct("ReturnToBattlefield")
                .field("target", target)
                .field("target_reference_surface", target_reference_surface)
                .field("from_graveyard_or_exile", from_graveyard_or_exile)
                .field("tapped", tapped)
                .field("transformed", transformed)
                .field("converted", converted)
                .field("controller", controller)
                .field("count_value", count_value)
                .field("as_aura", as_aura)
                .field("top_only", top_only)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface,
            }) => f
                .debug_struct("ReturnAllToBattlefield")
                .field("filter", filter)
                .field("tapped", tapped)
                .field("face_down", face_down)
                .field("controller", controller)
                .field("verb_surface", verb_surface)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                target,
                duration,
                leave_watcher,
                face_down,
                all,
                explicit_return_surface,
            }) => f
                .debug_struct("ExileUntilSourceLeaves")
                .field("target", target)
                .field("duration", duration)
                .field("leave_watcher", leave_watcher)
                .field("face_down", face_down)
                .field("all", all)
                .field("explicit_return_surface", explicit_return_surface)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target,
                source_top_only,
                zone,
                to_top,
                library_order,
                library_order_chooser,
                verb_surface,
                target_plural_surface,
                target_reference_surface,
                destination_player_surface,
                destination_player_reference_surface,
                exiled_with_source_surface,
                battlefield_controller,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_face_down,
                battlefield_transformed,
                attached_to,
                all,
            }) => f
                .debug_struct("MoveToZone")
                .field("target", target)
                .field("source_top_only", source_top_only)
                .field("zone", zone)
                .field("to_top", to_top)
                .field("library_order", library_order)
                .field("library_order_chooser", library_order_chooser)
                .field("verb_surface", verb_surface)
                .field("target_plural_surface", target_plural_surface)
                .field("target_reference_surface", target_reference_surface)
                .field("destination_player_surface", destination_player_surface)
                .field(
                    "destination_player_reference_surface",
                    destination_player_reference_surface,
                )
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .field("battlefield_controller", battlefield_controller)
                .field("battlefield_tapped", battlefield_tapped)
                .field("battlefield_attacking", battlefield_attacking)
                .field(
                    "battlefield_attack_target_player_or_planeswalker_controlled_by",
                    battlefield_attack_target_player_or_planeswalker_controlled_by,
                )
                .field("battlefield_face_down", battlefield_face_down)
                .field("battlefield_transformed", battlefield_transformed)
                .field("attached_to", attached_to)
                .field("all", all)
                .finish(),
            Self::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target }) => f
                .debug_struct("MoveToLibraryTopOrBottomChoice")
                .field("target", target)
                .finish(),
            Self::TargetOnly {
                target,
                explicit_declaration,
            } => f
                .debug_struct("TargetOnly")
                .field("target", target)
                .field("explicit_declaration", explicit_declaration)
                .finish(),
            Self::TagMatchingObjects {
                filter,
                zones,
                tag,
                source_tags,
            } => {
                let mut debug = f.debug_struct("TagMatchingObjects");
                debug
                    .field("filter", filter)
                    .field("zones", zones)
                    .field("tag", tag);
                if !source_tags.is_empty() {
                    debug.field("source_tags", source_tags);
                }
                debug.finish()
            }
            Self::StatChanges(StatChangeActionAst::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
                set_quantifier_surface,
            }) => f
                .debug_struct("Pump")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                set_quantifier_surface,
            }) => f
                .debug_struct("SetBasePowerToughness")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                name_override, add_supertypes, remove_all_abilities,
                power,
                toughness,
                target,
                card_types,
                subtypes,
                subtype_families,
                colors,
                abilities,
                granted_abilities,
                preserve_other_types,
                type_retention_surface,
                animation_pt_surface,
                animation_duration_surface,
                set_quantifier_surface,
                duration,
            }) => f
                .debug_struct("BecomeBasePtCreature")
                .field("name_override", name_override)
                .field("add_supertypes", add_supertypes)
                .field("remove_all_abilities", remove_all_abilities)
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("card_types", card_types)
                .field("subtypes", subtypes)
                .field("subtype_families", subtype_families)
                .field("colors", colors)
                .field("abilities", abilities)
                .field("granted_abilities", granted_abilities)
                .field("preserve_other_types", preserve_other_types)
                .field("type_retention_surface", type_retention_surface)
                .field("animation_pt_surface", animation_pt_surface)
                .field("animation_duration_surface", animation_duration_surface)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::SetBasePower {
                power,
                target,
                duration,
            }) => f
                .debug_struct("SetBasePower")
                .field("power", power)
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::PumpForEach {
                power_per,
                toughness_per,
                target,
                count,
                duration,
            }) => f
                .debug_struct("PumpForEach")
                .field("power_per", power_per)
                .field("toughness_per", toughness_per)
                .field("target", target)
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::PumpAll {
                filter,
                power,
                toughness,
                duration,
                set_quantifier_surface,
            }) => f
                .debug_struct("PumpAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("duration", duration)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::StatChanges(StatChangeActionAst::PumpByLastEffect {
                power,
                toughness,
                target,
                duration,
                includes_this_way,
            }) => f
                .debug_struct("PumpByLastEffect")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("includes_this_way", includes_this_way)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::AddCardTypes {
                target,
                card_types,
                duration,
            }) => f
                .debug_struct("AddCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::SetCardTypes {
                target,
                card_types,
                duration,
            }) => f
                .debug_struct("SetCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::RemoveCardTypes {
                target,
                card_types,
                duration,
            }) => f
                .debug_struct("RemoveCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::AddSubtypes {
                target,
                subtypes,
                duration,
            }) => f
                .debug_struct("AddSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::RemoveSubtypes {
                target,
                subtypes,
                duration,
            }) => f
                .debug_struct("RemoveSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::SetCreatureSubtypes {
                target,
                subtypes,
                duration,
            }) => f
                .debug_struct("SetCreatureSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { target }) => f
                .debug_struct("BecomeSaddledUntilEndOfTurn")
                .field("target", target)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::AddColors {
                target,
                colors,
                duration,
            }) => f
                .debug_struct("AddColors")
                .field("target", target)
                .field("colors", colors)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily {
                target,
                family,
                duration,
            }) => f
                .debug_struct("AddAllSubtypesOfFamily")
                .field("target", target)
                .field("family", family)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily {
                target,
                family,
                duration,
            }) => f
                .debug_struct("RemoveAllSubtypesOfFamily")
                .field("target", target)
                .field("family", family)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                target,
                attachment_filter,
                granted_abilities,
                duration,
            }) => f
                .debug_struct("BecomeAuraEnchantment")
                .field("target", target)
                .field("attachment_filter", attachment_filter)
                .field("granted_abilities", granted_abilities)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeBasicLandType {
                target,
                subtype,
                duration,
            }) => f
                .debug_struct("BecomeBasicLandType")
                .field("target", target)
                .field("subtype", subtype)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::SetColors {
                target,
                colors,
                duration,
            }) => f
                .debug_struct("SetColors")
                .field("target", target)
                .field("colors", colors)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::MakeColorless { target, duration }) => f
                .debug_struct("MakeColorless")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { target, duration }) => f
                .debug_struct("BecomeBasicLandTypeChoice")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice {
                target,
                duration,
                excluded_subtypes,
            }) => f
                .debug_struct("BecomeCreatureTypeChoice")
                .field("target", target)
                .field("duration", duration)
                .field("excluded_subtypes", excluded_subtypes)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                target,
                duration,
                allow_multiple,
            }) => f
                .debug_struct("BecomeColorChoice")
                .field("target", target)
                .field("duration", duration)
                .field("allow_multiple", allow_multiple)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeCopy {
                target,
                source,
                duration,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                remove_supertypes,
                add_colors,
                add_card_types,
                set_card_types,
                add_subtypes,
                set_subtypes,
                granted_abilities,
                set_base_power_toughness,
                copy_exception_surface,
            }) => f
                .debug_struct("BecomeCopy")
                .field("target", target)
                .field("source", source)
                .field("duration", duration)
                .field("preserve_source_abilities", preserve_source_abilities)
                .field("name_override", name_override)
                .field("name_override_surface", name_override_surface)
                .field("add_supertypes", add_supertypes)
                .field("remove_supertypes", remove_supertypes)
                .field("add_colors", add_colors)
                .field("add_card_types", add_card_types)
                .field("set_card_types", set_card_types)
                .field("add_subtypes", add_subtypes)
                .field("set_subtypes", set_subtypes)
                .field("granted_abilities", granted_abilities)
                .field("set_base_power_toughness", set_base_power_toughness)
                .field("copy_exception_surface", copy_exception_surface)
                .finish(),
            Self::Grants(GrantActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
                lock_filter_at_resolution,
            }) => f
                .debug_struct("GrantAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("lock_filter_at_resolution", lock_filter_at_resolution)
                .finish(),
            Self::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
            }) => f
                .debug_struct("RemoveAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::Grants(GrantActionAst::GrantAbilitiesChoiceAll {
                filter,
                abilities,
                duration,
            }) => f
                .debug_struct("GrantAbilitiesChoiceAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::Grants(GrantActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
            }) => f
                .debug_struct("GrantAbilitiesToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::Grants(GrantActionAst::GrantToTarget {
                target,
                grantable,
                duration,
            }) => f
                .debug_struct("GrantToTarget")
                .field("target", target)
                .field("grantable", grantable)
                .field("duration", duration)
                .finish(),
            Self::Grants(GrantActionAst::GrantBySpec {
                spec,
                player,
                duration,
            }) => f
                .debug_struct("GrantBySpec")
                .field("spec", spec)
                .field("player", player)
                .field("duration", duration)
                .finish(),
            Self::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            }) => f
                .debug_struct("RemoveAbilitiesFromTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            }) => f
                .debug_struct("GrantAbilitiesChoiceToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::Library(LibraryActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed,
                all_tag,
                match_tag,
            }) => f
                .debug_struct("ConsultTopOfLibrary")
                .field("player", player)
                .field("mode", mode)
                .field("filter", filter)
                .field("stop_rule", stop_rule)
                .field("max_exposed", max_exposed)
                .field("all_tag", all_tag)
                .field("match_tag", match_tag)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
                filter,
                search_zones,
                destination,
                chooser,
                player,
                search_mode,
                reveal,
                reveal_reference_surface,
                shuffle,
                count,
                count_value,
                library_position_from_top,
                result_reference_surface,
                search_top_in_any_order_surface,
                tapped,
                enters_with_counters,
                enters_under_your_control,
            }) => f
                .debug_struct("SearchLibrary")
                .field("enters_under_your_control", enters_under_your_control)
                .field("filter", filter)
                .field("search_zones", search_zones)
                .field("destination", destination)
                .field("chooser", chooser)
                .field("player", player)
                .field("search_mode", search_mode)
                .field("reveal", reveal)
                .field("reveal_reference_surface", reveal_reference_surface)
                .field("shuffle", shuffle)
                .field("count", count)
                .field("count_value", count_value)
                .field("library_position_from_top", library_position_from_top)
                .field("result_reference_surface", result_reference_surface)
                .field(
                    "search_top_in_any_order_surface",
                    search_top_in_any_order_surface,
                )
                .field("tapped", tapped)
                .field("enters_with_counters", enters_with_counters)
                .finish(),
            Self::Cant {
                restriction,
                duration,
                start,
                duration_surface,
                condition,
            } => f
                .debug_struct("Cant")
                .field("restriction", restriction)
                .field("duration", duration)
                .field("start", start)
                .field("duration_surface", duration_surface)
                .field("condition", condition)
                .finish(),
            Self::Tokens(TokenActionAst::CreateTokenCopy { .. }) => f.write_str("CreateTokenCopy"),
            Self::Tokens(TokenActionAst::CreateTokenCopyFromSource { .. }) => f.write_str("CreateTokenCopyFromSource"),
            Self::Tokens(TokenActionAst::CreateTokenWithMods {
                name,
                count,
                player,
                ..
            }) => f
                .debug_struct("CreateTokenWithMods")
                .field("name", name)
                .field("count", count)
                .field("player", player)
                .finish(),
            Self::Tokens(TokenActionAst::CreateTokenChoice { options }) => {
                let mut builder = f.debug_struct("CreateTokenChoice");
                for (display, _) in options {
                    builder.field("option", display);
                }
                builder.finish()
            }
            Self::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target,
                destination,
                destination_target,
            }) => f
                .debug_struct("RedirectNextDamageFromSourceToTarget")
                .field("amount", amount)
                .field("protected_target", protected_target)
                .field("destination", destination)
                .field("destination_target", destination_target)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target,
                all_this_turn,
            }) => f
                .debug_struct("RedirectNextTimeDamageToSource")
                .field("source", source)
                .field("target", target)
                .field("destination", destination)
                .field("destination_target", destination_target)
                .field("all_this_turn", all_this_turn)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { source }) => f
                .debug_struct("RedirectAllDamageThisTurnBySourceToSourceController")
                .field("source", source)
                .finish(),
            Self::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget {
                player_filter,
                object_filter,
                target,
            }) => f
                .debug_struct("RedirectAllDamageThisTurnToTarget")
                .field("player_filter", player_filter)
                .field("object_filter", object_filter)
                .field("target", target)
                .finish(),
            Self::KeywordActions(KeywordActionAst::Meld {
                result_name,
                enters_tapped,
                enters_attacking,
            }) => f
                .debug_struct("Meld")
                .field("result_name", result_name)
                .field("enters_tapped", enters_tapped)
                .field("enters_attacking", enters_attacking)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::SearchLibrarySlotsToHand {
                slots,
                destination,
                reveal,
                progress_tag,
            }) => f
                .debug_struct("SearchLibrarySlotsToHand")
                .field("slots", slots)
                .field("destination", destination)
                .field("reveal", reveal)
                .field("progress_tag", progress_tag)
                .finish(),
            Self::Stack(StackActionAst::RetargetStackObject {
                target,
                mode,
                require_change,
                copy_reference_plural,
            }) => f
                .debug_struct("RetargetStackObject")
                .field("target", target)
                .field("mode", mode)
                .field("require_change", require_change)
                .field("copy_reference_plural", copy_reference_plural)
                .finish(),
            Self::Grants(GrantActionAst::GrantAbilityToSource { ability, duration }) => f
                .debug_struct("GrantAbilityToSource")
                .field("ability", ability)
                .field("duration", duration)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::TurnFaceUp { target }) => f
                .debug_struct("TurnFaceUp")
                .field("target", target)
                .finish(),
            Self::Damage(DamageActionAst::DealDamage { amount, target, .. }) => f
                .debug_struct("DealDamage")
                .field("amount", amount)
                .field("target", target)
                .finish(),
            Self::Damage(DamageActionAst::DealDamageEach { amount, filter }) => f
                .debug_struct("DealDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .finish(),
            Self::Damage(DamageActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
                unpreventable,
            }) => f
                .debug_struct("DealDamageEqualToPower")
                .field("source", source)
                .field("amount", amount)
                .field("target", target)
                .field("unpreventable", unpreventable)
                .finish(),
            Self::Damage(DamageActionAst::DealDistributedDamage {
                amount,
                target,
                source,
                chooser,
                distribution,
            }) => f
                .debug_struct("DealDistributedDamage")
                .field("amount", amount)
                .field("target", target)
                .field("source", source)
                .field("chooser", chooser)
                .field("distribution", distribution)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::Tap { target }) => f.debug_tuple("Tap").field(target).finish(),
            Self::PermanentState(PermanentStateActionAst::Untap { target }) => f.debug_tuple("Untap").field(target).finish(),
            Self::PermanentState(PermanentStateActionAst::TapAll { filter }) => f.debug_tuple("TapAll").field(filter).finish(),
            Self::PermanentState(PermanentStateActionAst::UntapAll { filter }) => f.debug_tuple("UntapAll").field(filter).finish(),
            Self::PermanentState(PermanentStateActionAst::TapOrUntap { target }) => f.debug_tuple("TapOrUntap").field(target).finish(),
            Self::PermanentState(PermanentStateActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            }) => f
                .debug_struct("TapOrUntapAll")
                .field("tap_filter", tap_filter)
                .field("untap_filter", untap_filter)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::PhaseOut {
                target,
                duration,
                source_surface,
            }) => f
                .debug_struct("PhaseOut")
                .field("target", target)
                .field("duration", duration)
                .field("source_surface", source_surface)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::PhaseOutAll {
                filter,
                duration,
                source_surface,
            }) => f
                .debug_struct("PhaseOutAll")
                .field("filter", filter)
                .field("duration", duration)
                .field("source_surface", source_surface)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::PhaseIn { target }) => f.debug_tuple("PhaseIn").field(target).finish(),
            Self::PermanentState(PermanentStateActionAst::PhaseInAll { filter }) => f.debug_tuple("PhaseInAll").field(filter).finish(),
            Self::PermanentState(PermanentStateActionAst::Transform { target }) => f.debug_tuple("Transform").field(target).finish(),
            Self::PermanentState(PermanentStateActionAst::Convert { target }) => f.debug_tuple("Convert").field(target).finish(),
            Self::ZoneMoves(ZoneMoveActionAst::Destroy {
                target,
                no_regeneration,
                creature_destroyed_this_way_surface,
            }) => f
                .debug_struct("Destroy")
                .field("target", target)
                .field("no_regeneration", no_regeneration)
                .field(
                    "creature_destroyed_this_way_surface",
                    creature_destroyed_this_way_surface,
                )
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::DestroyAll {
                filter,
                no_regeneration,
                creature_destroyed_this_way_surface,
            }) => f
                .debug_struct("DestroyAll")
                .field("filter", filter)
                .field("no_regeneration", no_regeneration)
                .field(
                    "creature_destroyed_this_way_surface",
                    creature_destroyed_this_way_surface,
                )
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
                filter,
                no_regeneration,
                creature_destroyed_this_way_surface,
            }) => f
                .debug_struct("DestroyAllOfChosenColor")
                .field("filter", filter)
                .field("no_regeneration", no_regeneration)
                .field(
                    "creature_destroyed_this_way_surface",
                    creature_destroyed_this_way_surface,
                )
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { filter, target }) => f
                .debug_struct("DestroyAllAttachedTo")
                .field("filter", filter)
                .field("target", target)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo {
                filter,
                target,
                face_down,
            }) => f
                .debug_struct("ExileAllAttachedTo")
                .field("filter", filter)
                .field("target", target)
                .field("face_down", face_down)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::Exile {
                target,
                face_down,
                source_top_only,
                target_plural_surface,
            }) => f
                .debug_struct("Exile")
                .field("target", target)
                .field("face_down", face_down)
                .field("source_top_only", source_top_only)
                .field("target_plural_surface", target_plural_surface)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, face_down }) => f
                .debug_struct("ExileAll")
                .field("filter", filter)
                .field("face_down", face_down)
                .finish(),
            Self::RevealLook(RevealLookActionAst::LookAtHand { target }) => f.debug_tuple("LookAtHand").field(target).finish(),
            Self::Stack(StackActionAst::Counter { target }) => f.debug_tuple("Counter").field(target).finish(),
            Self::Stack(StackActionAst::CounterUnlessPays { target, cost }) => f
                .debug_struct("CounterUnlessPays")
                .field("target", target)
                .field("cost", cost)
                .finish(),
            Self::Counters(CounterActionAst::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            }) => f
                .debug_struct("PutCounters")
                .field("counter_type", counter_type)
                .field("count", count)
                .field("target", target)
                .field("target_count", target_count)
                .field("distributed", distributed)
                .finish(),
            Self::Counters(CounterActionAst::PutCounterChoice {
                counter_types,
                count,
                mode_texts,
                target,
                target_count,
            }) => f
                .debug_struct("PutCounterChoice")
                .field("counter_types", counter_types)
                .field("count", count)
                .field("mode_texts", mode_texts)
                .field("target", target)
                .field("target_count", target_count)
                .finish(),
            Self::Counters(CounterActionAst::PutOrRemoveCounters {
                put_counter_type,
                put_count,
                remove_counter_type,
                remove_count,
                put_mode_text,
                remove_mode_text,
                target,
                target_count,
            }) => f
                .debug_struct("PutOrRemoveCounters")
                .field("put_counter_type", put_counter_type)
                .field("put_count", put_count)
                .field("remove_counter_type", remove_counter_type)
                .field("remove_count", remove_count)
                .field("put_mode_text", put_mode_text)
                .field("remove_mode_text", remove_mode_text)
                .field("target", target)
                .field("target_count", target_count)
                .finish(),
            Self::Counters(CounterActionAst::PutCountersAll {
                counter_type,
                count,
                filter,
            }) => f
                .debug_struct("PutCountersAll")
                .field("counter_type", counter_type)
                .field("count", count)
                .field("filter", filter)
                .finish(),
            Self::Counters(CounterActionAst::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
                distributed_across_all,
                all_of_them,
            }) => f
                .debug_struct("RemoveUpToAnyCounters")
                .field("amount", amount)
                .field("target", target)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
                .field("distributed_across_all", distributed_across_all)
                .field("all_of_them", all_of_them)
                .finish(),
            Self::Counters(CounterActionAst::MoveAllCounters { from, to }) => f
                .debug_struct("MoveAllCounters")
                .field("from", from)
                .field("to", to)
                .finish(),
            Self::Counters(CounterActionAst::MoveOneCounter { from, to }) => f
                .debug_struct("MoveOneCounter")
                .field("from", from)
                .field("to", to)
                .finish(),
            Self::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source,
                all_kinds,
                fixed_counter_type,
                optional_action,
                put_only,
                choose_target_per_kind,
            }) => f
                .debug_struct("ForEachCounterKindPutOrRemove")
                .field("target", target)
                .field("counter_source", counter_source)
                .field("all_kinds", all_kinds)
                .field("fixed_counter_type", fixed_counter_type)
                .field("optional_action", optional_action)
                .field("put_only", put_only)
                .field("choose_target_per_kind", choose_target_per_kind)
                .finish(),
            Self::Counters(CounterActionAst::PutCounterOfChosenKind { target }) => f
                .debug_struct("PutCounterOfChosenKind")
                .field("target", target)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                target,
                random,
                destination_player_surface,
                exiled_with_source_surface,
                set_quantifier_surface,
                set_reference_surface,
            }) => f
                .debug_struct("ReturnToHand")
                .field("target", target)
                .field("random", random)
                .field("destination_player_surface", destination_player_surface)
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("set_reference_surface", set_reference_surface)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand {
                filter,
                destination_player_surface,
                exiled_with_source_surface,
            }) => f
                .debug_struct("ReturnAllToHand")
                .field("filter", filter)
                .field("destination_player_surface", destination_player_surface)
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter }) => f
                .debug_struct("ReturnAllToHandOfChosenColor")
                .field("filter", filter)
                .finish(),
            Self::Library(LibraryActionAst::MoveToLibraryNthFromTop { target, position }) => f
                .debug_struct("MoveToLibraryNthFromTop")
                .field("target", target)
                .field("position", position)
                .finish(),
            Self::Counters(CounterActionAst::DoubleCountersOnEach {
                counter_type,
                filter,
            }) => f
                .debug_struct("DoubleCountersOnEach")
                .field("counter_type", counter_type)
                .field("filter", filter)
                .finish(),
            Self::Counters(CounterActionAst::DoubleCountersOnTarget {
                counter_type,
                target,
            }) => f
                .debug_struct("DoubleCountersOnTarget")
                .field("counter_type", counter_type)
                .field("target", target)
                .finish(),
            Self::Counters(CounterActionAst::RemoveCountersAll {
                amount,
                filter,
                counter_type,
                up_to,
            }) => f
                .debug_struct("RemoveCountersAll")
                .field("amount", amount)
                .field("filter", filter)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
                .finish(),
            Self::PutSticker { target, action } => f
                .debug_struct("PutSticker")
                .field("target", target)
                .field("action", action)
                .finish(),
            Self::KeywordActions(KeywordActionAst::UnlockRoomDoor) => f.write_str("UnlockRoomDoor"),
            Self::PermanentState(PermanentStateActionAst::SwitchPowerToughness { target, duration }) => f
                .debug_struct("SwitchPowerToughness")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll {
                filter,
                power,
                toughness,
                multiplier,
                duration,
            }) => f
                .debug_struct("ScalePowerToughnessAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("multiplier", multiplier)
                .field("duration", duration)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            }) => f
                .debug_struct("Discard")
                .field("count", count)
                .field("random", random)
                .field("any_number", any_number)
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::DiscardHand) => f.write_str("DiscardHand"),
            Self::Counters(CounterActionAst::PoisonCounters { count }) => f.debug_tuple("PoisonCounters").field(count).finish(),
            Self::Counters(CounterActionAst::EnergyCounters { count }) => f.debug_tuple("EnergyCounters").field(count).finish(),
            Self::Counters(CounterActionAst::ExperienceCounters { count }) => {
                f.debug_tuple("ExperienceCounters").field(count).finish()
            }
            Self::Counters(CounterActionAst::TicketCounters { count }) => f.debug_tuple("TicketCounters").field(count).finish(),
            Self::LifeResources(LifeResourceActionAst::PayEnergy { amount }) => f.debug_tuple("PayEnergy").field(amount).finish(),
            Self::LifeResources(LifeResourceActionAst::PayAnyEnergy { min_amount }) => f
                .debug_struct("PayAnyEnergy")
                .field("min_amount", min_amount)
                .finish(),
            Self::LifeResources(LifeResourceActionAst::PayAnyLife { min_amount }) => f
                .debug_struct("PayAnyLife")
                .field("min_amount", min_amount)
                .finish(),
            Self::Mana(ManaActionAst::PayMana {
                cost,
                x_value,
                x_maximum,
            }) => f
                .debug_struct("PayMana")
                .field("cost", cost)
                .field("x_value", x_value)
                .field("x_maximum", x_maximum)
                .finish(),
            Self::Mana(ManaActionAst::DoubleManaPool) => f.write_str("DoubleManaPool"),
            Self::Mana(ManaActionAst::EmptyManaPool) => f.write_str("EmptyManaPool"),
            Self::Characteristics(CharacteristicActionAst::SetLifeTotal { amount }) => f.debug_tuple("SetLifeTotal").field(amount).finish(),
            Self::Game(GameActionAst::ReverseTurnOrder) => f.write_str("ReverseTurnOrder"),
            Self::Game(GameActionAst::EndTurn) => f.write_str("EndTurn"),
            Self::Game(GameActionAst::EndCombatPhase) => f.write_str("EndCombatPhase"),
            Self::TurnStructure(TurnStructureActionAst::SkipTurn) => f.write_str("SkipTurn"),
            Self::TurnStructure(TurnStructureActionAst::SkipCombatPhases) => f.write_str("SkipCombatPhases"),
            Self::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn) => f.write_str("SkipNextCombatPhaseThisTurn"),
            Self::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn) => f.write_str("SkipMainPhasesThisTurn"),
            Self::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn) => f.write_str("SkipCombatPhasesThisTurn"),
            Self::TurnStructure(TurnStructureActionAst::SkipDrawStep) => f.write_str("SkipDrawStep"),
            Self::TurnStructure(TurnStructureActionAst::AdditionalPhases { phases }) => {
                f.debug_tuple("AdditionalPhases").field(phases).finish()
            }
            Self::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot) => f.write_str("PlayFromGraveyardUntilEot"),
            Self::Control(ControlActionAst::ControlPlayer { player, duration }) => f
                .debug_struct("ControlPlayer")
                .field("player", player)
                .field("duration", duration)
                .finish(),
            Self::Stack(StackActionAst::ReduceNextSpellCostThisTurn { filter, reduction }) => f
                .debug_struct("ReduceNextSpellCostThisTurn")
                .field("filter", filter)
                .field("reduction", reduction)
                .finish(),
            Self::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration,
                next_only,
            }) => f
                .debug_struct("ReduceMatchingSpellCostThisTurn")
                .field("filter", filter)
                .field("reduction", reduction)
                .field("duration", duration)
                .field("next_only", next_only)
                .finish(),
            Self::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn { filter, ability }) => f
                .debug_struct("GrantNextSpellAbilityThisTurn")
                .field("filter", filter)
                .field("ability", ability)
                .finish(),
            Self::KeywordActions(KeywordActionAst::RingTemptsYou) => f.write_str("RingTemptsYou"),
            Self::KeywordActions(KeywordActionAst::VentureIntoDungeon {
                undercity_if_no_active,
            }) => f
                .debug_struct("VentureIntoDungeon")
                .field("undercity_if_no_active", undercity_if_no_active)
                .finish(),
            Self::Characteristics(CharacteristicActionAst::BecomeMonarch) => f.write_str("BecomeMonarch"),
            Self::KeywordActions(KeywordActionAst::TakeInitiative) => f.write_str("TakeInitiative"),
            Self::Tokens(TokenActionAst::CreateEmblem { emblem }) => f.debug_tuple("CreateEmblem").field(emblem).finish(),
            Self::Game(GameActionAst::LoseGame) => f.write_str("LoseGame"),
            Self::Game(GameActionAst::WinGame) => f.write_str("WinGame"),
            Self::KeywordActions(KeywordActionAst::Detain { target }) => f.debug_tuple("Detain").field(target).finish(),
            Self::KeywordActions(KeywordActionAst::Goad { target, duration }) => f
                .debug_struct("Goad")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::KeywordActions(KeywordActionAst::Suspect { target }) => f.debug_tuple("Suspect").field(target).finish(),
            Self::KeywordActions(KeywordActionAst::ClearSuspected { target }) => {
                f.debug_tuple("ClearSuspected").field(target).finish()
            }
            Self::Damage(DamageActionAst::HealDamage { target, amount }) => f
                .debug_struct("HealDamage")
                .field("target", target)
                .field("amount", amount)
                .finish(),
            Self::PermanentState(PermanentStateActionAst::RemoveFromCombat { target }) => {
                f.debug_tuple("RemoveFromCombat").field(target).finish()
            }
            Self::PermanentState(PermanentStateActionAst::Flip { target }) => f.debug_tuple("Flip").field(target).finish(),
            Self::KeywordActions(KeywordActionAst::Regenerate {
                target,
                follow_up_effects,
            }) => f
                .debug_struct("Regenerate")
                .field("target", target)
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::KeywordActions(KeywordActionAst::RegenerateAll { filter }) => f.debug_tuple("RegenerateAll").field(filter).finish(),
            Self::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set,
            }) => f
                .debug_struct("Sacrifice")
                .field("filter", filter)
                .field("count", count)
                .field("target", target)
                .field("one_of_referenced_set", one_of_referenced_set)
                .finish(),
            Self::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) => f
                .debug_struct("SacrificeAll")
                .field("filter", filter)
                .finish(),
        }
    }
}

impl std::fmt::Debug for SubjectVerbEffectAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectVerb")
            .field("subject", &self.subject)
            .field("action", &self.action)
            .finish()
    }
}
