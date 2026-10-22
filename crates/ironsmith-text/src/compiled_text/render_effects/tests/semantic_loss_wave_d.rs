//! Semantic-loss fix wave (2026-10-21), package D: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

/// Picks every legal candidate up to the decision's maximum and accepts
/// every optional instruction.
struct TakeAll;
impl crate::decision::DecisionMaker for TakeAll {
    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(ctx.max.unwrap_or(usize::MAX))
            .map(|candidate| candidate.id)
            .collect()
    }
}

fn compile(name: &str, card_types: Vec<CardType>, text: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(text)
        .unwrap_or_else(|err| panic!("{name} compiles: {err:?}"))
}

fn spell_program(definition: &crate::CardDefinition) -> String {
    format!("{:?}", definition.spell_effect.as_ref().expect("spell effect"))
}

fn run_spell(
    game: &mut crate::game_state::GameState,
    definition: &crate::CardDefinition,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) {
    let mut decisions = TakeAll;
    let mut ctx = crate::effects::EffectContext::new(source, controller, &mut decisions);
    for segment in &definition.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
    }
}

fn creature_card(name: &str) -> crate::card::Card {
    crate::card::CardBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .build()
}

// ---------------------------------------------------------------------------
// Finding: a non-target sacrifice of a whole earlier set sacrificed only its
// first member (Force of Rage, Push the Limit, Depthshaker Titan).
// ---------------------------------------------------------------------------

#[test]
fn sacrifice_of_a_tagged_set_sacrifices_every_member() {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let card = creature_card("Returned Vehicle");
    let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let source = game.create_object_from_card(&card, alice, Zone::Graveyard);
    let snapshots = [first, second]
        .iter()
        .map(|id| crate::snapshot::ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))
        .collect::<Vec<_>>();
    let mut decisions = TakeAll;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
    ctx.set_tagged_objects("returned_0", snapshots);
    let sacrifice = crate::effect::Effect::new(
        crate::effects::SacrificeTargetEffect::new(ChooseSpec::Object(
            crate::target::ObjectFilter::exact_tagged("returned_0"),
        ))
        .with_player(PlayerFilter::You),
    );
    crate::effects::execute_effect(&mut game, &sacrifice, &mut ctx).unwrap();
    assert!(
        game.battlefield.is_empty(),
        "both members of the referenced set are sacrificed, not only the first"
    );
}

#[test]
fn sacrifice_them_keeps_the_plural_reference_and_text() {
    let definition = compile(
        "Push the Limit",
        vec![CardType::Sorcery],
        "Return all Mount and Vehicle cards from your graveyard to the battlefield. Sacrifice them at the beginning of the next end step.",
    );
    let program = spell_program(&definition);
    assert!(program.contains("SacrificeTargetEffect"), "{program}");
    assert!(
        program.contains("plural_pronoun_reference: true"),
        "the delayed sacrifice names the whole returned set: {program}"
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        ["Return all Mount and Vehicle cards from your graveyard to the battlefield. Sacrifice them at the beginning of the next end step."]
    );
}

// ---------------------------------------------------------------------------
// Finding: sacrificing fewer than the required number counted as having
// happened, so "If you can't" never ran (Infernal Denizen).
// ---------------------------------------------------------------------------

fn swamp_card() -> crate::card::Card {
    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Swamp")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Swamp])
        .build()
}

#[test]
fn short_sacrifice_runs_the_if_you_cant_branch() {
    let definition = compile(
        "Swamp Tithe",
        vec![CardType::Instant],
        "Sacrifice two Swamps. If you can't, you gain 5 life.",
    );
    for swamps in [0usize, 1, 2, 3] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        for _ in 0..swamps {
            game.create_object_from_card(&swamp_card(), alice, Zone::Battlefield);
        }
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        run_spell(&mut game, &definition, source, alice);
        let expected_life = if swamps >= 2 { 20 } else { 25 };
        assert_eq!(game.players[0].life, expected_life, "swamps={swamps}");
        assert_eq!(
            game.battlefield.len(),
            swamps.saturating_sub(2),
            "every sacrificeable Swamp is still sacrificed (swamps={swamps})"
        );
    }
}

// ---------------------------------------------------------------------------
// Finding: "At this turn's next end of combat" was a forever delayed
// trigger (Gaze of the Gorgon, Venomous Breath, Glyph of Doom).
// ---------------------------------------------------------------------------

#[test]
fn this_turns_next_end_of_combat_expires_at_end_of_turn() {
    let definition = compile(
        "Gaze of the Gorgon",
        vec![CardType::Instant],
        "Regenerate target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn.",
    );
    let program = spell_program(&definition);
    assert!(program.contains("EndOfCombatTrigger"), "{program}");
    assert!(program.contains("until_end_of_turn: true"), "{program}");
    assert!(
        !program.contains("duration: Forever"),
        "the end-of-combat registration must not outlive the turn: {program}"
    );
    assert!(
        program.contains("regenerated_0"),
        "the delayed body still names the regenerated creature: {program}"
    );
}

