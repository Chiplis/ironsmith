//! CR audit (in-review completion pass, package q09): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{DecisionMaker, EffectExecutor, PlayerFilter, SelectOptionsContext};
use ironsmith::{EffectContext, GameState, PlayerId};

fn q09_complete_priority_action(
    game: &mut GameState,
    player: PlayerId,
    action: ironsmith::LegalAction,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        ironsmith::LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    assert_eq!(game.turn.priority_player, Some(player));
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if let Some(pending) = &state.pending_method_selection {
            let index = pending
                .available_methods
                .iter()
                .position(|choice| Some(&choice.method) == requested_method.as_ref())
                .ok_or_else(|| {
                    "requested casting method lost during method selection".to_string()
                })?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                &mut queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                &mut dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, &mut queue, &mut state, &context, &mut dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    if state.pending_cast.is_some()
        || state.pending_method_selection.is_some()
        || state.pending_activation.is_some()
    {
        return Err("native action exceeded decision bound".into());
    }
    Ok(())
}

fn q09_observer(game: &mut GameState, owner: PlayerId) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let def = CardDefinitionBuilder::new(ironsmith::CardId::new(), "q09 observer")
        .card_types(vec![ironsmith::CardType::Artifact])
        .build();
    game.create_object_from_definition(&def, owner, ironsmith::Zone::Battlefield)
}

fn q09_resolve_ability(
    game: &mut GameState,
    source: ironsmith::ObjectId,
    controller: PlayerId,
    effects: Vec<ironsmith::Effect>,
) {
    game.stack
        .push(ironsmith::StackEntry::ability(source, controller, effects));
    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
}

/// Resolve "you gain life equal to the mana value of <object>" and return the gain.
fn q09_resolved_mana_value(
    game: &mut GameState,
    source: ironsmith::ObjectId,
    controller: PlayerId,
    subject: ironsmith::ObjectId,
) -> i32 {
    let before = game.player(controller).unwrap().life;
    q09_resolve_ability(
        game,
        source,
        controller,
        vec![ironsmith::Effect::gain_life(ironsmith::Value::ManaValueOf(
            Box::new(ironsmith::ChooseSpec::SpecificObject(subject)),
        ))],
    );
    game.player(controller).unwrap().life - before
}

fn q09_snapshot_mv(game: &GameState, id: ironsmith::ObjectId) -> u32 {
    ironsmith::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(id).unwrap(),
        game,
    )
    .mana_value()
}

#[test]
fn cr_709_3b_split_half_spell_has_only_the_cast_halfs_characteristics() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{
        Ability, CardId, CardType, ColorSet, Effect, LegalAction, ManaCost, ManaSymbol, Phase,
        StaticAbility, Subtype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut mismatches = Vec::new();
    for right_half in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        let left_id = CardId::new();
        let right_id = CardId::new();
        let left = CardDefinitionBuilder::new(left_id, "Red Left Half")
            .card_types(vec![CardType::Instant])
            .subtypes(vec![Subtype::Arcane])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
            .with_ability(Ability::static_ability(StaticAbility::flash()))
            .with_spell_effect(vec![Effect::gain_life(3)])
            .other_face(right_id)
            .other_face_name("Blue Right Half")
            .linked_face_layout(LinkedFaceLayout::Split)
            .build();
        let right = CardDefinitionBuilder::new(right_id, "Blue Right Half")
            .card_types(vec![CardType::Sorcery])
            .subtypes(vec![Subtype::Lesson])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Blue,
            ]))
            .with_ability(Ability::static_ability(StaticAbility::haste()))
            .with_spell_effect(vec![Effect::gain_life(7)])
            .other_face(left_id)
            .other_face_name("Red Left Half")
            .linked_face_layout(LinkedFaceLayout::Split)
            .build();
        game.register_linked_face_definition(&left);
        game.register_linked_face_definition(&right);
        let card = game.create_object_from_definition(&left, alice, Zone::Hand);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);
        q09_complete_priority_action(
            &mut game,
            alice,
            LegalAction::CastSpell {
                spell_id: card,
                from_zone: Zone::Hand,
                casting_method: if right_half {
                    CastingMethod::SplitOtherHalf
                } else {
                    CastingMethod::Normal
                },
            },
        )
        .unwrap();
        assert_eq!(game.stack.len(), 1);
        let spell = game.stack[0].object_id;
        let chars = game.current_characteristics(spell).unwrap();
        let flash = chars
            .abilities
            .iter()
            .filter(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_flash()))
            .count();
        let haste = chars
            .abilities
            .iter()
            .filter(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_haste()))
            .count();
        let (name, types, subtypes, colors, mv, abil) = if right_half {
            (
                "Blue Right Half",
                vec![CardType::Sorcery],
                vec![Subtype::Lesson],
                ColorSet::BLUE,
                3,
                (0, 1),
            )
        } else {
            (
                "Red Left Half",
                vec![CardType::Instant],
                vec![Subtype::Arcane],
                ColorSet::RED,
                1,
                (1, 0),
            )
        };
        let observed_mv = q09_snapshot_mv(&game, spell);
        if chars.name != name
            || chars.card_types.as_slice() != types.as_slice()
            || chars.subtypes.as_slice() != subtypes.as_slice()
            || chars.colors != colors
            || observed_mv != mv
            || (flash, haste) != abil
        {
            mismatches.push(format!(
                "right_half={right_half}: name={:?} types={:?} subtypes={:?} colors={:?} mv={observed_mv} flash/haste={:?}; required {name} {types:?} {subtypes:?} {colors:?} {mv} {abil:?}",
                chars.name, chars.card_types, chars.subtypes, chars.colors, (flash, haste)
            ));
        }
        let life = game.player(alice).unwrap().life;
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let gain = game.player(alice).unwrap().life - life;
        let required = if right_half { 7 } else { 3 };
        if gain != required {
            mismatches.push(format!("right_half={right_half}: resolved gain {gain}, required only the cast half's {required}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 709.3b: only the cast half's characteristics exist on the stack:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_709_4d_fused_split_spell_has_both_halves_characteristics_combined() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{
        CardId, CardType, ColorSet, Effect, LegalAction, ManaCost, ManaSymbol, Phase, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    let left_id = CardId::new();
    let right_id = CardId::new();
    let left = CardDefinitionBuilder::new(left_id, "Fused Left")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .with_spell_effect(vec![Effect::gain_life(3)])
        .other_face(right_id)
        .other_face_name("Fused Right")
        .linked_face_layout(LinkedFaceLayout::Split)
        .has_fuse()
        .build();
    let right = CardDefinitionBuilder::new(right_id, "Fused Right")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Blue,
        ]))
        .with_spell_effect(vec![Effect::gain_life(4)])
        .other_face(left_id)
        .other_face_name("Fused Left")
        .linked_face_layout(LinkedFaceLayout::Split)
        .has_fuse()
        .build();
    game.register_linked_face_definition(&left);
    game.register_linked_face_definition(&right);
    let card = game.create_object_from_definition(&left, alice, Zone::Hand);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    q09_complete_priority_action(
        &mut game,
        alice,
        LegalAction::CastSpell {
            spell_id: card,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Fuse,
        },
    )
    .unwrap();
    assert_eq!(game.stack.len(), 1);
    let spell = game.stack[0].object_id;
    let chars = game.current_characteristics(spell).unwrap();
    let mv = q09_snapshot_mv(&game, spell);
    let observer = q09_observer(&mut game, alice);
    let resolved_mv = q09_resolved_mana_value(&mut game, observer, alice, spell);
    let mut mismatches = Vec::new();
    if !chars.card_types.contains(&CardType::Instant)
        || !chars.card_types.contains(&CardType::Sorcery)
    {
        mismatches.push(format!("types={:?}", chars.card_types));
    }
    if chars.colors != ColorSet::RED.union(ColorSet::BLUE) {
        mismatches.push(format!("colors={:?}", chars.colors));
    }
    if mv != 3 || resolved_mv != 3 {
        mismatches.push(format!(
            "mana value snapshot={mv} resolved={resolved_mv}, required 3"
        ));
    }
    if !(chars.name.contains("Fused Left") && chars.name.contains("Fused Right")) {
        mismatches.push(format!("name={:?}", chars.name));
    }
    let life = game.player(alice).unwrap().life;
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    if game.player(alice).unwrap().life - life != 7 {
        mismatches.push(format!(
            "resolved gain {}",
            game.player(alice).unwrap().life - life
        ));
    }
    assert!(
        mismatches.is_empty(),
        "CR 709.4d: a fused split spell has both halves' characteristics combined:\n{}",
        mismatches.join("\n")
    );
}

