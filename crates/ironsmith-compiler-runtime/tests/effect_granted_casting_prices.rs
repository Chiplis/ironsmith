//! UNVALIDATED source-authored effect-granted price scenarios; no execution.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::LinkedFaceLayout;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack, resolve_stack_entry,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::grant::{GrantSpec, GrantUsageLimit, Grantable};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/effect_granted_casting_prices.json.fixture"
    ))
    .unwrap()
}
fn pair(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    pair(name, row["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}
fn printed(g: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    g.create_object_from_definition(
        &compile_to_runtime_definition(name, text, false).unwrap(),
        owner,
        zone,
    )
}
fn spell(g: &mut GameState, owner: PlayerId, zone: Zone, cost: &str) -> ObjectId {
    printed(
        g,
        owner,
        zone,
        "Price candidate",
        &format!("Mana cost: {cost}\nType: Creature — Bear\nPower/Toughness: 2/2"),
    )
}
fn casts(g: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(g, player)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==id))
        .collect()
}
fn priced(
    g: &GameState,
    player: PlayerId,
    id: ObjectId,
    provider: ObjectId,
) -> Option<LegalAction> {
    casts(g,player,id).into_iter().find(|a|matches!(a,LegalAction::CastSpell{casting_method:CastingMethod::AlternativePrice{price,..},..} if price.source==provider))
}
#[derive(Default)]
struct Choices {
    objects: Vec<ObjectId>,
    targets: Vec<Target>,
    players: Vec<PlayerId>,
    x: u32,
    accept: Option<bool>,
}
impl DecisionMaker for Choices {
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.min == 0 && self.accept == Some(false) {
            return Vec::new();
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min.max(1).min(ctx.max))
            .map(|option| option.index)
            .collect()
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.accept.unwrap_or(true)
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        assert!(self.x >= ctx.min && self.x <= ctx.max);
        self.x
    }

    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.players.push(ctx.player);
        let chosen = self
            .objects
            .iter()
            .copied()
            .filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal))
            .collect::<Vec<_>>();
        if chosen.is_empty() {
            SelectFirstDecisionMaker.decide_objects(g, ctx)
        } else {
            chosen
        }
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for t in &self.targets {
            assert!(ctx.requirements.iter().any(|r| r.legal_targets.contains(t)));
        }
        self.targets.clone()
    }
}
fn announce(g: &mut GameState, action: LegalAction, dm: &mut impl DecisionMaker) -> ObjectId {
    let LegalAction::CastSpell { spell_id, .. } = &action else {
        panic!("cast required")
    };
    let stable = g.object(*spell_id).unwrap().stable_id;
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            let id = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(g.object(id).unwrap().zone, Zone::Stack);
            ironsmith::game_loop::put_triggers_on_stack_with_dm(g, &mut queue, dm).unwrap();
            return id;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("announcement did not finish")
}
fn settle(g: &mut GameState) {
    let mut q = TriggerQueue::new();
    for _ in 0..32 {
        put_triggers_on_stack(g, &mut q).unwrap();
        if g.stack_is_empty() {
            return;
        }
        resolve_stack_entry(g).unwrap();
    }
    panic!("did not settle")
}

#[test]
fn nine_price_root_bodies_round_trip_and_two_full_body_gates_remain_explicit() {
    let rows = rows();
    assert_eq!(rows.len(), 11);
    assert_eq!(
        rows.iter()
            .filter(|r| r["proposed_coverage"] == "partial")
            .count(),
        2
    );
    for row in rows.iter().filter(|r| r["proposed_coverage"] != "partial") {
        for def in definitions(row["name"].as_str().unwrap()) {
            let text = ironsmith_text::compiled_text_lines(&def).join("\n");
            assert!(
                text.to_ascii_lowercase().contains("rather than"),
                "{}: {text}",
                row["name"]
            );
        }
    }
}

