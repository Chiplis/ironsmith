//! UNVALIDATED: exact prices preserve independently authorized origins.
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
        "../../../fixtures/independent_casting_prices.json.fixture"
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
}
impl DecisionMaker for Choices {
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
fn permit(g: &mut GameState, zone: Zone, once: bool, on_use: bool) -> ObjectId {
    let mut spec = GrantSpec::new(
        Grantable::PlayFrom,
        ironsmith::target::ObjectFilter::nonland().owned_by(ironsmith::target::PlayerFilter::You),
        zone,
    );
    spec.usage_limit = once.then_some(GrantUsageLimit::OnceEachTurn);
    if on_use {
        spec.on_use_effects = vec![ironsmith::effect::Effect::gain_life(1)];
    }
    let def = CardDefinitionBuilder::new(CardId::new(), "Independent origin")
        .card_types(vec![CardType::Enchantment])
        .with_ability(ironsmith::ability::Ability::static_ability(
            StaticAbility::grants(spec),
        ))
        .build();
    g.create_object_from_definition(&def, A, Zone::Battlefield)
}
#[test]
fn six_exact_front_bodies_and_retrieve_prey_round_trip_without_loss() {
    assert_eq!(rows().len(), 6);
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for def in definitions(name) {
            assert!(def.abilities.iter().any(|a|matches!(&a.kind,ironsmith::ability::AbilityKind::Static(s) if s.grant_spec().is_some_and(|g|matches!(g.grantable,Grantable::AlternativePrice{..})))),"{name}");
            let rendered = ironsmith_text::compiled_text_lines(&def).join("\n");
            assert!(rendered.contains("rather than"), "{rendered}");
        }
        if let Some(back) = row.get("other_face") {
            pair(
                back["name"].as_str().unwrap(),
                back["text"].as_str().unwrap(),
            );
        }
    }
}
#[test]
fn as_foretold_uses_its_own_counters_and_never_supplies_an_origin() {
    for def in definitions("As Foretold") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let hand = spell(&mut g, A, Zone::Hand, "{2}");
        let grave = spell(&mut g, A, Zone::Graveyard, "{2}");
        let exile = spell(&mut g, A, Zone::Exile, "{2}");
        let other = g.create_object_from_definition(&def, B, Zone::Battlefield);
        g.add_counters(other, CounterType::Time, 20);
        assert!(priced(&g, A, hand, source).is_none());
        g.add_counters(source, CounterType::Time, 2);
        assert!(priced(&g, A, hand, source).is_some());
        assert!(priced(&g, A, grave, source).is_none());
        assert!(priced(&g, A, exile, source).is_none());
        let action = priced(&g, A, hand, source).unwrap();
        let stack = announce(&mut g, action, &mut SelectFirstDecisionMaker);
        assert!(g.object(stack).unwrap().cast_price.is_some());
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry(&mut g).unwrap();
        let later = spell(&mut g, A, Zone::Hand, "{1}");
        assert!(priced(&g, A, later, source).is_none());
        g.set_current_controller(source, B).unwrap();
        g.turn.priority_player = Some(B);
        let b = spell(&mut g, B, Zone::Hand, "{1}");
        g.turn.active_player = B;
        assert!(
            priced(&g, B, b, source).is_some(),
            "budget belongs to the player and exact permission"
        );
    }
}
#[test]
fn separate_origin_and_price_budgets_and_on_use_commit_once_each() {
    for def in definitions("As Foretold") {
        let mut g = game();
        let price = g.create_object_from_definition(&def, A, Zone::Battlefield);
        g.add_counters(price, CounterType::Time, 8);
        let origin = permit(&mut g, Zone::Graveyard, true, true);
        let card = spell(&mut g, A, Zone::Graveyard, "{7}");
        let action = priced(&g, A, card, price).unwrap();
        let stack = announce(&mut g, action, &mut SelectFirstDecisionMaker);
        let object = g.object(stack).unwrap();
        let origin_key = object.cast_grant_usage_identity.as_deref().unwrap();
        let price_key = &object.cast_price.as_ref().unwrap().identity;
        assert_ne!(origin_key, price_key);
        assert!(
            g.turn_store
                .grant_cast_uses_this_turn
                .contains(&(A, origin_key.clone()))
        );
        assert!(
            g.turn_store
                .grant_cast_uses_this_turn
                .contains(&(A, price_key.clone()))
        );
        settle(&mut g);
        assert_eq!(g.player(A).unwrap().life, 21);
        let next = spell(&mut g, A, Zone::Graveyard, "{0}");
        assert!(casts(&g, A, next).is_empty());
        g.move_object_by_effect(origin, Zone::Exile).unwrap();
        let own = spell(&mut g, A, Zone::Hand, "{1}");
        assert!(priced(&g, A, own, price).is_none());
    }
}
#[test]
fn paying_ordinary_price_keeps_the_alternative_price_use_available() {
    for def in definitions("Darksteel Monolith") {
        let mut g = game();
        let provider = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let colored = spell(&mut g, A, Zone::Hand, "{U}");
        assert!(priced(&g, A, colored, provider).is_none());
        let card = spell(&mut g, A, Zone::Hand, "{2}");
        g.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let ordinary = casts(&g, A, card)
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
        announce(&mut g, ordinary, &mut SelectFirstDecisionMaker);
        resolve_stack_entry(&mut g).unwrap();
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        let next = spell(&mut g, A, Zone::Hand, "{7}");
        assert!(priced(&g, A, next, provider).is_some());
        permit(&mut g, Zone::Exile, false, false);
        let exiled = spell(&mut g, A, Zone::Exile, "{7}");
        assert!(priced(&g, A, exiled, provider).is_none());
    }
}
#[test]
fn conspiracy_pays_real_evidence_from_its_casters_graveyard_and_retains_taxes() {
    for def in definitions("Conspiracy Unraveler") {
        let mut g = game();
        let provider = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let card = spell(&mut g, A, Zone::Hand, "{9}");
        let own = spell(&mut g, A, Zone::Graveyard, "{9}");
        let foreign = spell(&mut g, B, Zone::Graveyard, "{20}");
        assert!(priced(&g, A, card, provider).is_none());
        let extra = spell(&mut g, A, Zone::Graveyard, "{2}");
        printed(
            &mut g,
            B,
            Zone::Battlefield,
            "Tax",
            "Type: Enchantment\nSpells cost {1} more to cast.",
        );
        assert!(priced(&g, A, card, provider).is_none());
        g.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let action = priced(&g, A, card, provider).unwrap();
        let mut dm = Choices {
            objects: vec![own, extra],
            ..Default::default()
        };
        announce(&mut g, action, &mut dm);
        assert_eq!(dm.players, vec![A]);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        assert!(g.object(own).is_none() && g.object(extra).is_none());
        assert_eq!(g.object(foreign).unwrap().zone, Zone::Graveyard);
        assert_eq!(g.exile.len(), 2);
        resolve_stack_entry(&mut g).unwrap();
        assert!(g.battlefield.iter().any(|id| {
            g.object(*id)
                .is_some_and(|o| o.name.as_ref() == "Price candidate")
        }));
    }
}
#[test]
fn nissa_pays_exact_energy_for_permanents_and_primal_timing_requires_selected_price() {
    for (name, amount) in [("Nissa, Worldsoul Speaker", 8), ("Primal Prayers", 1)] {
        for def in definitions(name) {
            let mut g = game();
            let provider = g.create_object_from_definition(&def, A, Zone::Battlefield);
            let card = spell(&mut g, A, Zone::Hand, "{3}");
            g.player_mut(A).unwrap().energy_counters = amount - 1;
            assert!(priced(&g, A, card, provider).is_none());
            g.player_mut(A).unwrap().energy_counters = amount;
            let sorcery = printed(
                &mut g,
                A,
                Zone::Hand,
                "Not a permanent",
                "Mana cost: {1}\nType: Sorcery\nYou gain 1 life.",
            );
            assert!(priced(&g, A, sorcery, provider).is_none());
            if name == "Primal Prayers" {
                g.turn.active_player = B;
                g.turn.phase = Phase::Ending;
                g.turn.step = Some(Step::End);
                g.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 3);
                assert!(!casts(&g, A, card).iter().any(|a| matches!(
                    a,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::Normal,
                        ..
                    }
                )));
                let large = spell(&mut g, A, Zone::Hand, "{4}");
                assert!(priced(&g, A, large, provider).is_none());
            }
            let action = priced(&g, A, card, provider).unwrap();
            announce(&mut g, action, &mut SelectFirstDecisionMaker);
            assert_eq!(g.player(A).unwrap().energy_counters, 0);
            resolve_stack_entry(&mut g).unwrap();
        }
    }
}
#[test]
fn mandatory_graveyard_additional_cost_survives_the_replaced_mana_price() {
    for def in definitions("As Foretold") {
        let mut g = game();
        let provider = g.create_object_from_definition(&def, A, Zone::Battlefield);
        g.add_counters(provider, CounterType::Time, 10);
        printed(
            &mut g,
            A,
            Zone::Battlefield,
            "Extra-price origin",
            "Type: Enchantment\nOnce during each of your turns, you may cast a creature spell from your graveyard by paying 3 life in addition to paying its other costs.",
        );
        let card = spell(&mut g, A, Zone::Graveyard, "{8}");
        let action = priced(&g, A, card, provider).unwrap();
        let mut forged = action.clone();
        if let LegalAction::CastSpell {
            casting_method:
                CastingMethod::AlternativePrice {
                    origin_permission, ..
                },
            ..
        } = &mut forged
        {
            *origin_permission = None;
        }
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        assert!(
            apply_priority_response_with_dm(
                &mut g,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(forged),
                &mut SelectFirstDecisionMaker
            )
            .is_err()
        );
        assert_eq!(g.object(card).unwrap().zone, Zone::Graveyard);
        assert_eq!(g.player(A).unwrap().life, 20);
        assert!(g.turn_store.grant_cast_uses_this_turn.is_empty());

        announce(&mut g, action, &mut SelectFirstDecisionMaker);
        assert_eq!(g.player(A).unwrap().life, 17);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn no_second_alternative_price_combines_with_flashback_or_face_down() {
    for def in definitions("As Foretold") {
        let mut g = game();
        let provider = g.create_object_from_definition(&def, A, Zone::Battlefield);
        g.add_counters(provider, CounterType::Time, 20);
        let flashback = printed(
            &mut g,
            A,
            Zone::Graveyard,
            "Flashback candidate",
            "Mana cost: {2}\nType: Sorcery\nYou gain 1 life.\nFlashback {3}",
        );
        assert!(priced(&g, A, flashback, provider).is_none());
        let morph = printed(
            &mut g,
            A,
            Zone::Hand,
            "Morph candidate",
            "Mana cost: {8}\nType: Creature\nPower/Toughness: 4/4\nMorph {2}",
        );
        for action in casts(&g, A, morph) {
            if let LegalAction::CastSpell {
                casting_method: CastingMethod::AlternativePrice { origin, .. },
                ..
            } = action
            {
                assert!(!matches!(
                    *origin,
                    CastingMethod::FaceDown
                        | CastingMethod::FaceDownPlayFrom { .. }
                        | CastingMethod::AlternativePrice { .. }
                ));
            }
        }
    }
}
#[test]
fn retrieve_prey_creates_the_separate_exile_permission_hunter_never_invents() {
    for def in definitions("Tlincalli Hunter") {
        let mut g = game();
        let hunter = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let grave = spell(&mut g, A, Zone::Graveyard, "{7}");
        let unlinked = spell(&mut g, A, Zone::Exile, "{7}");
        assert!(priced(&g, A, unlinked, hunter).is_none());
        let back = rows()
            .into_iter()
            .find_map(|r| r.get("other_face").cloned())
            .unwrap();
        let retrieve = printed(
            &mut g,
            A,
            Zone::Hand,
            back["name"].as_str().unwrap(),
            back["text"].as_str().unwrap(),
        );
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 1);
        g.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let ordinary = casts(&g, A, retrieve)
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
        announce(
            &mut g,
            ordinary,
            &mut Choices {
                targets: vec![Target::Object(grave)],
                ..Default::default()
            },
        );
        resolve_stack_entry(&mut g).unwrap();
        let exiled = g
            .exile
            .iter()
            .copied()
            .find(|id| {
                g.object(*id)
                    .is_some_and(|o| o.name.as_ref() == "Price candidate" && *id != unlinked)
            })
            .unwrap();
        let action = priced(&g, A, exiled, hunter).unwrap();
        announce(&mut g, action, &mut SelectFirstDecisionMaker);
        resolve_stack_entry(&mut g).unwrap();
        assert!(priced(&g, A, unlinked, hunter).is_none());
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn selected_face_filters_and_both_identities_survive_provider_sacrifice() {
    let mut g = game();
    let provider = g.create_object_from_definition(
        &definitions("Nissa, Worldsoul Speaker")[0],
        A,
        Zone::Battlefield,
    );
    g.player_mut(A).unwrap().energy_counters = 8;
    permit(&mut g, Zone::Graveyard, false, true);
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Priced front")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::new())
        .other_face(back_id)
        .other_face_name("Priced back")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let mut back=compile_to_runtime_definition("Priced back","Mana cost: {7}\nType: Creature\nPower/Toughness: 3/3\nAs an additional cost to cast this spell, sacrifice a creature.",false).unwrap();
    back.card.id = back_id;
    back.card.other_face = Some(front_id);
    back.card.other_face_name = Some("Priced front".into());
    back.card.linked_face_layout = LinkedFaceLayout::TransformLike;
    g.register_linked_face_definition(&front);
    g.register_linked_face_definition(&back);
    let card = g.create_object_from_definition(&front, A, Zone::Graveyard);
    let action = priced(&g, A, card, provider).unwrap();
    assert!(
        matches!(&action,LegalAction::CastSpell{casting_method:CastingMethod::AlternativePrice{origin,..},..} if matches!(origin.as_ref(),CastingMethod::SplitOtherHalfPlayFrom{..}))
    );
    let stack = announce(
        &mut g,
        action,
        &mut Choices {
            objects: vec![provider],
            ..Default::default()
        },
    );
    assert!(g.object(provider).is_none());
    assert_eq!(g.object(stack).unwrap().name.as_ref(), "Priced back");
    assert_eq!(g.player(A).unwrap().energy_counters, 0);
    assert!(g.object(stack).unwrap().cast_price.is_some());
    settle(&mut g);
    assert_eq!(g.player(A).unwrap().life, 21);
}
#[test]
fn public_zone_cancellation_restores_both_budgets_and_can_retry_the_same_route() {
    struct Cancel {
        canceled: bool,
    }
    impl DecisionMaker for Cancel {
        fn decide_mana_payment(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::ManaPaymentContext,
        ) -> ironsmith::mana_payment::ManaPaymentResponse {
            self.canceled = true;
            ironsmith::mana_payment::ManaPaymentResponse::Cancel
        }
    }
    let mut g = game();
    let provider =
        g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
    g.add_counters(provider, CounterType::Time, 9);
    permit(&mut g, Zone::Graveyard, true, true);
    printed(
        &mut g,
        B,
        Zone::Battlefield,
        "Public tax",
        "Type: Enchantment\nSpells cost {1} more to cast.",
    );
    let card = spell(&mut g, A, Zone::Graveyard, "{8}");
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    let action = priced(&g, A, card, provider).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut q = TriggerQueue::new();
    let mut dm = Cancel { canceled: false };
    let mut result = apply_priority_response_with_dm(
        &mut g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..32 {
        match result {
            Ok(ironsmith::GameProgress::NeedsDecisionCtx(ref ctx))
                if state.has_pending_action() =>
            {
                let ctx = ctx.clone();
                result = apply_decision_context_with_dm(&mut g, &mut q, &mut state, &ctx, &mut dm);
            }
            _ => break,
        }
    }
    // Cancellation rolls back and returns to priority through a successful response.
    result.unwrap();
    assert!(dm.canceled);
    assert!(!state.has_pending_action());
    assert_eq!(g.object(card).unwrap().zone, Zone::Graveyard);
    assert!(g.turn_store.grant_cast_uses_this_turn.is_empty());
    assert_eq!(g.player(A).unwrap().mana_pool.total(), 1);
    let retry = priced(&g, A, card, provider).unwrap();
    announce(&mut g, retry, &mut SelectFirstDecisionMaker);
    settle(&mut g);
    assert_eq!(g.player(A).unwrap().life, 21);
}
#[test]
fn free_alternative_price_locks_printed_x_to_zero_and_rejects_stale_price_identity() {
    let mut g = game();
    let provider =
        g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
    g.add_counters(provider, CounterType::Time, 3);
    let card = printed(
        &mut g,
        A,
        Zone::Hand,
        "X price",
        "Mana cost: {X}\nType: Sorcery\nYou gain X life.",
    );
    let action = priced(&g, A, card, provider).unwrap();
    let stale = action.clone();
    let stack = announce(&mut g, action, &mut SelectFirstDecisionMaker);
    assert_eq!(g.object(stack).unwrap().x_value, Some(0));
    resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.player(A).unwrap().life, 20);
    let mut state = PriorityLoopState::new(2);
    let mut q = TriggerQueue::new();
    assert!(
        apply_priority_response_with_dm(
            &mut g,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(stale),
            &mut SelectFirstDecisionMaker
        )
        .is_err()
    );
}
#[test]
fn jump_start_additional_discard_and_departure_replacement_remain_with_price() {
    let mut g = game();
    let provider =
        g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
    g.add_counters(provider, CounterType::Time, 5);
    let card = printed(
        &mut g,
        A,
        Zone::Graveyard,
        "Jump price",
        "Mana cost: {4}\nType: Sorcery\nYou gain 1 life.\nJump-start",
    );
    assert!(priced(&g, A, card, provider).is_none());
    let discarded = spell(&mut g, A, Zone::Hand, "{2}");
    let action = priced(&g, A, card, provider).unwrap();
    let stack = announce(
        &mut g,
        action,
        &mut Choices {
            objects: vec![discarded],
            ..Default::default()
        },
    );
    assert!(g.object(discarded).is_none());
    assert!(matches!(
        g.object(stack).unwrap().cast_alternative_method.as_deref(),
        Some(ironsmith::alternative_cast::AlternativeCastingMethod::JumpStart { .. })
    ));
    resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.player(A).unwrap().life, 21);
    assert!(g.exile.iter().any(|id| {
        g.object(*id)
            .is_some_and(|o| o.name.as_ref() == "Jump price")
    }));
}
#[test]
fn upkeep_landfall_and_entry_energy_bodies_still_execute() {
    for def in definitions("As Foretold") {
        let mut g = game();
        let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
        for player in [B, A] {
            g.queue_trigger_event(
                Default::default(),
                ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::BeginningOfUpkeepEvent::new(player),
                    Default::default(),
                ),
            );
            settle(&mut g);
            assert_eq!(
                g.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::Time)
                    .copied()
                    .unwrap_or(0),
                u32::from(player == A)
            );
        }
    }
    for name in ["Primal Prayers", "Nissa, Worldsoul Speaker"] {
        for def in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&def, A, Zone::Hand);
            g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 4);
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
            announce(&mut g, action, &mut SelectFirstDecisionMaker);
            settle(&mut g);
            if name == "Primal Prayers" {
                assert_eq!(g.player(A).unwrap().energy_counters, 2);
            } else {
                for player in [B, A] {
                    g.turn.active_player = player;
                    g.turn.priority_player = Some(player);
                    let land = printed(&mut g, player, Zone::Hand, "Landfall input", "Type: Land");
                    ironsmith::special_actions::perform(
                        ironsmith::special_actions::SpecialAction::PlayLand { card_id: land },
                        &mut g,
                        player,
                        &mut SelectFirstDecisionMaker,
                    )
                    .unwrap();
                    settle(&mut g);
                    assert_eq!(
                        g.player(A).unwrap().energy_counters,
                        u32::from(player == A) * 2
                    );
                }
            }
        }
    }
}
#[test]
fn commander_tax_remains_payable_and_a_command_zone_price_provider_is_inactive() {
    let mut g = game();
    let inactive = g.create_object_from_definition(
        &definitions("Nissa, Worldsoul Speaker")[0],
        A,
        Zone::Command,
    );
    g.set_as_commander(inactive, A);
    g.player_mut(A).unwrap().energy_counters = 8;
    let own = spell(&mut g, A, Zone::Hand, "{8}");
    assert!(priced(&g, A, own, inactive).is_none());
    let provider =
        g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
    g.add_counters(provider, CounterType::Time, 4);
    g.record_commander_cast_from_command_zone(inactive);
    assert!(priced(&g, A, inactive, provider).is_none());
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    g.phase_out(provider);
    assert!(priced(&g, A, inactive, provider).is_none());
    g.phase_in(provider);
    let action = priced(&g, A, inactive, provider).unwrap();
    announce(&mut g, action, &mut SelectFirstDecisionMaker);
    assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
    assert_eq!(g.commander_cast_count(inactive), 2);
}
#[test]
fn resolving_instruction_can_select_a_price_without_leaking_its_origin_permission() {
    struct PickPrice {
        offers: usize,
    }
    impl DecisionMaker for PickPrice {
        fn decide_options(
            &mut self,
            g: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| option.description.contains("alternative price"))
            {
                self.offers += 1;
                vec![option.index]
            } else {
                SelectFirstDecisionMaker.decide_options(g, ctx)
            }
        }
    }
    for free in [false, true] {
        let mut g = game();
        let provider =
            g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
        g.add_counters(provider, CounterType::Time, 9);
        let card = spell(&mut g, B, Zone::Exile, "{8}");
        let other = spell(&mut g, B, Zone::Exile, "{8}");
        assert!(priced(&g, A, card, provider).is_none());
        let source = printed(&mut g, A, Zone::Stack, "Resolving origin", "Type: Instant");
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object(g.object(card).unwrap(), &g);
        let mut effect = ironsmith::effects::CastTaggedEffect::new(
            "effect-card",
            ironsmith::target::PlayerFilter::You,
        );
        effect.without_paying_mana_cost = free;
        let mut dm = PickPrice { offers: 0 };
        let outcome = {
            let mut ctx = ironsmith::effects::EffectContext::new(source, A, &mut dm);
            ctx.tag_object("effect-card", snapshot);
            ironsmith::effects::execute_effect(
                &mut g,
                &ironsmith::effect::Effect::new(effect),
                &mut ctx,
            )
            .unwrap()
        };
        assert!(!outcome.events.is_empty());
        assert_eq!(dm.offers, usize::from(!free));
        assert!(priced(&g, A, other, provider).is_none());
        let cast = g.stack.last().unwrap().object_id;
        assert_eq!(g.controller_of_id(cast), Some(A));
        assert_eq!(g.object(cast).unwrap().cast_price.is_some(), !free);
        resolve_stack_entry(&mut g).unwrap();
        assert!(priced(&g, A, other, provider).is_none());
    }
}

