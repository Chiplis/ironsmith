//! Create token copy effect implementation.

use crate::ability::Ability;
use crate::card::PtValue;
use crate::combat_state::{AttackTarget, AttackerInfo};
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;

use super::lifecycle::{
    TokenCleanupOptions, TokenEntryOptions, apply_token_battlefield_entry,
    create_replacement_additional_tokens, remaining_token_slots, schedule_token_cleanup,
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
pub type CopyPtAdjustment = ironsmith_core::CopyPtAdjustment;
pub type CopyAttackTargetMode = ironsmith_core::CopyAttackTargetMode;
pub type CreateTokenCopyEffect = ironsmith_core::CreateTokenCopyEffect<StaticAbility>;

fn attack_targets_for_player(game: &GameState, player_id: PlayerId) -> Vec<AttackTarget> {
    let mut targets = Vec::new();
    if game
        .player(player_id)
        .is_some_and(|player| player.is_in_game())
    {
        targets.push(AttackTarget::Player(player_id));
    }

    for &object_id in &game.battlefield {
        if let Some(object) = game.object(object_id) {
            if game.controller_of(object) == player_id
                && object.has_card_type(CardType::Planeswalker)
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            } else if object.has_card_type(CardType::Battle)
                && game.battle_protector(object_id) == Some(player_id)
            {
                targets.push(AttackTarget::Battle(object_id));
            }
        }
    }

    targets
}

fn choose_attack_target(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    targets: &[AttackTarget],
) -> Option<AttackTarget> {
    if targets.len() == 1 {
        return Some(targets[0].clone());
    }

    let player_name = game
        .player(player_id)
        .map(|player| player.name.to_string())
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
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a planeswalker".to_string());
                    format!("Attack {walker_name} controlled by {player_name}")
                }
                AttackTarget::Battle(battle_id) => {
                    let battle_name = game
                        .object(*battle_id)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a battle".to_string());
                    format!("Attack {battle_name} protected by {player_name}")
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
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    chosen
        .first()
        .copied()
        .filter(|selected| *selected < targets.len())
        .and_then(|index| targets.get(index))
        .cloned()
}

fn build_token_copy_object(
    effect: &CreateTokenCopyEffect,
    id: ObjectId,
    controller_id: PlayerId,
    target_object: Option<&Object>,
    copy_snapshot: Option<&ObjectSnapshot>,
    resolved_target_id: ObjectId,
    half_power: i32,
    half_toughness: i32,
    static_abilities_to_grant: &[StaticAbility],
) -> Result<Object, ExecutionError> {
    let mut token = if let Some(snapshot) = copy_snapshot {
        Object::token_copy_from_snapshot(snapshot, id, controller_id)
    } else {
        let target = target_object.ok_or(ExecutionError::ObjectNotFound(resolved_target_id))?;
        Object::token_copy_of(target, id, controller_id)
    };

    if let Some(CopyPtAdjustment::HalfRoundUp) = effect.pt_adjustment {
        token.base_power = Some(PtValue::Fixed(half_power));
        token.base_toughness = Some(PtValue::Fixed(half_toughness));
    }
    if let Some((power, toughness)) = effect.set_base_power_toughness {
        token.base_power = Some(PtValue::Fixed(power));
        token.base_toughness = Some(PtValue::Fixed(toughness));
    }
    if let Some(colors) = effect.set_colors {
        token.color_override = Some(colors);
    }
    if effect.clear_mana_cost {
        token.mana_cost = None;
    }
    if let Some(card_types) = &effect.set_card_types {
        token.card_types = card_types.clone().into();
    }
    if let Some(subtypes) = &effect.set_subtypes {
        token.subtypes = subtypes.clone().into();
    }
    for card_type in &effect.added_card_types {
        if !token.card_types.contains(card_type) {
            token.card_types.push(*card_type);
        }
    }
    for subtype in &effect.added_subtypes {
        if !token.subtypes.contains(subtype) {
            token.subtypes.push(*subtype);
        }
    }
    if !effect.removed_supertypes.is_empty() {
        token
            .supertypes
            .retain(|supertype| !effect.removed_supertypes.contains(supertype));
    }
    for static_ability in static_abilities_to_grant {
        token
            .abilities_mut()
            .push(Ability::static_ability(static_ability.clone()));
    }

    Ok(token)
}

