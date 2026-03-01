//! Create token copy effect implementation.

use crate::card::PtValue;
use crate::color::ColorSet;
use crate::combat_state::{AttackTarget, AttackerInfo};
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::object::Object;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::lifecycle::{
    TokenCleanupOptions, TokenEntryOptions, apply_token_battlefield_entry,
    grant_token_static_abilities, schedule_token_cleanup,
};

/// Effect that creates a token copy of a permanent.
///
/// # Fields
///
/// * `target` - Which permanent to copy
/// * `count` - How many copies to create
/// * `controller` - Who controls the tokens
/// * `enters_tapped` - Whether the copy enters tapped
/// * `has_haste` - Whether the copy has haste
/// * `enters_attacking` - Whether the copy enters attacking
/// * `attack_target_mode` - Optional custom attack-target selection when attacking
/// * `exile_at_end_of_combat` - Whether to exile at end of combat
///
/// # Example
///
/// ```ignore
/// // Create a token copy of target creature
/// let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
///
/// // Create a copy with haste that's exiled at end of combat (Kiki-Jiki style)
/// let effect = CreateTokenCopyEffect::kiki_jiki_style(ChooseSpec::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTokenCopyEffect {
    /// Which permanent to copy.
    pub target: ChooseSpec,
    /// How many copies to create.
    pub count: Value,
    /// Who controls the tokens.
    pub controller: PlayerFilter,
    /// Whether the copy enters tapped.
    pub enters_tapped: bool,
    /// Whether the copy has haste.
    pub has_haste: bool,
    /// Whether the copy enters attacking.
    pub enters_attacking: bool,
    /// Optional custom attack-target selection when entering attacking.
    pub attack_target_mode: Option<CopyAttackTargetMode>,
    /// Whether to exile at end of combat.
    pub exile_at_end_of_combat: bool,
    /// Whether to sacrifice at the beginning of the next end step.
    pub sacrifice_at_next_end_step: bool,
    /// Whether to exile at the beginning of the next end step.
    pub exile_at_next_end_step: bool,
    /// Optional power/toughness adjustment for the created tokens.
    pub pt_adjustment: Option<CopyPtAdjustment>,
    /// Card types to add to copied tokens.
    pub added_card_types: Vec<CardType>,
    /// Subtypes to add to copied tokens.
    pub added_subtypes: Vec<Subtype>,
    /// Supertypes to remove from copied tokens.
    pub removed_supertypes: Vec<Supertype>,
    /// Optional fixed base power/toughness override.
    pub set_base_power_toughness: Option<(i32, i32)>,
    /// Optional color override for copied tokens.
    pub set_colors: Option<ColorSet>,
    /// Optional exact card-type override for copied tokens.
    pub set_card_types: Option<Vec<CardType>>,
    /// Optional exact subtype override for copied tokens.
    pub set_subtypes: Option<Vec<Subtype>>,
    /// Static abilities to grant to copied tokens.
    pub granted_static_abilities: Vec<StaticAbility>,
}

/// Optional power/toughness adjustment for copied tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyPtAdjustment {
    /// Set base power/toughness to half (rounded up) of the original.
    HalfRoundUp,
}

/// Attack-target assignment mode for copied tokens entering attacking.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyAttackTargetMode {
    /// Choose among the player and planeswalkers controlled by that player.
    PlayerOrPlaneswalkerControlledBy(PlayerFilter),
}

impl CreateTokenCopyEffect {
    /// Create a new create token copy effect.
    pub fn new(target: ChooseSpec, count: impl Into<Value>, controller: PlayerFilter) -> Self {
        Self {
            target,
            count: count.into(),
            controller,
            enters_tapped: false,
            has_haste: false,
            enters_attacking: false,
            attack_target_mode: None,
            exile_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            pt_adjustment: None,
            added_card_types: Vec::new(),
            added_subtypes: Vec::new(),
            removed_supertypes: Vec::new(),
            set_base_power_toughness: None,
            set_colors: None,
            set_card_types: None,
            set_subtypes: None,
            granted_static_abilities: Vec::new(),
        }
    }

    /// Create a single token copy under your control.
    pub fn one(target: ChooseSpec) -> Self {
        Self::new(target, 1, PlayerFilter::You)
    }

