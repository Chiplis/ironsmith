//! Source-pass regression coverage. Intentionally unrun under the no-build request.
use super::compile_to_runtime_definition;

fn compile(text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition("Mechanic regression", text, false)
        .expect("reusable mechanic compiles")
}

#[test]
fn grouped_dice_export_their_complete_result_set() {
    let card = compile("Type: Sorcery\nRoll three six-sided dice. For each different result, create a 1/1 white Clown Robot artifact creature token.");
    let debug = format!("{:?}", card.spell_effect);
    assert!(debug.contains("total: true"), "one grouped roll: {debug}");
    assert!(debug.contains("DistinctDieResults"), "exact roll receipt: {debug}");
    assert!(!debug.contains("RepeatEffectsEffect"), "no repeat-last result: {debug}");
}

#[test]
fn entry_dice_and_counters_share_one_program() {
    let card = compile("Mana cost: {X}{X}{G}{G}\nType: Creature — Hydra\nPower/Toughness: 0/0\nAs this creature enters, roll X d6. It enters with a number of +1/+1 counters on it equal to the total of those results.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("AsEntersEffectProgram"), "{debug}");
    assert!(debug.contains("total: true") && debug.contains("PutCountersEffect"), "{debug}");
}

#[test]
fn type_setting_and_ability_loss_share_event_ended_duration() {
    let card = compile("Type: Creature\nPower/Toughness: 1/1\n{1}{W}, {T}: Target creature becomes an enchantment and loses all abilities until a player casts a creature spell.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("SetCardTypes") && debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(debug.matches("AnyPlayerCastsCreatureSpell").count() >= 2, "both changes end together: {debug}");
}

#[test]
fn die_table_multisentence_rows_remain_owned_by_the_roll() {
    let card = compile("Type: Planeswalker\nLoyalty: 5\n0: Roll a six-sided die.\n1 or 2 — [+2], then create two 1/1 green Squirrel creature tokens. They gain haste until end of turn.\n3 — [−1], then return a card with mana value 2 or less from your graveyard to your hand.\n4 or 5 — This planeswalker deals damage equal to the number of loyalty counters on it to a creature or player, then [−2].\n6 — [+1], and you may activate this planeswalker's loyalty ability two more times this turn.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("RollDieEffect") && debug.contains("Haste"), "{debug}");
}

#[test]
fn counted_die_results_reference_one_local_roll() {
    let card = compile("Type: Artifact — Bobblehead\n{1}, {T}: Roll X six-sided dice, where X is the number of Bobbleheads you control. Create a tapped Treasure token for each even result. If you rolled 6 exactly seven times, you win the game.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("EvenDieResults") && debug.contains("DieResultsEqual(6)"), "{debug}");
    assert!(debug.contains("count_value: Some("), "dynamic count survives: {debug}");
}

#[test]
fn numeric_die_metrics_count_duplicate_faces_in_the_exact_receipt() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::{GameState, PlayerId, Value};
    use ironsmith::target::PlayerFilter;
    use ironsmith::effect::{EffectId, EffectMetric, EffectMetricSource, PriorEffectMetricQuery};
    let player = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.new_object_id();
    for face in [2, 6, 6, 3] { game.force_next_die_roll(face); }
    let mut ctx = EffectContext::new_default(source, player);
    let roll = ironsmith::effects::RollDiceChooseResultEffect::new(PlayerFilter::You, 4, 6).with_total(None);
    let outcome = roll.execute(&mut game, &mut ctx).expect("roll resolves");
    ctx.store_outcome(EffectId(0), outcome);
    for (metric, expected) in [(EffectMetric::EvenDieResults, 3), (EffectMetric::OddDieResults, 1), (EffectMetric::DieResultsEqual(6), 2), (EffectMetric::DistinctDieResults, 3)] {
        let value = Value::PriorEffectMetric { effect_id: EffectId(0), query: PriorEffectMetricQuery::new(EffectMetricSource::Outcome, metric) };
        assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(), expected);
    }
}

#[test]
fn cast_trigger_registers_future_destination_instead_of_exiling_on_cast() {
    for body in [
        "exile it instead of putting it into a graveyard as it resolves.",
        "exile that card with three time counters on it instead of putting it into your graveyard as it resolves. Then if the exiled card doesn't have suspend, it gains suspend.",
    ] {
        let card = compile(&format!("Type: Artifact\nWhenever a player casts an instant or sorcery spell, {body}"));
        let debug = format!("{:?}", card.abilities);
        assert!(debug.contains("RegisterFutureZoneReplacementEffect") || debug.contains("RegisterZoneReplacementEffect"), "{debug}");
        assert!(!debug.contains("ExileEffect {"), "spell remains on stack until resolution: {debug}");
        let surface = ironsmith_text::compiled_text_lines(&card).join("\n");
        assert!(surface.contains("instead") && surface.contains("resolves"), "{surface}");
    }
}

#[test]
fn consult_damage_replacement_retains_original_damage_and_stop_card() {
    let card = compile("Type: Artifact\n{3}, {T}: Reveal cards from the top of your library until you reveal a land card. This artifact deals damage equal to the number of nonland cards revealed this way to any target. If the revealed land card was a Mountain, this artifact deals double that damage instead. Put the revealed cards on the bottom of your library in any order.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("self_replacements") && debug.contains("Mountain"), "{debug}");
    let surface = ironsmith_text::compiled_text_lines(&card).join("\n");
    assert!(surface.contains("double") && surface.contains("instead"), "{surface}");
}

#[test]
fn noted_counter_noun_conjunction_is_one_instruction() {
    let card = compile("Type: Artifact\n{3}, {T}: Exile target creature and all Auras attached to it. Note the number and kind of counters that were on that creature. When this artifact leaves the battlefield or becomes untapped, return that exiled card to the battlefield under its owner's control tapped with the noted number and kind of counters on it. If you do, return the other exiled cards to the battlefield under their owner's control attached to that permanent.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("NoteCountersEffect") && debug.contains("noted_counters"), "{debug}");
}

#[test]
fn mixed_animation_grant_preserves_final_quoted_dice_trigger() {
    let card = compile("Type: Creature\nPower/Toughness: 1/1\nAt the beginning of combat on your turn, choose target nonland permanent you control. Until end of turn, it becomes a Vehicle artifact with base power and toughness each equal to its mana value, and it gains crew 2 and \"Crash Land — Whenever this Vehicle deals damage, roll a six-sided die. If the result is equal to this Vehicle's mana value, sacrifice this Vehicle, then it deals that much damage to any target.\"");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("CrewCostEffect") && debug.contains("RollDieEffect"), "{debug}");
}

#[test]
fn highest_roll_restriction_renders_its_declared_player_group() {
    let card = compile("Type: Creature\nPower/Toughness: 4/4\nAt the beginning of combat on your turn, each player rolls a d20. If one or more opponents had the highest result, this creature can't attack those players or planeswalkers they control this combat.");
    let surface = ironsmith_text::compiled_text_lines(&card).join("\n");
    assert!(surface.contains("opponents had the highest result"), "{surface}");
    assert!(!surface.contains("tagged") && !surface.contains("__sentence_helper"), "{surface}");
}

#[test]
fn sticker_aggregate_reads_all_physical_pt_stickers_not_final_creature_stats() {
    use ironsmith::effects::EffectContext;
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    let card = compile("Type: Creature\nPower/Toughness: 1/1");
    let player = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.create_object_from_definition(&card, player, Zone::Battlefield);
    let first = game.add_accessible_pt_sticker(player, 0, 2, 3);
    let second = game.add_accessible_pt_sticker(player, 0, 4, 5);
    assert!(game.apply_pt_sticker(source, first).is_some());
    assert!(game.apply_pt_sticker(source, second).is_some());
    let ctx = EffectContext::new_default(source, player);
    for (toughness, expected) in [(false, 6), (true, 8)] {
        let value = ironsmith::Value::StickerCharacteristicTotal {
            filter: ObjectFilter::permanent().controlled_by(PlayerFilter::You), toughness,
        };
        assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(), expected);
    }
}

#[test]
fn coupled_base_characteristics_read_sticker_totals() {
    let card = compile("Type: Creature\nPower/Toughness: 3/3\nAt the beginning of each combat, you may have this creature's base power become equal to the total power of all stickers on permanents you control and its base toughness become equal to those stickers' total toughness.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.matches("StickerCharacteristicTotal").count() >= 2, "{debug}");
}

#[test]
fn chosen_player_every_draw_reveal_is_a_static_rule() {
    let card = compile("Type: Artifact\nAs this artifact enters, choose an opponent and a card name other than a basic land card name.\nThe chosen player reveals each card they draw.\nWhen the chosen player draws a card with the chosen name, sacrifice this artifact. If you do, this artifact deals 10 damage to that player.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("RevealCardsDrawn") && debug.contains("ChosenPlayer"), "{debug}");
}

#[test]
fn original_ambassador_name_keeps_both_sticker_characteristics() {
    let card = compile_to_runtime_definition("Ambassador Blorpityblorpboop", "Mana cost: {3}{G}{U}\nType: Legendary Creature — Alien Advisor Guest\nPower/Toughness: 3/3\nWhen Ambassador Blorpityblorpboop enters, you get {TK}{TK}{TK}, then you may put a sticker on a nonland permanent you own.\nAt the beginning of each combat, you may have Ambassador Blorpityblorpboop's base power become equal to the total power of all stickers on permanents you control and its base toughness become equal to those stickers' total toughness.", false).unwrap();
    let debug = format!("{:?}", card.abilities);
    assert!(debug.matches("StickerCharacteristicTotal").count() >= 2, "{debug}");
}

#[test]
fn extra_die_replacement_uses_its_own_chooser_and_expires_after_one_batch() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    use ironsmith::{GameState, PlayerId};
    struct IgnoreLast(Vec<PlayerId>);
    impl DecisionMaker for IgnoreLast {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            self.0.push(ctx.player); vec![ctx.options.len() - 1]
        }
    }
    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.new_object_id();
    let rule = ironsmith::effects::RegisterExtraDieReplacementEffect {
        player: ChooseSpec::SpecificPlayer(b), chooser: PlayerFilter::You, additional: 1,
    };
    let mut choices = IgnoreLast(Vec::new());
    let mut ctx = EffectContext::new_default(source, a).with_decision_maker(&mut choices);
    rule.execute(&mut game, &mut ctx).unwrap();
    for face in [2, 6, 4] { game.force_next_die_roll(face); }
    let roll = ironsmith::effects::RollDieEffect::new(PlayerFilter::Specific(b), 6);
    assert_eq!(roll.execute(&mut game, &mut ctx).unwrap().as_count(), Some(2));
    assert_eq!(roll.execute(&mut game, &mut ctx).unwrap().as_count(), Some(4));
    drop(ctx);
    assert_eq!(choices.0, vec![a]);
}

#[test]
fn extra_die_replacement_compiles_as_a_future_rule() {
    let card = compile("Type: Creature — Beeble\nPower/Toughness: 1/1\n{1}, {T}: The next time target player would roll one or more dice this turn, instead they roll that many dice plus one and you choose one of those rolls to ignore.");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("RegisterExtraDieReplacementEffect"), "{debug}");
    assert!(!debug.contains("RollDieEffect"), "registration must not roll immediately: {debug}");
}

#[test]
fn chosen_player_pair_damage_is_correlated_and_excludes_self_damage() {
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::target::ChooseSpec;
    use ironsmith::{GameState, PlayerId, Zone};
    let rule = compile("Type: Enchantment\nIf a source controlled by one of the chosen players would deal damage to the other chosen player or a permanent that player controls, that source deals double that damage to that player or permanent instead.");
    let source_card = compile("Type: Creature\nPower/Toughness: 2/2");
    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1); let c = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    let enchantment = game.create_object_from_definition(&rule, c, Zone::Battlefield);
    game.set_chosen_players(enchantment, vec![a, b]);
    let source = game.create_object_from_definition(&source_card, a, Zone::Battlefield);
    for (recipient, expected_life) in [(a, 18), (b, 16), (c, 18)] {
        let damage = ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(recipient));
        damage.execute(&mut game, &mut EffectContext::new_default(source, a)).unwrap();
        assert_eq!(game.player(recipient).unwrap().life, expected_life);
    }
}