fn life_price() -> ironsmith::TotalCost {
    ironsmith::TotalCost::from_cost(ironsmith::costs::Cost::effect(
        ironsmith::effects::PayLifeEffect::you(ironsmith::effect::Value::ManaValueOf(Box::new(
            ironsmith::target::ChooseSpec::Source,
        ))),
    ))
}
fn execute_priced(
    g: &mut GameState,
    card: ObjectId,
    price: ironsmith::TotalCost,
    dm: &mut impl DecisionMaker,
) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
    use ironsmith::effects::{EffectContext as ExecutionContext, EffectExecutor};
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(g.object(card).unwrap(), g);
    let mut context = ExecutionContext::new(ObjectId::from_raw(9000), A, dm).with_tagged_objects(
        std::collections::HashMap::from([("priced".into(), vec![snapshot])]),
    );
    ironsmith::effects::CastTaggedEffect::new("priced", ironsmith::target::PlayerFilter::You)
        .with_alternative_cost(price)
        .execute(g, &mut context)
}
#[test]
fn immediate_life_price_uses_real_payment_and_retains_additional_mana_costs() {
    for paid in [false, true] {
        let mut g = game();
        let card = printed(
            &mut g,
            B,
            Zone::Exile,
            "Additional price",
            "Mana cost: {7}\nType: Creature — Bear\nPower/Toughness: 2/2\nAs an additional cost to cast this spell, pay {2}.",
        );
        g.player_mut(A).unwrap().mana_pool.colorless = if paid { 2 } else { 0 };
        let outcome =
            execute_priced(&mut g, card, life_price(), &mut SelectFirstDecisionMaker).unwrap();
        if paid {
            assert_eq!(g.player(A).unwrap().life, 13);
            assert_eq!(g.player(B).unwrap().life, 20);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(g.stack.len(), 1);
            assert_eq!(g.current_controller(g.stack[0].object_id).unwrap(), A);
            resolve_stack_entry(&mut g).unwrap();
            assert_eq!(g.battlefield.len(), 1);
        } else {
            assert!(g.stack_is_empty(), "{outcome:?}");
            assert_eq!(g.object(card).unwrap().zone, Zone::Exile);
            assert_eq!(
                g.player(A).unwrap().life,
                20,
                "failed payment must roll back the earlier cost"
            );
        }
    }
}
#[test]
fn fixed_effect_price_gets_reductions_and_taxes_once_and_printed_x_is_zero() {
    for (modifier, expected) in [
        ("Spells you cast cost {1} less to cast.", 2),
        ("Spells cost {1} more to cast.", 4),
    ] {
        let mut g = game();
        printed(
            &mut g,
            A,
            Zone::Battlefield,
            "Price modifier",
            &format!("Type: Enchantment\n{modifier}"),
        );
        let card = printed(
            &mut g,
            A,
            Zone::Exile,
            "X result",
            "Mana cost: {X}{B}\nType: Sorcery\nYou gain X life.",
        );
        g.player_mut(A).unwrap().mana_pool.colorless = 4;
        let cost = ironsmith::TotalCost::mana(ManaCost::new().add_generic(3));
        execute_priced(&mut g, card, cost, &mut SelectFirstDecisionMaker).unwrap();
        let stack = g.stack.last().unwrap();
        assert_eq!(g.object(stack.object_id).unwrap().x_value, Some(0));
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 4 - expected);
        resolve_stack_entry(&mut g).unwrap();
        assert_eq!(g.player(A).unwrap().life, 20);
    }
}
#[test]
fn mandatory_price_does_not_offer_a_second_independent_alternative() {
    let mut g = game();
    g.create_object_from_definition(&definitions("Dream Halls")[0], A, Zone::Battlefield);
    let spell = printed(
        &mut g,
        A,
        Zone::Exile,
        "Black seven",
        "Mana cost: {6}{B}\nType: Sorcery\nYou gain 1 life.",
    );
    let discard = printed(
        &mut g,
        A,
        Zone::Hand,
        "Black discard",
        "Mana cost: {B}\nType: Creature — Rat\nPower/Toughness: 1/1",
    );
    execute_priced(
        &mut g,
        spell,
        life_price(),
        &mut Choices {
            objects: vec![discard],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(g.player(A).unwrap().life, 13);
    assert_eq!(g.object(discard).unwrap().zone, Zone::Hand);
    assert_eq!(g.stack.len(), 1);
}
#[test]
fn dream_halls_benefits_each_caster_shares_actual_spell_color_and_grants_no_origin() {
    for def in definitions("Dream Halls") {
        for caster in [A, B] {
            let mut g = game();
            g.turn.active_player = caster;
            g.turn.priority_player = Some(caster);
            let provider = g.create_object_from_definition(&def, A, Zone::Battlefield);
            let spell = printed(
                &mut g,
                caster,
                Zone::Hand,
                "Blue spell",
                "Mana cost: {5}{U}\nType: Sorcery\nYou gain 1 life.",
            );
            let wrong = printed(
                &mut g,
                caster,
                Zone::Hand,
                "Red card",
                "Mana cost: {R}\nType: Instant\nYou gain 1 life.",
            );
            let foreign = printed(
                &mut g,
                if caster == A { B } else { A },
                Zone::Hand,
                "Foreign blue",
                "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
            );
            assert!(
                priced(&g, caster, spell, provider).is_none(),
                "the spell cannot discard itself or an opponent's card"
            );
            let right = printed(
                &mut g,
                caster,
                Zone::Hand,
                "Blue and red card",
                "Mana cost: {U}{R}\nType: Instant\nYou gain 1 life.",
            );
            let grave = printed(
                &mut g,
                caster,
                Zone::Graveyard,
                "Forbidden origin",
                "Mana cost: {U}\nType: Instant\nYou gain 1 life.",
            );
            assert!(priced(&g, caster, grave, provider).is_none());
            let action = priced(&g, caster, spell, provider).unwrap();
            announce(
                &mut g,
                action,
                &mut Choices {
                    objects: vec![right],
                    ..Default::default()
                },
            );
            assert_eq!(g.player(caster).unwrap().graveyard.len(), 2);
            assert_eq!(g.object(wrong).unwrap().zone, Zone::Hand);
            assert_eq!(g.object(foreign).unwrap().zone, Zone::Hand);
            assert_eq!(g.player(caster).unwrap().mana_pool.total(), 0);
            resolve_stack_entry(&mut g).unwrap();
            assert_eq!(g.player(caster).unwrap().life, 21);
        }
    }
}
#[test]
fn a_priced_collection_spends_one_budget_for_either_land_or_spell_and_keeps_exact_ids() {
    use ironsmith::effects::{
        EffectContext as ExecutionContext, EffectExecutor, GrantPlayTaggedDuration,
        GrantPlayTaggedEffect,
    };
    use ironsmith::target::PlayerFilter;
    for land_first in [false, true] {
        let mut g = game();
        let source = printed(
            &mut g,
            A,
            Zone::Battlefield,
            "Grant source",
            "Type: Creature — Rat\nPower/Toughness: 1/1",
        );
        let land = printed(&mut g, B, Zone::Exile, "Exiled land", "Type: Land");
        let spell = spell(&mut g, B, Zone::Exile, "{4}");
        let snapshots = [land, spell]
            .into_iter()
            .map(|id| ironsmith::snapshot::ObjectSnapshot::from_object(g.object(id).unwrap(), &g))
            .collect();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, A, &mut dm).with_tagged_objects(
            std::collections::HashMap::from([("pool".into(), snapshots)]),
        );
        GrantPlayTaggedEffect::new(
            "pool",
            PlayerFilter::You,
            GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )
        .with_max_plays(Some(1))
        .with_alternative_cost(life_price())
        .execute(&mut g, &mut ctx)
        .unwrap();
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        if land_first {
            let action = compute_legal_actions(&g, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::PlayLand { land_id, .. } if *land_id == land)).unwrap();
            let mut state = PriorityLoopState::new(2);
            let mut q = TriggerQueue::new();
            apply_priority_response_with_dm(
                &mut g,
                &mut q,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            assert!(casts(&g, A, spell).is_empty());
            assert_eq!(g.player(A).unwrap().life, 20);
        } else {
            let options = casts(&g, A, spell);
            assert_eq!(
                options.len(),
                1,
                "no ordinary-price escape from the mandatory substitution"
            );
            announce(&mut g, options[0].clone(), &mut dm);
            assert_eq!(g.player(A).unwrap().life, 16);
            resolve_stack_entry(&mut g).unwrap();
            assert!(!compute_legal_actions(&g, A).unwrap().iter().any(
                |action| matches!(action, LegalAction::PlayLand { land_id, .. } if *land_id == land)
            ));
        }
    }
}

fn combat_hit(g: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let mut combat = ironsmith::combat_state::CombatState {
        attackers: vec![ironsmith::combat_state::AttackerInfo {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
        block_declaration_complete: true,
        ..Default::default()
    };
    combat.record_attacked_permanent_types(g);
    g.combat = Some(combat.clone());
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::CombatDamage);
    let events =
        ironsmith::game_loop::try_execute_combat_damage_step_with_dm(g, &combat, false, dm)
            .unwrap();
    let mut q = TriggerQueue::new();
    ironsmith::game_loop::queue_combat_damage_triggers(g, &events, &mut q);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
}
#[test]
fn bismuth_uses_damaged_players_library_but_controllers_life_and_decline_pays_nothing() {
    for def in definitions("Bismuth Mindrender") {
        for accept in [false, true] {
            let mut g = game();
            let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
            let hit = spell(&mut g, B, Zone::Library, "{5}");
            let stable = g.object(hit).unwrap().stable_id;
            let own = spell(&mut g, A, Zone::Library, "{8}");
            let mut dm = Choices {
                accept: Some(accept),
                ..Default::default()
            };
            combat_hit(&mut g, source, &mut dm);
            assert_eq!(g.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            let hit = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(g.object(own).unwrap().zone, Zone::Library);
            assert_eq!(
                g.object(hit).unwrap().zone,
                if accept { Zone::Stack } else { Zone::Exile }
            );
            assert_eq!(g.player(A).unwrap().life, if accept { 15 } else { 20 });
            assert_eq!(g.object(hit).unwrap().owner, B);
            if accept {
                assert_eq!(g.current_controller(hit).unwrap(), A);
                resolve_stack_entry(&mut g).unwrap();
            }
        }
    }
}
#[test]
fn cruelclaw_discards_a_real_own_hand_card_and_never_pays_ordinary_mana() {
    for def in definitions("The Infamous Cruelclaw") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let hit = spell(&mut g, A, Zone::Library, "{9}");
        let stable = g.object(hit).unwrap().stable_id;
        let discard = printed(&mut g, A, Zone::Hand, "Actual discarded card", "Type: Land");
        let foreign = printed(&mut g, B, Zone::Hand, "Opponent's card", "Type: Land");
        let mut dm = Choices {
            objects: vec![discard],
            ..Default::default()
        };
        combat_hit(&mut g, source, &mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(
            g.object(g.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Stack
        );
        assert_eq!(g.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(g.object(foreign).unwrap().zone, Zone::Hand);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry(&mut g).unwrap();
        assert_eq!(g.battlefield.len(), 2);
    }
}
#[test]
fn nashi_exact_trigger_retains_one_shared_use_after_source_departure() {
    for def in definitions("Nashi, Moon Sage's Scion") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let own = spell(&mut g, A, Zone::Library, "{3}");
        let own_stable = g.object(own).unwrap().stable_id;
        let enemy = spell(&mut g, B, Zone::Library, "{4}");
        let enemy_stable = g.object(enemy).unwrap().stable_id;
        let mut dm = Choices::default();
        combat_hit(&mut g, source, &mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        g.turn.phase = Phase::NextMain;
        g.turn.step = None;
        let own = g.find_object_by_stable_id(own_stable).unwrap();
        let enemy = g.find_object_by_stable_id(enemy_stable).unwrap();
        assert_eq!(casts(&g, A, own).len(), 1);
        assert_eq!(casts(&g, A, enemy).len(), 1);
        let action = casts(&g, A, enemy).remove(0);
        announce(&mut g, action, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 16);
        resolve_stack_entry(&mut g).unwrap();
        assert!(casts(&g, A, own).is_empty());
    }
}
#[test]
fn exact_temporary_price_never_chases_a_card_that_leaves_and_returns_to_exile() {
    use ironsmith::effects::{
        EffectContext as ExecutionContext, EffectExecutor, GrantPlayTaggedDuration,
        GrantPlayTaggedEffect,
    };
    let mut g = game();
    let card = spell(&mut g, B, Zone::Exile, "{4}");
    let snap = ironsmith::snapshot::ObjectSnapshot::from_object(g.object(card).unwrap(), &g);
    let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(1234), A).with_tagged_objects(
        std::collections::HashMap::from([("old".into(), vec![snap])]),
    );
    let next = g.move_object_by_effect(card, Zone::Graveyard).unwrap();
    let next = g.move_object_by_effect(next, Zone::Exile).unwrap();
    GrantPlayTaggedEffect::new(
        "old",
        ironsmith::target::PlayerFilter::You,
        GrantPlayTaggedDuration::UntilEndOfTurn,
        false,
        false,
    )
    .with_alternative_cost(life_price())
    .execute(&mut g, &mut ctx)
    .unwrap();
    assert!(casts(&g, A, next).is_empty());
    let result =
        ironsmith::effects::CastTaggedEffect::new("old", ironsmith::target::PlayerFilter::You)
            .with_alternative_cost(life_price())
            .execute(&mut g, &mut ctx)
            .unwrap();
    assert!(g.stack_is_empty(), "{result:?}");
    assert_eq!(g.player(A).unwrap().life, 20);
}
#[test]
fn effect_price_cost_payload_round_trip_reencodes_native_components_without_erasure() {
    let effect = ironsmith::Effect::new(
        ironsmith::effects::CastTaggedEffect::new("it", ironsmith::target::PlayerFilter::You)
            .with_alternative_cost(life_price()),
    );
    let wire =
        ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(effect).unwrap();
    let round =
        ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire.clone()).unwrap();
    let actual = round
        .downcast_ref::<ironsmith::effects::CastTaggedEffect>()
        .unwrap();
    assert!(
        actual
            .alternative_cost
            .as_ref()
            .unwrap()
            .has_non_mana_costs()
    );
    let encoded = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(
        ironsmith::Effect::new(actual.clone()),
    )
    .unwrap();
    assert_eq!(encoded, wire);
}

#[test]
fn inside_information_announces_x_but_its_later_spell_price_uses_that_spells_mana_value() {
    for def in definitions("Inside Information") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Hand);
        let card = spell(&mut g, B, Zone::Library, "{4}");
        let stable = g.object(card).unwrap().stable_id;
        g.player_mut(A).unwrap().mana_pool.black = 2;
        g.player_mut(A).unwrap().mana_pool.colorless = 1;
        let action = casts(&g, A, source)
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::Normal,
                        ..
                    }
                )
            })
            .unwrap();
        let mut dm = Choices {
            x: 1,
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        announce(&mut g, action, &mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        let card = g.find_object_by_stable_id(stable).unwrap();
        let options = casts(&g, A, card);
        assert_eq!(options.len(), 1);
        announce(&mut g, options[0].clone(), &mut Choices::default());
        assert_eq!(g.player(A).unwrap().life, 16);
        resolve_stack_entry(&mut g).unwrap();
        assert!(
            g.battlefield
                .iter()
                .any(|id| g.object(*id).unwrap().stable_id == stable)
        );
    }
}
#[test]
fn pending_discard_after_a_life_component_restores_the_whole_effect_cast_attempt() {
    struct Pending {
        waiting: bool,
    }
    impl DecisionMaker for Pending {
        fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> {
            self.waiting = true;
            Vec::new()
        }
        fn awaiting_choice(&self) -> bool {
            self.waiting
        }
    }
    let mut g = game();
    let card = spell(&mut g, B, Zone::Exile, "{7}");
    let hand = printed(&mut g, A, Zone::Hand, "Retained hand card", "Type: Land");
    let cost = ironsmith::TotalCost::from_costs(vec![
        ironsmith::costs::Cost::life(3),
        ironsmith::costs::Cost::discard(1, None),
    ]);
    let mut dm = Pending { waiting: false };
    execute_priced(&mut g, card, cost, &mut dm).unwrap();
    assert!(dm.waiting);
    assert!(g.stack_is_empty());
    assert_eq!(g.player(A).unwrap().life, 20);
    assert_eq!(g.object(card).unwrap().zone, Zone::Exile);
    assert_eq!(g.object(hand).unwrap().zone, Zone::Hand);
    assert!(g.player(A).unwrap().graveyard.is_empty());
}
#[test]
fn zero_life_alternative_can_cast_a_no_mana_cost_spell() {
    let mut g = game();
    let card = printed(
        &mut g,
        A,
        Zone::Exile,
        "No mana cost",
        "Type: Sorcery\nYou gain 2 life.",
    );
    execute_priced(&mut g, card, life_price(), &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(g.stack.len(), 1);
    assert_eq!(g.player(A).unwrap().life, 20);
    resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.player(A).unwrap().life, 22);
}