#[test]
fn conspiracys_evidence_price_does_not_pay_the_spells_linked_optional_evidence_cost() {
    let mut g = game();
    let source = g.create_object_from_definition(
        &definitions("Conspiracy Unraveler")[0],
        A,
        Zone::Battlefield,
    );
    let evidence = spell(&mut g, A, Zone::Graveyard, "{10}");
    let victim = printed(
        &mut g,
        B,
        Zone::Battlefield,
        "Mask target",
        "Type: Creature
Power/Toughness: 7/7",
    );
    let card = printed(
        &mut g,
        A,
        Zone::Hand,
        "Behind the Mask",
        "Mana cost: {U}\nType: Instant\nAs an additional cost to cast this spell, you may collect evidence 6. (Exile cards with total mana value 6 or greater from your graveyard.)\nUntil end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/3. If evidence was collected, it has base power and toughness 1/1 until end of turn instead.",
    );
    let action = priced(&g, A, card, source).unwrap();
    let stack = announce(
        &mut g,
        action,
        &mut Choices {
            objects: vec![evidence],
            targets: vec![Target::Object(victim)],
            ..Default::default()
        },
    );
    assert!(
        !g.object(stack)
            .unwrap()
            .optional_costs_paid
            .was_paid_label("Evidence")
    );
    resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.current_power(victim), Some(4));
    assert_eq!(g.current_toughness(victim), Some(3));
}

