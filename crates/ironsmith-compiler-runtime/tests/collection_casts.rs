//! Source-authored and deliberately unrun (cf8 p04): one-shot casts drawn
//! from a named collection while an ability resolves (CR 608.2g, CR 601.2) —
//! "from among cards exiled this way", "from among the exiled cards", "from
//! among cards [you own] exiled with <this source>" (CR 607.2a), "cards
//! exiled with <this source>", and "from your graveyard".
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::resolution::ResolutionProgram;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const COHORT: &[(&str, &str)] = &[
    ("Hellcarver Demon", "90514aa5-84b6-4be7-b6d0-b05529c97140"),
    ("Izzet Chemister", "e4a3d6f3-36ba-4e6d-a254-351bc8886f20"),
    ("Kylox, Visionary Inventor", "37f6bbb8-2136-4f03-966c-85d72180e71f"),
    ("Doom Reigns Supreme", "f398a742-22c1-4da9-9d49-fdce2eb93d01"),
    ("Kaho, Minamo Historian", "395a7bd6-7c2d-448e-842c-ca53256d7008"),
    ("Krang & Shredder", "2385c8fb-9c38-4ff3-8f61-4e25a8c7d46b"),
    ("Shell of the Last Kappa", "a8b24b38-019c-45e7-a5f4-a1bc3014f7a2"),
    ("Boiling Rock Rioter", "5193172c-9024-409e-bd9e-0387971d65fe"),
    ("Jeleva, Nephalia's Scourge", "a014f283-c531-415c-ac00-e6773ea5d64d"),
    ("Summon: Esper Valigarmanda", "cecd2f49-8ccb-472d-9cf1-fd0f4d93546d"),
    ("Chandra Ablaze", "3fae28e6-2ffc-460e-9dd3-e321e8a53ba9"),
    ("Forger's Foundry", "118da256-d1ea-44e8-9026-317e49694d29"),
];

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/collection_casts.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing fixture row {name}"));
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn frozen_bodies_compile_strictly_with_a_resolution_cast_choice() {
    let rows = rows();
    assert_eq!(rows.len(), COHORT.len());
    for (name, oracle_id) in COHORT {
        assert!(
            rows.iter().any(|row| row["name"] == *name && row["oracle_id"] == *oracle_id),
            "{name}"
        );
        for definition in definitions(name) {
            fn inspect(effect:&ironsmith::Effect, found:&mut (usize,usize)) {
                found.0+=usize::from(effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().is_some());
                found.1+=usize::from(effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>().is_some());
                effect.visit_child_effects(&mut |child|inspect(child,found));
            }
            let mut found=(0,0);
            if let Some(program)=&definition.spell_effect {for effect in program.all_effects(){inspect(effect,&mut found);}}
            for ability in &definition.abilities {
                let program=match &ability.kind {AbilityKind::Triggered(ability)=>Some(&ability.effects),AbilityKind::Activated(ability)=>Some(&ability.effects),_=>None};
                if let Some(program)=program {for effect in program.all_effects(){inspect(effect,&mut found);}}
            }
            assert!(found.0>0,"{name}: an actual resolving collection choice is retained");
            assert!(found.1>0,"{name}: the selected cards have actual cast instructions");
        }
    }
}

