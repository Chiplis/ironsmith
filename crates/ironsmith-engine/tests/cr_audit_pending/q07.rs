//! CR audit (in-review completion pass, package q07): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::decisions::context::TargetsContext;
use ironsmith::{
    CardType, CastingMethod, ChooseSpec, ColorSet, DecisionMaker, EffectExecutor, LegalAction,
    ManaSymbol, ObjectId, Phase, PlayerFilter, Step, Target, Zone,
};

fn q07_compile(name: &str, header_and_text: &str) -> ironsmith::cards::CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, header_and_text, false)
        .unwrap_or_else(|e| panic!("{name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "{name} has unimplemented content"
    );
    def
}

fn q07_game(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn q07_absolute_virtue() -> ironsmith::cards::CardDefinition {
    q07_compile(
        "Absolute Virtue",
        "Mana cost: {6}{W}{U}\nType: Legendary Creature — Avatar Warrior\nPower/Toughness: 8/8\nThis spell can't be countered.\nFlying\nYou have protection from each of your opponents. (You can't be dealt damage, enchanted, or targeted by anything controlled by your opponents.)",
    )
}

/// Records the legal targets of the first target requirement and picks a
/// preferred harmless target.
struct Q07RecordTargets {
    seen: Vec<Vec<Target>>,
    prefer: Target,
}

impl DecisionMaker for Q07RecordTargets {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut out = Vec::new();
        for req in &ctx.requirements {
            self.seen.push(req.legal_targets.clone());
            let pick = if req.legal_targets.contains(&self.prefer) {
                self.prefer
            } else {
                req.legal_targets[0]
            };
            out.push(pick);
        }
        out
    }
}

fn q07_complete_action<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: LegalAction,
    dm: &mut D,
    queue: &mut ironsmith::triggers::TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    assert_eq!(game.turn.priority_player, Some(player));
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
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
                .ok_or_else(|| "requested casting method lost".to_string())?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending native action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    Ok(())
}

/// Pass priority around until the stack is empty.
fn q07_settle<D: DecisionMaker>(
    game: &mut GameState,
    queue: &mut ironsmith::triggers::TriggerQueue,
    dm: &mut D,
) {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    for _ in 0..20 {
        ironsmith::game_loop::advance_priority_with_dm(game, queue, dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        let mut state = PriorityLoopState::new(game.players_in_game());
        for _ in 0..game.players_in_game() {
            ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                dm,
            )
            .unwrap();
        }
    }
    panic!("fixture did not settle");
}

fn q07_cast_action(game: &GameState, player: PlayerId, id: ObjectId) -> Option<LegalAction> {
    ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Normal, .. } if *spell_id == id))
}

// ---------------------------------------------------------------------------
// CR 702.16b: player protection and abilities from sources with the quality.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_16b_protection_restricts_spell_and_ability_targets_for_players_and_sources() {
    use ironsmith::ability::ProtectionFrom;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, PowerToughness, StaticAbility};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let shock = q07_compile(
        "Shock",
        "Mana cost: {R}\nType: Instant\nShock deals 2 damage to any target.",
    );
    let pyromancer = q07_compile(
        "Prodigal Pyromancer",
        "Mana cost: {2}{R}\nType: Creature — Human Wizard\nPower/Toughness: 1/1\n{T}: This creature deals 1 damage to any target.",
    );
    let colorless_pinger = q07_compile(
        "Colorless Pinger",
        "Mana cost: {2}\nType: Artifact\n{T}: This artifact deals 1 damage to any target.",
    );
    let protected_def = CardDefinitionBuilder::new(CardId::new(), "Red-protected bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(StaticAbility::protection(
            ProtectionFrom::Color(ColorSet::RED),
        )))
        .build();
    let mut errors = Vec::new();
    // source: 0 = red Shock spell, 1 = red Pyromancer ability, 2 = colorless artifact ability
    for source_kind in 0..3 {
        for virtue in [false, true] {
            let mut game = q07_game(alice);
            if virtue {
                game.create_object_from_definition(&q07_absolute_virtue(), alice, Zone::Battlefield);
            }
            let protected = game.create_object_from_definition(&protected_def, alice, Zone::Battlefield);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = Q07RecordTargets { seen: Vec::new(), prefer: Target::Player(bob) };
            game.turn.priority_player = Some(bob);
            let action = match source_kind {
                0 => {
                    game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Red, 1);
                    let card = game.create_object_from_definition(&shock, bob, Zone::Hand);
                    q07_cast_action(&game, bob, card).expect("Shock castable")
                }
                _ => {
                    let def = if source_kind == 1 { &pyromancer } else { &colorless_pinger };
                    let src = game.create_object_from_definition(def, bob, Zone::Battlefield);
                    game.remove_summoning_sickness(src);
                    ironsmith::compute_legal_actions(&game, bob)
                        .unwrap()
                        .into_iter()
                        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == src))
                        .expect("pinger ability offered")
                }
            };
            q07_complete_action(&mut game, bob, action, &mut dm, &mut queue).unwrap();
            let legal = dm.seen.first().cloned().unwrap_or_default();
            let red = source_kind != 2;
            let alice_allowed = !virtue;
            let protected_allowed = !red;
            if legal.contains(&Target::Player(alice)) != alice_allowed {
                errors.push(format!("source={source_kind} virtue={virtue}: Alice targetable={} expected {alice_allowed}", !alice_allowed));
            }
            if legal.contains(&Target::Object(protected)) != protected_allowed {
                errors.push(format!("source={source_kind} virtue={virtue}: pro-red creature targetable={} expected {protected_allowed}", !protected_allowed));
            }
            if !legal.contains(&Target::Player(bob)) {
                errors.push(format!("source={source_kind} virtue={virtue}: Bob not targetable"));
            }
        }
    }
    assert!(errors.is_empty(), "CR 702.16b: protected permanents/players can't be targeted by spells with the quality or abilities from sources with the quality: {errors:?}");
}