#[test]
fn hunter_prices_a_real_adventure_exile_for_its_designated_caster_and_only_its_normal_face() {
    for hunter_def in definitions("Tlincalli Hunter") {
        let mut g = game();
        let hunter = g.create_object_from_definition(&hunter_def, A, Zone::Battlefield);
        let mut front = compile_to_runtime_definition(
            "Adventure creature",
            "Mana cost: {7}\nType: Creature\nPower/Toughness: 4/4",
            false,
        )
        .unwrap();
        let mut back = compile_to_runtime_definition(
            "Small adventure",
            "Mana cost: {1}\nType: Sorcery — Adventure\nYou gain 1 life.",
            false,
        )
        .unwrap();
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.to_string());
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike;
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.to_string());
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike;
        g.register_linked_face_definition(&front);
        g.register_linked_face_definition(&back);
        let card = g.create_object_from_definition(&front, A, Zone::Hand);
        let stable = g.object(card).unwrap().stable_id;
        g.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let action = casts(&g, A, card)
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell {
                        casting_method: CastingMethod::SplitOtherHalf,
                        ..
                    }
                )
            })
            .unwrap();
        announce(&mut g, action, &mut SelectFirstDecisionMaker);
        resolve_stack_entry(&mut g).unwrap();
        let exiled = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(g.adventure_exiled_player(exiled), Some(A));
        let action = priced(&g, A, exiled, hunter).unwrap();
        assert!(
            matches!(&action,LegalAction::CastSpell{casting_method:CastingMethod::AlternativePrice{origin,..},..} if matches!(origin.as_ref(),CastingMethod::Normal))
        );
        // A broad spell price must not turn the native designation into
        // permission to cast the Adventure half again.
        let foretold =
            g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
        g.add_counters(foretold, CounterType::Time, 9);
        assert!(!casts(&g,A,exiled).iter().any(|action|matches!(action,LegalAction::CastSpell{casting_method:CastingMethod::AlternativePrice{origin,..},..} if matches!(origin.as_ref(),CastingMethod::SplitOtherHalf))));
        g.set_adventure_exiled_for(exiled, B);
        assert!(priced(&g, A, exiled, hunter).is_none());
        let b_hunter = g.create_object_from_definition(&hunter_def, B, Zone::Battlefield);
        g.turn.active_player = B;
        g.turn.priority_player = Some(B);
        let action = priced(&g, B, exiled, b_hunter).unwrap();
        let stack = announce(&mut g, action, &mut SelectFirstDecisionMaker);
        assert_eq!(g.controller_of_id(stack), Some(B));
        assert_eq!(g.object(stack).unwrap().owner, A);
        resolve_stack_entry(&mut g).unwrap();
    }
}