#[test]
fn free_and_paid_casts_keep_their_authored_price_and_pool() {
    for (name, free, linked, owned) in [
        ("Hellcarver Demon", true, false, false),
        ("Kylox, Visionary Inventor", true, false, false),
        ("Doom Reigns Supreme", true, false, false),
        ("Izzet Chemister", true, true, false),
        ("Kaho, Minamo Historian", true, true, false),
        ("Krang & Shredder", true, true, false),
        ("Shell of the Last Kappa", true, true, false),
        ("Jeleva, Nephalia's Scourge", true, true, false),
        ("Forger's Foundry", true, true, false),
        ("Boiling Rock Rioter", false, true, true),
        ("Summon: Esper Valigarmanda", false, true, false),
        ("Chandra Ablaze", true, false, true),
    ] {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert_eq!(
                debug.contains("without_paying_mana_cost: true"),
                free,
                "{name}: {debug}"
            );
            assert_eq!(debug.contains("__source_exiled__"), linked, "{name}: {debug}");
            if owned {
                assert!(debug.contains("owner: Some(You)"), "{name}: {debug}");
            }
        }
    }
    for definition in definitions("Summon: Esper Valigarmanda") {
        assert!(format!("{definition:?}").contains("AnyType"), "mana of any type");
    }
    for definition in definitions("Kaho, Minamo Historian") {
        assert!(format!("{definition:?}").contains("EqualExpr"), "mana value exactly X");
    }
}

/// Accepts every optional instruction and every legal candidate.
struct TakeEverything;

impl DecisionMaker for TakeEverything {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id);
        match ctx.max {
            Some(max) => legal.take(max).collect(),
            None => legal.collect(),
        }
    }
}

fn activated_program(definition: &CardDefinition, index: usize) -> ResolutionProgram {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability.effects.clone()),
            _ => None,
        })
        .nth(index)
        .expect("activated ability")
}

fn resolve_ability(
    game: &mut GameState,
    source: ObjectId,
    program: ResolutionProgram,
    targets: Vec<Target>,
    dm: &mut impl DecisionMaker,
) {
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        &program,
        A,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), targets.len());
    let assignments = requirements
        .iter()
        .enumerate()
        .map(|(index, requirement)| TargetAssignment {
            spec: requirement.spec.clone(),
            range: index..index + 1,
        })
        .collect();
    game.push_to_stack(
        StackEntry::ability(source, A, program)
            .with_targets(targets)
            .with_target_assignments(assignments),
    );
    resolve_stack_entry_with(game, dm).unwrap();
}

#[test]
fn izzet_chemister_casts_only_its_linked_exiles_for_free() {
    for definition in definitions("Izzet Chemister") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let chemister = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let insight = compile_to_runtime_definition(
            "Linked Insight",
            "Mana cost: {4}{U}\nType: Sorcery\nDraw a card.",
            false,
        )
        .unwrap();
        let in_graveyard = game.create_object_from_definition(&insight, A, Zone::Graveyard);
        // An unrelated exiled card is not part of the linked pool.
        let unrelated = game.create_object_from_definition(&insight, B, Zone::Exile);
        for n in 0..2 {
            game.create_object_from_definition(
                &compile_to_runtime_definition(&format!("Library {n}"), "Type: Land", false)
                    .unwrap(),
                A,
                Zone::Library,
            );
        }
        resolve_ability(
            &mut game,
            chemister,
            activated_program(&definition, 0),
            vec![Target::Object(in_graveyard)],
            &mut SelectFirstDecisionMaker,
        );
        let linked = game.get_exiled_with_source_links(chemister).to_vec();
        assert_eq!(linked.len(), 1);
        resolve_ability(
            &mut game,
            chemister,
            activated_program(&definition, 1),
            vec![],
            &mut TakeEverything,
        );
        assert_eq!(game.stack.len(), 1, "the linked card was cast");
        let cast = game.object(game.stack[0].object_id).unwrap();
        assert_eq!(cast.name, "Linked Insight");
        assert_eq!(cast.owner, A);
        assert_eq!(
            game.object(unrelated).map(|object| object.zone),
            Some(Zone::Exile),
            "an unlinked exiled card stays put"
        );
        let mana_before = game.player(A).unwrap().mana_pool.clone();
        let hand_before = game.player(A).unwrap().hand.len();
        resolve_stack_entry_with(&mut game, &mut TakeEverything).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 1);
        assert_eq!(game.player(A).unwrap().mana_pool, mana_before);
    }
}