#[test]
fn plain_end_of_combat_delay_is_unchanged() {
    let definition = compile(
        "Plain Delay",
        vec![CardType::Instant],
        "Regenerate target creature. At end of combat, destroy all creatures that blocked or were blocked by it this turn.",
    );
    let program = spell_program(&definition);
    assert!(program.contains("EndOfCombatTrigger"), "{program}");
    assert!(!program.contains("until_end_of_turn: true"), "{program}");
}

#[test]
fn this_turns_next_end_of_combat_renders_its_window() {
    let definition = compile(
        "Gaze of the Gorgon",
        vec![CardType::Instant],
        "Regenerate target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn.",
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        ["Regenerate target creature. At this turn's next end of combat, destroy all creatures that blocked or were blocked by it this turn."]
    );
}

// ---------------------------------------------------------------------------
// Finding: "You have no maximum hand size for as long as you control this
// Saga" granted an ability to the Saga (Jin-Gitaxias // The Great Synthesis).
// ---------------------------------------------------------------------------

#[test]
fn no_maximum_hand_size_while_controlled_is_a_player_effect() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Synthesis Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(
            "I — Draw a card. You have no maximum hand size for as long as you control this Saga.\nII — Draw a card.\nIII — Draw a card.",
        )
        .expect("saga compiles");
    let program = format!("{:?}", definition.abilities);
    assert!(program.contains("CantEffect"), "{program}");
    assert!(program.contains("NoMaximumHandSize(You)"), "{program}");
    assert!(program.contains("ObjectControlledBy"), "{program}");
    assert!(
        !program.contains("AddAbility"),
        "the rule change is not an ability granted to the Saga: {program}"
    );
}

// ---------------------------------------------------------------------------
// Finding: a repeated "exile the top card" tagged only the last repetition's
// card (Fateful Tempest).
// ---------------------------------------------------------------------------

#[test]
fn present_votes_exile_every_card_into_the_play_permission_pool() {
    let definition = compile(
        "Fateful Tempest",
        vec![CardType::Sorcery],
        "Council's dilemma — Starting with you, each player votes for past or present. You mill a card for each past vote, then Fateful Tempest deals damage to each opponent equal to the total mana value of cards milled this way. Exile the top card of your library for each present vote. Until the end of your next turn, you may play the exiled cards.",
    );
    let program = spell_program(&definition);
    let exile = program
        .split("ExileTopOfLibraryEffect")
        .nth(1)
        .expect("exile-top effect");
    let exile = &exile[..exile.find("face_down").unwrap_or(exile.len())];
    assert!(
        exile.contains("moved_tags: []"),
        "the repeated exile must not reset its pool each vote: {exile}"
    );
    assert!(exile.contains("accumulated_tags: [TagKey("), "{exile}");
}

// ---------------------------------------------------------------------------
// Finding: an eligible optional cast that was declined (or a card that
// wasn't eligible) never reached the "put it into your hand" fallback
// (Bre of Clan Stoutarm, Ziatora's Envoy, Fishing Gear).
// ---------------------------------------------------------------------------

#[test]
fn declined_eligible_cast_also_goes_to_hand() {
    let definition = compile(
        "Bre of Clan Stoutarm",
        vec![CardType::Creature],
        "At the beginning of your end step, if you gained life this turn, exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is less than or equal to the amount of life you gained this turn. Otherwise, put it into your hand.",
    );
    let program = format!("{:?}", definition.abilities);
    let conditional = program
        .split("ConditionalEffect")
        .nth(1)
        .expect("eligibility conditional");
    let if_true = &conditional[..conditional.find("if_false").expect("if_false arm")];
    assert!(if_true.contains("CastTaggedEffect"), "{if_true}");
    assert!(
        if_true.contains("DidNotHappen") && if_true.contains("zone: Hand"),
        "declining the eligible cast puts the card into hand: {if_true}"
    );
}

#[test]
fn if_you_dont_fallback_follows_the_eligibility_check() {
    let definition = compile(
        "Envoy Probe",
        vec![CardType::Instant],
        "Look at the top card of your library. You may play a land from the top of your library or cast a spell with mana value 3 or less from the top of your library without paying its mana cost. If you don't, put that card into your hand.",
    );
    let segments = &definition.spell_effect.as_ref().unwrap().segments;
    let effects = segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .collect::<Vec<_>>();
    let conditional_index = effects
        .iter()
        .position(|effect| {
            effect
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .is_some()
        })
        .unwrap_or_else(|| panic!("eligibility conditional: {effects:?}"));
    let conditional = effects[conditional_index]
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .unwrap();
    assert!(
        !format!("{:?}", conditional.if_true).contains("DidNotHappen"),
        "the fallback no longer hides inside the eligible arm: {:?}",
        conditional.if_true
    );
    let fallback = effects
        .get(conditional_index + 1)
        .and_then(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
        .unwrap_or_else(|| panic!("fallback follows the conditional: {effects:?}"));
    assert_eq!(fallback.predicate, crate::effect::EffectPredicate::DidNotHappen);
    assert!(format!("{:?}", fallback.then).contains("Hand"));
}

// ---------------------------------------------------------------------------
// Finding: "When this creature dies, you may cast it from your graveyard as
// an Adventure ..." never granted a permission (Mosswood Dreadknight,
// Hildibrand Manderville).
// ---------------------------------------------------------------------------

fn adventure_dies_definition() -> crate::CardDefinition {
    compile(
        "Dreadknight Probe",
        vec![CardType::Creature],
        "When this creature dies, you may cast it from your graveyard as an Adventure until the end of your next turn.",
    )
}

fn grant_permission_in(zone: Zone) -> bool {
    let definition = adventure_dies_definition();
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("dies trigger");
    };
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let card = game.create_object_from_definition(&definition, alice, zone);
    let before = game.effect_store.grant_registry.grants.len();
    let mut decisions = TakeAll;
    let mut ctx = crate::effects::EffectContext::new(card, alice, &mut decisions);
    for segment in &triggered.effects.segments {
        for effect in &segment.default_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
    }
    game.effect_store.grant_registry.grants[before..]
        .iter()
        .any(|grant| grant.target_id == Some(card))
}