#[test]
fn dynamic_equipment_grants_continue_after_the_subtype_layer() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::object::AttachmentTarget;
    let rule = compile("Type: Enchantment\nEach noncreature, non-Equipment artifact is an Equipment with equip {X} and \"Equipped creature gets +X/+0,\" where X is that artifact's mana value.");
    let artifact = compile("Mana cost: {4}\nType: Artifact");
    let creature = compile("Type: Creature\nPower/Toughness: 2/2");
    let a = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let _rule = game.create_object_from_definition(&rule, a, Zone::Battlefield);
    let equipment = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
    let host = game.create_object_from_definition(&creature, a, Zone::Battlefield);
    assert!(game.current_has_subtype(equipment, ironsmith::Subtype::Equipment));
    let abilities = game.current_abilities(equipment).unwrap();
    assert!(abilities.iter().any(|ability| format!("{ability:?}").contains("DynamicMana")), "{abilities:?}");
    assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
    assert_eq!(game.current_power(host), Some(6));
}

#[test]
fn untapped_source_protects_new_attachments_and_control_without_reverting_custody() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::object::AttachmentTarget;
    let beast = compile("Mana cost: {3}{B}\nType: Creature — Beast\nPower/Toughness: 2/4\nAs long as this creature is untapped, noncreature artifacts you control can't be enchanted, they have indestructible, and other players can't gain control of them. This effect doesn't remove Auras already attached to those artifacts.");
    let debug = format!("{:?}", beast.abilities);
    assert!(debug.contains("BeAttachedByNew") && debug.contains("GainControl") && debug.contains("Indestructible"), "{debug}");
    let artifact = compile("Type: Artifact");
    let aura = compile("Type: Enchantment — Aura\nEnchant artifact");
    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.create_object_from_definition(&beast, a, Zone::Battlefield);
    let protected = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
    let old_aura = game.create_object_from_definition(&aura, a, Zone::Battlefield);
    game.tap(source);
    game.refresh_continuous_state().unwrap();
    assert!(game.attach_object_to_target(old_aura, AttachmentTarget::Object(protected)));
    game.untap(source);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.object(old_aura).unwrap().attached_to, Some(AttachmentTarget::Object(protected)));
    let new_aura = game.create_object_from_definition(&aura, b, Zone::Battlefield);
    // Exercise attachment legality through the effect; the game-state
    // transition method is the unchecked mutation used after validation.
    use ironsmith::effects::{EffectExecutor, EffectContext, ResolvedTarget};
    let mut attachment_ctx = EffectContext::new_default(new_aura, b)
        .with_targets(vec![ResolvedTarget::Object(protected)]);
    ironsmith::effects::AttachToEffect::new(ironsmith::target::ChooseSpec::target_permanent())
        .execute(&mut game, &mut attachment_ctx).unwrap();
    drop(attachment_ctx);
    assert_eq!(game.object(new_aura).unwrap().attached_to, None);
    game.set_current_controller(protected, b).unwrap();
    assert_eq!(game.current_controller(protected), Some(a));
    game.tap(source);
    game.set_current_controller(protected, b).unwrap();
    assert_eq!(game.current_controller(protected), Some(b));
    game.untap(source);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(protected), Some(b), "protection does not reverse an accepted acquisition");
}