// ---------------------------------------------------------------------------
// CR 702.16c: Auras with the quality can't enchant a protected player.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_16c_player_protection_blocks_curse_targeting_and_removes_attached_curse() {
    use ironsmith::object::AttachmentTarget;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let curse = q07_compile(
        "Curse of the Pierced Heart",
        "Mana cost: {1}{R}\nType: Enchantment — Aura Curse\nEnchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.",
    );
    let mut errors = Vec::new();
    // Casting: Curse spell can't target the protected player.
    for virtue in [false, true] {
        let mut game = q07_game(bob);
        if virtue {
            game.create_object_from_definition(&q07_absolute_virtue(), alice, Zone::Battlefield);
        }
        game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Red, 2);
        let card = game.create_object_from_definition(&curse, bob, Zone::Hand);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = Q07RecordTargets { seen: Vec::new(), prefer: Target::Player(alice) };
        let action = q07_cast_action(&game, bob, card).expect("Curse castable (Bob is always a legal player)");
        q07_complete_action(&mut game, bob, action, &mut dm, &mut queue).unwrap();
        let legal = dm.seen.first().cloned().unwrap_or_default();
        if legal.contains(&Target::Player(alice)) == virtue {
            errors.push(format!("cast virtue={virtue}: legal targets {legal:?}"));
        }
        q07_settle(&mut game, &mut queue, &mut dm);
        let stable = game.object(card).map(|o| o.stable_id);
        let _ = stable;
        let attached_to_alice = game.battlefield.iter().any(|id| {
            game.object(*id).map(|o| o.attached_to == Some(AttachmentTarget::Player(alice))).unwrap_or(false)
        });
        if attached_to_alice == virtue {
            errors.push(format!("cast virtue={virtue}: curse attached to Alice = {attached_to_alice}"));
        }
    }
    // SBA: already-attached Curse goes to owner's graveyard once the player gains protection.
    for virtue in [false, true] {
        let mut game = q07_game(alice);
        let aura = game.create_object_from_definition(&curse, bob, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Player(alice)));
        if virtue {
            game.create_object_from_definition(&q07_absolute_virtue(), alice, Zone::Battlefield);
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let on_bf = game.battlefield.contains(&aura);
        let in_bob_gy = game.player(bob).unwrap().graveyard.len() == 1;
        if on_bf == virtue || in_bob_gy != virtue {
            errors.push(format!("sba virtue={virtue}: on_battlefield={on_bf} in_owner_graveyard={in_bob_gy}"));
        }
    }
    assert!(errors.is_empty(), "CR 702.16c: a protected player can't be enchanted by Auras with the stated quality; attached ones go to their owner's graveyard: {errors:?}");
}