#[test]
fn dies_adventure_permission_is_granted_for_the_graveyard_card() {
    assert!(grant_permission_in(Zone::Graveyard));
}

#[test]
fn dies_adventure_permission_needs_the_card_in_the_graveyard() {
    assert!(!grant_permission_in(Zone::Exile));
    let program = format!("{:?}", adventure_dies_definition().abilities);
    assert!(program.contains("GrantPlayTaggedEffect"), "{program}");
    assert!(program.contains("Graveyard"), "{program}");
}

// ---------------------------------------------------------------------------
// Smaller lowering findings: Wall of Dust start player, Mystic Reflection
// copy source, Secret of Bloodbending replacement player, Genesis Storm
// cleanup set, Null Summoner cast-only permission.
// ---------------------------------------------------------------------------

#[test]
fn blocked_creatures_restriction_starts_on_its_controllers_turn() {
    let definition = compile(
        "Wall of Dust",
        vec![CardType::Creature],
        "Defender\nWhenever this creature blocks a creature, that creature can't attack during its controller's next turn.",
    );
    let program = format!("{:?}", definition.abilities);
    let start = program.split("start: NextTurn(").nth(1).expect("next-turn start");
    let start = &start[..start.find("duration_surface").unwrap_or(start.len())];
    assert!(start.contains("\"blocked\""), "{start}");
    assert!(!start.contains("__it__"), "not the blocker's controller: {start}");
}

#[test]
fn copy_source_binds_the_declared_chosen_creature() {
    let definition = compile(
        "Mystic Reflection",
        vec![CardType::Instant],
        "Choose target nonlegendary creature. The next time one or more creatures or planeswalkers enter this turn, they enter as copies of the chosen creature.",
    );
    let program = spell_program(&definition);
    let declaration = program
        .split("TargetOnlyEffect")
        .next()
        .expect("declaration prefix");
    assert!(
        declaration.contains("__chosen_objects__"),
        "the declaration is recorded as the chosen object: {program}"
    );
    assert!(program.contains("RegisterNextBatchEnterAsCopyEffect"), "{program}");
}

#[test]
fn replacement_controls_the_same_targeted_player() {
    let definition = compile(
        "Bloodbending Probe",
        vec![CardType::Sorcery],
        "As an additional cost to cast this spell, you may waterbend {10}.\nYou control target opponent during their next combat phase. If this spell's additional cost was paid, you control that player during their next turn instead.",
    );
    let program = spell_program(&definition);
    let replacement = program
        .split("replacement_effects")
        .nth(1)
        .expect("replacement branch");
    assert!(
        !replacement.contains("Target(\n                                                    Target("),
        "no nested second target: {replacement}"
    );
    assert!(replacement.contains("AliasedTarget"), "{replacement}");
    assert!(
        !replacement.contains("target_spec: Some("),
        "the replacement announces no new target: {replacement}"
    );
}

#[test]
fn revealed_cleanup_excludes_the_card_put_onto_the_battlefield() {
    let definition = compile(
        "Genesis Probe",
        vec![CardType::Sorcery],
        "Reveal cards from the top of your library until you reveal a nonland permanent card. You may put that card onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield on the bottom of your library in a random order.",
    );
    let program = spell_program(&definition);
    let cleanup = program.rsplit("MoveToZoneEffect").next().unwrap();
    assert!(
        cleanup.contains("target: All(ObjectFilter { zone: Some(Library)"),
        "only cards still in the library go to the bottom: {cleanup}"
    );
}

#[test]
fn cast_the_exiled_card_permission_excludes_land_plays() {
    let definition = compile(
        "Null Probe",
        vec![CardType::Creature],
        "When this creature enters, if you cast it, target opponent reveals their hand. You choose a nonland card from it. Exile that card.\nThreshold — As long as there are seven or more cards in your graveyard, you may cast the exiled card, and mana of any type can be spent to cast that spell.",
    );
    let program = format!("{:?}", definition.abilities);
    assert!(program.contains("grantable: PlayFrom"), "{program}");
    assert!(
        program.contains("excluded_card_types: [Land]"),
        "a cast-only permission never plays a land: {program}"
    );
}