#[test]
fn gwenom_retains_current_top_view_and_mandatory_price_for_the_whole_duration() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::game_loop::{
        apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
    };
    for def in definitions("Gwenom, Remorseless") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
        spell(&mut g, A, Zone::Library, "{4}");
        spell(&mut g, A, Zone::Library, "{4}");
        g.remove_summoning_sickness(source);
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        let mut combat = CombatState::default();
        let mut q = TriggerQueue::new();
        let mut dm = Choices::default();
        apply_attacker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration {
                creature: source,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        g.turn.phase = Phase::NextMain;
        g.turn.step = None;
        let top = *g.player(A).unwrap().library.last().unwrap();
        let lower = g.player(A).unwrap().library[0];
        assert_eq!(casts(&g, A, top).len(), 1);
        assert!(casts(&g, A, lower).is_empty());
        assert!(
            g.effect_store
                .grant_registry
                .get_grants_for_card(&g, top, Zone::Library, A)
                .iter()
                .any(|grant| grant.play_from_constraints.may_look_at_top
                    && grant.play_from_constraints.top_card_only)
        );
        let action = casts(&g, A, top).remove(0);
        announce(&mut g, action, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 16);
        resolve_stack_entry(&mut g).unwrap();
        assert_eq!(
            casts(&g, A, lower).len(),
            1,
            "the permission follows the new top"
        );
        g.turn.turn_number += 1;
        assert!(casts(&g, A, lower).is_empty());
    }
}
#[test]
fn xanders_pact_uses_each_opponents_card_and_requires_the_spells_own_life_price() {
    for def in definitions("Xander's Pact") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Hand);
        let their = spell(&mut g, B, Zone::Library, "{3}");
        let stable = g.object(their).unwrap().stable_id;
        let own = spell(&mut g, A, Zone::Library, "{9}");
        g.player_mut(A).unwrap().mana_pool.black = 2;
        g.player_mut(A).unwrap().mana_pool.colorless = 4;
        let mut dm = Choices {
            accept: Some(false),
            ..Default::default()
        };
        let action = casts(&g, A, source)
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::Normal,
                        ..
                    }
                )
            })
            .unwrap();
        announce(&mut g, action, &mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(g.object(own).unwrap().zone, Zone::Library);
        let their = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(g.object(their).unwrap().zone, Zone::Exile);
        let actions = casts(&g, A, their);
        assert_eq!(actions.len(), 1);
        announce(&mut g, actions[0].clone(), &mut Choices::default());
        assert_eq!(g.player(A).unwrap().life, 17);
        resolve_stack_entry(&mut g).unwrap();
    }
}