#[test]
fn pending_aura_control_starts_when_source_becomes_tapped() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::object::AttachmentTarget;
    let beast = compile("Type: Creature — Beast\nPower/Toughness: 2/4\nAs long as this creature is untapped, noncreature artifacts you control can't be enchanted, they have indestructible, and other players can't gain control of them. This effect doesn't remove Auras already attached to those artifacts.");
    let artifact = compile("Type: Artifact");
    let aura = compile("Type: Enchantment — Aura\nEnchant artifact\nYou control enchanted artifact.");
    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.create_object_from_definition(&beast, a, Zone::Battlefield);
    let protected = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
    // Establish the Aura while protection is inactive, then change its
    // controller while the source is untapped: this is a fresh control gain.
    game.tap(source);
    let attachment = game.create_object_from_definition(&aura, a, Zone::Battlefield);
    assert!(game.attach_object_to_target(attachment, AttachmentTarget::Object(protected)));
    game.untap(source);
    game.set_current_controller(attachment, b).unwrap();
    assert_eq!(game.current_controller(protected), Some(a));
    game.tap(source);
    let view = game.continuous_query_snapshot().unwrap();
    assert_eq!(view.current_controller(protected), Some(b));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(protected), Some(b));
    game.untap(source);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(protected), Some(b));
}