#[test]
fn prepared_copy_native_origin_is_not_general_exile_access() {
    let mut g = game();
    let price =
        g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
    g.add_counters(price, CounterType::Time, 4);
    let source = spell(&mut g, A, Zone::Battlefield, "{1}");
    let copy = printed(
        &mut g,
        A,
        Zone::Exile,
        "Prepared cast",
        "Mana cost: {4}\nType: Sorcery\nYou gain 1 life.",
    );
    let ordinary = printed(
        &mut g,
        A,
        Zone::Exile,
        "Unmarked cast",
        "Mana cost: {4}\nType: Sorcery\nYou gain 1 life.",
    );
    let prepared_definition = compile_to_runtime_definition(
        "Prepared cast",
        "Mana cost: {4}\nType: Sorcery\nYou gain 1 life.",
        false,
    )
    .unwrap();
    g.register_linked_face_definition(&prepared_definition);
    g.object_mut(source).unwrap().linked_face_layout = LinkedFaceLayout::Prepare;
    g.object_mut(source).unwrap().other_face = Some(prepared_definition.card.id);
    assert!(g.set_prepared(source));
    let copy = *g
        .exile
        .iter()
        .find(|id| g.prepared_spell_source(**id) == Some(source))
        .unwrap();
    assert!(priced(&g, A, ordinary, price).is_none());
    assert!(casts(&g, B, copy).is_empty());
    let action = priced(&g, A, copy, price).unwrap();
    announce(&mut g, action, &mut SelectFirstDecisionMaker);
    assert!(!g.is_prepared(source));
    resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.player(A).unwrap().life, 21);
    assert!(priced(&g, A, ordinary, price).is_none());
}