// ---------------------------------------------------------------------------
// CR 702.16e: damage to a protected player is prevented.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_16e_damage_to_protected_player_is_prevented() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, PowerToughness, StaticAbility};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Lifelink source")
        .card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(Ability::static_ability(StaticAbility::lifelink()))
        .build();
    let mut errors = Vec::new();
    for virtue in [false, true] {
        for source_controller in [alice, bob] {
            let mut game = q07_game(alice);
            if virtue {
                game.create_object_from_definition(&q07_absolute_virtue(), alice, Zone::Battlefield);
            }
            let source = game.create_object_from_definition(&source_def, source_controller, Zone::Battlefield);
            ironsmith::effects::DealDamageEffect::new(3, ChooseSpec::Player(PlayerFilter::Specific(alice)))
                .execute(&mut game, &mut EffectContext::new_default(source, source_controller))
                .unwrap();
            let prevented = virtue && source_controller == bob;
            let dealt = if prevented { 0 } else { 3 };
            let alice_life = game.player(alice).unwrap().life;
            let expected_alice = 20 - dealt + if source_controller == alice { dealt } else { 0 };
            if alice_life != expected_alice {
                errors.push(format!("virtue={virtue} source_controller={source_controller:?}: Alice life {alice_life}, expected {expected_alice}"));
            }
            if source_controller == bob && game.player(bob).unwrap().life != 20 + dealt {
                errors.push(format!("virtue={virtue}: Bob lifelink gain {} expected {dealt}", game.player(bob).unwrap().life - 20));
            }
        }
    }
    assert!(errors.is_empty(), "CR 702.16e: damage from sources with the quality to a protected player is prevented: {errors:?}");
}

// ---------------------------------------------------------------------------
// CR 702.8a: Flash.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_8a_flash_card_castable_whenever_an_instant_could_be() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let viper = q07_compile(
        "Ambush Viper",
        "Mana cost: {1}{G}\nType: Creature — Snake\nPower/Toughness: 2/1\nFlash\nDeathtouch",
    );
    let bears = q07_compile(
        "Grizzly Bears",
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2\n",
    );
    let mut errors = Vec::new();
    // window: 0 = own main empty stack (sorcery timing), 1 = opponent's main, 2 = own combat step,
    //         3 = own main with nonempty stack
    for window in 0..4 {
        let caster = if window == 1 { bob } else { alice };
        let mut game = q07_game(alice);
        if window == 2 {
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::BeginCombat);
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        if window == 3 {
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Generic(0), 0);
            let bg = q07_compile(
                "Background Instant",
                "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
            );
            let card = game.create_object_from_definition(&bg, alice, Zone::Hand);
            let action = q07_cast_action(&game, alice, card).expect("background instant");
            q07_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 1);
            game.turn.priority_player = Some(alice);
        }
        game.turn.priority_player = Some(caster);
        game.player_mut(caster).unwrap().mana_pool.add(ManaSymbol::Green, 2);
        let v = game.create_object_from_definition(&viper, caster, Zone::Hand);
        let b = game.create_object_from_definition(&bears, caster, Zone::Hand);
        let viper_ok = q07_cast_action(&game, caster, v).is_some();
        let bears_ok = q07_cast_action(&game, caster, b).is_some();
        let bears_expected = window == 0;
        if !viper_ok {
            errors.push(format!("window={window}: Flash creature not castable"));
        }
        if bears_ok != bears_expected {
            errors.push(format!("window={window}: non-flash creature castable={bears_ok}, expected {bears_expected}"));
        }
    }
    // Actually cast the Flash creature on the opponent's turn and resolve it.
    let mut game = q07_game(alice);
    game.turn.priority_player = Some(bob);
    game.player_mut(bob).unwrap().mana_pool.add(ManaSymbol::Green, 2);
    let v = game.create_object_from_definition(&viper, bob, Zone::Hand);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let action = q07_cast_action(&game, bob, v).expect("flash cast");
    q07_complete_action(&mut game, bob, action, &mut dm, &mut queue).unwrap();
    q07_settle(&mut game, &mut queue, &mut dm);
    let on_bf = game.battlefield.iter().any(|id| {
        game.object(*id).map(|o| o.owner == bob && game.current_name(*id).as_deref() == Some("Ambush Viper")).unwrap_or(false)
    });
    if !on_bf {
        errors.push("Flash creature cast on opponent's turn did not resolve onto battlefield".into());
    }
    assert!(errors.is_empty(), "CR 702.8a: a card with flash may be cast any time its controller could cast an instant: {errors:?}");
}