#[test]
fn protection_source_departure_releases_a_preexisting_control_effect() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::object::AttachmentTarget;
    let beast = compile("Type: Creature — Beast\nPower/Toughness: 2/4\nAs long as this creature is untapped, noncreature artifacts you control can't be enchanted, they have indestructible, and other players can't gain control of them. This effect doesn't remove Auras already attached to those artifacts.");
    let artifact = compile("Type: Artifact");
    let aura = compile("Type: Enchantment — Aura\nEnchant artifact\nYou control enchanted artifact.");
    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.create_object_from_definition(&beast, a, Zone::Battlefield);
    let protected = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
    game.tap(source);
    let attachment = game.create_object_from_definition(&aura, a, Zone::Battlefield);
    assert!(game.attach_object_to_target(attachment, AttachmentTarget::Object(protected)));
    game.untap(source);
    game.set_current_controller(attachment, b).unwrap();
    assert_eq!(game.current_controller(protected), Some(a));
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_controller(protected), Some(b));
    assert_eq!(game.object(attachment).unwrap().attached_to, Some(AttachmentTarget::Object(protected)));
}

#[test]
fn mana_eligible_private_face_down_cast_retains_the_complete_event_rule() {
    let card = compile_to_runtime_definition("Illusionary Mask", "Mana cost: {2}\nType: Artifact\n{X}: You may choose a creature card in your hand whose mana cost could be paid by some amount of, or all of, the mana you spent on {X}. If you do, you may cast that card face down as a 2/2 creature spell without paying its mana cost. If the creature that spell becomes as it resolves has not been turned face up and would assign or deal damage, be dealt damage, or become tapped, instead it's turned face up and assigns or deals damage, is dealt damage, or becomes tapped. Activate only as a sorcery.", false).expect("full original named oracle compiles");
    let debug = format!("{:?}", card.abilities);
    assert!(debug.contains("CastFaceDownUsingActivationManaEffect"), "{debug}");
    assert!(debug.contains("turn_face_up_before_action: true"), "{debug}");
    assert!(debug.contains("Sorcery"), "{debug}");
}