#[test]
fn life_price_is_derived_after_the_selected_prototype_characteristics() {
    struct Prototype;
    impl DecisionMaker for Prototype {
        fn decide_options(
            &mut self,
            g: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|choice| choice.description.contains("Prototype"))
                .map(|choice| vec![choice.index])
                .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(g, ctx))
        }
    }
    let mut g = game();
    let card = printed(
        &mut g,
        B,
        Zone::Exile,
        "Selected prototype",
        "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 7/7\nPrototype {1}{G} — 2/3",
    );
    execute_priced(&mut g, card, life_price(), &mut Prototype).unwrap();
    let stack = g.stack.last().unwrap().object_id;
    assert!(g.object(stack).unwrap().prototype_cast_state.is_some());
    assert_eq!(g.player(A).unwrap().life, 18);
    assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
}
#[test]
fn a_referenced_split_card_uses_the_chosen_spell_face_for_the_life_price() {
    struct Back;
    impl DecisionMaker for Back {
        fn decide_options(
            &mut self,
            g: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description == "Choose which spell to cast" {
                vec![1]
            } else {
                SelectFirstDecisionMaker.decide_options(g, ctx)
            }
        }
    }
    let mut g = game();
    let mut front = compile_to_runtime_definition(
        "Large half",
        "Mana cost: {6}\nType: Sorcery\nYou gain 6 life.",
        false,
    )
    .unwrap();
    let mut back = compile_to_runtime_definition(
        "Small half",
        "Mana cost: {1}\nType: Sorcery\nYou gain 1 life.",
        false,
    )
    .unwrap();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = LinkedFaceLayout::Split;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = LinkedFaceLayout::Split;
    g.register_linked_face_definition(&front);
    g.register_linked_face_definition(&back);
    let card = g.create_object_from_definition(&front, B, Zone::Exile);
    execute_priced(&mut g, card, life_price(), &mut Back).unwrap();
    assert_eq!(g.player(A).unwrap().life, 19);
    assert_eq!(
        g.object(g.stack.last().unwrap().object_id)
            .unwrap()
            .name
            .as_str(),
        "Small half"
    );
    resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.player(A).unwrap().life, 20);
}