// ---------------------------------------------------------------------------
// CR 702.37b / 702.37e: Megamorph face-down casting and face-up rules.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_37b_megamorph_card_cast_face_down_for_three_then_turned_up_with_counter() {
    let alice = PlayerId::from_index(0);
    let charger = q07_compile(
        "Sandstorm Charger",
        "Mana cost: {4}{W}\nType: Creature — Beast\nPower/Toughness: 3/4\nMegamorph {4}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its megamorph cost and put a +1/+1 counter on it.)",
    );
    let mut game = q07_game(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
    let card = game.create_object_from_definition(&charger, alice, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let actions = ironsmith::compute_legal_actions(&game, alice).unwrap();
    let face_down = actions.into_iter().find(|a| {
        matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::FaceDown, .. } if *spell_id == card)
    });
    assert!(face_down.is_some(), "CR 702.37b: a megamorph card may be cast face down by paying {{3}}");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    q07_complete_action(&mut game, alice, face_down.unwrap(), &mut dm, &mut queue).unwrap();
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0, "CR 702.37b: face-down cast pays {{3}}");
    q07_settle(&mut game, &mut queue, &mut dm);
    let perm = game.find_object_by_stable_id(stable).expect("face-down permanent");
    assert_eq!(game.object(perm).unwrap().zone, Zone::Battlefield);
    assert!(game.is_face_down(perm), "CR 702.37b: permanent is face down");
    assert_eq!(game.current_power(perm), Some(2), "CR 702.37b: face-down 2/2");
    assert_eq!(game.current_toughness(perm), Some(2), "CR 702.37b: face-down 2/2");
    // The engine's nameless sentinel (display-only label, treated as no name by name comparisons).
    let name = game.current_name(perm).unwrap_or_default();
    assert!(name.is_empty() || name == ironsmith::object::FACE_DOWN_DISPLAY_NAME, "CR 702.37b: face-down has no name, got {name:?}");
    assert!(game.current_subtypes(perm).unwrap_or_default().is_empty(), "CR 702.37b: no subtypes");
    // Turn face up for the megamorph cost.
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 4);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::White, 1);
    game.turn.priority_player = Some(alice);
    let up = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == perm))
        .expect("CR 702.37e: turn face up offered");
    q07_complete_action(&mut game, alice, up, &mut dm, &mut queue).unwrap();
    assert!(!game.is_face_down(perm));
    assert_eq!(game.current_power(perm), Some(4), "CR 702.37b: 3/4 plus +1/+1 counter");
    assert_eq!(game.current_toughness(perm), Some(5), "CR 702.37b: 3/4 plus +1/+1 counter");
}

#[test]
fn cr_702_37e_turning_face_up_does_not_trigger_enters_abilities() {
    let alice = PlayerId::from_index(0);
    let def = q07_compile(
        "Morph Lifegainer",
        "Mana cost: {2}{W}\nType: Creature — Human Cleric\nPower/Toughness: 2/3\nWhen this creature enters, you gain 3 life.\nMorph {1}",
    );
    let mut game = q07_game(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
    let card = game.create_object_from_definition(&def, alice, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let face_down = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::FaceDown, .. } if *spell_id == card))
        .expect("face-down cast offered");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    q07_complete_action(&mut game, alice, face_down, &mut dm, &mut queue).unwrap();
    q07_settle(&mut game, &mut queue, &mut dm);
    let perm = game.find_object_by_stable_id(stable).unwrap();
    assert!(game.is_face_down(perm));
    assert_eq!(game.player(alice).unwrap().life, 20, "face-down entry has no ETB text");
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
    game.turn.priority_player = Some(alice);
    let stack_before = game.stack.len();
    let up = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == perm))
        .expect("turn face up offered");
    q07_complete_action(&mut game, alice, up, &mut dm, &mut queue).unwrap();
    assert!(!game.is_face_down(perm));
    assert_eq!(game.stack.len(), stack_before, "CR 702.37e: turning face up is a special action and doesn't use the stack");
    q07_settle(&mut game, &mut queue, &mut dm);
    assert_eq!(game.player(alice).unwrap().life, 20, "CR 702.37e: enters-the-battlefield abilities don't trigger when a permanent is turned face up");
    assert_eq!(game.current_power(perm), Some(2));
    assert_eq!(game.current_toughness(perm), Some(3));
}

