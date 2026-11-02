//! Control-duration permissions own their mana conversion and never resume.
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{compute_legal_actions, LegalAction};
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
const A: PlayerId = PlayerId(0); const B: PlayerId = PlayerId(1);
fn pairs(name: &str, text: &str) -> [CardDefinition;2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true,true,||
        ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
    assert!(!loss.is_lossy(),"{name}: {}",loss.reasons_text());
    let (artifact,direct)=result.unwrap();
    let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn offered(game:&GameState,id:ObjectId)->bool { compute_legal_actions(game,A).unwrap().into_iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if spell_id==id)) }
#[test]
fn source_creed_full_program_round_trips_with_price_haste_attack_and_duration() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-creed-control-duration.json")).unwrap();
    let card=&fixture["mechanics"][0]["representative_cards"][0];
    assert_eq!(card["source_card_key"],"Creed's Desperado_OTH");
    assert!(card["original_text"].as_str().unwrap().contains("You may cast it as long as you control a Warrior"));
    for definition in pairs("Creed's Desperado",card["executable_text"].as_str().unwrap()) {
        assert_eq!(definition.alternative_casts.len(),1);
        assert!(definition.abilities.iter().any(|ability|matches!(ability.kind,ironsmith::ability::AbilityKind::Triggered(_))));
        assert!(definition.abilities.len()>=2);
    }
}
#[test]
fn independent_cast_permission_cannot_inherit_an_expired_or_never_started_mana_rider() {
    let text="Mana cost: {0}\nType: Sorcery\nExile the top card of target opponent's library. You may cast it as long as you control a Warrior, and you may spend mana as though it were mana of any color to cast it.";
    for (route,definition) in pairs("Duration authority",text).into_iter().enumerate() { for initially_active in [false,true] {
        if std::env::var_os("IRONSMITH_CREED_DIAGNOSTICS").is_some() {eprintln!("CREED route={route} initially_active={initially_active} program={:?}",definition.spell_effect);}
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
        let warrior=pairs("Warrior","Type: Creature — Warrior\nPower/Toughness: 2/2")[0].clone();
        game.create_object_from_definition(&warrior,B,Zone::Battlefield);
        let own=initially_active.then(||game.create_object_from_definition(&warrior,A,Zone::Battlefield));
        let blue=pairs("Blue spell","Mana cost: {U}\nType: Instant\nYou gain 1 life.")[0].clone();
        let card=game.create_object_from_definition(&blue,B,Zone::Library);let stable=game.object(card).unwrap().stable_id;
        let source=game.create_object_from_definition(&definition,A,Zone::Stack);
        let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
        let exiled=game.find_object_by_stable_id(stable).unwrap();
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red,1);
        if std::env::var_os("IRONSMITH_CREED_DIAGNOSTICS").is_some() {
            eprintln!("CREED source={source:?} context_controller={:?} exiled={exiled:?} registry={:?} mana={:?}",ctx.controller,game.effect_store.grant_registry,game.effect_store.mana_spend_effects);
            for id in &game.battlefield {let object=game.object(*id).unwrap();eprintln!("CREED permanent={id:?} owner={:?} current_controller={:?} subtypes={:?}",object.owner,game.controller_of(object),object.subtypes);}
        }
        assert_eq!(offered(&game,exiled),initially_active);
        assert!(game.effect_store.mana_spend_effects.permissions.is_empty(),"duration-bound conversion must not create a free-standing spending rule");
        if let Some(own)=own {game.move_object_by_effect(own,Zone::Graveyard).unwrap();game.refresh_continuous_state().unwrap();}
        game.create_object_from_definition(&warrior,A,Zone::Battlefield);game.refresh_continuous_state().unwrap();
        assert!(!offered(&game,exiled),"a failed duration never starts again");
        game.effect_store.grant_registry.grant_play_from_to_card(exiled,Zone::Exile,A,Default::default(),ironsmith::grant_registry::GrantSource::Effect{source_id:source,expires_end_of_turn:u32::MAX});
        assert!(!offered(&game,exiled),"another permission must still pay the spell's blue cost");
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);
        assert!(offered(&game,exiled),"normal independently granted authority remains usable with proper mana");
    }}
}