    /// Create a token copy with haste.
    pub fn with_haste(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.has_haste = true;
        effect
    }

    /// Create a token copy that enters tapped.
    pub fn tapped(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.enters_tapped = true;
        effect
    }

    /// Create a Kiki-Jiki style copy: has haste and is exiled at end of combat.
    pub fn kiki_jiki_style(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.has_haste = true;
        effect.exile_at_end_of_combat = true;
        effect
    }

    /// Set whether the copy enters tapped.
    pub fn enters_tapped(mut self, value: bool) -> Self {
        self.enters_tapped = value;
        self
    }

    /// Set whether the copy has haste.
    pub fn haste(mut self, value: bool) -> Self {
        self.has_haste = value;
        self
    }

    /// Set whether the copy enters attacking.
    pub fn attacking(mut self, value: bool) -> Self {
        self.enters_attacking = value;
        if !value {
            self.attack_target_mode = None;
        }
        self
    }

    /// Set a custom attack-target assignment mode for copied tokens.
    pub fn attack_target_mode(mut self, mode: CopyAttackTargetMode) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(mode);
        self
    }

    /// Enter attacking a chosen player or a planeswalker that player controls.
    pub fn attacking_player_or_planeswalker_controlled_by(mut self, player: PlayerFilter) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
            player,
        ));
        self
    }

    /// Set whether to exile at end of combat.
    pub fn exile_at_eoc(mut self, value: bool) -> Self {
        self.exile_at_end_of_combat = value;
        self
    }

    /// Set whether to sacrifice at the beginning of the next end step.
    pub fn sacrifice_at_next_end_step(mut self, value: bool) -> Self {
        self.sacrifice_at_next_end_step = value;
        self
    }

    /// Set whether to exile at the beginning of the next end step.
    pub fn exile_at_next_end_step(mut self, value: bool) -> Self {
        self.exile_at_next_end_step = value;
        self
    }

    /// Set base power/toughness to half (rounded up) of the original.
    pub fn half_power_toughness_round_up(mut self) -> Self {
        self.pt_adjustment = Some(CopyPtAdjustment::HalfRoundUp);
        self
    }

    /// Add a card type to copied tokens.
    pub fn added_card_type(mut self, card_type: CardType) -> Self {
        if !self.added_card_types.contains(&card_type) {
            self.added_card_types.push(card_type);
        }
        self
    }

    /// Add a subtype to copied tokens.
    pub fn added_subtype(mut self, subtype: Subtype) -> Self {
        if !self.added_subtypes.contains(&subtype) {
            self.added_subtypes.push(subtype);
        }
        self
    }

    /// Remove a supertype from copied tokens.
    pub fn removed_supertype(mut self, supertype: Supertype) -> Self {
        if !self.removed_supertypes.contains(&supertype) {
            self.removed_supertypes.push(supertype);
        }
        self
    }

    /// Set copied tokens to a fixed base power/toughness.
    pub fn set_base_power_toughness(mut self, power: i32, toughness: i32) -> Self {
        self.set_base_power_toughness = Some((power, toughness));
        self
    }

    /// Set copied tokens to exact colors.
    pub fn set_colors(mut self, colors: ColorSet) -> Self {
        self.set_colors = Some(colors);
        self
    }

    /// Set copied tokens to exact card types.
    pub fn set_card_types(mut self, card_types: Vec<CardType>) -> Self {
        self.set_card_types = Some(card_types);
        self
    }

    /// Set copied tokens to exact subtypes.
    pub fn set_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.set_subtypes = Some(subtypes);
        self
    }

    /// Grant a static ability to copied tokens.
    pub fn grant_static_ability(mut self, ability: StaticAbility) -> Self {
        self.granted_static_abilities.push(ability);
        self
    }

    fn attack_targets_for_player(game: &GameState, player_id: PlayerId) -> Vec<AttackTarget> {
        let mut targets = Vec::new();
        if game
            .player(player_id)
            .is_some_and(|player| player.is_in_game())
        {
            targets.push(AttackTarget::Player(player_id));
        }

        for &object_id in &game.battlefield {
            if let Some(object) = game.object(object_id)
                && object.controller == player_id
                && object.has_card_type(CardType::Planeswalker)
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            }
        }

        targets
    }

    fn choose_attack_target(
        game: &GameState,
        ctx: &mut ExecutionContext,
        player_id: PlayerId,
        targets: &[AttackTarget],
    ) -> AttackTarget {
        if targets.len() == 1 {
            return targets[0].clone();
        }

        let player_name = game
            .player(player_id)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| "that player".to_string());
        let options: Vec<SelectableOption> = targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let description = match target {
                    AttackTarget::Player(_) => format!("Attack {player_name}"),
                    AttackTarget::Planeswalker(planeswalker_id) => {
                        let walker_name = game
                            .object(*planeswalker_id)
                            .map(|object| object.name.clone())
                            .unwrap_or_else(|| "a planeswalker".to_string());
                        format!("Attack {walker_name} controlled by {player_name}")
                    }
                };
                SelectableOption::new(index, description)
            })
            .collect();
        let choice_ctx = SelectOptionsContext::new(
            ctx.controller,
            Some(ctx.source),
            format!("Choose attack target for token copy attacking {player_name}"),
            options,
            1,
            1,
        );
        let chosen = ctx.decision_maker.decide_options(game, &choice_ctx);
        let index = chosen
            .first()
            .copied()
            .filter(|selected| *selected < targets.len())
            .unwrap_or(0);
        targets[index].clone()
    }
}