// ---------------------------------------------------------------------------
// CR 702.80b: wither via last known information after the source left.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_80b_sacrificed_wither_source_uses_lki_for_damage_counters() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CounterType, PowerToughness};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut errors = Vec::new();
    for wither in [false, true] {
        let text = format!(
            "Mana cost: {{2}}{{B}}\nType: Creature — Elemental\nPower/Toughness: 2/2\n{}Sacrifice this creature: It deals 2 damage to target creature.",
            if wither { "Wither\n" } else { "" }
        );
        let def = q07_compile("Wither Sacrificer", &text);
        let victim_def = CardDefinitionBuilder::new(CardId::new(), "Victim")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let mut game = q07_game(alice);
        let src = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(src);
        let victim = game.create_object_from_definition(&victim_def, bob, Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = Q07RecordTargets { seen: Vec::new(), prefer: Target::Object(victim) };
        let action = ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == src))
            .expect("sacrifice ability offered");
        q07_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
        assert!(!game.battlefield.contains(&src), "source sacrificed as a cost");
        q07_settle(&mut game, &mut queue, &mut dm);
        let counters = game.counter_count(victim, CounterType::MinusOneMinusOne);
        let damage = game.damage_on(victim);
        let (want_counters, want_damage) = if wither { (2, 0) } else { (0, 2) };
        if counters != want_counters || damage != want_damage {
            errors.push(format!("wither={wither}: counters={counters} damage={damage}, expected {want_counters}/{want_damage}"));
        }
    }
    assert!(errors.is_empty(), "CR 702.80b: LKI determines whether a departed source had wither when it deals damage: {errors:?}");
}

// ---------------------------------------------------------------------------
// CR 702.6e: Equip planeswalker targets only a planeswalker you control, sorcery speed.
// ---------------------------------------------------------------------------

#[test]
fn cr_702_6e_equip_planeswalker_targets_only_own_planeswalkers_at_sorcery_speed() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, PowerToughness, Subtype};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let luxior = q07_compile(
        "Luxior, Giada's Gift",
        "Mana cost: {1}\nType: Legendary Artifact — Equipment\nEquipped creature gets +1/+1 for each counter on it.\nEquipped permanent isn't a planeswalker and is a creature in addition to its other types. (Loyalty abilities can still be activated.)\nEquip planeswalker {1}\nEquip {3}",
    );
    let walker = CardDefinitionBuilder::new(CardId::new(), "Test walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .build();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Test creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut errors = Vec::new();
    // Main phase: collect each equip ability's legal targets.
    let mut target_sets = Vec::new();
    let mut ids = (ObjectId::from_raw(0), ObjectId::from_raw(0), ObjectId::from_raw(0), ObjectId::from_raw(0));
    for ability_pick in 0..2 {
        let mut game = q07_game(alice);
        let lux = game.create_object_from_definition(&luxior, alice, Zone::Battlefield);
        let own_pw = game.create_object_from_definition(&walker, alice, Zone::Battlefield);
        let opp_pw = game.create_object_from_definition(&walker, bob, Zone::Battlefield);
        let own_cr = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let opp_cr = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        ids = (own_pw, opp_pw, own_cr, opp_cr);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        let actions: Vec<_> = ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .into_iter()
            .filter(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == lux))
            .collect();
        if actions.len() != 2 {
            errors.push(format!("main phase: {} equip actions offered, expected 2", actions.len()));
            break;
        }
        let mut dm = Q07RecordTargets { seen: Vec::new(), prefer: Target::Object(own_pw) };
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        q07_complete_action(&mut game, alice, actions[ability_pick].clone(), &mut dm, &mut queue).unwrap();
        let mut set: Vec<Target> = dm.seen.first().cloned().unwrap_or_default();
        set.retain(|t| matches!(t, Target::Object(o) if [own_pw, opp_pw, own_cr, opp_cr].contains(o)));
        target_sets.push(set);
    }
    let (own_pw, _opp_pw, own_cr, _opp_cr) = ids;
    let mut sorted: Vec<Vec<Target>> = target_sets.clone();
    sorted.sort_by_key(|s| s.contains(&Target::Object(own_cr)));
    if sorted.len() == 2 {
        if sorted[0] != vec![Target::Object(own_pw)] {
            errors.push(format!("equip planeswalker targets {:?}, expected only own planeswalker", sorted[0]));
        }
        if sorted[1] != vec![Target::Object(own_cr)] {
            errors.push(format!("ordinary equip targets {:?}, expected only own creature", sorted[1]));
        }
    }
    // Combat step / opponent's turn: no equip activation.
    for window in 0..2 {
        let mut game = q07_game(if window == 0 { alice } else { bob });
        if window == 0 {
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::BeginCombat);
        }
        game.turn.priority_player = Some(alice);
        let lux = game.create_object_from_definition(&luxior, alice, Zone::Battlefield);
        game.create_object_from_definition(&walker, alice, Zone::Battlefield);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        let offered = ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .into_iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if source == lux));
        if offered {
            errors.push(format!("window={window}: equip offered outside sorcery timing"));
        }
    }
    let _ = Subtype::Equipment;
    assert!(errors.is_empty(), "CR 702.6e: equip planeswalker targets a planeswalker you control, only as a sorcery: {errors:?}");
}