struct Q09LockNamed(String);
impl DecisionMaker for Q09LockNamed {
    fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let wanted = format!("Lock {}", self.0);
        vec![
            ctx.options
                .iter()
                .find(|o| o.legal && o.description == wanted)
                .or_else(|| ctx.options.iter().find(|o| o.legal))
                .unwrap()
                .index,
        ]
    }
}

#[test]
fn cr_709_5a_d_g_h_cast_room_half_enters_unlocked_triggers_and_lock_removes_one_designation() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{PutOntoBattlefieldEffect, UnlockRoomDoorEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::special_actions::{RoomDoor, SpecialAction};
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, LegalAction, ManaCost, ManaSymbol,
        ObjectFilter, Phase, Subtype, Zone,
    };
    let mut mismatches = Vec::new();
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for cast_right in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = actor;
            game.turn.priority_player = Some(actor);
            game.turn.phase = Phase::FirstMain;
            let left_id = CardId::new();
            let right_id = CardId::new();
            let left = CardDefinitionBuilder::new(left_id, "Left q09 door")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Room])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
                .other_face(right_id)
                .other_face_name("Right q09 door")
                .linked_face_layout(LinkedFaceLayout::Split)
                .with_ability(Ability::triggered(
                    Trigger::keyword_action_from_source(
                        KeywordActionKind::UnlockDoor,
                        PlayerFilter::You,
                    ),
                    vec![Effect::energy_counters_player(1, PlayerFilter::You)],
                ))
                .with_ability(Ability::triggered(
                    Trigger::keyword_action_from_source(
                        KeywordActionKind::FullyUnlockRoom,
                        PlayerFilter::You,
                    ),
                    vec![Effect::energy_counters_player(100, PlayerFilter::You)],
                ))
                .build();
            let right = CardDefinitionBuilder::new(right_id, "Right q09 door")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Room])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
                .other_face(left_id)
                .other_face_name("Left q09 door")
                .linked_face_layout(LinkedFaceLayout::Split)
                .with_ability(Ability::triggered(
                    Trigger::keyword_action_from_source(
                        KeywordActionKind::UnlockDoor,
                        PlayerFilter::You,
                    ),
                    vec![Effect::energy_counters_player(10, PlayerFilter::You)],
                ))
                .build();
            game.register_linked_face_definition(&left);
            game.register_linked_face_definition(&right);
            let observer = q09_observer(&mut game, actor);

            // Non-cast entry: neither designation, but the shared type line persists (709.5a/d).
            let other = game.create_object_from_definition(&left, actor, Zone::Graveyard);
            let other_stable = game.object(other).unwrap().stable_id;
            q09_resolve_ability(
                &mut game,
                observer,
                actor,
                vec![Effect::new(PutOntoBattlefieldEffect::you_control(
                    ChooseSpec::SpecificObject(other),
                    false,
                ))],
            );
            let other = game.find_object_by_stable_id(other_stable).unwrap();
            if !game.room_has_no_unlocked_door(other) {
                mismatches.push(format!(
                    "actor={actor:?}: noncast entry must have neither unlocked designation (709.5d)"
                ));
            }
            let locked = game.current_characteristics(other).unwrap();
            if !locked.card_types.contains(&CardType::Enchantment)
                || !locked.subtypes.contains(&Subtype::Room)
            {
                mismatches.push(format!(
                    "actor={actor:?}: fully locked Room lost shared type line: {:?}/{:?} (709.5a)",
                    locked.card_types, locked.subtypes
                ));
            }
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            if !queue.entries.is_empty() {
                mismatches.push(format!(
                    "actor={actor:?}: noncast entry queued {} unlock triggers",
                    queue.entries.len()
                ));
            }
            queue.entries.clear();

            let card = game.create_object_from_definition(&left, actor, Zone::Hand);
            let stable = game.object(card).unwrap().stable_id;
            game.player_mut(actor)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Red, 1);
            game.player_mut(actor)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 1);
            q09_complete_priority_action(
                &mut game,
                actor,
                LegalAction::CastSpell {
                    spell_id: card,
                    from_zone: Zone::Hand,
                    casting_method: if cast_right {
                        CastingMethod::SplitOtherHalf
                    } else {
                        CastingMethod::Normal
                    },
                },
            )
            .unwrap();
            assert_eq!(game.stack.len(), 1);
            let spell = game.stack[0].object_id;
            let spell_chars = game.current_characteristics(spell).unwrap();
            if !spell_chars.card_types.contains(&CardType::Enchantment)
                || !spell_chars.subtypes.contains(&Subtype::Room)
            {
                mismatches.push(format!(
                    "actor={actor:?} right={cast_right}: spell half lacks shared type line (709.5a)"
                ));
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let room = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(room).unwrap().zone, Zone::Battlefield);
            let (cast_name, other_name) = if cast_right {
                ("Right q09 door", "Left q09 door")
            } else {
                ("Left q09 door", "Right q09 door")
            };
            let chars = game.current_characteristics(room).unwrap();
            if game.room_has_no_unlocked_door(room)
                || game.is_room_fully_unlocked(room)
                || chars.name != cast_name
                || ironsmith::special_actions::locked_room_doors(&game, room).len() != 1
            {
                mismatches.push(format!(
                    "actor={actor:?} right={cast_right}: cast entry must unlock exactly the cast half: none={}, full={}, name={:?} (709.5d)",
                    game.room_has_no_unlocked_door(room), game.is_room_fully_unlocked(room), chars.name
                ));
            }
            if !chars.card_types.contains(&CardType::Enchantment)
                || !chars.subtypes.contains(&Subtype::Room)
            {
                mismatches.push(format!(
                    "actor={actor:?} right={cast_right}: permanent lacks shared type line (709.5a)"
                ));
            }
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            let entry_triggers = queue.entries.len();
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            let energy = game.player(actor).unwrap().energy_counters;
            let required = if cast_right { 10 } else { 1 };
            if entry_triggers != 1 || energy != required {
                mismatches.push(format!(
                    "actor={actor:?} right={cast_right}: entering with the unlocked designation must trigger that door's unlock ability once: queued={entry_triggers}, energy={energy}, required {required} (709.5h)"
                ));
            }
            // Unlock the other half: other door's unlock trigger + fully unlocked trigger.
            ironsmith::special_actions::perform(
                SpecialAction::UnlockRoomDoor {
                    room_id: room,
                    door: RoomDoor::Linked,
                },
                &mut game,
                actor,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            assert!(game.is_room_fully_unlocked(room));
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            let energy = game.player(actor).unwrap().energy_counters;
            if energy != 111 {
                mismatches.push(format!("actor={actor:?} right={cast_right}: after unlocking the other door energy={energy}, required 111 (709.5h/i)"));
            }
            // Lock the half that was cast (709.5g).
            let mut filter = ObjectFilter::default();
            filter.specific = Some(room);
            game.stack.push(ironsmith::StackEntry::ability(
                observer,
                actor,
                vec![Effect::new(
                    UnlockRoomDoorEffect::new(PlayerFilter::You, filter).with_allow_lock(true),
                )],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut Q09LockNamed(cast_name.to_string()),
            )
            .unwrap();
            let after = game.current_characteristics(room).unwrap();
            if game.is_room_fully_unlocked(room)
                || game.room_has_no_unlocked_door(room)
                || after.name != other_name
            {
                mismatches.push(format!(
                    "actor={actor:?} right={cast_right}: locking the chosen unlocked half must remove only that designation: full={}, none={}, name={:?}, required {other_name} (709.5g)",
                    game.is_room_fully_unlocked(room), game.room_has_no_unlocked_door(room), after.name
                ));
            }
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            if !queue.entries.is_empty() {
                mismatches.push(format!(
                    "actor={actor:?} right={cast_right}: locking queued {} unlock triggers",
                    queue.entries.len()
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 709.5a/d/g/h: Room shared type line, cast-entry designation, unlock triggers and locking:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_710_1_1a_4_flip_card_uses_top_half_until_flipped_and_flipping_is_one_way() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{FlipEffect, PutOntoBattlefieldEffect, ReturnToHandEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, ManaCost, ManaSymbol, PowerToughness,
        StaticAbility, Subtype, Supertype, Zone,
    };
    let mut mismatches = Vec::new();
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Normal top half")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wizard])
            .power_toughness(PowerToughness::fixed(1, 2))
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Blue,
            ]))
            .other_face(back_id)
            .other_face_name("Upside-down alternative half")
            .linked_face_layout(LinkedFaceLayout::Flip)
            .with_ability(Ability::static_ability(StaticAbility::vigilance()))
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Upside-down alternative half")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .supertypes(vec![Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(4, 5))
            .other_face(front_id)
            .other_face_name("Normal top half")
            .linked_face_layout(LinkedFaceLayout::Flip)
            .with_ability(Ability::static_ability(StaticAbility::flying()))
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let observer = q09_observer(&mut game, actor);
        let card = game.create_object_from_definition(&front, actor, Zone::Battlefield);
        let stable = game.object(card).unwrap().stable_id;
        let has = |game: &GameState, id, flying: bool| {
            game.current_characteristics(id).unwrap().abilities.iter().any(|a| {
                matches!(&a.kind, AbilityKind::Static(s) if if flying { s.has_flying() } else { s.has_vigilance() })
            })
        };
        let normal = |game: &GameState, id| {
            let c = game.current_characteristics(id).unwrap();
            c.name == "Normal top half"
                && (c.power, c.toughness) == (Some(1), Some(2))
                && c.subtypes.contains(&Subtype::Wizard)
                && !c.supertypes.contains(&Supertype::Legendary)
                && has(game, id, false)
                && !has(game, id, true)
        };
        let flipped = |game: &GameState, id| {
            let c = game.current_characteristics(id).unwrap();
            c.name == "Upside-down alternative half"
                && (c.power, c.toughness) == (Some(4), Some(5))
                && c.subtypes.contains(&Subtype::Spirit)
                && c.supertypes.contains(&Supertype::Legendary)
                && has(game, id, true)
                && !has(game, id, false)
        };
        if game.is_flipped(card) || !normal(&game, card) {
            mismatches.push(format!(
                "actor={actor:?}: unflipped permanent must use only the top half (710.1a)"
            ));
        }
        for attempt in 1..=2 {
            q09_resolve_ability(
                &mut game,
                observer,
                actor,
                vec![Effect::new(FlipEffect::new(ChooseSpec::SpecificObject(
                    card,
                )))],
            );
            if !game.is_flipped(card) || !flipped(&game, card) {
                mismatches.push(format!("actor={actor:?} attempt={attempt}: flip must leave the permanent flipped with alternative characteristics (710.1/710.4)"));
            }
        }
        q09_resolve_ability(
            &mut game,
            observer,
            actor,
            vec![Effect::new(ReturnToHandEffect::with_spec(
                ChooseSpec::SpecificObject(card),
            ))],
        );
        let in_hand = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(in_hand).unwrap().zone, Zone::Hand);
        if game.is_flipped(in_hand)
            || game.current_characteristics(in_hand).unwrap().name != "Normal top half"
        {
            mismatches.push(format!("actor={actor:?}: card in hand must have normal characteristics and no flipped status (710.4)"));
        }
        q09_resolve_ability(
            &mut game,
            observer,
            actor,
            vec![Effect::new(PutOntoBattlefieldEffect::you_control(
                ChooseSpec::SpecificObject(in_hand),
                false,
            ))],
        );
        let back_on = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(back_on).unwrap().zone, Zone::Battlefield);
        if game.is_flipped(back_on) || !normal(&game, back_on) {
            mismatches.push(format!("actor={actor:?}: returning permanent must retain no memory of flipped status (710.4)"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 710.1/710.1a/710.4:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_711_1_leveler_card_has_two_level_symbols_in_one_text_box() {
    let student = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Student of Warfare",
        "Mana cost: {W}\nType: Creature — Human Knight\nFirst printed set: Rise of the Eldrazi\nPower/Toughness: 1/1\nLevel up {W} ({W}: Put a level counter on this. Level up only as a sorcery.)\nLEVEL 2-6\n3/3\nFirst strike\nLEVEL 7+\n4/4\nDouble strike",
        false,
    )
    .expect("strict compiler accepts Student of Warfare");
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&student));
    let debug = format!("{student:?}");
    let tiers = debug.matches("min_level:").count();
    assert_eq!(
        tiers, 2,
        "CR 711.1: a leveler has exactly two level symbols; found {tiers}"
    );
    assert!(
        debug.contains("min_level: 2, max_level: Some(6)")
            && debug.contains("min_level: 7, max_level: None"),
        "CR 711.1/711.2: the two level symbols are LEVEL 2-6 and LEVEL 7+"
    );
}