#[test]
fn blue_mages_cane_keeps_job_token_attachment_defender_filter_and_paid_copy() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    use ironsmith::game_loop::{
        apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
    };
    for def in definitions("Blue Mage's Cane") {
        let mut g = game();
        let equipment = g.create_object_from_definition(&def, A, Zone::Hand);
        g.player_mut(A).unwrap().mana_pool.blue = 1;
        g.player_mut(A).unwrap().mana_pool.colorless = 2;
        let action = casts(&g, A, equipment)
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::Normal,
                        ..
                    }
                )
            })
            .unwrap();
        let stable = g.object(equipment).unwrap().stable_id;
        announce(&mut g, action, &mut SelectFirstDecisionMaker);
        settle(&mut g);
        let equipment = g.find_object_by_stable_id(stable).unwrap();
        let hero = g
            .object(equipment)
            .unwrap()
            .attached_to
            .as_ref()
            .and_then(|target| target.object_id())
            .expect("job select attached to its Hero");
        assert_eq!(g.current_power(hero), Some(1));
        assert_eq!(g.current_toughness(hero), Some(3));
        assert!(
            g.current_subtypes(hero)
                .unwrap()
                .contains(&ironsmith::Subtype::Wizard)
        );
        g.remove_summoning_sickness(hero);
        let target = printed(
            &mut g,
            B,
            Zone::Graveyard,
            "Copied spell",
            "Mana cost: {8}\nType: Instant\nYou gain 2 life.",
        );
        let target_stable = g.object(target).unwrap().stable_id;
        let own = printed(
            &mut g,
            A,
            Zone::Graveyard,
            "Own graveyard spell",
            "Mana cost: {1}\nType: Instant\nYou gain 1 life.",
        );
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        let mut combat = CombatState::default();
        let mut q = TriggerQueue::new();
        apply_attacker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration {
                creature: hero,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        let mut dm = Choices {
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut dm).unwrap();
        g.player_mut(A).unwrap().mana_pool.colorless = 3;
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(
            g.player(A).unwrap().mana_pool.total(),
            0,
            "stack={:?}, target_zone={:?}",
            g.stack
                .iter()
                .map(|entry| (
                    g.object(entry.object_id).map(|o| o.name.clone()),
                    &entry.targets
                ))
                .collect::<Vec<_>>(),
            g.object(target).map(|o| o.zone)
        );
        assert_eq!(g.object(own).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            g.object(g.find_object_by_stable_id(target_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
        assert_eq!(g.stack.len(), 1);
        resolve_stack_entry(&mut g).unwrap();
        assert_eq!(g.player(A).unwrap().life, 22);
    }
}

#[test]
fn direct_and_dispatcher_priced_casts_share_one_resource_allowance_across_all_costs() {
    use ironsmith::effects::{
        CastTaggedEffect, EffectContext as ExecutionContext, EffectExecutor, ExecutionError,
    };
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::target::PlayerFilter;
    for dispatcher in [false, true] {
        for limit in [1, 2] {
            let mut g = game();
            let provider = printed(
                &mut g,
                A,
                Zone::Battlefield,
                "Replacement source",
                "Type: Artifact",
            );
            let card = spell(&mut g, B, Zone::Exile, "{8}");
            let first = printed(&mut g, A, Zone::Hand, "First discard", "Type: Land");
            let second = printed(&mut g, A, Zone::Hand, "Second discard", "Type: Land");
            let replacement = g.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    provider,
                    A,
                    ironsmith::events::cards::matchers::WouldDiscardMatcher::you(),
                    ReplacementAction::Additionally(vec![ironsmith::Effect::new(
                        ironsmith::effects::CreateTokenEffect::you(
                            ironsmith::cards::tokens::treasure_token_definition(),
                            1,
                        ),
                    )]),
                ),
            );
            g.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits {
                max_created_tokens: limit,
                max_instructions: limit,
                max_nesting: 1,
                ..Default::default()
            });
            g.take_pending_trigger_events();
            let next_id = g.next_object_id_counter();
            let snapshot =
                ironsmith::snapshot::ObjectSnapshot::from_object(g.object(card).unwrap(), &g);
            let effect = CastTaggedEffect::new("priced", PlayerFilter::You).with_alternative_cost(
                ironsmith::TotalCost::from_costs(vec![
                    ironsmith::costs::Cost::life(3),
                    ironsmith::costs::Cost::discard(1, None),
                    ironsmith::costs::Cost::discard(1, None),
                ]),
            );
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(provider, A, &mut dm).with_tagged_objects(
                std::collections::HashMap::from([("priced".into(), vec![snapshot])]),
            );
            let result = if dispatcher {
                ironsmith::effects::execute_effect(
                    &mut g,
                    &ironsmith::Effect::new(effect),
                    &mut ctx,
                )
            } else {
                effect.execute(&mut g, &mut ctx)
            };
            if limit == 1 {
                assert!(
                    matches!(result, Err(ExecutionError::ResourceLimitExceeded { .. })),
                    "{result:?}"
                );
                assert_eq!(g.player(A).unwrap().life, 20);
                assert!(g.stack_is_empty());
                for id in [first, second] {
                    assert_eq!(g.object(id).unwrap().zone, Zone::Hand);
                }
                assert_eq!(g.object(card).unwrap().zone, Zone::Exile);
                assert_eq!(g.next_object_id_counter(), next_id);
                assert_eq!(g.battlefield.len(), 1);
                assert!(
                    g.effect_store
                        .replacement_effects
                        .get_effect(replacement)
                        .is_some()
                );
                assert!(g.take_pending_trigger_events().is_empty());
            } else {
                result.unwrap();
                assert_eq!(g.player(A).unwrap().life, 17);
                assert_eq!(g.player(A).unwrap().graveyard.len(), 2);
                assert_eq!(g.battlefield.len(), 3);
                assert_eq!(g.stack.len(), 1);
            }
        }
    }
}

