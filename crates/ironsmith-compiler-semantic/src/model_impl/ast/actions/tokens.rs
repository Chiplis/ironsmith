//! The tokens actions of `SubjectVerbActionAst`.

use super::*;

/// Less common ways a created token enters combat.
#[derive(Debug, Clone, PartialEq, Default, TagKeyWalk)]
pub struct TokenCombatEntryAst {
    /// "a token that's blocking <that creature>" (CR 509.4): the attacking
    /// creature each token enters blocking.
    pub blocking: Option<TargetAst>,
    /// "attacking that player or a planeswalker they control": the token's
    /// controller chooses among `attack_target_player` and the planeswalkers
    /// that player controls (CR 508.4).
    pub attack_target_includes_planeswalkers: bool,
}

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum TokenActionAst {
    CreateTokenCopy {
        object: ObjectRefAst,
        count: Value,
        player: PlayerAst,
        enters_tapped: bool,
        enters_attacking: bool,
        entry_tapped_attacking_followup: bool,
        attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        attack_target_player_only: bool,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        haste_followup_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        exile_at_end_of_combat: bool,
        exile_at_end_of_combat_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        loses_soulbond: bool,
        sacrifice_at_next_end_step: bool,
        sacrifice_at_next_end_step_reference_surface:
            Option<crate::effect::TokenCopyReferenceSurface>,
        sacrifice_at_next_end_step_ability_surface: Option<TokenCopySacrificeAbilitySurface>,
        exile_at_next_end_step: bool,
        exile_at_next_end_step_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        next_end_step_player: PlayerFilter,
        set_colors: Option<ColorSet>,
        set_card_types: Option<Vec<CardType>>,
        set_subtypes: Option<Vec<Subtype>>,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        set_base_power_toughness_to_source_totals: bool,
        starting_loyalty: Option<u32>,
        granted_abilities: Vec<GrantedAbilityAst>,
    },
    CreateTokenCopyFromSource {
        source: TargetAst,
        count: Value,
        player: PlayerAst,
        enters_tapped: bool,
        enters_attacking: bool,
        entry_tapped_attacking_followup: bool,
        attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        attack_target_player_only: bool,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        haste_followup_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        exile_at_end_of_combat: bool,
        exile_at_end_of_combat_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        loses_soulbond: bool,
        sacrifice_at_next_end_step: bool,
        sacrifice_at_next_end_step_reference_surface:
            Option<crate::effect::TokenCopyReferenceSurface>,
        sacrifice_at_next_end_step_ability_surface: Option<TokenCopySacrificeAbilitySurface>,
        exile_at_next_end_step: bool,
        exile_at_next_end_step_reference_surface: Option<crate::effect::TokenCopyReferenceSurface>,
        next_end_step_player: PlayerFilter,
        set_colors: Option<ColorSet>,
        set_card_types: Option<Vec<CardType>>,
        set_subtypes: Option<Vec<Subtype>>,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        set_base_power_toughness_to_source_totals: bool,
        starting_loyalty: Option<u32>,
        granted_abilities: Vec<GrantedAbilityAst>,
        /// CR 707.9b: "except its name is X" / "... named X".
        set_name: Option<String>,
        /// CR 707.9b: "except it's legendary".
        added_supertypes: Vec<Supertype>,
    },
    CreateTokenWithMods {
        name: String,
        definition: crate::model::token_definition::TokenDefinitionSpec,
        count: Value,
        dynamic_power_toughness: Option<(Value, Value)>,
        player: PlayerAst,
        /// The source text explicitly used `you` as the create-action actor.
        /// This does not participate in controller resolution.
        actor_surface_explicit: bool,
        attached_to: Option<TargetAst>,
        tapped: bool,
        attacking: bool,
        /// Authored player attacked by the created token (for example,
        /// `attacking that player` inside a per-opponent loop).
        attack_target_player: Option<PlayerAst>,
        /// Less common combat-entry modifiers (blocking, attacking a player
        /// or a planeswalker they control). Defaults to none.
        combat_entry: TokenCombatEntryAst,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        ability_presentation: Option<ironsmith_core::TokenAbilityPresentation>,
    },
    /// "Create your choice of A, B, or C" — one mode per option, each mode a
    /// complete create effect.
    CreateTokenChoice {
        options: Vec<(String, Box<EffectAst>)>,
    },
    CreateEmblem {
        emblem: EmblemDescriptionAst,
    },
}