#[test]
fn cr_712_4c_unmelded_meld_cards_ignore_transform_and_convert_instructions() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{ConvertEffect, TransformEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, ColorSet, Effect, ManaCost, ManaSymbol,
        PowerToughness, StaticAbility, Zone,
    };
    let mut mismatches = Vec::new();
    for (front_name, partner, result) in [
        (
            "Bruna, the Fading Light",
            "Gisela, the Broken Blade",
            "Brisela, Voice of Nightmares",
        ),
        ("Graf Rats", "Midnight Scavengers", "Chittering Host"),
    ] {
        assert_eq!(
            ironsmith::cards::meld_counterpart_name(front_name),
            Some(partner)
        );
        for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
            for convert in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let front = CardDefinitionBuilder::new(CardId::new(), front_name)
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(5, 7))
                    .mana_cost(ManaCost::from_symbols(vec![
                        ManaSymbol::Generic(5),
                        ManaSymbol::White,
                    ]))
                    .with_ability(Ability::static_ability(StaticAbility::vigilance()))
                    .build();
                let combined = CardDefinitionBuilder::new(CardId::new(), result)
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(9, 10))
                    .color_indicator(ColorSet::BLACK)
                    .build();
                game.register_linked_face_definition(&front);
                game.register_linked_face_definition(&combined);
                let observer = q09_observer(&mut game, actor);
                let subject = game.create_object_from_definition(&front, actor, Zone::Battlefield);
                for attempt in 1..=2 {
                    let effect = if convert {
                        Effect::new(ConvertEffect::new(ChooseSpec::SpecificObject(subject)))
                    } else {
                        Effect::new(TransformEffect::new(ChooseSpec::SpecificObject(subject)))
                    };
                    q09_resolve_ability(&mut game, observer, actor, vec![effect]);
                    let chars = game.current_characteristics(subject).unwrap();
                    if game.transform_count(subject) != 0
                        || chars.name != front_name
                        || (chars.power, chars.toughness) != (Some(5), Some(7))
                    {
                        mismatches.push(format!("{front_name} actor={actor:?} convert={convert} attempt={attempt}: count={} name={:?}", game.transform_count(subject), chars.name));
                    }
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 712.4c/712.9: meld cards can't be transformed or converted:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_712_8c_8e_13_disturbed_spell_is_back_face_up_with_front_mana_value_and_back_face_copies_have_zero()
 {
    use ironsmith::ability::AbilityKind;
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::CreateTokenCopyEffect;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, ColorSet, Effect, LegalAction, ManaCost, ManaSymbol,
        Phase, PowerToughness, StaticAbility, Subtype, Zone,
    };
    let mut mismatches = Vec::new();
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = actor;
        game.turn.priority_player = Some(actor);
        game.turn.phase = Phase::FirstMain;
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front = CardDefinitionBuilder::new(front_id, "Disturb front soldier")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(3, 3))
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::White,
            ]))
            .disturb(ManaCost::from_symbols(vec![ManaSymbol::White]))
            .other_face(back_id)
            .other_face_name("Disturbed back spirit")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Disturbed back spirit")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .power_toughness(PowerToughness::fixed(1, 1))
            .color_indicator(ColorSet::BLUE)
            .with_ability(Ability::static_ability(StaticAbility::flying()))
            .other_face(front_id)
            .other_face_name("Disturb front soldier")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);
        let observer = q09_observer(&mut game, actor);
        let card = game.create_object_from_definition(&front, actor, Zone::Graveyard);
        let stable = game.object(card).unwrap().stable_id;
        game.player_mut(actor)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        let actions = ironsmith::compute_legal_actions(&game, actor).unwrap();
        let action = actions
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, from_zone: Zone::Graveyard, casting_method: CastingMethod::Alternative(_) } if *spell_id == card))
            .expect("disturb cast from graveyard is offered");
        q09_complete_priority_action(&mut game, actor, action).unwrap();
        assert_eq!(game.stack.len(), 1);
        let spell = game.stack[0].object_id;
        let chars = game.current_characteristics(spell).unwrap();
        let flying = chars
            .abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Static(s) if s.has_flying()));
        if chars.name != "Disturbed back spirit"
            || chars.colors != ColorSet::BLUE
            || (chars.power, chars.toughness) != (Some(1), Some(1))
            || !flying
            || chars.subtypes.contains(&Subtype::Human)
        {
            mismatches.push(format!("actor={actor:?}: transformed spell must have only back-face characteristics: name={:?} colors={:?} pt={:?} (712.8c)", chars.name, chars.colors, (chars.power, chars.toughness)));
        }
        let spell_mv = q09_resolved_mana_value(&mut game, observer, actor, spell);
        if spell_mv != 3 {
            mismatches.push(format!("actor={actor:?}: transformed spell mana value {spell_mv}, required front-face 3 (712.8c)"));
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        if game.object(permanent).unwrap().zone != Zone::Battlefield
            || game.current_characteristics(permanent).unwrap().name != "Disturbed back spirit"
        {
            mismatches.push(format!("actor={actor:?}: resolving transformed spell must enter with the back face up (712.13)"));
        }
        let permanent_mv = q09_resolved_mana_value(&mut game, observer, actor, permanent);
        if permanent_mv != 3 {
            mismatches.push(format!("actor={actor:?}: back-face-up permanent mana value {permanent_mv}, required front-face 3 (712.8e)"));
        }
        let before = game.battlefield.clone();
        q09_resolve_ability(
            &mut game,
            observer,
            actor,
            vec![Effect::new(CreateTokenCopyEffect::one(
                ChooseSpec::SpecificObject(permanent),
            ))],
        );
        let token = *game
            .battlefield
            .iter()
            .find(|id| !before.contains(id))
            .unwrap();
        let token_chars = game.current_characteristics(token).unwrap();
        let token_mv = q09_resolved_mana_value(&mut game, observer, actor, token);
        if token_chars.name != "Disturbed back spirit" || token_mv != 0 {
            mismatches.push(format!("actor={actor:?}: copy of back face name={:?} mana value {token_mv}, required 0 (712.8e)", token_chars.name));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 712.8c/712.8e/712.13:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_714_2a_chapter_symbol_roman_numerals_map_to_chapter_numbers() {
    let mut mismatches = Vec::new();
    for (name, text, expected) in [
        (
            "History of Benalia",
            "Mana cost: {1}{W}{W}\nType: Enchantment — Saga\nFirst printed set: Dominaria\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI, II — Create a 2/2 white Knight creature token with vigilance.\nIII — Knights you control get +2/+1 until end of turn.",
            vec![vec![1u32, 2], vec![3]],
        ),
        (
            "Summon: Knights of Round",
            "Mana cost: {6}{W}{W}\nType: Enchantment Creature — Saga Knight\nFirst printed set: Final Fantasy\nPower/Toughness: 3/3\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after V.)\nI, II, III, IV — Create three 2/2 white Knight creature tokens.\nV — Ultimate End — Other creatures you control get +2/+2 until end of turn. Put an indestructible counter on each of them.\nIndestructible",
            vec![vec![1, 2, 3, 4], vec![5]],
        ),
    ] {
        let definition =
            match ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false) {
                Ok(d) => d,
                Err(e) => {
                    mismatches.push(format!("{name}: strict compile failed: {e:?}"));
                    continue;
                }
            };
        let chapters = definition
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                ironsmith::ability::AbilityKind::Triggered(t) => {
                    t.trigger.saga_chapters().map(|c| c.to_vec())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if chapters != expected {
            mismatches.push(format!(
                "{name}: chapters={chapters:?}, required {expected:?}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 714.2a: Roman numeral chapter symbols map to their numbers:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_718_2_2a_3_prototype_choice_spell_permanent_and_copy_use_selected_characteristics() {
    use ironsmith::alternative_cast::AlternativeCastingMethod;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::CreateTokenCopyEffect;
    use ironsmith::{
        CardId, CardType, CastingMethod, ChooseSpec, ColorSet, Effect, LegalAction, ManaCost,
        ManaSymbol, Phase, PowerToughness, Subtype, Zone,
    };
    let mut mismatches = Vec::new();
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for prototyped in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = actor;
            game.turn.priority_player = Some(actor);
            game.turn.phase = Phase::FirstMain;
            let definition = CardDefinitionBuilder::new(CardId::new(), "q09 prototype construct")
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .subtypes(vec![Subtype::Construct])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(7)]))
                .power_toughness(PowerToughness::fixed(6, 7))
                .haste()
                .alternative_cast(AlternativeCastingMethod::prototype(
                    ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Red]),
                    PowerToughness::fixed(1, 2),
                ))
                .build();
            let card = game.create_object_from_definition(&definition, actor, Zone::Hand);
            let observer = q09_observer(&mut game, actor);
            game.player_mut(actor).unwrap().mana_pool.colorless = 7;
            game.player_mut(actor).unwrap().mana_pool.red = 1;
            let actions = ironsmith::compute_legal_actions(&game, actor).unwrap();
            let normal = LegalAction::CastSpell {
                spell_id: card,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            };
            let proto = LegalAction::CastSpell {
                spell_id: card,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            };
            if !actions.contains(&normal) || !actions.contains(&proto) {
                mismatches.push(format!(
                    "actor={actor:?}: both normal and prototyped casting must be offered (718.3)"
                ));
            }
            q09_complete_priority_action(&mut game, actor, if prototyped { proto } else { normal })
                .unwrap();
            let (pt, mv, color) = if prototyped {
                ((1, 2), 2, ColorSet::RED)
            } else {
                ((6, 7), 7, ColorSet::COLORLESS)
            };
            let check = |game: &mut GameState, id, stage: &str, mismatches: &mut Vec<String>| {
                let c = game.current_characteristics(id).unwrap();
                let resolved = q09_resolved_mana_value(game, observer, actor, id);
                let has_haste = c.abilities.iter().any(|a| matches!(&a.kind, ironsmith::ability::AbilityKind::Static(s) if s.has_haste()));
                if (c.power, c.toughness) != (Some(pt.0), Some(pt.1))
                    || resolved != mv
                    || c.colors != color
                    || !c.subtypes.contains(&Subtype::Construct)
                    || !has_haste
                {
                    mismatches.push(format!(
                        "actor={actor:?} prototyped={prototyped} {stage}: pt={:?} mv={resolved} colors={:?} haste={has_haste}; required {pt:?}/{mv}/{color:?}",
                        (c.power, c.toughness), c.colors
                    ));
                }
            };
            let spell = game.stack[0].object_id;
            let stable = game.object(spell).unwrap().stable_id;
            check(&mut game, spell, "spell", &mut mismatches);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let permanent = game.find_object_by_stable_id(stable).unwrap();
            check(&mut game, permanent, "permanent", &mut mismatches);
            let before = game.battlefield.clone();
            q09_resolve_ability(
                &mut game,
                observer,
                actor,
                vec![Effect::new(CreateTokenCopyEffect::one(
                    ChooseSpec::SpecificObject(permanent),
                ))],
            );
            let token = *game
                .battlefield
                .iter()
                .find(|id| !before.contains(id))
                .unwrap();
            check(&mut game, token, "token copy", &mut mismatches);
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 718.2/718.2a/718.3:\n{}",
        mismatches.join("\n")
    );
}

