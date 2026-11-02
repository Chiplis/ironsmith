//! Generic activation history: actual payment, exact occurrence and lifetime.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const RULE: &str =
    "Spend only mana of a color that hasn't been spent this way to activate this ability.";
fn setup() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    main(&mut g);
    g
}
fn main(g: &mut GameState) {
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = ironsmith::game_state::Phase::FirstMain;
    g.turn.step = None;
}
fn legal(g: &GameState, source: ObjectId, index: usize) -> Option<LegalAction> {
    compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,
        LegalAction::ActivateAbility{source:id,ability_index}
        | LegalAction::ActivateManaAbility{source:id,ability_index}
            if *id==source&&*ability_index==index))
}
fn activate(g: &mut GameState, source: ObjectId, index: usize) {
    activate_with_dm(g, source, index, &mut SelectFirstDecisionMaker);
}
fn activate_with_dm(
    g: &mut GameState,
    source: ObjectId,
    index: usize,
    dm: &mut impl ironsmith::decision::DecisionMaker,
) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let action = legal(g, source, index).expect("requested occurrence payable");
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(g, &mut q, &mut state, &ctx, dm).unwrap();
    }
    assert!(!state.has_pending_action());
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(g, &mut q, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(g, &mut q, dm).unwrap();
    while !g.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(g, dm).unwrap();
    }
    g.turn.priority_player = Some(A);
}
fn mana(g: &mut GameState, s: ManaSymbol) {
    g.player_mut(A).unwrap().mana_pool.add(s, 1);
}
#[test]
fn physical_colors_persist_across_turns_but_not_occurrences_or_incarnations() {
    let text =
        format!("Type: Artifact\n{{1}}: You gain 1 life. {RULE}\n{{1}}: You gain 2 life. {RULE}");
    for definition in source_support::definitions(
        &serde_json::json!({"name":"Color history witness","executable_text":text}),
    )
    .unwrap()
    {
        let mut g = setup();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        mana(&mut g, ManaSymbol::Red);
        assert!(legal(&g, source, 0).is_some());
        activate(&mut g, source, 0);
        mana(&mut g, ManaSymbol::Red);
        assert!(legal(&g, source, 0).is_none());
        assert!(legal(&g, source, 1).is_some());
        activate(&mut g, source, 1);
        mana(&mut g, ManaSymbol::Blue);
        activate(&mut g, source, 0);
        g.next_turn();
        main(&mut g);
        mana(&mut g, ManaSymbol::Red);
        assert!(legal(&g, source, 0).is_none());
        assert!(legal(&g, source, 1).is_none());
        let fresh = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(legal(&g, fresh, 0).is_some());
    }
}
#[test]
fn immediate_mana_activation_records_payment_before_reuse_and_colorless_is_not_a_color() {
    let text = format!("Type: Artifact\n{{1}}: Add {{C}}{{C}}. {RULE}");
    for definition in source_support::definitions(
        &serde_json::json!({"name":"Mana history witness","executable_text":text}),
    )
    .unwrap()
    {
        let mut g = setup();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        mana(&mut g, ManaSymbol::Colorless);
        assert!(legal(&g, source, 0).is_none());
        mana(&mut g, ManaSymbol::Blue);
        activate(&mut g, source, 0);
        assert!(g.stack.is_empty());
        assert!(legal(&g, source, 0).is_none());
        mana(&mut g, ManaSymbol::Blue);
        assert!(legal(&g, source, 0).is_none());
        mana(&mut g, ManaSymbol::Red);
        assert!(legal(&g, source, 0).is_some());
        activate(&mut g, source, 0);
    }
}
#[test]
fn zero_actual_payment_does_not_invent_a_spent_color() {
    let text = format!("Type: Artifact\n{{0}}: You gain 1 life. {RULE}");
    for definition in source_support::definitions(
        &serde_json::json!({"name":"Zero payment witness","executable_text":text}),
    )
    .unwrap()
    {
        let mut g = setup();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        activate(&mut g, source, 0);
        activate(&mut g, source, 0);
        assert!(legal(&g, source, 0).is_some());
    }
}