#[test]
fn monolith_cannot_change_its_colorless_price_proposal_into_a_colored_prototype() {
    struct TryPrototype {
        offered: bool,
    }
    impl DecisionMaker for TryPrototype {
        fn decide_options(
            &mut self,
            g: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(choice) = ctx
                .options
                .iter()
                .find(|choice| choice.description.contains("Prototype"))
            {
                self.offered = true;
                vec![choice.index]
            } else {
                SelectFirstDecisionMaker.decide_options(g, ctx)
            }
        }
    }
    for def in definitions("Darksteel Monolith") {
        let mut g = game();
        let price = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let card = printed(
            &mut g,
            A,
            Zone::Hand,
            "Colored prototype",
            "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 7/7\nPrototype {1}{G} — 2/3",
        );
        let options = casts(&g, A, card);
        let action = priced(&g, A, card, price).unwrap();
        assert!(!options.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                casting_method: CastingMethod::AlternativePrice {
                    prototype: Some(_),
                    ..
                },
                ..
            }
        )));
        let mut forged = action.clone();
        if let LegalAction::CastSpell {
            casting_method: CastingMethod::AlternativePrice { prototype, .. },
            ..
        } = &mut forged
        {
            *prototype = Some(0);
        }
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        assert!(
            apply_priority_response_with_dm(
                &mut g,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(forged),
                &mut SelectFirstDecisionMaker
            )
            .is_err()
        );
        assert_eq!(g.object(card).unwrap().zone, Zone::Hand);
        assert!(g.turn_store.grant_cast_uses_this_turn.is_empty());
        let mut dm = TryPrototype { offered: false };
        let stack = announce(&mut g, action, &mut dm);
        assert!(!dm.offered);
        assert!(g.object(stack).unwrap().prototype_cast_state.is_none());
        assert_eq!(g.current_power(stack), Some(7));
        assert!(g.object(stack).unwrap().colors().is_empty());
        assert_eq!(
            g.object(stack)
                .unwrap()
                .mana_cost
                .as_ref()
                .unwrap()
                .mana_value(),
            7
        );
    }
}