fn q09_omen_definitions() -> (
    ironsmith::cards::CardDefinition,
    ironsmith::cards::CardDefinition,
) {
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{
        Ability, CardId, CardType, Effect, ManaCost, ManaSymbol, PowerToughness, StaticAbility,
        Subtype,
    };
    let front_id = CardId::new();
    let inset_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Normal Omen creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 5))
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Green,
        ]))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()))
        .other_face(inset_id)
        .other_face_name("Inset Omen spell")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let inset = CardDefinitionBuilder::new(inset_id, "Inset Omen spell")
        .card_types(vec![CardType::Instant])
        .subtypes(vec![Subtype::Omen])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::Red,
        ]))
        .with_spell_effect(vec![Effect::gain_life(5)])
        .other_face(front_id)
        .other_face_name("Normal Omen creature")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    (front, inset)
}

#[test]
fn cr_720_2_2b_3_3a_omen_choice_legality_and_spell_characteristics_use_the_chosen_set() {
    use ironsmith::{
        CardType, CastingMethod, ChooseSpec, ColorSet, Effect, LegalAction, Phase, StackEntry,
        Subtype, Zone,
    };
    let mut mismatches = Vec::new();
    for caster in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let opponent = PlayerId::from_index(1 - caster.0);
        // window 0: caster's own main phase (both choices legal);
        // window 1: opponent's main phase (only the instant Omen is legal).
        for window in 0..2 {
            for as_omen in [false, true] {
                for copied in [false, true] {
                    if window == 1 && !as_omen && copied {
                        continue;
                    }
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = if window == 0 { caster } else { opponent };
                    game.turn.priority_player = Some(caster);
                    game.turn.phase = Phase::FirstMain;
                    let (front, inset) = q09_omen_definitions();
                    game.register_linked_face_definition(&front);
                    game.register_linked_face_definition(&inset);
                    let card = game.create_object_from_definition(&front, caster, Zone::Hand);
                    let observer = q09_observer(&mut game, caster);
                    game.player_mut(caster).unwrap().mana_pool.colorless = 2;
                    game.player_mut(caster).unwrap().mana_pool.red = 1;
                    game.player_mut(caster).unwrap().mana_pool.green = 1;
                    let actions = ironsmith::compute_legal_actions(&game, caster).unwrap();
                    let normal = LegalAction::CastSpell {
                        spell_id: card,
                        from_zone: Zone::Hand,
                        casting_method: CastingMethod::Normal,
                    };
                    let omen = LegalAction::CastSpell {
                        spell_id: card,
                        from_zone: Zone::Hand,
                        casting_method: CastingMethod::SplitOtherHalf,
                    };
                    let (normal_legal, omen_legal) = (window == 0, true);
                    if actions.contains(&normal) != normal_legal
                        || actions.contains(&omen) != omen_legal
                    {
                        mismatches.push(format!(
                            "caster={caster:?} window={window}: offered normal={} omen={}, required {normal_legal}/{omen_legal} (720.3/720.3a)",
                            actions.contains(&normal), actions.contains(&omen)
                        ));
                    }
                    if window == 1 && !as_omen {
                        let result = q09_complete_priority_action(&mut game, caster, normal);
                        if !game.stack.is_empty() {
                            mismatches.push(format!("caster={caster:?}: normal creature cast completed during opponent's turn ({result:?}) (720.3a)"));
                        }
                        continue;
                    }
                    q09_complete_priority_action(
                        &mut game,
                        caster,
                        if as_omen { omen } else { normal },
                    )
                    .unwrap();
                    assert_eq!(game.stack.len(), 1);
                    let mut subject = game.stack[0].object_id;
                    if copied {
                        game.stack.push(StackEntry::ability(
                            observer,
                            caster,
                            vec![Effect::copy_spell(ChooseSpec::SpecificObject(subject))],
                        ));
                        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                        assert_eq!(game.stack.len(), 2);
                        subject = game.stack.last().unwrap().object_id;
                    }
                    let c = game.current_characteristics(subject).unwrap();
                    let mv = q09_snapshot_mv(&game, subject);
                    let ok = if as_omen {
                        c.name == "Inset Omen spell"
                            && c.card_types.as_slice() == [CardType::Instant]
                            && c.subtypes.contains(&Subtype::Omen)
                            && c.colors == ColorSet::RED
                            && mv == 2
                            && c.power.is_none()
                    } else {
                        c.name == "Normal Omen creature"
                            && c.card_types.as_slice() == [CardType::Creature]
                            && c.colors == ColorSet::GREEN
                            && mv == 3
                            && (c.power, c.toughness) == (Some(4), Some(5))
                    };
                    if !ok {
                        mismatches.push(format!(
                            "caster={caster:?} window={window} omen={as_omen} copied={copied}: name={:?} types={:?} colors={:?} mv={mv} pt={:?} (720.2/720.2b/720.3)",
                            c.name, c.card_types, c.colors, (c.power, c.toughness)
                        ));
                    }
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 720.2/720.2b/720.3/720.3a:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_720_2c_drawing_an_omen_card_draws_one_card() {
    use ironsmith::Zone;
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (front, inset) = q09_omen_definitions();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&inset);
        let _ = game.create_object_from_definition(&front, actor, Zone::Library);
        let hand_before = game.player(actor).unwrap().hand.len();
        let library_before = game.player(actor).unwrap().library.len();
        let drawn = game.draw_cards(actor, 1);
        assert_eq!(
            drawn.len(),
            1,
            "CR 720.2c: one omen card is drawn as one card"
        );
        assert_eq!(
            game.player(actor).unwrap().hand.len(),
            hand_before + 1,
            "CR 720.2c: hand grows by one card"
        );
        assert_eq!(
            game.player(actor).unwrap().library.len(),
            library_before - 1,
            "CR 720.2c: library shrinks by one card"
        );
        assert_eq!(
            game.current_name(drawn[0]).as_deref(),
            Some("Normal Omen creature"),
            "CR 720.2c/720.4: the drawn card has only its normal characteristics in hand"
        );
    }
}

#[test]
fn cr_720_5_player_may_choose_an_omen_cards_alternative_name() {
    use ironsmith::effects::ChooseCardNameEffect;
    use ironsmith::{CardType, Effect, ObjectFilter, StackEntry};
    struct Name(&'static str);
    impl DecisionMaker for Name {
        fn decide_text(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TextInputContext,
        ) -> String {
            self.0.into()
        }
    }
    let mut mismatches = Vec::new();
    // Real Omen card: Dirgur Island Dragon // Skimming Strike (Instant — Omen).
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for restricted in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = q09_observer(&mut game, actor);
            let filter = restricted.then(|| ObjectFilter::default().with_type(CardType::Instant));
            game.stack.push(StackEntry::ability(
                source,
                actor,
                vec![Effect::new(ChooseCardNameEffect::new(
                    PlayerFilter::You,
                    filter,
                    "omen alternative name",
                ))],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Name("Skimming Strike"))
                .unwrap();
            let chosen = game.chosen_named_option(source);
            if chosen != Some("Skimming Strike") {
                mismatches.push(format!(
                    "actor={actor:?} restricted_to_instant={restricted}: stored {chosen:?}"
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 720.5: a player may choose an omen card's alternative name:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_722_2_prepare_inset_defines_the_cast_copys_spell_characteristics() {
    use ironsmith::card::LinkedFaceLayout;
    use ironsmith::effects::PrepareEffect;
    use ironsmith::{CardType, ChooseSpec, ColorSet, Effect, LegalAction, Phase, Zone};
    let mut mismatches = Vec::new();
    for actor in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = actor;
        game.turn.priority_player = Some(actor);
        game.turn.phase = Phase::FirstMain;
        let (mut front, mut inset) = q09_omen_definitions();
        front.card.name = "Normal preparation creature".into();
        front.card.other_face_name = Some("Inset prepare spell".into());
        front.card.linked_face_layout = LinkedFaceLayout::Prepare;
        inset.card.name = "Inset prepare spell".into();
        inset.card.card_types = vec![CardType::Sorcery].into();
        inset.card.subtypes = vec![].into();
        inset.card.other_face = None;
        inset.card.other_face_name = None;
        inset.card.linked_face_layout = LinkedFaceLayout::None;
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&inset);
        let source = game.create_object_from_definition(&front, actor, Zone::Battlefield);
        let observer = q09_observer(&mut game, actor);
        let normal = game.current_characteristics(source).unwrap();
        if normal.name != "Normal preparation creature"
            || (normal.power, normal.toughness) != (Some(4), Some(5))
        {
            mismatches.push(format!(
                "actor={actor:?}: permanent must keep normal characteristics: {:?}",
                normal.name
            ));
        }
        q09_resolve_ability(
            &mut game,
            observer,
            actor,
            vec![Effect::new(PrepareEffect::new(ChooseSpec::SpecificObject(
                source,
            )))],
        );
        assert!(game.is_prepared(source));
        assert_eq!(game.exile.len(), 1);
        let copy = game.exile[0];
        game.player_mut(actor).unwrap().mana_pool.colorless = 1;
        game.player_mut(actor).unwrap().mana_pool.red = 1;
        let action = ironsmith::compute_legal_actions(&game, actor)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == copy))
            .expect("prepared copy may be cast");
        q09_complete_priority_action(&mut game, actor, action).unwrap();
        assert_eq!(game.stack.len(), 1);
        let spell = game.stack[0].object_id;
        let c = game.current_characteristics(spell).unwrap();
        let mv = q09_snapshot_mv(&game, spell);
        if c.name != "Inset prepare spell"
            || c.card_types.as_slice() != [CardType::Sorcery]
            || c.colors != ColorSet::RED
            || mv != 2
            || c.power.is_some()
        {
            mismatches.push(format!(
                "actor={actor:?}: prepare spell name={:?} types={:?} colors={:?} mv={mv}",
                c.name, c.card_types, c.colors
            ));
        }
        let life = game.player(actor).unwrap().life;
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        if game.player(actor).unwrap().life - life != 5 {
            mismatches.push(format!(
                "actor={actor:?}: prepare spell resolved gain {}",
                game.player(actor).unwrap().life - life
            ));
        }
        if game.current_name(source).as_deref() != Some("Normal preparation creature") {
            mismatches.push(format!("actor={actor:?}: source changed name"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 722.2: the inset defines the alternative characteristics used by the cast prepare spell:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_723_2_7_word_of_command_and_opposition_agent_are_supported() {
    let mut mismatches = Vec::new();
    for (name, text) in [
        (
            "Word of Command",
            "Mana cost: {B}{B}\nType: Instant\nFirst printed set: Limited Edition Alpha\nLook at target opponent's hand and choose a card from it. You control that player until Word of Command finishes resolving. The player plays that card if able. While doing so, the player can activate mana abilities only if they're from lands that player controls and only if mana they produce is spent to activate other mana abilities of lands the player controls and/or to play that card. If the chosen card is cast as a spell, you control the player while that spell is resolving.",
        ),
        (
            "Opposition Agent",
            "Mana cost: {2}{B}\nType: Creature — Human Rogue\nFirst printed set: Commander Legends\nPower/Toughness: 3/2\nFlash\nYou control your opponents while they're searching their libraries.\nWhile an opponent is searching their library, they exile each card they find. You may play those cards for as long as they remain exiled, and you may spend mana as though it were mana of any color to cast them.",
        ),
    ] {
        match ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false) {
            Ok(def) => {
                if ironsmith::cards::generated_definition_has_unimplemented_content(&def) {
                    mismatches.push(format!(
                        "{name}: compiled definition has unimplemented content"
                    ));
                }
                let debug = format!("{def:?}").to_ascii_lowercase();
                if !(debug.contains("controlplayer")
                    || debug.contains("controlopponentswhilesearchinglibraries"))
                {
                    mismatches.push(format!(
                        "{name}: compiled definition has no player-control effect"
                    ));
                }
            }
            Err(error) => mismatches.push(format!(
                "{name}: strict compile rejected the card: {error:?}"
            )),
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 723.2/723.7: the two limited-duration player-control cards (and Word of Command's action restrictions) must be supported:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn cr_723_6_controlled_player_may_still_concede_and_controller_is_unaffected() {
    use ironsmith::effects::ControlPlayerEffect;
    use ironsmith::{Effect, Phase};
    for target in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let controller = PlayerId::from_index(1 - target.0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = target;
        game.turn.priority_player = Some(target);
        game.turn.phase = Phase::FirstMain;
        let source = q09_observer(&mut game, controller);
        q09_resolve_ability(
            &mut game,
            source,
            controller,
            vec![Effect::new(ControlPlayerEffect::until_end_of_turn(
                PlayerFilter::Specific(target),
            ))],
        );
        assert_eq!(game.controlling_player_for(target), controller);
        let result = game.concede_game(target);
        assert_eq!(
            result.ok(),
            Some(true),
            "CR 723.6: a controlled player may concede at any time"
        );
        assert!(
            game.player(target).unwrap().has_lost,
            "CR 723.6/104.3a: the conceding (controlled) player loses"
        );
        assert!(
            !game.player(controller).unwrap().has_lost,
            "CR 723.6: the controlling player does not concede by the controlled player's concession"
        );
    }
}