impl EffectExecutor for CreateTokenCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Resolve target from spec (supports tagged/spec-specific references)
        let target_ids = resolve_objects_from_spec(game, &self.target, ctx)?;
        let target_id = *target_ids.first().ok_or(ExecutionError::InvalidTarget)?;

        // Resolve target object (supports tagged LKI with stable_id lookup)
        let mut tagged_snapshot = None;
        let mut resolved_target_id = target_id;
        let mut target_object = game.object(resolved_target_id);

        if target_object.is_none() {
            if let ChooseSpec::Tagged(tag) = &self.target {
                if let Some(snapshot) = ctx.get_tagged(tag.as_str()) {
                    tagged_snapshot = Some(snapshot.clone());
                    if let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id) {
                        resolved_target_id = current_id;
                        target_object = game.object(resolved_target_id);
                    }
                }
            }
        }

        let Some(target_object) = target_object else {
            return Err(ExecutionError::ObjectNotFound(target_id));
        };
        let configured_attack_player = match &self.attack_target_mode {
            Some(CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player_filter)) => {
                Some(resolve_player_filter(game, player_filter, ctx)?)
            }
            None => None,
        };
        let cleanup_options = TokenCleanupOptions::new(
            self.exile_at_end_of_combat,
            false,
            self.sacrifice_at_next_end_step,
            self.exile_at_next_end_step,
        );
        let entry_options = TokenEntryOptions::new(
            self.enters_tapped,
            self.enters_attacking && configured_attack_player.is_none(),
        );
        let mut static_abilities_to_grant =
            Vec::with_capacity(self.granted_static_abilities.len() + usize::from(self.has_haste));
        if self.has_haste {
            static_abilities_to_grant.push(StaticAbility::haste());
        }
        static_abilities_to_grant.extend(self.granted_static_abilities.iter().cloned());

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count);

        let target_for_stats = &target_object;
        let (half_power, half_toughness) = match self.pt_adjustment {
            Some(CopyPtAdjustment::HalfRoundUp) => {
                let (power, toughness) = if let Some(snapshot) = &tagged_snapshot {
                    (snapshot.power.unwrap_or(0), snapshot.toughness.unwrap_or(0))
                } else {
                    (
                        target_for_stats.power().unwrap_or(0),
                        target_for_stats.toughness().unwrap_or(0),
                    )
                };
                ((power + 1) / 2, (toughness + 1) / 2)
            }
            None => (0, 0),
        };

        for _ in 0..count {
            let id = game.new_object_id();
            // Get fresh reference to target each iteration
            let target = game
                .object(resolved_target_id)
                .ok_or(ExecutionError::ObjectNotFound(resolved_target_id))?;
            let mut token = Object::token_copy_of(target, id, controller_id);
            token.zone = Zone::Battlefield;

            if let Some(CopyPtAdjustment::HalfRoundUp) = self.pt_adjustment {
                token.base_power = Some(PtValue::Fixed(half_power));
                token.base_toughness = Some(PtValue::Fixed(half_toughness));
            }
            if let Some((power, toughness)) = self.set_base_power_toughness {
                token.base_power = Some(PtValue::Fixed(power));
                token.base_toughness = Some(PtValue::Fixed(toughness));
            }
            if let Some(colors) = self.set_colors {
                token.color_override = Some(colors);
            }
            if let Some(card_types) = &self.set_card_types {
                token.card_types = card_types.clone();
            }
            if let Some(subtypes) = &self.set_subtypes {
                token.subtypes = subtypes.clone();
            }
            for card_type in &self.added_card_types {
                if !token.card_types.contains(card_type) {
                    token.card_types.push(*card_type);
                }
            }
            for subtype in &self.added_subtypes {
                if !token.subtypes.contains(subtype) {
                    token.subtypes.push(*subtype);
                }
            }
            if !self.removed_supertypes.is_empty() {
                token
                    .supertypes
                    .retain(|supertype| !self.removed_supertypes.contains(supertype));
            }
            let token_is_creature = token.is_creature();

            game.add_object(token);
            created_ids.push(id);
            apply_token_battlefield_entry(
                game,
                ctx,
                id,
                controller_id,
                token_is_creature,
                entry_options,
                &mut events,
            )?;

            if let Some(attack_player) = configured_attack_player {
                let targets = Self::attack_targets_for_player(game, attack_player);
                if !targets.is_empty() {
                    let chosen_target =
                        Self::choose_attack_target(game, ctx, attack_player, &targets);
                    if let Some(combat) = game.combat.as_mut() {
                        combat.attackers.push(AttackerInfo {
                            creature: id,
                            target: chosen_target,
                        });
                    }
                }
            }

            schedule_token_cleanup(game, ctx, id, controller_id, cleanup_options)?;
            grant_token_static_abilities(game, ctx, id, &static_abilities_to_grant)?;
        }

        Ok(EffectOutcome::from_result(EffectResult::Objects(created_ids)).with_events(events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to copy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::ObjectKind;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_planeswalker(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Planeswalker])
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_create_token_copy() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Grizzly Bears");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_with_haste() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Baneslayer Angel", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::with_haste(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token = game.object(ids[0]).unwrap();
            // Token should have haste ability
            let has_haste = token.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            assert!(has_haste, "Token should have haste");
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Serra Angel", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::tapped(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert!(game.is_tapped(ids[0]), "Token should enter tapped");
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_multiple_token_copies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Llanowar Elves", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::new(ChooseSpec::creature(), 3, PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Llanowar Elves");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err(), "Should fail without target");
    }

    #[test]
    fn test_create_token_copy_clone_box() {
        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenCopyEffect"));
    }

    #[test]
    fn test_create_token_copy_kiki_jiki_style() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Pestermite", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::kiki_jiki_style(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token_id = ids[0];
            let token = game.object(token_id).unwrap();

            // Token should have haste
            let has_haste = token.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            });
            assert!(has_haste, "Token should have haste");

            // Should have delayed trigger to exile at end of combat
            assert_eq!(game.delayed_triggers.len(), 1);
            let delayed = &game.delayed_triggers[0];
            assert!(delayed.trigger.display().contains("end of combat"));
            assert!(delayed.one_shot);
            assert_eq!(delayed.target_objects, vec![token_id]);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_enters_attacking() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature(&mut game, "Goblin Guide", alice);
        let source = create_creature(&mut game, "Source Attacker", alice);

        // Set up combat with source attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature()).attacking(true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token_id = ids[0];
            // Token should be added to combat attackers
            let combat = game.combat.as_ref().expect("Combat should still be active");
            assert!(
                combat
                    .attackers
                    .iter()
                    .any(|info| info.creature == token_id),
                "Token should be in combat attackers"
            );
            // Token should be attacking the same target as source
            let token_attacker = combat
                .attackers
                .iter()
                .find(|info| info.creature == token_id)
                .expect("Token should be attacking");
            assert_eq!(
                token_attacker.target,
                AttackTarget::Player(bob),
                "Token should attack the same target as source"
            );
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_attacks_chosen_planeswalker_of_iterated_player() {
        use crate::combat_state::{AttackTarget, CombatState};
        use crate::decision::DecisionMaker;

        struct ChooseLastOptionDecisionMaker;
        impl DecisionMaker for ChooseLastOptionDecisionMaker {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                vec![ctx.options.last().map(|option| option.index).unwrap_or(0)]
            }
        }

        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let creature_id = create_creature(&mut game, "Goblin Guide", alice);
        let source = create_creature(&mut game, "Source Attacker", alice);
        let charlie_walker = create_planeswalker(&mut game, "Charlie Walker", charlie);
        game.combat = Some(CombatState::default());

        let mut dm = ChooseLastOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.iterated_player = Some(charlie);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature())
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(ids) = result.result {
            let token_id = ids[0];
            let combat = game.combat.as_ref().expect("Combat should still be active");
            let token_attacker = combat
                .attackers
                .iter()
                .find(|info| info.creature == token_id)
                .expect("Token should be attacking");
            assert_eq!(
                token_attacker.target,
                AttackTarget::Planeswalker(charlie_walker),
                "Token should attack the chosen planeswalker"
            );
        } else {
            panic!("Expected Objects result");
        }
        assert_ne!(bob, charlie, "sanity check");
    }

    #[test]
    fn test_composed_myriad_effect_creates_for_each_other_opponent_and_exiles_at_eoc() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
        use crate::decision::DecisionMaker;
        use crate::effect::Effect;
        use crate::events::phase::EndOfCombatEvent;
        use crate::executor::execute_effect;
        use crate::triggers::TriggerEvent;

        struct AlwaysYesDecisionMaker;
        impl DecisionMaker for AlwaysYesDecisionMaker {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }

        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Dana".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let dana = PlayerId::from_index(3);
        let source = create_creature(&mut game, "Myriad Source", alice);
        let other_attacker = create_creature(&mut game, "Other Attacker", alice);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: source,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other_attacker,
                    target: AttackTarget::Player(charlie),
                },
            ],
            ..CombatState::default()
        });

        let composed_myriad = Effect::for_players(
            PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending),
            vec![Effect::may(vec![Effect::new(
                CreateTokenCopyEffect::new(ChooseSpec::Source, 1, PlayerFilter::You)
                    .enters_tapped(true)
                    .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer)
                    .exile_at_eoc(true),
            )])],
        );

        let mut dm = AlwaysYesDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_defending_player(bob);
        let outcome = execute_effect(&mut game, &composed_myriad, &mut ctx).unwrap();
        assert!(
            outcome.result.something_happened(),
            "expected composed myriad effect to create at least one token"
        );

        let combat = game.combat.as_ref().expect("combat should exist");
        let token_attackers: Vec<_> = combat
            .attackers
            .iter()
            .filter_map(|info| {
                (info.creature != source && info.creature != other_attacker)
                    .then_some((info.creature, info.target.clone()))
            })
            .collect();
        assert_eq!(token_attackers.len(), 2);

        let mut attacked_players: Vec<_> = token_attackers
            .iter()
            .filter_map(|(_, target)| match target {
                AttackTarget::Player(player) => Some(*player),
                AttackTarget::Planeswalker(_) => None,
            })
            .collect();
        attacked_players.sort();
        assert_eq!(attacked_players, vec![charlie, dana]);

        let token_ids: Vec<_> = token_attackers.iter().map(|(id, _)| *id).collect();
        let cleanup_trigger_count = game
            .delayed_triggers
            .iter()
            .filter(|delayed| {
                delayed.trigger.display().contains("end of combat")
                    && delayed.target_objects.len() == 1
                    && token_ids.contains(&delayed.target_objects[0])
            })
            .count();
        assert_eq!(cleanup_trigger_count, 2);

        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        let event = TriggerEvent::new(EndOfCombatEvent::new());
        for entry in crate::triggers::check_delayed_triggers(&mut game, &event) {
            trigger_queue.add(entry);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("put delayed triggers on stack");
        while !game.stack_is_empty() {
            crate::game_loop::resolve_stack_entry(&mut game).expect("resolve delayed trigger");
        }

        for token_id in token_ids {
            assert!(
                !game.battlefield.contains(&token_id),
                "myriad token should be exiled at end of combat"
            );
        }
    }
}