#[test]
fn jadzi_and_journey_both_complete_payloads_round_trip_as_the_exact_frozen_identity() {
    let row = rows()
        .into_iter()
        .find(|row| row["frozen_name"] == "Jadzi, Oracle of Arcavios // Journey to the Oracle")
        .unwrap();
    for definition in definitions("Jadzi, Oracle of Arcavios") {
        assert!(
            definition.abilities.iter().any(|ability| matches!(
                ability.kind,
                ironsmith::ability::AbilityKind::Activated(_)
            ))
        );
        assert!(
            definition.abilities.iter().any(|ability| matches!(
                ability.kind,
                ironsmith::ability::AbilityKind::Triggered(_)
            ))
        );
    }
    let back = &row["other_face"];
    for definition in pair(
        back["name"].as_str().unwrap(),
        back["text"].as_str().unwrap(),
    ) {
        assert!(definition.card.card_types.contains(&CardType::Sorcery));
        assert!(
            ironsmith_text::compiled_text_lines(&definition)
                .join("\n")
                .contains("eight")
        );
    }
}
#[test]
fn jadzi_magecraft_pays_one_for_the_revealed_spell_and_puts_lands_without_playing_them() {
    for definition in definitions("Jadzi, Oracle of Arcavios") {
        for land in [false, true] {
            let mut g = game();
            g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let revealed = printed(
                &mut g,
                A,
                Zone::Library,
                "Magecraft reveal",
                if land {
                    "Type: Land"
                } else {
                    "Mana cost: {7}\nType: Creature — Bear\nPower/Toughness: 2/2"
                },
            );
            let stable = g.object(revealed).unwrap().stable_id;
            let cast = printed(
                &mut g,
                A,
                Zone::Hand,
                "Magecraft enabler",
                "Mana cost: {0}\nType: Instant\nYou gain 1 life.",
            );
            g.player_mut(A).unwrap().mana_pool.colorless = 1;
            let action = casts(&g, A, cast)
                .into_iter()
                .find(|a| {
                    matches!(
                        a,
                        LegalAction::CastSpell {
                            casting_method: CastingMethod::Normal,
                            ..
                        }
                    )
                })
                .unwrap();
            let mut dm = Choices::default();
            announce(&mut g, action, &mut dm);
            assert_eq!(g.stack.len(), 2, "magecraft is above the original spell");
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            let current = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                g.object(current).unwrap().zone,
                if land { Zone::Battlefield } else { Zone::Stack }
            );
            assert_eq!(g.player(A).unwrap().mana_pool.total(), u32::from(land));
            if !land {
                resolve_stack_entry(&mut g).unwrap();
            }
            resolve_stack_entry(&mut g).unwrap();
            assert_eq!(g.player(A).unwrap().life, 21);
        }
    }
}
#[test]
fn jadzi_discards_a_real_card_to_return_only_itself_to_its_owners_hand() {
    for definition in definitions("Jadzi, Oracle of Arcavios") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = g.object(source).unwrap().stable_id;
        let discard = printed(&mut g, A, Zone::Hand, "Escape payment", "Type: Land");
        let action = compute_legal_actions(&g, A)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
            .unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = Choices {
            objects: vec![discard],
            ..Default::default()
        };
        let mut progress = apply_priority_response_with_dm(
            &mut g,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..64 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress =
                apply_decision_context_with_dm(&mut g, &mut queue, &mut state, &ctx, &mut dm)
                    .unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(g.player(A).unwrap().graveyard.len(), 1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(
            g.object(g.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Hand
        );
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn journey_checks_eight_lands_after_putting_them_and_requires_the_optional_discard_for_return() {
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == "Jadzi, Oracle of Arcavios")
        .unwrap();
    let back = &row["other_face"];
    for definition in pair(
        back["name"].as_str().unwrap(),
        back["text"].as_str().unwrap(),
    ) {
        for eighth in [false, true] {
            let mut g = game();
            for _ in 0..7 {
                printed(&mut g, A, Zone::Battlefield, "Existing land", "Type: Land");
            }
            let incoming =
                eighth.then(|| printed(&mut g, A, Zone::Hand, "Eighth land", "Type: Land"));
            let discard = printed(
                &mut g,
                A,
                Zone::Hand,
                "Journey return payment",
                "Type: Artifact",
            );
            let source = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(source).unwrap().stable_id;
            g.player_mut(A).unwrap().mana_pool.green = 2;
            g.player_mut(A).unwrap().mana_pool.colorless = 2;
            let action = casts(&g, A, source)
                .into_iter()
                .find(|a| {
                    matches!(
                        a,
                        LegalAction::CastSpell {
                            casting_method: CastingMethod::Normal,
                            ..
                        }
                    )
                })
                .unwrap();
            let mut dm = Choices {
                objects: incoming.into_iter().chain([discard]).collect(),
                ..Default::default()
            };
            announce(&mut g, action, &mut dm);
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            let current = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                g.object(current).unwrap().zone,
                if eighth { Zone::Hand } else { Zone::Graveyard }
            );
            assert_eq!(g.battlefield.len(), if eighth { 8 } else { 7 });
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            if !eighth {
                assert_eq!(g.object(discard).unwrap().zone, Zone::Hand);
            }
        }
    }
}

#[test]
fn journey_already_at_eight_lands_can_decline_land_put_and_independently_choose_discard() {
    struct JourneyChoices {
        choices: Choices,
        decisions: std::collections::VecDeque<bool>,
        descriptions: Vec<String>,
    }
    impl DecisionMaker for JourneyChoices {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            assert_eq!(ctx.player, A);
            assert!(ctx.can_accept);
            self.descriptions.push(ctx.description.clone());
            self.decisions
                .pop_front()
                .expect("only the two printed optional actions")
        }
        fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
            self.choices.decide_objects(g, ctx)
        }
    }
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == "Jadzi, Oracle of Arcavios")
        .unwrap();
    let back = &row["other_face"];
    for definition in pair(
        back["name"].as_str().unwrap(),
        back["text"].as_str().unwrap(),
    ) {
        for accept_discard in [false, true] {
            let mut g = game();
            for _ in 0..8 {
                printed(&mut g, A, Zone::Battlefield, "Existing land", "Type: Land");
            }
            let held_land = printed(&mut g, A, Zone::Hand, "Declined ninth land", "Type: Land");
            let discard = printed(
                &mut g,
                A,
                Zone::Hand,
                "Independent return payment",
                "Type: Artifact",
            );
            let discard_stable = g.object(discard).unwrap().stable_id;
            let source = g.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = g.object(source).unwrap().stable_id;
            g.player_mut(A).unwrap().mana_pool.green = 2;
            g.player_mut(A).unwrap().mana_pool.colorless = 2;
            let action = casts(&g, A, source)
                .into_iter()
                .find(|a| {
                    matches!(
                        a,
                        LegalAction::CastSpell {
                            casting_method: CastingMethod::Normal,
                            ..
                        }
                    )
                })
                .unwrap();
            announce(&mut g, action, &mut Choices::default());
            let mut dm = JourneyChoices {
                choices: Choices {
                    objects: vec![discard],
                    ..Default::default()
                },
                decisions: [false, accept_discard].into(),
                descriptions: Vec::new(),
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert!(
                dm.decisions.is_empty(),
                "the eight-land condition is independent of the declined first option"
            );
            assert_eq!(dm.descriptions.len(), 2);
            assert_eq!(g.battlefield.len(), 8);
            assert_eq!(g.object(held_land).unwrap().zone, Zone::Hand);
            assert_eq!(
                g.object(g.find_object_by_stable_id(discard_stable).unwrap())
                    .unwrap()
                    .zone,
                if accept_discard {
                    Zone::Graveyard
                } else {
                    Zone::Hand
                }
            );
            assert_eq!(
                g.object(g.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                if accept_discard {
                    Zone::Hand
                } else {
                    Zone::Graveyard
                }
            );
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        }
    }
}