#[test]
fn unpaid_proposal_does_not_commit_color_history() {
    let text = format!("Type: Artifact\n{{1}}: You gain 1 life. {RULE}");
    for definition in source_support::definitions(
        &serde_json::json!({"name":"Unpaid history witness","executable_text":text}),
    )
    .unwrap()
    {
        let mut g = setup();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(legal(&g, source, 0).is_none());
        let mut q = ironsmith::triggers::TriggerQueue::new();
        let mut state = ironsmith::game_loop::PriorityLoopState::new(2);
        let attempted = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut g,
            &mut q,
            &mut state,
            &ironsmith::game_loop::PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
                source,
                ability_index: 0,
            }),
            &mut SelectFirstDecisionMaker,
        );
        assert!(
            attempted.is_err(),
            "a proposal with no payable mana must fail"
        );
        assert!(g.turn_store.activation_mana_colors.is_empty());
        main(&mut g);
        mana(&mut g, ManaSymbol::Blue);
        assert!(legal(&g, source, 0).is_some());
    }
}

struct CosmicChoices {
    fragment_prompts: usize,
}
impl ironsmith::decision::DecisionMaker for CosmicChoices {
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description == "Choose a bracketed text fragment to remove" {
            self.fragment_prompts += 1;
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.legal)
                    .unwrap()
                    .index,
            ];
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min)
            .map(|option| option.index)
            .collect()
    }
}
fn retained_identities(
    g: &GameState,
    source: ObjectId,
) -> Vec<(
    usize,
    Option<ironsmith::continuous::AbilityOrigin>,
    Option<ironsmith_core::LinkedExileDefinition>,
)> {
    let chars = g.calculated_characteristics(source).unwrap();
    chars
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(index, ability)| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some((
                index,
                chars.abilities.origin(index).cloned(),
                ability.effects.activation_definition,
            )),
            _ => None,
        })
        .collect()
}
fn removed_fragments(g: &GameState, source: ObjectId) -> u64 {
    g.calculated_characteristics(source)
        .unwrap()
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => {
                ability
                    .canonical_model()
                    .and_then(|model| match model.payload {
                        ironsmith_core::StaticAbilityPayload::OptionalTextFragments(spec) => {
                            Some(spec.current_removed)
                        }
                        _ => None,
                    })
            }
            _ => None,
        })
        .expect("actual Cosmic source has a compiled fragment plan")
}
#[test]
fn source_derived_cosmic_control_preserves_history_and_records_unresolved_full_source() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-glitch-fragments.json"
    ))
    .unwrap();
    let card = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["source_card_key"] == "Glitch, Cosmic Dreamer_OTH")
        .unwrap();
    assert!(
        card["original_text"]
            .as_str()
            .unwrap()
            .contains("Spend only mana of a color that hasn't been spent this way")
    );
    assert_eq!(card["unresolved_source_boundary"]["status"],"explicit_user_boundary");
    assert_eq!(card["unresolved_source_boundary"]["user_instruction"],"Leave ambiguous source unresolved");
    assert!(card["executable_text"].is_null(),"the complete ambiguous source has no certified executable body");
    let evidence=&card["unresolved_executable_evidence"];
    let complete=evidence["executable_text"].as_str().expect("preserved prior unverified candidate");
    let omitted=card["unresolved_source_boundary"]["ability_text"].as_str().unwrap();
    assert!(card["original_text"].as_str().unwrap().contains(omitted));
    // This independently named primitive control exercises the unambiguous
    // history/deletion/loyalty operations. It is explicitly NOT the complete
    // Cosmic source or a replacement for the unresolved creation instruction.
    let control_text=complete.lines().filter(|line|!line.starts_with("[-6]:")).collect::<Vec<_>>().join("\n");
    assert!(!control_text.contains("-1/-1 counters on them"));
    let control=serde_json::json!({"name":"Source-derived color history control","executable_text":control_text,
        "custom_subtypes":card["custom_subtypes"],"custom_keyword_identities":card["custom_keyword_identities"],
        "custom_keyword_payments":card["custom_keyword_payments"],"custom_keyword_counter_definitions":card["custom_keyword_counter_definitions"]});
    for definition in source_support::definitions(&control).unwrap() {
        let mut g = setup();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = retained_identities(&g, source);
        assert_eq!(original.len(), 3);
        let manager = original[0].0;
        let loyalty = original[1].0;
        assert!(
            original
                .iter()
                .all(|(_, origin, definition)| origin.is_some() && definition.is_some())
        );
        assert_eq!(removed_fragments(&g, source), 0);
        assert!(
            legal(&g, source, loyalty).is_some(),
            "the separate loyalty ability is legal before management"
        );
        mana(&mut g, ManaSymbol::Red);
        let mut dm = CosmicChoices {
            fragment_prompts: 0,
        };
        activate_with_dm(&mut g, source, manager, &mut dm);
        assert_eq!(
            dm.fragment_prompts, 1,
            "the actual paid manager must resolve its fragment choice"
        );
        assert_eq!(removed_fragments(&g, source), 1);
        assert_eq!(
            retained_identities(&g, source),
            original,
            "deletion preserves every retained acquisition and definition"
        );
        mana(&mut g, ManaSymbol::Red);
        assert!(
            legal(&g, source, manager).is_none(),
            "deletion cannot reopen a spent physical color"
        );
        assert!(
            legal(&g, source, loyalty).is_some(),
            "management does not activate or restrict the distinct loyalty occurrence"
        );
        mana(&mut g, ManaSymbol::Blue);
        activate_with_dm(&mut g, source, manager, &mut dm);
        assert_eq!(dm.fragment_prompts, 2);
        assert_eq!(removed_fragments(&g, source).count_ones(), 2);
        assert_eq!(retained_identities(&g, source), original);
        let exiled = g.move_object_by_effect(source, Zone::Exile).unwrap();
        let returned = g.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
        assert_ne!(source, returned);
        main(&mut g);
        assert_eq!(
            removed_fragments(&g, returned),
            0,
            "persistent deletions belong to the old incarnation"
        );
        assert!(
            legal(&g, returned, manager).is_some(),
            "the previously rejected red mana can pay a fresh incarnation"
        );
        activate_with_dm(&mut g, returned, manager, &mut dm);
        assert_eq!(dm.fragment_prompts, 3);
        assert_eq!(removed_fragments(&g, returned).count_ones(), 1);
    }
}