#[test]
fn small_prototype_is_discovered_and_locked_before_price_and_origin_filtering() {
    for name in ["Primal Prayers", "As Foretold"] {
        for def in definitions(name) {
            for from_graveyard in [false, true] {
                let mut g = game();
                let price = g.create_object_from_definition(&def, A, Zone::Battlefield);
                if name == "As Foretold" {
                    g.add_counters(price, CounterType::Time, 2);
                } else {
                    g.player_mut(A).unwrap().energy_counters = 1;
                }
                let zone = if from_graveyard {
                    Zone::Graveyard
                } else {
                    Zone::Hand
                };
                if from_graveyard {
                    let mut filter = ironsmith::target::ObjectFilter::creature()
                        .owned_by(ironsmith::target::PlayerFilter::You);
                    filter.zone = None;
                    filter.power = Some(ironsmith::filter::Comparison::LessThanOrEqual(2));
                    let grant = GrantSpec::new(Grantable::PlayFrom, filter, Zone::Graveyard);
                    let origin = CardDefinitionBuilder::new(CardId::new(), "Small origin")
                        .card_types(vec![CardType::Enchantment])
                        .with_ability(ironsmith::ability::Ability::static_ability(
                            StaticAbility::grants(grant),
                        ))
                        .build();
                    g.create_object_from_definition(&origin, A, Zone::Battlefield);
                }
                let card = printed(
                    &mut g,
                    A,
                    zone,
                    "Small prototype",
                    "Mana cost: {7}\nType: Artifact Creature — Construct\nPower/Toughness: 7/7\nPrototype {1}{G} — 2/3",
                );
                let options = casts(&g, A, card);
                let action=options.into_iter().find(|action|matches!(action,LegalAction::CastSpell{casting_method:CastingMethod::AlternativePrice{price:choice,prototype:Some(_),..},..} if choice.source==price)).unwrap();
                let stack = announce(&mut g, action, &mut SelectFirstDecisionMaker);
                assert!(g.object(stack).unwrap().prototype_cast_state.is_some());
                assert_eq!(
                    g.object(stack)
                        .unwrap()
                        .cast_price
                        .as_ref()
                        .unwrap()
                        .prototype,
                    Some(0)
                );
                assert_eq!(g.current_power(stack), Some(2));
                assert_eq!(g.current_toughness(stack), Some(3));
                assert_eq!(
                    g.object(stack)
                        .unwrap()
                        .mana_cost
                        .as_ref()
                        .unwrap()
                        .mana_value(),
                    2
                );
                assert!(
                    g.object(stack)
                        .unwrap()
                        .colors()
                        .contains(ironsmith::color::Color::Green)
                );
                assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
                if name == "Primal Prayers" {
                    assert_eq!(g.player(A).unwrap().energy_counters, 0);
                }
                let stable = g.object(stack).unwrap().stable_id;
                resolve_stack_entry(&mut g).unwrap();
                let permanent = g.find_object_by_stable_id(stable).unwrap();
                assert_eq!(g.current_power(permanent), Some(2));
                assert_eq!(g.current_toughness(permanent), Some(3));
            }
        }
    }
}