impl EffectExecutor for CreateTokenCopyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        if !game
            .player(controller_id)
            .is_some_and(|player| player.is_in_game())
        {
            return Ok(EffectOutcome::with_objects(Vec::new()));
        }
        let base_count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        // A sacrificed copy source has already left the battlefield. Its tag
        // carries the calculated snapshot captured while paying the cost, so
        // use that identity and LKI directly instead of relocating the object
        // by stable id into its new zone.
        let sacrificed_snapshot =
            self.target
                .sacrificed_object_kind()
                .and_then(|_| match self.target.base() {
                    ChooseSpec::Tagged(tag) => ctx.get_tagged(tag.as_str()).cloned(),
                    _ => None,
                });
        let target_id = if let Some(snapshot) = sacrificed_snapshot.as_ref() {
            snapshot.object_id
        } else {
            let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
            *target_ids.first().ok_or(ExecutionError::InvalidTarget)?
        };

        // Resolve target object, falling back to stored LKI snapshots when needed.
        let resolved_target_id = target_id;
        let target_object = sacrificed_snapshot
            .is_none()
            .then(|| game.object(resolved_target_id).cloned())
            .flatten();
        let mut stored_snapshot = sacrificed_snapshot;
        if target_object.is_none() {
            if stored_snapshot.is_some() {
                // Typed sacrificed sources always prefer the cost-time LKI.
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                stored_snapshot = Some(snapshot.clone());
            } else {
                match self.target.base() {
                    ChooseSpec::Tagged(tag) => {
                        if let Some(snapshot) = ctx.get_tagged(tag.as_str()) {
                            stored_snapshot = Some(snapshot.clone());
                        }
                    }
                    ChooseSpec::Source => {
                        if let Some(snapshot) = &ctx.source_snapshot {
                            stored_snapshot = Some(snapshot.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        if stored_snapshot.is_none()
            && let Some(target) = target_object.as_ref()
        {
            stored_snapshot = Some(ObjectSnapshot::from_object_with_calculated_characteristics(
                target, game,
            ));
        }
        let copy_snapshot = stored_snapshot.as_ref();
        if target_object.is_none() && copy_snapshot.is_none() {
            return Err(ExecutionError::ObjectNotFound(target_id));
        }
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
            self.next_end_step_player.clone(),
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

        let (half_power, half_toughness) = match self.pt_adjustment {
            Some(CopyPtAdjustment::HalfRoundUp) => {
                let (power, toughness) = if let Some(snapshot) = copy_snapshot {
                    (snapshot.power.unwrap_or(0), snapshot.toughness.unwrap_or(0))
                } else {
                    let target = target_object
                        .as_ref()
                        .expect("target object should exist when no snapshot is available");
                    (target.power().unwrap_or(0), target.toughness().unwrap_or(0))
                };
                ((power + 1) / 2, (toughness + 1) / 2)
            }
            None => (0, 0),
        };

        let token_preview = build_token_copy_object(
            self,
            ObjectId::from_raw(0),
            controller_id,
            target_object.as_ref(),
            copy_snapshot,
            resolved_target_id,
            half_power,
            half_toughness,
            &static_abilities_to_grant,
        )?;
        let replacement = crate::events::processing::process_token_creation_for_token_with_event(
            game,
            controller_id,
            base_count,
            Some(token_preview.clone()),
            ctx.cause.clone(),
            &mut ctx.decision_maker,
        );
        let count = (replacement.count as usize).min(remaining_token_slots(game, controller_id));

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count);

        for _ in 0..count {
            let id = game.new_object_id();
            let mut token = build_token_copy_object(
                self,
                id,
                controller_id,
                target_object.as_ref(),
                copy_snapshot,
                resolved_target_id,
                half_power,
                half_toughness,
                &static_abilities_to_grant,
            )?;
            token.zone = Zone::Command;
            let token_is_creature = token.is_creature();

            game.add_object(token);
            let Some(entry_result) = game.move_object_with_etb_processing_with_dm(
                id,
                Zone::Battlefield,
                &mut ctx.decision_maker,
            ) else {
                game.remove_object(id);
                continue;
            };
            let entered_id = entry_result.new_id;
            created_ids.push(entered_id);
            let entered_battlefield = game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield);

            if entered_battlefield {
                let effective_tapped = entry_result.enters_tapped || self.enters_tapped;
                let entered_is_creature = game.current_is_creature(entered_id);
                let tracks_creature_etb = entered_is_creature || token_is_creature;
                apply_token_battlefield_entry(
                    game,
                    ctx,
                    entered_id,
                    controller_id,
                    tracks_creature_etb,
                    entry_options,
                    Zone::Command,
                    effective_tapped,
                    &mut events,
                )?;

                if let Some(attack_player) = configured_attack_player {
                    let targets = attack_targets_for_player(game, attack_player);
                    if !targets.is_empty() {
                        if let Some(chosen_target) =
                            choose_attack_target(game, ctx, attack_player, &targets)
                        {
                            if let Some(combat) = game.combat.as_mut() {
                                combat.attackers.push(AttackerInfo {
                                    creature: entered_id,
                                    target: chosen_target,
                                });
                            }
                        }
                    }
                }

                schedule_token_cleanup(
                    game,
                    ctx,
                    entered_id,
                    controller_id,
                    cleanup_options.clone(),
                )?;
            }
        }

        let primary_created_count = created_ids.len() as u32;
        if primary_created_count > 0 {
            game.queue_trigger_event(
                ctx.provenance,
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CreateTokensEvent::with_token_cause(
                        controller_id,
                        primary_created_count,
                        token_preview,
                        ctx.cause.clone(),
                    ),
                    ctx.provenance,
                ),
            );
        }

        let additional_ids = create_replacement_additional_tokens(
            game,
            ctx,
            controller_id,
            &replacement.additional_tokens,
            &mut events,
        )?;
        created_ids.extend(additional_ids);

        Ok(EffectOutcome::with_objects(created_ids).with_events(events))
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
    use crate::cards::{CardDefinition, CardDefinitionBuilder};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::object::ObjectKind;
    use crate::snapshot::ObjectSnapshot;
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::tag::TagKey;
    use crate::target::{ChooseSpecSurfaceHint, ObjectFilter, SacrificedObjectKind};
    use crate::test_prelude::*;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

    fn treasure_token_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Treasure")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build()
    }

    fn fancy_treasure_token_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Fancy Treasure")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Treasure])
            .build()
    }

    fn clue_token_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Clue")
            .token()
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Clue])
            .build()
    }

    fn xorn_definition() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Xorn")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elemental])
            .parse_text(
                "If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.",
            )
            .expect("Xorn should parse strictly")
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
    fn sacrificed_copy_source_uses_cost_time_lki_after_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Battlefield Form", alice);
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(creature_id).expect("sacrifice source exists"),
            &game,
        );
        let graveyard_id = game
            .move_object(
                creature_id,
                Zone::Graveyard,
                crate::events::EventCause::effect(),
            )
            .expect("sacrifice source moves");
        game.object_mut(graveyard_id)
            .expect("moved card exists")
            .name = "Graveyard Form".into();

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("sacrifice_cost_0", vec![snapshot]);
        let target = ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
            ChooseSpecSurfaceHint::SacrificedObject(SacrificedObjectKind::Creature),
        );

        let result = CreateTokenCopyEffect::one(target)
            .execute(&mut game, &mut ctx)
            .expect("copy sacrificed creature from LKI");
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected copied token");
        };
        assert_eq!(
            game.object(ids[0]).expect("token exists").name,
            "Battlefield Form"
        );
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
    fn test_create_token_copy_can_clear_mana_cost_and_add_embalm_modifiers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(100), "Angel of Sanctions")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![crate::types::Subtype::Angel])
            .power_toughness(PowerToughness::fixed(3, 4))
            .build();
        let source_id = game.new_object_id();
        let source = Object::from_card(source_id, &source_card, alice, Zone::Graveyard);
        game.add_object(source);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        let effect = CreateTokenCopyEffect::new(ChooseSpec::Source, 1, PlayerFilter::You)
            .set_colors(crate::color::ColorSet::WHITE)
            .added_subtype(crate::types::Subtype::Zombie)
            .without_mana_cost();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(token.name, "Angel of Sanctions");
        assert_eq!(token.mana_cost, None);
        assert_eq!(token.colors(), crate::color::ColorSet::WHITE);
        assert!(token.subtypes.contains(&crate::types::Subtype::Angel));
        assert!(token.subtypes.contains(&crate::types::Subtype::Zombie));
    }

    #[test]
    fn test_create_token_copy_with_haste_is_seen_by_etb_replacements() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Swift Probe", alice);
        let source = game.new_object_id();

        let haste_matters = CardDefinitionBuilder::new(CardId::new(), "Haste Matters")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::enters_with_counters_for_filter(
                    ObjectFilter::creature().with_static_ability(StaticAbilityId::Haste),
                    CounterType::Vigilance,
                    1,
                ),
            ))
            .build();
        game.create_object_from_definition(&haste_matters, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = CreateTokenCopyEffect::with_haste(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(
            token.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "ETB replacement effects should see the token's granted haste while it is being created"
        );
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
    fn create_token_copy_replacement_doubles_token_copies_created_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();
        let doubler = CardDefinitionBuilder::new(CardId::new(), "Token Doubler")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_token_creation_replacement(
                    PlayerFilter::You,
                    "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.".to_string(),
                ),
            ))
            .build();
        game.create_object_from_definition(&doubler, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = CreateTokenCopyEffect::one(ChooseSpec::creature())
            .execute(&mut game, &mut ctx)
            .unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2);
        assert!(ids.iter().all(|id| {
            game.object(*id).is_some_and(|token| {
                token.name == "Grizzly Bears" && token.kind == ObjectKind::Token
            })
        }));
    }

    #[test]
    fn xorn_adds_one_token_when_copying_a_treasure_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let treasure_id = game.create_object_from_definition(
            &fancy_treasure_token_definition(),
            alice,
            Zone::Battlefield,
        );
        let source = game.new_object_id();
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(treasure_id)]);
        let effect = CreateTokenCopyEffect::new(
            ChooseSpec::Object(ObjectFilter::artifact().with_subtype(Subtype::Treasure)),
            1,
            PlayerFilter::You,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2, "Xorn should add one Treasure token");
        let copied_count = ids
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|token| token.name == "Fancy Treasure")
            })
            .count();
        let normal_count = ids
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|token| token.name == "Treasure")
            })
            .count();
        assert_eq!(
            copied_count, 1,
            "the original token copy should be preserved"
        );
        assert_eq!(normal_count, 1, "Xorn should add one normal Treasure token");
        assert!(ids.iter().all(|id| {
            game.object(*id).is_some_and(|token| {
                token.kind == ObjectKind::Token
                    && token.subtypes.contains(&Subtype::Treasure)
                    && game.controller_of(token) == alice
            })
        }));
    }

    #[test]
    fn xorn_does_not_add_tokens_when_copying_non_treasure_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let clue_id =
            game.create_object_from_definition(&clue_token_definition(), alice, Zone::Battlefield);
        let source = game.new_object_id();
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(clue_id)]);
        let effect = CreateTokenCopyEffect::new(
            ChooseSpec::Object(ObjectFilter::artifact().with_subtype(Subtype::Clue)),
            1,
            PlayerFilter::You,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1, "Xorn should ignore non-Treasure token copies");
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(token.name, "Clue");
        assert!(token.subtypes.contains(&Subtype::Clue));
    }

    #[test]
    fn xorn_does_not_add_tokens_to_other_players_treasure_token_copies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let treasure_id = game.create_object_from_definition(
            &treasure_token_definition(),
            bob,
            Zone::Battlefield,
        );
        let source = game.new_object_id();
        game.create_object_from_definition(&xorn_definition(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(treasure_id)]);
        let effect = CreateTokenCopyEffect::new(
            ChooseSpec::Object(ObjectFilter::artifact().with_subtype(Subtype::Treasure)),
            1,
            PlayerFilter::Specific(bob),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(
            ids.len(),
            1,
            "Xorn should only affect its controller's Treasure token copies"
        );
        let token = game.object(ids[0]).expect("token should exist");
        assert_eq!(token.name, "Treasure");
        assert_eq!(game.controller_of(token), bob);
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
            assert_eq!(game.effect_store.delayed_triggers.len(), 1);
            let delayed = &game.effect_store.delayed_triggers[0];
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

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
        ctx.iteration.iterated_player = Some(charlie);

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature())
            .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
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
        use crate::effects::execute_effect;
        use crate::events::phase::EndOfCombatEvent;
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
            outcome.something_happened(),
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
                AttackTarget::Planeswalker(_) | AttackTarget::Battle(_) => None,
            })
            .collect();
        attacked_players.sort();
        assert_eq!(attacked_players, vec![charlie, dana]);

        let token_ids: Vec<_> = token_attackers.iter().map(|(id, _)| *id).collect();
        let cleanup_trigger_count = game
            .effect_store
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
        let event = TriggerEvent::new_with_provenance(EndOfCombatEvent::new(), ctx.provenance);
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

    #[test]
    fn test_create_token_copy_uses_source_snapshot_after_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Offspring Source", alice);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source exists"),
            &game,
        );

        let moved_id = game
            .move_object_by_effect(source, Zone::Graveyard)
            .expect("source should move to graveyard");
        assert_ne!(
            moved_id, source,
            "zone change should create a new object id"
        );

        let mut ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(source_snapshot);
        let effect = CreateTokenCopyEffect::one(ChooseSpec::Source);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).expect("token should exist");
            assert_eq!(token.name, "Offspring Source");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_copy_uses_target_snapshot_after_zone_change() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Returned Copy Target", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);
        ctx.snapshot_targets(&game);

        let moved_id = game
            .move_object_by_effect(creature_id, Zone::Hand)
            .expect("target should move to hand");
        assert_ne!(
            moved_id, creature_id,
            "zone change should create a new object id"
        );

        let effect = CreateTokenCopyEffect::one(ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).expect("token should exist");
            assert_eq!(token.name, "Returned Copy Target");
            assert_eq!(token.kind, ObjectKind::Token);
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        } else {
            panic!("Expected Objects result");
        }
    }
}