#[test]
fn optional_joint_player_subject_gives_each_participant_their_own_choice_and_keeps_followup_controller(){
    struct Decisions { asked:Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Decisions {
        fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool{
            self.asked.push(ctx.player);ctx.player==PlayerId(1)
        }
    }
    for definition in source_support::definitions(&serde_json::json!({"name":"Independent joint offer","executable_text":"Type: Sorcery\nYou and each opponent may gain 1 life. Draw a card."})).unwrap(){
        let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        let card=ironsmith_compiler_runtime::compile_to_runtime_definition("Draw witness","Type: Basic Land — Island",false).unwrap();
        let top=game.create_object_from_definition(&card,A,Zone::Library);let stable=game.object(top).unwrap().stable_id;
        let mut decisions=Decisions{asked:Vec::new()};
        let mut context=ironsmith::effects::EffectContext::new_default(source,A).with_decision_maker(&mut decisions);
        for effect in definition.spell_effect.as_ref().unwrap().iter(){ironsmith::effects::execute_effect(&mut game,effect,&mut context).unwrap();}
        drop(context);
        let mut asked=decisions.asked;asked.sort();assert_eq!(asked,vec![A,PlayerId(1),PlayerId(2)]);
        assert_eq!(game.player(A).unwrap().life,20);assert_eq!(game.player(PlayerId(1)).unwrap().life,21);assert_eq!(game.player(PlayerId(2)).unwrap().life,20);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Hand,"mandatory following sentence belongs only to the controller");
    }
}
