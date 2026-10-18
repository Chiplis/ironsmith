//! Claude package 72 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::types::CardType;
use ironsmith::{PlayerId, Zone};

fn p72_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

/// Balduvian Warlord: the targeted blocker leaves combat, the attackers only
/// it blocked become unblocked (CR 506.4), then it must block an attacking
/// creature its controller's opponent... chosen by the Warlord's controller.
#[test]
fn balduvian_warlord_unblocks_and_reblocks_a_chosen_attacker() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState, is_blocked};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    struct Pick {
        attacker: ironsmith::ObjectId,
        min_seen: Option<usize>,
    }
    impl ironsmith::decision::DecisionMaker for Pick {
        fn decide_objects(
            &mut self,
            _: &ironsmith::GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            self.min_seen = Some(ctx.min);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.id == self.attacker)
                .map(|candidate| candidate.id)
                .collect()
        }
    }
    let definition = p72_compile(
        "Balduvian Warlord",
        "Mana cost: {3}{R}\nType: Creature — Human Barbarian\nPower/Toughness: 3/2\n{T}: Remove target blocking creature from combat. Creatures it was blocking that hadn't become blocked by another creature this combat become unblocked, then it blocks an attacking creature of your choice. Activate only during the declare blockers step.",
    );
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Warlord's activated ability");
    let debug = format!("{:?}", activated.effects);
    assert!(
        debug.contains("UnblockSoleBlockedAttackersEffect")
            && debug.contains("BlockAttackerOfChoiceEffect")
            && debug.contains("optional: false"),
        "{debug}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = |name: &str| {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    let first = game.create_object_from_card(&creature("Blocked attacker"), alice, Zone::Battlefield);
    let second = game.create_object_from_card(&creature("Other attacker"), alice, Zone::Battlefield);
    let blocker = game.create_object_from_card(&creature("Bob's blocker"), bob, Zone::Battlefield);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo { creature: first, target: AttackTarget::Player(bob) });
    combat.attackers.push(AttackerInfo { creature: second, target: AttackTarget::Player(bob) });
    combat.blockers.insert(first, vec![blocker]);
    combat.blocked_attackers.insert(first);
    combat.block_declaration_complete = true;
    game.combat = Some(combat);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::CreatureBlockedEvent::new(blocker, first),
        ironsmith::ProvNodeId::default(),
    );
    game.turn_store.turn_history.record_event(&event, None, None);
    let warlord = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut dm = Pick { attacker: second, min_seen: None };
    let mut ctx = EffectContext::new(warlord, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Object(blocker)]);
    for effect in activated.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(dm.min_seen, Some(1), "the new block is mandatory");
    let combat = game.combat.as_ref().unwrap();
    assert!(!is_blocked(combat, first), "its only blocker left combat");
    assert!(is_blocked(combat, second), "the chosen attacker is now blocked");
    assert!(combat.blockers.get(&second).is_some_and(|blockers| blockers.contains(&blocker)));
}

/// Avatar Aang: one trigger per bending action, and the transform checks
/// that each of the four actions was performed this turn.
#[test]
fn avatar_aang_transforms_only_after_all_four_bends() {
    let definition = p72_compile(
        "Avatar Aang",
        "Mana cost: {R}{G}{W}{U}\nType: Legendary Creature — Human Avatar Ally\nPower/Toughness: 4/4\nFlying, firebending 2\nWhenever you waterbend, earthbend, firebend, or airbend, draw a card. Then if you've done all four this turn, transform Avatar Aang.",
    );
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger),
            _ => None,
        })
        .expect("bending trigger");
    let trigger_debug = format!("{:?}", trigger.trigger);
    for action in ["Waterbend", "Earthbend", "Firebend", "Airbend"] {
        assert!(trigger_debug.contains(action), "{action} triggers: {trigger_debug}");
    }
    let effects = format!("{:?}", trigger.effects);
    assert!(effects.contains("Draw"), "{effects}");
    assert!(effects.contains("Transform"), "{effects}");
    assert_eq!(
        effects.matches("KeywordActionsPerformed").count(),
        4,
        "one performed-this-turn check per bending action: {effects}"
    );
}