/// The original complete body must offer zero or several cards, rather than
/// capture the collection and silently cast only its first member.
#[test]
fn hellcarver_chooses_none_or_several_original_exiles_and_casts_each_selected_spell() {
    for definition in definitions("Hellcarver Demon") {
        for take_two in [false,true] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            game.turn.active_player=A;game.turn.priority_player=Some(A);
            game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
            let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
            let body=definition.abilities.iter().find_map(|ability|match &ability.kind {
                AbilityKind::Triggered(trigger)=>Some(trigger.effects.clone()),_=>None,
            }).unwrap();
            let filler=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Sacrifice witness")
                .card_types(vec![ironsmith::CardType::Creature]).power_toughness(ironsmith::PowerToughness::fixed(1,2)).build();
            let permanent=game.create_object_from_definition(&filler,A,Zone::Battlefield);let permanent=game.object(permanent).unwrap().stable_id;
            let hand=game.create_object_from_definition(&filler,A,Zone::Hand);let hand=game.object(hand).unwrap().stable_id;
            let mut original=Vec::new();let mut spell_names=std::collections::HashSet::new();
            for i in 0..6 {
                let name=format!("Original exile member {i}");
                let card=if i==5 {ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),&name).card_types(vec![ironsmith::CardType::Land]).build()}
                    else {spell_names.insert(name.clone());ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),&name)
                        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(5)]))
                        .card_types(vec![ironsmith::CardType::Sorcery]).with_spell_effect(vec![ironsmith::Effect::gain_life(i+2)]).build()};
                let id=game.create_object_from_definition(&card,A,Zone::Library);original.push(game.object(id).unwrap().stable_id);
            }
            let unrelated=game.create_object_from_definition(&filler,A,Zone::Exile);
            let mut dm=SelectOriginalExiles {take_two,offered:Default::default()};
            resolve_ability(&mut game,source,body,vec![],&mut dm);
            assert_eq!(dm.offered,spell_names,"only the five original nonland exiles are offered, not an unrelated exile");
            assert_eq!(game.stack.len(),if take_two{2}else{0},"the original optional any-number choice controls the number of actual casts");
            let cast_names:std::collections::HashSet<_>=game.stack.iter().map(|entry|game.object(entry.object_id).unwrap().name.to_string()).collect();
            let expected=if take_two{["Original exile member 0".to_string(),"Original exile member 1".to_string()].into_iter().collect()}else{Default::default()};
            assert_eq!(cast_names,expected);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
            assert_eq!(game.object(unrelated).unwrap().zone,Zone::Exile);
            for (index,stable) in original.into_iter().enumerate(){let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,if take_two&&index<2{Zone::Stack}else{Zone::Exile});}
            for stable in [permanent,hand]{let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Graveyard,"the preceding sacrifice/discard instructions execute once");}
            while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
            assert_eq!(game.player(A).unwrap().life,if take_two{25}else{20},"both selected original spells actually resolve, without paying their printed costs");
        }
    }
}
struct SelectOriginalExiles {take_two:bool,offered:std::collections::HashSet<String>}
impl DecisionMaker for SelectOriginalExiles {
    fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{true}
    fn decide_objects(&mut self,game:&GameState,ctx:&SelectObjectsContext)->Vec<ObjectId>{
        if !ctx.candidates.is_empty()&&ctx.candidates.iter().all(|candidate|game.object(candidate.id).is_some_and(|object|object.zone==Zone::Exile)) {
            self.offered.extend(ctx.candidates.iter().map(|candidate|game.object(candidate.id).unwrap().name.to_string()));
            return ctx.candidates.iter().filter(|candidate|self.take_two&&matches!(game.object(candidate.id).unwrap().name.as_str(),"Original exile member 0"|"Original exile member 1")).map(|candidate|candidate.id).collect();
        }
        SelectFirstDecisionMaker.decide_objects(game,ctx)
    }
}