#[test]
fn private_effect_price_also_selects_prototype_before_eligibility() {
    struct PrototypePrice;
    impl DecisionMaker for PrototypePrice {
        fn decide_options(
            &mut self,
            g: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| option.description.contains("prototyped"))
            {
                vec![option.index]
            } else {
                SelectFirstDecisionMaker.decide_options(g, ctx)
            }
        }
    }
    let mut g = game();
    let price =
        g.create_object_from_definition(&definitions("As Foretold")[0], A, Zone::Battlefield);
    g.add_counters(price, CounterType::Time, 2);
    let card = printed(
        &mut g,
        B,
        Zone::Exile,
        "Effect prototype",
        "Mana cost: {7}\nType: Artifact Creature\nPower/Toughness: 7/7\nPrototype {1}{G} — 2/3",
    );
    assert!(priced(&g, A, card, price).is_none());
    let source = printed(&mut g, A, Zone::Stack, "Effect origin", "Type: Instant");
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(g.object(card).unwrap(), &g);
    let effect = ironsmith::effects::CastTaggedEffect::new(
        "prototype",
        ironsmith::target::PlayerFilter::You,
    );
    let mut dm = PrototypePrice;
    {
        let mut ctx = ironsmith::effects::EffectContext::new(source, A, &mut dm);
        ctx.tag_object("prototype", snapshot);
        ironsmith::effects::execute_effect(
            &mut g,
            &ironsmith::effect::Effect::new(effect),
            &mut ctx,
        )
        .unwrap();
    }
    let stack = g.stack.last().unwrap().object_id;
    assert!(g.object(stack).unwrap().prototype_cast_state.is_some());
    assert_eq!(g.current_power(stack), Some(2));
    assert_eq!(g.controller_of_id(stack), Some(A));
    assert_eq!(g.object(stack).unwrap().owner, B);
}
